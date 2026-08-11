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
use crate::module::Modules;
use crate::origin::{Interval, SourceSpan};
use crate::pitch::{PitchClass, WrittenPitch};
use crate::resolve::{NameKind, Resolver};

/// Check and evaluate imported definitions followed by a piece's definitions.
pub(crate) fn check_piece(
    resolver: &mut Resolver,
    libraries: &Libraries,
    root: &SyntaxNode,
    piece: &musa_language::ast::PieceDecl,
    bindings: Vec<Binding>,
) -> Option<Program> {
    if !validate_imports(resolver, libraries) {
        return None;
    }
    let modules = Modules::read(resolver, module_owners(libraries, root));
    check_and_evaluate(
        resolver,
        libraries
            .each()
            .flat_map(|(from, library)| declarations(library.syntax(), Some(from)))
            .chain(root_preamble(root))
            .chain(bindings.into_iter().map(SurfaceDefinition::Bound))
            .chain(declarations(piece.syntax(), None)),
        Some(piece.syntax()),
        UnknownRootMusic::Defer,
        &modules,
    )
}

/// Check and evaluate a root instance site's arguments, in the only scope a
/// document root has: its imports and its own values and functions.
///
/// This exists as its own pass because a piece made at the root has no piece
/// to be checked with — the piece *is* what the arguments are for.
pub(crate) fn check_arguments(
    resolver: &mut Resolver,
    libraries: &Libraries,
    root: &SyntaxNode,
    bindings: Vec<Binding>,
) -> Option<Program> {
    if !validate_imports(resolver, libraries) {
        return None;
    }
    let modules = Modules::read(resolver, module_owners(libraries, root));
    check_and_evaluate(
        resolver,
        libraries
            .each()
            .flat_map(|(from, library)| declarations(library.syntax(), Some(from)))
            .chain(root_preamble(root))
            .chain(bindings.into_iter().map(SurfaceDefinition::Bound)),
        None,
        UnknownRootMusic::Reject,
        &modules,
    )
}

/// Check one instance of a voice template: the template's body, read with
/// its parameters bound and nothing else the site could lend it.
///
/// The site's own scope is deliberately absent. A template body that could
/// read the piece it lands in would mean the same body means different
/// things in different places, which is the dynamic scoping this design
/// exists to avoid — the file's root is the one scope it shares.
pub(crate) fn check_template_voice(
    resolver: &mut Resolver,
    libraries: &Libraries,
    root: &SyntaxNode,
    voice: &musa_language::ast::VoiceDecl,
    bindings: Vec<Binding>,
) -> Option<Program> {
    let modules = Modules::read(resolver, module_owners(libraries, root));
    check_and_evaluate(
        resolver,
        libraries
            .each()
            .flat_map(|(from, library)| declarations(library.syntax(), Some(from)))
            .chain(root_preamble(root))
            .chain(bindings.into_iter().map(SurfaceDefinition::Bound))
            .chain(declarations(voice.syntax(), None)),
        Some(voice.syntax()),
        UnknownRootMusic::Defer,
        &modules,
    )
}

/// Every place a document's signatures and modules may be written: what its
/// imports export, in import order, then its own lexical root.
fn module_owners<'a>(
    libraries: &'a Libraries,
    root: &SyntaxNode,
) -> impl Iterator<Item = (Option<&'a str>, SyntaxNode)> {
    libraries
        .each()
        .map(|(from, library)| (Some(from.path), library.syntax().clone()))
        .chain(std::iter::once((None, root.clone())))
}

/// The definitions written at a document's lexical root, before its piece or
/// library. Templates are collected separately; these are the ordinary
/// values and functions their bodies may read.
fn root_preamble(root: &SyntaxNode) -> Vec<SurfaceDefinition> {
    root.children()
        .filter_map(|node| {
            LetDecl::cast(node.clone())
                .map(|declaration| SurfaceDefinition::Let {
                    declaration,
                    source: None,
                    qualifier: None,
                })
                .or_else(|| {
                    FnDecl::cast(node).map(|declaration| SurfaceDefinition::Function {
                        declaration,
                        source: None,
                        qualifier: None,
                    })
                })
        })
        .collect()
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
    let modules = Modules::read(
        resolver,
        libraries
            .each()
            .map(|(from, imported)| (Some(from.path), imported.syntax().clone()))
            .chain(std::iter::once((None, library.syntax().clone()))),
    );
    check_and_evaluate(
        resolver,
        libraries
            .each()
            .flat_map(|(from, imported)| declarations(imported.syntax(), Some(from)))
            .chain(declarations(library.syntax(), None)),
        None,
        UnknownRootMusic::Reject,
        &modules,
    )
    .is_some()
}

