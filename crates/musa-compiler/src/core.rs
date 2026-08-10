//! The private total elaboration core (`docs/language/02-core-calculus.md`).
//!
//! This module owns lowering, checking, dependency validation, and evaluation
//! for elaboration values.  Its two crate-private entry points deliberately
//! return only success: the existing compiler facade remains the sole public
//! operation, diagnostics accumulate in the ordinary resolver, and no caller
//! can observe or orchestrate the pass representation.

use indexmap::{IndexMap, IndexSet};
use musa_language::ast::{AstNode as _, FnDecl, LetDecl, VoiceItem};
use musa_language::{SyntaxElement, SyntaxKind, SyntaxNode, SyntaxToken};
use num_rational::Ratio;

use crate::core_budget::WorkMeter;
use crate::diagnose::{Code, Diagnostic};
use crate::imports::Libraries;
use crate::origin::{Interval, SourceSpan};
use crate::pitch::{PitchClass, WrittenPitch};
use crate::resolve::{NameKind, Resolver};

/// Check and evaluate imported definitions followed by a piece's definitions.
pub(crate) fn check_piece(
    resolver: &mut Resolver,
    libraries: &Libraries,
    piece: &musa_language::ast::PieceDecl,
) -> Option<Program> {
    if !validate_imports(resolver, libraries) {
        return None;
    }
    check_and_evaluate(
        resolver,
        libraries
            .each()
            .flat_map(|(path, library)| declarations(library.syntax(), Some(path)))
            .chain(declarations(piece.syntax(), None)),
        Some(piece.syntax()),
        UnknownRootMusic::Defer,
    )
}

/// Check and evaluate imported definitions followed by an opened library's.
pub(crate) fn check_material(
    resolver: &mut Resolver,
    libraries: &Libraries,
    library: &musa_language::ast::LibraryDecl,
) -> bool {
    if !validate_imports(resolver, libraries) {
        return false;
    }
    check_and_evaluate(
        resolver,
        libraries
            .each()
            .flat_map(|(path, imported)| declarations(imported.syntax(), Some(path)))
            .chain(declarations(library.syntax(), None)),
        None,
        UnknownRootMusic::Reject,
    )
    .is_some()
}

/// Check each library with precisely the earlier libraries available to it.
/// A failure is then restated at the importing document's `use` span: spans
/// inside the foreign CST must never be published as spans in this document.
fn validate_imports(resolver: &mut Resolver, libraries: &Libraries) -> bool {
    let mut prefix = Vec::new();
    for (path, library, import_span) in libraries.each_with_import_span() {
        prefix.extend(declarations(library.syntax(), Some(path)));
        let mut foreign_resolver = Resolver::new();
        let evaluated = check_and_evaluate(
            &mut foreign_resolver,
            prefix.clone().into_iter(),
            None,
            UnknownRootMusic::Reject,
        )
        .is_some();
        let first_error = foreign_resolver
            .diagnostics
            .iter()
            .find(|diagnostic| diagnostic.severity == crate::diagnose::Severity::Error);
        if !evaluated || first_error.is_some() {
            let detail = first_error.map_or("elaboration evaluation failed", |diagnostic| {
                diagnostic.message.as_str()
            });
            resolver.report(
                Diagnostic::error(Code::Import, format!("`{path}` does not compile"))
                    .at(import_span, "imported here")
                    .note(format!("it says: {detail}"))
                    .help(format!("run `musa check {path}`")),
            );
            return false;
        }
    }
    true
}

fn declarations(owner: &SyntaxNode, source: Option<&str>) -> Vec<SurfaceDefinition> {
    let mut found: Vec<_> = owner
        .children()
        .filter_map(|node| {
            LetDecl::cast(node.clone())
                .map(|declaration| SurfaceDefinition::Let {
                    declaration,
                    source: source.map(str::to_owned),
                })
                .or_else(|| {
                    FnDecl::cast(node.clone()).map(|declaration| SurfaceDefinition::Function {
                        declaration,
                        source: source.map(str::to_owned),
                    })
                })
                .or_else(|| {
                    musa_language::ast::MotifDecl::cast(node.clone()).map(|declaration| SurfaceDefinition::Legacy {
                        name: declaration.name().unwrap_or_default(),
                        syntax: declaration.syntax().clone(),
                        parameters: declaration.params(),
                        material: crate::resolve::Material::Motif,
                        source: source.map(str::to_owned),
                    })
                })
                .or_else(|| {
                    musa_language::ast::FragmentDecl::cast(node).map(|declaration| SurfaceDefinition::Legacy {
                        name: declaration.name().unwrap_or_default(),
                        syntax: declaration.syntax().clone(),
                        parameters: Vec::new(),
                        material: crate::resolve::Material::Fragment,
                        source: source.map(str::to_owned),
                    })
                })
        })
        .collect();
    // Named bars are declarations even though they sit inside the score.
    // Anonymous bars remain ordinary structure and never enter a namespace.
    found.extend(owner.descendants().filter_map(|node| {
        musa_language::ast::BarStmt::cast(node).and_then(|declaration| {
            declaration.name().map(|name| SurfaceDefinition::Legacy {
                name,
                syntax: declaration.syntax().clone(),
                parameters: Vec::new(),
                material: crate::resolve::Material::Bar,
                source: source.map(str::to_owned),
            })
        })
    }));
    found
}

fn root_uses(owner: &SyntaxNode) -> Vec<SyntaxNode> {
    root_nodes(owner, SyntaxKind::UseStmt)
}

/// Statements written among a piece's own items rather than inside material.
///
/// A motif, fragment, named bar or `music { ... }` body is a definition: the
/// core checks and evaluates it once, under its own name, and what it writes
/// belongs to that definition. Everything left over is written where the
/// piece plays it, and is checked here instead.
fn root_nodes(owner: &SyntaxNode, kind: SyntaxKind) -> Vec<SyntaxNode> {
    owner
        .descendants()
        .filter(|node| node.kind() == kind)
        .filter(|node| {
            !node.ancestors().skip(1).any(|ancestor| {
                matches!(
                    ancestor.kind(),
                    SyntaxKind::MusicExpr | SyntaxKind::MotifDecl | SyntaxKind::FragmentDecl
                ) || (ancestor.kind() == SyntaxKind::BarStmt
                    && musa_language::ast::BarStmt::cast(ancestor)
                        .and_then(|bar| bar.name())
                        .is_some())
            })
        })
        .collect()
}

#[derive(Clone)]
enum SurfaceDefinition {
    Let {
        declaration: LetDecl,
        source: Option<String>,
    },
    Function {
        declaration: FnDecl,
        source: Option<String>,
    },
    Legacy {
        name: String,
        syntax: SyntaxNode,
        parameters: Vec<musa_language::ast::Param>,
        material: crate::resolve::Material,
        source: Option<String>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Type {
    Unit,
    Bool,
    Nat,
    Ratio,
    Duration,
    Pitch,
    PitchClass,
    Interval,
    Scale,
    Key,
    Degree,
    Frame,
    Product(Vec<Self>),
    Option(Box<Self>),
    List(Box<Self>),
    Music,
    Function(Vec<Self>, Box<Self>),
}

impl std::fmt::Display for Type {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unit => out.write_str("unit"),
            Self::Bool => out.write_str("bool"),
            Self::Nat => out.write_str("nat"),
            Self::Ratio => out.write_str("ratio"),
            Self::Duration => out.write_str("duration"),
            Self::Pitch => out.write_str("pitch"),
            Self::PitchClass => out.write_str("pitchclass"),
            Self::Interval => out.write_str("interval"),
            Self::Scale => out.write_str("scale"),
            Self::Key => out.write_str("key"),
            Self::Degree => out.write_str("degree"),
            Self::Frame => out.write_str("frame"),
            Self::Product(members) => {
                out.write_str("(")?;
                for (index, member) in members.iter().enumerate() {
                    if index > 0 {
                        out.write_str(", ")?;
                    }
                    write!(out, "{member}")?;
                }
                out.write_str(")")
            }
            Self::Option(member) => write!(out, "option[{member}]"),
            Self::List(member) => write!(out, "list[{member}]"),
            Self::Music => out.write_str("music"),
            Self::Function(parameters, result) => {
                if parameters.len() == 1 {
                    let parameter = parameters.first().unwrap_or(&Self::Unit);
                    if matches!(parameter, Self::Function(_, _)) {
                        write!(out, "({parameter}) -> {result}")
                    } else {
                        write!(out, "{parameter} -> {result}")
                    }
                } else {
                    out.write_str("(")?;
                    for (index, parameter) in parameters.iter().enumerate() {
                        if index > 0 {
                            out.write_str(", ")?;
                        }
                        write!(out, "{parameter}")?;
                    }
                    write!(out, ") -> {result}")
                }
            }
        }
    }
}

#[derive(Clone)]
struct RawParameter {
    name: String,
    ty: Type,
    default: Option<RawDefault>,
    span: SourceSpan,
}

#[derive(Clone)]
enum RawDefault {
    Expression(SyntaxNode),
    Value(Box<Value>),
}

struct RawDefinition {
    name: String,
    ty: Type,
    kind: RawDefinitionKind,
    name_span: SourceSpan,
    span: SourceSpan,
    foreign: bool,
    source: Option<String>,
    role: Option<MusicRole>,
}

enum RawDefinitionKind {
    Let {
        body: SyntaxNode,
    },
    Function {
        parameters: Vec<RawParameter>,
        body: SyntaxNode,
    },
    Music {
        parameters: Vec<RawParameter>,
        body: SyntaxNode,
        callable: bool,
    },
}

impl RawDefinition {
    fn name_kind(&self) -> NameKind {
        match self.kind {
            RawDefinitionKind::Let { .. } => NameKind::Value,
            RawDefinitionKind::Function { .. } => NameKind::Function,
            RawDefinitionKind::Music { ref parameters, .. } if parameters.is_empty() => self
                .role
                .as_ref()
                .map_or(NameKind::Value, |role| role.material.name_kind()),
            RawDefinitionKind::Music { .. } => self
                .role
                .as_ref()
                .map_or(NameKind::Function, |role| role.material.name_kind()),
        }
    }
}

#[derive(Clone)]
struct Symbol {
    ty: Type,
    kind: NameKind,
    definition: usize,
    external_declaration: Option<crate::resolve::SourceLocation>,
}

#[derive(Clone)]
struct Expr {
    kind: ExprKind,
    ty: Type,
    span: SourceSpan,
}

#[derive(Clone)]
enum ExprKind {
    Literal(Value),
    Name(String),
    Product(Vec<Expr>),
    Option(Option<Box<Expr>>),
    List(Vec<Expr>),
    Apply {
        function: Box<Expr>,
        arguments: Vec<CallArgument>,
    },
    PitchAction {
        pitch: Box<Expr>,
        interval: Box<Expr>,
        down: bool,
    },
    Primitive {
        primitive: Primitive,
        arguments: Vec<Expr>,
    },
    Match {
        scrutinee: Box<Expr>,
        arms: Vec<CheckedArm>,
    },
    Step {
        base: Box<Expr>,
        steps: Box<Expr>,
        down: bool,
    },
    Music(CheckedMusic),
}

#[derive(Clone)]
struct CheckedMusic {
    items: Vec<VoiceItem>,
    uses: Vec<(SourceSpan, Expr)>,
    pitches: Vec<(SourceSpan, Expr)>,
    scales: Vec<(SourceSpan, Expr)>,
    bindings: Vec<String>,
    role: Option<MusicRole>,
    definition_span: SourceSpan,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Primitive {
    NatFold,
    ListFold,
    OptionFold,
    Map,
    Filter,
    Range,
    Repeat,
    IntervalAdd,
    IntervalInverse,
    PitchClassOf,
    SignatureScale,
    ScaleOn,
    ScaleTonic,
    ScaleSize,
    ScalePitch,
    PitchFrame,
    FrameScale,
    FrameTonic,
    FramePitch,
    DegreeOf,
    DegreeStepUp,
    DegreeStepDown,
    DegreeRaised,
    DegreeLowered,
}

#[derive(Clone, Copy)]
struct PrimitiveOwnership<T> {
    operation: T,
    spelling: &'static str,
    hidden_information: &'static str,
}

const PRIMITIVE_OWNERSHIP: [PrimitiveOwnership<Primitive>; 24] = [
    PrimitiveOwnership {
        operation: Primitive::NatFold,
        spelling: "nat_fold",
        hidden_information: "the evaluator's finite natural representation and structural work budget",
    },
    PrimitiveOwnership {
        operation: Primitive::ListFold,
        spelling: "list_fold",
        hidden_information: "the evaluator's finite list representation and structural work budget",
    },
    PrimitiveOwnership {
        operation: Primitive::OptionFold,
        spelling: "option_fold",
        hidden_information: "the evaluator's hidden option representation and total case dispatch",
    },
    PrimitiveOwnership {
        operation: Primitive::Map,
        spelling: "map",
        hidden_information: "rank-1 monomorphization over the evaluator's hidden finite list representation",
    },
    PrimitiveOwnership {
        operation: Primitive::Filter,
        spelling: "filter",
        hidden_information: "rank-1 monomorphization over the evaluator's hidden finite list representation",
    },
    PrimitiveOwnership {
        operation: Primitive::Range,
        spelling: "range",
        hidden_information: "bounded construction governed by the evaluator's structural work budget",
    },
    PrimitiveOwnership {
        operation: Primitive::Repeat,
        spelling: "repeat",
        hidden_information: "rank-1 finite-list construction governed by the structural work budget",
    },
    PrimitiveOwnership {
        operation: Primitive::IntervalAdd,
        spelling: "interval_add",
        hidden_information: "the evaluator's exact written-interval coordinate representation",
    },
    PrimitiveOwnership {
        operation: Primitive::IntervalInverse,
        spelling: "interval_inverse",
        hidden_information: "the evaluator's exact written-interval coordinate representation",
    },
    PrimitiveOwnership {
        operation: Primitive::PitchClassOf,
        spelling: "pitchclass_of",
        hidden_information: "the written pitch's octave coordinate and spelling-preserving quotient",
    },
    PrimitiveOwnership {
        operation: Primitive::SignatureScale,
        spelling: "signature_scale",
        hidden_information: "the compiler's table of named collections, which no source text can enumerate",
    },
    PrimitiveOwnership {
        operation: Primitive::ScaleOn,
        spelling: "scale_on",
        hidden_information: "the scale's private ordered offset cycle, re-rooted without being exposed",
    },
    PrimitiveOwnership {
        operation: Primitive::ScaleTonic,
        spelling: "scale_tonic",
        hidden_information: "the scale's private tonic field",
    },
    PrimitiveOwnership {
        operation: Primitive::ScaleSize,
        spelling: "scale_size",
        hidden_information: "the length of the scale's private offset cycle",
    },
    PrimitiveOwnership {
        operation: Primitive::ScalePitch,
        spelling: "scale_pitch",
        hidden_information: "spelled membership against the scale's private offset cycle",
    },
    PrimitiveOwnership {
        operation: Primitive::PitchFrame,
        spelling: "pitch_frame",
        hidden_information: "the register frame's representation invariant, which only the compiler can enforce",
    },
    PrimitiveOwnership {
        operation: Primitive::FrameScale,
        spelling: "frame_scale",
        hidden_information: "the frame's private scale field",
    },
    PrimitiveOwnership {
        operation: Primitive::FrameTonic,
        spelling: "frame_tonic",
        hidden_information: "the frame's private registered tonic field",
    },
    PrimitiveOwnership {
        operation: Primitive::FramePitch,
        spelling: "frame_pitch",
        hidden_information: "Euclidean division of a degree through the scale's private period",
    },
    PrimitiveOwnership {
        operation: Primitive::DegreeOf,
        spelling: "degree_of",
        hidden_information: "the degree's private signed coordinate, which is not the written ordinal",
    },
    PrimitiveOwnership {
        operation: Primitive::DegreeStepUp,
        spelling: "degree_step_up",
        hidden_information: "the degree's private signed coordinate and its machine-integer bound",
    },
    PrimitiveOwnership {
        operation: Primitive::DegreeStepDown,
        spelling: "degree_step_down",
        hidden_information: "the degree's private signed coordinate and its machine-integer bound",
    },
    PrimitiveOwnership {
        operation: Primitive::DegreeRaised,
        spelling: "degree_raised",
        hidden_information: "the degree's private chromatic alteration and its machine-integer bound",
    },
    PrimitiveOwnership {
        operation: Primitive::DegreeLowered,
        spelling: "degree_lowered",
        hidden_information: "the degree's private chromatic alteration and its machine-integer bound",
    },
];

impl Primitive {
    fn name(self) -> &'static str {
        match self {
            Self::NatFold => "nat_fold",
            Self::ListFold => "list_fold",
            Self::OptionFold => "option_fold",
            Self::Map => "map",
            Self::Filter => "filter",
            Self::Range => "range",
            Self::Repeat => "repeat",
            Self::IntervalAdd => "interval_add",
            Self::IntervalInverse => "interval_inverse",
            Self::PitchClassOf => "pitchclass_of",
            Self::SignatureScale => "signature_scale",
            Self::ScaleOn => "scale_on",
            Self::ScaleTonic => "scale_tonic",
            Self::ScaleSize => "scale_size",
            Self::ScalePitch => "scale_pitch",
            Self::PitchFrame => "pitch_frame",
            Self::FrameScale => "frame_scale",
            Self::FrameTonic => "frame_tonic",
            Self::FramePitch => "frame_pitch",
            Self::DegreeOf => "degree_of",
            Self::DegreeStepUp => "degree_step_up",
            Self::DegreeStepDown => "degree_step_down",
            Self::DegreeRaised => "degree_raised",
            Self::DegreeLowered => "degree_lowered",
        }
    }
}

#[derive(Clone)]
struct CheckedArm {
    pattern: Pattern,
    body: Expr,
}

#[derive(Clone)]
enum Pattern {
    Wildcard,
    Bind(String),
    Literal(Value),
    None,
    Some(String),
    EmptyList,
    Cons { head: String, tail: String },
    Product(Vec<String>),
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum Coverage {
    CatchAll,
    True,
    False,
    None,
    Some,
    EmptyList,
    Cons,
    Literal(String),
}

#[derive(Clone)]
struct CallArgument {
    parameter: usize,
    value: Expr,
}

struct CheckedDefinition {
    name: String,
    ty: Type,
    kind: CheckedDefinitionKind,
    dependencies: IndexMap<String, SourceSpan>,
    span: SourceSpan,
    foreign: bool,
}

enum CheckedDefinitionKind {
    Let {
        body: Expr,
    },
    Function {
        parameters: Vec<CheckedParameter>,
        body: Expr,
    },
}

#[derive(Clone)]
struct CheckedParameter {
    name: String,
    ty: Type,
    default: Option<Expr>,
}

#[derive(Clone)]
enum Value {
    Bool(bool),
    Nat(u64),
    Ratio(Ratio<i64>),
    Duration(Ratio<i64>),
    Pitch(WrittenPitch),
    PitchClass(PitchClass),
    Interval(Interval),
    Scale(crate::scale::Scale),
    Key(crate::Key),
    Degree(crate::scale::Degree),
    Frame(crate::scale::Frame),
    Product(Vec<Self>),
    Option { member: Type, value: Option<Box<Self>> },
    List { member: Type, values: Vec<Self> },
    Music(Music),
    Closure(Box<Closure>),
    Builtin(Box<BuiltinValue>),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Builtin {
    Transpose,
    Stretch,
    Retrograde,
    Invert,
    Shift,
    Overlay,
    MapNotePitches,
}

const BUILTIN_OWNERSHIP: [PrimitiveOwnership<Builtin>; 7] = [
    PrimitiveOwnership {
        operation: Builtin::Transpose,
        spelling: "transpose",
        hidden_information: "contextual music representation, written-pitch provenance, and kernel construction",
    },
    PrimitiveOwnership {
        operation: Builtin::Stretch,
        spelling: "stretch",
        hidden_information: "contextual music representation, exact-time provenance, and kernel construction",
    },
    PrimitiveOwnership {
        operation: Builtin::Retrograde,
        spelling: "retrograde",
        hidden_information: "contextual music extent, occurrence provenance, and kernel construction",
    },
    PrimitiveOwnership {
        operation: Builtin::Invert,
        spelling: "invert",
        hidden_information: "contextual music representation, written-pitch provenance, and kernel construction",
    },
    PrimitiveOwnership {
        operation: Builtin::Shift,
        spelling: "shift",
        hidden_information: "contextual music representation, exact-time provenance, and kernel construction",
    },
    PrimitiveOwnership {
        operation: Builtin::Overlay,
        spelling: "overlay",
        hidden_information: "contextual music representation, origin paths, and kernel overlay construction",
    },
    PrimitiveOwnership {
        operation: Builtin::MapNotePitches,
        spelling: "map_note_pitches",
        hidden_information: "controlled traversal of contextual notes while preserving non-note facts and provenance",
    },
];

#[derive(Clone)]
struct BuiltinValue {
    builtin: Builtin,
    bound: Vec<Option<Value>>,
}

impl Builtin {
    fn named(name: &str) -> Option<Self> {
        let entry = BUILTIN_OWNERSHIP.iter().find(|entry| entry.spelling == name)?;
        debug_assert!(!entry.hidden_information.is_empty());
        Some(entry.operation)
    }

    fn parameters(self) -> Vec<Type> {
        match self {
            Self::Transpose => vec![Type::Interval, Type::Music],
            Self::Stretch => vec![Type::Ratio, Type::Music],
            Self::Retrograde => vec![Type::Music],
            Self::Invert => vec![Type::Pitch, Type::Music],
            Self::Shift => vec![Type::Duration, Type::Music],
            Self::Overlay => vec![Type::Music, Type::Music],
            Self::MapNotePitches => vec![Type::Function(vec![Type::Pitch], Box::new(Type::Pitch)), Type::Music],
        }
    }
}

/// A notation-first value retained until a voice supplies scope and onset.
/// Its representation is crate-private by design: only the elaborator may
/// instantiate it, and consumers continue to see a closed kernel term.
#[derive(Clone)]
pub(crate) struct Music {
    pub(crate) items: Vec<VoiceItem>,
    pub(crate) uses: IndexMap<u64, Self>,
    pub(crate) pitches: Box<IndexMap<u64, PitchTerm>>,
    pub(crate) scales: Box<IndexMap<u64, crate::scale::Scale>>,
    pub(crate) bindings: IndexMap<String, crate::resolve::BoundValue>,
    pub(crate) role: Option<MusicRole>,
    pub(crate) definition_span: SourceSpan,
    pub(crate) operation: Option<Box<MusicOperation>>,
}

/// A written note's pitch, evaluated as far as a scale-free evaluator can
/// take it.
///
/// A pitch that reads the ambient scale has no answer while the `music` value
/// is being built: the same binding is used under `in scale c major` and
/// `in scale c dorian`, and freezing either would make saving a phrase change
/// what it means. So `step` stays a term here and is finished once per use,
/// when a voice supplies the scale in force. Everything that does not read the
/// scale is evaluated once, at the definition.
#[derive(Clone)]
pub(crate) enum PitchTerm {
    /// Already a pitch: no scale was read.
    Written(WrittenPitch),
    /// `base step n` — `n` scale steps from `base`, signed.
    Stepped { base: Box<Self>, steps: i64 },
    /// `base up i` / `base down i` over a base that still reads the scale.
    Moved {
        base: Box<Self>,
        interval: Interval,
        down: bool,
    },
}

/// Why a deferred pitch could not be finished under the scale in force.
pub(crate) enum PitchTermError {
    /// `step` was written where no scale is in force.
    NoScale,
    /// `step` was written from a pitch the scale in force does not contain.
    NotInScale {
        pitch: WrittenPitch,
        scale: crate::scale::Scale,
    },
    /// The written coordinates left the range exact arithmetic covers.
    OutOfRange,
}

impl PitchTerm {
    /// Whether finishing this term needs a scale.
    pub(crate) fn reads_scale(&self) -> bool {
        match self {
            Self::Written(_) => false,
            Self::Stepped { .. } => true,
            Self::Moved { base, .. } => base.reads_scale(),
        }
    }

    /// Finish the term under the scale in force, if one is.
    pub(crate) fn resolve(&self, scale: Option<crate::scale::Scale>) -> Result<WrittenPitch, PitchTermError> {
        match self {
            Self::Written(pitch) => Ok(*pitch),
            Self::Stepped { base, steps } => {
                let from = base.resolve(scale)?;
                let scale = scale.ok_or(PitchTermError::NoScale)?;
                let frame = crate::scale::Frame::around(scale, from).ok_or(PitchTermError::OutOfRange)?;
                let degree = frame
                    .locate(from)
                    .ok_or(PitchTermError::NotInScale { pitch: from, scale })?;
                let moved = degree.step(*steps).ok_or(PitchTermError::OutOfRange)?;
                frame.pitch(moved).ok_or(PitchTermError::OutOfRange)
            }
            Self::Moved { base, interval, down } => {
                let from = base.resolve(scale)?;
                let interval = if *down {
                    interval.inverse().ok_or(PitchTermError::OutOfRange)?
                } else {
                    *interval
                };
                from.transpose(interval).ok_or(PitchTermError::OutOfRange)
            }
        }
    }
}

impl std::fmt::Display for PitchTerm {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Written(pitch) => write!(out, "{pitch}"),
            Self::Stepped { base, steps } => write!(out, "{base}step{steps}"),
            Self::Moved { base, interval, down } => {
                write!(out, "{base}{}{:?}", if *down { "-" } else { "+" }, interval)
            }
        }
    }
}

/// Opaque contextual constructors. They are interpreted only when a voice
/// supplies scope and onset; no kernel occurrence is exposed as a value.
#[derive(Clone)]
pub(crate) enum MusicOperation {
    Transpose { interval: Interval, source: Music },
    Stretch { factor: Ratio<i64>, source: Music },
    Retrograde { source: Music },
    Invert { axis: WrittenPitch, source: Music },
    Shift { by: Ratio<i64>, source: Music },
    Overlay { left: Music, right: Box<Music> },
    MapNotePitches { mapper: PitchFunction, source: Music },
}

/// A checked total `pitch -> pitch` closure. Its representation stays inside
/// the elaboration core, so the controlled traversal cannot become a general
/// callback over score facts.
#[derive(Clone)]
pub(crate) struct PitchFunction(Box<Closure>);

pub(crate) fn apply_pitch_function(function: &PitchFunction, pitch: WrittenPitch) -> Option<WrittenPitch> {
    let mut meter = WorkMeter::default();
    let value = apply_closure(
        &function.0,
        vec![Some(Value::Pitch(pitch))],
        &mut meter,
        SourceSpan::default(),
    )?;
    let Value::Pitch(pitch) = value else {
        return None;
    };
    Some(pitch)
}

pub(crate) fn pitch_function_key(function: &PitchFunction) -> String {
    let closure = &function.0;
    let mut key = format!("{}:{}", closure.body.span.start, closure.body.span.end);
    for (name, value) in &closure.captures {
        use std::fmt::Write as _;
        let _ = write!(key, "|{name}={}", value.normalization_witness());
    }
    key
}

#[derive(Clone)]
pub(crate) struct MusicRole {
    pub(crate) name: String,
    pub(crate) material: crate::resolve::Material,
    pub(crate) foreign: bool,
}

impl Music {
    pub(crate) fn music_at(&self, span: SourceSpan) -> Option<&Self> {
        self.uses.get(&span_key(span))
    }

    pub(crate) fn pitch_at(&self, span: SourceSpan) -> Option<&PitchTerm> {
        self.pitches.get(&span_key(span))
    }

    pub(crate) fn scale_at(&self, span: SourceSpan) -> Option<crate::scale::Scale> {
        self.scales.get(&span_key(span)).copied()
    }
}

/// Checked root `use` expressions. This is the only bridge from the total
/// value evaluator into contextual score elaboration.
pub(crate) struct Program {
    uses: IndexMap<u64, Music>,
    pitches: IndexMap<u64, PitchTerm>,
    scales: IndexMap<u64, crate::scale::Scale>,
    named_music: IndexMap<String, Music>,
    #[cfg(test)]
    values: IndexMap<String, Value>,
}

impl Program {
    pub(crate) fn root_music(&self) -> Music {
        Music {
            items: Vec::new(),
            uses: self.uses.clone(),
            pitches: Box::new(self.pitches.clone()),
            scales: Box::new(self.scales.clone()),
            bindings: IndexMap::new(),
            role: None,
            definition_span: SourceSpan::default(),
            operation: None,
        }
    }

    pub(crate) fn named_music_values(&self) -> IndexMap<String, Music> {
        self.named_music.clone()
    }
}

#[derive(Clone)]
struct Closure {
    parameters: Vec<CheckedParameter>,
    result: Type,
    body: Expr,
    captures: IndexMap<String, Value>,
}

impl Value {
    fn ty(&self) -> Type {
        match self {
            Self::Bool(_) => Type::Bool,
            Self::Nat(_) => Type::Nat,
            Self::Ratio(_) => Type::Ratio,
            Self::Duration(_) => Type::Duration,
            Self::Pitch(_) => Type::Pitch,
            Self::PitchClass(_) => Type::PitchClass,
            Self::Interval(_) => Type::Interval,
            Self::Scale(_) => Type::Scale,
            Self::Key(_) => Type::Key,
            Self::Degree(_) => Type::Degree,
            Self::Frame(_) => Type::Frame,
            Self::Product(members) => Type::Product(members.iter().map(Self::ty).collect()),
            Self::Option { member, .. } => Type::Option(Box::new(member.clone())),
            Self::List { member, .. } => Type::List(Box::new(member.clone())),
            Self::Music(_) => Type::Music,
            Self::Closure(closure) => Type::Function(
                closure
                    .parameters
                    .iter()
                    .map(|parameter| parameter.ty.clone())
                    .collect(),
                Box::new(closure.result.clone()),
            ),
            Self::Builtin(value) => {
                let parameters = value
                    .builtin
                    .parameters()
                    .into_iter()
                    .enumerate()
                    .filter_map(|(index, ty)| value.bound.get(index).is_none_or(Option::is_none).then_some(ty))
                    .collect();
                Type::Function(parameters, Box::new(Type::Music))
            }
        }
    }

    /// Read the whole finite value into a deterministic witness.  The result
    /// is not a semantic hash or cache key; it makes the executable
    /// normalization check traverse, rather than merely construct, the value
    /// before the private environment is discarded by this prompt's caller.
    fn normalization_witness(&self) -> u64 {
        match self {
            Self::Bool(value) => u64::from(*value),
            Self::Nat(value) => *value,
            Self::Ratio(value) | Self::Duration(value) => {
                value.numer().unsigned_abs().rotate_left(7) ^ value.denom().unsigned_abs()
            }
            Self::Pitch(value) => {
                u64::from(value.letter.steps().unsigned_abs()).rotate_left(8)
                    ^ u64::from(value.accidental.0.unsigned_abs()).rotate_left(4)
                    ^ u64::from(value.octave.unsigned_abs())
            }
            Self::PitchClass(value) => {
                u64::from(value.letter.steps().unsigned_abs()).rotate_left(8)
                    ^ u64::from(value.accidental.0.unsigned_abs())
            }
            Self::Interval(value) => {
                value.diatonic_steps.unsigned_abs().rotate_left(8) ^ value.semitones.unsigned_abs()
            }
            Self::Scale(value) => scale_witness(*value),
            Self::Key(value) => {
                Self::PitchClass(value.tonic()).normalization_witness().rotate_left(2)
                    ^ u64::from(value.mode() == crate::Mode::Minor)
            }
            Self::Degree(value) => value.ordinal().unsigned_abs().rotate_left(8) ^ value.alteration().unsigned_abs(),
            Self::Frame(value) => {
                scale_witness(value.scale()).rotate_left(5) ^ Self::Pitch(value.tonic()).normalization_witness()
            }
            Self::Product(members) => members.iter().fold(0u64, |witness, member| {
                witness.rotate_left(5) ^ member.normalization_witness()
            }),
            Self::Option { value, .. } => value.as_deref().map_or(0, Self::normalization_witness).rotate_left(1),
            Self::List { values, .. } => values.iter().fold(0u64, |witness, value| {
                witness.rotate_left(5) ^ value.normalization_witness()
            }),
            Self::Music(music) => music_witness(music),
            Self::Closure(closure) => closure.captures.values().fold(
                u64::try_from(closure.parameters.len()).unwrap_or(u64::MAX),
                |witness, captured| witness.rotate_left(5) ^ captured.normalization_witness(),
            ),
            Self::Builtin(value) => value.bound.iter().flatten().fold(0, |witness, value| {
                witness.rotate_left(5) ^ value.normalization_witness()
            }),
        }
    }
}

/// Read a scale into a witness the same way the other finite values are read.
fn scale_witness(scale: crate::scale::Scale) -> u64 {
    Value::PitchClass(scale.tonic()).normalization_witness().rotate_left(3)
        ^ u64::try_from(scale.collection() as usize).unwrap_or(0)
}

fn music_witness(music: &Music) -> u64 {
    let base = u64::try_from(music.items.len()).unwrap_or(u64::MAX);
    let operation = match music.operation.as_deref() {
        None => 0,
        Some(
            MusicOperation::Transpose { source, .. }
            | MusicOperation::Stretch { source, .. }
            | MusicOperation::Retrograde { source }
            | MusicOperation::Invert { source, .. }
            | MusicOperation::Shift { source, .. }
            | MusicOperation::MapNotePitches { source, .. },
        ) => music_witness(source),
        Some(MusicOperation::Overlay { left, right }) => music_witness(left).rotate_left(7) ^ music_witness(right),
    };
    base.rotate_left(3) ^ operation
}