/// Check each library against precisely what it imports. A failure is then
/// restated at the importing document's `import` span: spans inside the
/// foreign CST must never be published as spans in this document.
fn validate_imports(resolver: &mut Resolver, libraries: &Libraries) -> bool {
    for (from, library, import_span, dependencies) in libraries.each_with_dependencies() {
        let path = from.path;
        // Validated as written, never as the importer qualified it: an `as`
        // belongs to the statement that wrote it, and a library must compile
        // on its own terms or the diagnostic is about the wrong document.
        let unqualified = |path| Some(crate::imports::Imported { path, qualifier: None });
        let mut prefix = Vec::new();
        let mut owners: Vec<(Option<&str>, SyntaxNode)> = Vec::new();
        for dependency in &dependencies {
            prefix.extend(declarations(dependency.syntax(), unqualified(path)));
            owners.push((Some(path), dependency.syntax().clone()));
        }
        prefix.extend(declarations(library.syntax(), unqualified(path)));
        owners.push((Some(path), library.syntax().clone()));
        let mut foreign_resolver = Resolver::new();
        let modules = Modules::read(&mut foreign_resolver, owners.iter().cloned());
        let evaluated = check_and_evaluate(
            &mut foreign_resolver,
            prefix.clone().into_iter(),
            None,
            UnknownRootMusic::Reject,
            &modules,
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

/// Every name `owner` declares, as the document reading it sees them.
///
/// `from` is absent for the document's own declarations and present for an
/// imported library, carrying both where the text came from and the `as`
/// qualifier the import wrote. An unqualified import binds flat — an imported
/// motif is called what it is called — and a qualified one binds `alias.name`
/// instead, which is how a collision between two modules is resolved.
fn declarations(owner: &SyntaxNode, from: Option<crate::imports::Imported<'_>>) -> Vec<SurfaceDefinition> {
    let source = from.map(|from| from.path.to_owned());
    let qualifier = from.and_then(|from| from.qualifier).map(str::to_owned);
    let mut found: Vec<_> = owner
        .children()
        .filter_map(|node| {
            LetDecl::cast(node.clone())
                .map(|declaration| SurfaceDefinition::Let {
                    declaration,
                    source: source.clone(),
                    qualifier: qualifier.clone(),
                })
                .or_else(|| {
                    FnDecl::cast(node.clone()).map(|declaration| SurfaceDefinition::Function {
                        declaration,
                        source: source.clone(),
                        qualifier: qualifier.clone(),
                    })
                })
                .or_else(|| {
                    musa_language::ast::MotifDecl::cast(node.clone()).map(|declaration| SurfaceDefinition::Legacy {
                        name: declaration.name().unwrap_or_default(),
                        syntax: declaration.syntax().clone(),
                        parameters: declaration.params(),
                        material: crate::resolve::Material::Motif,
                        source: source.clone(),
                        qualifier: qualifier.clone(),
                    })
                })
                .or_else(|| {
                    musa_language::ast::FragmentDecl::cast(node).map(|declaration| SurfaceDefinition::Legacy {
                        name: declaration.name().unwrap_or_default(),
                        syntax: declaration.syntax().clone(),
                        parameters: Vec::new(),
                        material: crate::resolve::Material::Fragment,
                        source: source.clone(),
                        qualifier: qualifier.clone(),
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
                source: source.clone(),
                qualifier: qualifier.clone(),
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
        qualifier: Option<String>,
    },
    Function {
        declaration: FnDecl,
        source: Option<String>,
        qualifier: Option<String>,
    },
    Legacy {
        name: String,
        syntax: SyntaxNode,
        parameters: Vec<musa_language::ast::Param>,
        material: crate::resolve::Material,
        source: Option<String>,
        qualifier: Option<String>,
    },
    /// A name a template body reads, and what stands for it — see
    /// [`Binding`].
    Bound(Binding),
    /// A module's member, held in the one flat namespace under the qualified
    /// name it is reached by. See [`crate::module`] for why a module needs
    /// nothing else from the core than a name and a scope.
    Member {
        name: String,
        item: crate::module::MemberItem,
        scope: crate::module::NameScope,
        source: Option<String>,
    },
}

/// One name bound into a checking pass from outside the source it checks.
///
/// This is how a template body is given its arguments: the parameter is an
/// ordinary definition of the declared type, and the body that reads it is
/// checked exactly as a written-out declaration would be. Nothing here is a
/// substitution over syntax, so nothing can capture a name or move a span.
#[derive(Clone)]
pub(crate) struct Binding {
    name: String,
    name_span: SourceSpan,
    span: SourceSpan,
    ty: SyntaxNode,
    stands_for: StandsFor,
    hidden: bool,
}

#[derive(Clone)]
enum StandsFor {
    /// An expression written at the instance site, checked in the scope the
    /// site stands in.
    Argument(SyntaxNode),
    /// What that expression already evaluated to, one pass earlier.
    Value(Box<Value>),
}

impl Binding {
    /// A name standing for an argument expression, checked where the
    /// argument is written.
    pub(crate) fn argument(
        name: String,
        name_span: SourceSpan,
        span: SourceSpan,
        ty: SyntaxNode,
        argument: SyntaxNode,
        hidden: bool,
    ) -> Self {
        Self {
            name,
            name_span,
            span,
            ty,
            stands_for: StandsFor::Argument(argument),
            hidden,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Type {
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
    ChordClass,
    Triad,
    Roman,
    Voicing,
    Pc12,
    PcSet12,
    Row12,
    Product(Vec<Self>),
    Option(Box<Self>),
    List(Box<Self>),
    Music,
    Function(Vec<Self>, Box<Self>),
}

impl std::fmt::Display for Type {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unit => out.write_str("Unit"),
            Self::Bool => out.write_str("Bool"),
            Self::Nat => out.write_str("Nat"),
            Self::Ratio => out.write_str("Ratio"),
            Self::Duration => out.write_str("Duration"),
            Self::Pitch => out.write_str("Pitch"),
            Self::PitchClass => out.write_str("NoteName"),
            Self::Interval => out.write_str("Interval"),
            Self::Scale => out.write_str("Scale"),
            Self::Key => out.write_str("Key"),
            Self::Degree => out.write_str("Degree"),
            Self::Frame => out.write_str("Frame"),
            Self::ChordClass => out.write_str("ChordClass"),
            Self::Triad => out.write_str("Triad"),
            Self::Roman => out.write_str("Roman"),
            Self::Voicing => out.write_str("Voicing"),
            Self::Pc12 => out.write_str("Pc12"),
            Self::PcSet12 => out.write_str("PcSet12"),
            Self::Row12 => out.write_str("Row12"),
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
            Self::Option(member) => write!(out, "Option<{member}>"),
            Self::List(member) => write!(out, "List<{member}>"),
            Self::Music => out.write_str("Music"),
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
    /// The default as the source spells it, kept for the signature an editor
    /// shows. The lowered `default` is what the checker applies; this is what
    /// the writer wrote, and rendering one from the other would be a second
    /// opinion about their own text.
    written_default: Option<String>,
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
    /// Whether the name is one the source wrote. A `make` site's arguments
    /// are held under names no one can type, so they must not enter the
    /// reference index and be offered for rename or completion.
    hidden: bool,
    /// How names read inside it. Empty for everything but a module's members
    /// — see [`crate::module::NameScope`].
    scope: crate::module::NameScope,
    /// The comment block written above it, for [`crate::docs`].
    summary: Option<String>,
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
    /// A value settled before this pass began.
    Bound {
        value: Box<Value>,
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
            RawDefinitionKind::Bound { .. } => NameKind::Value,
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
    KernelQuote(CheckedQuote),
}

/// A quotation, read once at its definition.
///
/// The term is parsed *here* rather than at each use, because the quoted text
/// does not depend on the environment: what depends on the environment is
/// what each hole evaluates to, and a hole is a name in the term by the time
/// this exists. So a malformed quote is one diagnostic at the quote, not one
/// per placement.
#[derive(Clone)]
struct CheckedQuote {
    /// The quoted term, with a fresh name standing where each hole was
    /// written.
    term: musa_kernel::Term<crate::elaborate::ScoreFact>,
    /// Each hole: the fresh name that stands for it, where it sits in the
    /// quote's own time, and the expression spliced there.
    holes: Vec<CheckedHole>,
    definition_span: SourceSpan,
}

#[derive(Clone)]
struct CheckedHole {
    name: String,
    locus: Ratio<i64>,
    value: Expr,
}

#[derive(Clone)]
struct CheckedMusic {
    items: Vec<VoiceItem>,
    uses: Vec<(SourceSpan, Expr)>,
    pitches: Vec<(SourceSpan, Expr)>,
    scales: Vec<(SourceSpan, Expr)>,
    claims: Vec<(SourceSpan, CheckedClaim)>,
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
    ScaleClass,
    ScaleChord,
    PitchFrame,
    FrameScale,
    FrameTonic,
    FramePitch,
    DegreeOf,
    DegreeStepUp,
    DegreeStepDown,
    DegreeRaised,
    DegreeLowered,
    ChordOn,
    ChordRoot,
    ChordBass,
    ChordMembers,
    ChordInversion,
    ChordOver,
    ChordTriad,
    TriadChord,
    TriadMajor,
    RomanOf,
    RomanOrdinal,
    RomanSize,
    RomanInversion,
    VoicingOf,
    VoicingPitches,
    VoicingBass,
    VoicingChord,
    VoicingPosition,
    CloseVoicing,
    DropVoicing,
    OmitVoicing,
    Pc12Of,
    Pc12Number,
    Pc12Forget,
    Pc12Transposed,
    Pc12Inverted,
    Pc12Spelled,
    PcSet12Of,
    PcSet12Members,
    PcSet12Normal,
    PcSet12Prime,
    PcSet12Vector,
    Row12Of,
    Row12Pcs,
    Row12Head,
    Row12Transposed,
    Row12Inverted,
    Row12Retrograde,
    Row12Matrix,
    Row12Forms,
    Row12Symmetries,
    Row12Repeats,
    Row12Missing,
}

/// A base type as a primitive signature names it.
///
/// These are the inert types of `docs/language/02-core-calculus.md` §5.8: a closed value of one is
/// an opaque constant, no reduction rule inspects its structure, and everything observable about it
/// is observed by applying a primitive. That is condition D1, and it holds here by construction —
/// there is no variant for a type with an eliminator.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Base {
    Bool,
    Nat,
    Pitch,
    PitchClass,
    Interval,
    Key,
    Scale,
    Degree,
    Frame,
    ChordClass,
    Triad,
    Roman,
    Voicing,
    Pc12,
    PcSet12,
    Row12,
}

/// An argument or result type of a δ-primitive.
///
/// There is deliberately no arrow constructor. §5.8's no-arrow premise is therefore true of every
/// declared δ signature by construction rather than by inspection, and a primitive that wanted a
/// function argument could not be spelled here at all — it would have to join the eliminators,
/// which is exactly the classification the theorem depends on.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Shape {
    Base(Base),
    Option(&'static Self),
    List(&'static Self),
}

impl Shape {
    /// The core type this shape denotes.
    fn ty(self) -> Type {
        match self {
            Self::Base(base) => base.ty(),
            Self::Option(member) => Type::Option(Box::new(member.ty())),
            Self::List(member) => Type::List(Box::new(member.ty())),
        }
    }

    /// Whether absence is expressible in this shape's outermost position.
    ///
    /// D2 lets a primitive be partial only by saying so in its result type, so this is what the
    /// sampling law consults before it accepts a `None` from an evaluator.
    #[cfg(test)]
    const fn admits_absence(self) -> bool {
        matches!(self, Self::Option(_))
    }
}

impl Base {
    fn ty(self) -> Type {
        match self {
            Self::Bool => Type::Bool,
            Self::Nat => Type::Nat,
            Self::Pitch => Type::Pitch,
            Self::PitchClass => Type::PitchClass,
            Self::Interval => Type::Interval,
            Self::Key => Type::Key,
            Self::Scale => Type::Scale,
            Self::Degree => Type::Degree,
            Self::Frame => Type::Frame,
            Self::ChordClass => Type::ChordClass,
            Self::Triad => Type::Triad,
            Self::Roman => Type::Roman,
            Self::Voicing => Type::Voicing,
            Self::Pc12 => Type::Pc12,
            Self::PcSet12 => Type::PcSet12,
            Self::Row12 => Type::Row12,
        }
    }
}

/// Which of `02-core-calculus.md` §5.8's three families a compiler-owned operation belongs to.
///
/// The families are disjoint and exhaustive, which is what lets Theorem 5 be stated once instead of
/// once per musical domain. A new domain is admissible when its operations can be declared here as
/// `Delta` and discharge D1–D4; it does not get a new induction.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Family {
    /// First-order and arrow-free. Covered by Theorem 5 once D1–D4 hold, and the declared
    /// signature is the single statement of the operation's type: the checker reads argument and
    /// result types from it rather than restating them.
    Delta { arguments: &'static [Shape], result: Shape },
    /// Takes a function argument or carries a rank-1 scheme. Covered by §5.6, and checked by hand
    /// because its type depends on its arguments' types.
    Eliminator(Eliminator),
    /// Constructs or transforms `music`. Covered by §5.7.
    Music,
}

/// The seven structural eliminators of `02-core-calculus.md` §5.6.
///
/// They are named as a closed set rather than matched out of [`Primitive`] because §5.6's proof is
/// about exactly these seven. Naming them here is what lets the checker's remaining hand-written
/// arms be exhaustive: once a primitive's family is an `Eliminator`, which one it is has already
/// been decided, and no arm is left over for the sixty-two δ-primitives to fall into by accident.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Eliminator {
    NatFold,
    ListFold,
    OptionFold,
    Map,
    Filter,
    Range,
    Repeat,
}

impl Eliminator {
    const fn arity(self) -> usize {
        match self {
            Self::NatFold | Self::ListFold | Self::OptionFold => 3,
            Self::Map | Self::Filter | Self::Repeat => 2,
            Self::Range => 1,
        }
    }
}

#[derive(Clone, Copy)]
struct PrimitiveOwnership<T> {
    operation: T,
    spelling: &'static str,
    hidden_information: &'static str,
    family: Family,
}

const BOOL: Shape = Shape::Base(Base::Bool);
const NAT: Shape = Shape::Base(Base::Nat);
const PITCH: Shape = Shape::Base(Base::Pitch);
const CLASS: Shape = Shape::Base(Base::PitchClass);
const INTERVAL: Shape = Shape::Base(Base::Interval);
const KEY: Shape = Shape::Base(Base::Key);
const SCALE: Shape = Shape::Base(Base::Scale);
const DEGREE: Shape = Shape::Base(Base::Degree);
const FRAME: Shape = Shape::Base(Base::Frame);
const CHORD: Shape = Shape::Base(Base::ChordClass);
const TRIAD: Shape = Shape::Base(Base::Triad);
const ROMAN: Shape = Shape::Base(Base::Roman);
const VOICING: Shape = Shape::Base(Base::Voicing);
const PC12: Shape = Shape::Base(Base::Pc12);
const PCSET12: Shape = Shape::Base(Base::PcSet12);
const ROW12: Shape = Shape::Base(Base::Row12);

const NATS: Shape = Shape::List(&NAT);
const PITCHES: Shape = Shape::List(&PITCH);
const INTERVALS: Shape = Shape::List(&INTERVAL);
const PC12S: Shape = Shape::List(&PC12);
const ROW12S: Shape = Shape::List(&ROW12);

const MAYBE_NAT: Shape = Shape::Option(&NAT);
const MAYBE_CLASS: Shape = Shape::Option(&CLASS);
const MAYBE_DEGREE: Shape = Shape::Option(&DEGREE);
const MAYBE_FRAME: Shape = Shape::Option(&FRAME);
const MAYBE_CHORD: Shape = Shape::Option(&CHORD);
const MAYBE_TRIAD: Shape = Shape::Option(&TRIAD);
const MAYBE_ROMAN: Shape = Shape::Option(&ROMAN);
const MAYBE_VOICING: Shape = Shape::Option(&VOICING);
const MAYBE_ROW12: Shape = Shape::Option(&ROW12);

/// A first-order signature, for the common case of writing one inline.
const fn delta(arguments: &'static [Shape], result: Shape) -> Family {
    Family::Delta { arguments, result }
}

const PRIMITIVE_OWNERSHIP: [PrimitiveOwnership<Primitive>; 69] = [
    PrimitiveOwnership {
        operation: Primitive::NatFold,
        spelling: "nat_fold",
        hidden_information: "the evaluator's finite natural representation and structural work budget",
        family: Family::Eliminator(Eliminator::NatFold),
    },
    PrimitiveOwnership {
        operation: Primitive::ListFold,
        spelling: "list_fold",
        hidden_information: "the evaluator's finite list representation and structural work budget",
        family: Family::Eliminator(Eliminator::ListFold),
    },
    PrimitiveOwnership {
        operation: Primitive::OptionFold,
        spelling: "option_fold",
        hidden_information: "the evaluator's hidden option representation and total case dispatch",
        family: Family::Eliminator(Eliminator::OptionFold),
    },
    PrimitiveOwnership {
        operation: Primitive::Map,
        spelling: "map",
        hidden_information: "rank-1 monomorphization over the evaluator's hidden finite list representation",
        family: Family::Eliminator(Eliminator::Map),
    },
    PrimitiveOwnership {
        operation: Primitive::Filter,
        spelling: "filter",
        hidden_information: "rank-1 monomorphization over the evaluator's hidden finite list representation",
        family: Family::Eliminator(Eliminator::Filter),
    },
    PrimitiveOwnership {
        operation: Primitive::Range,
        spelling: "range",
        hidden_information: "bounded construction governed by the evaluator's structural work budget",
        family: Family::Eliminator(Eliminator::Range),
    },
    PrimitiveOwnership {
        operation: Primitive::Repeat,
        spelling: "repeat",
        hidden_information: "rank-1 finite-list construction governed by the structural work budget",
        family: Family::Eliminator(Eliminator::Repeat),
    },
    PrimitiveOwnership {
        operation: Primitive::IntervalAdd,
        spelling: "interval_add",
        hidden_information: "the evaluator's exact written-interval coordinate representation",
        family: delta(&[INTERVAL, INTERVAL], INTERVAL),
    },
    PrimitiveOwnership {
        operation: Primitive::IntervalInverse,
        spelling: "interval_inverse",
        hidden_information: "the evaluator's exact written-interval coordinate representation",
        family: delta(&[INTERVAL], INTERVAL),
    },
    PrimitiveOwnership {
        operation: Primitive::PitchClassOf,
        spelling: "pitchclass_of",
        hidden_information: "the written pitch's octave coordinate and spelling-preserving quotient",
        family: delta(&[PITCH], CLASS),
    },
    PrimitiveOwnership {
        operation: Primitive::SignatureScale,
        spelling: "signature_scale",
        hidden_information: "the compiler's table of named collections, which no source text can enumerate",
        family: delta(&[KEY], SCALE),
    },
    PrimitiveOwnership {
        operation: Primitive::ScaleOn,
        spelling: "scale_on",
        hidden_information: "the scale's private ordered offset cycle, re-rooted without being exposed",
        family: delta(&[SCALE, CLASS], SCALE),
    },
    PrimitiveOwnership {
        operation: Primitive::ScaleTonic,
        spelling: "scale_tonic",
        hidden_information: "the scale's private tonic field",
        family: delta(&[SCALE], CLASS),
    },
    PrimitiveOwnership {
        operation: Primitive::ScaleSize,
        spelling: "scale_size",
        hidden_information: "the length of the scale's private offset cycle",
        family: delta(&[SCALE], NAT),
    },
    PrimitiveOwnership {
        operation: Primitive::ScalePitch,
        spelling: "scale_pitch",
        hidden_information: "spelled membership against the scale's private offset cycle",
        family: delta(&[SCALE, PITCH], MAYBE_DEGREE),
    },
    PrimitiveOwnership {
        operation: Primitive::ScaleClass,
        spelling: "scale_class",
        hidden_information: "the scale's private offset cycle, read without a register",
        family: delta(&[SCALE, DEGREE], MAYBE_CLASS),
    },
    PrimitiveOwnership {
        operation: Primitive::ScaleChord,
        spelling: "scale_chord",
        hidden_information: "the scale's offset cycle and the chord vocabulary's member table at once",
        family: delta(&[SCALE, DEGREE, NAT], MAYBE_CHORD),
    },
    PrimitiveOwnership {
        operation: Primitive::PitchFrame,
        spelling: "pitch_frame",
        hidden_information: "the register frame's representation invariant, which only the compiler can enforce",
        family: delta(&[SCALE, PITCH], MAYBE_FRAME),
    },
    PrimitiveOwnership {
        operation: Primitive::FrameScale,
        spelling: "frame_scale",
        hidden_information: "the frame's private scale field",
        family: delta(&[FRAME], SCALE),
    },
    PrimitiveOwnership {
        operation: Primitive::FrameTonic,
        spelling: "frame_tonic",
        hidden_information: "the frame's private registered tonic field",
        family: delta(&[FRAME], PITCH),
    },
    PrimitiveOwnership {
        operation: Primitive::FramePitch,
        spelling: "frame_pitch",
        hidden_information: "Euclidean division of a degree through the scale's private period",
        family: delta(&[FRAME, DEGREE], PITCH),
    },
    PrimitiveOwnership {
        operation: Primitive::DegreeOf,
        spelling: "degree_of",
        hidden_information: "the degree's private signed coordinate, which is not the written ordinal",
        family: delta(&[NAT], DEGREE),
    },
    PrimitiveOwnership {
        operation: Primitive::DegreeStepUp,
        spelling: "degree_step_up",
        hidden_information: "the degree's private signed coordinate and its machine-integer bound",
        family: delta(&[DEGREE, NAT], DEGREE),
    },
    PrimitiveOwnership {
        operation: Primitive::DegreeStepDown,
        spelling: "degree_step_down",
        hidden_information: "the degree's private signed coordinate and its machine-integer bound",
        family: delta(&[DEGREE, NAT], DEGREE),
    },
    PrimitiveOwnership {
        operation: Primitive::DegreeRaised,
        spelling: "degree_raised",
        hidden_information: "the degree's private chromatic alteration and its machine-integer bound",
        family: delta(&[DEGREE], DEGREE),
    },
    PrimitiveOwnership {
        operation: Primitive::DegreeLowered,
        spelling: "degree_lowered",
        hidden_information: "the degree's private chromatic alteration and its machine-integer bound",
        family: delta(&[DEGREE], DEGREE),
    },
    PrimitiveOwnership {
        operation: Primitive::ChordOn,
        spelling: "chord_on",
        hidden_information: "the compiler's table of chord types, re-rooted without being exposed",
        family: delta(&[CHORD, CLASS], CHORD),
    },
    PrimitiveOwnership {
        operation: Primitive::ChordRoot,
        spelling: "chord_root",
        hidden_information: "the chord class's private root field",
        family: delta(&[CHORD], CLASS),
    },
    PrimitiveOwnership {
        operation: Primitive::ChordBass,
        spelling: "chord_bass",
        hidden_information: "the chord class's private bass designation, which is absent and not the root",
        family: delta(&[CHORD], MAYBE_CLASS),
    },
    PrimitiveOwnership {
        operation: Primitive::ChordMembers,
        spelling: "chord_members",
        hidden_information: "the private spelled member stack, which no source text can enumerate",
        family: delta(&[CHORD], INTERVALS),
    },
    PrimitiveOwnership {
        operation: Primitive::ChordInversion,
        spelling: "chord_inversion",
        hidden_information: "membership of the private member stack, which is what makes an inversion true",
        family: delta(&[CHORD, NAT], MAYBE_CHORD),
    },
    PrimitiveOwnership {
        operation: Primitive::ChordOver,
        spelling: "chord_over",
        hidden_information: "the chord class's private bass designation",
        family: delta(&[CHORD, CLASS], CHORD),
    },
    PrimitiveOwnership {
        operation: Primitive::ChordTriad,
        spelling: "chord_triad",
        hidden_information: "the triad refinement's representation invariant, which only the compiler can enforce",
        family: delta(&[CHORD], MAYBE_TRIAD),
    },
    PrimitiveOwnership {
        operation: Primitive::TriadChord,
        spelling: "triad_chord",
        hidden_information: "the triad refinement's private witness",
        family: delta(&[TRIAD], CHORD),
    },
    PrimitiveOwnership {
        operation: Primitive::RomanOf,
        spelling: "roman_of",
        hidden_information: "the numeral's representation invariant, which only the compiler can enforce",
        family: delta(&[NAT, NAT, NAT], MAYBE_ROMAN),
    },
    PrimitiveOwnership {
        operation: Primitive::RomanOrdinal,
        spelling: "roman_ordinal",
        hidden_information: "the numeral's private ordinal",
        family: delta(&[ROMAN], NAT),
    },
    PrimitiveOwnership {
        operation: Primitive::RomanSize,
        spelling: "roman_size",
        hidden_information: "the numeral's private member count",
        family: delta(&[ROMAN], NAT),
    },
    PrimitiveOwnership {
        operation: Primitive::RomanInversion,
        spelling: "roman_inversion",
        hidden_information: "the numeral's private bass designation, which is a position and not a pitch",
        family: delta(&[ROMAN], NAT),
    },
    PrimitiveOwnership {
        operation: Primitive::TriadMajor,
        spelling: "triad_major",
        hidden_information: "the chord class's private type, which is the only place the two triads differ",
        family: delta(&[TRIAD], BOOL),
    },
    PrimitiveOwnership {
        operation: Primitive::VoicingOf,
        spelling: "voicing_of",
        hidden_information: "the voicing's representation invariant: ascending distinct pitches drawn from the class",
        family: delta(&[CHORD, PITCHES], MAYBE_VOICING),
    },
    PrimitiveOwnership {
        operation: Primitive::VoicingPitches,
        spelling: "voicing_pitches",
        hidden_information: "the voicing's private ordered pitch sequence",
        family: delta(&[VOICING], PITCHES),
    },
    PrimitiveOwnership {
        operation: Primitive::VoicingBass,
        spelling: "voicing_bass",
        hidden_information: "the voicing's private lowest pitch, held apart from the rest",
        family: delta(&[VOICING], PITCH),
    },
    PrimitiveOwnership {
        operation: Primitive::VoicingChord,
        spelling: "voicing_chord",
        hidden_information: "the voicing's private association to the class it voices",
        family: delta(&[VOICING], CHORD),
    },
    PrimitiveOwnership {
        operation: Primitive::VoicingPosition,
        spelling: "voicing_position",
        hidden_information: "membership of the private member stack, which is what classifies an inversion",
        family: delta(&[VOICING], MAYBE_NAT),
    },
    PrimitiveOwnership {
        operation: Primitive::CloseVoicing,
        spelling: "close_voicing",
        hidden_information: "the private member stack walked upward, and the voicing invariant it must satisfy",
        family: delta(&[CHORD, PITCH], MAYBE_VOICING),
    },
    PrimitiveOwnership {
        operation: Primitive::DropVoicing,
        spelling: "drop_voicing",
        hidden_information: "the private member stack walked upward, and the voicing invariant it must satisfy",
        family: delta(&[CHORD, PITCH, NAT], MAYBE_VOICING),
    },
    PrimitiveOwnership {
        operation: Primitive::OmitVoicing,
        spelling: "omit_voicing",
        hidden_information: "the private member stack, which is what says which pitch an omission removes",
        family: delta(&[VOICING, NAT], MAYBE_VOICING),
    },
    PrimitiveOwnership {
        operation: Primitive::Pc12Of,
        spelling: "pc12_of",
        hidden_information: "the canonical representative of a residue class modulo twelve",
        family: delta(&[NAT], PC12),
    },
    PrimitiveOwnership {
        operation: Primitive::Pc12Number,
        spelling: "pc12_number",
        hidden_information: "the canonical representative, which is the only number a residue class has",
        family: delta(&[PC12], NAT),
    },
    PrimitiveOwnership {
        operation: Primitive::Pc12Forget,
        spelling: "pc12_forget",
        hidden_information: "the chromatic coordinate of a spelled pitch class, taken modulo twelve",
        family: delta(&[CLASS], PC12),
    },
    PrimitiveOwnership {
        operation: Primitive::Pc12Transposed,
        spelling: "pc12_transposed",
        hidden_information: "modular addition, which a `nat` without subtraction cannot express",
        family: delta(&[PC12, NAT], PC12),
    },
    PrimitiveOwnership {
        operation: Primitive::Pc12Inverted,
        spelling: "pc12_inverted",
        hidden_information: "modular subtraction, which a `nat` without subtraction cannot express",
        family: delta(&[PC12, NAT], PC12),
    },
    PrimitiveOwnership {
        operation: Primitive::Pc12Spelled,
        spelling: "pc12_spelled",
        hidden_information: "the collection's spelled members, searched for the one this class forgets to",
        family: delta(&[PC12, SCALE], MAYBE_CLASS),
    },
    PrimitiveOwnership {
        operation: Primitive::PcSet12Of,
        spelling: "pcset12_of",
        hidden_information: "the twelve-bit membership word that makes duplication unrepresentable",
        family: delta(&[PC12S], PCSET12),
    },
    PrimitiveOwnership {
        operation: Primitive::PcSet12Members,
        spelling: "pcset12_members",
        hidden_information: "the membership word, read out ascending",
        family: delta(&[PCSET12], PC12S),
    },
    PrimitiveOwnership {
        operation: Primitive::PcSet12Normal,
        spelling: "pcset12_normal",
        hidden_information: "every rotation of the set and the compactness order that chooses between them",
        family: delta(&[PCSET12], PC12S),
    },
    PrimitiveOwnership {
        operation: Primitive::PcSet12Prime,
        spelling: "pcset12_prime",
        hidden_information: "the normal orders of the set and its inversion, and which of the two reads lower",
        family: delta(&[PCSET12], PCSET12),
    },
    PrimitiveOwnership {
        operation: Primitive::PcSet12Vector,
        spelling: "pcset12_vector",
        hidden_information: "every unordered pair of members and the interval class each realizes",
        family: delta(&[PCSET12], NATS),
    },
    PrimitiveOwnership {
        operation: Primitive::Row12Of,
        spelling: "row12_of",
        hidden_information: "the permutation invariant: twelve order positions and each pitch class once",
        family: delta(&[PC12S], MAYBE_ROW12),
    },
    PrimitiveOwnership {
        operation: Primitive::Row12Pcs,
        spelling: "row12_pcs",
        hidden_information: "the private order-position array",
        family: delta(&[ROW12], PC12S),
    },
    PrimitiveOwnership {
        operation: Primitive::Row12Head,
        spelling: "row12_head",
        hidden_information: "order position zero of the private array, which the finite list eliminators cannot index",
        family: delta(&[ROW12], PC12),
    },
    PrimitiveOwnership {
        operation: Primitive::Row12Transposed,
        spelling: "row12_transposed",
        hidden_information: "modular addition, and the finite-closure lemma that keeps the result a row",
        family: delta(&[ROW12, NAT], ROW12),
    },
    PrimitiveOwnership {
        operation: Primitive::Row12Inverted,
        spelling: "row12_inverted",
        hidden_information: "modular subtraction, and the finite-closure lemma that keeps the result a row",
        family: delta(&[ROW12, NAT], ROW12),
    },
    PrimitiveOwnership {
        operation: Primitive::Row12Retrograde,
        spelling: "row12_retrograde",
        hidden_information: "reversal of the order positions, which the finite list eliminators cannot express",
        family: delta(&[ROW12], ROW12),
    },
    PrimitiveOwnership {
        operation: Primitive::Row12Matrix,
        spelling: "row12_matrix",
        hidden_information: "the classical construction: the inversion about the row's own head, read as starting pitches",
        family: delta(&[ROW12], ROW12S),
    },
    PrimitiveOwnership {
        operation: Primitive::Row12Forms,
        spelling: "row12_forms",
        hidden_information: "the forty-eight labelled forms, compared for equality and counted once each",
        family: delta(&[ROW12], NAT),
    },
    PrimitiveOwnership {
        operation: Primitive::Row12Symmetries,
        spelling: "row12_symmetries",
        hidden_information: "the forty-eight labelled forms, counted where they fix the row",
        family: delta(&[ROW12], NAT),
    },
    PrimitiveOwnership {
        operation: Primitive::Row12Repeats,
        spelling: "row12_repeats",
        hidden_information: "pitch-class equality, which the surface has no operator for",
        family: delta(&[PC12S], NATS),
    },
    PrimitiveOwnership {
        operation: Primitive::Row12Missing,
        spelling: "row12_missing",
        hidden_information: "pitch-class equality against the whole finite domain",
        family: delta(&[PC12S], PC12S),
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
            Self::ScaleClass => "scale_class",
            Self::ScaleChord => "scale_chord",
            Self::PitchFrame => "pitch_frame",
            Self::FrameScale => "frame_scale",
            Self::FrameTonic => "frame_tonic",
            Self::FramePitch => "frame_pitch",
            Self::DegreeOf => "degree_of",
            Self::DegreeStepUp => "degree_step_up",
            Self::DegreeStepDown => "degree_step_down",
            Self::DegreeRaised => "degree_raised",
            Self::DegreeLowered => "degree_lowered",
            Self::ChordOn => "chord_on",
            Self::ChordRoot => "chord_root",
            Self::ChordBass => "chord_bass",
            Self::ChordMembers => "chord_members",
            Self::ChordInversion => "chord_inversion",
            Self::ChordOver => "chord_over",
            Self::ChordTriad => "chord_triad",
            Self::TriadChord => "triad_chord",
            Self::TriadMajor => "triad_major",
            Self::RomanOf => "roman_of",
            Self::RomanOrdinal => "roman_ordinal",
            Self::RomanSize => "roman_size",
            Self::RomanInversion => "roman_inversion",
            Self::VoicingOf => "voicing_of",
            Self::VoicingPitches => "voicing_pitches",
            Self::VoicingBass => "voicing_bass",
            Self::VoicingChord => "voicing_chord",
            Self::VoicingPosition => "voicing_position",
            Self::CloseVoicing => "close_voicing",
            Self::DropVoicing => "drop_voicing",
            Self::OmitVoicing => "omit_voicing",
            Self::Pc12Of => "pc12_of",
            Self::Pc12Number => "pc12_number",
            Self::Pc12Forget => "pc12_forget",
            Self::Pc12Transposed => "pc12_transposed",
            Self::Pc12Inverted => "pc12_inverted",
            Self::Pc12Spelled => "pc12_spelled",
            Self::PcSet12Of => "pcset12_of",
            Self::PcSet12Members => "pcset12_members",
            Self::PcSet12Normal => "pcset12_normal",
            Self::PcSet12Prime => "pcset12_prime",
            Self::PcSet12Vector => "pcset12_vector",
            Self::Row12Of => "row12_of",
            Self::Row12Pcs => "row12_pcs",
            Self::Row12Head => "row12_head",
            Self::Row12Transposed => "row12_transposed",
            Self::Row12Inverted => "row12_inverted",
            Self::Row12Retrograde => "row12_retrograde",
            Self::Row12Matrix => "row12_matrix",
            Self::Row12Forms => "row12_forms",
            Self::Row12Symmetries => "row12_symmetries",
            Self::Row12Repeats => "row12_repeats",
            Self::Row12Missing => "row12_missing",
        }
    }
}

/// One `assert` statement's claim, checked and waiting to be evaluated.
///
/// The predicate is the registry's, not a name in scope: which claims exist is
/// `crate::assert::CLAIMS`, and this is a pointer into it. What varies is the
/// arguments, and each carries the shape the registry declared for it, so
/// evaluation reads a `Scale` where a scale was asked for and cannot be handed
/// something else by an expression that happened to check.
#[derive(Clone)]
struct CheckedClaim {
    predicate: &'static crate::assert::Predicate,
    arguments: Vec<CheckedArgument>,
}

/// One argument of a claim, in the shape the registry declared.
///
/// A policy is not an expression, and neither is a rule id: their inhabitants
/// are words, read straight off the source, because a type in the value
/// language that no function can take or return would be surface with no
/// caller.
#[derive(Clone)]
enum CheckedArgument {
    Scale(Expr),
    Chord(Expr),
    Count(Expr),
    Ranges(Expr),
    Policy(crate::assert::Realization),
    Rule(crate::analysis::RuleName),
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
    ChordClass(crate::chord::ChordClass),
    Triad(crate::chord::Triad),
    Roman(crate::roman::Roman),
    Voicing(crate::chord::Voicing),
    Pc12(crate::pc12::Pc12),
    PcSet12(crate::pc12::PcSet12),
    Row12(crate::pc12::Row12),
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
    Play,
}

const BUILTIN_OWNERSHIP: [PrimitiveOwnership<Builtin>; 8] = [
    PrimitiveOwnership {
        operation: Builtin::Transpose,
        spelling: "transpose",
        hidden_information: "contextual music representation, written-pitch provenance, and kernel construction",
        family: Family::Music,
    },
    PrimitiveOwnership {
        operation: Builtin::Stretch,
        spelling: "stretch",
        hidden_information: "contextual music representation, exact-time provenance, and kernel construction",
        family: Family::Music,
    },
    PrimitiveOwnership {
        operation: Builtin::Retrograde,
        spelling: "retrograde",
        hidden_information: "contextual music extent, occurrence provenance, and kernel construction",
        family: Family::Music,
    },
    PrimitiveOwnership {
        operation: Builtin::Invert,
        spelling: "invert",
        hidden_information: "contextual music representation, written-pitch provenance, and kernel construction",
        family: Family::Music,
    },
    PrimitiveOwnership {
        operation: Builtin::Shift,
        spelling: "shift",
        hidden_information: "contextual music representation, exact-time provenance, and kernel construction",
        family: Family::Music,
    },
    PrimitiveOwnership {
        operation: Builtin::Overlay,
        spelling: "overlay",
        hidden_information: "contextual music representation, origin paths, and kernel overlay construction",
        family: Family::Music,
    },
    PrimitiveOwnership {
        operation: Builtin::MapNotePitches,
        spelling: "map_note_pitches",
        hidden_information: "controlled traversal of contextual notes while preserving non-note facts and provenance",
        family: Family::Music,
    },
    PrimitiveOwnership {
        operation: Builtin::Play,
        spelling: "play",
        hidden_information: "contextual music construction: the voicing's private pitches become sounded occurrences with provenance",
        family: Family::Music,
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
            Self::Play => vec![Type::Voicing, Type::Duration],
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
    /// The claim each `assert` statement makes, keyed by the statement's own
    /// span. Evaluated here because a claim's arguments are ordinary values —
    /// `scale c major`, `chord c major7`, a list of pitch pairs — and the
    /// value language is what evaluates values.
    pub(crate) claims: Box<IndexMap<u64, crate::assert::Claim>>,
    /// Keys named rather than written out: `key k;` in a template's body,
    /// resolved once the instance's argument is known. Only a document's
    /// root music carries any — a `music` value may not change the context
    /// it is used in, so a key statement inside one is already an error.
    pub(crate) keys: Box<IndexMap<u64, crate::Key>>,
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
    Transpose {
        interval: Interval,
        source: Music,
    },
    Stretch {
        factor: Ratio<i64>,
        source: Music,
    },
    Retrograde {
        source: Music,
    },
    Invert {
        axis: WrittenPitch,
        source: Music,
    },
    Shift {
        by: Ratio<i64>,
        source: Music,
    },
    Overlay {
        left: Music,
        right: Box<Music>,
    },
    MapNotePitches {
        mapper: PitchFunction,
        source: Music,
    },
    /// A checked kernel quote: a closed term over the fresh names its holes
    /// bind, and the host music each name stands for, with the locus the
    /// quote's own structure puts that name at.
    ///
    /// Unlike every other operation here, the material is already assembled —
    /// the term *is* the composition. Elaboration binds the holes and reads
    /// the extent off the structure; it never rebuilds the term.
    KernelQuote {
        /// The quoted term, over the hole names.
        term: musa_kernel::Term<crate::elaborate::ScoreFact>,
        /// Each hole: its fresh name, its locus in the quote, and its music.
        holes: Vec<(String, Ratio<i64>, Music)>,
    },
    /// The one music constructor with no music underneath it: a chosen
    /// voicing, sounded for a written length.
    Play {
        voicing: crate::chord::Voicing,
        held: Ratio<i64>,
    },
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

    pub(crate) fn key_at(&self, span: SourceSpan) -> Option<crate::Key> {
        self.keys.get(&span_key(span)).copied()
    }

    pub(crate) fn claim_at(&self, span: SourceSpan) -> Option<&crate::assert::Claim> {
        self.claims.get(&span_key(span))
    }
}

/// Checked root `use` expressions. This is the only bridge from the total
/// value evaluator into contextual score elaboration.
pub(crate) struct Program {
    uses: IndexMap<u64, Music>,
    pitches: IndexMap<u64, PitchTerm>,
    scales: IndexMap<u64, crate::scale::Scale>,
    claims: IndexMap<u64, crate::assert::Claim>,
    keys: IndexMap<u64, crate::Key>,
    named_music: IndexMap<String, Music>,
    values: IndexMap<String, Value>,
}

impl Program {
    pub(crate) fn root_music(&self) -> Music {
        Music {
            items: Vec::new(),
            uses: self.uses.clone(),
            pitches: Box::new(self.pitches.clone()),
            scales: Box::new(self.scales.clone()),
            claims: Box::new(self.claims.clone()),
            keys: Box::new(self.keys.clone()),
            bindings: IndexMap::new(),
            role: None,
            definition_span: SourceSpan::default(),
            operation: None,
        }
    }

    /// Rebind what a hidden argument holder evaluated to under the name the
    /// template's body reads it by.
    ///
    /// This is the whole of "substitute the arguments": the value crosses
    /// from the pass that evaluated it, in the site's scope, into the pass
    /// that checks the body, under the parameter's own name. Absent when the
    /// site's argument did not evaluate — the diagnostic for that was
    /// reported where the argument is written.
    pub(crate) fn rebind(
        &self,
        holder: &str,
        name: String,
        name_span: SourceSpan,
        span: SourceSpan,
        ty: SyntaxNode,
    ) -> Option<Binding> {
        let value = self.values.get(holder)?.clone();
        Some(Binding {
            name,
            name_span,
            span,
            ty,
            stands_for: StandsFor::Value(Box::new(value)),
            hidden: false,
        })
    }

    /// A key the header names rather than spells, at the statement's span.
    pub(crate) fn key_at(&self, span: SourceSpan) -> Option<crate::Key> {
        self.keys.get(&span_key(span)).copied()
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
            Self::ChordClass(_) => Type::ChordClass,
            Self::Triad(_) => Type::Triad,
            Self::Roman(_) => Type::Roman,
            Self::Voicing(_) => Type::Voicing,
            Self::Pc12(_) => Type::Pc12,
            Self::PcSet12(_) => Type::PcSet12,
            Self::Row12(_) => Type::Row12,
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
    /// before the private environment is discarded by the caller.
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
            Self::ChordClass(value) => chord_witness(*value),
            Self::Triad(value) => chord_witness(value.class()).rotate_left(1),
            Self::Roman(value) => value
                .ordinal()
                .rotate_left(5)
                .wrapping_add(value.members().rotate_left(10))
                .wrapping_add(value.inversion()),
            Self::Pc12(value) => u64::from(value.number()),
            Self::PcSet12(value) => value.members().fold(0u64, |witness, member| {
                witness.rotate_left(5) ^ u64::from(member.number())
            }),
            Self::Row12(value) => value.pcs().fold(1u64, |witness, member| {
                witness.rotate_left(5) ^ u64::from(member.number())
            }),
            Self::Voicing(value) => value.pitches().fold(chord_witness(value.class()), |witness, pitch| {
                witness.rotate_left(5) ^ Self::Pitch(pitch).normalization_witness()
            }),
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
/// A chord class read into a witness the same way a scale is: the root, then
/// the table row, so two classes that print differently traverse differently.
fn chord_witness(chord: crate::chord::ChordClass) -> u64 {
    let bass = chord
        .bass()
        .map_or(0, |bass| Value::PitchClass(bass).normalization_witness());
    Value::PitchClass(chord.root()).normalization_witness().rotate_left(3)
        ^ u64::try_from(chord.members().len()).unwrap_or(0)
        ^ chord
            .kind()
            .name()
            .bytes()
            .fold(0u64, |witness, byte| witness.rotate_left(5) ^ u64::from(byte))
        ^ bass.rotate_left(9)
}

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
        Some(MusicOperation::Play { voicing, held }) => {
            Value::Voicing(voicing.clone()).normalization_witness().rotate_left(11)
                ^ Value::Duration(*held).normalization_witness()
        }
        Some(MusicOperation::KernelQuote { term, holes }) => {
            holes
                .iter()
                .fold(term.occurrence_bound().rotate_left(13), |witness, (_, locus, music)| {
                    witness.rotate_left(5) ^ Value::Ratio(*locus).normalization_witness() ^ music_witness(music)
                })
        }
    };
    base.rotate_left(3) ^ operation
}

fn check_and_evaluate(
    resolver: &mut Resolver,
    declarations: impl Iterator<Item = SurfaceDefinition>,
    root: Option<&SyntaxNode>,
    unknown_root_music: UnknownRootMusic,
    modules: &Modules,
) -> Option<Program> {
    let root_uses = root.map(root_uses).unwrap_or_default();
    let mut meter = WorkMeter::default();
    let mut raw = Vec::new();
    // Per bound name: where it was bound, which library it came from if it
    // came from one, and whether it was a legacy material declaration.
    let mut names: IndexMap<String, (SourceSpan, Option<String>, bool)> = IndexMap::new();
    // A module's members and a functor's arguments are ordinary definitions
    // in the one flat namespace: what a module changes is the name they are
    // filed under and the scope their bodies read in, never the pass.
    let declarations = modules
        .members()
        .iter()
        .map(|member| SurfaceDefinition::Member {
            name: member.name.clone(),
            item: member.item.clone(),
            scope: member.scope.clone(),
            source: member.source.clone(),
        })
        .chain(modules.arguments().iter().map(|argument| {
            SurfaceDefinition::Bound(Binding::argument(
                argument.holder.clone(),
                argument.name_span,
                argument.span,
                argument.ty.clone(),
                argument.expr.clone(),
                true,
            ))
        }))
        .chain(declarations);
    for declaration in declarations {
        let is_legacy = matches!(declaration, SurfaceDefinition::Legacy { .. });
        let (name, name_span, span, source) = surface_identity(&declaration)?;
        if let Some((first, first_source, first_is_legacy)) = names.get(&name).cloned() {
            // The structural resolver retains the established, role-specific
            // diagnostic for two legacy material declarations. The core must
            // still see only the first definition, but reporting here as well
            // would turn one source mistake into two diagnostics.
            if is_legacy && first_is_legacy {
                continue;
            }
            // Two imports exporting one name is the price of flat binding, so
            // it is paid where it is incurred: the diagnostic names both
            // modules, and `as` on either import resolves it.
            let mut diagnostic = match (&first_source, &source) {
                (Some(first_path), Some(path)) => Diagnostic::error(
                    Code::DuplicateName,
                    format!("`{first_path}` and `{path}` both declare `{name}`"),
                )
                .at(name_span, "imported again here")
                .help("qualify one of the imports with `as`, so its names are reached through it"),
                _ => Diagnostic::error(Code::DuplicateName, format!("`{name}` is bound twice"))
                    .at(name_span, "bound again here")
                    .help("give one of the bindings a different name"),
            };
            if first_source.is_none() {
                diagnostic = diagnostic.also(first, "first bound here");
            }
            resolver.report(diagnostic);
            continue;
        }
        names.insert(name.clone(), (name_span, source.clone(), is_legacy));
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
        if !definition.foreign && !definition.hidden && definition.role.is_none() {
            resolver
                .references
                .declare(definition.name_kind(), &definition.name, definition.name_span);
        }
        if !definition.hidden {
            resolver.references.document(document(definition));
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
            scope: &definition.scope,
            modules,
        };
        let kind = match &definition.kind {
            RawDefinitionKind::Bound { value } => Some(CheckedDefinitionKind::Let {
                body: Expr {
                    kind: ExprKind::Literal(value.as_ref().clone()),
                    ty: definition.ty.clone(),
                    span: definition.span,
                },
            }),
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
                            claims: Box::default(),
                            keys: Box::default(),
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
            scope: crate::module::NameScope::empty(),
            modules,
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
    let mut claims = IndexMap::new();
    let mut keys = IndexMap::new();
    let mut pitches = IndexMap::new();
    if let Some(root) = root {
        for statement in root_nodes(root, SyntaxKind::InScaleStmt) {
            let Some(expression) = child_of(&statement, is_expr_node) else {
                continue;
            };
            let span = crate::resolve::trimmed_span(&statement);
            let mut checker = root_checker(resolver, &raw, &symbols, &mut meter, span, modules);
            let checked = checker.check(&expression, Some(&Type::Scale))?;
            let Value::Scale(scale) = eval(&checked, &values, &mut meter)? else {
                return None;
            };
            scales.insert(span_key(span), scale);
        }
        // `key k;` — a key the source names rather than spells. The written
        // form has no expression child at all, so it never reaches here and
        // the two spellings stay one statement.
        for statement in root_nodes(root, SyntaxKind::KeyStmt) {
            let Some(expression) = child_of(&statement, is_expr_node) else {
                continue;
            };
            let span = crate::resolve::trimmed_span(&statement);
            let mut checker = root_checker(resolver, &raw, &symbols, &mut meter, span, modules);
            let checked = checker.check(&expression, Some(&Type::Key))?;
            let Value::Key(key) = eval(&checked, &values, &mut meter)? else {
                return None;
            };
            keys.insert(span_key(span), key);
        }
        for statement in root_nodes(root, SyntaxKind::AssertStmt) {
            let span = crate::resolve::trimmed_span(&statement);
            let mut checker = root_checker(resolver, &raw, &symbols, &mut meter, span, modules);
            let checked = checker.claim(&statement)?;
            claims.insert(span_key(span), eval_claim(&checked, &values, &mut meter)?);
        }
        for statement in root_nodes(root, SyntaxKind::NoteStmt) {
            let Some(expression) =
                musa_language::ast::NoteStmt::cast(statement.clone()).and_then(|note| note.pitch_expr())
            else {
                continue;
            };
            let span = crate::resolve::trimmed_span(&statement);
            let mut checker = root_checker(resolver, &raw, &symbols, &mut meter, span, modules);
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
            | Value::ChordClass(_)
            | Value::Triad(_)
            | Value::Roman(_)
            | Value::Voicing(_)
            | Value::Pc12(_)
            | Value::PcSet12(_)
            | Value::Row12(_)
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
        claims,
        keys,
        named_music,
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
    modules: &'a Modules,
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
        scope: crate::module::NameScope::empty(),
        modules,
    }
}

#[derive(Clone, Copy)]
enum UnknownRootMusic {
    Reject,
    Defer,
    Silent,
}

/// Check and evaluate on the import-free interchange path (`piece_term`):
/// the document's own preamble, whatever a template instance bound, and the
/// items of `scope` when there is a declaration to read them from.
///
/// One function rather than one per caller because the interchange path has
/// exactly one rule — no imports, and a `use` no local declaration supplies
/// is silence rather than an error — and that rule is the same whether the
/// scope is a piece, a template's voice, or nothing at all.
pub(crate) fn check_for_kernel(
    resolver: &mut Resolver,
    root: &SyntaxNode,
    scope: Option<&SyntaxNode>,
    bindings: Vec<Binding>,
) -> Option<Program> {
    let modules = Modules::read(resolver, std::iter::once((None, root.clone())));
    check_and_evaluate(
        resolver,
        root_preamble(root)
            .into_iter()
            .chain(bindings.into_iter().map(SurfaceDefinition::Bound))
            .chain(scope.map(|node| declarations(node, None)).unwrap_or_default()),
        scope,
        UnknownRootMusic::Silent,
        &modules,
    )
}

/// The name a [`SyntaxKind::NameExpr`] writes, starting at its first name
/// token: one identifier, or the two words of a `Module.member` path joined
/// the way the flat namespace holds it.
fn qualified_name(node: &SyntaxNode, first: &SyntaxToken) -> String {
    let mut name = first.text().to_owned();
    let mut rest = significant_tokens(node).skip_while(|token| token != first).skip(1);
    if rest.next().is_some_and(|token| token.kind() == SyntaxKind::Dot)
        && let Some(member) = rest.next().filter(|token| token.kind() == SyntaxKind::Identifier)
    {
        name.push('.');
        name.push_str(member.text());
    }
    name
}

fn surface_identity(definition: &SurfaceDefinition) -> Option<(String, SourceSpan, SourceSpan, Option<String>)> {
    let (syntax, name, source, qualifier) = match definition {
        SurfaceDefinition::Let {
            declaration,
            source,
            qualifier,
        } => (declaration.syntax(), declaration.name(), source.clone(), qualifier),
        SurfaceDefinition::Function {
            declaration,
            source,
            qualifier,
        } => (declaration.syntax(), declaration.name(), source.clone(), qualifier),
        SurfaceDefinition::Legacy {
            name,
            syntax,
            source,
            qualifier,
            ..
        } => (syntax, Some(name.clone()), source.clone(), qualifier),
        SurfaceDefinition::Bound(binding) => {
            return Some((binding.name.clone(), binding.name_span, binding.span, None));
        }
        SurfaceDefinition::Member { name, item, source, .. } => {
            let syntax = match item {
                crate::module::MemberItem::Let(declaration) => declaration.syntax(),
                crate::module::MemberItem::Function(declaration) => declaration.syntax(),
            };
            return Some((
                name.clone(),
                crate::resolve::token_span(syntax, SyntaxKind::Identifier)?,
                crate::resolve::trimmed_span(syntax),
                source.clone(),
            ));
        }
    };
    let name = name?;
    // A qualified import reaches its names as `alias.name` and by nothing
    // else: the point of writing `as` is that the bare name was ambiguous.
    let name = match qualifier {
        Some(qualifier) => format!("{qualifier}{}{name}", crate::module::DOT),
        None => name,
    };
    let name_span = crate::resolve::token_span(syntax, SyntaxKind::Identifier)?;
    Some((name, name_span, crate::resolve::trimmed_span(syntax), source))
}

/// What an editor is told about one checked declaration (`crate::docs`).
///
/// Read off the lowered definition rather than off the text: the signature is
/// the type the checker settled on, so an editor cannot show a reader a
/// signature the compiler disagrees with. The declaring word is chosen from
/// the kind, so a motif reads as a motif and a `let` as a `let`.
fn document(definition: &RawDefinition) -> crate::docs::ItemDoc {
    let kind = definition.name_kind();
    let parameters = match &definition.kind {
        RawDefinitionKind::Function { parameters, .. } | RawDefinitionKind::Music { parameters, .. } => parameters
            .iter()
            .map(|parameter| {
                let ty = crate::docs::TypeNote::new(parameter.ty.to_string());
                let label = match &parameter.written_default {
                    Some(default) => format!("{}: {} = {default}", parameter.name, ty.name),
                    None => format!("{}: {}", parameter.name, ty.name),
                };
                crate::docs::ParameterDoc {
                    name: parameter.name.clone(),
                    label,
                    ty,
                    default: parameter.written_default.clone(),
                }
            })
            .collect(),
        RawDefinitionKind::Let { .. } | RawDefinitionKind::Bound { .. } => Vec::new(),
    };
    // A callable evaluates to its result; everything else evaluates to itself.
    let result = crate::docs::TypeNote::new(if let Type::Function(_, result) = &definition.ty {
        result.to_string()
    } else {
        definition.ty.to_string()
    });
    // Every declaration the core lowers names a value; static structure is
    // documented where it is declared, in `crate::module`.
    let result = Some(result);
    let word = match kind {
        NameKind::Value | NameKind::Function => {
            if parameters.is_empty() {
                "let"
            } else {
                "fn"
            }
        }
        NameKind::Motif => "motif",
        NameKind::Bar => "bar",
        NameKind::Fragment => "fragment",
        NameKind::Part => "part",
        NameKind::Voice => "voice",
        NameKind::Patch => "patch",
        NameKind::Module => "signature",
        NameKind::Template => "template",
    };
    let mut signature = format!("{word} {}", definition.name);
    if !parameters.is_empty() {
        signature.push('(');
        for (index, parameter) in parameters.iter().enumerate() {
            if index > 0 {
                signature.push_str(", ");
            }
            signature.push_str(&parameter.label);
        }
        signature.push(')');
    }
    // A motif's result is `Music` by construction, and saying so adds a word
    // to every line without adding a fact. Everything else states it.
    if !matches!(kind, NameKind::Motif | NameKind::Fragment | NameKind::Bar) {
        signature.push_str(if parameters.is_empty() { ": " } else { " -> " });
        if let Some(result) = &result {
            signature.push_str(&result.name);
        }
    }
    crate::docs::ItemDoc {
        name: definition.name.clone(),
        kind,
        source: crate::docs::ItemSource {
            uri: definition.source.clone(),
            span: definition.name_span,
            read_only: definition
                .source
                .as_deref()
                .is_some_and(|uri| crate::imports::standard_library_source(uri).is_some()),
        },
        deprecation: definition.summary.as_deref().and_then(crate::docs::deprecation_in),
        summary: definition.summary.clone(),
        signature,
        result,
        parameters,
    }
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
            let summary = crate::docs::summary_above(declaration.syntax());
            Some(RawDefinition {
                name,
                ty,
                kind: RawDefinitionKind::Let { body },
                name_span,
                span,
                foreign,
                source,
                role: None,
                hidden: false,
                scope: crate::module::NameScope::default(),
                summary,
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
                let written = child_of(parameter.syntax(), is_expr_node);
                parameters.push(RawParameter {
                    name: parameter_name,
                    ty: parameter_ty,
                    written_default: written.as_ref().map(|node| node.text().to_string().trim().to_owned()),
                    default: written.map(RawDefault::Expression),
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
            let summary = crate::docs::summary_above(declaration.syntax());
            Some(RawDefinition {
                name,
                ty,
                kind: RawDefinitionKind::Function { parameters, body },
                name_span,
                span,
                foreign,
                source,
                role: None,
                hidden: false,
                scope: crate::module::NameScope::default(),
                summary,
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
                    "Pitch" => Type::Pitch,
                    "Duration" => Type::Duration,
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
                    written_default: parameter.default,
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
            let summary = crate::docs::summary_above(&syntax);
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
                hidden: false,
                scope: crate::module::NameScope::default(),
                summary,
            })
        }
        SurfaceDefinition::Member {
            item, scope: member, ..
        } => {
            // A member is lowered exactly as the same declaration written at
            // a document's root would be. All a module changes is the name it
            // is filed under and the scope its body reads in.
            let inner = match item {
                crate::module::MemberItem::Let(declaration) => SurfaceDefinition::Let {
                    declaration,
                    source: source.clone(),
                    // The name is already qualified by the module it belongs
                    // to; qualifying it again would file it under two dots.
                    qualifier: None,
                },
                crate::module::MemberItem::Function(declaration) => SurfaceDefinition::Function {
                    declaration,
                    source: source.clone(),
                    qualifier: None,
                },
            };
            let mut lowered = lower_signature(resolver, inner, name, name_span, span, source)?;
            lowered.scope = member;
            Some(lowered)
        }
        SurfaceDefinition::Bound(binding) => {
            let ty = parse_type(resolver, &binding.ty)?;
            let kind = match binding.stands_for {
                StandsFor::Argument(argument) => RawDefinitionKind::Let { body: argument },
                StandsFor::Value(value) => {
                    // A value that reached here already type-checked once, at
                    // the site that produced it. Restating the type is how
                    // the second pass proves that, rather than assuming it.
                    if value.ty() != ty {
                        resolver.report(
                            Diagnostic::error(
                                Code::TypeMismatch,
                                format!("`{name}` was given a {} where a {} was declared", value.ty(), ty),
                            )
                            .at(span, "this argument"),
                        );
                        return None;
                    }
                    RawDefinitionKind::Bound { value }
                }
            };
            Some(RawDefinition {
                name,
                ty,
                kind,
                name_span,
                span,
                foreign: false,
                source: None,
                role: None,
                hidden: binding.hidden,
                scope: crate::module::NameScope::default(),
                // A template's argument is documented at the template, not at
                // the binding a `make` site produced for it.
                summary: None,
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
        | Type::ChordClass
        | Type::Triad
        | Type::Roman
        | Type::Voicing
        | Type::Pc12
        | Type::PcSet12
        | Type::Row12
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
        | Type::ChordClass
        | Type::Triad
        | Type::Roman
        | Type::Voicing
        | Type::Pc12
        | Type::PcSet12
        | Type::Row12
        | Type::Music
        | Type::Product(_)
        | Type::Option(_)
        | Type::List(_) => None,
    }
}

fn parse_type(resolver: &mut Resolver, node: &SyntaxNode) -> Option<Type> {
    lower_type(Some(resolver), node)
}

/// The type a node declares, read without reporting what it is not.
///
/// The module stage asks a member what its type *is*, in order to match it
/// against a signature; whether the type exists at all is a question the core
/// answers once, where the declaration is lowered, so asking here would
/// report the same mistake twice.
pub(crate) fn declared_type(node: &SyntaxNode) -> Option<Type> {
    lower_type(None, node)
}

/// The type a signature member declares.
///
/// Reporting, unlike [`declared_type`]: a signature member's type is read
/// exactly once, here, so this is the only place that can say it is not a
/// type at all.
pub(crate) fn signature_type(resolver: &mut Resolver, node: &SyntaxNode) -> Option<Type> {
    lower_type(Some(resolver), node)
}

/// The arrow type a `fn` declares, which is the type a signature member of
/// arrow type must match.
pub(crate) fn function_type(declaration: &FnDecl) -> Option<Type> {
    let mut parameters = Vec::new();
    for parameter in declaration.params() {
        let ty_node = child_of(parameter.syntax(), is_type_node)?;
        parameters.push(declared_type(&ty_node)?);
    }
    let result = declaration
        .syntax()
        .children()
        .filter(|node| is_type_node(node.kind()))
        .last()?;
    Some(Type::Function(parameters, Box::new(declared_type(&result)?)))
}

/// The type a written name denotes, for the names the compiler owns.
///
/// The spellings are `musa-language`'s `PRIMITIVE_TYPES`, which is where the
/// language server and the parser read them too; this is the one place that
/// says which [`Type`] each of them is.
fn named_type(text: &str) -> Option<Type> {
    match text {
        "Unit" => Some(Type::Unit),
        "Bool" => Some(Type::Bool),
        "Nat" => Some(Type::Nat),
        "Ratio" => Some(Type::Ratio),
        "Duration" => Some(Type::Duration),
        "Pitch" => Some(Type::Pitch),
        "NoteName" => Some(Type::PitchClass),
        "Interval" => Some(Type::Interval),
        "Scale" => Some(Type::Scale),
        "Key" => Some(Type::Key),
        "Degree" => Some(Type::Degree),
        "Frame" => Some(Type::Frame),
        "ChordClass" => Some(Type::ChordClass),
        "Triad" => Some(Type::Triad),
        "Roman" => Some(Type::Roman),
        "Voicing" => Some(Type::Voicing),
        "Pc12" => Some(Type::Pc12),
        "PcSet12" => Some(Type::PcSet12),
        "Row12" => Some(Type::Row12),
        "Music" => Some(Type::Music),
        _ => None,
    }
}

fn lower_type(mut resolver: Option<&mut Resolver>, node: &SyntaxNode) -> Option<Type> {
    let kind = node.kind();
    if kind == SyntaxKind::TypeExpr {
        return child_of(node, is_type_node).and_then(|child| lower_type(resolver, &child));
    }
    if kind == SyntaxKind::TypeName {
        let text = node.to_string();
        let text = text.trim();
        if let Some(named) = named_type(text) {
            return Some(named);
        }
        // A removed spelling has already been reported at the word, with the
        // capital that replaces it, by the parser. Reading it as the type it
        // named leaves the rest of the declaration checked and keeps the
        // file's one complaint one complaint.
        if let Some(now) = musa_language::respelled_type(text) {
            return named_type(now);
        }
        if let Some(resolver) = resolver.as_deref_mut() {
            let vocabulary = musa_language::PRIMITIVE_TYPES
                .iter()
                .map(|(name, _)| format!("`{name}`"))
                .collect::<Vec<_>>()
                .join(", ");
            resolver.report(
                Diagnostic::error(Code::UnknownName, format!("unknown type `{text}`"))
                    .at(crate::resolve::trimmed_span(node), "not a value type")
                    .help(format!(
                        "use {vocabulary}, `Option<τ>`, `List<τ>`, a product, or a function type"
                    )),
            );
        }
        return None;
    }
    if kind == SyntaxKind::ProductType {
        let members: Option<Vec<_>> = node
            .children()
            .filter(|child| is_type_node(child.kind()))
            .map(|child| lower_type(resolver.as_deref_mut(), &child))
            .collect();
        return members.map(Type::Product);
    }
    if kind == SyntaxKind::FunctionType {
        let mut parts = node.children().filter(|child| is_type_node(child.kind()));
        let parameter = parts
            .next()
            .and_then(|part| lower_type(resolver.as_deref_mut(), &part))?;
        let result = parts.next().and_then(|part| lower_type(resolver, &part))?;
        return Some(Type::Function(vec![parameter], Box::new(result)));
    }
    if matches!(kind, SyntaxKind::OptionType | SyntaxKind::ListType) {
        let member = child_of(node, is_type_node).and_then(|child| lower_type(resolver, &child))?;
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
    /// How names read here: empty everywhere but inside a module's members.
    scope: &'a crate::module::NameScope,
    modules: &'a Modules,
}

impl Checker<'_> {
    fn check(&mut self, node: &SyntaxNode, expected: Option<&Type>) -> Option<Expr> {
        let span = crate::resolve::trimmed_span(node);
        let kind = node.kind();
        let checked = if kind == SyntaxKind::ParenExpr || kind == SyntaxKind::BlockExpr {
            // `⟦{ e }⟧ = ⟦e⟧`, exactly as for parentheses. A block delimits
            // one expression and holds no sequence, so it adds a shape to the
            // surface and no case to this checker.
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
        } else if kind == SyntaxKind::ChordExpr {
            self.chord_literal(node)
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
        } else if kind == SyntaxKind::KernelQuote {
            self.kernel_quote(node)
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

    /// Check `kernel Timeline[ScoreFact] { … }` — a quotation.
    ///
    /// The quote is read here, once, in four steps that are deliberately
    /// separate: the type constructor and payload name are *this* language's
    /// words and are checked against what this build can mean; the body text
    /// with its holes replaced by fresh names is handed to `musa-kernel`,
    /// which owns the term grammar and is the only thing that reads it; the
    /// payloads that came back are checked for context authority, because a
    /// reusable value may read the context it is used in but never change it;
    /// and the holes are checked as ordinary `Music` expressions.
    ///
    /// Closure and shadowing are asked of the term with the holes bound to
    /// empty material — the shape elaboration will build — so that a free
    /// name in a quote is a complaint about the quote rather than about the
    /// use that first reached it.
    fn kernel_quote(&mut self, node: &SyntaxNode) -> Option<Expr> {
        let span = crate::resolve::trimmed_span(node);
        let quote = musa_language::ast::KernelQuote::cast(node.clone())?;
        let base = u32::from(node.text_range().start());
        let text = quote_text(node);

        if let Some((constructor, at)) = quote.constructor()
            && constructor != "Timeline"
        {
            self.resolver.report(
                Diagnostic::error(
                    Code::UnknownName,
                    format!("`{constructor}` is not a kernel type constructor"),
                )
                .at(SourceSpan::new(at.0, at.1), "expected `Timeline`")
                .note("a quote writes one composition expression, and a composition is a timeline"),
            );
            self.failed = true;
            return None;
        }
        let (payload, payload_at) = quote.payload_type()?;
        if payload != "ScoreFact" {
            self.resolver.report(
                Diagnostic::error(
                    Code::UnsupportedPayload,
                    format!("this build has no meaning for `{payload}` payloads"),
                )
                .at(
                    SourceSpan::new(payload_at.0, payload_at.1),
                    "no payload type by this name",
                )
                .help("write `ScoreFact`, the payload a musa score is made of")
                .note("the kernel is parametric in its payload; this compiler implements one"),
            );
            self.failed = true;
            return None;
        }

        let (body_start, body_end) = quote.body_span()?;
        let holes = quote.holes();
        // Fresh names, chosen so that nothing in the quoted text can be one:
        // capture is prevented by construction rather than diagnosed after
        // the fact.
        let mut stem = "splice".to_owned();
        while text.contains(&stem) {
            stem.push('_');
        }
        let (source, spans) = substitute_holes(&text, base, body_start, body_end, &stem, &holes);
        let term = match musa_kernel::parse_expression::<crate::elaborate::ScoreFact>(&source) {
            Ok(term) => term,
            Err(error) => {
                self.resolver.report(
                    Diagnostic::error(Code::Syntax, "this kernel quote is not well formed").at(
                        quote_error_span(&spans, &error, body_start, body_end),
                        error.to_string(),
                    ),
                );
                self.failed = true;
                return None;
            }
        };

        let mut authority = Vec::new();
        term.for_each_payload(&mut |fact| {
            if let Some(what) = context_authority(&fact.kind) {
                authority.push(what);
            }
            if fact.scope != crate::elaborate::SHARED_SCOPE {
                authority.push("a voice of its own");
            }
            if fact.origin.source_span != crate::elaborate::SHARED_ORIGIN {
                authority.push("an origin of its own");
            }
        });
        if let Some(what) = authority.first() {
            self.resolver.report(
                Diagnostic::error(Code::Misplaced, format!("a quote cannot carry {what}"))
                    .at(span, "this material would settle what its use is entitled to settle")
                    .help("write what the material *is*; the use supplies where it goes and where it came from")
                    .note("reusable music may read the context supplied at each use, but it cannot change it"),
            );
            self.failed = true;
            return None;
        }

        let mut checked = Vec::new();
        for (index, hole) in holes.iter().enumerate() {
            let name = format!("{stem}{index}");
            let (start, end) = hole.span();
            let Some(locus) = term.locus(&name) else {
                self.resolver.report(
                    Diagnostic::error(Code::Misplaced, "nothing is spliced here")
                        .at(
                            SourceSpan::new(start, end),
                            "this hole is not in a position that names material",
                        )
                        .help("write `${…}` where the term expects a composition")
                        .note("a hole stands for material, so it stands where material does"),
                );
                self.failed = true;
                return None;
            };
            let expression = hole.expr()?;
            let value = self.check(&expression, Some(&Type::Music))?;
            checked.push(CheckedHole {
                name,
                locus: locus.as_ratio(),
                value,
            });
        }

        // Closure, with the holes standing where elaboration will put them.
        let mut closed = term.clone();
        for hole in checked.iter().rev() {
            closed = musa_kernel::Term::bind(
                hole.name.clone(),
                musa_kernel::Term::literal(musa_kernel::zero()),
                closed,
            );
        }
        if let Err(error) = closed.check() {
            self.resolver.report(
                Diagnostic::error(Code::UnknownName, "this kernel quote does not stand on its own")
                    .at(span, error.to_string())
                    .note(
                        "a quote is closed: every name it uses is one it binds, and `${…}` is how the outside gets in",
                    ),
            );
            self.failed = true;
            return None;
        }

        Some(Expr {
            kind: ExprKind::KernelQuote(CheckedQuote {
                term,
                holes: checked,
                definition_span: self.definition_span,
            }),
            ty: Type::Music,
            span,
        })
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

        let mut claims = Vec::new();
        for statement in owned_descendants(node, SyntaxKind::AssertStmt) {
            let checked = self.claim(&statement)?;
            claims.push((crate::resolve::trimmed_span(&statement), checked));
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
                claims,
                bindings: bindings.into_iter().collect(),
                role: self.music_role.clone(),
                definition_span: self.definition_span,
            }),
            ty: Type::Music,
            span,
        })
    }

    /// Check one `assert` statement: its name against the registry, and its
    /// arguments against the shapes that name declares.
    ///
    /// Returns `None` on any complaint, having reported it, so a claim never
    /// reaches elaboration half-understood. A claim nobody could check is
    /// worse than no claim at all — the composer would read the `assert` and
    /// believe it.
    fn claim(&mut self, statement: &SyntaxNode) -> Option<CheckedClaim> {
        let assertion = musa_language::ast::AssertStmt::cast(statement.clone())?;
        let statement_span = crate::resolve::trimmed_span(statement);
        let name = assertion.claim().unwrap_or_default();
        let name_span = assertion
            .claim_span()
            .map_or(statement_span, |(start, end)| SourceSpan::new(start, end));
        let Some(predicate) = crate::assert::predicate(&name) else {
            let known: Vec<&str> = crate::assert::names().collect();
            self.resolver.report(
                Diagnostic::error(Code::UnknownName, format!("nothing is claimed by `{name}`"))
                    .at(name_span, "not a claim musa can prove")
                    .maybe_help(
                        crate::diagnose::nearest(&name, known.iter().copied())
                            .map(|near| format!("did you mean `{near}`?")),
                    )
                    .note(format!("the claims are: {}", known.join(", "))),
            );
            self.failed = true;
            return None;
        };
        let arguments = assertion.args();
        if arguments.len() != predicate.parameters.len() {
            let wanted: Vec<&str> = predicate
                .parameters
                .iter()
                .map(|parameter| parameter.as_str())
                .collect();
            self.resolver.report(
                Diagnostic::error(
                    Code::WrongArity,
                    format!(
                        "`{name}` takes {}, and {} written",
                        spell_arguments(predicate.parameters.len()),
                        spell_written(arguments.len())
                    ),
                )
                .at(name_span, "this claim's arguments do not match it")
                .help(if wanted.is_empty() {
                    format!("`{name}()` — it reads the passage and needs nothing else")
                } else {
                    format!("`{name}({})`", wanted.join(", "))
                })
                .note(predicate.checks),
            );
            self.failed = true;
            return None;
        }
        let mut checked = Vec::new();
        for (argument, parameter) in arguments.iter().zip(predicate.parameters) {
            let node = argument.syntax();
            let span = crate::resolve::trimmed_span(node);
            let expected = match parameter {
                crate::assert::ParamType::Policy => {
                    let word: String = significant_tokens(node).map(|token| token.text().to_owned()).collect();
                    let Some(policy) = crate::assert::Realization::named(&word) else {
                        let spellings: Vec<&str> = crate::assert::Realization::ALL
                            .iter()
                            .map(|policy| policy.as_str())
                            .collect();
                        self.resolver.report(
                            Diagnostic::error(Code::UnknownWord, format!("`{word}` is not a realization policy"))
                                .at(span, "expected one of three words")
                                .maybe_help(
                                    crate::diagnose::nearest(&word, spellings.iter().copied())
                                        .map(|near| format!("did you mean `{near}`?")),
                                )
                                .note(
                                    "`exactly` is set equality, `may_omit` lets a member be missing, \
                                     and `may_add` lets other notes sound",
                                ),
                        );
                        self.failed = true;
                        return None;
                    };
                    checked.push(CheckedArgument::Policy(policy));
                    continue;
                }
                crate::assert::ParamType::Rule => {
                    let word: String = significant_tokens(node).map(|token| token.text().to_owned()).collect();
                    let Some(rule) = crate::analysis::assertable().find(|rule| rule.id() == word) else {
                        let assertable: Vec<&str> = crate::analysis::assertable().map(|rule| rule.id()).collect();
                        self.resolver.report(
                            Diagnostic::error(
                                Code::UnknownWord,
                                format!("`{word}` is not a rule this claim can check"),
                            )
                            .at(span, "expected the id of a voice-leading rule")
                            .maybe_help(
                                crate::diagnose::nearest(&word, assertable.iter().copied())
                                    .map(|near| format!("did you mean `{near}`?")),
                            )
                            .help(format!("the rules a source may assert are: {}", assertable.join(", ")))
                            .note(
                                "every other rule is reported by `musa analyze --kind voice-leading`, \
                                     which says how strongly a style holds it rather than failing the build",
                            ),
                        );
                        self.failed = true;
                        return None;
                    };
                    checked.push(CheckedArgument::Rule(rule));
                    continue;
                }
                crate::assert::ParamType::Scale => Type::Scale,
                crate::assert::ParamType::Chord => Type::ChordClass,
                crate::assert::ParamType::Count => Type::Nat,
                crate::assert::ParamType::Ranges => Type::List(Box::new(Type::Product(vec![Type::Pitch, Type::Pitch]))),
            };
            let expression = child_of(node, is_expr_node)?;
            let value = self.check(&expression, Some(&expected))?;
            checked.push(match parameter {
                crate::assert::ParamType::Scale => CheckedArgument::Scale(value),
                crate::assert::ParamType::Chord => CheckedArgument::Chord(value),
                crate::assert::ParamType::Count => CheckedArgument::Count(value),
                crate::assert::ParamType::Ranges => CheckedArgument::Ranges(value),
                crate::assert::ParamType::Policy | crate::assert::ParamType::Rule => continue,
            });
        }
        Some(CheckedClaim {
            predicate,
            arguments: checked,
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

    /// `chord c major7` — a root and the content stacked on it.
    ///
    /// A literal and not a call, for the reason `scale c dorian` is one: the
    /// words that name a chord type are a closed table the compiler owns, and
    /// a call would need each of them to be a value a musician could bind and
    /// misuse.
    fn chord_literal(&mut self, node: &SyntaxNode) -> Option<Expr> {
        let span = crate::resolve::trimmed_span(node);
        let root = self.written_pitch_class(node, span)?;
        let word = significant_tokens(node)
            .filter(|token| token.kind() == SyntaxKind::Identifier)
            .last();
        let word = word.map(|token| token.text().to_owned()).unwrap_or_default();
        let Some(kind) = crate::chord::ChordType::named(&word) else {
            self.resolver.report(
                Diagnostic::error(Code::UnknownName, format!("unknown chord type `{word}`"))
                    .at(span, "not a named chord type")
                    .help(format!("try one of: {}", chord_type_list()))
                    .note("a chord type is the content; the symbol written above the staff is a separate annotation"),
            );
            self.failed = true;
            return None;
        };
        Some(Expr {
            kind: ExprKind::Literal(Value::ChordClass(crate::chord::ChordClass::new(root, kind))),
            ty: Type::ChordClass,
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

    /// `<pitch-or-pitch-class> up|down <interval>`.
    ///
    /// One operator over two domains, because it is one action: a written
    /// interval moves the letter by its generic size and lets the accidental
    /// absorb the rest, and whether an octave is being carried along changes
    /// nothing about that. The operand's own type decides the result's, so
    /// `c4 up M3` is a pitch and `chord_root(triad) up M3` is a pitch class.
    fn pitch_action(&mut self, node: &SyntaxNode) -> Option<Expr> {
        let span = crate::resolve::trimmed_span(node);
        let mut children = node.children().filter(|child| is_expr_node(child.kind()));
        let pitch = self.check(&children.next()?, None)?;
        if !matches!(pitch.ty, Type::Pitch | Type::PitchClass) {
            self.resolver.report(
                Diagnostic::error(Code::TypeMismatch, "only a pitch or a pitch class can be transposed")
                    .at(pitch.span, format!("this is a `{}`", pitch.ty)),
            );
            self.failed = true;
            return None;
        }
        let moved = pitch.ty.clone();
        let interval = self.check(&children.next()?, Some(&Type::Interval))?;
        let down = significant_tokens(node).any(|token| token.kind() == SyntaxKind::DownKw);
        Some(Expr {
            kind: ExprKind::PitchAction {
                pitch: Box::new(pitch),
                interval: Box::new(interval),
                down,
            },
            ty: moved,
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
        let written = qualified_name(node, &token);
        let span = crate::resolve::trimmed_span(node);
        if let Some(ty) = self.locals.get(&written) {
            return Some(Expr {
                kind: ExprKind::Name(written),
                ty: ty.clone(),
                span,
            });
        }
        // How a name reads is settled once, here, before anything is looked
        // up: inside a module it may name a sibling, and inside a functor's
        // body its parameters name what the site passed.
        let reading = self
            .modules
            .read_name(self.scope, &written, &|candidate| self.symbols.contains_key(candidate));
        if let Some((signature, ascription)) = reading.sealed_by {
            self.resolver.report(
                Diagnostic::error(Code::UnknownName, format!("`{written}` is private"))
                    .at(span, "named from outside the structure that defines it")
                    .also(ascription, format!("`{signature}` does not export it"))
                    .help("a structure exports exactly what its signature lists; everything else is its own"),
            );
            self.failed = true;
            return None;
        }
        let name = reading.name;
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
            | Value::ChordClass(_)
            | Value::Triad(_)
            | Value::Roman(_)
            | Value::Voicing(_)
            | Value::Pc12(_)
            | Value::PcSet12(_)
            | Value::Row12(_)
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
        let wanted = match primitive.family()? {
            Family::Delta { arguments, .. } => arguments.len(),
            Family::Eliminator(eliminator) => eliminator.arity(),
            Family::Music => 1,
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
        let family = primitive.family()?;
        if let Family::Delta { arguments, result } = family {
            let mut checked = Vec::with_capacity(arguments.len());
            for (index, shape) in arguments.iter().enumerate() {
                checked.push(self.check(nodes.get(index)?, Some(&shape.ty()))?);
            }
            return Some(Expr {
                kind: ExprKind::Primitive {
                    primitive,
                    arguments: checked,
                },
                ty: result.ty(),
                span,
            });
        }
        let Family::Eliminator(eliminator) = family else {
            return None;
        };
        let (arguments, ty) = match eliminator {
            Eliminator::Range => {
                let count = self.check(nodes.first()?, Some(&Type::Nat))?;
                (vec![count], Type::List(Box::new(Type::Nat)))
            }
            Eliminator::Repeat => {
                let value = self.check(nodes.first()?, None)?;
                let count = self.check(nodes.get(1)?, Some(&Type::Nat))?;
                let result = Type::List(Box::new(value.ty.clone()));
                (vec![value, count], result)
            }
            Eliminator::Map => {
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
            Eliminator::Filter => {
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
            Eliminator::NatFold => {
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
            Eliminator::ListFold => {
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
            Eliminator::OptionFold => {
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

impl Primitive {
    /// Which of §5.8's three families this operation belongs to, and for a δ-primitive its declared
    /// signature.
    ///
    /// This is the single statement of a δ-primitive's type: the checker reads argument and result
    /// types from here rather than restating them, so the two cannot disagree.
    /// Absent only if the registry has lost an entry, which the registry's own laws forbid.
    fn family(self) -> Option<Family> {
        PRIMITIVE_OWNERSHIP
            .iter()
            .find(|entry| entry.operation == self)
            .map(|entry| entry.family)
    }
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
            | Type::ChordClass
            | Type::Triad
            | Type::Roman
            | Type::Voicing
            | Type::Pc12
            | Type::PcSet12
            | Type::Row12
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
        Value::ChordClass(value) => format!("chord_class:{value}"),
        Value::Triad(value) => format!("triad:{value}"),
        Value::Roman(value) => format!("roman:{value}"),
        Value::Voicing(value) => format!("voicing:{value}"),
        Value::Pc12(value) => format!("pc12:{value}"),
        Value::PcSet12(value) => format!("pcset12:{value}"),
        Value::Row12(value) => format!("row12:{value}"),
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
                | Value::ChordClass(_)
                | Value::Triad(_)
                | Value::Roman(_)
                | Value::Voicing(_)
                | Value::Pc12(_)
                | Value::PcSet12(_)
                | Value::Row12(_)
                | Value::Product(_)
                | Value::Option { .. }
                | Value::List { .. }
                | Value::Music(_) => None,
            }
        }
        ExprKind::PitchAction { pitch, interval, down } => {
            let moved = eval(pitch, environment, meter)?;
            let Value::Interval(mut interval) = eval(interval, environment, meter)? else {
                return None;
            };
            if *down {
                interval = interval.inverse()?;
            }
            match moved {
                Value::Pitch(pitch) => pitch.transpose(interval).map(Value::Pitch),
                Value::PitchClass(spelled) => spelled.transpose(interval).map(Value::PitchClass),
                Value::Bool(_)
                | Value::Nat(_)
                | Value::Ratio(_)
                | Value::Duration(_)
                | Value::Interval(_)
                | Value::Scale(_)
                | Value::Key(_)
                | Value::Degree(_)
                | Value::Frame(_)
                | Value::ChordClass(_)
                | Value::Triad(_)
                | Value::Roman(_)
                | Value::Voicing(_)
                | Value::Pc12(_)
                | Value::PcSet12(_)
                | Value::Row12(_)
                | Value::Product(_)
                | Value::Option { .. }
                | Value::List { .. }
                | Value::Music(_)
                | Value::Closure(_)
                | Value::Builtin(_) => None,
            }
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
            let mut claims = IndexMap::new();
            for (span, claim) in &music.claims {
                claims.insert(span_key(*span), eval_claim(claim, environment, meter)?);
            }
            let mut bindings = IndexMap::new();
            for name in &music.bindings {
                let bound = match environment.get(name)? {
                    Value::Pitch(pitch) => crate::resolve::BoundValue::Pitch(*pitch),
                    Value::Duration(duration) => {
                        crate::resolve::BoundValue::Duration(crate::score::NotatedDuration::spelled(*duration))
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
                    | Value::ChordClass(_)
                    | Value::Triad(_)
                    | Value::Roman(_)
                    | Value::Voicing(_)
                    | Value::Pc12(_)
                    | Value::PcSet12(_)
                    | Value::Row12(_)
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
                claims: Box::new(claims),
                keys: Box::default(),
                bindings,
                role: music.role.clone(),
                definition_span: music.definition_span,
                operation: None,
            }))
        }
        ExprKind::KernelQuote(quote) => {
            let mut holes = Vec::with_capacity(quote.holes.len());
            for hole in &quote.holes {
                let Value::Music(value) = eval(&hole.value, environment, meter)? else {
                    return None;
                };
                holes.push((hole.name.clone(), hole.locus, value));
            }
            Some(Value::Music(Music {
                items: Vec::new(),
                uses: IndexMap::new(),
                pitches: Box::default(),
                scales: Box::default(),
                claims: Box::default(),
                keys: Box::default(),
                bindings: IndexMap::new(),
                role: None,
                definition_span: quote.definition_span,
                operation: Some(Box::new(MusicOperation::KernelQuote {
                    term: quote.term.clone(),
                    holes,
                })),
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
        | ExprKind::Music(_)
        | ExprKind::KernelQuote(_) => {
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
        Builtin::Play => {
            let Value::Voicing(voicing) = arguments.next()? else {
                return None;
            };
            let held = duration_value(&arguments.next()?)?;
            if held <= Ratio::ZERO {
                return None;
            }
            MusicOperation::Play { voicing, held }
        }
    };
    Some(Value::Music(Music {
        items: Vec::new(),
        uses: IndexMap::new(),
        pitches: Box::default(),
        scales: Box::default(),
        claims: Box::default(),
        keys: Box::default(),
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
        Primitive::Pc12Of => Some(Value::Pc12(crate::pc12::Pc12::from_number(nat_value(values.first()?)?))),
        Primitive::Pc12Number => {
            let Value::Pc12(member) = values.first()? else {
                return None;
            };
            Some(Value::Nat(u64::from(member.number())))
        }
        Primitive::Pc12Forget => {
            let Value::PitchClass(spelled) = values.first()? else {
                return None;
            };
            Some(Value::Pc12(crate::pc12::Pc12::forgetting(*spelled)))
        }
        Primitive::Pc12Transposed | Primitive::Pc12Inverted => {
            let Value::Pc12(member) = values.first()? else {
                return None;
            };
            let index = nat_value(values.get(1)?)?;
            Some(Value::Pc12(if primitive == Primitive::Pc12Transposed {
                member.transposed(index)
            } else {
                member.inverted(index)
            }))
        }
        Primitive::Pc12Spelled => {
            let (Value::Pc12(member), Value::Scale(collection)) = (values.first()?, values.get(1)?) else {
                return None;
            };
            Some(optional(
                Type::PitchClass,
                member.spelled(*collection).map(Value::PitchClass),
            ))
        }
        Primitive::PcSet12Of => Some(Value::PcSet12(crate::pc12::PcSet12::of(pc12_list(values.first()?)?))),
        Primitive::PcSet12Members | Primitive::PcSet12Normal => {
            let Value::PcSet12(set) = values.first()? else {
                return None;
            };
            let members: Vec<crate::pc12::Pc12> = if primitive == Primitive::PcSet12Members {
                set.members().collect()
            } else {
                set.normal_order()
            };
            Some(pc12_values(members))
        }
        Primitive::PcSet12Prime => {
            let Value::PcSet12(set) = values.first()? else {
                return None;
            };
            Some(Value::PcSet12(set.prime_form()))
        }
        Primitive::PcSet12Vector => {
            let Value::PcSet12(set) = values.first()? else {
                return None;
            };
            Some(Value::List {
                member: Type::Nat,
                values: set
                    .interval_class_vector()
                    .into_iter()
                    .map(|count| Value::Nat(u64::from(count)))
                    .collect(),
            })
        }
        Primitive::Row12Of => {
            let pcs = pc12_list(values.first()?)?;
            Some(optional(
                Type::Row12,
                crate::pc12::Row12::checked(&pcs).map(Value::Row12),
            ))
        }
        Primitive::Row12Pcs => {
            let Value::Row12(row) = values.first()? else {
                return None;
            };
            Some(pc12_values(row.pcs().collect()))
        }
        Primitive::Row12Head => {
            let Value::Row12(row) = values.first()? else {
                return None;
            };
            Some(Value::Pc12(row.head()))
        }
        Primitive::Row12Transposed | Primitive::Row12Inverted => {
            let Value::Row12(row) = values.first()? else {
                return None;
            };
            let index = nat_value(values.get(1)?)?;
            Some(Value::Row12(if primitive == Primitive::Row12Transposed {
                row.transposed(index)
            } else {
                row.inverted(index)
            }))
        }
        Primitive::Row12Retrograde => {
            let Value::Row12(row) = values.first()? else {
                return None;
            };
            Some(Value::Row12(row.retrograde()))
        }
        Primitive::Row12Matrix => {
            let Value::Row12(row) = values.first()? else {
                return None;
            };
            Some(Value::List {
                member: Type::Row12,
                values: row.matrix().into_iter().map(Value::Row12).collect(),
            })
        }
        Primitive::Row12Forms | Primitive::Row12Symmetries => {
            let Value::Row12(row) = values.first()? else {
                return None;
            };
            let count = if primitive == Primitive::Row12Forms {
                row.forms()
            } else {
                row.symmetries()
            };
            Some(Value::Nat(u64::from(count)))
        }
        Primitive::Row12Repeats => {
            let pcs = pc12_list(values.first()?)?;
            Some(Value::List {
                member: Type::Nat,
                values: crate::pc12::repeated_positions(&pcs)
                    .into_iter()
                    .map(Value::Nat)
                    .collect(),
            })
        }
        Primitive::Row12Missing => Some(pc12_values(crate::pc12::missing_classes(&pc12_list(values.first()?)?))),
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
        Primitive::ScaleClass => {
            let (Value::Scale(scale), Value::Degree(degree)) = (values.first()?, values.get(1)?) else {
                return None;
            };
            Some(optional(Type::PitchClass, scale.class(*degree).map(Value::PitchClass)))
        }
        Primitive::ScaleChord => {
            let (Value::Scale(scale), Value::Degree(degree)) = (values.first()?, values.get(1)?) else {
                return None;
            };
            let members = usize::try_from(nat_value(values.get(2)?)?).ok()?;
            Some(optional(
                Type::ChordClass,
                scale.stacked(*degree, members).map(Value::ChordClass),
            ))
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
            // Degrees are written from one, as musicians write them, and
            // `Degree` counts from one as well: no adjustment belongs here.
            let ordinal = i64::try_from(nat_value(values.first()?)?).ok()?;
            Some(Value::Degree(crate::scale::Degree::new(ordinal)))
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
        Primitive::ChordOn => {
            let (Value::ChordClass(class), Value::PitchClass(root)) = (values.first()?, values.get(1)?) else {
                return None;
            };
            Some(Value::ChordClass(class.rooted_at(*root)))
        }
        Primitive::ChordRoot => {
            let Value::ChordClass(class) = values.first()? else {
                return None;
            };
            Some(Value::PitchClass(class.root()))
        }
        Primitive::ChordBass => {
            let Value::ChordClass(class) = values.first()? else {
                return None;
            };
            Some(optional(Type::PitchClass, class.bass().map(Value::PitchClass)))
        }
        Primitive::ChordMembers => {
            let Value::ChordClass(class) = values.first()? else {
                return None;
            };
            Some(Value::List {
                member: Type::Interval,
                values: class.members().iter().copied().map(Value::Interval).collect(),
            })
        }
        Primitive::ChordInversion => {
            let Value::ChordClass(class) = values.first()? else {
                return None;
            };
            let position = usize::try_from(nat_value(values.get(1)?)?).ok()?;
            Some(optional(
                Type::ChordClass,
                class.inverted(position).ok().map(Value::ChordClass),
            ))
        }
        Primitive::ChordOver => {
            let (Value::ChordClass(class), Value::PitchClass(bass)) = (values.first()?, values.get(1)?) else {
                return None;
            };
            Some(Value::ChordClass(class.over(*bass)))
        }
        Primitive::ChordTriad => {
            let Value::ChordClass(class) = values.first()? else {
                return None;
            };
            Some(optional(Type::Triad, crate::chord::Triad::of(*class).map(Value::Triad)))
        }
        Primitive::TriadChord => {
            let Value::Triad(triad) = values.first()? else {
                return None;
            };
            Some(Value::ChordClass(triad.class()))
        }
        Primitive::RomanOf => {
            let (Value::Nat(ordinal), Value::Nat(members), Value::Nat(inversion)) =
                (values.first()?, values.get(1)?, values.get(2)?)
            else {
                return None;
            };
            Some(optional(
                Type::Roman,
                crate::roman::Roman::new(*ordinal, *members, *inversion).map(Value::Roman),
            ))
        }
        Primitive::RomanOrdinal => {
            let Value::Roman(numeral) = values.first()? else {
                return None;
            };
            Some(Value::Nat(numeral.ordinal()))
        }
        Primitive::RomanSize => {
            let Value::Roman(numeral) = values.first()? else {
                return None;
            };
            Some(Value::Nat(numeral.members()))
        }
        Primitive::RomanInversion => {
            let Value::Roman(numeral) = values.first()? else {
                return None;
            };
            Some(Value::Nat(numeral.inversion()))
        }
        Primitive::TriadMajor => {
            let Value::Triad(triad) = values.first()? else {
                return None;
            };
            Some(Value::Bool(triad.is_major()))
        }
        Primitive::VoicingOf => {
            let (Value::ChordClass(class), Value::List { values: pitches, .. }) = (values.first()?, values.get(1)?)
            else {
                return None;
            };
            let written: Option<Vec<WrittenPitch>> = pitches
                .iter()
                .map(|value| match *value {
                    Value::Pitch(pitch) => Some(pitch),
                    Value::Bool(_)
                    | Value::Nat(_)
                    | Value::Ratio(_)
                    | Value::Duration(_)
                    | Value::PitchClass(_)
                    | Value::Interval(_)
                    | Value::Scale(_)
                    | Value::Key(_)
                    | Value::Degree(_)
                    | Value::Frame(_)
                    | Value::ChordClass(_)
                    | Value::Triad(_)
                    | Value::Roman(_)
                    | Value::Voicing(_)
                    | Value::Pc12(_)
                    | Value::PcSet12(_)
                    | Value::Row12(_)
                    | Value::Product(_)
                    | Value::Option { .. }
                    | Value::List { .. }
                    | Value::Music(_)
                    | Value::Closure(_)
                    | Value::Builtin(_) => None,
                })
                .collect();
            Some(optional(
                Type::Voicing,
                crate::chord::Voicing::new(*class, written?).ok().map(Value::Voicing),
            ))
        }
        Primitive::VoicingPitches => {
            let Value::Voicing(voicing) = values.first()? else {
                return None;
            };
            Some(Value::List {
                member: Type::Pitch,
                values: voicing.pitches().map(Value::Pitch).collect(),
            })
        }
        Primitive::VoicingBass => {
            let Value::Voicing(voicing) = values.first()? else {
                return None;
            };
            Some(Value::Pitch(voicing.bass()))
        }
        Primitive::VoicingChord => {
            let Value::Voicing(voicing) = values.first()? else {
                return None;
            };
            Some(Value::ChordClass(voicing.class()))
        }
        Primitive::VoicingPosition => {
            let Value::Voicing(voicing) = values.first()? else {
                return None;
            };
            let position = voicing
                .inversion()
                .and_then(|position| u64::try_from(position).ok())
                .map(Value::Nat);
            Some(optional(Type::Nat, position))
        }
        Primitive::CloseVoicing => {
            let (Value::ChordClass(class), Value::Pitch(bass)) = (values.first()?, values.get(1)?) else {
                return None;
            };
            Some(optional(
                Type::Voicing,
                crate::chord::Voicing::close_position(*class, *bass)
                    .ok()
                    .map(Value::Voicing),
            ))
        }
        Primitive::DropVoicing => {
            let (Value::ChordClass(class), Value::Pitch(bass)) = (values.first()?, values.get(1)?) else {
                return None;
            };
            let voice = usize::try_from(nat_value(values.get(2)?)?).ok()?;
            Some(optional(
                Type::Voicing,
                crate::chord::Voicing::dropped(*class, *bass, voice)
                    .ok()
                    .map(Value::Voicing),
            ))
        }
        Primitive::OmitVoicing => {
            let Value::Voicing(voicing) = values.first()? else {
                return None;
            };
            let position = usize::try_from(nat_value(values.get(1)?)?).ok()?;
            Some(optional(
                Type::Voicing,
                voicing.omitting(position).ok().map(Value::Voicing),
            ))
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

/// The unspelled pitch classes a checked `list[pc12]` holds.
///
/// The list's member type was already checked, so a value of any other shape
/// means the evaluator and the checker disagree — which is a bug, not a
/// musical failure, and so answers nothing rather than guessing.
fn pc12_list(value: &Value) -> Option<Vec<crate::pc12::Pc12>> {
    let Value::List { values, .. } = value else {
        return None;
    };
    values
        .iter()
        .map(|member| {
            if let Value::Pc12(member) = member {
                Some(*member)
            } else {
                None
            }
        })
        .collect()
}

/// Unspelled pitch classes as a core list value.
fn pc12_values(members: Vec<crate::pc12::Pc12>) -> Value {
    Value::List {
        member: Type::Pc12,
        values: members.into_iter().map(Value::Pc12).collect(),
    }
}

fn nat_value(value: &Value) -> Option<u64> {
    if let Value::Nat(value) = value {
        Some(*value)
    } else {
        None
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
        Value::ChordClass(_) | Value::Triad(_) => (1, 24),
        Value::Roman(_) => (1, 12),
        Value::Pc12(_) => (1, 1),
        Value::PcSet12(_) => (1, 2),
        Value::Row12(_) => (1, 12),
        Value::Voicing(value) => (1, u64::try_from(value.size()).unwrap_or(u64::MAX).saturating_mul(12)),
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
        Some(MusicOperation::Play { voicing, .. }) => value_shape(&Value::Voicing(voicing.clone())),
        Some(MusicOperation::KernelQuote { term, holes }) => {
            let payloads = term.occurrence_bound();
            holes
                .iter()
                .fold((payloads, payloads.saturating_mul(64)), |shape, (_, _, music)| {
                    let hole = music_shape(music);
                    (shape.0.saturating_add(hole.0), shape.1.saturating_add(hole.1))
                })
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

/// The quote's own text with its comments blanked out, byte for byte.
///
/// A quote is written in a `.musa` file, so it is commented the way the rest
/// of the file is — `//` and `/* */`, which the lexer already reads as trivia
/// here. The kernel's alphabet has no `//` and Musa's has no `%`, so there is
/// exactly one comment syntax inside a quote and it is the host's.
///
/// Blanked rather than removed: every offset in what comes back is still the
/// offset it has in the document, which is what lets a complaint from the
/// kernel's reader point at the character it stopped on. Newlines survive so
/// the line a complaint lands on is the line it was written on.
fn quote_text(node: &SyntaxNode) -> String {
    let mut text = node.text().to_string();
    let base = usize::from(node.text_range().start());
    for token in node.descendants_with_tokens().filter_map(SyntaxElement::into_token) {
        if !matches!(token.kind(), SyntaxKind::LineComment | SyntaxKind::BlockComment) {
            continue;
        }
        let start = usize::from(token.text_range().start()).saturating_sub(base);
        let end = usize::from(token.text_range().end()).saturating_sub(base);
        // Byte-wise, and only ASCII bytes are written: a newline is never
        // part of a multi-byte sequence, so the string stays valid UTF-8 and
        // stays exactly as long as it was.
        // SAFETY-BY-CONSTRUCTION: `blanked` is the same length as the range
        // it replaces, so no later token's offsets move.
        let Some(comment) = text.get(start..end) else {
            continue;
        };
        let blanked: String = comment
            .bytes()
            .map(|byte| if byte == b'\n' { '\n' } else { ' ' })
            .collect();
        text.replace_range(start..end, &blanked);
    }
    text
}

/// The quoted body with each hole replaced by its fresh name, and the map
/// back.
///
/// The map is a list of `(offset in the substituted text, offset in the
/// document)` at each seam, which is what turns a kernel parse error into a
/// place in the composer's file. Without it every complaint about a quote
/// would point at the whole quote.
fn substitute_holes(
    text: &str,
    base: u32,
    body_start: u32,
    body_end: u32,
    stem: &str,
    holes: &[musa_language::ast::KernelHole],
) -> (String, Vec<(usize, u32)>) {
    let relative = |absolute: u32| usize::try_from(absolute.saturating_sub(base)).unwrap_or_default();
    let mut source = String::with_capacity(text.len());
    let mut spans = Vec::new();
    let mut at = relative(body_start);
    for (index, hole) in holes.iter().enumerate() {
        let (start, end) = hole.span();
        let (start, end) = (relative(start), relative(end));
        let Some(before) = text.get(at..start) else {
            continue;
        };
        spans.push((
            source.len(),
            body_start.saturating_add(u32::try_from(at).unwrap_or_default()),
        ));
        source.push_str(before);
        source.push(' ');
        source.push_str(stem);
        source.push_str(&index.to_string());
        source.push(' ');
        at = end;
    }
    if let Some(rest) = text.get(at..relative(body_end)) {
        spans.push((source.len(), base.saturating_add(u32::try_from(at).unwrap_or_default())));
        source.push_str(rest);
    }
    (source, spans)
}

/// Where a kernel parse error lands in the document.
fn quote_error_span(
    spans: &[(usize, u32)],
    error: &musa_kernel::KernelError,
    body_start: u32,
    body_end: u32,
) -> SourceSpan {
    let musa_kernel::KernelError::Parse { offset, .. } = error else {
        return SourceSpan::new(body_start, body_end);
    };
    let Some((seam, document)) = spans.iter().rev().find(|(seam, _)| seam <= offset) else {
        return SourceSpan::new(body_start, body_end);
    };
    let at = document.saturating_add(u32::try_from(offset.saturating_sub(*seam)).unwrap_or_default());
    SourceSpan::new(at, at.saturating_add(1).min(body_end))
}

/// What a fact would take authority over, if it is one of the four that can.
///
/// Key, meter, tempo and clef are *context*: they hold from where they are
/// written until they are written again, so a value carrying one would change
/// its caller's context from inside — the very thing a reusable `music` value
/// must not do (`docs/language/00-semantics.md`, contextual closure).
fn context_authority(kind: &crate::elaborate::FactKind) -> Option<&'static str> {
    match kind {
        crate::elaborate::FactKind::Key { .. } => Some("a key"),
        crate::elaborate::FactKind::Meter { .. } => Some("a meter"),
        crate::elaborate::FactKind::Tempo { .. } => Some("a tempo"),
        crate::elaborate::FactKind::Clef { .. } => Some("a clef"),
        crate::elaborate::FactKind::Note { .. }
        | crate::elaborate::FactKind::Rest { .. }
        | crate::elaborate::FactKind::Mark { .. }
        | crate::elaborate::FactKind::Grace { .. }
        | crate::elaborate::FactKind::Slur
        | crate::elaborate::FactKind::Phrase { .. }
        | crate::elaborate::FactKind::Tuplet { .. }
        | crate::elaborate::FactKind::Dynamic { .. }
        | crate::elaborate::FactKind::Hairpin { .. }
        | crate::elaborate::FactKind::Section { .. }
        | crate::elaborate::FactKind::Harmony { .. }
        | crate::elaborate::FactKind::Repeat { .. }
        | crate::elaborate::FactKind::Mobile { .. }
        | crate::elaborate::FactKind::Improvise { .. }
        | crate::elaborate::FactKind::Ending { .. } => None,
    }
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

/// Evaluate a checked claim's arguments and hand them to the assertion
/// registry.
///
/// This is the whole bridge between the value language and the claim family:
/// `Value` does not leave this module, and `crate::assert::Argument` is the
/// five shapes that do. Adding a claim therefore cannot widen what a checker
/// can see — it can only ask for one of these again.
fn eval_claim(
    claim: &CheckedClaim,
    environment: &IndexMap<String, Value>,
    meter: &mut crate::core_budget::WorkMeter,
) -> Option<crate::assert::Claim> {
    let mut arguments = Vec::new();
    for argument in &claim.arguments {
        arguments.push(match argument {
            CheckedArgument::Policy(policy) => crate::assert::Argument::Policy(*policy),
            CheckedArgument::Rule(rule) => crate::assert::Argument::Rule(*rule),
            CheckedArgument::Scale(expression) => {
                let Value::Scale(scale) = eval(expression, environment, meter)? else {
                    return None;
                };
                crate::assert::Argument::Scale(scale)
            }
            CheckedArgument::Chord(expression) => {
                let Value::ChordClass(chord) = eval(expression, environment, meter)? else {
                    return None;
                };
                crate::assert::Argument::Chord(chord)
            }
            CheckedArgument::Count(expression) => {
                let Value::Nat(count) = eval(expression, environment, meter)? else {
                    return None;
                };
                crate::assert::Argument::Count(count)
            }
            CheckedArgument::Ranges(expression) => {
                let Value::List { values, .. } = eval(expression, environment, meter)? else {
                    return None;
                };
                crate::assert::Argument::Ranges(
                    values
                        .iter()
                        .map(|pair| {
                            let Value::Product(bounds) = pair else {
                                return None;
                            };
                            match (bounds.first(), bounds.get(1)) {
                                (Some(Value::Pitch(low)), Some(Value::Pitch(high))) => Some((*low, *high)),
                                _ => None,
                            }
                        })
                        .collect::<Option<Vec<_>>>()?,
                )
            }
        });
    }
    crate::assert::Claim::build(claim.predicate.name, arguments)
}

/// `two arguments`, `no arguments` — what a claim's signature asks for.
fn spell_arguments(count: usize) -> String {
    match count {
        0 => "no arguments".to_owned(),
        1 => "one argument".to_owned(),
        other => format!("{other} arguments"),
    }
}

/// `none were`, `one was`, `three were` — what the source actually wrote.
fn spell_written(count: usize) -> String {
    match count {
        0 => "none were".to_owned(),
        1 => "one was".to_owned(),
        other => format!("{other} were"),
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

/// Every chord-type spelling, sorted, for the unknown-type diagnostic.
fn chord_type_list() -> String {
    let mut spellings: Vec<_> = crate::chord::ChordType::spellings().collect();
    spellings.sort_unstable();
    format!("`{}`", spellings.join("`, `"))
}

/// The type a declaration or parameter annotates, as a node.
pub(crate) fn type_node_of(node: &SyntaxNode) -> Option<SyntaxNode> {
    child_of(node, is_type_node)
}

/// The expression a wrapper node holds, as a node.
pub(crate) fn expr_node_of(node: &SyntaxNode) -> Option<SyntaxNode> {
    child_of(node, is_expr_node)
}

fn is_expr_node(kind: SyntaxKind) -> bool {
    matches!(
        kind,
        SyntaxKind::NameExpr
            | SyntaxKind::LiteralExpr
            | SyntaxKind::ParenExpr
            | SyntaxKind::BlockExpr
            | SyntaxKind::ProductExpr
            | SyntaxKind::ListExpr
            | SyntaxKind::OptionExpr
            | SyntaxKind::ApplyExpr
            | SyntaxKind::PitchExpr
            | SyntaxKind::ChordExpr
            | SyntaxKind::ScaleExpr
            | SyntaxKind::KeyExpr
            | SyntaxKind::StepExpr
            | SyntaxKind::MatchExpr
            | SyntaxKind::MusicExpr
            | SyntaxKind::KernelQuote
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
// A law suite reports a violated law by failing, which is what `panic!` and `expect` are for here }
// the crate's integration tests carry the same allowances for the same reason.
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    /// The vocabulary the language offers, the types this module reads, and
    /// the spellings it writes back are one vocabulary or they are three.
    #[test]
    fn every_offered_type_name_is_read_and_written_the_same_way() {
        for (name, _) in musa_language::PRIMITIVE_TYPES {
            let read = named_type(name);
            assert!(read.is_some(), "`{name}` is offered to the composer but is not a type");
            assert_eq!(
                read.map(|ty| ty.to_string()).as_deref(),
                Some(*name),
                "`{name}` is read as a type that prints under another name"
            );
        }
    }

    /// Every spelling the language removed still reaches the type it named,
    /// so a file written against the old vocabulary gets the parser's one
    /// complaint and not a second one from here.
    #[test]
    fn every_removed_type_name_still_reaches_its_type() {
        for (was, now) in musa_language::RESPELLED_TYPES {
            if matches!(*now, "Option" | "List") {
                continue; // Parameterized: a node kind, not a name.
            }
            assert!(
                named_type(now).is_some(),
                "`{was}` was respelled to a name that is not a type"
            );
        }
    }

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
            &Modules::default(),
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
            77,
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

    // --- `docs/language/02-core-calculus.md` §5.8: the conservative-extension laws ---
    //
    // Theorem 5 holds for any base type with no eliminator and any δ-primitives satisfying D1–D4.
    // These laws check its premises against the implementation, so that a later musical domain
    // costs a registry entry rather than a new induction and cannot be added without discharging
    // them. They say nothing about whether the musical content is right; that is what the domain
    // law suites in `crates/musa-compiler/tests/` are for.

    /// How far the natural samples run. Thirteen covers every ordinal, member count, inversion,
    /// transposition index, and voice position the current domains accept, plus the first value
    /// past each — which is the half that matters, since D2 is about total answers on the whole
    /// declared domain rather than correct answers on the intended one.
    const SAMPLED_NATS: u64 = 13;

    /// How many times the sample pool is closed under the δ-primitives. Three rounds is what it
    /// takes to reach every constructed domain from the seeds: degrees and pitch classes appear in
    /// the first, frames, chords, rows and sets in the second, triads and voicings in the third.
    const SAMPLE_ROUNDS: usize = 3;

    /// How many applications each δ-primitive is sampled at. The pool is deliberately not
    /// exhausted combinatorially: a three-argument primitive over a pool of forty would be sixty
    /// thousand evaluations for no additional coverage of the property being checked. The budget
    /// is spent per *primitive* rather than per argument position so that a unary primitive sees
    /// its whole domain — which is how all twelve pitch classes reach the pool.
    const SAMPLED_APPLICATIONS: usize = 64;

    /// How long a synthesized list sample is. Twelve, because the row domain's whole point is that
    /// a row is an ordering of all twelve pitch classes, and a sampler that never offered twelve
    /// distinct classes would only ever see `row12_of` decline.
    const SAMPLED_LIST_LENGTH: usize = 12;

    /// How many values one argument position draws, given the primitive's arity, so that the
    /// product stays within [`SAMPLED_APPLICATIONS`].
    const fn samples_per_argument(arity: usize) -> usize {
        match arity {
            0 | 1 => SAMPLED_APPLICATIONS,
            2 => 8,
            _ => 4,
        }
    }

    /// The type of an evaluated value, for the shapes a δ-primitive can return.
    ///
    /// Absent for `music`, closures, and builtins, none of which a δ signature can name — which is
    /// itself part of what the totality law checks.
    fn value_type(value: &Value) -> Option<Type> {
        Some(match value {
            Value::Bool(_) => Type::Bool,
            Value::Nat(_) => Type::Nat,
            Value::Ratio(_) => Type::Ratio,
            Value::Duration(_) => Type::Duration,
            Value::Pitch(_) => Type::Pitch,
            Value::PitchClass(_) => Type::PitchClass,
            Value::Interval(_) => Type::Interval,
            Value::Scale(_) => Type::Scale,
            Value::Key(_) => Type::Key,
            Value::Degree(_) => Type::Degree,
            Value::Frame(_) => Type::Frame,
            Value::ChordClass(_) => Type::ChordClass,
            Value::Triad(_) => Type::Triad,
            Value::Roman(_) => Type::Roman,
            Value::Voicing(_) => Type::Voicing,
            Value::Pc12(_) => Type::Pc12,
            Value::PcSet12(_) => Type::PcSet12,
            Value::Row12(_) => Type::Row12,
            Value::Option { member, .. } => Type::Option(Box::new(member.clone())),
            Value::List { member, .. } => Type::List(Box::new(member.clone())),
            Value::Product(members) => Type::Product(members.iter().map(value_type).collect::<Option<Vec<_>>>()?),
            Value::Music(_) | Value::Closure(_) | Value::Builtin(_) => return None,
        })
    }

    /// The values every base type the seeds cannot reach is later constructed from.
    ///
    /// Only the domains with surface literals are seeded here. Everything else — degrees, frames,
    /// triads, numerals, voicings, and the twelve-tone domains — is reached by *applying the
    /// primitives*, which is why the closure below doubles as the totality check rather than
    /// needing a separate constructor for each domain.
    fn sample_seeds() -> Vec<Value> {
        let source = "piece \"law\" { \
             let a_pitch: Pitch = c4; \
             let b_pitch: Pitch = eb3; \
             let c_pitch: Pitch = f5; \
             let an_interval: Interval = P5; \
             let b_interval: Interval = m3; \
             let a_scale: Scale = scale c major; \
             let b_scale: Scale = scale eb harmonic_minor; \
             let a_key: Key = key c major; \
             let b_key: Key = key f# minor; \
             let a_chord: ChordClass = chord c major; \
             let b_chord: ChordClass = chord ab dominant7; \
         }";
        let mut pool: Vec<Value> = (0..SAMPLED_NATS).map(Value::Nat).collect();
        pool.push(Value::Bool(true));
        pool.push(Value::Bool(false));
        pool.extend(values(source).expect("the seed piece must compile").into_values());
        pool
    }

    /// Group a pool of values by the type they inhabit, and offer a list of each base type.
    ///
    /// The lists are synthesized rather than discovered because several list arguments are not
    /// reachable from any result: `pcset12_of` wants a `list[pc12]`, and the only primitive that
    /// returns one wants a `pcset12`. Seeding the list closes that circle without seeding any
    /// value the language itself could not write.
    fn by_type(pool: &[Value]) -> IndexMap<String, Vec<Value>> {
        let mut grouped: IndexMap<String, Vec<Value>> = IndexMap::new();
        for value in pool {
            let Some(ty) = value_type(value) else { continue };
            grouped.entry(ty.to_string()).or_default().push(value.clone());
        }
        let mut lists: Vec<(String, Value)> = Vec::new();
        for members in grouped.values() {
            let Some(member) = members.first().and_then(value_type) else {
                continue;
            };
            if matches!(member, Type::Option(_) | Type::List(_) | Type::Product(_)) {
                continue;
            }
            let ty = Type::List(Box::new(member.clone())).to_string();
            // An empty list and a full one: the first is the edge case every list primitive has to
            // answer for, the second is what a domain of twelve needs before it can say yes.
            lists.push((
                ty.clone(),
                Value::List {
                    member: member.clone(),
                    values: Vec::new(),
                },
            ));
            lists.push((
                ty,
                Value::List {
                    member,
                    values: members.iter().take(SAMPLED_LIST_LENGTH).cloned().collect(),
                },
            ));
        }
        for (spelling, list) in lists {
            // At the front, so that a synthesized list is inside the sample even where discovered
            // lists of the same type already fill the position's budget.
            grouped.entry(spelling).or_default().insert(0, list);
        }
        grouped
    }

    /// Evaluate one δ-primitive on already-evaluated arguments.
    fn apply(primitive: Primitive, arguments: &[(Type, Value)]) -> Option<Value> {
        let span = SourceSpan::new(0, 0);
        let exprs: Vec<Expr> = arguments
            .iter()
            .map(|(ty, value)| Expr {
                kind: ExprKind::Literal(value.clone()),
                ty: ty.clone(),
                span,
            })
            .collect();
        let site = Expr {
            kind: ExprKind::Literal(Value::Nat(0)),
            ty: Type::Nat,
            span,
        };
        eval_primitive(primitive, &exprs, &IndexMap::new(), &mut WorkMeter::default(), &site)
    }

    /// Every δ-primitive applied to every sampled argument tuple, with what it answered.
    fn sampled_applications() -> Vec<(Primitive, Shape, Option<Value>)> {
        let mut pool = sample_seeds();
        let mut observed = Vec::new();
        for _ in 0..SAMPLE_ROUNDS {
            let grouped = by_type(&pool);
            let mut discovered = Vec::new();
            for entry in &PRIMITIVE_OWNERSHIP {
                let Family::Delta { arguments, result } = entry.family else {
                    continue;
                };
                let budget = samples_per_argument(arguments.len());
                let choices: Vec<Vec<Value>> = arguments
                    .iter()
                    .map(|shape| {
                        grouped
                            .get(&shape.ty().to_string())
                            .map(|values| values.iter().take(budget).cloned().collect())
                            .unwrap_or_default()
                    })
                    .collect();
                if choices.iter().any(Vec::is_empty) {
                    continue;
                }
                for tuple in tuples(&choices) {
                    let typed: Vec<(Type, Value)> = arguments.iter().map(|shape| shape.ty()).zip(tuple).collect();
                    let answer = apply(entry.operation, &typed);
                    if let Some(value) = &answer {
                        discovered.push(value.clone());
                        // An option's or list's members are themselves samples, which is how the
                        // pool reaches domains no seed can spell.
                        if let Value::Option { value: Some(inner), .. } = value {
                            discovered.push(inner.as_ref().clone());
                        } else if let Value::List { values, .. } = value {
                            discovered.extend(values.iter().cloned());
                        }
                    }
                    observed.push((entry.operation, result, answer));
                }
            }
            pool.extend(discovered);
        }
        observed
    }

    /// The cartesian product of the per-position choices, in a stable order.
    fn tuples(choices: &[Vec<Value>]) -> Vec<Vec<Value>> {
        choices.iter().fold(vec![Vec::new()], |built, options| {
            built
                .iter()
                .flat_map(|prefix| {
                    options.iter().map(move |option| {
                        let mut next = prefix.clone();
                        next.push(option.clone());
                        next
                    })
                })
                .collect()
        })
    }

    #[test]
    fn every_compiler_owned_operation_belongs_to_exactly_one_family() {
        let primitives = PRIMITIVE_OWNERSHIP.iter().map(|entry| entry.family);
        let music = BUILTIN_OWNERSHIP.iter().map(|entry| entry.family);
        let (delta, eliminator) = primitives.clone().fold((0, 0), |(d, e), family| match family {
            Family::Delta { .. } => (d + 1, e),
            Family::Eliminator(_) => (d, e + 1),
            Family::Music => panic!("a `music` operation belongs in the builtin registry"),
        });
        assert_eq!(delta + eliminator, PRIMITIVE_OWNERSHIP.len());
        assert_eq!(
            eliminator, 7,
            "the structural eliminators of §5.6 are nat_fold, list_fold, option_fold, map, filter, range, and repeat"
        );
        assert!(
            music.clone().all(|family| family == Family::Music),
            "every builtin constructs or transforms `music`"
        );
        assert_eq!(
            delta + eliminator + music.count(),
            77,
            "a new compiler operation must be classified before it is admitted"
        );
    }

    #[test]
    fn no_first_order_signature_mentions_a_function() {
        for entry in &PRIMITIVE_OWNERSHIP {
            let Family::Delta { arguments, result } = entry.family else {
                continue;
            };
            for ty in arguments.iter().chain([&result]).map(|shape| shape.ty()) {
                assert!(
                    !mentions_function(&ty),
                    "`{}` is classified δ but its signature contains an arrow; it belongs with the eliminators",
                    entry.spelling
                );
            }
        }
    }

    fn mentions_function(ty: &Type) -> bool {
        match ty {
            Type::Function(..) => true,
            Type::Option(member) | Type::List(member) => mentions_function(member),
            Type::Product(members) => members.iter().any(mentions_function),
            // Listed rather than wildcarded: a new *type former* would otherwise be assumed
            // arrow-free, and this law is the only thing standing between that assumption and
            // §5.8's no-arrow premise.
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
            | Type::ChordClass
            | Type::Triad
            | Type::Roman
            | Type::Voicing
            | Type::Pc12
            | Type::PcSet12
            | Type::Row12
            | Type::Music => false,
        }
    }

    #[test]
    fn a_musical_value_admits_no_destructuring_pattern() {
        // D1: a musical base value is an opaque constant. Nothing takes it apart, which is what
        // lets its reducibility candidate be the plain `SN` clause every base type already has.
        for (ty, literal) in [
            ("Scale", "scale c major"),
            ("Key", "key c major"),
            ("ChordClass", "chord c major"),
            ("Pitch", "c4"),
            ("Interval", "P5"),
        ] {
            for pattern in ["Some(inner)", "[]", "[head, ..others]", "(left, right)"] {
                let source = format!(
                    "piece \"law\" {{ let subject: {ty} = {literal}; \
                     let result: Nat = match subject {{ {pattern} -> 0, _ -> 1 }}; }}"
                );
                let refusals = refusals(&source);
                // Not merely "it failed": a parse error would satisfy that vacuously, and a
                // renamed binding is enough to turn one of these into a parse error. The refusal
                // has to be the type checker saying which type refused.
                assert!(
                    refusals.iter().any(|diagnostic| diagnostic.code == Code::TypeMismatch
                        && diagnostic.labels.iter().any(|label| label.text.contains(ty))),
                    "`{pattern}` over a `{ty}` must be refused by the type checker, naming the \
                     type; saw {refusals:?}"
                );
            }
        }
    }

    /// Why a piece was refused.
    fn refusals(source: &str) -> Vec<Diagnostic> {
        let parsed = musa_language::parse(source);
        let Some(piece) = musa_language::ast::PieceDecl::from_root(&parsed.syntax()) else {
            return Vec::new();
        };
        let mut resolver = Resolver::new();
        check_and_evaluate(
            &mut resolver,
            declarations(piece.syntax(), None).into_iter(),
            Some(piece.syntax()),
            UnknownRootMusic::Reject,
            &Modules::default(),
        );
        resolver.diagnostics
    }

    #[test]
    fn every_first_order_primitive_is_total_on_its_declared_domain() {
        let observed = sampled_applications();
        assert!(
            observed.len() > 1_000,
            "the sample must actually exercise the domains, saw {} applications",
            observed.len()
        );
        let mut exercised: Vec<Primitive> = Vec::new();
        for (primitive, result, answer) in observed {
            if !exercised.contains(&primitive) {
                exercised.push(primitive);
            }
            let Some(value) = answer else {
                panic!(
                    "`{}` returned no value on a well-typed argument tuple; D2 requires partiality \
                     to be declared in the result type, not reported by the evaluator",
                    primitive.name()
                );
            };
            let actual = value_type(&value)
                .unwrap_or_else(|| panic!("`{}` returned a value with no first-order type", primitive.name()));
            assert_eq!(
                actual,
                result.ty(),
                "`{}` returned a `{actual}` where its signature declares `{}`",
                primitive.name(),
                result.ty()
            );
            if !result.admits_absence() {
                assert!(
                    !matches!(value, Value::Option { value: None, .. }),
                    "`{}` reported absence at a result type that cannot express it",
                    primitive.name()
                );
            }
        }
        let declared = PRIMITIVE_OWNERSHIP
            .iter()
            .filter(|entry| matches!(entry.family, Family::Delta { .. }))
            .count();
        assert_eq!(
            exercised.len(),
            declared,
            "every δ-primitive must be reached by the sample; unreached: {:?}",
            PRIMITIVE_OWNERSHIP
                .iter()
                .filter(|entry| matches!(entry.family, Family::Delta { .. }))
                .filter(|entry| !exercised.contains(&entry.operation))
                .map(|entry| entry.spelling)
                .collect::<Vec<_>>()
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
                let source = format!("piece \"law\" {{ fn id(x: Nat) -> Nat {{ x }} let result: Nat = {applied}; }}");
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
        let source = "piece \"law\" { let pair: (Nat, Bool) = (3, true); fn keep(x: (Nat, Bool)) -> (Nat, Bool) { x } let result: (Nat, Bool) = keep(pair); }";
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
                 fn latest(index: Nat, accumulator: Nat) -> Nat {{ index }} \
                 fn item(value: Nat, accumulator: Nat) -> Nat {{ value }} \
                 fn id(value: Nat) -> Nat {{ value }} \
                 fn reject(value: Nat) -> Bool {{ false }} \
                 fn from_option(value: Option<Nat>) -> Nat {{ match value {{ None -> 0, Some(found) -> found }} }} \
                 let by_nat: Nat = nat_fold(0, latest, {count}); \
                 let values: List<Nat> = range({count}); \
                 let mapped: List<Nat> = map(id, values); \
                 let filtered: List<Nat> = filter(reject, mapped); \
                 let by_list: Nat = list_fold(0, item, mapped); \
                 let selected: Nat = from_option(Some(by_list)); \
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