fn check_and_evaluate(
    resolver: &mut Resolver,
    declarations: impl Iterator<Item = SurfaceDefinition>,
    root: Option<&SyntaxNode>,
    unknown_root_music: UnknownRootMusic,
) -> Option<Program> {
    let root_uses = root.map(root_uses).unwrap_or_default();
    let mut meter = WorkMeter::default();
    let mut raw = Vec::new();
    let mut names: IndexMap<String, (SourceSpan, bool, bool)> = IndexMap::new();
    for declaration in declarations {
        let is_legacy = matches!(declaration, SurfaceDefinition::Legacy { .. });
        let (name, name_span, span, source) = surface_identity(&declaration)?;
        let foreign = source.is_some();
        if let Some((first, first_is_foreign, first_is_legacy)) = names.get(&name).copied() {
            // The structural resolver retains the established, role-specific
            // diagnostic for two legacy material declarations. The core must
            // still see only the first definition, but reporting here as well
            // would turn one source mistake into two diagnostics.
            if is_legacy && first_is_legacy {
                continue;
            }
            let mut diagnostic = Diagnostic::error(Code::DuplicateName, format!("`{name}` is bound twice"))
                .at(name_span, "bound again here");
            if !first_is_foreign {
                diagnostic = diagnostic.also(first, "first bound here");
            }
            resolver.report(diagnostic.help("give one of the bindings a different name"));
            continue;
        }
        names.insert(name.clone(), (name_span, foreign, is_legacy));
        if let Some(definition) = lower_signature(resolver, declaration, name, name_span, span, source) {
            raw.push(definition);
        }
    }

    let mut symbols = IndexMap::new();
    for (index, definition) in raw.iter().enumerate() {
        symbols.insert(
            definition.name.clone(),
            Symbol {
                ty: definition.ty.clone(),
                kind: definition.name_kind(),
                definition: index,
                external_declaration: definition.source.as_ref().map(|uri| crate::resolve::SourceLocation {
                    uri: uri.clone(),
                    span: definition.name_span,
                }),
            },
        );
        if !definition.foreign && definition.role.is_none() {
            resolver
                .references
                .declare(definition.name_kind(), &definition.name, definition.name_span);
        }
    }

    let mut checked = Vec::with_capacity(raw.len());
    let mut type_errors = false;
    for definition in &raw {
        let mut checker = Checker {
            resolver,
            definitions: &raw,
            symbols: &symbols,
            locals: IndexMap::new(),
            dependencies: IndexMap::new(),
            foreign: definition.foreign,
            failed: false,
            meter: &mut meter,
            music_role: definition.role.clone(),
            deferred_pitch: false,
            definition_span: definition.span,
        };
        let kind = match &definition.kind {
            RawDefinitionKind::Let { body } => checker
                .check(body, Some(&definition.ty))
                .map(|body| CheckedDefinitionKind::Let { body }),
            RawDefinitionKind::Function { parameters, body } => {
                let mut checked_parameters = Vec::with_capacity(parameters.len());
                let mut duplicate_parameters = IndexMap::<String, SourceSpan>::new();
                for parameter in parameters {
                    if let Some(first) = duplicate_parameters.get(&parameter.name).copied() {
                        checker.resolver.report(
                            Diagnostic::error(
                                Code::DuplicateName,
                                format!("parameter `{}` is bound twice", parameter.name),
                            )
                            .at(parameter.span, "bound again here")
                            .also(first, "first bound here"),
                        );
                        checker.failed = true;
                        continue;
                    }
                    duplicate_parameters.insert(parameter.name.clone(), parameter.span);
                    let default = parameter.default.as_ref().and_then(|default| match default {
                        RawDefault::Expression(expression) => checker.check(expression, Some(&parameter.ty)),
                        RawDefault::Value(value) if value.ty() == parameter.ty => Some(Expr {
                            kind: ExprKind::Literal(value.as_ref().clone()),
                            ty: parameter.ty.clone(),
                            span: parameter.span,
                        }),
                        RawDefault::Value(_) => None,
                    });
                    checker.locals.insert(parameter.name.clone(), parameter.ty.clone());
                    checked_parameters.push(CheckedParameter {
                        name: parameter.name.clone(),
                        ty: parameter.ty.clone(),
                        default,
                    });
                }
                checker
                    .check(body, function_result(&definition.ty))
                    .map(|body| CheckedDefinitionKind::Function {
                        parameters: checked_parameters,
                        body,
                    })
            }
            RawDefinitionKind::Music {
                parameters,
                body,
                callable,
            } => {
                if *callable {
                    let mut checked_parameters = Vec::with_capacity(parameters.len());
                    for parameter in parameters {
                        let default = parameter.default.as_ref().and_then(|default| match default {
                            RawDefault::Expression(expression) => checker.check(expression, Some(&parameter.ty)),
                            RawDefault::Value(value) if value.ty() == parameter.ty => Some(Expr {
                                kind: ExprKind::Literal(value.as_ref().clone()),
                                ty: parameter.ty.clone(),
                                span: parameter.span,
                            }),
                            RawDefault::Value(_) => None,
                        });
                        checker.locals.insert(parameter.name.clone(), parameter.ty.clone());
                        checked_parameters.push(CheckedParameter {
                            name: parameter.name.clone(),
                            ty: parameter.ty.clone(),
                            default,
                        });
                    }
                    checker
                        .music_expression(body)
                        .map(|body| CheckedDefinitionKind::Function {
                            parameters: checked_parameters,
                            body,
                        })
                } else {
                    checker
                        .music_expression(body)
                        .map(|body| CheckedDefinitionKind::Let { body })
                }
            }
        };
        type_errors |= checker.failed || kind.is_none();
        if let Some(kind) = kind {
            checked.push(CheckedDefinition {
                name: definition.name.clone(),
                ty: definition.ty.clone(),
                kind,
                dependencies: checker.dependencies,
                span: definition.span,
                foreign: definition.foreign,
            });
        }
    }
    if meter.exhaustion().is_some() {
        report_exhaustion(resolver, &meter);
        return None;
    }
    if type_errors || checked.len() != raw.len() {
        return None;
    }

    let order = dependency_order(resolver, &checked)?;
    let values = evaluate(resolver, &checked, &order, &mut meter)?;
    let mut uses = IndexMap::new();
    for statement in root_uses {
        let expression = child_of(&statement, is_expr_node)?;
        let span = crate::resolve::trimmed_span(&statement);
        if first_name(&expression).is_some_and(|name| !symbols.contains_key(&name) && Builtin::named(&name).is_none()) {
            match unknown_root_music {
                UnknownRootMusic::Reject => {}
                UnknownRootMusic::Defer => continue,
                UnknownRootMusic::Silent => {
                    uses.insert(
                        span_key(span),
                        Music {
                            items: Vec::new(),
                            uses: IndexMap::new(),
                            pitches: Box::default(),
                            scales: Box::default(),
                            bindings: IndexMap::new(),
                            role: None,
                            definition_span: span,
                            operation: None,
                        },
                    );
                    continue;
                }
            }
        }
        let mut checker = Checker {
            resolver,
            definitions: &raw,
            symbols: &symbols,
            locals: IndexMap::new(),
            dependencies: IndexMap::new(),
            foreign: false,
            failed: false,
            meter: &mut meter,
            music_role: None,
            definition_span: span,
            deferred_pitch: false,
        };
        let checked_use = checker.check(&expression, Some(&Type::Music))?;
        let Value::Music(music) = eval(&checked_use, &values, &mut meter)? else {
            return None;
        };
        uses.insert(span_key(span), music);
    }
    // `in scale` and note pitches written among a piece's own items, rather
    // than inside a definition. `music { ... }` collects its own; these are
    // what is left, and the elaborator reads both through the same root value.
    let mut scales = IndexMap::new();
    let mut pitches = IndexMap::new();
    if let Some(root) = root {
        for statement in root_nodes(root, SyntaxKind::InScaleStmt) {
            let Some(expression) = child_of(&statement, is_expr_node) else {
                continue;
            };
            let span = crate::resolve::trimmed_span(&statement);
            let mut checker = root_checker(resolver, &raw, &symbols, &mut meter, span);
            let checked = checker.check(&expression, Some(&Type::Scale))?;
            let Value::Scale(scale) = eval(&checked, &values, &mut meter)? else {
                return None;
            };
            scales.insert(span_key(span), scale);
        }
        for statement in root_nodes(root, SyntaxKind::NoteStmt) {
            let Some(expression) =
                musa_language::ast::NoteStmt::cast(statement.clone()).and_then(|note| note.pitch_expr())
            else {
                continue;
            };
            let span = crate::resolve::trimmed_span(&statement);
            let mut checker = root_checker(resolver, &raw, &symbols, &mut meter, span);
            let checked = checker.deferring_pitch(|checker| checker.check(&expression, Some(&Type::Pitch)))?;
            pitches.insert(span_key(span), pitch_term(&checked, &values, &mut meter)?);
        }
    }
    if meter.exhaustion().is_some() {
        report_exhaustion(resolver, &meter);
        return None;
    }
    let named_music = values
        .iter()
        .filter_map(|(name, value)| match value {
            Value::Music(music) => Some((name.clone(), music.clone())),
            Value::Bool(_)
            | Value::Nat(_)
            | Value::Ratio(_)
            | Value::Duration(_)
            | Value::Pitch(_)
            | Value::PitchClass(_)
            | Value::Interval(_)
            | Value::Scale(_)
            | Value::Key(_)
            | Value::Degree(_)
            | Value::Frame(_)
            | Value::Product(_)
            | Value::Option { .. }
            | Value::List { .. }
            | Value::Closure(_)
            | Value::Builtin(_) => None,
        })
        .collect();
    Some(Program {
        uses,
        pitches,
        scales,
        named_music,
        #[cfg(test)]
        values,
    })
}

/// A checker for one expression written among a piece's own items.
fn root_checker<'a>(
    resolver: &'a mut Resolver,
    definitions: &'a [RawDefinition],
    symbols: &'a IndexMap<String, Symbol>,
    meter: &'a mut WorkMeter,
    span: SourceSpan,
) -> Checker<'a> {
    Checker {
        resolver,
        definitions,
        symbols,
        locals: IndexMap::new(),
        dependencies: IndexMap::new(),
        foreign: false,
        failed: false,
        meter,
        music_role: None,
        definition_span: span,
        deferred_pitch: false,
    }
}

#[derive(Clone, Copy)]
enum UnknownRootMusic {
    Reject,
    Defer,
    Silent,
}

pub(crate) fn check_piece_for_kernel(
    resolver: &mut Resolver,
    piece: &musa_language::ast::PieceDecl,
) -> Option<Program> {
    check_and_evaluate(
        resolver,
        declarations(piece.syntax(), None).into_iter(),
        Some(piece.syntax()),
        UnknownRootMusic::Silent,
    )
}

fn surface_identity(definition: &SurfaceDefinition) -> Option<(String, SourceSpan, SourceSpan, Option<String>)> {
    let (syntax, name, source) = match definition {
        SurfaceDefinition::Let { declaration, source } => (declaration.syntax(), declaration.name(), source.clone()),
        SurfaceDefinition::Function { declaration, source } => {
            (declaration.syntax(), declaration.name(), source.clone())
        }
        SurfaceDefinition::Legacy {
            name, syntax, source, ..
        } => (syntax, Some(name.clone()), source.clone()),
    };
    let name = name?;
    let name_span = crate::resolve::token_span(syntax, SyntaxKind::Identifier)?;
    Some((name, name_span, crate::resolve::trimmed_span(syntax), source))
}

fn lower_signature(
    resolver: &mut Resolver,
    definition: SurfaceDefinition,
    name: String,
    name_span: SourceSpan,
    span: SourceSpan,
    source: Option<String>,
) -> Option<RawDefinition> {
    let foreign = source.is_some();
    match definition {
        SurfaceDefinition::Let { declaration, .. } => {
            let ty_node = child_of(declaration.syntax(), is_type_node)?;
            let body = child_of(declaration.syntax(), is_expr_node)?;
            let ty = parse_type(resolver, &ty_node)?;
            Some(RawDefinition {
                name,
                ty,
                kind: RawDefinitionKind::Let { body },
                name_span,
                span,
                foreign,
                source,
                role: None,
            })
        }
        SurfaceDefinition::Function { declaration, .. } => {
            let mut parameters = Vec::new();
            for parameter in declaration.params() {
                let Some(ty_node) = child_of(parameter.syntax(), is_type_node) else {
                    continue;
                };
                let Some(parameter_ty) = parse_type(resolver, &ty_node) else {
                    continue;
                };
                let parameter_name = parameter.name().unwrap_or_default();
                let parameter_span = crate::resolve::token_span(parameter.syntax(), SyntaxKind::Identifier)
                    .unwrap_or_else(|| crate::resolve::trimmed_span(parameter.syntax()));
                parameters.push(RawParameter {
                    name: parameter_name,
                    ty: parameter_ty,
                    default: child_of(parameter.syntax(), is_expr_node).map(RawDefault::Expression),
                    span: parameter_span,
                });
            }
            let result_node = declaration
                .syntax()
                .children()
                .filter(|node| is_type_node(node.kind()))
                .last()?;
            let result = parse_type(resolver, &result_node)?;
            let body = child_of(declaration.syntax(), is_expr_node)?;
            let ty = Type::Function(
                parameters.iter().map(|parameter| parameter.ty.clone()).collect(),
                Box::new(result),
            );
            Some(RawDefinition {
                name,
                ty,
                kind: RawDefinitionKind::Function { parameters, body },
                name_span,
                span,
                foreign,
                source,
                role: None,
            })
        }
        SurfaceDefinition::Legacy {
            syntax,
            parameters,
            material,
            ..
        } => {
            let mut raw_parameters = Vec::with_capacity(parameters.len());
            for parameter in parameters {
                let ty = match parameter.kind.as_str() {
                    "pitch" => Type::Pitch,
                    "duration" => Type::Duration,
                    other => {
                        resolver.report(
                            Diagnostic::error(Code::UnknownName, format!("unknown type `{other}`"))
                                .at(span, "not a motif parameter type"),
                        );
                        return None;
                    }
                };
                let default = parameter
                    .default
                    .as_deref()
                    .and_then(|written| legacy_default(&ty, written))
                    .map(|value| RawDefault::Value(Box::new(value)));
                raw_parameters.push(RawParameter {
                    name: parameter.name,
                    ty,
                    default,
                    span,
                });
            }
            let callable = material == crate::resolve::Material::Motif;
            let ty = if callable {
                Type::Function(
                    raw_parameters.iter().map(|parameter| parameter.ty.clone()).collect(),
                    Box::new(Type::Music),
                )
            } else {
                Type::Music
            };
            Some(RawDefinition {
                name: name.clone(),
                ty,
                kind: RawDefinitionKind::Music {
                    parameters: raw_parameters,
                    body: syntax,
                    callable,
                },
                name_span,
                span,
                foreign,
                source,
                role: Some(MusicRole {
                    name,
                    material,
                    foreign,
                }),
            })
        }
    }
}

fn legacy_default(ty: &Type, written: &str) -> Option<Value> {
    match ty {
        Type::Pitch => WrittenPitch::parse(written).map(Value::Pitch),
        Type::Duration => crate::resolve::parse_ratio(written)
            .or_else(|| written.parse::<i64>().ok().map(Ratio::from_integer))
            .map(Value::Duration),
        Type::Unit
        | Type::Bool
        | Type::Nat
        | Type::Ratio
        | Type::PitchClass
        | Type::Interval
        | Type::Scale
        | Type::Key
        | Type::Degree
        | Type::Frame
        | Type::Product(_)
        | Type::Option(_)
        | Type::List(_)
        | Type::Music
        | Type::Function(_, _) => None,
    }
}

fn function_result(ty: &Type) -> Option<&Type> {
    match ty {
        Type::Function(_, result) => Some(result),
        Type::Unit
        | Type::Bool
        | Type::Nat
        | Type::Ratio
        | Type::Duration
        | Type::Pitch
        | Type::PitchClass
        | Type::Interval
        | Type::Scale
        | Type::Key
        | Type::Degree
        | Type::Frame
        | Type::Music
        | Type::Product(_)
        | Type::Option(_)
        | Type::List(_) => None,
    }
}

fn parse_type(resolver: &mut Resolver, node: &SyntaxNode) -> Option<Type> {
    let kind = node.kind();
    if kind == SyntaxKind::TypeExpr {
        return child_of(node, is_type_node).and_then(|child| parse_type(resolver, &child));
    }
    if kind == SyntaxKind::TypeName {
        let text = node.to_string();
        let text = text.trim();
        return match text {
            "unit" => Some(Type::Unit),
            "bool" => Some(Type::Bool),
            "nat" => Some(Type::Nat),
            "ratio" => Some(Type::Ratio),
            "duration" => Some(Type::Duration),
            "pitch" => Some(Type::Pitch),
            "pitchclass" => Some(Type::PitchClass),
            "interval" => Some(Type::Interval),
            "scale" => Some(Type::Scale),
            "key" => Some(Type::Key),
            "degree" => Some(Type::Degree),
            "frame" => Some(Type::Frame),
            "music" => Some(Type::Music),
            _ => {
                resolver.report(
                    Diagnostic::error(Code::UnknownName, format!("unknown type `{text}`"))
                        .at(crate::resolve::trimmed_span(node), "not a value type")
                        .help("use `bool`, `nat`, `ratio`, `duration`, `pitch`, `pitchclass`, `interval`, `scale`, `key`, `degree`, `frame`, a product, or a function type"),
                );
                None
            }
        };
    }
    if kind == SyntaxKind::ProductType {
        let members: Option<Vec<_>> = node
            .children()
            .filter(|child| is_type_node(child.kind()))
            .map(|child| parse_type(resolver, &child))
            .collect();
        return members.map(Type::Product);
    }
    if kind == SyntaxKind::FunctionType {
        let mut parts = node.children().filter(|child| is_type_node(child.kind()));
        let parameter = parts.next().and_then(|part| parse_type(resolver, &part))?;
        let result = parts.next().and_then(|part| parse_type(resolver, &part))?;
        return Some(Type::Function(vec![parameter], Box::new(result)));
    }
    if matches!(kind, SyntaxKind::OptionType | SyntaxKind::ListType) {
        let member = child_of(node, is_type_node).and_then(|child| parse_type(resolver, &child))?;
        return if kind == SyntaxKind::OptionType {
            Some(Type::Option(Box::new(member)))
        } else {
            Some(Type::List(Box::new(member)))
        };
    }
    None
}

struct Checker<'a> {
    resolver: &'a mut Resolver,
    definitions: &'a [RawDefinition],
    symbols: &'a IndexMap<String, Symbol>,
    locals: IndexMap<String, Type>,
    dependencies: IndexMap<String, SourceSpan>,
    foreign: bool,
    failed: bool,
    meter: &'a mut WorkMeter,
    music_role: Option<MusicRole>,
    definition_span: SourceSpan,
    /// Whether the expression being checked is a note's pitch inside music,
    /// which is the one place a scale is supplied later rather than now.
    deferred_pitch: bool,
}

impl Checker<'_> {
    fn check(&mut self, node: &SyntaxNode, expected: Option<&Type>) -> Option<Expr> {
        let span = crate::resolve::trimmed_span(node);
        let kind = node.kind();
        let checked = if kind == SyntaxKind::ParenExpr {
            child_of(node, is_expr_node).and_then(|child| self.check(&child, expected))
        } else if kind == SyntaxKind::LiteralExpr {
            self.literal(node, expected)
        } else if kind == SyntaxKind::NameExpr {
            let named = self.name(node)?;
            if expected == Some(&Type::Music)
                && matches!(&named.ty, Type::Function(parameters, result) if parameters.is_empty() && result.as_ref() == &Type::Music)
            {
                Some(Expr {
                    kind: ExprKind::Apply {
                        function: Box::new(named),
                        arguments: Vec::new(),
                    },
                    ty: Type::Music,
                    span,
                })
            } else {
                Some(named)
            }
        } else if kind == SyntaxKind::ProductExpr {
            self.product(node, expected)
        } else if kind == SyntaxKind::ListExpr {
            self.list(node, expected)
        } else if kind == SyntaxKind::OptionExpr {
            self.option(node, expected)
        } else if kind == SyntaxKind::ApplyExpr {
            self.application(node, expected)
        } else if kind == SyntaxKind::PitchExpr {
            self.pitch_action(node)
        } else if kind == SyntaxKind::ScaleExpr {
            self.scale_literal(node)
        } else if kind == SyntaxKind::KeyExpr {
            self.key_literal(node)
        } else if kind == SyntaxKind::StepExpr {
            self.scale_step(node)
        } else if kind == SyntaxKind::MatchExpr {
            self.match_expression(node, expected)
        } else if kind == SyntaxKind::MusicExpr {
            self.music_expression(node)
        } else {
            None
        }?;
        if let Some(expected) = expected
            && checked.ty != *expected
        {
            self.resolver.report(
                Diagnostic::error(
                    Code::TypeMismatch,
                    format!("expected `{expected}`, found `{}`", checked.ty),
                )
                .at(span, format!("this has type `{}`", checked.ty)),
            );
            self.failed = true;
            return None;
        }
        Some(checked)
    }

    fn music_expression(&mut self, node: &SyntaxNode) -> Option<Expr> {
        let span = crate::resolve::trimmed_span(node);
        let items = music_items(node);
        let mut uses = Vec::new();
        for statement in owned_descendants(node, SyntaxKind::UseStmt) {
            let expression = child_of(&statement, is_expr_node)?;
            let checked = self.check(&expression, Some(&Type::Music))?;
            uses.push((crate::resolve::trimmed_span(&statement), checked));
        }

        let mut pitches = Vec::new();
        let mut scales = Vec::new();
        for statement in owned_descendants(node, SyntaxKind::InScaleStmt) {
            let Some(expression) = child_of(&statement, is_expr_node) else {
                continue;
            };
            let checked = self.check(&expression, Some(&Type::Scale))?;
            scales.push((crate::resolve::trimmed_span(&statement), checked));
        }

        if self.music_role.is_none() {
            for statement in owned_descendants(node, SyntaxKind::TempoStmt)
                .into_iter()
                .chain(owned_descendants(node, SyntaxKind::MeterStmt))
                .chain(owned_descendants(node, SyntaxKind::KeyStmt))
                .chain(owned_descendants(node, SyntaxKind::ClefStmt))
            {
                self.resolver.report(
                    Diagnostic::error(Code::Misplaced, "context changes cannot be stored in a `music` value")
                        .at(
                            crate::resolve::trimmed_span(&statement),
                            "this statement needs one absolute place",
                        )
                        .help("write the change among the voice's own items, before `use`")
                        .note("reusable music may read the context supplied at each use, but it cannot change the caller's context"),
                );
                self.failed = true;
            }
        }

        let mut bindings = IndexSet::new();
        for statement in owned_descendants(node, SyntaxKind::NoteStmt) {
            if let Some(note) = musa_language::ast::NoteStmt::cast(statement.clone()) {
                if let Some(expression) = note.pitch_expr() {
                    let checked = self.deferring_pitch(|checker| checker.check(&expression, Some(&Type::Pitch)))?;
                    pitches.push((crate::resolve::trimmed_span(&statement), checked));
                } else if let Some(name) = note.pitch().filter(|text| WrittenPitch::parse(text).is_none()) {
                    self.music_binding(
                        &name,
                        &Type::Pitch,
                        crate::resolve::trimmed_span(&statement),
                        &mut bindings,
                    )?;
                }
            }
            if let Some(name) = musa_language::ast::Duration::of(&statement).and_then(|duration| duration.parameter()) {
                self.music_binding(
                    &name,
                    &Type::Duration,
                    crate::resolve::trimmed_span(&statement),
                    &mut bindings,
                )?;
            }
        }
        for statement in owned_descendants(node, SyntaxKind::RestStmt)
            .into_iter()
            .chain(owned_descendants(node, SyntaxKind::ChordStmt))
        {
            if let Some(name) = musa_language::ast::Duration::of(&statement).and_then(|duration| duration.parameter()) {
                self.music_binding(
                    &name,
                    &Type::Duration,
                    crate::resolve::trimmed_span(&statement),
                    &mut bindings,
                )?;
            }
        }

        Some(Expr {
            kind: ExprKind::Music(CheckedMusic {
                items,
                uses,
                pitches,
                scales,
                bindings: bindings.into_iter().collect(),
                role: self.music_role.clone(),
                definition_span: self.definition_span,
            }),
            ty: Type::Music,
            span,
        })
    }

    /// Check a note's pitch, where `step` may read a scale supplied later.
    fn deferring_pitch<T>(&mut self, check: impl FnOnce(&mut Self) -> Option<T>) -> Option<T> {
        let outer = std::mem::replace(&mut self.deferred_pitch, true);
        let checked = check(self);
        self.deferred_pitch = outer;
        checked
    }

    /// `scale c dorian` — a tonic pitch class and a named collection.
    fn scale_literal(&mut self, node: &SyntaxNode) -> Option<Expr> {
        let span = crate::resolve::trimmed_span(node);
        let tonic = self.written_pitch_class(node, span)?;
        let word = significant_tokens(node)
            .filter(|token| token.kind() == SyntaxKind::Identifier)
            .last();
        let word = word.map(|token| token.text().to_owned()).unwrap_or_default();
        let Some(collection) = crate::scale::Collection::named(&word) else {
            self.resolver.report(
                Diagnostic::error(Code::UnknownName, format!("unknown collection `{word}`"))
                    .at(span, "not a named scale collection")
                    .help(format!("try one of: {}", collection_list()))
                    .note("a mode is a rotation of the diatonic collection; other collections are their own values"),
            );
            self.failed = true;
            return None;
        };
        Some(Expr {
            kind: ExprKind::Literal(Value::Scale(crate::scale::Scale::new(tonic, collection))),
            ty: Type::Scale,
            span,
        })
    }

    /// `key c minor` — the structural fact, read here as a value.
    fn key_literal(&mut self, node: &SyntaxNode) -> Option<Expr> {
        let span = crate::resolve::trimmed_span(node);
        let tonic = self.written_pitch_class(node, span)?;
        let word = significant_tokens(node)
            .filter(|token| token.kind() == SyntaxKind::Identifier)
            .last();
        let mode = match word.as_ref().map(|token| token.text()) {
            Some("major") => crate::Mode::Major,
            Some("minor") => crate::Mode::Minor,
            _ => {
                self.resolver.report(
                    Diagnostic::error(Code::NotAValue, "this key cannot be read")
                        .at(span, "expected a tonic and a mode, like `key a minor`")
                        .help("a key names a signature and a mode; a collection is written `scale a dorian`"),
                );
                self.failed = true;
                return None;
            }
        };
        Some(Expr {
            kind: ExprKind::Literal(Value::Key(crate::Key::new(tonic, mode))),
            ty: Type::Key,
            span,
        })
    }

    /// `p step n` / `p step down n` — a move along the scale in force.
    fn scale_step(&mut self, node: &SyntaxNode) -> Option<Expr> {
        let span = crate::resolve::trimmed_span(node);
        if !self.deferred_pitch {
            self.resolver.report(
                Diagnostic::error(Code::Misplaced, "`step` needs a scale in force")
                    .at(span, "no scale reaches this expression")
                    .help(
                        "write the note inside `in scale c major { ... }`, or move the expression into `music { ... }`",
                    )
                    .note("an absent scale is never an implicit C major: the coordinate has to come from somewhere"),
            );
            self.failed = true;
            return None;
        }
        let mut children = node.children().filter(|child| is_expr_node(child.kind()));
        let base = self.check(&children.next()?, Some(&Type::Pitch))?;
        let steps = self.check(&children.next()?, Some(&Type::Nat))?;
        let down = significant_tokens(node).any(|token| token.kind() == SyntaxKind::DownKw);
        Some(Expr {
            kind: ExprKind::Step {
                base: Box::new(base),
                steps: Box::new(steps),
                down,
            },
            ty: Type::Pitch,
            span,
        })
    }

    /// The single `PitchClass` node a `scale` or `key` literal spells.
    fn written_pitch_class(&mut self, node: &SyntaxNode, span: SourceSpan) -> Option<PitchClass> {
        let written = node
            .children()
            .find(|child| child.kind() == SyntaxKind::PitchClass)
            .map(|child| child.text().to_string());
        let parsed = written.as_deref().map(str::trim).and_then(PitchClass::parse);
        if parsed.is_none() {
            self.resolver.report(
                Diagnostic::error(Code::NotAValue, "this tonic cannot be read")
                    .at(span, "expected a spelled pitch class such as `bf` or `g#`"),
            );
            self.failed = true;
        }
        parsed
    }

    fn pitch_action(&mut self, node: &SyntaxNode) -> Option<Expr> {
        let span = crate::resolve::trimmed_span(node);
        let mut children = node.children().filter(|child| is_expr_node(child.kind()));
        let pitch = self.check(&children.next()?, Some(&Type::Pitch))?;
        let interval = self.check(&children.next()?, Some(&Type::Interval))?;
        let down = significant_tokens(node).any(|token| token.kind() == SyntaxKind::DownKw);
        Some(Expr {
            kind: ExprKind::PitchAction {
                pitch: Box::new(pitch),
                interval: Box::new(interval),
                down,
            },
            ty: Type::Pitch,
            span,
        })
    }

    fn music_binding(
        &mut self,
        name: &str,
        expected: &Type,
        span: SourceSpan,
        bindings: &mut IndexSet<String>,
    ) -> Option<()> {
        let found = if let Some(ty) = self.locals.get(name) {
            Some(ty.clone())
        } else if let Some(symbol) = self.symbols.get(name) {
            self.dependencies.entry(name.to_owned()).or_insert(span);
            if !self.foreign {
                self.resolver
                    .references
                    .record_use_from(symbol.kind, name, span, symbol.external_declaration.clone());
            }
            Some(symbol.ty.clone())
        } else {
            None
        };
        if found.as_ref() != Some(expected) {
            self.resolver.report(
                Diagnostic::error(Code::TypeMismatch, format!("`{name}` is not a `{expected}` value"))
                    .at(span, format!("this position needs `{expected}`")),
            );
            self.failed = true;
            return None;
        }
        bindings.insert(name.to_owned());
        Some(())
    }

    fn literal(&mut self, node: &SyntaxNode, expected: Option<&Type>) -> Option<Expr> {
        let token = significant_tokens(node).next()?;
        let span = crate::resolve::trimmed_span(node);
        let kind = token.kind();
        let value = if kind == SyntaxKind::TrueKw {
            Value::Bool(true)
        } else if kind == SyntaxKind::FalseKw {
            Value::Bool(false)
        } else if kind == SyntaxKind::Integer && expected == Some(&Type::Duration) {
            Value::Duration(Ratio::from_integer(parse_i64(self.resolver, &token)?))
        } else if kind == SyntaxKind::Integer && expected == Some(&Type::Ratio) {
            Value::Ratio(Ratio::from_integer(parse_i64(self.resolver, &token)?))
        } else if kind == SyntaxKind::Rational && expected == Some(&Type::Duration) {
            Value::Duration(parse_ratio(self.resolver, &token)?)
        } else if kind == SyntaxKind::Integer {
            Value::Nat(parse_u64(self.resolver, &token)?)
        } else if kind == SyntaxKind::Rational {
            Value::Ratio(parse_ratio(self.resolver, &token)?)
        } else if kind == SyntaxKind::PitchLiteral {
            Value::Pitch(WrittenPitch::parse(token.text()).or_else(|| {
                self.resolver.report(
                    Diagnostic::error(Code::NotAValue, format!("`{}` is not a pitch", token.text()))
                        .at(span, "invalid pitch literal"),
                );
                None
            })?)
        } else if kind == SyntaxKind::IntervalLiteral {
            Value::Interval(Interval::parse(token.text(), false).or_else(|| {
                self.resolver.report(
                    Diagnostic::error(Code::NotAValue, format!("`{}` is not an interval", token.text()))
                        .at(span, "invalid interval literal"),
                );
                None
            })?)
        } else {
            return None;
        };
        Some(Expr {
            ty: value.ty(),
            kind: ExprKind::Literal(value),
            span,
        })
    }

    fn name(&mut self, node: &SyntaxNode) -> Option<Expr> {
        let token = significant_tokens(node).find(|token| {
            matches!(
                token.kind(),
                SyntaxKind::Identifier
                    | SyntaxKind::RepeatKw
                    | SyntaxKind::TransposeKw
                    | SyntaxKind::StretchKw
                    | SyntaxKind::RetrogradeKw
                    | SyntaxKind::InvertKw
            )
        })?;
        let name = token.text().to_owned();
        let span = crate::resolve::trimmed_span(node);
        if let Some(ty) = self.locals.get(&name) {
            return Some(Expr {
                kind: ExprKind::Name(name),
                ty: ty.clone(),
                span,
            });
        }
        if let Some(builtin) = Builtin::named(&name) {
            let value = Value::Builtin(Box::new(BuiltinValue {
                builtin,
                bound: vec![None; builtin.parameters().len()],
            }));
            return Some(Expr {
                ty: value.ty(),
                kind: ExprKind::Literal(value),
                span,
            });
        }
        if primitive_named(&name).is_some() {
            self.resolver.report(
                Diagnostic::error(
                    Code::TypeMismatch,
                    format!("`{name}` is a compiler-owned polymorphic operation"),
                )
                .at(span, "apply it directly so Musa can choose one concrete type"),
            );
            self.failed = true;
            return None;
        }
        let Some(symbol) = self.symbols.get(&name) else {
            self.resolver.report(
                Diagnostic::error(Code::UnknownName, format!("cannot find `{name}`"))
                    .at(span, "nothing binds this name"),
            );
            self.failed = true;
            return None;
        };
        self.dependencies.entry(name.clone()).or_insert(span);
        if !self.foreign {
            self.resolver
                .references
                .record_use_from(symbol.kind, &name, span, symbol.external_declaration.clone());
        }
        Some(Expr {
            kind: ExprKind::Name(name),
            ty: symbol.ty.clone(),
            span,
        })
    }

    fn product(&mut self, node: &SyntaxNode, expected: Option<&Type>) -> Option<Expr> {
        let children: Vec<_> = node.children().filter(|child| is_expr_node(child.kind())).collect();
        let expected_members = match expected {
            Some(Type::Product(members)) if members.len() == children.len() => Some(members.as_slice()),
            _ => None,
        };
        let members: Option<Vec<_>> = children
            .iter()
            .enumerate()
            .map(|(index, child)| self.check(child, expected_members.and_then(|members| members.get(index))))
            .collect();
        let members = members?;
        Some(Expr {
            ty: Type::Product(members.iter().map(|member| member.ty.clone()).collect()),
            kind: ExprKind::Product(members),
            span: crate::resolve::trimmed_span(node),
        })
    }

    fn list(&mut self, node: &SyntaxNode, expected: Option<&Type>) -> Option<Expr> {
        let expected_member = match expected {
            Some(Type::List(member)) => Some(member.as_ref()),
            _ => None,
        };
        let children: Vec<_> = node.children().filter(|child| is_expr_node(child.kind())).collect();
        if children.is_empty() && expected_member.is_none() {
            self.resolver.report(
                Diagnostic::error(Code::TypeMismatch, "the element type of this empty list is unknown")
                    .at(crate::resolve::trimmed_span(node), "add a `list[...]` type annotation"),
            );
            self.failed = true;
            return None;
        }
        let mut values = Vec::with_capacity(children.len());
        let mut member = expected_member.cloned();
        for child in &children {
            let value = self.check(child, member.as_ref())?;
            if member.is_none() {
                member = Some(value.ty.clone());
            }
            values.push(value);
        }
        let member = member?;
        Some(Expr {
            kind: ExprKind::List(values),
            ty: Type::List(Box::new(member)),
            span: crate::resolve::trimmed_span(node),
        })
    }

    fn option(&mut self, node: &SyntaxNode, expected: Option<&Type>) -> Option<Expr> {
        let expected_member = match expected {
            Some(Type::Option(member)) => Some(member.as_ref()),
            _ => None,
        };
        let some = significant_tokens(node).any(|token| token.kind() == SyntaxKind::SomeKw);
        let value = child_of(node, is_expr_node);
        if !some && expected_member.is_none() {
            self.resolver.report(
                Diagnostic::error(Code::TypeMismatch, "the value type of `none` is unknown").at(
                    crate::resolve::trimmed_span(node),
                    "add an `option[...]` type annotation",
                ),
            );
            self.failed = true;
            return None;
        }
        let checked = if let Some(value) = value.as_ref() {
            Some(Box::new(self.check(value, expected_member)?))
        } else {
            None
        };
        let member = expected_member
            .cloned()
            .or_else(|| checked.as_ref().map(|value| value.ty.clone()))?;
        Some(Expr {
            kind: ExprKind::Option(checked),
            ty: Type::Option(Box::new(member)),
            span: crate::resolve::trimmed_span(node),
        })
    }

    fn match_expression(&mut self, node: &SyntaxNode, expected: Option<&Type>) -> Option<Expr> {
        let scrutinee_node = child_of(node, is_expr_node)?;
        let scrutinee = self.check(&scrutinee_node, None)?;
        let mut coverage = IndexSet::new();
        let mut catch_all = false;
        let mut result = expected.cloned();
        let mut arms = Vec::new();
        for arm in node.children().filter(|child| child.kind() == SyntaxKind::MatchArm) {
            let pattern_node = arm.children().find(|child| child.kind() == SyntaxKind::Pattern)?;
            let (pattern, covered, bindings) = self.check_pattern(&pattern_node, &scrutinee.ty)?;
            if catch_all || is_exhaustive(&scrutinee.ty, &coverage) || !coverage.insert(covered.clone()) {
                self.resolver.report(
                    Diagnostic::error(Code::UnreachablePattern, "this match arm can never be selected")
                        .at(crate::resolve::trimmed_span(&pattern_node), "already covered above"),
                );
                self.failed = true;
            }
            catch_all |= covered == Coverage::CatchAll;
            let saved = self.locals.clone();
            self.locals.extend(bindings);
            let body_node = child_of(&arm, is_expr_node)?;
            let body = self.check(&body_node, result.as_ref())?;
            self.locals = saved;
            if result.is_none() {
                result = Some(body.ty.clone());
            }
            arms.push(CheckedArm { pattern, body });
        }
        if !is_exhaustive(&scrutinee.ty, &coverage) {
            self.resolver.report(
                Diagnostic::error(Code::NonExhaustiveMatch, "this match leaves a possible value uncovered")
                    .at(
                        crate::resolve::trimmed_span(node),
                        "add the missing constructor or a `_` fallback",
                    )
                    .note(format!("the matched value has type `{}`", scrutinee.ty)),
            );
            self.failed = true;
            return None;
        }
        Some(Expr {
            kind: ExprKind::Match {
                scrutinee: Box::new(scrutinee),
                arms,
            },
            ty: result?,
            span: crate::resolve::trimmed_span(node),
        })
    }

    fn check_pattern(
        &mut self,
        node: &SyntaxNode,
        target: &Type,
    ) -> Option<(Pattern, Coverage, IndexMap<String, Type>)> {
        let tokens: Vec<_> = significant_tokens(node).collect();
        let first = tokens.first()?;
        let span = crate::resolve::trimmed_span(node);
        let mut bindings = IndexMap::new();
        if first.kind() == SyntaxKind::Identifier {
            if first.text() == "_" {
                return Some((Pattern::Wildcard, Coverage::CatchAll, bindings));
            }
            bindings.insert(first.text().to_owned(), target.clone());
            return Some((Pattern::Bind(first.text().to_owned()), Coverage::CatchAll, bindings));
        }
        if first.kind() == SyntaxKind::NoneKw {
            if !matches!(target, Type::Option(_)) {
                return self.pattern_type_error(span, target, "`none` needs an option");
            }
            return Some((Pattern::None, Coverage::None, bindings));
        }
        if first.kind() == SyntaxKind::SomeKw {
            let Type::Option(member) = target else {
                return self.pattern_type_error(span, target, "`some` needs an option");
            };
            let name = tokens
                .iter()
                .find(|token| token.kind() == SyntaxKind::Identifier)
                .map(|token| token.text().to_owned())?;
            bindings.insert(name.clone(), member.as_ref().clone());
            return Some((Pattern::Some(name), Coverage::Some, bindings));
        }
        if first.kind() == SyntaxKind::LBracket {
            let Type::List(member) = target else {
                return self.pattern_type_error(span, target, "a list pattern needs a list");
            };
            let names: Vec<_> = tokens
                .iter()
                .filter(|token| token.kind() == SyntaxKind::Identifier)
                .map(|token| token.text().to_owned())
                .collect();
            return match names.as_slice() {
                [] => Some((Pattern::EmptyList, Coverage::EmptyList, bindings)),
                [head, tail] => {
                    bindings.insert(head.clone(), member.as_ref().clone());
                    bindings.insert(tail.clone(), Type::List(member.clone()));
                    Some((
                        Pattern::Cons {
                            head: head.clone(),
                            tail: tail.clone(),
                        },
                        Coverage::Cons,
                        bindings,
                    ))
                }
                _ => self.pattern_type_error(span, target, "write `[]` or `[head, ..tail]`"),
            };
        }
        if first.kind() == SyntaxKind::LParen {
            let Type::Product(members) = target else {
                return self.pattern_type_error(span, target, "a product pattern needs a product");
            };
            let names: Vec<_> = tokens
                .iter()
                .filter(|token| token.kind() == SyntaxKind::Identifier)
                .map(|token| token.text().to_owned())
                .collect();
            if names.len() != members.len() {
                return self.pattern_type_error(span, target, "the product pattern has the wrong number of bindings");
            }
            for (name, member) in names.iter().zip(members) {
                if bindings.insert(name.clone(), member.clone()).is_some() {
                    self.resolver.report(
                        Diagnostic::error(Code::DuplicateName, format!("pattern binding `{name}` is repeated"))
                            .at(span, "bind each product member once"),
                    );
                    self.failed = true;
                    return None;
                }
            }
            return Some((Pattern::Product(names), Coverage::CatchAll, bindings));
        }
        let value = self.pattern_literal(first, target, span)?;
        let covered = match value {
            Value::Bool(true) => Coverage::True,
            Value::Bool(false) => Coverage::False,
            Value::Nat(_)
            | Value::Ratio(_)
            | Value::Duration(_)
            | Value::Pitch(_)
            | Value::PitchClass(_)
            | Value::Interval(_)
            | Value::Scale(_)
            | Value::Key(_)
            | Value::Degree(_)
            | Value::Frame(_)
            | Value::Product(_)
            | Value::Option { .. }
            | Value::List { .. }
            | Value::Music(_)
            | Value::Closure(_)
            | Value::Builtin(_) => Coverage::Literal(literal_key(&value)),
        };
        Some((Pattern::Literal(value), covered, bindings))
    }

    fn pattern_literal(&mut self, token: &SyntaxToken, target: &Type, span: SourceSpan) -> Option<Value> {
        let value = match (token.kind(), target) {
            (SyntaxKind::TrueKw, Type::Bool) => Value::Bool(true),
            (SyntaxKind::FalseKw, Type::Bool) => Value::Bool(false),
            (SyntaxKind::Integer, Type::Nat) => Value::Nat(parse_u64(self.resolver, token)?),
            (SyntaxKind::Integer, Type::Duration) => {
                Value::Duration(Ratio::from_integer(parse_i64(self.resolver, token)?))
            }
            (SyntaxKind::Rational, Type::Ratio) => Value::Ratio(parse_ratio(self.resolver, token)?),
            (SyntaxKind::Rational, Type::Duration) => Value::Duration(parse_ratio(self.resolver, token)?),
            (SyntaxKind::PitchLiteral, Type::Pitch) => Value::Pitch(WrittenPitch::parse(token.text())?),
            (SyntaxKind::IntervalLiteral, Type::Interval) => Value::Interval(Interval::parse(token.text(), false)?),
            _ => return self.pattern_type_error(span, target, "this literal cannot match that value type"),
        };
        Some(value)
    }

    fn pattern_type_error<T>(&mut self, span: SourceSpan, target: &Type, message: &str) -> Option<T> {
        self.resolver.report(
            Diagnostic::error(Code::TypeMismatch, message).at(span, format!("the matched value has type `{target}`")),
        );
        self.failed = true;
        None
    }

    fn application(&mut self, node: &SyntaxNode, expected: Option<&Type>) -> Option<Expr> {
        let mut children = node.children();
        let function_node = children.find(|child| is_expr_node(child.kind()))?;
        if let Some(primitive) = name_of(&function_node).as_deref().and_then(primitive_named) {
            return self.primitive_application(node, primitive, expected);
        }
        let function = self.check(&function_node, None)?;
        let Type::Function(parameter_types, result) = function.ty.clone() else {
            self.resolver.report(
                Diagnostic::error(Code::TypeMismatch, format!("`{}` is not callable", function.ty))
                    .at(function.span, "this is a value, not a function"),
            );
            self.failed = true;
            return None;
        };
        let raw_arguments = raw_arguments(node);
        let parameter_shape = self.parameter_shape(&function_node, &parameter_types);
        let mut occupied = IndexSet::new();
        let mut positional = 0usize;
        let mut arguments = Vec::with_capacity(raw_arguments.len());
        for argument in &raw_arguments {
            let named = argument_name(argument);
            let parameter = if let Some(named) = named {
                let Some(index) = parameter_shape
                    .iter()
                    .position(|parameter| parameter.name.as_deref() == Some(&named))
                else {
                    self.resolver.report(
                        Diagnostic::error(Code::WrongArity, format!("this function has no parameter `{named}`"))
                            .at(crate::resolve::trimmed_span(argument), "unknown named argument"),
                    );
                    self.failed = true;
                    continue;
                };
                index
            } else {
                while occupied.contains(&positional) {
                    positional = positional.saturating_add(1);
                }
                let index = positional;
                positional = positional.saturating_add(1);
                index
            };
            if parameter >= parameter_types.len() || !occupied.insert(parameter) {
                self.resolver.report(
                    Diagnostic::error(Code::WrongArity, "this call supplies the wrong arguments")
                        .at(crate::resolve::trimmed_span(argument), "extra or repeated argument"),
                );
                self.failed = true;
                continue;
            }
            let value_node = child_of(argument, is_expr_node)?;
            let value = self.check(&value_node, parameter_types.get(parameter))?;
            arguments.push(CallArgument { parameter, value });
        }
        let missing: Vec<_> = parameter_shape
            .iter()
            .enumerate()
            .filter(|(index, parameter)| !occupied.contains(index) && !parameter.has_default)
            .collect();
        if !missing.is_empty() {
            let names = missing
                .iter()
                .map(|(index, parameter)| {
                    parameter
                        .name
                        .clone()
                        .unwrap_or_else(|| format!("argument {}", index.saturating_add(1)))
                })
                .collect::<Vec<_>>()
                .join(", ");
            let legacy = name_of(&function_node)
                .and_then(|name| self.symbols.get(&name))
                .and_then(|symbol| self.definitions.get(symbol.definition))
                .and_then(|definition| definition.role.as_ref());
            if let Some(role) = legacy {
                self.resolver.report(
                    Diagnostic::error(
                        Code::NotAValue,
                        format!("{} `{}` needs a value for `{names}`", role.material.word(), role.name),
                    )
                    .at(crate::resolve::trimmed_span(node), "not enough arguments"),
                );
                self.failed = true;
                return None;
            }
        }
        let result_ty = if missing.is_empty() {
            result.as_ref().clone()
        } else {
            Type::Function(
                missing
                    .iter()
                    .filter_map(|(index, _)| parameter_types.get(*index).cloned())
                    .collect(),
                result,
            )
        };
        Some(Expr {
            kind: ExprKind::Apply {
                function: Box::new(function),
                arguments,
            },
            ty: result_ty,
            span: crate::resolve::trimmed_span(node),
        })
    }

    fn primitive_application(
        &mut self,
        node: &SyntaxNode,
        primitive: Primitive,
        expected: Option<&Type>,
    ) -> Option<Expr> {
        let span = crate::resolve::trimmed_span(node);
        let raw = raw_arguments(node);
        if raw.iter().any(|argument| argument_name(argument).is_some()) {
            self.resolver.report(
                Diagnostic::error(
                    Code::WrongArity,
                    format!("`{}` uses positional arguments", primitive.name()),
                )
                .at(span, "named arguments are not part of this prelude operation"),
            );
            self.failed = true;
            return None;
        }
        let wanted = match primitive {
            Primitive::NatFold | Primitive::ListFold | Primitive::OptionFold => 3,
            Primitive::Map
            | Primitive::Filter
            | Primitive::Repeat
            | Primitive::IntervalAdd
            | Primitive::ScaleOn
            | Primitive::ScalePitch
            | Primitive::PitchFrame
            | Primitive::FramePitch
            | Primitive::DegreeStepUp
            | Primitive::DegreeStepDown => 2,
            Primitive::Range
            | Primitive::IntervalInverse
            | Primitive::PitchClassOf
            | Primitive::SignatureScale
            | Primitive::ScaleTonic
            | Primitive::ScaleSize
            | Primitive::FrameScale
            | Primitive::FrameTonic
            | Primitive::DegreeOf
            | Primitive::DegreeRaised
            | Primitive::DegreeLowered => 1,
        };
        if raw.len() != wanted {
            self.resolver.report(
                Diagnostic::error(
                    Code::WrongArity,
                    format!("`{}` takes {wanted} arguments, found {}", primitive.name(), raw.len()),
                )
                .at(span, "wrong number of arguments"),
            );
            self.failed = true;
            return None;
        }
        if !self.meter.instantiate(primitive.name(), span) {
            self.failed = true;
            return None;
        }
        let nodes: Vec<_> = raw
            .iter()
            .filter_map(|argument| child_of(argument, is_expr_node))
            .collect();
        let (arguments, ty) = match primitive {
            Primitive::IntervalAdd => {
                let first = self.check(nodes.first()?, Some(&Type::Interval))?;
                let second = self.check(nodes.get(1)?, Some(&Type::Interval))?;
                (vec![first, second], Type::Interval)
            }
            Primitive::IntervalInverse => {
                let interval = self.check(nodes.first()?, Some(&Type::Interval))?;
                (vec![interval], Type::Interval)
            }
            Primitive::PitchClassOf => {
                let pitch = self.check(nodes.first()?, Some(&Type::Pitch))?;
                (vec![pitch], Type::PitchClass)
            }
            Primitive::SignatureScale => {
                let key = self.check(nodes.first()?, Some(&Type::Key))?;
                (vec![key], Type::Scale)
            }
            Primitive::ScaleOn => {
                let scale = self.check(nodes.first()?, Some(&Type::Scale))?;
                let tonic = self.check(nodes.get(1)?, Some(&Type::PitchClass))?;
                (vec![scale, tonic], Type::Scale)
            }
            Primitive::ScaleTonic => {
                let scale = self.check(nodes.first()?, Some(&Type::Scale))?;
                (vec![scale], Type::PitchClass)
            }
            Primitive::ScaleSize => {
                let scale = self.check(nodes.first()?, Some(&Type::Scale))?;
                (vec![scale], Type::Nat)
            }
            Primitive::ScalePitch => {
                let scale = self.check(nodes.first()?, Some(&Type::Scale))?;
                let pitch = self.check(nodes.get(1)?, Some(&Type::Pitch))?;
                (vec![scale, pitch], Type::Option(Box::new(Type::Degree)))
            }
            Primitive::PitchFrame => {
                let scale = self.check(nodes.first()?, Some(&Type::Scale))?;
                let tonic = self.check(nodes.get(1)?, Some(&Type::Pitch))?;
                (vec![scale, tonic], Type::Option(Box::new(Type::Frame)))
            }
            Primitive::FrameScale => {
                let frame = self.check(nodes.first()?, Some(&Type::Frame))?;
                (vec![frame], Type::Scale)
            }
            Primitive::FrameTonic => {
                let frame = self.check(nodes.first()?, Some(&Type::Frame))?;
                (vec![frame], Type::Pitch)
            }
            Primitive::FramePitch => {
                let frame = self.check(nodes.first()?, Some(&Type::Frame))?;
                let degree = self.check(nodes.get(1)?, Some(&Type::Degree))?;
                (vec![frame, degree], Type::Pitch)
            }
            Primitive::DegreeOf => {
                let ordinal = self.check(nodes.first()?, Some(&Type::Nat))?;
                (vec![ordinal], Type::Degree)
            }
            Primitive::DegreeStepUp | Primitive::DegreeStepDown => {
                let degree = self.check(nodes.first()?, Some(&Type::Degree))?;
                let steps = self.check(nodes.get(1)?, Some(&Type::Nat))?;
                (vec![degree, steps], Type::Degree)
            }
            Primitive::DegreeRaised | Primitive::DegreeLowered => {
                let degree = self.check(nodes.first()?, Some(&Type::Degree))?;
                (vec![degree], Type::Degree)
            }
            Primitive::Range => {
                let count = self.check(nodes.first()?, Some(&Type::Nat))?;
                (vec![count], Type::List(Box::new(Type::Nat)))
            }
            Primitive::Repeat => {
                let value = self.check(nodes.first()?, None)?;
                let count = self.check(nodes.get(1)?, Some(&Type::Nat))?;
                let result = Type::List(Box::new(value.ty.clone()));
                (vec![value, count], result)
            }
            Primitive::Map => {
                let function = self.check(nodes.first()?, None)?;
                let Type::Function(parameters, result) = function.ty.clone() else {
                    return self.primitive_type_error(span, "`map` first needs a one-argument function");
                };
                let Some(parameter) = unary_parameter(&parameters) else {
                    return self.primitive_type_error(span, "`map` first needs a one-argument function");
                };
                let values = self.check(nodes.get(1)?, Some(&Type::List(Box::new(parameter.clone()))))?;
                (vec![function, values], Type::List(result))
            }
            Primitive::Filter => {
                let function = self.check(nodes.first()?, None)?;
                let Type::Function(parameters, result) = function.ty.clone() else {
                    return self.primitive_type_error(span, "`filter` first needs a predicate");
                };
                let Some(parameter) = unary_parameter(&parameters) else {
                    return self.primitive_type_error(span, "`filter` first needs a one-argument predicate");
                };
                if result.as_ref() != &Type::Bool {
                    return self.primitive_type_error(span, "a `filter` predicate must return `bool`");
                }
                let values = self.check(nodes.get(1)?, Some(&Type::List(Box::new(parameter.clone()))))?;
                (vec![function, values], Type::List(Box::new(parameter.clone())))
            }
            Primitive::NatFold => {
                let zero = self.check(nodes.first()?, expected)?;
                let step = self.check(nodes.get(1)?, None)?;
                let wanted = Type::Function(vec![Type::Nat, zero.ty.clone()], Box::new(zero.ty.clone()));
                if step.ty != wanted {
                    return self.primitive_type_error(
                        step.span,
                        "a `nat_fold` step must accept the index and accumulator and return the accumulator type",
                    );
                }
                let count = self.check(nodes.get(2)?, Some(&Type::Nat))?;
                let result = zero.ty.clone();
                (vec![zero, step, count], result)
            }
            Primitive::ListFold => {
                let zero = self.check(nodes.first()?, expected)?;
                let step = self.check(nodes.get(1)?, None)?;
                let Type::Function(parameters, result) = step.ty.clone() else {
                    return self.primitive_type_error(step.span, "a `list_fold` step must be a two-argument function");
                };
                let Some((member, accumulator)) = binary_parameters(&parameters) else {
                    return self.primitive_type_error(step.span, "a `list_fold` step must be a two-argument function");
                };
                if accumulator != &zero.ty || result.as_ref() != &zero.ty {
                    return self
                        .primitive_type_error(step.span, "a `list_fold` step must preserve the accumulator type");
                }
                let values = self.check(nodes.get(2)?, Some(&Type::List(Box::new(member.clone()))))?;
                let result = zero.ty.clone();
                (vec![zero, step, values], result)
            }
            Primitive::OptionFold => {
                let zero = self.check(nodes.first()?, expected)?;
                let some_case = self.check(nodes.get(1)?, None)?;
                let Type::Function(parameters, result) = some_case.ty.clone() else {
                    return self.primitive_type_error(
                        some_case.span,
                        "an `option_fold` some-case must be a one-argument function",
                    );
                };
                let Some(member) = unary_parameter(&parameters) else {
                    return self.primitive_type_error(
                        some_case.span,
                        "an `option_fold` some-case must be a one-argument function",
                    );
                };
                if result.as_ref() != &zero.ty {
                    return self.primitive_type_error(
                        some_case.span,
                        "an `option_fold` some-case must return the zero value's type",
                    );
                }
                let value = self.check(nodes.get(2)?, Some(&Type::Option(Box::new(member.clone()))))?;
                let result = zero.ty.clone();
                (vec![zero, some_case, value], result)
            }
        };
        Some(Expr {
            kind: ExprKind::Primitive { primitive, arguments },
            ty,
            span,
        })
    }

    fn primitive_type_error<T>(&mut self, span: SourceSpan, message: &str) -> Option<T> {
        self.resolver
            .report(Diagnostic::error(Code::TypeMismatch, message).at(span, "invalid prelude arguments"));
        self.failed = true;
        None
    }

    fn parameter_shape(&self, function: &SyntaxNode, types: &[Type]) -> Vec<ParameterShape> {
        let global = name_of(function)
            .and_then(|name| self.symbols.get(&name))
            .and_then(|symbol| self.definitions.get(symbol.definition));
        match global.map(|definition| &definition.kind) {
            Some(RawDefinitionKind::Function { parameters, .. } | RawDefinitionKind::Music { parameters, .. }) => {
                parameters
                    .iter()
                    .map(|parameter| ParameterShape {
                        name: Some(parameter.name.clone()),
                        has_default: parameter.default.is_some(),
                    })
                    .collect()
            }
            _ => types
                .iter()
                .map(|_| ParameterShape {
                    name: None,
                    has_default: false,
                })
                .collect(),
        }
    }
}

struct ParameterShape {
    name: Option<String>,
    has_default: bool,
}

fn primitive_named(name: &str) -> Option<Primitive> {
    let entry = PRIMITIVE_OWNERSHIP.iter().find(|entry| entry.spelling == name)?;
    debug_assert!(!entry.hidden_information.is_empty());
    Some(entry.operation)
}

fn unary_parameter(parameters: &[Type]) -> Option<&Type> {
    match parameters {
        [parameter] => Some(parameter),
        _ => None,
    }
}

fn binary_parameters(parameters: &[Type]) -> Option<(&Type, &Type)> {
    match parameters {
        [first, second] => Some((first, second)),
        _ => None,
    }
}

fn raw_arguments(node: &SyntaxNode) -> Vec<SyntaxNode> {
    node.children()
        .find(|child| child.kind() == SyntaxKind::ExprArgList)
        .map_or_else(Vec::new, |list| {
            list.children()
                .filter(|child| child.kind() == SyntaxKind::ExprArg)
                .collect()
        })
}

fn is_exhaustive(target: &Type, coverage: &IndexSet<Coverage>) -> bool {
    coverage.contains(&Coverage::CatchAll)
        || match target {
            Type::Bool => coverage.contains(&Coverage::True) && coverage.contains(&Coverage::False),
            Type::Option(_) => coverage.contains(&Coverage::None) && coverage.contains(&Coverage::Some),
            Type::List(_) => coverage.contains(&Coverage::EmptyList) && coverage.contains(&Coverage::Cons),
            Type::Unit
            | Type::Nat
            | Type::Ratio
            | Type::Duration
            | Type::Pitch
            | Type::PitchClass
            | Type::Interval
            | Type::Scale
            | Type::Key
            | Type::Degree
            | Type::Frame
            | Type::Music
            | Type::Product(_)
            | Type::Function(_, _) => false,
        }
}

fn literal_key(value: &Value) -> String {
    match value {
        Value::Bool(value) => format!("bool:{value}"),
        Value::Nat(value) => format!("nat:{value}"),
        Value::Ratio(value) => format!("ratio:{}/{}", value.numer(), value.denom()),
        Value::Duration(value) => format!("duration:{}/{}", value.numer(), value.denom()),
        Value::Pitch(value) => format!("pitch:{}:{}:{}", value.letter.steps(), value.accidental.0, value.octave),
        Value::PitchClass(value) => format!("pitchclass:{}:{}", value.letter.steps(), value.accidental.0),
        Value::Interval(value) => format!("interval:{}:{}", value.diatonic_steps, value.semitones),
        Value::Scale(value) => format!("scale:{value}"),
        Value::Key(value) => format!("key:{}:{:?}", value.tonic(), value.mode()),
        Value::Degree(value) => format!("degree:{}:{}", value.ordinal(), value.alteration()),
        Value::Frame(value) => format!("frame:{}:{}", value.scale(), value.tonic()),
        Value::Product(_)
        | Value::Option { .. }
        | Value::List { .. }
        | Value::Music(_)
        | Value::Closure(_)
        | Value::Builtin(_) => "constructor".to_owned(),
    }
}

fn dependency_order(resolver: &mut Resolver, definitions: &[CheckedDefinition]) -> Option<Vec<usize>> {
    let indices: IndexMap<_, _> = definitions
        .iter()
        .enumerate()
        .map(|(index, definition)| (definition.name.as_str(), index))
        .collect();
    let mut colors = vec![0u8; definitions.len()];
    let mut stack = Vec::new();
    let mut order = Vec::with_capacity(definitions.len());
    let mut acyclic = true;
    for index in 0..definitions.len() {
        visit(
            index,
            definitions,
            &indices,
            &mut colors,
            &mut stack,
            &mut order,
            resolver,
            &mut acyclic,
        );
    }
    acyclic.then_some(order)
}

#[allow(clippy::too_many_arguments)]
fn visit(
    index: usize,
    definitions: &[CheckedDefinition],
    indices: &IndexMap<&str, usize>,
    colors: &mut [u8],
    stack: &mut Vec<usize>,
    order: &mut Vec<usize>,
    resolver: &mut Resolver,
    acyclic: &mut bool,
) {
    match colors.get(index).copied() {
        Some(2) => return,
        Some(1) => {
            *acyclic = false;
            return;
        }
        _ => {}
    }
    if let Some(color) = colors.get_mut(index) {
        *color = 1;
    }
    stack.push(index);
    let Some(definition) = definitions.get(index) else {
        return;
    };
    for (dependency, use_span) in &definition.dependencies {
        let Some(dependency_index) = indices.get(dependency.as_str()).copied() else {
            continue;
        };
        if colors.get(dependency_index) == Some(&1) {
            let start = stack.iter().position(|member| *member == dependency_index).unwrap_or(0);
            let mut names: Vec<_> = stack
                .iter()
                .skip(start)
                .filter_map(|member| definitions.get(*member))
                .map(|member| member.name.as_str())
                .collect();
            names.push(dependency);
            let mut diagnostic = Diagnostic::error(Code::DependencyCycle, "these definitions call each other")
                .at(*use_span, "the cycle closes here")
                .note(format!("the cycle is {}", names.join(" → ")))
                .help("pass the changing value as a parameter, or replace the recursion with a finite fold");
            if let Some(target) = definitions.get(dependency_index)
                && !target.foreign
            {
                diagnostic = diagnostic.also(target.span, format!("`{dependency}` is defined here"));
            }
            resolver.report(diagnostic);
            *acyclic = false;
            continue;
        }
        visit(
            dependency_index,
            definitions,
            indices,
            colors,
            stack,
            order,
            resolver,
            acyclic,
        );
    }
    stack.pop();
    if let Some(color) = colors.get_mut(index) {
        *color = 2;
    }
    order.push(index);
}

fn evaluate(
    resolver: &mut Resolver,
    definitions: &[CheckedDefinition],
    order: &[usize],
    meter: &mut WorkMeter,
) -> Option<IndexMap<String, Value>> {
    let mut values = IndexMap::new();
    for index in order {
        let definition = definitions.get(*index)?;
        if !meter.output("scalar elaboration", 0, definition.span) {
            report_exhaustion(resolver, meter);
            return None;
        }
        let value = match &definition.kind {
            CheckedDefinitionKind::Let { body } => eval(body, &values, meter),
            CheckedDefinitionKind::Function { parameters, body } => {
                let mut captures = IndexMap::new();
                for dependency in definition.dependencies.keys() {
                    captures.insert(dependency.clone(), values.get(dependency)?.clone());
                }
                let result = function_result(&definition.ty)?.clone();
                Some(Value::Closure(Box::new(Closure {
                    parameters: parameters.clone(),
                    result,
                    body: body.clone(),
                    captures,
                })))
            }
        };
        let Some(value) = value else {
            if meter.exhaustion().is_some() {
                report_exhaustion(resolver, meter);
                return None;
            }
            resolver.report(
                Diagnostic::error(Code::TypeMismatch, "this checked expression could not be evaluated")
                    .at(definition.span, "evaluation stopped here")
                    .note("this is a compiler invariant failure, not a recoverable language effect"),
            );
            return None;
        };
        if value.ty() != definition.ty {
            resolver.report(
                Diagnostic::error(Code::TypeMismatch, "evaluation changed this expression's type")
                    .at(definition.span, "the preservation invariant failed here"),
            );
            return None;
        }
        let _normalization_witness = value.normalization_witness();
        values.insert(definition.name.clone(), value);
    }
    Some(values)
}

fn eval(expression: &Expr, environment: &IndexMap<String, Value>, meter: &mut WorkMeter) -> Option<Value> {
    if !meter.step("expression evaluation", 1, expression.span) {
        return None;
    }
    let value = match &expression.kind {
        ExprKind::Literal(value) => Some(value.clone()),
        ExprKind::Name(name) => environment.get(name).cloned(),
        ExprKind::Product(members) => members
            .iter()
            .map(|member| eval(member, environment, meter))
            .collect::<Option<Vec<_>>>()
            .map(Value::Product),
        ExprKind::Option(value) => {
            let Type::Option(member) = &expression.ty else {
                return None;
            };
            let member = member.as_ref().clone();
            let value = if let Some(value) = value.as_deref() {
                Some(Box::new(eval(value, environment, meter)?))
            } else {
                None
            };
            Some(Value::Option { member, value })
        }
        ExprKind::List(values) => {
            let Type::List(member) = &expression.ty else {
                return None;
            };
            let member = member.as_ref().clone();
            let values = values
                .iter()
                .map(|value| eval(value, environment, meter))
                .collect::<Option<Vec<_>>>()?;
            Some(Value::List { member, values })
        }
        ExprKind::Apply { function, arguments } => {
            let function = eval(function, environment, meter)?;
            match function {
                Value::Closure(closure) => {
                    let mut provided = vec![None; closure.parameters.len()];
                    for argument in arguments {
                        let slot = provided.get_mut(argument.parameter)?;
                        *slot = Some(eval(&argument.value, environment, meter)?);
                    }
                    apply_closure(&closure, provided, meter, expression.span)
                }
                Value::Builtin(value) => {
                    let Type::Function(parameters, _) = Value::Builtin(value.clone()).ty() else {
                        return None;
                    };
                    let mut provided = vec![None; parameters.len()];
                    for argument in arguments {
                        let slot = provided.get_mut(argument.parameter)?;
                        *slot = Some(eval(&argument.value, environment, meter)?);
                    }
                    apply_builtin(&value, provided, expression.span)
                }
                Value::Bool(_)
                | Value::Nat(_)
                | Value::Ratio(_)
                | Value::Duration(_)
                | Value::Pitch(_)
                | Value::PitchClass(_)
                | Value::Interval(_)
                | Value::Scale(_)
                | Value::Key(_)
                | Value::Degree(_)
                | Value::Frame(_)
                | Value::Product(_)
                | Value::Option { .. }
                | Value::List { .. }
                | Value::Music(_) => None,
            }
        }
        ExprKind::PitchAction { pitch, interval, down } => {
            let Value::Pitch(pitch) = eval(pitch, environment, meter)? else {
                return None;
            };
            let Value::Interval(mut interval) = eval(interval, environment, meter)? else {
                return None;
            };
            if *down {
                interval = interval.inverse()?;
            }
            pitch.transpose(interval).map(Value::Pitch)
        }
        ExprKind::Primitive { primitive, arguments } => {
            eval_primitive(*primitive, arguments, environment, meter, expression)
        }
        ExprKind::Match { scrutinee, arms } => {
            let value = eval(scrutinee, environment, meter)?;
            let mut selected = None;
            for arm in arms {
                if !meter.step("match arm", 1, expression.span) {
                    return None;
                }
                if let Some(bindings) = match_pattern(&arm.pattern, &value) {
                    selected = Some((arm, bindings));
                    break;
                }
            }
            let (arm, bindings) = selected?;
            let mut local = environment.clone();
            local.extend(bindings);
            eval(&arm.body, &local, meter)
        }
        ExprKind::Step { base, steps, down } => {
            // Reachable only through `pitch_term`; a `step` outside a `music`
            // value is refused while checking, where the diagnostic can name
            // the missing context.
            let _ = (base, steps, down);
            None
        }
        ExprKind::Music(music) => {
            let mut uses = IndexMap::new();
            for (span, expression) in &music.uses {
                let Value::Music(value) = eval(expression, environment, meter)? else {
                    return None;
                };
                uses.insert(span_key(*span), value);
            }
            let mut pitches = IndexMap::new();
            for (span, expression) in &music.pitches {
                pitches.insert(span_key(*span), pitch_term(expression, environment, meter)?);
            }
            let mut scales = IndexMap::new();
            for (span, expression) in &music.scales {
                let Value::Scale(value) = eval(expression, environment, meter)? else {
                    return None;
                };
                scales.insert(span_key(*span), value);
            }
            let mut bindings = IndexMap::new();
            for name in &music.bindings {
                let bound = match environment.get(name)? {
                    Value::Pitch(pitch) => crate::resolve::BoundValue::Pitch(*pitch),
                    Value::Duration(duration) => {
                        crate::resolve::BoundValue::Duration(crate::score::NotatedDuration::single(
                            crate::MusicalDuration::new(*duration),
                            ratio_text(duration),
                        ))
                    }
                    Value::Bool(_)
                    | Value::Nat(_)
                    | Value::Ratio(_)
                    | Value::PitchClass(_)
                    | Value::Interval(_)
                    | Value::Scale(_)
                    | Value::Key(_)
                    | Value::Degree(_)
                    | Value::Frame(_)
                    | Value::Product(_)
                    | Value::Option { .. }
                    | Value::List { .. }
                    | Value::Music(_)
                    | Value::Closure(_)
                    | Value::Builtin(_) => return None,
                };
                bindings.insert(name.clone(), bound);
            }
            Some(Value::Music(Music {
                items: music.items.clone(),
                uses,
                pitches: Box::new(pitches),
                scales: Box::new(scales),
                bindings,
                role: music.role.clone(),
                definition_span: music.definition_span,
                operation: None,
            }))
        }
    }?;
    let (nodes, bytes) = value_shape(&value);
    if value.ty() != expression.ty || !meter.construct("expression value", nodes, bytes, expression.span) {
        return None;
    }
    Some(value)
}

/// Evaluate a note's pitch expression as far as the ambient scale allows.
fn pitch_term(expression: &Expr, environment: &IndexMap<String, Value>, meter: &mut WorkMeter) -> Option<PitchTerm> {
    match &expression.kind {
        ExprKind::Step { base, steps, down } => {
            if !meter.step("scale step", 1, expression.span) {
                return None;
            }
            let base = pitch_term(base, environment, meter)?;
            let steps = i64::try_from(nat_value(&eval(steps, environment, meter)?)?).ok()?;
            let steps = if *down { steps.checked_neg()? } else { steps };
            Some(PitchTerm::Stepped {
                base: Box::new(base),
                steps,
            })
        }
        ExprKind::PitchAction { pitch, interval, down } => {
            let base = pitch_term(pitch, environment, meter)?;
            let Value::Interval(interval) = eval(interval, environment, meter)? else {
                return None;
            };
            let moved = PitchTerm::Moved {
                base: Box::new(base),
                interval,
                down: *down,
            };
            if moved.reads_scale() {
                return Some(moved);
            }
            // Nothing here reads the scale, so it is finished once, now.
            moved.resolve(None).ok().map(PitchTerm::Written)
        }
        ExprKind::Literal(_)
        | ExprKind::Name(_)
        | ExprKind::Product(_)
        | ExprKind::Option(_)
        | ExprKind::List(_)
        | ExprKind::Apply { .. }
        | ExprKind::Primitive { .. }
        | ExprKind::Match { .. }
        | ExprKind::Music(_) => {
            let Value::Pitch(pitch) = eval(expression, environment, meter)? else {
                return None;
            };
            Some(PitchTerm::Written(pitch))
        }
    }
}

fn apply_closure(
    closure: &Closure,
    mut provided: Vec<Option<Value>>,
    meter: &mut WorkMeter,
    span: SourceSpan,
) -> Option<Value> {
    if !meter.step("function application", 1, span) {
        return None;
    }
    let mut local = closure.captures.clone();
    let mut remaining = Vec::new();
    for (index, parameter) in closure.parameters.iter().enumerate() {
        let value = provided.get_mut(index).and_then(Option::take).or_else(|| {
            parameter
                .default
                .as_ref()
                .and_then(|default| eval(default, &local, meter))
        });
        if let Some(value) = value {
            if value.ty() != parameter.ty {
                return None;
            }
            local.insert(parameter.name.clone(), value);
        } else {
            remaining.push(parameter.clone());
        }
    }
    if remaining.is_empty() {
        eval(&closure.body, &local, meter)
    } else {
        Some(Value::Closure(Box::new(Closure {
            parameters: remaining,
            result: closure.result.clone(),
            body: closure.body.clone(),
            captures: local,
        })))
    }
}

fn apply_builtin(builtin: &BuiltinValue, provided: Vec<Option<Value>>, span: SourceSpan) -> Option<Value> {
    let mut value = builtin.clone();
    let remaining = value
        .bound
        .iter()
        .enumerate()
        .filter_map(|(index, slot)| slot.is_none().then_some(index))
        .collect::<Vec<_>>();
    for (argument, target) in provided.into_iter().zip(remaining) {
        if let Some(argument) = argument {
            *value.bound.get_mut(target)? = Some(argument);
        }
    }
    if value.bound.iter().any(Option::is_none) {
        return Some(Value::Builtin(Box::new(value)));
    }
    let mut arguments = value.bound.into_iter().collect::<Option<Vec<_>>>()?.into_iter();
    let operation = match value.builtin {
        Builtin::Transpose => MusicOperation::Transpose {
            interval: interval_value(&arguments.next()?)?,
            source: music_value(arguments.next()?)?,
        },
        Builtin::Stretch => {
            let factor = ratio_value(&arguments.next()?)?;
            if factor <= Ratio::ZERO {
                return None;
            }
            MusicOperation::Stretch {
                factor,
                source: music_value(arguments.next()?)?,
            }
        }
        Builtin::Retrograde => MusicOperation::Retrograde {
            source: music_value(arguments.next()?)?,
        },
        Builtin::Invert => MusicOperation::Invert {
            axis: pitch_value(&arguments.next()?)?,
            source: music_value(arguments.next()?)?,
        },
        Builtin::Shift => {
            let by = duration_value(&arguments.next()?)?;
            if by < Ratio::ZERO {
                return None;
            }
            MusicOperation::Shift {
                by,
                source: music_value(arguments.next()?)?,
            }
        }
        Builtin::Overlay => MusicOperation::Overlay {
            left: music_value(arguments.next()?)?,
            right: Box::new(music_value(arguments.next()?)?),
        },
        Builtin::MapNotePitches => {
            let Value::Closure(mapper) = arguments.next()? else {
                return None;
            };
            MusicOperation::MapNotePitches {
                mapper: PitchFunction(mapper),
                source: music_value(arguments.next()?)?,
            }
        }
    };
    Some(Value::Music(Music {
        items: Vec::new(),
        uses: IndexMap::new(),
        pitches: Box::default(),
        scales: Box::default(),
        bindings: IndexMap::new(),
        role: None,
        definition_span: span,
        operation: Some(Box::new(operation)),
    }))
}

fn music_value(value: Value) -> Option<Music> {
    let Value::Music(value) = value else { return None };
    Some(value)
}

fn pitch_value(value: &Value) -> Option<WrittenPitch> {
    let Value::Pitch(value) = value else { return None };
    Some(*value)
}

fn interval_value(value: &Value) -> Option<Interval> {
    let Value::Interval(value) = value else { return None };
    Some(*value)
}

fn ratio_value(value: &Value) -> Option<Ratio<i64>> {
    let Value::Ratio(value) = value else { return None };
    Some(*value)
}

fn duration_value(value: &Value) -> Option<Ratio<i64>> {
    let Value::Duration(value) = value else { return None };
    Some(*value)
}

fn eval_primitive(
    primitive: Primitive,
    arguments: &[Expr],
    environment: &IndexMap<String, Value>,
    meter: &mut WorkMeter,
    expression: &Expr,
) -> Option<Value> {
    let values = arguments
        .iter()
        .map(|argument| eval(argument, environment, meter))
        .collect::<Option<Vec<_>>>()?;
    match primitive {
        Primitive::IntervalAdd => {
            let Value::Interval(first) = values.first()? else {
                return None;
            };
            let Value::Interval(second) = values.get(1)? else {
                return None;
            };
            first.compose(*second).map(Value::Interval)
        }
        Primitive::IntervalInverse => {
            let Value::Interval(interval) = values.first()? else {
                return None;
            };
            interval.inverse().map(Value::Interval)
        }
        Primitive::PitchClassOf => {
            let Value::Pitch(pitch) = values.first()? else {
                return None;
            };
            Some(Value::PitchClass(pitch.pitch_class()))
        }
        Primitive::SignatureScale => {
            let Value::Key(key) = values.first()? else {
                return None;
            };
            Some(Value::Scale(crate::scale::signature_scale(*key)))
        }
        Primitive::ScaleOn => {
            let (Value::Scale(scale), Value::PitchClass(tonic)) = (values.first()?, values.get(1)?) else {
                return None;
            };
            Some(Value::Scale(scale.rooted_at(*tonic)))
        }
        Primitive::ScaleTonic => {
            let Value::Scale(scale) = values.first()? else {
                return None;
            };
            Some(Value::PitchClass(scale.tonic()))
        }
        Primitive::ScaleSize => {
            let Value::Scale(scale) = values.first()? else {
                return None;
            };
            Some(Value::Nat(u64::try_from(scale.size()).ok()?))
        }
        Primitive::ScalePitch => {
            let (Value::Scale(scale), Value::Pitch(pitch)) = (values.first()?, values.get(1)?) else {
                return None;
            };
            let located = crate::scale::Frame::around(*scale, *pitch).and_then(|frame| frame.locate(*pitch));
            Some(optional(Type::Degree, located.map(Value::Degree)))
        }
        Primitive::PitchFrame => {
            let (Value::Scale(scale), Value::Pitch(tonic)) = (values.first()?, values.get(1)?) else {
                return None;
            };
            Some(optional(
                Type::Frame,
                crate::scale::Frame::new(*scale, *tonic).map(Value::Frame),
            ))
        }
        Primitive::FrameScale => {
            let Value::Frame(frame) = values.first()? else {
                return None;
            };
            Some(Value::Scale(frame.scale()))
        }
        Primitive::FrameTonic => {
            let Value::Frame(frame) = values.first()? else {
                return None;
            };
            Some(Value::Pitch(frame.tonic()))
        }
        Primitive::FramePitch => {
            let (Value::Frame(frame), Value::Degree(degree)) = (values.first()?, values.get(1)?) else {
                return None;
            };
            frame.pitch(*degree).map(Value::Pitch)
        }
        Primitive::DegreeOf => {
            let ordinal = nat_value(values.first()?)?;
            let ordinal = i64::try_from(ordinal).ok()?;
            Some(Value::Degree(crate::scale::Degree::new(ordinal.checked_sub(1)?)))
        }
        Primitive::DegreeStepUp | Primitive::DegreeStepDown => {
            let Value::Degree(degree) = values.first()? else {
                return None;
            };
            let steps = i64::try_from(nat_value(values.get(1)?)?).ok()?;
            let steps = if primitive == Primitive::DegreeStepDown {
                steps.checked_neg()?
            } else {
                steps
            };
            degree.step(steps).map(Value::Degree)
        }
        Primitive::DegreeRaised => {
            let Value::Degree(degree) = values.first()? else {
                return None;
            };
            degree.raised().map(Value::Degree)
        }
        Primitive::DegreeLowered => {
            let Value::Degree(degree) = values.first()? else {
                return None;
            };
            degree.lowered().map(Value::Degree)
        }
        Primitive::Range => {
            let count = nat_value(values.first()?)?;
            let nodes = count.saturating_add(1);
            let bytes = count.saturating_mul(8);
            if !meter.step("range", count, expression.span)
                || !meter.preflight_construct("range", nodes, bytes, expression.span)
            {
                return None;
            }
            let capacity = usize::try_from(count).ok()?;
            let values = (0..count).map(Value::Nat).collect::<Vec<_>>();
            if values.len() != capacity {
                return None;
            }
            Some(Value::List {
                member: Type::Nat,
                values,
            })
        }
        Primitive::Repeat => {
            let value = values.first()?.clone();
            let count = nat_value(values.get(1)?)?;
            let (value_nodes, value_bytes) = value_shape(&value);
            let nodes = value_nodes.saturating_mul(count).saturating_add(1);
            let bytes = value_bytes.saturating_mul(count);
            if !meter.step("repeat", count, expression.span)
                || !meter.preflight_construct("repeat", nodes, bytes, expression.span)
            {
                return None;
            }
            let count = usize::try_from(count).ok()?;
            Some(Value::List {
                member: value.ty(),
                values: vec![value; count],
            })
        }
        Primitive::Map => {
            let Value::Closure(function) = values.first()? else {
                return None;
            };
            let Value::List { values: source, .. } = values.get(1)? else {
                return None;
            };
            if !meter.step("map", u64::try_from(source.len()).unwrap_or(u64::MAX), expression.span) {
                return None;
            }
            let mapped = source
                .iter()
                .cloned()
                .map(|value| apply_closure(function, vec![Some(value)], meter, expression.span))
                .collect::<Option<Vec<_>>>()?;
            let Type::List(member) = &expression.ty else {
                return None;
            };
            let member = member.as_ref().clone();
            Some(Value::List { member, values: mapped })
        }
        Primitive::Filter => {
            let Value::Closure(predicate) = values.first()? else {
                return None;
            };
            let Value::List { member, values } = values.get(1)? else {
                return None;
            };
            if !meter.step(
                "filter",
                u64::try_from(values.len()).unwrap_or(u64::MAX),
                expression.span,
            ) {
                return None;
            }
            let mut kept = Vec::new();
            for value in values {
                let decision = apply_closure(predicate, vec![Some(value.clone())], meter, expression.span)?;
                let Value::Bool(keep) = decision else {
                    return None;
                };
                if keep {
                    kept.push(value.clone());
                }
            }
            Some(Value::List {
                member: member.clone(),
                values: kept,
            })
        }
        Primitive::NatFold => {
            let mut accumulator = values.first()?.clone();
            let Value::Closure(step) = values.get(1)? else {
                return None;
            };
            let count = nat_value(values.get(2)?)?;
            if !meter.step("nat_fold", count, expression.span) {
                return None;
            }
            for index in 0..count {
                accumulator = apply_closure(
                    step,
                    vec![Some(Value::Nat(index)), Some(accumulator)],
                    meter,
                    expression.span,
                )?;
            }
            Some(accumulator)
        }
        Primitive::ListFold => {
            let mut accumulator = values.first()?.clone();
            let Value::Closure(step) = values.get(1)? else {
                return None;
            };
            let Value::List { values, .. } = values.get(2)? else {
                return None;
            };
            if !meter.step(
                "list_fold",
                u64::try_from(values.len()).unwrap_or(u64::MAX),
                expression.span,
            ) {
                return None;
            }
            for value in values {
                accumulator = apply_closure(
                    step,
                    vec![Some(value.clone()), Some(accumulator)],
                    meter,
                    expression.span,
                )?;
            }
            Some(accumulator)
        }
        Primitive::OptionFold => {
            let zero = values.first()?.clone();
            let Value::Closure(some_case) = values.get(1)? else {
                return None;
            };
            let Value::Option { value, .. } = values.get(2)? else {
                return None;
            };
            match value {
                Some(value) => {
                    if !meter.step("option_fold", 1, expression.span) {
                        return None;
                    }
                    apply_closure(some_case, vec![Some(value.as_ref().clone())], meter, expression.span)
                }
                None => Some(zero),
            }
        }
    }
}

/// Wrap a partial answer as this evaluator's option value.
fn optional(member: Type, value: Option<Value>) -> Value {
    Value::Option {
        member,
        value: value.map(Box::new),
    }
}

fn nat_value(value: &Value) -> Option<u64> {
    if let Value::Nat(value) = value {
        Some(*value)
    } else {
        None
    }
}

fn ratio_text(value: &Ratio<i64>) -> String {
    if *value.denom() == 1 {
        value.numer().to_string()
    } else {
        format!("{}/{}", value.numer(), value.denom())
    }
}

const fn span_key(span: SourceSpan) -> u64 {
    (span.start as u64) << 32 | span.end as u64
}

fn match_pattern(pattern: &Pattern, value: &Value) -> Option<IndexMap<String, Value>> {
    let mut bindings = IndexMap::new();
    let matched = match pattern {
        Pattern::Wildcard => true,
        Pattern::Bind(name) => {
            bindings.insert(name.clone(), value.clone());
            true
        }
        Pattern::Literal(expected) => literal_values_equal(expected, value),
        Pattern::None => matches!(value, Value::Option { value: None, .. }),
        Pattern::Some(name) => {
            if let Value::Option {
                value: Some(member), ..
            } = value
            {
                bindings.insert(name.clone(), member.as_ref().clone());
                true
            } else {
                false
            }
        }
        Pattern::EmptyList => matches!(value, Value::List { values, .. } if values.is_empty()),
        Pattern::Cons { head, tail } => {
            if let Value::List { member, values } = value
                && !values.is_empty()
            {
                let first = values.first()?.clone();
                let rest = values.get(1..)?.to_vec();
                bindings.insert(head.clone(), first);
                bindings.insert(
                    tail.clone(),
                    Value::List {
                        member: member.clone(),
                        values: rest,
                    },
                );
                true
            } else {
                false
            }
        }
        Pattern::Product(names) => {
            if let Value::Product(members) = value
                && names.len() == members.len()
            {
                bindings.extend(names.iter().cloned().zip(members.iter().cloned()));
                true
            } else {
                false
            }
        }
    };
    matched.then_some(bindings)
}

fn literal_values_equal(left: &Value, right: &Value) -> bool {
    if let (Value::Bool(left), Value::Bool(right)) = (left, right) {
        left == right
    } else if let (Value::Nat(left), Value::Nat(right)) = (left, right) {
        left == right
    } else if let (Value::Ratio(left), Value::Ratio(right)) = (left, right) {
        left == right
    } else if let (Value::Duration(left), Value::Duration(right)) = (left, right) {
        left == right
    } else if let (Value::Pitch(left), Value::Pitch(right)) = (left, right) {
        left == right
    } else if let (Value::PitchClass(left), Value::PitchClass(right)) = (left, right) {
        left == right
    } else if let (Value::Interval(left), Value::Interval(right)) = (left, right) {
        left == right
    } else {
        false
    }
}

fn value_shape(value: &Value) -> (u64, u64) {
    match value {
        Value::Bool(_) => (1, 1),
        Value::Nat(_) => (1, 8),
        Value::Ratio(_) | Value::Duration(_) => (1, 16),
        Value::Pitch(_) | Value::PitchClass(_) | Value::Interval(_) | Value::Key(_) | Value::Degree(_) => (1, 12),
        Value::Scale(_) => (1, 24),
        Value::Frame(_) => (1, 36),
        Value::Product(members) => aggregate_shape(members.iter()),
        Value::Option { value, .. } => value.as_deref().map_or((1, 1), |value| {
            let (nodes, bytes) = value_shape(value);
            (nodes.saturating_add(1), bytes.saturating_add(1))
        }),
        Value::List { values, .. } => aggregate_shape(values.iter()),
        Value::Music(music) => music_shape(music),
        Value::Closure(closure) => aggregate_shape(closure.captures.values()),
        Value::Builtin(value) => aggregate_shape(value.bound.iter().flatten()),
    }
}

fn music_shape(music: &Music) -> (u64, u64) {
    let items = u64::try_from(music.items.len()).unwrap_or(u64::MAX);
    let base = (items.saturating_add(1), items.saturating_mul(32));
    let child = match music.operation.as_deref() {
        None => (0, 0),
        Some(
            MusicOperation::Transpose { source, .. }
            | MusicOperation::Stretch { source, .. }
            | MusicOperation::Retrograde { source }
            | MusicOperation::Invert { source, .. }
            | MusicOperation::Shift { source, .. }
            | MusicOperation::MapNotePitches { source, .. },
        ) => music_shape(source),
        Some(MusicOperation::Overlay { left, right }) => {
            let left = music_shape(left);
            let right = music_shape(right);
            (left.0.saturating_add(right.0), left.1.saturating_add(right.1))
        }
    };
    (base.0.saturating_add(child.0), base.1.saturating_add(child.1))
}

fn aggregate_shape<'a>(values: impl Iterator<Item = &'a Value>) -> (u64, u64) {
    values.fold((1u64, 0u64), |(nodes, bytes), value| {
        let (value_nodes, value_bytes) = value_shape(value);
        (nodes.saturating_add(value_nodes), bytes.saturating_add(value_bytes))
    })
}

pub(crate) fn report_exhaustion(resolver: &mut Resolver, meter: &WorkMeter) {
    let Some(exhaustion) = meter.exhaustion() else {
        return;
    };
    resolver.report(
        Diagnostic::error(
            Code::ResourceLimit,
            format!("`{}` exceeds the compilation budget", exhaustion.operation),
        )
        .at(
            exhaustion.span,
            format!(
                "attempted {} {}, limit {}",
                exhaustion.attempted, exhaustion.metric, exhaustion.limit
            ),
        )
        .note("the expression is finite; Musa rejected its size before publishing a partial value")
        .help("reduce the bound or split the generated material into smaller declarations"),
    );
}

fn parse_i64(resolver: &mut Resolver, token: &SyntaxToken) -> Option<i64> {
    token.text().parse().ok().or_else(|| {
        resolver.report(
            Diagnostic::error(Code::OutOfRange, "this number is too large")
                .at(token_span(token), "outside Musa's exact integer range"),
        );
        None
    })
}

fn parse_u64(resolver: &mut Resolver, token: &SyntaxToken) -> Option<u64> {
    token.text().parse().ok().or_else(|| {
        resolver.report(
            Diagnostic::error(Code::OutOfRange, "this natural number is too large")
                .at(token_span(token), "outside Musa's exact natural range"),
        );
        None
    })
}

fn parse_ratio(resolver: &mut Resolver, token: &SyntaxToken) -> Option<Ratio<i64>> {
    let (numerator, denominator) = token.text().split_once('/')?;
    let (Ok(numerator), Ok(denominator)) = (numerator.parse::<i64>(), denominator.parse::<i64>()) else {
        resolver.report(
            Diagnostic::error(Code::OutOfRange, "this ratio is too large")
                .at(token_span(token), "outside Musa's exact rational range"),
        );
        return None;
    };
    if denominator == 0 {
        resolver.report(
            Diagnostic::error(Code::NotAValue, "a ratio cannot have zero below the line")
                .at(token_span(token), "division by zero"),
        );
        return None;
    }
    Some(Ratio::new(numerator, denominator))
}

fn music_items(node: &SyntaxNode) -> Vec<VoiceItem> {
    if let Some(expression) = musa_language::ast::MusicExpr::cast(node.clone()) {
        expression.items()
    } else if let Some(declaration) = musa_language::ast::MotifDecl::cast(node.clone()) {
        declaration.items()
    } else if let Some(declaration) = musa_language::ast::FragmentDecl::cast(node.clone()) {
        declaration.items()
    } else if let Some(declaration) = musa_language::ast::BarStmt::cast(node.clone()) {
        declaration.items()
    } else {
        Vec::new()
    }
}

fn owned_descendants(owner: &SyntaxNode, kind: SyntaxKind) -> Vec<SyntaxNode> {
    owner
        .descendants()
        .filter(|node| node.kind() == kind)
        .filter(|node| {
            !node
                .ancestors()
                .skip(1)
                .take_while(|ancestor| ancestor != owner)
                .any(|ancestor| ancestor.kind() == SyntaxKind::MusicExpr)
        })
        .collect()
}

fn child_of(node: &SyntaxNode, predicate: fn(SyntaxKind) -> bool) -> Option<SyntaxNode> {
    node.children().find(|child| predicate(child.kind()))
}

fn is_type_node(kind: SyntaxKind) -> bool {
    matches!(
        kind,
        SyntaxKind::TypeExpr
            | SyntaxKind::TypeName
            | SyntaxKind::FunctionType
            | SyntaxKind::ProductType
            | SyntaxKind::OptionType
            | SyntaxKind::ListType
    )
}

/// Every collection spelling, for the diagnostic that lists them.
fn collection_list() -> String {
    let mut spellings: Vec<_> = crate::scale::Collection::spellings().collect();
    spellings.sort_unstable();
    format!("`{}`", spellings.join("`, `"))
}

fn is_expr_node(kind: SyntaxKind) -> bool {
    matches!(
        kind,
        SyntaxKind::NameExpr
            | SyntaxKind::LiteralExpr
            | SyntaxKind::ParenExpr
            | SyntaxKind::ProductExpr
            | SyntaxKind::ListExpr
            | SyntaxKind::OptionExpr
            | SyntaxKind::ApplyExpr
            | SyntaxKind::PitchExpr
            | SyntaxKind::ScaleExpr
            | SyntaxKind::KeyExpr
            | SyntaxKind::StepExpr
            | SyntaxKind::MatchExpr
            | SyntaxKind::MusicExpr
    )
}

fn significant_tokens(node: &SyntaxNode) -> impl Iterator<Item = SyntaxToken> + '_ {
    node.descendants_with_tokens()
        .filter_map(SyntaxElement::into_token)
        .filter(|token| !token.kind().is_trivia())
}

fn name_of(node: &SyntaxNode) -> Option<String> {
    if node.kind() != SyntaxKind::NameExpr {
        return None;
    }
    significant_tokens(node)
        .find(|token| matches!(token.kind(), SyntaxKind::Identifier | SyntaxKind::RepeatKw))
        .map(|token| token.text().to_owned())
}

fn first_name(node: &SyntaxNode) -> Option<String> {
    node.descendants_with_tokens()
        .filter_map(SyntaxElement::into_token)
        .find(|token| token.kind() == SyntaxKind::Identifier)
        .map(|token| token.text().to_owned())
}

fn argument_name(node: &SyntaxNode) -> Option<String> {
    let direct: Vec<_> = node
        .children_with_tokens()
        .filter_map(SyntaxElement::into_token)
        .filter(|token| !token.kind().is_trivia())
        .collect();
    match direct.as_slice() {
        [name, colon, ..] if name.kind() == SyntaxKind::Identifier && colon.kind() == SyntaxKind::Colon => {
            Some(name.text().to_owned())
        }
        _ => None,
    }
}

fn token_span(token: &SyntaxToken) -> SourceSpan {
    SourceSpan::new(
        u32::from(token.text_range().start()),
        u32::from(token.text_range().end()),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    enum ReferenceTerm {
        Nat(u64),
        Identity(Box<Self>),
    }

    impl ReferenceTerm {
        fn source(&self) -> String {
            match self {
                Self::Nat(value) => value.to_string(),
                Self::Identity(argument) => format!("id({})", argument.source()),
            }
        }

        fn evaluate(&self) -> u64 {
            match self {
                Self::Nat(value) => *value,
                Self::Identity(argument) => argument.evaluate(),
            }
        }
    }

    fn values(source: &str) -> Option<IndexMap<String, Value>> {
        let parsed = musa_language::parse(source);
        let piece = musa_language::ast::PieceDecl::from_root(&parsed.syntax())?;
        let mut resolver = Resolver::new();
        check_and_evaluate(
            &mut resolver,
            declarations(piece.syntax(), None).into_iter(),
            Some(piece.syntax()),
            UnknownRootMusic::Reject,
        )
        .map(|program| program.values)
    }

    #[test]
    fn every_compiler_owned_operation_names_its_hidden_information() {
        let entries = PRIMITIVE_OWNERSHIP
            .iter()
            .map(|entry| (entry.spelling, entry.hidden_information))
            .chain(
                BUILTIN_OWNERSHIP
                    .iter()
                    .map(|entry| (entry.spelling, entry.hidden_information)),
            )
            .collect::<Vec<_>>();
        assert_eq!(
            entries.len(),
            31,
            "new compiler operations must enter the ownership registry"
        );
        let unique = entries.iter().map(|(spelling, _)| *spelling).collect::<IndexSet<_>>();
        assert_eq!(
            unique.len(),
            entries.len(),
            "compiler operation spellings must be unique"
        );
        assert!(
            entries.iter().all(|(_, reason)| !reason.trim().is_empty()),
            "compiler ownership requires a hidden-information justification"
        );
    }

    #[test]
    fn production_evaluation_agrees_with_a_small_generated_reference() {
        for depth in 0..8 {
            for value in 0..16u64 {
                let mut reference = ReferenceTerm::Nat(value);
                for _ in 0..depth {
                    reference = ReferenceTerm::Identity(Box::new(reference));
                }
                let applied = reference.source();
                let source = format!("piece \"law\" {{ fn id(x: nat) -> nat = x; let result: nat = {applied}; }}");
                let actual = values(&source);
                assert!(actual.is_some(), "generated well-typed source was rejected: {source}");
                assert!(matches!(
                    actual.as_ref().and_then(|bindings| bindings.get("result")),
                    Some(Value::Nat(found)) if *found == reference.evaluate()
                ));
            }
        }
    }

    #[test]
    fn every_evaluated_binding_has_its_checked_type() {
        let source = "piece \"law\" { let pair: (nat, bool) = (3, true); fn keep(x: (nat, bool)) -> (nat, bool) = x; let result: (nat, bool) = keep(pair); }";
        let actual = values(source);
        assert!(actual.is_some(), "well-typed source was rejected");
        assert!(matches!(
            actual.as_ref().and_then(|bindings| bindings.get("result")),
            Some(Value::Product(members))
                if matches!(members.as_slice(), [Value::Nat(3), Value::Bool(true)])
        ));
    }

    #[test]
    fn finite_primitives_agree_with_small_reference_folds() {
        for count in 0..16u64 {
            let source = format!(
                "piece \"law\" {{ \
                 fn latest(index: nat, accumulator: nat) -> nat = index; \
                 fn item(value: nat, accumulator: nat) -> nat = value; \
                 fn id(value: nat) -> nat = value; \
                 fn reject(value: nat) -> bool = false; \
                 fn from_option(value: option[nat]) -> nat = match value {{ none -> 0, some(found) -> found }}; \
                 let by_nat: nat = nat_fold(0, latest, {count}); \
                 let values: list[nat] = range({count}); \
                 let mapped: list[nat] = map(id, values); \
                 let filtered: list[nat] = filter(reject, mapped); \
                 let by_list: nat = list_fold(0, item, mapped); \
                 let selected: nat = from_option(some(by_list)); \
                 }}"
            );
            let actual = values(&source);
            assert!(actual.is_some(), "generated finite source was rejected: {source}");
            let expected = (0..count).last().unwrap_or(0);
            assert!(matches!(
                actual.as_ref().and_then(|bindings| bindings.get("by_nat")),
                Some(Value::Nat(found)) if *found == expected
            ));
            assert!(matches!(
                actual.as_ref().and_then(|bindings| bindings.get("by_list")),
                Some(Value::Nat(found)) if *found == expected
            ));
            assert!(matches!(
                actual.as_ref().and_then(|bindings| bindings.get("mapped")),
                Some(Value::List { values, .. })
                    if values.iter().enumerate().all(|(index, value)| {
                        matches!(value, Value::Nat(found) if usize::try_from(*found).ok() == Some(index))
                    })
            ));
            assert!(matches!(
                actual.as_ref().and_then(|bindings| bindings.get("filtered")),
                Some(Value::List { values, .. }) if values.is_empty()
            ));
            assert!(matches!(
                actual.as_ref().and_then(|bindings| bindings.get("selected")),
                Some(Value::Nat(found)) if *found == expected
            ));
        }
    }
}
