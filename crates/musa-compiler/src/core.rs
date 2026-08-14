//! The private total elaboration core (`docs/rules/language/02-core-calculus.md`).
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

use crate::core_budget::{Evaluation, Reduction, ResourceError, WorkMeter};
use crate::data::{TypeScope, World};
use crate::diagnose::{Code, Diagnostic};
use crate::imports::Libraries;
use crate::infer::{Kind, Mismatch, Scheme, Unifier};
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
    // The data world is built first, because a declaration is what a written
    // type *means*: a signature member or a `let` naming `Motive` cannot be
    // lowered until this says what `Motive` is.
    let world = World::read(resolver, &data_owners(libraries, root, Some(piece.syntax())));
    let modules = Modules::read(resolver, &world, module_owners(libraries, root));
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
        &world,
        Reading::Source,
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
    let world = World::read(resolver, &data_owners(libraries, root, None));
    let modules = Modules::read(resolver, &world, module_owners(libraries, root));
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
        &world,
        Reading::Source,
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
    let world = World::read(resolver, &data_owners(libraries, root, Some(voice.syntax())));
    let modules = Modules::read(resolver, &world, module_owners(libraries, root));
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
        &world,
        Reading::Source,
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

/// Every place a `data` declaration may be written for this pass: what its
/// imports declare, its own lexical root, and the piece or voice being
/// checked when there is one.
///
/// A wider list than [`module_owners`] by exactly that last node, because a
/// piece may declare data of its own while a signature or a structure written
/// inside one is not a thing the grammar admits.
fn data_owners(libraries: &Libraries, root: &SyntaxNode, inner: Option<&SyntaxNode>) -> Vec<SyntaxNode> {
    libraries
        .each()
        .map(|(_, library)| library.syntax().clone())
        .chain(std::iter::once(root.clone()))
        .chain(inner.cloned())
        .collect()
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
    let world = World::read(
        resolver,
        &libraries
            .each()
            .map(|(_, imported)| imported.syntax().clone())
            .chain(std::iter::once(library.syntax().clone()))
            .collect::<Vec<_>>(),
    );
    let modules = Modules::read(
        resolver,
        &world,
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
        &world,
        Reading::Source,
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
        let data_owners: Vec<_> = owners.iter().map(|(_, node)| node.clone()).collect();
        let world = World::read(&mut foreign_resolver, &data_owners);
        let modules = Modules::read(&mut foreign_resolver, &world, owners.iter().cloned());
        let evaluated = check_and_evaluate(
            &mut foreign_resolver,
            prefix.clone().into_iter(),
            None,
            UnknownRootMusic::Reject,
            &modules,
            &world,
            Reading::Source,
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

/// Which clock a duration or a position is measured against
/// (`docs/rules/language/02-core-calculus.md` §1's `C`).
///
/// A closed pair, and deliberately not a kind: `C` ranges over exactly these
/// two, so a coordinate-polymorphic builtin would be machinery for a
/// two-element domain. Theorem 5 applies to a finite family of inert leaves
/// without it, and promoting the index to a kind later is additive because the
/// tags are already written down.
///
/// The point of carrying it is that `Duration<WrittenTime>` and
/// `Duration<PhysicalTime>` do not unify, so adding a written beat to a number
/// of seconds is not a mistake this language can express.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Coordinate {
    /// Positions and durations as the page counts them.
    WrittenTime,
    /// Positions and durations as a clock counts them, after a time map has
    /// been applied. Nothing in the source language constructs one yet; the
    /// tag exists so that the day one arrives it cannot be quietly mixed with
    /// written time.
    PhysicalTime,
}

impl Coordinate {
    /// The word this coordinate is written with, in a type and in a
    /// diagnostic.
    pub(crate) const fn spelling(self) -> &'static str {
        match self {
            Self::WrittenTime => "WrittenTime",
            Self::PhysicalTime => "PhysicalTime",
        }
    }

    /// The coordinate a written word names.
    fn named(text: &str) -> Option<Self> {
        match text {
            "WrittenTime" => Some(Self::WrittenTime),
            "PhysicalTime" => Some(Self::PhysicalTime),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Type {
    /// A type inference has not decided yet, named by the
    /// [`crate::infer::Unifier`] that made it. It exists only while one
    /// declaration is being checked: every type that leaves the checker has
    /// been resolved, and a variable that survives that is a diagnostic.
    Var(crate::infer::TypeVar),
    Unit,
    Bool,
    Nat,
    Ratio,
    /// Opaque printable text. Storable data, and not a way in: nothing
    /// reads structure out of it, so it cannot carry what a type would
    /// otherwise have to say.
    Text,
    /// *How much* time, in coordinate `C`: a nonnegative exact rational.
    ///
    /// The ordered monoid `(ℚ≥0, +, 0)` of §1's commentary. Two durations add;
    /// the nonnegativity is checked at every constructor, which is why
    /// `duration_of` and `duration_scale` return `Result`.
    Duration(Coordinate),
    /// *When*, in coordinate `C`: an exact rational instant.
    ///
    /// The abelian group `(ℚ, +, 0)`, and a different type from
    /// [`Type::Duration`] on purpose. A position plus a duration is a
    /// position, two positions do not add at all, and their difference is a
    /// duration only when it is nonnegative. One type for both would let beat
    /// 3 and three beats be added, which is the one arithmetic error a tagged
    /// rational exists to catch.
    Position(Coordinate),
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
    /// The binary sum `τ + τ`, written `Result<T, E>`.
    ///
    /// There is no `Type::Result`, and that is the point of the spelling:
    /// `Result` is what a sum is *used for* here, not a second kind of
    /// thing the compiler privileges
    /// (`docs/rules/language/02-core-calculus.md` §1).
    Sum(Box<Self>, Box<Self>),
    Option(Box<Self>),
    List(Box<Self>),
    /// A type a library declared, applied to its arguments — §1's `N[τ, …]`.
    ///
    /// The identity is the declaration, not the name: two packages that both
    /// declare `Motive` declare two types, and this holds which one
    /// ([`crate::data::NominalId`]).
    Nominal(crate::data::NominalId, Vec<Self>),
    Music,
    /// One kind of machine step — §1's `K`, as a type so that it unifies.
    ///
    /// A tag has no values: it appears only as the first argument of
    /// [`Type::Primitive`] and [`Type::Machine`], where it says what one step
    /// of that machine counts. Making it an ordinary type is what lets
    /// `identity` be polymorphic in its step the way it is polymorphic in its
    /// ports — one inference discipline, not a second one for indices.
    Step(crate::machine::StepTag),
    /// `Primitive[K, δ, δ]` — one registered stepping unit whose private state
    /// and step function belong to its owner (§1's fourth decision).
    ///
    /// Source can hold one and hand it to `machine`; it can neither inspect
    /// nor forge the state behind it.
    Primitive {
        step: Box<Self>,
        input: Box<Self>,
        output: Box<Self>,
    },
    /// `Machine[K, δ, δ]` — a finite description of a stepping process, never
    /// the history it produces (`../constitution.md` §4).
    Machine {
        step: Box<Self>,
        input: Box<Self>,
        output: Box<Self>,
    },
    /// A finite syntax value ([`crate::syntax::Syntax`]).
    ///
    /// Phase-local: `../rules/language/02-core-calculus.md` §5 closes the
    /// source type grammar and says the source language has no syntax value,
    /// and this does not widen it. There is no written spelling for this type —
    /// [`named_type`] does not read one — so it cannot be annotated, and the
    /// operations over it are offered only where a transformer is checked.
    Syntax,
    /// Where one node sits ([`crate::syntax::NodePath`]).
    NodePath,
    /// Which name a binder declares ([`crate::syntax::BindingPath`]).
    BindingPath,
    /// `SyntaxStep<C, A>` — one suspended recursive call, sealed to the child
    /// it descends to and the algebra that exposed it.
    ///
    /// Phase-local like [`Type::Syntax`], and the one phase type with
    /// arguments, so `C` and `A` unify the way any other member does. It has
    /// no source constructor: a step is minted only by the recursor's group
    /// case and consumed only by `run_syntax_step`, which is what makes
    /// `docs/rules/language/02-core-calculus.md` §5.9's association lemma a
    /// fact about the value rather than a check someone has to run.
    ///
    /// **Never storable data**, at any depth, for a stronger reason than an
    /// arrow's: what it hides *is* an algebra of source closures. The
    /// exclusion is enforced structurally in [`crate::infer::Unifier`], beside
    /// the arrow's.
    SyntaxStep {
        context: Box<Self>,
        answer: Box<Self>,
    },
    Function(Vec<Self>, Box<Self>),
}

impl std::fmt::Display for Type {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            // A variable prints as a lowercase name, the way a signature
            // would write it if the file had written one: `a`, `b`, … and
            // `a26` onwards once the letters run out.
            Self::Var(variable) => match u8::try_from(*variable) {
                Ok(index) if index < 26 => write!(out, "{}", char::from(b'a'.saturating_add(index))),
                _ => write!(out, "a{variable}"),
            },
            Self::Unit => out.write_str("Unit"),
            Self::Bool => out.write_str("Bool"),
            Self::Nat => out.write_str("Nat"),
            Self::Ratio => out.write_str("Ratio"),
            Self::Text => out.write_str("Text"),
            Self::Duration(coordinate) => write!(out, "Duration<{}>", coordinate.spelling()),
            Self::Position(coordinate) => write!(out, "Position<{}>", coordinate.spelling()),
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
            Self::Sum(value, error) => write!(out, "Result<{value}, {error}>"),
            Self::Option(member) => write!(out, "Option<{member}>"),
            Self::List(member) => write!(out, "List<{member}>"),
            Self::Nominal(id, arguments) => {
                write!(out, "{id}")?;
                for (index, argument) in arguments.iter().enumerate() {
                    out.write_str(if index == 0 { "<" } else { ", " })?;
                    write!(out, "{argument}")?;
                }
                if arguments.is_empty() {
                    Ok(())
                } else {
                    out.write_str(">")
                }
            }
            Self::Music => out.write_str("Music"),
            Self::Step(tag) => write!(out, "{tag}"),
            Self::Primitive { step, input, output } => write!(out, "Primitive<{step}, {input}, {output}>"),
            Self::Machine { step, input, output } => write!(out, "Machine<{step}, {input}, {output}>"),
            // These three print but do not read back: they name themselves in a
            // transformer's diagnostics, and no source anywhere may write one.
            Self::Syntax => out.write_str("Syntax"),
            Self::NodePath => out.write_str("NodePath"),
            Self::BindingPath => out.write_str("BindingPath"),
            Self::SyntaxStep { context, answer } => write!(out, "SyntaxStep<{context}, {answer}>"),
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

/// The sentence that crosses a distinction the language keeps on purpose.
///
/// Most type mismatches are slips, and a slip needs no advice: the two type
/// names already say what went wrong. A few are *category* errors — a chord
/// class written where music was wanted, a degree where a pitch was, a key
/// where a collection was — and those are not slips at all. They are a reader
/// meeting a separation this language makes and ordinary musical talk does
/// not (`docs/book/src/concepts/distinctions.md`), and the useful thing to
/// say is not "these differ" but *which operation crosses the gap, and what it
/// needs from you that the value on its own does not carry*.
///
/// The table is deliberately one-directional per entry. Going from a voicing
/// to its chord class is total and going back is a choice, so the two
/// directions do not get one symmetric sentence; each says its own thing.
/// Every named operation lives in `stdlib/src/`, so a rename that orphans one
/// of these strings shows up in `stdlib/reference.md` in the same commit.
fn crossing_help(expected: &Type, found: &Type) -> Option<&'static str> {
    Some(match (expected, found) {
        // A container of the wrong element is the same confusion one layer
        // out. `Option<Roman>` where `Option<ChordClass>` was wanted is a
        // numeral that has not met a collection, and the sentence about that
        // is the sentence about numerals — the `Option` is not the problem
        // and mentioning it would bury the one that is.
        (Type::Option(expected), Type::Option(found)) | (Type::List(expected), Type::List(found)) => {
            return crossing_help(expected, found);
        }
        (Type::Music, Type::ChordClass) => {
            "a chord class has no register: `close_position` or `voiced_as` chooses the pitches, \
             and `sound_for` gives the result a duration"
        }
        (Type::Music, Type::Voicing) => "a voicing is pitches with no duration: `sound_for(chosen, held)` sounds it",
        (Type::Music, Type::Pitch | Type::PitchClass) => {
            "a pitch is not music until it lasts: write the duration, as in `c4/4`"
        }
        (Type::Pitch, Type::Degree) => {
            "a degree is an ordinal with no octave: `frame_on` registers the collection, and \
             `frame_degree` reads a pitch out of the frame"
        }
        (Type::Pitch, Type::PitchClass) => {
            "a note name has no octave: write one (`c4`), or realize the class against a frame"
        }
        (Type::Degree, Type::Pitch) => {
            "`degree_in(collection, written)` locates a pitch in a collection, and is absent when \
             it is not a member"
        }
        (Type::PitchClass, Type::Pc12) => {
            "a `Pc12` has forgotten its spelling: `spelled_in` chooses one back, against the \
             collection that decides it"
        }
        (Type::Pc12, Type::PitchClass) => "`forget_spelling` is the map into `Pc12`, and it is total",
        (Type::Scale, Type::Key) => {
            "a key is not a collection — C minor is three of them: `key_scale` takes the \
             signature's own collection, or name the one you mean"
        }
        (Type::ChordClass, Type::Roman) => {
            "a numeral carries no collection: `numeral_chord(collection, written)` reads it in one"
        }
        (Type::ChordClass, Type::Voicing) => "`chord_of(chosen)` forgets a voicing down to its class",
        (Type::Voicing, Type::ChordClass) => {
            "`close_position(content, bass)` chooses the pitches, and the bass is yours to name"
        }
        (_, Type::Function(parameters, result)) if **result == *expected => {
            if parameters.is_empty() {
                "this is a function, not its result: call it, as in `name()`"
            } else {
                "this is a function, not its result: apply it to its arguments"
            }
        }
        _ => return None,
    })
}

#[derive(Clone)]
struct RawParameter {
    name: String,
    ty: Type,
    span: SourceSpan,
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
    /// What the name means at a use: an annotated declaration's own type, or
    /// the principal type inferred for it, with everything the declaration
    /// left open quantified. A use instantiates it.
    scheme: Scheme,
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
    /// One injection into a binary sum. Both halves of the type are kept
    /// because the value is one side and the type is both.
    Injection {
        error: bool,
        held: Box<Expr>,
        value_type: Type,
        error_type: Type,
    },
    List(Vec<Expr>),
    Apply {
        function: Box<Expr>,
        arguments: Vec<CallArgument>,
    },
    /// An anonymous function, and the names its body reads from around it.
    ///
    /// `captures` is what makes it a closure rather than a term: the value it
    /// evaluates to holds those names as they stood *here*, so applying it
    /// later cannot see a different `steps` than the one it was written
    /// beside.
    Lambda {
        parameters: Vec<CheckedParameter>,
        result: Type,
        captures: Vec<String>,
        body: Box<Expr>,
    },
    PitchAction {
        pitch: Box<Expr>,
        interval: Box<Expr>,
        down: bool,
    },
    Builtin {
        builtin: Builtin,
        arguments: Vec<Expr>,
    },
    Match {
        scrutinee: Box<Expr>,
        arms: Vec<CheckedArm>,
    },
    /// One saturated use of a library-declared constructor.
    ///
    /// Saturated, because a constructor is not a first-class function here:
    /// a partial one would be an arrow whose type the declaration never wrote
    /// down, and the language already answers "apply it directly" for its own
    /// polymorphic operations.
    Construct {
        id: crate::data::NominalId,
        variant: usize,
        /// The type arguments this use makes the declaration at, kept for the
        /// same reason a list keeps its member: the value is one variant, and
        /// the type is the whole declaration.
        arguments: Vec<Type>,
        fields: Vec<Expr>,
    },
    /// One saturated use of a declaration's generated fold: one case per
    /// constructor of the group, in the order [`crate::data::Folding`] gives
    /// them, and then the value.
    Fold {
        cases: Vec<Expr>,
        shape: Vec<(crate::data::NominalId, usize)>,
        value: Box<Expr>,
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
    term: musa_kernel::Term<musa_kernel::WrittenTime, crate::elaborate::ScoreFact>,
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
enum Builtin {
    NatFold,
    ListFoldFromStart,
    ListFoldFromEnd,
    OptionFold,
    Map,
    Filter,
    Range,
    Repeat,
    RatioAdd,
    RatioSub,
    RatioMul,
    RatioDiv,
    RatioLess,
    RatioEqual,
    TextEqual,
    NatAdd,
    NatMul,
    NatSub,
    DurationOf,
    DurationRatio,
    DurationAdd,
    DurationScale,
    DurationLess,
    DurationEqual,
    PositionOf,
    PositionRatio,
    PositionShift,
    PositionBetween,
    PositionLess,
    PositionEqual,
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
    PcSet12Transposed,
    PcSet12Inverted,
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
    Transpose,
    Stretch,
    Retrograde,
    Invert,
    Shift,
    Together,
    MapNotePitches,
    Play,
    Primitive,
    Machine,
    Identity,
    Connect,
    Beside,
    Feedback,
    Copy,
    Drop,
    Swap,
    /// A phase-local syntax operation — see [`SyntaxOp`].
    ///
    /// It is a case of [`Builtin`] because there is **one** evaluator and one
    /// checker: a second operation type would be a second machine, which is the
    /// blocker prompt 127da exists to close. It is not a case of [`Family`],
    /// because §5.8's four families are the four families of *ordinary source*
    /// and these are not offered there — [`SYNTAX_OWNERSHIP`] owns them
    /// instead, and [`Builtin::named`] cannot return one.
    Syntax(SyntaxOp),
}

impl MachineTree {
    /// How much of the evaluator's budget this description occupies: one node
    /// per node, and its stored bytes as its size.
    fn shape(&self) -> (u64, u64) {
        let stored = |bytes: &[u8]| u64::try_from(bytes.len()).unwrap_or(u64::MAX);
        match self {
            Self::Identity | Self::Copy | Self::Drop | Self::Swap => (1, 0),
            Self::Primitive { configuration, .. } => (1, stored(configuration)),
            Self::Connect(first, second) | Self::Beside(first, second) => {
                let ((our_nodes, our_bytes), (their_nodes, their_bytes)) = (first.shape(), second.shape());
                (
                    our_nodes.saturating_add(their_nodes).saturating_add(1),
                    our_bytes.saturating_add(their_bytes),
                )
            }
            Self::Feedback { initial, inner } => {
                let (nodes, bytes) = inner.shape();
                (nodes.saturating_add(1), bytes.saturating_add(stored(initial)))
            }
        }
    }

    /// Append this description's nodes to `nodes`, children first, and answer
    /// where its own node landed.
    ///
    /// Children before parents is the order [`crate::MachineSpec`] promises,
    /// and it is what lets a consumer walk the array once, forwards, with
    /// every index it reads already filled in.
    fn flatten(&self, nodes: &mut Vec<crate::machine::SpecNode>) -> usize {
        use crate::machine::{SpecForm, SpecNode};
        let node = match self {
            Self::Primitive {
                descriptor,
                configuration,
            } => SpecNode::primitive(descriptor, configuration.clone()),
            Self::Identity => SpecNode::wiring(SpecForm::Identity, Vec::new()),
            Self::Copy => SpecNode::wiring(SpecForm::Copy, Vec::new()),
            Self::Drop => SpecNode::wiring(SpecForm::Drop, Vec::new()),
            Self::Swap => SpecNode::wiring(SpecForm::Swap, Vec::new()),
            Self::Connect(first, second) | Self::Beside(first, second) => {
                let children = vec![first.flatten(nodes), second.flatten(nodes)];
                let form = if matches!(self, Self::Connect(_, _)) {
                    SpecForm::Connect
                } else {
                    SpecForm::Beside
                };
                SpecNode::wiring(form, children)
            }
            Self::Feedback { initial, inner } => {
                let children = vec![inner.flatten(nodes)];
                SpecNode::initialized(SpecForm::Feedback, children, initial.clone())
            }
        };
        nodes.push(node);
        nodes.len().saturating_sub(1)
    }
}

/// Write `value`'s exact bytes, or answer `None` where it has none.
///
/// This is §1.1's storable data, encoded: every form that can be a machine
/// port, a registered unit's configuration, or a feedback value has a case
/// here, and the forms that cannot — a spelled pitch and its relatives, a
/// closure, contextual music — have none. A value with no encoding is not one
/// this language could have put in a configuration position, so answering
/// `None` is a statement about the caller rather than a gap.
///
/// Each case writes a distinguishing tag and frames every variable-length
/// part, so two different values cannot encode to one byte string.
/// The byte a coordinate encodes as, for [`encode_exactly`].
const fn coordinate_tag(coordinate: Coordinate) -> u8 {
    match coordinate {
        Coordinate::WrittenTime => 0,
        Coordinate::PhysicalTime => 1,
    }
}

fn encode_exactly(value: &Value, bytes: &mut Vec<u8>) -> Option<()> {
    fn framed(bytes: &mut Vec<u8>, part: &[u8]) {
        bytes.extend_from_slice(&u64::try_from(part.len()).unwrap_or(u64::MAX).to_be_bytes());
        bytes.extend_from_slice(part);
    }
    fn counted(bytes: &mut Vec<u8>, count: usize) {
        bytes.extend_from_slice(&u64::try_from(count).unwrap_or(u64::MAX).to_be_bytes());
    }
    fn exact(bytes: &mut Vec<u8>, value: Ratio<i64>) {
        bytes.extend_from_slice(&value.numer().to_be_bytes());
        bytes.extend_from_slice(&value.denom().to_be_bytes());
    }
    match value {
        Value::Bool(held) => {
            bytes.push(0);
            bytes.push(u8::from(*held));
        }
        Value::Nat(held) => {
            bytes.push(1);
            bytes.extend_from_slice(&held.to_be_bytes());
        }
        Value::Ratio(held) => {
            bytes.push(2);
            exact(bytes, *held);
        }
        // The coordinate is part of the value, not decoration: a written
        // beat and the same number of seconds are two values, and an encoding
        // that dropped the tag would make them one.
        Value::Duration(coordinate, held) => {
            bytes.push(3);
            bytes.push(coordinate_tag(*coordinate));
            exact(bytes, *held);
        }
        Value::Position(coordinate, held) => {
            bytes.push(11);
            bytes.push(coordinate_tag(*coordinate));
            exact(bytes, *held);
        }
        Value::Text(held) => {
            bytes.push(4);
            framed(bytes, held.as_bytes());
        }
        Value::Product(members) => {
            bytes.push(5);
            counted(bytes, members.len());
            for member in members {
                encode_exactly(member, bytes)?;
            }
        }
        Value::Option { value, .. } => {
            bytes.push(6);
            match value {
                Some(held) => {
                    bytes.push(1);
                    encode_exactly(held, bytes)?;
                }
                None => bytes.push(0),
            }
        }
        Value::List { values, .. } => {
            bytes.push(7);
            counted(bytes, values.len());
            for member in values {
                encode_exactly(member, bytes)?;
            }
        }
        // The three phase-local values encode exactly, because two syntax
        // values are the same value exactly when they were written the same
        // way and carry the same derived paths. Their own writers frame every
        // variable-length part for the same reason the cases above do.
        Value::Syntax(held) => {
            bytes.push(8);
            let mut written = Vec::new();
            held.write_into(&mut written);
            framed(bytes, &written);
        }
        Value::NodePath(held) => {
            bytes.push(9);
            let mut written = Vec::new();
            held.write_into(&mut written);
            framed(bytes, &written);
        }
        Value::BindingPath(held) => {
            bytes.push(10);
            let mut written = Vec::new();
            held.write_into(&mut written);
            framed(bytes, &written);
        }
        Value::Pitch(_)
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
        | Value::Sum { .. }
        | Value::Data { .. }
        | Value::Music(_)
        | Value::Primitive { .. }
        | Value::Machine { .. }
        | Value::Closure(_)
        // Refused beside the closure, and for the same reason: a sealed step
        // holds an algebra of them. Nothing storable can be one, so nothing
        // exactly encoded ever is.
        | Value::SyntaxStep(_)
        | Value::Builtin(_) => return None,
    }
    Some(())
}

/// A base type as a builtin signature names it.
///
/// These are the inert types of `docs/rules/language/02-core-calculus.md` §5.8: a closed value of one is
/// an opaque constant, no reduction rule inspects its structure, and everything observable about it
/// is observed by applying a builtin. That is condition D1, and it holds here by construction —
/// there is no variant for a type with an eliminator.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Base {
    Bool,
    Nat,
    /// An exact rational. Signed, and the ordinary arithmetic base: §1's
    /// commentary says the refinements live at the constructors of the tagged
    /// types rather than in a second numeric type.
    Ratio,
    /// Opaque printable text. Inert in D1's sense — nothing reads structure
    /// out of it — which is what lets it be the error half of a `Result`
    /// without giving a builtin a second way to say what went wrong.
    Text,
    Duration(Coordinate),
    Position(Coordinate),
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

/// An argument or result type of a δ-builtin.
///
/// There is deliberately no arrow constructor. §5.8's no-arrow premise is therefore true of every
/// declared δ signature by construction rather than by inspection, and a builtin that wanted a
/// function argument could not be spelled here at all — it would have to join the eliminators,
/// which is exactly the classification the theorem depends on.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Shape {
    Base(Base),
    Option(&'static Self),
    List(&'static Self),
    Product(&'static [Self]),
    /// `Result<value, error>` — how a builtin with more than one way to
    /// fail says which one happened. `Option` says only *that* it did.
    Result(&'static Self, &'static Self),
}

impl Shape {
    /// The core type this shape denotes.
    fn ty(self) -> Type {
        match self {
            Self::Base(base) => base.ty(),
            Self::Option(member) => Type::Option(Box::new(member.ty())),
            Self::List(member) => Type::List(Box::new(member.ty())),
            Self::Product(members) => Type::Product(members.iter().map(|member| member.ty()).collect()),
            Self::Result(value, error) => Type::Sum(Box::new(value.ty()), Box::new(error.ty())),
        }
    }

    /// Whether absence is expressible in this shape's outermost position.
    ///
    /// D2 lets a builtin be partial only by saying so in its result type, so this is what the
    /// sampling law consults before it accepts a `None` from an evaluator.
    ///
    /// A `Result` is *not* absence: both of its injections are values, so a builtin that
    /// returns one owes the law a value on every input. Saying which way it failed is a
    /// stronger promise than saying that it did, and this is where the law reads it.
    #[cfg(test)]
    const fn admits_absence(self) -> bool {
        matches!(self, Self::Option(_))
    }

    /// Whether this shape denotes storable data (`02-core-calculus.md` §1.1).
    ///
    /// A registered builtin's arguments and result must be storable: it is
    /// handed values that could equally have been written in a data field, and
    /// it hands one back. That is what makes it *foreign* rather than a second
    /// evaluator — it cannot receive a closure to call, a music value to walk,
    /// or anything else whose meaning depends on the elaboration around it.
    ///
    /// Today this is true of every shape that can be spelled, because [`Base`]
    /// names only storable domains and [`Shape`] has no arrow. It is written
    /// out rather than assumed so that the day a base is added the question is
    /// asked at the registration and answered in the build, not discovered at
    /// a call.
    const fn is_storable(self) -> bool {
        match self {
            Self::Base(base) => base.is_storable(),
            Self::Option(member) | Self::List(member) => member.is_storable(),
            Self::Product(members) => all_storable(members),
            Self::Result(value, error) => value.is_storable() && error.is_storable(),
        }
    }
}

impl Base {
    /// Every base a δ signature can name is storable data.
    ///
    /// Exhaustive rather than `true`, so that a base for a non-storable domain
    /// — music, a running signal, anything holding a function — has to answer
    /// this question before it can be registered.
    const fn is_storable(self) -> bool {
        match self {
            Self::Bool
            | Self::Nat
            | Self::Ratio
            | Self::Text
            | Self::Duration(_)
            | Self::Position(_)
            | Self::Pitch
            | Self::PitchClass
            | Self::Interval
            | Self::Key
            | Self::Scale
            | Self::Degree
            | Self::Frame
            | Self::ChordClass
            | Self::Triad
            | Self::Roman
            | Self::Voicing
            | Self::Pc12
            | Self::PcSet12
            | Self::Row12 => true,
        }
    }

    fn ty(self) -> Type {
        match self {
            Self::Bool => Type::Bool,
            Self::Nat => Type::Nat,
            Self::Ratio => Type::Ratio,
            Self::Text => Type::Text,
            Self::Duration(coordinate) => Type::Duration(coordinate),
            Self::Position(coordinate) => Type::Position(coordinate),
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

/// Whether every shape in a signature position is storable data.
///
/// Written as a recursion over the slice rather than a loop over indices
/// because it runs in a `const` context, where the registration is checked.
const fn all_storable(shapes: &[Shape]) -> bool {
    match shapes {
        [] => true,
        [first, rest @ ..] => first.is_storable() && all_storable(rest),
    }
}

/// Which of `02-core-calculus.md` §5.8's families a builtin belongs to.
///
/// The families are disjoint and exhaustive, which is what lets Theorem 5 be stated once instead of
/// once per musical domain. A new domain is admissible when its operations can be declared here as
/// `Delta` and discharge D1–D4; it does not get a new induction.
///
/// All four of §5.8's families are spelled here.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Family {
    /// §5.8's **δ-builtins**: first-order and arrow-free. Covered by Theorem 5 once D1–D4 hold,
    /// and the declared signature is the single statement of the operation's type: the checker
    /// reads argument and result types from it rather than restating them.
    Delta { arguments: &'static [Shape], result: Shape },
    /// §5.8's **structural eliminators**: takes a function argument or carries a rank-1 scheme.
    /// Covered by §5.6, and checked by hand because its type depends on its arguments' types.
    Eliminator(Eliminator),
    /// §5.8's **track builtins**: constructs or transforms an event track. Covered by §5.7.
    Track,
    /// §5.8's **machine builtins**: the nine forms that build a finite machine description
    /// (`../across-stages/03-machine-calculus.md` §2). Each carries a rank-1 scheme, the way the
    /// eliminators do, except `primitive`, whose type the build-local registry supplies.
    Machine(MachineOp),
}

/// The nine machine builtins of `../across-stages/03-machine-calculus.md` §2.
///
/// A closed set, for the same reason [`Eliminator`] is one: §2's admissible forms are exactly
/// these, and a tenth would be a change to the calculus rather than an addition to a library.
/// Seven of them are pure wiring and say nothing about what is being wired; `primitive` names a
/// registered unit, and `machine` is how one becomes a machine.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum MachineOp {
    /// `primitive(name, version, configuration)` — one instance of a registered unit.
    Primitive,
    /// `machine(p)` — §2's lifting of a registered unit into a machine.
    Machine,
    Identity,
    Connect,
    Beside,
    Feedback,
    Copy,
    Drop,
    Swap,
}

impl MachineOp {
    /// How many arguments this form is written with. Four of them take none: `identity`, `copy`,
    /// `drop`, and `swap` *are* machines, not functions to one, so they are named rather than
    /// applied.
    const fn arity(self) -> usize {
        match self {
            Self::Identity | Self::Copy | Self::Drop | Self::Swap => 0,
            Self::Machine => 1,
            Self::Connect | Self::Beside | Self::Feedback => 2,
            Self::Primitive => 3,
        }
    }

    /// This form's type, as a rank-1 scheme, instantiated fresh.
    ///
    /// Every port is a **data** variable, which is §1.1's storable-data rule doing the whole of
    /// the work: a machine whose port held a function would need that variable bound to an arrow,
    /// and [`crate::infer::Unifier::bind`] refuses one at any depth. There is no second check for
    /// a hidden closure because there is no way to write one down.
    ///
    /// The step is an **ordinary** variable, so `connect` is polymorphic in what a step counts
    /// while still forcing its two arguments to agree. What stops a step position from being
    /// filled by something that is not a step at all is [`machine_type`], at the one place a step
    /// can be written: the first argument of `Machine<…>` or `Primitive<…>`.
    ///
    /// `primitive` has no scheme. Its result is the unit the literal name and version select from
    /// the build-local registry, so it is checked by a rule of its own rather than by unification
    /// against a type written here.
    fn instantiate(self, unifier: &mut Unifier) -> Option<Type> {
        let step = unifier.fresh(Kind::Ordinary);
        let mut port = || unifier.fresh(Kind::Data);
        let (input, output) = (port(), port());
        let machine = |step: &Type, input: Type, output: Type| Type::Machine {
            step: Box::new(step.clone()),
            input: Box::new(input),
            output: Box::new(output),
        };
        Some(match self {
            Self::Primitive => return None,
            Self::Machine => Type::Function(
                vec![Type::Primitive {
                    step: Box::new(step.clone()),
                    input: Box::new(input.clone()),
                    output: Box::new(output.clone()),
                }],
                Box::new(machine(&step, input, output)),
            ),
            Self::Identity => machine(&step, input.clone(), input),
            Self::Connect => {
                let last = port();
                Type::Function(
                    vec![
                        machine(&step, input.clone(), output.clone()),
                        machine(&step, output, last.clone()),
                    ],
                    Box::new(machine(&step, input, last)),
                )
            }
            Self::Beside => {
                let (other_input, other_output) = (port(), port());
                Type::Function(
                    vec![
                        machine(&step, input.clone(), output.clone()),
                        machine(&step, other_input.clone(), other_output.clone()),
                    ],
                    Box::new(machine(
                        &step,
                        Type::Product(vec![input, other_input]),
                        Type::Product(vec![output, other_output]),
                    )),
                )
            }
            // The stored value is a port of the inner machine on both sides: it goes in as the
            // state this step began with and comes out as the state the next step begins with.
            // That is what makes the loop initialized rather than instantaneous — there is no
            // way to write `feedback` without saying what the first step reads.
            Self::Feedback => {
                let stored = port();
                Type::Function(
                    vec![
                        stored.clone(),
                        machine(
                            &step,
                            Type::Product(vec![input.clone(), stored.clone()]),
                            Type::Product(vec![output.clone(), stored]),
                        ),
                    ],
                    Box::new(machine(&step, input, output)),
                )
            }
            Self::Copy => machine(&step, input.clone(), Type::Product(vec![input.clone(), input])),
            Self::Drop => machine(&step, input, Type::Unit),
            Self::Swap => machine(
                &step,
                Type::Product(vec![input.clone(), output.clone()]),
                Type::Product(vec![output, input]),
            ),
        })
    }
}

/// The eight structural eliminators of `02-core-calculus.md` §5.6.
///
/// They are named as a closed set rather than matched out of [`Builtin`] because §5.6's proof is
/// about exactly these eight. Naming them here is what lets the checker's remaining hand-written
/// arms be exhaustive: once a builtin's family is an `Eliminator`, which one it is has already
/// been decided, and no arm is left over for the sixty-two δ-builtins to fall into by accident.
///
/// `list` has two because both directions have demonstrated consumers: its outermost cons holds
/// the *first* element, so folding from the outside in and accumulating from the start can
/// disagree. The projection step `s(x,a) = x` is one witness. The two folds share one type, which
/// is exactly why the direction has to be in the name. Other structures may admit ordered
/// traversals too; they keep one canonical eliminator until another primitive earns a caller.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Eliminator {
    NatFold,
    ListFoldFromStart,
    ListFoldFromEnd,
    OptionFold,
    Map,
    Filter,
    Range,
    Repeat,
}

impl Eliminator {
    const fn arity(self) -> usize {
        match self {
            Self::NatFold | Self::ListFoldFromStart | Self::ListFoldFromEnd | Self::OptionFold => 3,
            Self::Map | Self::Filter | Self::Repeat => 2,
            Self::Range => 1,
        }
    }

    /// This eliminator's type, as a rank-1 scheme, instantiated fresh.
    ///
    /// All eight of them *are* rank-1 schemes, which is why none is checked
    /// by hand any more. What the old arms tested one `if` at a time — that
    /// `map`'s first argument is a one-argument function, that a `filter`
    /// predicate returns `bool`, that a list fold's step takes the member and
    /// the accumulator and gives the accumulator back — is what unifying an
    /// application against these types says once, in the same words, with the
    /// same diagnostic every other type error gets. Nothing was kept: there
    /// is no eliminator here whose type needs a rank the source language does
    /// not have.
    ///
    /// Every variable here is **ordinary**.
    /// `docs/rules/language/02-core-calculus.md` §1.1 says when a `list`
    /// *is* storable data, not what a `list` may hold: a `List<Triad ->
    /// Triad>` is a perfectly good value type that simply cannot be stored,
    /// and `std/transformational.musa`'s `chain` folds over exactly that.
    /// A data variable belongs where §1.1 says storable data is *required* —
    /// an event-track payload, a machine port, a registered primitive's
    /// configuration — and the only such position this type set has is a
    /// quotation's payload, which is a written name rather than an inferred
    /// type. `EventTrack[C, δ]` at prompt 127c is what first puts a variable
    /// in one.
    fn instantiate(self, unifier: &mut Unifier) -> Type {
        match self {
            Self::Range => Type::Function(vec![Type::Nat], Box::new(Type::List(Box::new(Type::Nat)))),
            Self::Repeat => {
                let member = unifier.fresh(Kind::Ordinary);
                Type::Function(vec![member.clone(), Type::Nat], Box::new(Type::List(Box::new(member))))
            }
            Self::Map => {
                let from = unifier.fresh(Kind::Ordinary);
                let to = unifier.fresh(Kind::Ordinary);
                Type::Function(
                    vec![
                        Type::Function(vec![from.clone()], Box::new(to.clone())),
                        Type::List(Box::new(from)),
                    ],
                    Box::new(Type::List(Box::new(to))),
                )
            }
            Self::Filter => {
                let member = unifier.fresh(Kind::Ordinary);
                Type::Function(
                    vec![
                        Type::Function(vec![member.clone()], Box::new(Type::Bool)),
                        Type::List(Box::new(member.clone())),
                    ],
                    Box::new(Type::List(Box::new(member))),
                )
            }
            Self::NatFold => {
                let accumulator = unifier.fresh(Kind::Ordinary);
                Type::Function(
                    vec![
                        accumulator.clone(),
                        Type::Function(vec![Type::Nat, accumulator.clone()], Box::new(accumulator.clone())),
                        Type::Nat,
                    ],
                    Box::new(accumulator),
                )
            }
            // The two directions share one type. That is the point: a call migrates between them
            // by changing one word, and a reader comparing two call sites compares only the word
            // that differs. Which end the fold runs from is in the name because it cannot be in
            // the type.
            Self::ListFoldFromStart | Self::ListFoldFromEnd => {
                let member = unifier.fresh(Kind::Ordinary);
                let accumulator = unifier.fresh(Kind::Ordinary);
                Type::Function(
                    vec![
                        accumulator.clone(),
                        Type::Function(vec![member.clone(), accumulator.clone()], Box::new(accumulator.clone())),
                        Type::List(Box::new(member)),
                    ],
                    Box::new(accumulator),
                )
            }
            Self::OptionFold => {
                let member = unifier.fresh(Kind::Ordinary);
                let accumulator = unifier.fresh(Kind::Ordinary);
                Type::Function(
                    vec![
                        accumulator.clone(),
                        Type::Function(vec![member.clone()], Box::new(accumulator.clone())),
                        Type::Option(Box::new(member)),
                    ],
                    Box::new(accumulator),
                )
            }
        }
    }
}

/// The fourteen phase-local syntax operations.
///
/// A closed set, like [`Eliminator`] and [`MachineOp`], and deliberately *not* a
/// [`Family`]: §5.8's four families classify the builtins ordinary source can
/// name, and none of these is one of those. They are reachable only where
/// [`Checker::expansion`] is set, which is the whole of what "phase-local"
/// means here — one core, one evaluator, and an environment that offers more
/// names in one place.
///
/// Every path a transformer holds was *derived*: [`Self::Recurse`] hands each
/// input node its own structural path, and [`Self::Built`] and
/// [`Self::Binding`] derive a new one from a path already held. Nothing here
/// takes a number and returns a path, and nothing mints a fresh id, which is
/// the repair `37-final-blocker.md` §1 and `34-proof-review.md` asked for.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum SyntaxOp {
    /// `recurse_syntax(missing, token, identifier, group, context, subject)` —
    /// the way into a syntax value, with each branch receiving the inherited
    /// context and the node's own path.
    ///
    /// The group branch is handed a `List<SyntaxStep<C,A>>` rather than a
    /// `List<A>`: it decides whether, in what order, and under what context
    /// each child is read. Prompt 127da's "the fold is the only way in" is
    /// superseded here — `Syntax` is still opaque, paths are still derived,
    /// and what changed is only that an adapter may look at a node before
    /// reading its children (`../rules/language/02-core-calculus.md` §5.9).
    Recurse,
    /// `run_syntax_step(context, next)` — resume one sealed step.
    ///
    /// A builtin and not callable syntax, for two reasons that point the same
    /// way. A step spelled `next(c)` would *be* a function type, so any `C ->
    /// A` would unify with it and sealing would stop being a fact about the
    /// type; and an arrow-shaped step could not be excluded from `d` for the
    /// reason it must be.
    Run,
    /// `syntax_fold_from_leaves(missing, token, identifier, group, subject)` —
    /// the derived bottom-up fold: every child is read, in source order,
    /// before its group's branch runs.
    ///
    /// Named for which end it runs from, because that is the behaviour a
    /// caller has to plan around and it cannot be in the type — the same
    /// argument prompt 127dcfaa made for the two list folds. It is
    /// [`Self::Recurse`] at a context nothing reads, and the traversal below
    /// is literally the same function in its other mode.
    Fold,
    /// `syntax_at(subject, path)` — the input node at `path`, if there is one.
    /// How a transformer preserves input with its source information intact.
    At,
    /// `syntax_anchor(subject, path, here)` — the anchor of the input node at
    /// `path`, as a node built at `here`.
    ///
    /// An anchor is how a value remembers where it came from: the number this
    /// builds into the emitted expression is an index into the expansion
    /// record's table of the region's own ranges, so a package function
    /// complaining about the fourth connection three passes later is still
    /// complaining about the fourth connection. It answers with a *node* and
    /// not with the number because a transformer may emit a place and may not
    /// read one (`26-language-design-decision.md` §3.4) — this hands back
    /// something to splice, and nothing to compare.
    Anchor,
    /// `syntax_number(node)` — the exact rational a numeric token spells.
    ///
    /// The reader has already read it: the lexer keeps `3/8` whole as one
    /// `Rational` token and `4` as one `Integer`, so this hands back a reading
    /// the compiler performed rather than making a transformer re-derive one
    /// from text. Nothing is revealed that the fold did not already reveal —
    /// the transformer can see the same token's spelling — and what is saved is
    /// a conversion the phase has no operation for and no finite table could
    /// stand in for, since a written span is an arbitrary rational.
    Number,
    /// `syntax_built(path, role, child)` — an output path derived from `path`.
    Built,
    /// `syntax_binding(path, role)` — the binding `path` declares at `role`.
    Binding,
    /// `syntax_token(path, kind, text)`.
    Token,
    /// `syntax_identifier(path, name)` — a name the composer's own source binds.
    Identifier,
    /// `syntax_group(path, delimiter, children)`.
    Group,
    /// `syntax_binder(binding, name)` — the declaration of a name the expansion
    /// introduces.
    Binder,
    /// `syntax_reference(path, binding, name)` — a use of one.
    Reference,
    /// `checked_expression(subject)` — the gate, answering with the value or
    /// with what is wrong with it.
    Checked,
}

/// What kind of phase-local operation a [`SyntaxOp`] is.
///
/// Two cases, not four: this registry is small on purpose, and the split that
/// matters is between the operations that eliminate a syntax value and the
/// total first-order builders around it. "Descent into syntax happens in
/// exactly one place" is then a fact the registry states rather than a claim a
/// reader has to count out.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum PhaseFamily {
    /// Descent: [`SyntaxOp::Recurse`], [`SyntaxOp::Run`], and the derived
    /// [`SyntaxOp::Fold`]. Each takes function arguments and carries a rank-1
    /// scheme, so §5.6's account of an eliminator applies to them unchanged,
    /// and all three run the *same* traversal — [`recurse_syntax`] — so there
    /// is one descent in the implementation and not three.
    Fold,
    /// A total first-order operation over syntax values and paths. Each is a
    /// function of its displayed arguments and nothing else — no counter, no
    /// clock, no compiler state — which is what makes two runs agree exactly.
    Builder,
}

impl SyntaxOp {
    /// How many arguments this operation is written with.
    const fn arity(self) -> usize {
        match self {
            Self::Checked | Self::Number => 1,
            Self::At | Self::Binding | Self::Identifier | Self::Binder | Self::Run => 2,
            Self::Anchor | Self::Built | Self::Token | Self::Group | Self::Reference => 3,
            Self::Fold => 5,
            Self::Recurse => 6,
        }
    }

    /// The operation this name spells, where the phase environment is in scope.
    ///
    /// Separate from [`Builtin::named`] on purpose: ordinary source looks names
    /// up there and there only, so a piece that writes `syntax_group` gets the
    /// same "cannot find" it would get for any other unbound name.
    fn named(name: &str) -> Option<Self> {
        let entry = SYNTAX_OWNERSHIP.iter().find(|entry| entry.spelling == name)?;
        debug_assert!(!entry.hidden_information.is_empty());
        Some(entry.operation)
    }

    fn spelling(self) -> &'static str {
        SYNTAX_OWNERSHIP
            .iter()
            .find(|entry| entry.operation == self)
            .map_or("syntax", |entry| entry.spelling)
    }

    /// This operation's type, as a rank-1 scheme, instantiated fresh.
    ///
    /// Only the fold has a variable in it — what the transformer is folding
    /// *to* — and that variable is **ordinary**: a fold may build a list of
    /// functions as readily as a list of syntax. Everything else is
    /// monomorphic, because a builder's argument and result types are decided
    /// by which builder it is.
    fn instantiate(self, unifier: &mut Unifier) -> Type {
        let syntax = || Type::Syntax;
        let path = || Type::NodePath;
        match self {
            Self::Fold => {
                let to = unifier.fresh(Kind::Ordinary);
                let step = |arguments: Vec<Type>| Type::Function(arguments, Box::new(to.clone()));
                Type::Function(
                    vec![
                        step(vec![path()]),
                        step(vec![path(), Type::Text, Type::Text]),
                        step(vec![path(), Type::Text]),
                        step(vec![path(), Type::Text, Type::List(Box::new(to.clone()))]),
                        syntax(),
                    ],
                    Box::new(to),
                )
            }
            // Two variables, both **ordinary**: an adapter may inherit a
            // function and answer with one, and program five of prompt
            // 127dcfae's trial does both at once. Rank stays 1 — they are
            // quantified here, at the outside of this one scheme, and
            // `SyntaxStep<C, A>` is a type constructor over them rather than
            // a quantifier of its own.
            Self::Recurse => {
                let context = unifier.fresh(Kind::Ordinary);
                let to = unifier.fresh(Kind::Ordinary);
                let branch = |mut arguments: Vec<Type>| {
                    arguments.insert(0, context.clone());
                    Type::Function(arguments, Box::new(to.clone()))
                };
                let sealed = Type::SyntaxStep {
                    context: Box::new(context.clone()),
                    answer: Box::new(to.clone()),
                };
                Type::Function(
                    vec![
                        branch(vec![path()]),
                        branch(vec![path(), Type::Text, Type::Text]),
                        branch(vec![path(), Type::Text]),
                        branch(vec![path(), Type::Text, Type::List(Box::new(sealed))]),
                        context,
                        syntax(),
                    ],
                    Box::new(to),
                )
            }
            // The context comes first for the reason the folding use makes
            // plain: `fn (kid, later) { run_syntax_step(later, kid) }` puts
            // the context where the accumulator is.
            Self::Run => {
                let context = unifier.fresh(Kind::Ordinary);
                let to = unifier.fresh(Kind::Ordinary);
                let sealed = Type::SyntaxStep {
                    context: Box::new(context.clone()),
                    answer: Box::new(to.clone()),
                };
                Type::Function(vec![context, sealed], Box::new(to))
            }
            Self::At => Type::Function(vec![syntax(), path()], Box::new(Type::Option(Box::new(Type::Syntax)))),
            // Three arguments and not two: the node it is *about*, and the
            // place the node it hands back stands in. The answer is optional
            // for `syntax_at`'s reason — an adapter anchors a node it holds,
            // and a path it derived addresses no input node at all.
            Self::Anchor => Type::Function(
                vec![syntax(), path(), path()],
                Box::new(Type::Option(Box::new(Type::Syntax))),
            ),
            // `Option` because a node that is not a numeric token is not a
            // number, and `Ratio` because one operation covering both numeric
            // kinds is one operation an adapter has to learn.
            Self::Number => Type::Function(vec![syntax()], Box::new(Type::Option(Box::new(Type::Ratio)))),
            Self::Built => Type::Function(vec![path(), Type::Nat, Type::Nat], Box::new(Type::NodePath)),
            Self::Binding => Type::Function(vec![path(), Type::Nat], Box::new(Type::BindingPath)),
            Self::Token => Type::Function(vec![path(), Type::Text, Type::Text], Box::new(Type::Syntax)),
            Self::Identifier => Type::Function(vec![path(), Type::Text], Box::new(Type::Syntax)),
            Self::Group => Type::Function(
                vec![path(), Type::Text, Type::List(Box::new(Type::Syntax))],
                Box::new(Type::Syntax),
            ),
            Self::Binder => Type::Function(vec![Type::BindingPath, Type::Text], Box::new(Type::Syntax)),
            Self::Reference => Type::Function(vec![path(), Type::BindingPath, Type::Text], Box::new(Type::Syntax)),
            // The gate says which of several things is wrong, so it answers
            // with a `Result` rather than an `Option`, exactly as a δ-builtin
            // with more than one way to fail does.
            Self::Checked => Type::Function(
                vec![syntax()],
                Box::new(Type::Sum(Box::new(Type::Syntax), Box::new(Type::Text))),
            ),
        }
    }
}

#[derive(Clone, Copy)]
struct BuiltinOwnership<T, F = Family> {
    operation: T,
    spelling: &'static str,
    hidden_information: &'static str,
    family: F,
}

/// The phase-local registry.
///
/// Separate from [`BUILTIN_OWNERSHIP`] so that §5.8's four families stay the
/// four families of the source core: nothing here is a δ-builtin, an
/// eliminator, a track builtin, or a machine builtin, and nothing here is
/// looked up when ordinary source reads a name. Each entry says what it hides,
/// for the same reason the source entries do — an operation earns a place in a
/// compiler-owned registry by hiding something a library could not.
const SYNTAX_OWNERSHIP: [BuiltinOwnership<SyntaxOp, PhaseFamily>; 14] = [
    BuiltinOwnership {
        operation: SyntaxOp::Recurse,
        spelling: "recurse_syntax",
        hidden_information: "the reader's node representation, each node's structural path, and the suspended entry \
                             into a proper child",
        family: PhaseFamily::Fold,
    },
    BuiltinOwnership {
        operation: SyntaxOp::Run,
        spelling: "run_syntax_step",
        hidden_information: "which child and which algebra a step was minted for",
        family: PhaseFamily::Fold,
    },
    BuiltinOwnership {
        operation: SyntaxOp::Fold,
        spelling: "syntax_fold_from_leaves",
        hidden_information: "the reader's node representation and each node's structural path",
        family: PhaseFamily::Fold,
    },
    BuiltinOwnership {
        operation: SyntaxOp::At,
        spelling: "syntax_at",
        hidden_information: "descent into the reader's node representation, and a node's untouched source information",
        family: PhaseFamily::Builder,
    },
    BuiltinOwnership {
        operation: SyntaxOp::Anchor,
        spelling: "syntax_anchor",
        hidden_information: "a node's position in the region's own reading order, which is the only name the compiler \
                             and a later package function both have for it",
        family: PhaseFamily::Builder,
    },
    BuiltinOwnership {
        operation: SyntaxOp::Number,
        spelling: "syntax_number",
        hidden_information: "the reader's own numeric reading of a literal token, which a transformer has no operation \
                             to derive from that token's text",
        family: PhaseFamily::Builder,
    },
    BuiltinOwnership {
        operation: SyntaxOp::Built,
        spelling: "syntax_built",
        hidden_information: "path derivation, which keeps output paths disjoint from input paths by construction",
        family: PhaseFamily::Builder,
    },
    BuiltinOwnership {
        operation: SyntaxOp::Binding,
        spelling: "syntax_binding",
        hidden_information: "name identity as a derived coordinate rather than an allocated fresh id",
        family: PhaseFamily::Builder,
    },
    BuiltinOwnership {
        operation: SyntaxOp::Token,
        spelling: "syntax_token",
        hidden_information: "generated source information, which an adapter can carry but not forge",
        family: PhaseFamily::Builder,
    },
    BuiltinOwnership {
        operation: SyntaxOp::Identifier,
        spelling: "syntax_identifier",
        hidden_information: "generated source information, and the absence of a hygiene scope",
        family: PhaseFamily::Builder,
    },
    BuiltinOwnership {
        operation: SyntaxOp::Group,
        spelling: "syntax_group",
        hidden_information: "generated source information and the fixed grouper's delimiter set",
        family: PhaseFamily::Builder,
    },
    BuiltinOwnership {
        operation: SyntaxOp::Binder,
        spelling: "syntax_binder",
        hidden_information: "the opaque hygiene scope a binding carries, which no operation constructs",
        family: PhaseFamily::Builder,
    },
    BuiltinOwnership {
        operation: SyntaxOp::Reference,
        spelling: "syntax_reference",
        hidden_information: "the opaque hygiene scope a binding carries, which no operation constructs",
        family: PhaseFamily::Builder,
    },
    BuiltinOwnership {
        operation: SyntaxOp::Checked,
        spelling: "checked_expression",
        hidden_information: "output well-formedness: unique generated paths, one binder per binding, real delimiters",
        family: PhaseFamily::Builder,
    },
];

const BOOL: Shape = Shape::Base(Base::Bool);
const NAT: Shape = Shape::Base(Base::Nat);
const RATIO: Shape = Shape::Base(Base::Ratio);
const TEXT: Shape = Shape::Base(Base::Text);
/// Every time operation is registered at written time, because written time is
/// the only coordinate the source language constructs a value of
/// (`02-core-calculus.md` §5.7 fixes it as the score side's). `PhysicalTime`
/// exists in [`Coordinate`] so that the day a physical duration reaches the
/// source it arrives as a *different type* rather than as the same one with a
/// different meaning; registering operations for it before anything can make
/// one would be names nothing could call.
const DURATION: Shape = Shape::Base(Base::Duration(Coordinate::WrittenTime));
const POSITION: Shape = Shape::Base(Base::Position(Coordinate::WrittenTime));
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

const RATIO_OR_TEXT: Shape = Shape::Result(&RATIO, &TEXT);
const NAT_OR_TEXT: Shape = Shape::Result(&NAT, &TEXT);
const DURATION_OR_TEXT: Shape = Shape::Result(&DURATION, &TEXT);
const POSITION_OR_TEXT: Shape = Shape::Result(&POSITION, &TEXT);

const MAYBE_NAT: Shape = Shape::Option(&NAT);
const MAYBE_CLASS: Shape = Shape::Option(&CLASS);
const MAYBE_DEGREE: Shape = Shape::Option(&DEGREE);
const MAYBE_FRAME: Shape = Shape::Option(&FRAME);
const MAYBE_CHORD: Shape = Shape::Option(&CHORD);
const MAYBE_TRIAD: Shape = Shape::Option(&TRIAD);
const MAYBE_ROMAN: Shape = Shape::Option(&ROMAN);
const MAYBE_VOICING: Shape = Shape::Option(&VOICING);

/// A row, or the two exact reasons a sequence is not one: the order positions
/// whose pitch class already appeared, and the pitch classes it never names
/// (*Open Music Theory*, `108-basics-of-twelve-tone-theory.md`). Both, rather
/// than a choice between them, because a sequence of the wrong length can have
/// either without the other.
const ROW_FAULT: Shape = Shape::Product(&[NATS, PC12S]);
const ROW12_OR_FAULT: Shape = Shape::Result(&ROW12, &ROW_FAULT);

/// Register a first-order signature, checking it as it is written.
///
/// Every δ entry is built here, so this is the registration site, and a
/// violation is a build error rather than a call that fails once a composer
/// finds it. Four conditions hold of a registered builtin
/// (`docs/rules/language/02-core-calculus.md` §§1.1 and 5.8):
///
/// - **First-order**, and **no closure argument**: [`Shape`] has no arrow
///   constructor, so a signature that wanted a function could not be spelled.
///   An operation that needs one is an eliminator, checked by §5.6.
/// - **Data-only**: every argument and the result is storable data, asserted
///   here.
/// - **Total**: the operation answers on every input its signature admits.
///   Registration cannot see this, so it is the sampling law's D2, which reads
///   [`Shape::admits_absence`] and rejects an evaluator that declined to
///   answer where the shape promised a value.
/// - **Failing by value**: a builtin with one way to fail says so with
///   `Option`, and one with several says which with `Result`. Both injections
///   of a `Result` are values, so answering with one is still total.
const fn delta(arguments: &'static [Shape], result: Shape) -> Family {
    assert!(
        all_storable(arguments),
        "a registered builtin takes storable data; one of these arguments is not"
    );
    assert!(
        result.is_storable(),
        "a registered builtin answers with storable data; this result does not"
    );
    Family::Delta { arguments, result }
}

const BUILTIN_OWNERSHIP: [BuiltinOwnership<Builtin>; 111] = [
    BuiltinOwnership {
        operation: Builtin::NatFold,
        spelling: "nat_fold",
        hidden_information: "the evaluator's finite natural representation and structural work budget",
        family: Family::Eliminator(Eliminator::NatFold),
    },
    BuiltinOwnership {
        operation: Builtin::ListFoldFromStart,
        spelling: "list_fold_from_start",
        hidden_information: "the evaluator's finite list representation and structural work budget",
        family: Family::Eliminator(Eliminator::ListFoldFromStart),
    },
    BuiltinOwnership {
        operation: Builtin::ListFoldFromEnd,
        spelling: "list_fold_from_end",
        hidden_information: "the evaluator's finite list representation, reverse traversal, and structural work budget",
        family: Family::Eliminator(Eliminator::ListFoldFromEnd),
    },
    BuiltinOwnership {
        operation: Builtin::OptionFold,
        spelling: "option_fold",
        hidden_information: "the evaluator's hidden option representation and total case dispatch",
        family: Family::Eliminator(Eliminator::OptionFold),
    },
    BuiltinOwnership {
        operation: Builtin::Map,
        spelling: "map",
        hidden_information: "rank-1 monomorphization over the evaluator's hidden finite list representation",
        family: Family::Eliminator(Eliminator::Map),
    },
    BuiltinOwnership {
        operation: Builtin::Filter,
        spelling: "filter",
        hidden_information: "rank-1 monomorphization over the evaluator's hidden finite list representation",
        family: Family::Eliminator(Eliminator::Filter),
    },
    BuiltinOwnership {
        operation: Builtin::Range,
        spelling: "range",
        hidden_information: "bounded construction governed by the evaluator's structural work budget",
        family: Family::Eliminator(Eliminator::Range),
    },
    BuiltinOwnership {
        operation: Builtin::Repeat,
        spelling: "repeat",
        hidden_information: "rank-1 finite-list construction governed by the structural work budget",
        family: Family::Eliminator(Eliminator::Repeat),
    },
    BuiltinOwnership {
        operation: Builtin::RatioAdd,
        spelling: "ratio_add",
        hidden_information: "exact rational reduction, and whether the reduced result is representable at all",
        family: delta(&[RATIO, RATIO], RATIO_OR_TEXT),
    },
    BuiltinOwnership {
        operation: Builtin::RatioSub,
        spelling: "ratio_sub",
        hidden_information: "exact rational reduction, and whether the reduced result is representable at all",
        family: delta(&[RATIO, RATIO], RATIO_OR_TEXT),
    },
    BuiltinOwnership {
        operation: Builtin::RatioMul,
        spelling: "ratio_mul",
        hidden_information: "exact rational reduction, and whether the reduced result is representable at all",
        family: delta(&[RATIO, RATIO], RATIO_OR_TEXT),
    },
    BuiltinOwnership {
        operation: Builtin::RatioDiv,
        spelling: "ratio_div",
        hidden_information: "exact rational reduction, and whether the reduced result is representable at all",
        family: delta(&[RATIO, RATIO], RATIO_OR_TEXT),
    },
    BuiltinOwnership {
        operation: Builtin::RatioLess,
        spelling: "ratio_less",
        hidden_information: "the reduced form two exact rationals share, which no source expression can inspect",
        family: delta(&[RATIO, RATIO], BOOL),
    },
    BuiltinOwnership {
        operation: Builtin::RatioEqual,
        spelling: "ratio_equal",
        hidden_information: "the reduced form two exact rationals share, which no source expression can inspect",
        family: delta(&[RATIO, RATIO], BOOL),
    },
    BuiltinOwnership {
        operation: Builtin::TextEqual,
        spelling: "text_equal",
        hidden_information: "the encoding two texts are compared in, which no source expression can inspect",
        family: delta(&[TEXT, TEXT], BOOL),
    },
    BuiltinOwnership {
        operation: Builtin::NatAdd,
        spelling: "nat_add",
        hidden_information: "the representable range a whole number must stay inside",
        family: delta(&[NAT, NAT], NAT_OR_TEXT),
    },
    BuiltinOwnership {
        operation: Builtin::NatMul,
        spelling: "nat_mul",
        hidden_information: "the representable range a whole number must stay inside",
        family: delta(&[NAT, NAT], NAT_OR_TEXT),
    },
    BuiltinOwnership {
        operation: Builtin::NatSub,
        spelling: "nat_sub",
        hidden_information: "the representable range a whole number must stay inside",
        family: delta(&[NAT, NAT], MAYBE_NAT),
    },
    BuiltinOwnership {
        operation: Builtin::DurationOf,
        spelling: "duration_of",
        hidden_information: "the nonnegativity every duration constructor checks",
        family: delta(&[RATIO], DURATION_OR_TEXT),
    },
    BuiltinOwnership {
        operation: Builtin::DurationRatio,
        spelling: "duration_ratio",
        hidden_information: "the exact rational a duration is measured by, and its coordinate tag",
        family: delta(&[DURATION], RATIO),
    },
    BuiltinOwnership {
        operation: Builtin::DurationAdd,
        spelling: "duration_add",
        hidden_information: "exact rational reduction, and whether the reduced result is representable at all",
        family: delta(&[DURATION, DURATION], DURATION_OR_TEXT),
    },
    BuiltinOwnership {
        operation: Builtin::DurationScale,
        spelling: "duration_scale",
        hidden_information: "the nonnegativity every duration constructor checks",
        family: delta(&[DURATION, RATIO], DURATION_OR_TEXT),
    },
    BuiltinOwnership {
        operation: Builtin::DurationLess,
        spelling: "duration_less",
        hidden_information: "the reduced form two exact rationals share, which no source expression can inspect",
        family: delta(&[DURATION, DURATION], BOOL),
    },
    BuiltinOwnership {
        operation: Builtin::DurationEqual,
        spelling: "duration_equal",
        hidden_information: "the reduced form two exact rationals share, which no source expression can inspect",
        family: delta(&[DURATION, DURATION], BOOL),
    },
    BuiltinOwnership {
        operation: Builtin::PositionOf,
        spelling: "position_of",
        hidden_information: "the origin an instant is measured from, and its coordinate tag",
        family: delta(&[RATIO], POSITION),
    },
    BuiltinOwnership {
        operation: Builtin::PositionRatio,
        spelling: "position_ratio",
        hidden_information: "the origin an instant is measured from, and its coordinate tag",
        family: delta(&[POSITION], RATIO),
    },
    BuiltinOwnership {
        operation: Builtin::PositionShift,
        spelling: "position_shift",
        hidden_information: "exact rational reduction, and whether the reduced result is representable at all",
        family: delta(&[POSITION, DURATION], POSITION_OR_TEXT),
    },
    BuiltinOwnership {
        operation: Builtin::PositionBetween,
        spelling: "position_between",
        hidden_information: "the nonnegativity every duration constructor checks",
        family: delta(&[POSITION, POSITION], DURATION_OR_TEXT),
    },
    BuiltinOwnership {
        operation: Builtin::PositionLess,
        spelling: "position_less",
        hidden_information: "the reduced form two exact rationals share, which no source expression can inspect",
        family: delta(&[POSITION, POSITION], BOOL),
    },
    BuiltinOwnership {
        operation: Builtin::PositionEqual,
        spelling: "position_equal",
        hidden_information: "the reduced form two exact rationals share, which no source expression can inspect",
        family: delta(&[POSITION, POSITION], BOOL),
    },
    BuiltinOwnership {
        operation: Builtin::IntervalAdd,
        spelling: "interval_add",
        hidden_information: "the evaluator's exact written-interval coordinate representation",
        family: delta(&[INTERVAL, INTERVAL], INTERVAL),
    },
    BuiltinOwnership {
        operation: Builtin::IntervalInverse,
        spelling: "interval_inverse",
        hidden_information: "the evaluator's exact written-interval coordinate representation",
        family: delta(&[INTERVAL], INTERVAL),
    },
    BuiltinOwnership {
        operation: Builtin::PitchClassOf,
        spelling: "pitchclass_of",
        hidden_information: "the written pitch's octave coordinate and spelling-preserving quotient",
        family: delta(&[PITCH], CLASS),
    },
    BuiltinOwnership {
        operation: Builtin::SignatureScale,
        spelling: "signature_scale",
        hidden_information: "the compiler's table of named collections, which no source text can enumerate",
        family: delta(&[KEY], SCALE),
    },
    BuiltinOwnership {
        operation: Builtin::ScaleOn,
        spelling: "scale_on",
        hidden_information: "the scale's private ordered offset cycle, re-rooted without being exposed",
        family: delta(&[SCALE, CLASS], SCALE),
    },
    BuiltinOwnership {
        operation: Builtin::ScaleTonic,
        spelling: "scale_tonic",
        hidden_information: "the scale's private tonic field",
        family: delta(&[SCALE], CLASS),
    },
    BuiltinOwnership {
        operation: Builtin::ScaleSize,
        spelling: "scale_size",
        hidden_information: "the length of the scale's private offset cycle",
        family: delta(&[SCALE], NAT),
    },
    BuiltinOwnership {
        operation: Builtin::ScalePitch,
        spelling: "scale_pitch",
        hidden_information: "spelled membership against the scale's private offset cycle",
        family: delta(&[SCALE, PITCH], MAYBE_DEGREE),
    },
    BuiltinOwnership {
        operation: Builtin::ScaleClass,
        spelling: "scale_class",
        hidden_information: "the scale's private offset cycle, read without a register",
        family: delta(&[SCALE, DEGREE], MAYBE_CLASS),
    },
    BuiltinOwnership {
        operation: Builtin::ScaleChord,
        spelling: "scale_chord",
        hidden_information: "the scale's offset cycle and the chord vocabulary's member table at once",
        family: delta(&[SCALE, DEGREE, NAT], MAYBE_CHORD),
    },
    BuiltinOwnership {
        operation: Builtin::PitchFrame,
        spelling: "pitch_frame",
        hidden_information: "the register frame's representation invariant, which only the compiler can enforce",
        family: delta(&[SCALE, PITCH], MAYBE_FRAME),
    },
    BuiltinOwnership {
        operation: Builtin::FrameScale,
        spelling: "frame_scale",
        hidden_information: "the frame's private scale field",
        family: delta(&[FRAME], SCALE),
    },
    BuiltinOwnership {
        operation: Builtin::FrameTonic,
        spelling: "frame_tonic",
        hidden_information: "the frame's private registered tonic field",
        family: delta(&[FRAME], PITCH),
    },
    BuiltinOwnership {
        operation: Builtin::FramePitch,
        spelling: "frame_pitch",
        hidden_information: "Euclidean division of a degree through the scale's private period",
        family: delta(&[FRAME, DEGREE], PITCH),
    },
    BuiltinOwnership {
        operation: Builtin::DegreeOf,
        spelling: "degree_of",
        hidden_information: "the degree's private signed coordinate, which is not the written ordinal",
        family: delta(&[NAT], DEGREE),
    },
    BuiltinOwnership {
        operation: Builtin::DegreeStepUp,
        spelling: "degree_step_up",
        hidden_information: "the degree's private signed coordinate and its machine-integer bound",
        family: delta(&[DEGREE, NAT], DEGREE),
    },
    BuiltinOwnership {
        operation: Builtin::DegreeStepDown,
        spelling: "degree_step_down",
        hidden_information: "the degree's private signed coordinate and its machine-integer bound",
        family: delta(&[DEGREE, NAT], DEGREE),
    },
    BuiltinOwnership {
        operation: Builtin::DegreeRaised,
        spelling: "degree_raised",
        hidden_information: "the degree's private chromatic alteration and its machine-integer bound",
        family: delta(&[DEGREE], DEGREE),
    },
    BuiltinOwnership {
        operation: Builtin::DegreeLowered,
        spelling: "degree_lowered",
        hidden_information: "the degree's private chromatic alteration and its machine-integer bound",
        family: delta(&[DEGREE], DEGREE),
    },
    BuiltinOwnership {
        operation: Builtin::ChordOn,
        spelling: "chord_on",
        hidden_information: "the compiler's table of chord types, re-rooted without being exposed",
        family: delta(&[CHORD, CLASS], CHORD),
    },
    BuiltinOwnership {
        operation: Builtin::ChordRoot,
        spelling: "chord_root",
        hidden_information: "the chord class's private root field",
        family: delta(&[CHORD], CLASS),
    },
    BuiltinOwnership {
        operation: Builtin::ChordBass,
        spelling: "chord_bass",
        hidden_information: "the chord class's private bass designation, which is absent and not the root",
        family: delta(&[CHORD], MAYBE_CLASS),
    },
    BuiltinOwnership {
        operation: Builtin::ChordMembers,
        spelling: "chord_members",
        hidden_information: "the private spelled member stack, which no source text can enumerate",
        family: delta(&[CHORD], INTERVALS),
    },
    BuiltinOwnership {
        operation: Builtin::ChordInversion,
        spelling: "chord_inversion",
        hidden_information: "membership of the private member stack, which is what makes an inversion true",
        family: delta(&[CHORD, NAT], MAYBE_CHORD),
    },
    BuiltinOwnership {
        operation: Builtin::ChordOver,
        spelling: "chord_over",
        hidden_information: "the chord class's private bass designation",
        family: delta(&[CHORD, CLASS], CHORD),
    },
    BuiltinOwnership {
        operation: Builtin::ChordTriad,
        spelling: "chord_triad",
        hidden_information: "the triad refinement's representation invariant, which only the compiler can enforce",
        family: delta(&[CHORD], MAYBE_TRIAD),
    },
    BuiltinOwnership {
        operation: Builtin::TriadChord,
        spelling: "triad_chord",
        hidden_information: "the triad refinement's private witness",
        family: delta(&[TRIAD], CHORD),
    },
    BuiltinOwnership {
        operation: Builtin::RomanOf,
        spelling: "roman_of",
        hidden_information: "the numeral's representation invariant, which only the compiler can enforce",
        family: delta(&[NAT, NAT, NAT], MAYBE_ROMAN),
    },
    BuiltinOwnership {
        operation: Builtin::RomanOrdinal,
        spelling: "roman_ordinal",
        hidden_information: "the numeral's private ordinal",
        family: delta(&[ROMAN], NAT),
    },
    BuiltinOwnership {
        operation: Builtin::RomanSize,
        spelling: "roman_size",
        hidden_information: "the numeral's private member count",
        family: delta(&[ROMAN], NAT),
    },
    BuiltinOwnership {
        operation: Builtin::RomanInversion,
        spelling: "roman_inversion",
        hidden_information: "the numeral's private bass designation, which is a position and not a pitch",
        family: delta(&[ROMAN], NAT),
    },
    BuiltinOwnership {
        operation: Builtin::TriadMajor,
        spelling: "triad_major",
        hidden_information: "the chord class's private type, which is the only place the two triads differ",
        family: delta(&[TRIAD], BOOL),
    },
    BuiltinOwnership {
        operation: Builtin::VoicingOf,
        spelling: "voicing_of",
        hidden_information: "the voicing's representation invariant: ascending distinct pitches drawn from the class",
        family: delta(&[CHORD, PITCHES], MAYBE_VOICING),
    },
    BuiltinOwnership {
        operation: Builtin::VoicingPitches,
        spelling: "voicing_pitches",
        hidden_information: "the voicing's private ordered pitch sequence",
        family: delta(&[VOICING], PITCHES),
    },
    BuiltinOwnership {
        operation: Builtin::VoicingBass,
        spelling: "voicing_bass",
        hidden_information: "the voicing's private lowest pitch, held apart from the rest",
        family: delta(&[VOICING], PITCH),
    },
    BuiltinOwnership {
        operation: Builtin::VoicingChord,
        spelling: "voicing_chord",
        hidden_information: "the voicing's private association to the class it voices",
        family: delta(&[VOICING], CHORD),
    },
    BuiltinOwnership {
        operation: Builtin::VoicingPosition,
        spelling: "voicing_position",
        hidden_information: "membership of the private member stack, which is what classifies an inversion",
        family: delta(&[VOICING], MAYBE_NAT),
    },
    BuiltinOwnership {
        operation: Builtin::CloseVoicing,
        spelling: "close_voicing",
        hidden_information: "the private member stack walked upward, and the voicing invariant it must satisfy",
        family: delta(&[CHORD, PITCH], MAYBE_VOICING),
    },
    BuiltinOwnership {
        operation: Builtin::DropVoicing,
        spelling: "drop_voicing",
        hidden_information: "the private member stack walked upward, and the voicing invariant it must satisfy",
        family: delta(&[CHORD, PITCH, NAT], MAYBE_VOICING),
    },
    BuiltinOwnership {
        operation: Builtin::OmitVoicing,
        spelling: "omit_voicing",
        hidden_information: "the private member stack, which is what says which pitch an omission removes",
        family: delta(&[VOICING, NAT], MAYBE_VOICING),
    },
    BuiltinOwnership {
        operation: Builtin::Pc12Of,
        spelling: "pc12_of",
        hidden_information: "the canonical representative of a residue class modulo twelve",
        family: delta(&[NAT], PC12),
    },
    BuiltinOwnership {
        operation: Builtin::Pc12Number,
        spelling: "pc12_number",
        hidden_information: "the canonical representative, which is the only number a residue class has",
        family: delta(&[PC12], NAT),
    },
    BuiltinOwnership {
        operation: Builtin::Pc12Forget,
        spelling: "pc12_forget",
        hidden_information: "the chromatic coordinate of a spelled pitch class, taken modulo twelve",
        family: delta(&[CLASS], PC12),
    },
    BuiltinOwnership {
        operation: Builtin::Pc12Transposed,
        spelling: "pc12_transposed",
        hidden_information: "modular addition, which a `nat` without subtraction cannot express",
        family: delta(&[PC12, NAT], PC12),
    },
    BuiltinOwnership {
        operation: Builtin::Pc12Inverted,
        spelling: "pc12_inverted",
        hidden_information: "modular subtraction, which a `nat` without subtraction cannot express",
        family: delta(&[PC12, NAT], PC12),
    },
    BuiltinOwnership {
        operation: Builtin::Pc12Spelled,
        spelling: "pc12_spelled",
        hidden_information: "the collection's spelled members, searched for the one this class forgets to",
        family: delta(&[PC12, SCALE], MAYBE_CLASS),
    },
    BuiltinOwnership {
        operation: Builtin::PcSet12Of,
        spelling: "pcset12_of",
        hidden_information: "the twelve-bit membership word that makes duplication unrepresentable",
        family: delta(&[PC12S], PCSET12),
    },
    BuiltinOwnership {
        operation: Builtin::PcSet12Members,
        spelling: "pcset12_members",
        hidden_information: "the membership word, read out ascending",
        family: delta(&[PCSET12], PC12S),
    },
    BuiltinOwnership {
        operation: Builtin::PcSet12Transposed,
        spelling: "pcset12_transposed",
        hidden_information: "the membership word, rotated by the index without unpacking it",
        family: delta(&[PCSET12, NAT], PCSET12),
    },
    BuiltinOwnership {
        operation: Builtin::PcSet12Inverted,
        spelling: "pcset12_inverted",
        hidden_information: "the membership word, reflected about the index without unpacking it",
        family: delta(&[PCSET12, NAT], PCSET12),
    },
    BuiltinOwnership {
        operation: Builtin::PcSet12Normal,
        spelling: "pcset12_normal",
        hidden_information: "every rotation of the set and the compactness order that chooses between them",
        family: delta(&[PCSET12], PC12S),
    },
    BuiltinOwnership {
        operation: Builtin::PcSet12Prime,
        spelling: "pcset12_prime",
        hidden_information: "the normal orders of the set and its inversion, and which of the two reads lower",
        family: delta(&[PCSET12], PCSET12),
    },
    BuiltinOwnership {
        operation: Builtin::PcSet12Vector,
        spelling: "pcset12_vector",
        hidden_information: "every unordered pair of members and the interval class each realizes",
        family: delta(&[PCSET12], NATS),
    },
    BuiltinOwnership {
        operation: Builtin::Row12Of,
        spelling: "row12_of",
        hidden_information: "the permutation invariant: twelve order positions and each pitch class once",
        family: delta(&[PC12S], ROW12_OR_FAULT),
    },
    BuiltinOwnership {
        operation: Builtin::Row12Pcs,
        spelling: "row12_pcs",
        hidden_information: "the private order-position array",
        family: delta(&[ROW12], PC12S),
    },
    BuiltinOwnership {
        operation: Builtin::Row12Head,
        spelling: "row12_head",
        hidden_information: "order position zero of the private array, which the finite list eliminators cannot index",
        family: delta(&[ROW12], PC12),
    },
    BuiltinOwnership {
        operation: Builtin::Row12Transposed,
        spelling: "row12_transposed",
        hidden_information: "modular addition, and the finite-closure lemma that keeps the result a row",
        family: delta(&[ROW12, NAT], ROW12),
    },
    BuiltinOwnership {
        operation: Builtin::Row12Inverted,
        spelling: "row12_inverted",
        hidden_information: "modular subtraction, and the finite-closure lemma that keeps the result a row",
        family: delta(&[ROW12, NAT], ROW12),
    },
    BuiltinOwnership {
        operation: Builtin::Row12Retrograde,
        spelling: "row12_retrograde",
        hidden_information: "reversal of the order positions, which the finite list eliminators cannot express",
        family: delta(&[ROW12], ROW12),
    },
    BuiltinOwnership {
        operation: Builtin::Row12Matrix,
        spelling: "row12_matrix",
        hidden_information: "the classical construction: the inversion about the row's own head, read as starting pitches",
        family: delta(&[ROW12], ROW12S),
    },
    BuiltinOwnership {
        operation: Builtin::Row12Forms,
        spelling: "row12_forms",
        hidden_information: "the forty-eight labelled forms, compared for equality and counted once each",
        family: delta(&[ROW12], NAT),
    },
    BuiltinOwnership {
        operation: Builtin::Row12Symmetries,
        spelling: "row12_symmetries",
        hidden_information: "the forty-eight labelled forms, counted where they fix the row",
        family: delta(&[ROW12], NAT),
    },
    BuiltinOwnership {
        operation: Builtin::Row12Repeats,
        spelling: "row12_repeats",
        hidden_information: "pitch-class equality, which the surface has no operator for",
        family: delta(&[PC12S], NATS),
    },
    BuiltinOwnership {
        operation: Builtin::Row12Missing,
        spelling: "row12_missing",
        hidden_information: "pitch-class equality against the whole finite domain",
        family: delta(&[PC12S], PC12S),
    },
    BuiltinOwnership {
        operation: Builtin::Transpose,
        spelling: "transpose",
        hidden_information: "contextual music representation, written-pitch provenance, and kernel construction",
        family: Family::Track,
    },
    BuiltinOwnership {
        operation: Builtin::Stretch,
        spelling: "stretch",
        hidden_information: "contextual music representation, exact-time provenance, and kernel construction",
        family: Family::Track,
    },
    BuiltinOwnership {
        operation: Builtin::Retrograde,
        spelling: "retrograde",
        hidden_information: "contextual music extent, occurrence provenance, and kernel construction",
        family: Family::Track,
    },
    BuiltinOwnership {
        operation: Builtin::Invert,
        spelling: "invert",
        hidden_information: "contextual music representation, written-pitch provenance, and kernel construction",
        family: Family::Track,
    },
    BuiltinOwnership {
        operation: Builtin::Shift,
        spelling: "shift",
        hidden_information: "contextual music representation, exact-time provenance, and kernel construction",
        family: Family::Track,
    },
    BuiltinOwnership {
        operation: Builtin::Together,
        spelling: "together",
        hidden_information: "contextual music representation, origin paths, and kernel stacking construction",
        family: Family::Track,
    },
    BuiltinOwnership {
        operation: Builtin::MapNotePitches,
        spelling: "map_note_pitches",
        hidden_information: "controlled traversal of contextual notes while preserving non-note facts and provenance",
        family: Family::Track,
    },
    BuiltinOwnership {
        operation: Builtin::Play,
        spelling: "play",
        hidden_information: "contextual music construction: the voicing's private pitches become sounded occurrences with provenance",
        family: Family::Track,
    },
    BuiltinOwnership {
        operation: Builtin::Primitive,
        spelling: "primitive",
        hidden_information: "the build-local registry: which unit a name and version select, and that unit's private state layout, start, and step",
        family: Family::Machine(MachineOp::Primitive),
    },
    BuiltinOwnership {
        operation: Builtin::Machine,
        spelling: "machine",
        hidden_information: "the exact configuration encoding a registered unit is instantiated with",
        family: Family::Machine(MachineOp::Machine),
    },
    BuiltinOwnership {
        operation: Builtin::Identity,
        spelling: "identity",
        hidden_information: "the machine description's node representation",
        family: Family::Machine(MachineOp::Identity),
    },
    BuiltinOwnership {
        operation: Builtin::Connect,
        spelling: "connect",
        hidden_information: "the machine description's node representation and the order its children are stored in",
        family: Family::Machine(MachineOp::Connect),
    },
    BuiltinOwnership {
        operation: Builtin::Beside,
        spelling: "beside",
        hidden_information: "the machine description's node representation and the order its children are stored in",
        family: Family::Machine(MachineOp::Beside),
    },
    BuiltinOwnership {
        operation: Builtin::Feedback,
        spelling: "feedback",
        hidden_information: "the exact encoding of the stored value the first step reads",
        family: Family::Machine(MachineOp::Feedback),
    },
    BuiltinOwnership {
        operation: Builtin::Copy,
        spelling: "copy",
        hidden_information: "the machine description's node representation",
        family: Family::Machine(MachineOp::Copy),
    },
    BuiltinOwnership {
        operation: Builtin::Drop,
        spelling: "drop",
        hidden_information: "the machine description's node representation",
        family: Family::Machine(MachineOp::Drop),
    },
    BuiltinOwnership {
        operation: Builtin::Swap,
        spelling: "swap",
        hidden_information: "the machine description's node representation",
        family: Family::Machine(MachineOp::Swap),
    },
];

impl Builtin {
    fn name(self) -> &'static str {
        match self {
            Self::NatFold => "nat_fold",
            Self::ListFoldFromStart => "list_fold_from_start",
            Self::ListFoldFromEnd => "list_fold_from_end",
            Self::OptionFold => "option_fold",
            Self::Map => "map",
            Self::Filter => "filter",
            Self::Range => "range",
            Self::Repeat => "repeat",
            Self::RatioAdd => "ratio_add",
            Self::RatioSub => "ratio_sub",
            Self::RatioMul => "ratio_mul",
            Self::RatioDiv => "ratio_div",
            Self::RatioLess => "ratio_less",
            Self::RatioEqual => "ratio_equal",
            Self::TextEqual => "text_equal",
            Self::NatAdd => "nat_add",
            Self::NatMul => "nat_mul",
            Self::NatSub => "nat_sub",
            Self::DurationOf => "duration_of",
            Self::DurationRatio => "duration_ratio",
            Self::DurationAdd => "duration_add",
            Self::DurationScale => "duration_scale",
            Self::DurationLess => "duration_less",
            Self::DurationEqual => "duration_equal",
            Self::PositionOf => "position_of",
            Self::PositionRatio => "position_ratio",
            Self::PositionShift => "position_shift",
            Self::PositionBetween => "position_between",
            Self::PositionLess => "position_less",
            Self::PositionEqual => "position_equal",
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
            Self::PcSet12Transposed => "pcset12_transposed",
            Self::PcSet12Inverted => "pcset12_inverted",
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
            Self::Transpose => "transpose",
            Self::Stretch => "stretch",
            Self::Retrograde => "retrograde",
            Self::Invert => "invert",
            Self::Shift => "shift",
            Self::Together => "together",
            Self::MapNotePitches => "map_note_pitches",
            Self::Play => "play",
            Self::Primitive => "primitive",
            Self::Machine => "machine",
            Self::Identity => "identity",
            Self::Connect => "connect",
            Self::Beside => "beside",
            Self::Feedback => "feedback",
            Self::Copy => "copy",
            Self::Drop => "drop",
            Self::Swap => "swap",
            Self::Syntax(operation) => operation.spelling(),
        }
    }

    /// The builtin this name spells, if any.
    ///
    /// One registry, one lookup: a name is a compiler-owned operation exactly when
    /// [`BUILTIN_OWNERSHIP`] holds it, and what *kind* of operation it is the entry's family
    /// says. Two tables were two answers to one question.
    fn named(name: &str) -> Option<Self> {
        let entry = BUILTIN_OWNERSHIP.iter().find(|entry| entry.spelling == name)?;
        debug_assert!(!entry.hidden_information.is_empty());
        Some(entry.operation)
    }

    /// Which of §5.8's families this operation belongs to, and for a δ-builtin its declared
    /// signature.
    ///
    /// This is the single statement of a δ-builtin's type: the checker reads argument and result
    /// types from here rather than restating them, so the two cannot disagree.
    /// Absent only if the registry has lost an entry, which the registry's own laws forbid.
    fn family(self) -> Option<Family> {
        BUILTIN_OWNERSHIP
            .iter()
            .find(|entry| entry.operation == self)
            .map(|entry| entry.family)
    }

    /// Whether this is a track builtin — §5.8's track family, and the only one whose members
    /// are ordinary values.
    ///
    /// A track builtin has an arrow type and is applied like anything else with one; a
    /// δ-builtin or an eliminator is checked at its call site against the signature the
    /// registry declares, and naming one without applying it is an error. That is the whole
    /// difference between the two routes, so it is asked here rather than spelled out at each
    /// of them.
    fn is_track(self) -> bool {
        matches!(self.family(), Some(Family::Track))
    }

    /// A track builtin's parameters, or `None` where the operation is not one.
    ///
    /// The other seventy-one state their types in the registry instead, where the checker
    /// reads them; they never become a [`Value`], so there is no arrow to give them here.
    fn parameters(self) -> Option<Vec<Type>> {
        match self {
            Self::Transpose => Some(vec![Type::Interval, Type::Music]),
            Self::Stretch => Some(vec![Type::Ratio, Type::Music]),
            Self::Retrograde => Some(vec![Type::Music]),
            Self::Invert => Some(vec![Type::Pitch, Type::Music]),
            Self::Shift => Some(vec![Type::Duration(Coordinate::WrittenTime), Type::Music]),
            Self::Together => Some(vec![Type::Music, Type::Music]),
            Self::MapNotePitches => Some(vec![
                Type::Function(vec![Type::Pitch], Box::new(Type::Pitch)),
                Type::Music,
            ]),
            Self::Play => Some(vec![Type::Voicing, Type::Duration(Coordinate::WrittenTime)]),
            Self::NatFold
            | Self::ListFoldFromStart
            | Self::ListFoldFromEnd
            | Self::OptionFold
            | Self::Map
            | Self::Filter
            | Self::Range
            | Self::Repeat
            | Self::RatioAdd
            | Self::RatioSub
            | Self::RatioMul
            | Self::RatioDiv
            | Self::RatioLess
            | Self::RatioEqual
            | Self::TextEqual
            | Self::NatAdd
            | Self::NatMul
            | Self::NatSub
            | Self::DurationOf
            | Self::DurationRatio
            | Self::DurationAdd
            | Self::DurationScale
            | Self::DurationLess
            | Self::DurationEqual
            | Self::PositionOf
            | Self::PositionRatio
            | Self::PositionShift
            | Self::PositionBetween
            | Self::PositionLess
            | Self::PositionEqual
            | Self::IntervalAdd
            | Self::IntervalInverse
            | Self::PitchClassOf
            | Self::SignatureScale
            | Self::ScaleOn
            | Self::ScaleTonic
            | Self::ScaleSize
            | Self::ScalePitch
            | Self::ScaleClass
            | Self::ScaleChord
            | Self::PitchFrame
            | Self::FrameScale
            | Self::FrameTonic
            | Self::FramePitch
            | Self::DegreeOf
            | Self::DegreeStepUp
            | Self::DegreeStepDown
            | Self::DegreeRaised
            | Self::DegreeLowered
            | Self::ChordOn
            | Self::ChordRoot
            | Self::ChordBass
            | Self::ChordMembers
            | Self::ChordInversion
            | Self::ChordOver
            | Self::ChordTriad
            | Self::TriadChord
            | Self::TriadMajor
            | Self::RomanOf
            | Self::RomanOrdinal
            | Self::RomanSize
            | Self::RomanInversion
            | Self::VoicingOf
            | Self::VoicingPitches
            | Self::VoicingBass
            | Self::VoicingChord
            | Self::VoicingPosition
            | Self::CloseVoicing
            | Self::DropVoicing
            | Self::OmitVoicing
            | Self::Pc12Of
            | Self::Pc12Number
            | Self::Pc12Forget
            | Self::Pc12Transposed
            | Self::Pc12Inverted
            | Self::Pc12Spelled
            | Self::PcSet12Of
            | Self::PcSet12Members
            | Self::PcSet12Transposed
            | Self::PcSet12Inverted
            | Self::PcSet12Normal
            | Self::PcSet12Prime
            | Self::PcSet12Vector
            | Self::Row12Of
            | Self::Row12Pcs
            | Self::Row12Head
            | Self::Row12Transposed
            | Self::Row12Inverted
            | Self::Row12Retrograde
            | Self::Row12Matrix
            | Self::Row12Forms
            | Self::Row12Symmetries
            | Self::Row12Repeats
            | Self::Row12Missing
            | Self::Primitive
            | Self::Machine
            | Self::Identity
            | Self::Connect
            | Self::Beside
            | Self::Feedback
            | Self::Copy
            | Self::Drop
            | Self::Swap
            // A phase-local operation states its type in [`SyntaxOp::instantiate`]
            // for the same reason: it is checked at its call site and never
            // becomes a bare value.
            | Self::Syntax(_) => None,
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
    Ok(String),
    Err(String),
    EmptyList,
    Cons {
        head: String,
        tail: String,
    },
    Product(Vec<String>),
    /// One constructor of a library-declared type, binding its fields in the
    /// order the declaration wrote them.
    Constructor {
        variant: usize,
        fields: Vec<String>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum Coverage {
    CatchAll,
    True,
    False,
    None,
    Some,
    Ok,
    Err,
    EmptyList,
    Cons,
    Literal(String),
    /// One constructor of a library-declared type, by its index in the
    /// declaration — the name would be ambiguous across declarations, and the
    /// index is what the exhaustiveness check counts.
    Constructor(usize),
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
}

#[derive(Clone)]
enum Value {
    Bool(bool),
    Nat(u64),
    Ratio(Ratio<i64>),
    Text(String),
    /// A nonnegative exact rational in its coordinate. The coordinate travels
    /// with the value because the evaluator has to hand back a value of the
    /// declared result type, and `duration_add` is only well typed when both
    /// operands agree.
    Duration(Coordinate, Ratio<i64>),
    /// An exact rational instant in its coordinate. Signed, unlike a duration:
    /// nothing forbids a position before the origin, and the nonnegativity
    /// lives on the difference instead.
    Position(Coordinate, Ratio<i64>),
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
    /// One injection into a binary sum, carrying both halves of its type.
    ///
    /// Both are kept for the same reason `Option` keeps its member: the
    /// value is one side, and the type is both, so a value that dropped the
    /// other half could not say what it is.
    Sum {
        value_type: Type,
        error_type: Type,
        error: bool,
        held: Box<Self>,
    },
    Option {
        member: Type,
        value: Option<Box<Self>>,
    },
    List {
        member: Type,
        values: Vec<Self>,
    },
    /// One value of a library-declared type: which declaration, which
    /// constructor, what it was made at, and what it holds.
    ///
    /// The arguments are here for the same reason a list keeps its member
    /// type: a value that dropped them could not say what it is, and
    /// `Leaf(3)` of `Tree<Nat>` and `Leaf(3)` of `Tree<Duration>` are values
    /// of two types.
    Data {
        id: crate::data::NominalId,
        arguments: Vec<Type>,
        variant: usize,
        fields: Vec<Self>,
    },
    Music(Music),
    /// One instance of a registered stepping unit — §2's `p`.
    ///
    /// The configuration is kept as the value it was written as rather than as
    /// bytes, so that a diagnostic can print it; the bytes are taken once, at
    /// projection, where a machine's exact identity is settled.
    Primitive {
        descriptor: &'static crate::machine::PrimitiveDescriptor,
        configuration: Box<Self>,
    },
    /// A finite machine *description* — never the history it produces
    /// (`docs/rules/constitution.md` §4).
    ///
    /// The type is carried for the reason a list carries its member type: a
    /// machine that dropped it could not say what it is, and the projection
    /// needs the step and the two ports that only the type holds.
    Machine {
        ty: Type,
        tree: Box<MachineTree>,
    },
    /// A finite syntax value. Phase-local: no ordinary source expression can
    /// produce one, because no operation that returns one is in scope there.
    Syntax(Box<crate::syntax::Syntax>),
    /// Where one node sits, as a path from an expansion's root.
    NodePath(Box<crate::syntax::NodePath>),
    /// Which name a binder declares and a reference means.
    BindingPath(Box<crate::syntax::BindingPath>),
    /// One suspended recursive call, sealed (see [`SealedStep`]).
    SyntaxStep(Box<SealedStep>),
    Closure(Box<Closure>),
    Builtin(Builtin),
}

/// A suspended recursive call into one immediate proper child.
///
/// The three fields travel together and are never separable, which is the
/// whole of `docs/rules/language/02-core-calculus.md` §5.9's association
/// lemma: minting is the only way one comes into existence, running is the
/// only way one is consumed, and nothing between the two can replace the
/// algebra or the child. A nested recursor handed this value therefore runs
/// *this* child under *this* algebra, and needs no ownership check to be
/// stopped from doing anything else — there is no operation that would let it
/// try.
#[derive(Clone)]
struct SealedStep {
    /// The four branches of the recursor that minted it, in the order
    /// [`SyntaxOp::Recurse`] takes them.
    algebra: Vec<Value>,
    /// The one immediate proper child this descends to. Strictly smaller than
    /// the group that minted it, which is the local decrease §5.9 states.
    child: Box<crate::syntax::Syntax>,
    /// `SyntaxStep<C, A>`, kept because a group inside `child` has to build
    /// the list of steps it hands its own branch, and a list carries its
    /// member type.
    ty: Type,
}

/// A machine description, as §2's forms build one.
///
/// A tree rather than the flat array [`crate::MachineSpec`] publishes, because
/// this is what evaluation produces and evaluation is compositional: `connect`
/// holds the two machines it was applied to. Flattening happens once, at the
/// projection, where the order a consumer wants — children before parents — is
/// what matters.
///
/// It holds no source closure, no environment, and no state. What a step *does*
/// is the registered unit's, and prompt 127f is where that arrives.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum MachineTree {
    Primitive {
        descriptor: &'static crate::machine::PrimitiveDescriptor,
        configuration: Vec<u8>,
    },
    Identity,
    Connect(Box<Self>, Box<Self>),
    Beside(Box<Self>, Box<Self>),
    /// The stored value the first step reads, and the machine it is fed back
    /// through. There is no uninitialized form: `feedback` takes the initial
    /// value as an argument, so a loop with no delay cannot be written.
    Feedback {
        initial: Vec<u8>,
        inner: Box<Self>,
    },
    Copy,
    Drop,
    Swap,
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
    Together {
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
        term: musa_kernel::Term<musa_kernel::WrittenTime, crate::elaborate::ScoreFact>,
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
        vec![Value::Pitch(pitch)],
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
    /// The settled type of each declaration, by name.
    ///
    /// Kept because the expansion phase asks a module whether the operation it
    /// declares is the operation the phase runs — `expand` is a transformer or
    /// it is not — and that is a question about a type. Nothing else reads it:
    /// a value leaves this compiler as a value.
    types: IndexMap<String, Type>,
}

impl Program {
    /// Every machine this program names, with its exact projection, in the
    /// order the source declares them.
    ///
    /// This is the whole of how a machine leaves the compiler. The evaluator's
    /// value stays private: a consumer that could see it could also see the
    /// source types, environments, and provenance that built it, none of which
    /// is part of what a machine means.
    ///
    /// A machine whose type is still open — `identity` names one at every step
    /// and every port — is skipped rather than guessed at. It is a perfectly
    /// good polymorphic value and simply not yet *a* machine: the step and the
    /// two ports are what a projection is for, and a consumer cannot prepare a
    /// port whose type has not been decided.
    pub(crate) fn machines(&self) -> Vec<(String, crate::MachineSpec)> {
        fn decided(ty: &Type) -> bool {
            !matches!(ty, Type::Var(_)) && crate::infer::member_types(ty).into_iter().all(decided)
        }
        self.values
            .iter()
            .filter_map(|(name, value)| {
                let Value::Machine { ty, tree } = value else {
                    return None;
                };
                let Type::Machine { step, input, output } = ty else {
                    return None;
                };
                let Type::Step(tag) = **step else {
                    return None;
                };
                if !decided(input) || !decided(output) {
                    return None;
                }
                let mut nodes = Vec::new();
                tree.flatten(&mut nodes);
                Some((
                    name.clone(),
                    crate::MachineSpec::new(tag, input.to_string(), output.to_string(), nodes),
                ))
            })
            .collect()
    }

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
            Self::Text(_) => Type::Text,
            Self::Duration(coordinate, _) => Type::Duration(*coordinate),
            Self::Position(coordinate, _) => Type::Position(*coordinate),
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
            Self::Sum {
                value_type, error_type, ..
            } => Type::Sum(Box::new(value_type.clone()), Box::new(error_type.clone())),
            Self::Option { member, .. } => Type::Option(Box::new(member.clone())),
            Self::List { member, .. } => Type::List(Box::new(member.clone())),
            Self::Data { id, arguments, .. } => Type::Nominal(id.clone(), arguments.clone()),
            Self::Music(_) => Type::Music,
            Self::SyntaxStep(step) => step.ty.clone(),
            Self::Primitive { descriptor, .. } => Type::Primitive {
                step: Box::new(Type::Step(descriptor.step())),
                input: Box::new(descriptor.input().ty()),
                output: Box::new(descriptor.output().ty()),
            },
            Self::Machine { ty, .. } => ty.clone(),
            Self::Syntax(_) => Type::Syntax,
            Self::NodePath(_) => Type::NodePath,
            Self::BindingPath(_) => Type::BindingPath,
            Self::Closure(closure) => Type::Function(
                closure
                    .parameters
                    .iter()
                    .map(|parameter| parameter.ty.clone())
                    .collect(),
                Box::new(closure.result.clone()),
            ),
            // Only a track builtin is ever a value — a name position takes no other — so its
            // parameters are here to be read.
            Self::Builtin(builtin) => Type::Function(builtin.parameters().unwrap_or_default(), Box::new(Type::Music)),
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
            Self::Ratio(value) => value.numer().unsigned_abs().rotate_left(7) ^ value.denom().unsigned_abs(),
            Self::Duration(coordinate, value) | Self::Position(coordinate, value) => {
                u64::from(coordinate_tag(*coordinate)).rotate_left(3)
                    ^ value.numer().unsigned_abs().rotate_left(7)
                    ^ value.denom().unsigned_abs()
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
            Self::Text(value) => value
                .bytes()
                .fold(0u64, |witness, byte| witness.rotate_left(5) ^ u64::from(byte)),
            Self::Product(members) => members.iter().fold(0u64, |witness, member| {
                witness.rotate_left(5) ^ member.normalization_witness()
            }),
            Self::Sum { error, held, .. } => held.normalization_witness().rotate_left(3) ^ u64::from(*error),
            Self::Option { value, .. } => value.as_deref().map_or(0, Self::normalization_witness).rotate_left(1),
            Self::List { values, .. } => values.iter().fold(0u64, |witness, value| {
                witness.rotate_left(5) ^ value.normalization_witness()
            }),
            // The variant number joins the fields, because two constructors
            // holding nothing are two values and a witness that could not
            // tell them apart would not be a witness.
            Self::Data { variant, fields, .. } => fields.iter().fold(
                u64::try_from(*variant).unwrap_or(u64::MAX).rotate_left(3),
                |witness, field| witness.rotate_left(5) ^ field.normalization_witness(),
            ),
            Self::Music(music) => music_witness(music),
            Self::Closure(closure) => closure.captures.values().fold(
                u64::try_from(closure.parameters.len()).unwrap_or(u64::MAX),
                |witness, captured| witness.rotate_left(5) ^ captured.normalization_witness(),
            ),
            // A builtin carries nothing: it is applied completely or it is
            // the operation itself, and either way there is no argument of
            // its own to read.
            Self::Builtin(_) => 0,
            // A description is traversed the way every other finite value is:
            // through the bytes its identity is taken over, which is the whole
            // of what it holds.
            Self::Primitive {
                descriptor,
                configuration,
            } => u64::from(descriptor.version()).rotate_left(9) ^ configuration.normalization_witness(),
            Self::Machine { tree, .. } => {
                let mut nodes = Vec::new();
                tree.flatten(&mut nodes);
                nodes.iter().fold(0u64, |witness, node| {
                    node.stored()
                        .iter()
                        .fold(witness.rotate_left(5), |witness, byte| witness ^ u64::from(*byte))
                })
            }
            // A syntax value and a path are traversed through the bytes their
            // identity is taken over, exactly as a machine description is.
            Self::Syntax(held) => {
                let mut written = Vec::new();
                held.write_into(&mut written);
                bytes_witness(&written)
            }
            Self::NodePath(held) => {
                let mut written = Vec::new();
                held.write_into(&mut written);
                bytes_witness(&written)
            }
            Self::BindingPath(held) => {
                let mut written = Vec::new();
                held.write_into(&mut written);
                bytes_witness(&written)
            }
            // The child it seals, and the algebra it holds, each read the way
            // they would be read alone. A step is never a completed phase
            // result — it is not storable data — so this is reached only where
            // one is still in flight inside a value being traversed.
            Self::SyntaxStep(step) => {
                let mut written = Vec::new();
                step.child.write_into(&mut written);
                step.algebra.iter().fold(bytes_witness(&written), |witness, branch| {
                    witness.rotate_left(5) ^ branch.normalization_witness()
                })
            }
        }
    }
}

/// Fold a byte string into a witness, the way a machine's stored bytes are folded.
fn bytes_witness(written: &[u8]) -> u64 {
    written
        .iter()
        .fold(0u64, |witness, byte| witness.rotate_left(5) ^ u64::from(*byte))
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
        Some(MusicOperation::Together { left, right }) => music_witness(left).rotate_left(7) ^ music_witness(right),
        Some(MusicOperation::Play { voicing, held }) => {
            Value::Voicing(voicing.clone()).normalization_witness().rotate_left(11)
                ^ Value::Duration(Coordinate::WrittenTime, *held).normalization_witness()
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

/// Replace every type variable in a checked declaration by what the
/// substitution decided it was.
///
/// An expression is checked before the rest of its declaration constrains it,
/// so the type it was given at the time is not always the type it ends up
/// with: in `fn identity(value) { value }` the parameter and the result are
/// two variables until the body makes them one. This pass runs once the
/// declaration is finished, which is the first moment every constraint on it
/// exists, and it is what lets everything downstream — evaluation, the
/// preservation check, the closure a function becomes — read a plain type
/// rather than a promise of one.
///
/// It also answers the other question that moment settles: whether the
/// program determined every type it had to. What comes back is the first
/// expression that has to *build* a value out of a type nothing decided —
/// `docs/rules/language/02-core-calculus.md` §1.1's located error, rather
/// than a silent default.
fn settle(unifier: &Unifier, kind: &mut CheckedDefinitionKind) -> Option<(SourceSpan, Type)> {
    match kind {
        CheckedDefinitionKind::Let { body } => settle_expr(unifier, body),
        CheckedDefinitionKind::Function { parameters, body } => {
            for parameter in parameters.iter_mut() {
                parameter.ty = unifier.resolve(&parameter.ty);
            }
            settle_expr(unifier, body)
        }
    }
}

/// [`settle`], for one expression and everything under it.
///
/// `Music` and a kernel quotation are left alone: their types are `Music` and
/// their contents are placed material, neither of which a variable can reach.
fn settle_expr(unifier: &Unifier, expr: &mut Expr) -> Option<(SourceSpan, Type)> {
    expr.ty = unifier.resolve(&expr.ty);
    let under = match &mut expr.kind {
        ExprKind::Literal(_) | ExprKind::Name(_) | ExprKind::Music(_) | ExprKind::KernelQuote(_) => None,
        ExprKind::Product(members) | ExprKind::List(members) => {
            members.iter_mut().find_map(|member| settle_expr(unifier, member))
        }
        ExprKind::Option(member) => member.as_mut().and_then(|member| settle_expr(unifier, member)),
        ExprKind::Lambda {
            parameters,
            result,
            body,
            ..
        } => {
            for parameter in parameters.iter_mut() {
                parameter.ty = unifier.resolve(&parameter.ty);
            }
            *result = unifier.resolve(result);
            settle_expr(unifier, body)
        }
        ExprKind::Injection {
            held,
            value_type,
            error_type,
            ..
        } => {
            *value_type = unifier.resolve(value_type);
            *error_type = unifier.resolve(error_type);
            settle_expr(unifier, held)
        }
        ExprKind::Apply { function, arguments } => settle_expr(unifier, function).or_else(|| {
            arguments
                .iter_mut()
                .find_map(|argument| settle_expr(unifier, &mut argument.value))
        }),
        ExprKind::PitchAction { pitch, interval, .. } => {
            settle_expr(unifier, pitch).or_else(|| settle_expr(unifier, interval))
        }
        ExprKind::Builtin { arguments, .. } => arguments.iter_mut().find_map(|argument| settle_expr(unifier, argument)),
        ExprKind::Match { scrutinee, arms } => settle_expr(unifier, scrutinee)
            .or_else(|| arms.iter_mut().find_map(|arm| settle_expr(unifier, &mut arm.body))),
        ExprKind::Construct { arguments, fields, .. } => {
            for argument in arguments.iter_mut() {
                *argument = unifier.resolve(argument);
            }
            fields.iter_mut().find_map(|field| settle_expr(unifier, field))
        }
        ExprKind::Fold { cases, value, .. } => cases
            .iter_mut()
            .find_map(|case| settle_expr(unifier, case))
            .or_else(|| settle_expr(unifier, value)),
        ExprKind::Step { base, steps, .. } => settle_expr(unifier, base).or_else(|| settle_expr(unifier, steps)),
    };
    // A collection records its member type *inside the value it builds*, so
    // it is the one place a type nothing decided cannot simply be carried:
    // there is no member type to write down. Everywhere else an undecided
    // type is polymorphism, and the caller decides it.
    let ambiguous = matches!(
        expr.kind,
        ExprKind::Option(_) | ExprKind::List(_) | ExprKind::Injection { .. } | ExprKind::Construct { .. }
    ) && unifier.residue(&expr.ty).is_some();
    under.or_else(|| ambiguous.then(|| (expr.span, expr.ty.clone())))
}

/// Infer the declarations whose type the file did not write in full.
///
/// A declaration that wrote every annotation already has its type, and the
/// ordinary checking loop below is the only pass it needs. One that did not
/// is checked here first, with a scratch resolver whose diagnostics are
/// discarded, so that the name means its *principal* type by the time
/// anything reads it — rather than whatever the first use happened to need.
/// The loop below then checks it again, for real, against the types this
/// pass decided; unification is idempotent, so the second pass adds nothing
/// but the diagnostics.
///
/// Nothing happens here when every declaration wrote its type, which is what
/// keeps the second pass off the path files that do not need it.
fn infer_open_declarations(
    raw: &[RawDefinition],
    symbols: &mut IndexMap<String, Symbol>,
    unifier: &mut Unifier,
    modules: &Modules,
    world: &World,
) {
    let open: Vec<usize> = raw
        .iter()
        .enumerate()
        .filter(|(_, definition)| unifier.residue(&definition.ty).is_some())
        .map(|(index, _)| index)
        .collect();
    if open.is_empty() {
        return;
    }
    for index in dependency_first(raw, &open) {
        let Some(definition) = raw.get(index) else {
            continue;
        };
        let mut scratch = Resolver::new();
        let mut meter = WorkMeter::default();
        {
            let mut checker = Checker {
                resolver: &mut scratch,
                definitions: raw,
                symbols,
                locals: IndexMap::new(),
                unifier,
                dependencies: IndexMap::new(),
                mentioned: Vec::new(),
                reading: if definition.foreign {
                    Reading::Foreign
                } else {
                    Reading::Source
                },
                failed: false,
                meter: &mut meter,
                music_role: definition.role.clone(),
                deferred_pitch: false,
                definition_span: definition.span,
                scope: &definition.scope,
                modules,
                world,
                questions: Vec::new(),
                asked: 0,
                tail: false,
            };
            check_definition(&mut checker, definition);
        }
        let scheme = unifier.generalize(&definition.ty);
        if let Some(symbol) = symbols.get_mut(&definition.name) {
            symbol.scheme = scheme;
        }
    }
}

/// The open declarations, each ordered after the open declarations it names.
///
/// The scan is syntactic and deliberately generous: an extra edge only orders
/// two declarations that did not need ordering, while a missing one would let
/// a use decide a type its own declaration should have decided. The language
/// has no recursion, so this graph is acyclic; if a malformed program manages
/// a cycle anyway, the remaining declarations keep declaration order rather
/// than looping.
fn dependency_first(raw: &[RawDefinition], open: &[usize]) -> Vec<usize> {
    let mut named: IndexMap<&str, usize> = IndexMap::new();
    for index in open {
        let Some(definition) = raw.get(*index) else {
            continue;
        };
        named.insert(definition.name.as_str(), *index);
        // A module member is filed under `Module.member` and written as
        // `member` inside its own module, so both spellings find it.
        if let Some((_, member)) = definition.name.rsplit_once('.') {
            named.entry(member).or_insert(*index);
        }
    }
    let mut ordered = Vec::with_capacity(open.len());
    let mut placed = IndexSet::new();
    while ordered.len() < open.len() {
        let ready = open.iter().copied().find(|index| {
            !placed.contains(index)
                && raw.get(*index).is_some_and(|definition| {
                    mentions(definition, &named)
                        .into_iter()
                        .all(|dependency| dependency == *index || placed.contains(&dependency))
                })
        });
        let Some(next) = ready.or_else(|| open.iter().copied().find(|index| !placed.contains(index))) else {
            break;
        };
        placed.insert(next);
        ordered.push(next);
    }
    ordered
}

/// Which of `named` this declaration's body writes.
fn mentions(definition: &RawDefinition, named: &IndexMap<&str, usize>) -> Vec<usize> {
    let mut found = Vec::new();
    let mut scan = |node: &SyntaxNode| {
        for token in node.descendants_with_tokens().filter_map(SyntaxElement::into_token) {
            if token.kind() == SyntaxKind::Identifier
                && let Some(index) = named.get(token.text())
            {
                found.push(*index);
            }
        }
    };
    match &definition.kind {
        RawDefinitionKind::Bound { .. } => {}
        RawDefinitionKind::Let { body } => scan(body),
        RawDefinitionKind::Function { body, .. } | RawDefinitionKind::Music { body, .. } => scan(body),
    }
    found
}

/// Check one declaration's body, whatever kind of declaration it is.
///
/// Factored out because it runs twice for a declaration that did not write
/// its type: once with a scratch resolver, to infer and generalize before
/// anything reads the name, and once for real. For an annotated declaration
/// it runs exactly once, from the loop in [`check_and_evaluate`].
fn check_definition(checker: &mut Checker<'_>, definition: &RawDefinition) -> Option<CheckedDefinitionKind> {
    match &definition.kind {
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
                checker
                    .locals
                    .insert(parameter.name.clone(), Scheme::monomorphic(parameter.ty.clone()));
                checked_parameters.push(CheckedParameter {
                    name: parameter.name.clone(),
                    ty: parameter.ty.clone(),
                });
            }
            // A declaration's body is the one place a failure written in it
            // can leave from, so this is where its questions discharge.
            checker
                .answering(body, function_result(&definition.ty))
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
                    checker
                        .locals
                        .insert(parameter.name.clone(), Scheme::monomorphic(parameter.ty.clone()));
                    checked_parameters.push(CheckedParameter {
                        name: parameter.name.clone(),
                        ty: parameter.ty.clone(),
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
    }
}

/// Check and evaluate a document, under one budget and one meter.
///
/// The wrapper exists so that the meter has exactly one boundary. Anything
/// inside may stop at any `?`, and a run that stopped because the budget
/// stopped it must say so: a refusal that published no diagnostic and no score
/// would be the partial result `02-core-calculus.md` §4 forbids, and would
/// read to the composer as the compiler losing their piece.
fn check_and_evaluate(
    resolver: &mut Resolver,
    declarations: impl Iterator<Item = SurfaceDefinition>,
    root: Option<&SyntaxNode>,
    unknown_root_music: UnknownRootMusic,
    modules: &Modules,
    world: &World,
    reading: Reading,
) -> Option<Program> {
    let mut meter = WorkMeter::default();
    let mut unifier = Unifier::default();
    let program = check_and_evaluate_metered(
        resolver,
        declarations,
        root,
        unknown_root_music,
        modules,
        world,
        reading,
        &IndexMap::new(),
        &mut unifier,
        &mut meter,
    );
    if program.is_none()
        && let Some(failure) = meter.failure()
    {
        report_resource_error(resolver, failure);
    }
    program
}

fn check_and_evaluate_metered(
    resolver: &mut Resolver,
    declarations: impl Iterator<Item = SurfaceDefinition>,
    root: Option<&SyntaxNode>,
    unknown_root_music: UnknownRootMusic,
    modules: &Modules,
    world: &World,
    // Which document these declarations belong to: `Reading::Source` for every
    // ordinary path, and `Reading::Expansion` for an adapter module, which is
    // the whole of what makes one an adapter module. It rides here rather than
    // being written in at the checker because it is a fact about the document,
    // and the pass is the only thing that knows which document this is.
    reading: Reading,
    // Types the caller already knows some declarations must have, by name.
    //
    // Empty on every ordinary path: a piece's declarations mean what they say.
    // The expansion phase seeds it with `expand` and `edit`, because those are
    // the phase's own operations and their types are the interface rather than
    // something an adapter gets to infer. Without it an adapter whose `expand`
    // never refuses would leave the error half of its answer open, and be
    // rejected for not saying what it holds — the type would be missing from
    // the module because it belongs to the phase.
    expected: &IndexMap<String, Type>,
    // One substitution for the whole module. A variable minted for a
    // declaration that did not write its type is the same variable wherever it
    // is read, which is what makes the answer one answer. The caller owns it so
    // that the phase can read off what checking a module cost.
    unifier: &mut Unifier,
    meter: &mut WorkMeter,
) -> Option<Program> {
    let root_uses = root.map(root_uses).unwrap_or_default();
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
        if let Some(definition) = lower_signature(
            resolver,
            &world.scope(),
            &mut *unifier,
            declaration,
            name,
            name_span,
            span,
            source,
        ) {
            raw.push(definition);
        }
    }

    let mut symbols = IndexMap::new();
    for (index, definition) in raw.iter().enumerate() {
        symbols.insert(
            definition.name.clone(),
            Symbol {
                scheme: Scheme::monomorphic(definition.ty.clone()),
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
    }

    // A declaration whose type the file did not write in full is inferred
    // before anything reads it, so that a use meets the declaration's
    // principal type rather than whatever the first use happened to need.
    // Nothing runs here when every declaration wrote its type, which is the
    // case this pass costs nothing in.
    let mut expectation_failed = false;
    for definition in &raw {
        if let Some(wanted) = expected.get(&definition.name)
            && unifier.unify(&definition.ty, wanted).is_err()
        {
            expectation_failed = true;
        }
    }
    infer_open_declarations(&raw, &mut symbols, &mut *unifier, modules, world);

    // Documentation is written from the type each declaration ended up with,
    // which for an annotated one is what it wrote and for an inferred one is
    // what was inferred. That is why it is written here and not above.
    for definition in &raw {
        if !definition.hidden {
            let scheme = symbols.get(&definition.name).map_or_else(
                || Scheme::monomorphic(definition.ty.clone()),
                |symbol| symbol.scheme.clone(),
            );
            resolver.references.document(document(definition, &scheme));
        }
    }

    let mut checked = Vec::with_capacity(raw.len());
    let mut type_errors = expectation_failed;
    for definition in &raw {
        let mut checker = Checker {
            resolver,
            definitions: &raw,
            symbols: &symbols,
            locals: IndexMap::new(),
            unifier: &mut *unifier,
            dependencies: IndexMap::new(),
            mentioned: Vec::new(),
            reading: if definition.foreign { Reading::Foreign } else { reading },
            failed: false,
            meter: &mut *meter,
            music_role: definition.role.clone(),
            deferred_pitch: false,
            definition_span: definition.span,
            scope: &definition.scope,
            modules,
            world,
            questions: Vec::new(),
            asked: 0,
            tail: false,
        };
        let kind = check_definition(&mut checker, definition);
        let failed = checker.failed;
        let dependencies = checker.dependencies;
        type_errors |= failed || kind.is_none();
        let ty = (*unifier).resolve(&definition.ty);
        if let Some(mut kind) = kind {
            if let Some((span, undetermined)) = settle(unifier, &mut kind) {
                resolver.report(
                    Diagnostic::error(Code::TypeMismatch, "the program does not say what this holds")
                        .at(
                            span,
                            format!("this has type `{}`", crate::infer::plain_one(&undetermined)),
                        )
                        .help("annotate the declaration, or write this where its type is already decided"),
                );
                type_errors = true;
            }
            checked.push(CheckedDefinition {
                name: definition.name.clone(),
                ty,
                kind,
                dependencies,
                span: definition.span,
                foreign: definition.foreign,
            });
        }
    }
    if meter.failure().is_some() {
        return None;
    }
    if type_errors || checked.len() != raw.len() {
        return None;
    }

    let order = dependency_order(resolver, &checked)?;
    let values = evaluate(resolver, &checked, &order, &mut *meter)?;
    let mut uses = IndexMap::new();
    for statement in root_uses {
        let expression = child_of(&statement, is_expr_node)?;
        let span = crate::resolve::trimmed_span(&statement);
        // A track builtin heads a `use` the way a declared name does. A δ-builtin does not: it is
        // not a value, so a `use` naming one is as unknown here as a misspelling.
        if first_name(&expression)
            .is_some_and(|name| !symbols.contains_key(&name) && !Builtin::named(&name).is_some_and(Builtin::is_track))
        {
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
            unifier: &mut *unifier,
            dependencies: IndexMap::new(),
            mentioned: Vec::new(),
            reading: Reading::Source,
            failed: false,
            meter: &mut *meter,
            music_role: None,
            definition_span: span,
            deferred_pitch: false,
            scope: crate::module::NameScope::empty(),
            modules,
            world,
            questions: Vec::new(),
            asked: 0,
            tail: false,
        };
        let checked_use = checker.check(&expression, Some(&Type::Music))?;
        let Value::Music(music) = eval(&checked_use, &values, &mut *meter)? else {
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
            let mut checker = root_checker(
                resolver,
                &raw,
                &symbols,
                &mut *unifier,
                &mut *meter,
                span,
                modules,
                world,
            );
            let checked = checker.check(&expression, Some(&Type::Scale))?;
            let Value::Scale(scale) = eval(&checked, &values, &mut *meter)? else {
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
            let mut checker = root_checker(
                resolver,
                &raw,
                &symbols,
                &mut *unifier,
                &mut *meter,
                span,
                modules,
                world,
            );
            let checked = checker.check(&expression, Some(&Type::Key))?;
            let Value::Key(key) = eval(&checked, &values, &mut *meter)? else {
                return None;
            };
            keys.insert(span_key(span), key);
        }
        for statement in root_nodes(root, SyntaxKind::AssertStmt) {
            let span = crate::resolve::trimmed_span(&statement);
            let mut checker = root_checker(
                resolver,
                &raw,
                &symbols,
                &mut *unifier,
                &mut *meter,
                span,
                modules,
                world,
            );
            let checked = checker.claim(&statement)?;
            claims.insert(span_key(span), eval_claim(&checked, &values, &mut *meter)?);
        }
        for statement in root_nodes(root, SyntaxKind::NoteStmt) {
            let Some(expression) =
                musa_language::ast::NoteStmt::cast(statement.clone()).and_then(|note| note.pitch_expr())
            else {
                continue;
            };
            let span = crate::resolve::trimmed_span(&statement);
            let mut checker = root_checker(
                resolver,
                &raw,
                &symbols,
                &mut *unifier,
                &mut *meter,
                span,
                modules,
                world,
            );
            let checked = checker.deferring_pitch(|checker| checker.check(&expression, Some(&Type::Pitch)))?;
            pitches.insert(span_key(span), pitch_term(&checked, &values, &mut *meter)?);
        }
    }
    if meter.failure().is_some() {
        return None;
    }
    let named_music = values
        .iter()
        .filter_map(|(name, value)| match value {
            Value::Music(music) => Some((name.clone(), music.clone())),
            Value::Bool(_)
            | Value::Nat(_)
            | Value::Ratio(_)
            | Value::Text(_)
            | Value::Duration(..)
            | Value::Position(..)
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
            | Value::Sum { .. }
            | Value::Option { .. }
            | Value::List { .. }
            | Value::Data { .. }
            | Value::Closure(_)
            | Value::Primitive { .. }
            | Value::Machine { .. }
            | Value::Syntax(_)
            | Value::NodePath(_)
            | Value::BindingPath(_)
            | Value::SyntaxStep(_)
            | Value::Builtin(_) => None,
        })
        .collect();
    let types = checked
        .iter()
        .map(|definition| (definition.name.clone(), definition.ty.clone()))
        .collect();
    Some(Program {
        uses,
        pitches,
        scales,
        claims,
        keys,
        named_music,
        values,
        types,
    })
}

/// A checker for one expression written among a piece's own items.
fn root_checker<'a>(
    resolver: &'a mut Resolver,
    definitions: &'a [RawDefinition],
    symbols: &'a IndexMap<String, Symbol>,
    unifier: &'a mut Unifier,
    meter: &'a mut WorkMeter,
    span: SourceSpan,
    modules: &'a Modules,
    world: &'a World,
) -> Checker<'a> {
    Checker {
        resolver,
        definitions,
        symbols,
        locals: IndexMap::new(),
        unifier,
        dependencies: IndexMap::new(),
        mentioned: Vec::new(),
        reading: Reading::Source,
        failed: false,
        meter,
        music_role: None,
        definition_span: span,
        deferred_pitch: false,
        scope: crate::module::NameScope::empty(),
        modules,
        world,
        questions: Vec::new(),
        asked: 0,
        tail: false,
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
    // The document's root *and* the piece or voice being elaborated: a `data`
    // declaration is written where the values that use it are, so a path that
    // read only the root would elaborate a piece whose own types are unknown.
    let owners: Vec<SyntaxNode> = std::iter::once(root.clone()).chain(scope.cloned()).collect();
    let world = World::read(resolver, &owners);
    let modules = Modules::read(resolver, &world, std::iter::once((None, root.clone())));
    check_and_evaluate(
        resolver,
        root_preamble(root)
            .into_iter()
            .chain(bindings.into_iter().map(SurfaceDefinition::Bound))
            .chain(scope.map(|node| declarations(node, None)).unwrap_or_default()),
        scope,
        UnknownRootMusic::Silent,
        &modules,
        &world,
        Reading::Source,
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
///
/// `scheme` is what the declaration ended up meaning: what it wrote, or what
/// was inferred for it. A reader hovering an unannotated declaration is shown
/// the type it has, spelled the way an annotation would spell it.
fn document(definition: &RawDefinition, scheme: &Scheme) -> crate::docs::ItemDoc {
    let kind = definition.name_kind();
    let declared = scheme.renamed();
    let written: &[Type] = if let Type::Function(parameters, _) = &declared {
        parameters
    } else {
        &[]
    };
    let parameters = match &definition.kind {
        RawDefinitionKind::Function { parameters, .. } | RawDefinitionKind::Music { parameters, .. } => parameters
            .iter()
            .enumerate()
            .map(|(index, parameter)| {
                let ty = crate::docs::TypeNote::new(written.get(index).unwrap_or(&parameter.ty).to_string());
                crate::docs::ParameterDoc {
                    label: format!("{}: {}", parameter.name, ty.name),
                    name: parameter.name.clone(),
                    ty,
                }
            })
            .collect(),
        RawDefinitionKind::Let { .. } | RawDefinitionKind::Bound { .. } => Vec::new(),
    };
    // A callable evaluates to its result; everything else evaluates to itself.
    let result = crate::docs::TypeNote::new(if let Type::Function(_, result) = &declared {
        result.to_string()
    } else {
        declared.to_string()
    });
    // Every declaration the core lowers names a value; static structure is
    // documented where it is declared, in `crate::module`.
    let result = Some(result);
    // Whether the reader wrote `fn`, which is not the same question as whether
    // the declaration took arguments. A nullary `fn` is written with an empty
    // parameter list and called with one — `do_re_mi_strong()` — and reading
    // the word off the parameter *count* spelled it `let do_re_mi_strong:
    // List<Bool>`, which is wrong twice over: a reader who believed the record
    // would write the name bare and be told it has type `() -> List<Bool>`.
    // A motif is callable however it is written, so its own word still stands.
    let written_fn = matches!(definition.kind, RawDefinitionKind::Function { .. });
    let word = match kind {
        NameKind::Value | NameKind::Function => {
            if written_fn {
                "fn"
            } else {
                "let"
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
    if written_fn || !parameters.is_empty() {
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
        signature.push_str(if written_fn || !parameters.is_empty() {
            " -> "
        } else {
            ": "
        });
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

/// The type a declaration *declares*, before its body is read.
///
/// Where the file wrote a type, that is the type. Where it did not, a fresh
/// variable stands in — one the body will decide, or, if the body does not,
/// one the checker reports rather than guesses. That is the whole of what
/// "optional annotation" means here: the signature is still a signature, and
/// only some of it is written down.
fn lower_signature(
    resolver: &mut Resolver,
    scope: &TypeScope<'_>,
    unifier: &mut Unifier,
    definition: SurfaceDefinition,
    name: String,
    name_span: SourceSpan,
    span: SourceSpan,
    source: Option<String>,
) -> Option<RawDefinition> {
    let foreign = source.is_some();
    match definition {
        SurfaceDefinition::Let { declaration, .. } => {
            let body = child_of(declaration.syntax(), is_expr_node)?;
            let ty = match child_of(declaration.syntax(), is_type_node) {
                Some(node) => parse_type(resolver, scope, &node)?,
                None => unifier.fresh(Kind::Ordinary),
            };
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
                let parameter_ty = match child_of(parameter.syntax(), is_type_node) {
                    Some(ty_node) => {
                        let Some(parsed) = parse_type(resolver, scope, &ty_node) else {
                            continue;
                        };
                        parsed
                    }
                    None => unifier.fresh(Kind::Ordinary),
                };
                let parameter_name = parameter.name().unwrap_or_default();
                let parameter_span = crate::resolve::token_span(parameter.syntax(), SyntaxKind::Identifier)
                    .unwrap_or_else(|| crate::resolve::trimmed_span(parameter.syntax()));
                parameters.push(RawParameter {
                    name: parameter_name,
                    ty: parameter_ty,
                    span: parameter_span,
                });
            }
            // A parameter's type sits inside the parameter list, so the one
            // type node a `fn` has as a direct child is its result.
            let result = match declaration.syntax().children().find(|node| is_type_node(node.kind())) {
                Some(node) => parse_type(resolver, scope, &node)?,
                None => unifier.fresh(Kind::Ordinary),
            };
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
                    "Duration" => Type::Duration(Coordinate::WrittenTime),
                    "Position" => Type::Position(Coordinate::WrittenTime),
                    other => {
                        resolver.report(
                            Diagnostic::error(Code::UnknownName, format!("unknown type `{other}`"))
                                .at(span, "not a motif parameter type"),
                        );
                        return None;
                    }
                };
                raw_parameters.push(RawParameter {
                    name: parameter.name,
                    ty,
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
            let mut lowered = lower_signature(resolver, scope, unifier, inner, name, name_span, span, source)?;
            lowered.scope = member;
            Some(lowered)
        }
        SurfaceDefinition::Bound(binding) => {
            let ty = parse_type(resolver, scope, &binding.ty)?;
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

fn function_result(ty: &Type) -> Option<&Type> {
    match ty {
        Type::Function(_, result) => Some(result),
        Type::Var(_)
        | Type::Unit
        | Type::Bool
        | Type::Nat
        | Type::Ratio
        | Type::Text
        | Type::Duration(_)
        | Type::Position(_)
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
        | Type::Step(_)
        | Type::Primitive { .. }
        | Type::Machine { .. }
        | Type::Product(_)
        | Type::Sum(_, _)
        | Type::Option(_)
        | Type::Nominal(_, _)
        | Type::Syntax
        | Type::NodePath
        | Type::BindingPath
        | Type::SyntaxStep { .. }
        | Type::List(_) => None,
    }
}

fn parse_type(resolver: &mut Resolver, scope: &TypeScope<'_>, node: &SyntaxNode) -> Option<Type> {
    lower_type(Some(resolver), scope, node)
}

/// The type a node declares, read without reporting what it is not.
///
/// The module stage asks a member what its type *is*, in order to match it
/// against a signature; whether the type exists at all is a question the core
/// answers once, where the declaration is lowered, so asking here would
/// report the same mistake twice.
pub(crate) fn declared_type(scope: &TypeScope<'_>, node: &SyntaxNode) -> Option<Type> {
    lower_type(None, scope, node)
}

/// The type a signature member declares.
///
/// Reporting, unlike [`declared_type`]: a signature member's type is read
/// exactly once, here, so this is the only place that can say it is not a
/// type at all.
pub(crate) fn signature_type(resolver: &mut Resolver, scope: &TypeScope<'_>, node: &SyntaxNode) -> Option<Type> {
    lower_type(Some(resolver), scope, node)
}

/// The type a `data` declaration's field writes, read in the scope of that
/// declaration's own type parameters.
///
/// This is [`crate::data`]'s way in, and the only caller that passes a scope
/// with parameters in it.
pub(crate) fn scoped_type(resolver: &mut Resolver, scope: &TypeScope<'_>, node: &SyntaxNode) -> Option<Type> {
    lower_type(Some(resolver), scope, node)
}

/// Whether `name` is one of the types the compiler owns, so a library cannot
/// declare a second thing by that name and leave two readings of one word.
pub(crate) fn is_builtin_type_name(name: &str) -> bool {
    named_type(name).is_some()
        || crate::machine::StepTag::named(name).is_some()
        || matches!(name, "Option" | "List" | "Result" | "Machine" | "Primitive")
}

/// The arrow type a `fn` declares, which is the type a signature member of
/// arrow type must match.
///
/// A declaration that omitted an annotation still declares an arrow — it just
/// leaves part of it open. Each omission is a hole here, and
/// [`crate::infer::admits`] is how a signature reads one: the signature says
/// what goes there, and matching by name and exact type is unchanged for
/// every part the declaration did write. Returning `None` for an omission
/// would instead have made a member the signature fully describes look like
/// a member the module never defined.
///
/// The holes are numbered per declaration and belong to no [`Unifier`]. They
/// are never unified — only compared, by a relation in which any variable
/// matches — so there is nothing for them to be numbered against.
pub(crate) fn function_type(scope: &TypeScope<'_>, declaration: &FnDecl) -> Option<Type> {
    let mut hole = 0u32;
    let mut open = |node: Option<SyntaxNode>| match node {
        Some(node) => declared_type(scope, &node),
        None => {
            let variable = Type::Var(hole);
            hole = hole.saturating_add(1);
            Some(variable)
        }
    };
    let mut parameters = Vec::new();
    for parameter in declaration.params() {
        parameters.push(open(child_of(parameter.syntax(), is_type_node))?);
    }
    // A parameter's type is written inside the parameter list, so the one
    // type node a `fn` has as a direct child is its result.
    let result = open(declaration.syntax().children().find(|node| is_type_node(node.kind())))?;
    Some(Type::Function(parameters, Box::new(result)))
}

/// The type a written name denotes in an adapter module, and nowhere else.
///
/// Deliberately absent from `musa-language`'s `BASE_TYPES`: these are not
/// spellings the parser offers, the language server completes, or a composer
/// can write. They are read only where [`crate::data::TypeScope::in_phase`]
/// holds, which is the same boundary [`Reading::Expansion`] draws for the
/// phase's operations — one line between the two languages rather than two.
///
/// The three here take no arguments. `SyntaxStep<C, A>` does, so it is read
/// where the other applied forms are, under the same `in_phase` gate.
fn phase_type(text: &str) -> Option<Type> {
    match text {
        "Syntax" => Some(Type::Syntax),
        "NodePath" => Some(Type::NodePath),
        "BindingPath" => Some(Type::BindingPath),
        _ => None,
    }
}

/// The type a written name denotes, for the names the compiler owns.
///
/// The spellings are `musa-language`'s `BASE_TYPES`, which is where the
/// language server and the parser read them too; this is the one place that
/// says which [`Type`] each of them is.
fn named_type(text: &str) -> Option<Type> {
    match text {
        "Unit" => Some(Type::Unit),
        "Bool" => Some(Type::Bool),
        "Nat" => Some(Type::Nat),
        "Ratio" => Some(Type::Ratio),
        "Text" => Some(Type::Text),
        // `Duration` and `Position` are deliberately absent: they take a
        // coordinate, so the bare word names no type. Admitting it as an
        // alias for written time would make the vocabulary offered, the
        // vocabulary read, and the vocabulary printed three vocabularies —
        // a composer would write `Duration` and be answered about
        // `Duration<WrittenTime>`. [`coordinate_type`] reads the written
        // form, and the bare word is refused with it.
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

fn lower_type(mut resolver: Option<&mut Resolver>, scope: &TypeScope<'_>, node: &SyntaxNode) -> Option<Type> {
    let kind = node.kind();
    if kind == SyntaxKind::TypeExpr {
        return child_of(node, is_type_node).and_then(|child| lower_type(resolver, scope, &child));
    }
    if kind == SyntaxKind::TypeName {
        let text = node.to_string();
        let text = text.trim();
        if let Some(named) = named_type(text) {
            return Some(named);
        }
        // The phase's own three, and only where an adapter module is being
        // read. An adapter traffics in these types, so it must be able to
        // annotate a parameter and declare a `data` that holds a node;
        // ordinary source is read in a scope that is not `in_phase`, so there
        // the words fall through to the same "cannot find" any other unbound
        // type name gets, and §5's sentence stands.
        if scope.in_phase()
            && let Some(named) = phase_type(text)
        {
            return Some(named);
        }
        // A step tag is a type so that `K` unifies like any other index, but
        // it is not a *value* type: nothing inhabits it, and the only place it
        // can be written is the first argument of `Machine<…>` or
        // `Primitive<…>`, which is checked where those are read.
        if let Some(tag) = crate::machine::StepTag::named(text) {
            return Some(Type::Step(tag));
        }
        // A library-declared type, or one of the declaration's own
        // parameters. The compiler's own names are asked first, so no
        // declaration can quietly become a second reading of `Pitch`;
        // `crate::data` refuses such a declaration where it is written.
        if let Some(declared) = scope.named(text, Vec::new()) {
            return Some(declared);
        }
        if let Some(wanted) = scope.arity(text)
            && wanted > 0
        {
            if let Some(resolver) = resolver.as_deref_mut() {
                resolver.report(
                    Diagnostic::error(Code::WrongArity, format!("`{text}` takes {wanted} type arguments"))
                        .at(crate::resolve::trimmed_span(node), "written with none")
                        .help(format!("write `{text}<…>`")),
                );
            }
            return None;
        }
        // A removed spelling has already been reported at the word, with the
        // capital that replaces it, by the parser. Reading it as the type it
        // named leaves the rest of the declaration checked and keeps the
        // file's one complaint one complaint.
        if let Some(now) = musa_language::respelled_type(text) {
            return named_type(now);
        }
        if matches!(text, "Duration" | "Position") {
            if let Some(resolver) = resolver.as_deref_mut() {
                resolver.report(
                    Diagnostic::error(Code::WrongArity, format!("`{text}` takes a coordinate"))
                        .at(crate::resolve::trimmed_span(node), "written with none")
                        .help(format!(
                            "write `{text}<WrittenTime>`, or `<PhysicalTime>` for clock time"
                        )),
                );
            }
            return None;
        }
        if let Some(resolver) = resolver.as_deref_mut() {
            let vocabulary = musa_language::BASE_TYPES
                .iter()
                .map(|(name, _)| format!("`{name}`"))
                .collect::<Vec<_>>()
                .join(", ");
            resolver.report(
                Diagnostic::error(Code::UnknownName, format!("unknown type `{text}`"))
                    .at(crate::resolve::trimmed_span(node), "not a value type")
                    .help(format!(
                        "use {vocabulary}, `Duration<C>`, `Position<C>`, `Option<τ>`, `List<τ>`, a product, or a function type"
                    )),
            );
        }
        return None;
    }
    if kind == SyntaxKind::AppliedType {
        // The name is the first type child and the arguments are the rest:
        // an argument may itself be a bare `TypeName`, so telling them apart
        // by kind would take `Pair<Nat>` for a `Pair` of nothing.
        let mut parts = node.children().filter(|child| is_type_node(child.kind()));
        let name = parts.next()?;
        let written = name.to_string();
        let written = written.trim();
        // `Duration<C>` and `Position<C>` take a *coordinate*, which is a tag
        // rather than a type: nothing inhabits `WrittenTime`, and the only
        // place the word can be written is here. Reading it from the argument
        // node's own text — before the arguments are lowered — is what keeps
        // it out of [`Type`] entirely, so there is no non-value type to carry
        // through unification and no way to write `List<WrittenTime>`.
        if matches!(written, "Duration" | "Position") {
            let arguments: Vec<SyntaxNode> = parts.collect();
            return coordinate_type(resolver.as_deref_mut(), node, written, &arguments);
        }
        let arguments: Option<Vec<_>> = parts
            .collect::<Vec<_>>()
            .iter()
            .map(|child| lower_type(resolver.as_deref_mut(), scope, child))
            .collect();
        let arguments = arguments?;
        if matches!(written, "Machine" | "Primitive") {
            return machine_type(resolver.as_deref_mut(), node, written, arguments);
        }
        // The phase's one type constructor, and only where an adapter is being
        // read. It is spellable at all because a group branch is worth
        // factoring out: `fn group_read(state, here, delimiter, kids:
        // List<SyntaxStep<State, State>>)` is a definition, and without a
        // written spelling every branch would have to be one inline lambda.
        // Ordinary source is read in a scope that is not `in_phase`, so there
        // the word falls through to "not a type that takes arguments".
        if written == "SyntaxStep" && scope.in_phase() {
            let [context, answer] = arguments.as_slice() else {
                if let Some(resolver) = resolver.as_deref_mut() {
                    resolver.report(
                        Diagnostic::error(
                            Code::WrongArity,
                            format!("`SyntaxStep` takes 2 type arguments, not {}", arguments.len()),
                        )
                        .at(crate::resolve::trimmed_span(node), "written here")
                        .help("write `SyntaxStep<C, A>`: the context a step is run under, and what it answers with"),
                    );
                }
                return None;
            };
            return Some(Type::SyntaxStep {
                context: Box::new(context.clone()),
                answer: Box::new(answer.clone()),
            });
        }
        // Arity is checked here rather than at unification, because a
        // declaration written at the wrong size names no type at all: there is
        // nothing for a later pass to be wrong about.
        let wanted = scope.arity(written);
        if wanted != Some(arguments.len()) {
            if let Some(resolver) = resolver.as_deref_mut() {
                let complaint = match wanted {
                    Some(wanted) => format!("`{written}` takes {wanted} type arguments, not {}", arguments.len()),
                    None => format!("`{written}` is not a type that takes arguments"),
                };
                resolver.report(
                    Diagnostic::error(Code::WrongArity, complaint)
                        .at(crate::resolve::trimmed_span(node), "written here")
                        .help("`Option<τ>` and `List<τ>` are spelled the same way, and take one"),
                );
            }
            return None;
        }
        return scope.named(written, arguments);
    }
    if kind == SyntaxKind::ProductType {
        let members: Option<Vec<_>> = node
            .children()
            .filter(|child| is_type_node(child.kind()))
            .map(|child| lower_type(resolver.as_deref_mut(), scope, &child))
            .collect();
        return members.map(Type::Product);
    }
    if kind == SyntaxKind::FunctionType {
        let mut parts = node.children().filter(|child| is_type_node(child.kind()));
        let parameter = parts
            .next()
            .and_then(|part| lower_type(resolver.as_deref_mut(), scope, &part))?;
        let result = parts.next().and_then(|part| lower_type(resolver, scope, &part))?;
        return Some(Type::Function(vec![parameter], Box::new(result)));
    }
    if kind == SyntaxKind::ResultType {
        let mut parts = node.children().filter(|child| is_type_node(child.kind()));
        let value = parts
            .next()
            .and_then(|part| lower_type(resolver.as_deref_mut(), scope, &part))?;
        let error = parts.next().and_then(|part| lower_type(resolver, scope, &part))?;
        return Some(Type::Sum(Box::new(value), Box::new(error)));
    }
    if matches!(kind, SyntaxKind::OptionType | SyntaxKind::ListType) {
        let member = child_of(node, is_type_node).and_then(|child| lower_type(resolver, scope, &child))?;
        return if kind == SyntaxKind::OptionType {
            Some(Type::Option(Box::new(member)))
        } else {
            Some(Type::List(Box::new(member)))
        };
    }
    None
}

/// `Duration<C>` or `Position<C>`, read from what was written.
///
/// The argument is a coordinate word rather than a type, so it is read from
/// the node's own text: `WrittenTime` and `PhysicalTime` are the two, they
/// inhabit nothing, and no other type takes one. Getting it wrong is an error
/// here rather than an unsolvable constraint later, because a type written at
/// the wrong coordinate names no type at all.
fn coordinate_type(
    mut resolver: Option<&mut Resolver>,
    node: &SyntaxNode,
    written: &str,
    arguments: &[SyntaxNode],
) -> Option<Type> {
    let complain = |resolver: Option<&mut Resolver>, message: String, help: &str| {
        if let Some(resolver) = resolver {
            resolver.report(
                Diagnostic::error(Code::WrongArity, message)
                    .at(crate::resolve::trimmed_span(node), "written here")
                    .help(help.to_owned()),
            );
        }
    };
    let [argument] = arguments else {
        complain(
            resolver.as_deref_mut(),
            format!("`{written}` takes one coordinate, not {}", arguments.len()),
            "write `WrittenTime` or `PhysicalTime`, or write the name alone for written time",
        );
        return None;
    };
    let text = argument.to_string();
    let Some(coordinate) = Coordinate::named(text.trim()) else {
        complain(
            resolver,
            format!("`{}` is not a coordinate", text.trim()),
            "the coordinates are `WrittenTime` and `PhysicalTime`",
        );
        return None;
    };
    Some(match written {
        "Position" => Type::Position(coordinate),
        _ => Type::Duration(coordinate),
    })
}

/// `Machine<K, A, B>` or `Primitive<K, A, B>`, read from what was written.
///
/// Both are checked here rather than at unification because a written type is
/// the one place `K` can be got wrong by writing something that is not a step
/// tag at all, and because a port holding an arrow names no machine: §1.1 says
/// a machine's ports are storable data, and a type that could not be stored has
/// nothing for the unifier to be wrong about later.
fn machine_type(
    resolver: Option<&mut Resolver>,
    node: &SyntaxNode,
    written: &str,
    arguments: Vec<Type>,
) -> Option<Type> {
    let mut resolver = resolver;
    let mut complain = |complaint: String, label: &str, help: String| {
        if let Some(resolver) = resolver.as_deref_mut() {
            resolver.report(
                Diagnostic::error(Code::WrongArity, complaint)
                    .at(crate::resolve::trimmed_span(node), label.to_owned())
                    .help(help),
            );
        }
    };
    let [step, input, output] = <[Type; 3]>::try_from(arguments).ok().or_else(|| {
        complain(
            format!("`{written}` takes 3 type arguments: a step, an input port, and an output port"),
            "written here",
            format!("write `{written}<AudioFrameStep, τ, τ>`"),
        );
        None
    })?;
    if !matches!(step, Type::Step(_)) {
        complain(
            format!("`{step}` is not a step"),
            "written where a step belongs",
            "a step says what one step of the machine counts; `AudioFrameStep` is one".to_owned(),
        );
        return None;
    }
    for port in [&input, &output] {
        if holds_an_arrow(port) {
            complain(
                format!("`{port}` is not storable data, so it cannot be a port"),
                "written as a port",
                "a machine's ports carry values between steps, and a function is not a value that can be stored"
                    .to_owned(),
            );
            return None;
        }
    }
    let (step, input, output) = (Box::new(step), Box::new(input), Box::new(output));
    Some(if written == "Machine" {
        Type::Machine { step, input, output }
    } else {
        Type::Primitive { step, input, output }
    })
}

/// Whether an arrow appears anywhere in `ty` — §1.1's storable-data rule, read
/// on a written type rather than on an inferred one.
///
/// The inferred side of the same rule is `Kind::Data`, which refuses an arrow
/// structurally at unification; this is the same question asked of a type the
/// file wrote out, where there is no variable to constrain.
fn holds_an_arrow(ty: &Type) -> bool {
    matches!(ty, Type::Function(_, _) | Type::SyntaxStep { .. })
        || crate::infer::member_types(ty).into_iter().any(holds_an_arrow)
}

/// What a dotted name turned out to be, once a record projection is one of the
/// things it could have been.
///
/// Three answers, because "not a projection" and "a projection that was wrong"
/// are different facts: the first sends the name back to the ordinary lookup —
/// `M.member` reaches a structure this way — and the second has already been
/// reported and must not be looked up again.
enum Projected {
    /// Not a projection. The name reads the way it always did.
    Elsewhere,
    /// A projection, and wrong; the diagnostic is already reported.
    Rejected,
    /// The one-arm `match` the projection stands for.
    Made(Box<Expr>),
}

/// Which document the checker is reading.
///
/// It decides two things that go together: which names are in scope, and
/// whose spans this document may publish. The three readings are exclusive —
/// a transformer is not a foreign module and ordinary source is neither —
/// which is why they are one field rather than a pair of flags that could be
/// set at once.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Reading {
    /// The composer's own source.
    Source,
    /// A module compiled elsewhere: its spans belong to that document, so no
    /// use is recorded against them from here.
    Foreign,
    /// A transformer, where the phase environment is in scope.
    ///
    /// This is what makes `02-core-calculus.md` §5's "no syntax value"
    /// sentence still true of the source language: [`SYNTAX_OWNERSHIP`] is
    /// consulted only under this reading, and the three syntax types have no
    /// written spelling at all, so a piece can neither name one nor obtain
    /// one. Set only by [`expand_region`], which is the whole of the phase
    /// environment this prompt delivers.
    Expansion,
}

struct Checker<'a> {
    resolver: &'a mut Resolver,
    definitions: &'a [RawDefinition],
    symbols: &'a IndexMap<String, Symbol>,
    /// Parameters and other names bound inside this definition. A parameter
    /// is monomorphic while its function's body is checked — rank 1 says the
    /// quantifiers are outside — so most of these quantify nothing.
    locals: IndexMap<String, Scheme>,
    /// The substitution every type in this definition is read through. It is
    /// shared across the whole piece, so a variable minted for one
    /// declaration's missing annotation is the same variable there and here.
    unifier: &'a mut Unifier,
    dependencies: IndexMap<String, SourceSpan>,
    /// One frame per anonymous function being checked, holding every name its
    /// body mentioned. A lambda captures by value, and this is how it learns
    /// what to capture: the names it read, less its own parameters. Empty
    /// while nothing anonymous is open, which is most of the time.
    mentioned: Vec<IndexSet<String>>,
    reading: Reading,
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
    /// Every `data` declaration in scope: what a constructor, a fold, and a
    /// nominal type name mean here.
    world: &'a World,
    /// One frame per expression a failure could leave from, innermost last: a
    /// function body, and every branch whose value *is* that body's value.
    ///
    /// `?` has no core term to become, so what it elaborates to is a match
    /// wrapped around the answer it was written inside of, and this is where
    /// the questions wait until that answer has been checked. Empty at the top
    /// level, where a `?` has no function to leave.
    questions: Vec<QuestionFrame>,
    /// How many questions this definition has asked, for naming their binders.
    ///
    /// One counter for the whole definition rather than one per frame, so that
    /// no two binders in one body are spelled alike whatever they nest inside.
    asked: usize,
    /// Whether the expression about to be checked *is* the enclosing
    /// function's answer, rather than a part of something that is.
    ///
    /// Taken at the top of [`Checker::check`] and restored only by the forms
    /// that pass it on — a block, a parenthesis, and the branches of a
    /// conditional or a match — so every other position clears it by default
    /// and a form that wants to be transparent has to say so.
    tail: bool,
}

/// One place a `?` could carry a failure out to.
enum QuestionFrame {
    /// An answer of the enclosing function: a failure written here leaves the
    /// function, so `result` is what it leaves as and `asked` collects the
    /// questions in the order they were written.
    Open { result: Type, asked: Vec<PendingQuestion> },
    /// A branch whose value is *not* the function's answer.
    ///
    /// `g(match s { A -> h(e?) })` has nowhere to send a failure: the branch's
    /// value is an argument to `g`, so a match wrapped around it would answer
    /// `g` rather than the function's caller. Rust writes `return` here; this
    /// language has no statement to return from, so the position is refused
    /// and the author writes the `match` they mean.
    Blocked,
}

/// One `?`, waiting for the answer it was written inside of.
struct PendingQuestion {
    /// The subject, checked once. It becomes the scrutinee, which is what
    /// evaluates it exactly once.
    subject: Expr,
    /// The unspellable name standing where the `?` was written.
    binder: String,
    /// The `?` itself, so a failure points at the question that propagated it
    /// rather than at a match nobody wrote.
    span: SourceSpan,
}

impl Checker<'_> {
    fn check(&mut self, node: &SyntaxNode, expected: Option<&Type>) -> Option<Expr> {
        let span = crate::resolve::trimmed_span(node);
        let kind = node.kind();
        // Read the expected type through the substitution once, here, so that
        // every case below may take it apart. A position whose type is
        // *known* through a variable — a list member decided two arguments
        // ago — is a known position, and matching on the variable rather
        // than on what it stands for would lose that.
        let expected = expected.map(|ty| self.unifier.resolve(ty));
        let expected = expected.as_ref();
        // Taken rather than read: a position is not the function's answer
        // unless the form that wrote it says so, and the three that do say so
        // below. Everything else — an argument, a scrutinee, a field — clears
        // it by being checked at all.
        let tail = std::mem::take(&mut self.tail);
        let checked = if kind == SyntaxKind::ParenExpr || kind == SyntaxKind::BlockExpr {
            // `⟦{ e }⟧ = ⟦e⟧`, exactly as for parentheses. A block delimits
            // one expression and holds no sequence, so it adds a shape to the
            // surface and no case to this checker.
            child_of(node, is_expr_node).and_then(|child| {
                self.tail = tail;
                self.check(&child, expected)
            })
        } else if kind == SyntaxKind::LiteralExpr {
            self.literal(node, expected)
        } else if kind == SyntaxKind::NameExpr {
            let named = self.name(node)?;
            let shape = self.unifier.resolve(&named.ty);
            if expected == Some(&Type::Music)
                && matches!(&shape, Type::Function(parameters, result) if parameters.is_empty() && result.as_ref() == &Type::Music)
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
        } else if kind == SyntaxKind::ResultExpr {
            self.result(node, expected)
        } else if kind == SyntaxKind::ApplyExpr {
            self.application(node, expected)
        } else if kind == SyntaxKind::LambdaExpr {
            self.lambda(node, expected)
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
            self.match_expression(node, expected, tail)
        } else if kind == SyntaxKind::IfExpr {
            self.if_expression(node, expected, tail)
        } else if kind == SyntaxKind::RecordUpdateExpr {
            self.record_update(node)
        } else if kind == SyntaxKind::QuestionExpr {
            self.question(node)
        } else if kind == SyntaxKind::MusicExpr {
            self.music_expression(node)
        } else if kind == SyntaxKind::KernelQuote {
            self.kernel_quote(node)
        } else {
            None
        }?;
        if let Some(expected) = expected {
            self.reconcile(expected, &checked.ty, span)?;
        }
        Some(Expr {
            ty: self.unifier.resolve(&checked.ty),
            ..checked
        })
    }

    /// Make an expression's type the type its position asks for.
    ///
    /// This is the only place the checker compares two types, and it compares
    /// them by unifying: an expression whose type is not yet decided is
    /// decided *here*, by where it was written, which is what makes a
    /// missing annotation an inference problem rather than an error.
    ///
    /// Both types are resolved before they are quoted, so a diagnostic names
    /// what the substitution knows rather than the variable that stood in
    /// for it.
    fn reconcile(&mut self, expected: &Type, found: &Type, span: SourceSpan) -> Option<()> {
        let Err(mismatch) = self.unifier.unify(found, expected) else {
            return Some(());
        };
        // Resolved, then renamed together: what the reader is shown is the
        // shape the checker settled on, with one letter meaning one type
        // across both sides.
        let [expected, found] = crate::infer::plain([&self.unifier.resolve(expected), &self.unifier.resolve(found)]);
        let diagnostic = match mismatch {
            Mismatch::Shape => Diagnostic::error(Code::TypeMismatch, format!("expected `{expected}`, found `{found}`"))
                .at(span, format!("this has type `{found}`"))
                .maybe_help(crossing_help(&expected, &found)),
            Mismatch::Recursive => {
                Diagnostic::error(Code::TypeMismatch, format!("`{expected}` would have to contain itself"))
                    .at(span, format!("this has type `{found}`"))
                    .help("a type is finite; annotate this so the two sides say different things")
            }
            Mismatch::NotStorable => Diagnostic::error(Code::TypeMismatch, "a function cannot be stored here")
                .at(span, format!("this has type `{found}`"))
                .help(
                    "`Option<τ>` and `List<τ>` hold storable data — a type with no function at any depth; \
                     return the function from a call instead of putting it in a collection",
                ),
        };
        self.resolver.report(diagnostic);
        self.failed = true;
        None
    }

    /// Check `kernel EventTrack[WrittenTime, ScoreFact] { … }` — a quotation.
    ///
    /// The quote is read here, once, in four steps that are deliberately
    /// separate: the type constructor, coordinate, and payload name are
    /// *this* language's words and are checked against what this build can
    /// mean; the body text
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
            && constructor != "EventTrack"
        {
            self.resolver.report(
                Diagnostic::error(
                    Code::UnknownName,
                    format!("`{constructor}` is not a kernel type constructor"),
                )
                .at(SourceSpan::new(at.0, at.1), "expected `EventTrack`")
                .note("a quote writes one composition expression, and a composition is an event track"),
            );
            self.failed = true;
            return None;
        }
        // The coordinate is checked before the payload because it is the
        // stronger claim: a track in performed time is not a score, whatever
        // its payloads say, and nothing converts one coordinate into another.
        if let Some((coordinate, at)) = quote.coordinate()
            && coordinate != "WrittenTime"
        {
            self.resolver.report(
                Diagnostic::error(
                    Code::UnknownName,
                    format!("a score quote cannot be written in `{coordinate}`"),
                )
                .at(SourceSpan::new(at.0, at.1), "expected `WrittenTime`")
                .help("write `WrittenTime`, the coordinate notation is written in")
                .note("nothing converts one coordinate into another; performance derives its own"),
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
        let term = match musa_kernel::parse_expression::<musa_kernel::WrittenTime, crate::elaborate::ScoreFact>(&source)
        {
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
                musa_kernel::Term::literal(musa_kernel::empty(musa_kernel::Duration::ZERO)),
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
                    &Type::Duration(Coordinate::WrittenTime),
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
                    &Type::Duration(Coordinate::WrittenTime),
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
        let found = if let Some(scheme) = self.locals.get(name).cloned() {
            Some(self.unifier.instantiate(&scheme))
        } else if let Some(symbol) = self.symbols.get(name) {
            self.dependencies.entry(name.to_owned()).or_insert(span);
            if self.reading != Reading::Foreign {
                self.resolver
                    .references
                    .record_use_from(symbol.kind, name, span, symbol.external_declaration.clone());
            }
            let scheme = symbol.scheme.clone();
            Some(self.unifier.instantiate(&scheme))
        } else {
            None
        };
        if found
            .as_ref()
            .is_none_or(|found| self.unifier.unify(found, expected).is_err())
        {
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
        } else if kind == SyntaxKind::Integer && matches!(expected, Some(&Type::Duration(_))) {
            Value::Duration(
                Coordinate::WrittenTime,
                Ratio::from_integer(parse_i64(self.resolver, &token)?),
            )
        } else if kind == SyntaxKind::Integer && expected == Some(&Type::Ratio) {
            Value::Ratio(Ratio::from_integer(parse_i64(self.resolver, &token)?))
        } else if kind == SyntaxKind::Rational && matches!(expected, Some(&Type::Duration(_))) {
            Value::Duration(Coordinate::WrittenTime, parse_ratio(self.resolver, &token)?)
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
        } else if kind == SyntaxKind::String {
            Value::Text(musa_language::ast::unquote(token.text()))
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
        // A constructor and a generated fold are named before anything else
        // is looked up, because neither is a definition: they exist because a
        // `data` declaration does, and no `let` can shadow one.
        if self.world.is_constructor(&written) {
            return self.construct(&written, node, &[], span);
        }
        if self.world.is_fold(&written) {
            self.resolver.report(
                Diagnostic::error(Code::TypeMismatch, format!("`{written}` is a fold"))
                    .at(span, "apply it to a case per constructor and a value"),
            );
            self.failed = true;
            return None;
        }
        match self.projection(&written, span) {
            Projected::Made(expr) => return Some(*expr),
            Projected::Rejected => return None,
            Projected::Elsewhere => {}
        }
        if let Some(scheme) = self.locals.get(&written).cloned() {
            self.mention(&written);
            return Some(Expr {
                kind: ExprKind::Name(written),
                ty: self.unifier.instantiate(&scheme),
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
        // `identity`, `copy`, `drop`, and `swap` are machines, not functions to
        // one. A name is the whole of how they are written, which is why they
        // are read here rather than at an application.
        if let Some(builtin) = Builtin::named(&name)
            && let Some(Family::Machine(operation)) = builtin.family()
            && operation.arity() == 0
        {
            let ty = operation.instantiate(self.unifier)?;
            return Some(Expr {
                kind: ExprKind::Builtin {
                    builtin,
                    arguments: Vec::new(),
                },
                ty,
                span,
            });
        }
        if let Some(builtin) = Builtin::named(&name)
            && builtin.is_track()
        {
            let value = Value::Builtin(builtin);
            return Some(Expr {
                ty: value.ty(),
                kind: ExprKind::Literal(value),
                span,
            });
        }
        if Builtin::named(&name).is_some() {
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
            // `list_fold` was retired by prompt 127dcfaa rather than renamed, because a direction
            // change is the one break a reader cannot see: every call would keep compiling and
            // start answering differently wherever the step is not symmetric. The name resolves to
            // nothing, and this diagnostic is the migration.
            let retired = if name == "list_fold" {
                Diagnostic::error(Code::UnknownName, "`list_fold` no longer names an eliminator")
                    .at(span, "this fold has to say which end it runs from")
                    .help(
                        "`list_fold_from_start` keeps the old meaning: it accumulates left to right. \
                         `list_fold_from_end` is the catamorphism, and it builds right-nested data without a \
                         closure chain",
                    )
                    .note(
                        "`list` is the one type in the language whose fold direction is observable, because its \
                         outermost cons holds the first element — `02-core-calculus.md` §5.6",
                    )
                    .fix("keep the current meaning", span, "list_fold_from_start")
            } else {
                Diagnostic::error(Code::UnknownName, format!("cannot find `{name}`"))
                    .at(span, "nothing binds this name")
            };
            self.resolver.report(retired);
            self.failed = true;
            return None;
        };
        self.dependencies.entry(name.clone()).or_insert(span);
        if self.reading != Reading::Foreign {
            self.resolver
                .references
                .record_use_from(symbol.kind, &name, span, symbol.external_declaration.clone());
        }
        let scheme = symbol.scheme.clone();
        self.mention(&name);
        Some(Expr {
            kind: ExprKind::Name(name),
            ty: self.unifier.instantiate(&scheme),
            span,
        })
    }

    /// Note that a name was read, for whichever anonymous functions are open.
    ///
    /// Every open frame hears it, not just the innermost: a name a nested
    /// lambda reads through two of them has to be carried into both, or the
    /// outer closure would not have it to hand the inner one.
    fn mention(&mut self, name: &str) {
        for frame in &mut self.mentioned {
            frame.insert(name.to_owned());
        }
    }

    /// Check one use of a library-declared constructor.
    ///
    /// `arguments` is what the call wrote, empty for a bare name. A
    /// constructor is saturated or it is not a value: partial application
    /// would be an arrow the declaration never wrote down, and the language
    /// already answers "apply it directly" for its own polymorphic
    /// operations.
    fn construct(&mut self, name: &str, node: &SyntaxNode, arguments: &[SyntaxNode], span: SourceSpan) -> Option<Expr> {
        let owner = self.scope.owner().map(str::to_owned);
        let reading = self.world.constructor(name, owner.as_deref(), self.unifier)?;
        let (id, variant, declared, type_arguments, result) = match reading {
            crate::data::Constructing::Found {
                id,
                variant,
                fields,
                arguments,
                result,
            } => (id, variant, fields, arguments, result),
            crate::data::Constructing::Sealed {
                declared_in,
                ty,
                declared_at,
            } => {
                self.resolver.report(
                    Diagnostic::error(Code::UnknownName, format!("`{name}` is private"))
                        .at(span, "named from outside the structure that declares it")
                        .also(declared_at, format!("`{declared_in}` declares `{ty}` here"))
                        .help(format!(
                            "a structure's constructors are its own; `{declared_in}` has to expose a way to make a \
                             `{ty}`"
                        )),
                );
                self.failed = true;
                return None;
            }
        };
        if arguments.len() != declared.len() {
            self.resolver.report(
                Diagnostic::error(
                    Code::WrongArity,
                    format!(
                        "`{name}` takes {} field{}, not {}",
                        declared.len(),
                        if declared.len() == 1 { "" } else { "s" },
                        arguments.len()
                    ),
                )
                .at(span, "written here")
                .help("a constructor is written with every field it declares"),
            );
            self.failed = true;
            return None;
        }
        // A field may be named, exactly as a parameter may: `Sounded(held: 1)`
        // says which field it is, so the reading does not depend on order.
        let mut occupied = IndexSet::new();
        let mut positional = 0usize;
        let mut placed: Vec<Option<Expr>> = (0..declared.len()).map(|_| None).collect();
        for argument in arguments {
            let index = if let Some(named) = argument_name(argument) {
                let Some(index) = declared.iter().position(|(field, _)| field == &named) else {
                    self.resolver.report(
                        Diagnostic::error(Code::WrongArity, format!("`{name}` has no field `{named}`"))
                            .at(crate::resolve::trimmed_span(argument), "unknown field"),
                    );
                    self.failed = true;
                    return None;
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
            if index >= declared.len() || !occupied.insert(index) {
                self.resolver.report(
                    Diagnostic::error(Code::WrongArity, "this field is written twice")
                        .at(crate::resolve::trimmed_span(argument), "already given"),
                );
                self.failed = true;
                return None;
            }
            let value_node = child_of(argument, is_expr_node)?;
            let checked = self.check(&value_node, declared.get(index).map(|(_, ty)| ty))?;
            let (_, wanted) = declared.get(index)?;
            self.reconcile(wanted, &checked.ty, checked.span)?;
            *placed.get_mut(index)? = Some(checked);
        }
        let fields = placed.into_iter().collect::<Option<Vec<_>>>()?;
        Some(Expr {
            kind: ExprKind::Construct {
                id,
                variant,
                arguments: type_arguments,
                fields,
            },
            ty: result,
            span: crate::resolve::trimmed_span(node),
        })
    }

    /// Check one use of a declaration's generated fold: a case per
    /// constructor of the group, in declaration order, and then the value.
    fn fold_application(&mut self, name: &str, node: &SyntaxNode, expected: Option<&Type>) -> Option<Expr> {
        let folding = self.world.fold(name, self.unifier)?;
        let span = crate::resolve::trimmed_span(node);
        let Type::Function(parameters, result) = folding.ty.clone() else {
            return None;
        };
        let written = raw_arguments(node);
        if written.len() != parameters.len() {
            self.resolver.report(
                Diagnostic::error(
                    Code::WrongArity,
                    format!("`{name}` takes {} arguments, not {}", parameters.len(), written.len()),
                )
                .at(span, "written here")
                .note("a fold takes one case per constructor of its group, and then the value"),
            );
            self.failed = true;
            return None;
        }
        let mut checked = Vec::with_capacity(written.len());
        for (argument, wanted) in written.iter().zip(&parameters) {
            let value_node = child_of(argument, is_expr_node)?;
            let value = self.check(&value_node, Some(wanted))?;
            self.reconcile(wanted, &value.ty, value.span)?;
            checked.push(value);
        }
        let value = checked.pop()?;
        if let Some(expected) = expected {
            self.reconcile(expected, &result, span)?;
        }
        Some(Expr {
            kind: ExprKind::Fold {
                cases: checked,
                shape: folding.cases,
                value: Box::new(value),
            },
            ty: result.as_ref().clone(),
            span,
        })
    }

    /// Read `value.field` as a projection, when `value` names a record — a
    /// declaration with one constructor. See [`Projected`].
    ///
    /// Only one constructor, because with two there is no field every value
    /// has, and a projection that could fail is not a projection. It is
    /// checked as the `match` a reader would otherwise write, so the one
    /// eliminator stays the one eliminator.
    ///
    /// [`Projected::Elsewhere`] means this is not a projection at all and the
    /// name reads the way it always did.
    fn projection(&mut self, written: &str, span: SourceSpan) -> Projected {
        let Some((head, field)) = written.rsplit_once(crate::module::DOT) else {
            return Projected::Elsewhere;
        };
        // A record is projected wherever it is bound: a parameter, a `let` in
        // a body, or a declaration of the document. The declaration case has
        // to be recorded as a use, the way naming it plainly would be, or the
        // evaluation order would not know this expression needs it first.
        let declared = !self.locals.contains_key(head);
        let scheme = match self
            .locals
            .get(head)
            .or_else(|| self.symbols.get(head).map(|symbol| &symbol.scheme))
        {
            Some(scheme) => scheme.clone(),
            None => return Projected::Elsewhere,
        };
        let ty = self.unifier.instantiate(&scheme);
        let Type::Nominal(id, arguments) = self.unifier.resolve(&ty) else {
            return Projected::Elsewhere;
        };
        if declared {
            self.dependencies.entry(head.to_owned()).or_insert(span);
        }
        self.mention(head);
        let variants = self.world.variants(&id);
        if variants.len() != 1 {
            self.resolver.report(
                Diagnostic::error(Code::TypeMismatch, format!("`{id}` has more than one constructor"))
                    .at(span, "so there is no field every value of it has")
                    .help("match on it instead, which answers for each constructor"),
            );
            self.failed = true;
            return Projected::Rejected;
        }
        let fields = self.world.fields(&id, 0, &arguments);
        let Some((index, (_, member))) = fields
            .iter()
            .enumerate()
            .find(|(_, (name, _))| name == field)
            .map(|(index, member)| (index, member.clone()))
        else {
            self.resolver.report(
                Diagnostic::error(Code::UnknownName, format!("`{id}` has no field `{field}`"))
                    .at(span, "not a field of this record")
                    .help(format!(
                        "it has {}",
                        fields
                            .iter()
                            .map(|(name, _)| format!("`{name}`"))
                            .collect::<Vec<_>>()
                            .join(", ")
                    )),
            );
            self.failed = true;
            return Projected::Rejected;
        };
        // Names no source can write, so a field called `value` cannot capture
        // the record it was projected out of.
        let bindings: Vec<String> = (0..fields.len()).map(|slot| format!("#field.{slot}")).collect();
        let body = Expr {
            kind: ExprKind::Name(format!("#field.{index}")),
            ty: member.clone(),
            span,
        };
        Projected::Made(Box::new(Expr {
            kind: ExprKind::Match {
                scrutinee: Box::new(Expr {
                    kind: ExprKind::Name(head.to_owned()),
                    ty,
                    span,
                }),
                arms: vec![CheckedArm {
                    pattern: Pattern::Constructor {
                        variant: 0,
                        fields: bindings,
                    },
                    body,
                }],
            },
            ty: member,
            span,
        }))
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
        // As in `option`: a variable expected type names a member the
        // elements decide, and `settle` reports one nothing decided.
        let expected_member = match expected {
            Some(Type::List(member)) => Some(member.as_ref().clone()),
            Some(Type::Var(_)) => Some(self.unifier.fresh(Kind::Ordinary)),
            _ => None,
        };
        let expected_member = expected_member.as_ref();
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

    /// `Ok(value)` or `Err(reason)` — one injection into the binary sum.
    ///
    /// The written side decides one half of the type and inference decides
    /// the other, so a constructor alone never determines a `Result`: the
    /// side not written is a fresh variable, which the annotation, the use,
    /// or the other arm of a `match` settles. Where nothing does, `settle`
    /// reports it at the declaration, which is the same treatment `None`
    /// already gets and for the same reason.
    fn result(&mut self, node: &SyntaxNode, expected: Option<&Type>) -> Option<Expr> {
        let (expected_value, expected_error) = match expected {
            Some(Type::Sum(value, error)) => (value.as_ref().clone(), error.as_ref().clone()),
            _ => (self.unifier.fresh(Kind::Ordinary), self.unifier.fresh(Kind::Ordinary)),
        };
        // The constructor is this node's *own* word, so only this node's own
        // tokens are asked. A descendant's word belongs to the descendant:
        // `Ok(match r { Ok(v) -> v, Err(m) -> … })` is an `Ok` whose value
        // happens to mention `Err`, and reading it as an `Err` would check the
        // value against the error half of a type nobody wrote it for.
        let error = node
            .children_with_tokens()
            .filter_map(SyntaxElement::into_token)
            .any(|token| token.kind() == SyntaxKind::ErrKw);
        let held_node = child_of(node, is_expr_node)?;
        let wanted = if error { &expected_error } else { &expected_value };
        let held = Box::new(self.check(&held_node, Some(wanted))?);
        Some(Expr {
            kind: ExprKind::Injection {
                error,
                held,
                value_type: expected_value.clone(),
                error_type: expected_error.clone(),
            },
            ty: Type::Sum(Box::new(expected_value), Box::new(expected_error)),
            span: crate::resolve::trimmed_span(node),
        })
    }

    fn option(&mut self, node: &SyntaxNode, expected: Option<&Type>) -> Option<Expr> {
        // An expected type that is still a variable says only "something
        // goes here": the member is a fresh variable, which whatever is
        // written decides, and which `settle` reports if nothing does.
        let expected_member = match expected {
            Some(Type::Option(member)) => Some(member.as_ref().clone()),
            Some(Type::Var(_)) => Some(self.unifier.fresh(Kind::Ordinary)),
            _ => None,
        };
        let expected_member = expected_member.as_ref();
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

    /// `if c { a } else { b }` — the two-arm boolean match, written the way an
    /// author asks the question.
    ///
    /// The elaboration is the whole of it: there is no `If` term, no typing
    /// rule, no reduction, and no measure case, because what this builds is
    /// the `Match` the surface already had
    /// (`docs/rules/language/02-core-calculus.md` §1). Adding a core term
    /// would give `bool` two eliminators, which §5.6's own argument against
    /// redundant eliminators refuses.
    ///
    /// Each of the three parts is checked at *its* span, so a non-boolean
    /// condition is reported on the condition and a branch that disagrees is
    /// reported on that branch — never on a synthesized match the author did
    /// not write. The consequent goes first so that, with no expected type
    /// from above, it is the one that decides and the alternative is the one
    /// asked to agree.
    fn if_expression(&mut self, node: &SyntaxNode, expected: Option<&Type>, tail: bool) -> Option<Expr> {
        let mut parts = node.children().filter(|child| is_expr_node(child.kind()));
        let condition_node = parts.next()?;
        let consequent_node = parts.next()?;
        let alternative_node = parts.next()?;
        let condition = self.check(&condition_node, Some(&Type::Bool))?;
        // Same reason as a match's first arm: a `?` in the consequent has to
        // know what the conditional answers before the consequent decides it.
        let wanted = match (tail, expected) {
            (true, None) => Some(self.unifier.fresh(Kind::Ordinary)),
            (_, expected) => expected.cloned(),
        };
        let consequent = self.branch(&consequent_node, wanted.as_ref(), tail)?;
        let alternative = self.branch(&alternative_node, Some(&consequent.ty), tail)?;
        let result = self.unifier.resolve(&consequent.ty);
        Some(Expr {
            kind: ExprKind::Match {
                scrutinee: Box::new(condition),
                arms: vec![
                    CheckedArm {
                        pattern: Pattern::Literal(Value::Bool(true)),
                        body: consequent,
                    },
                    CheckedArm {
                        pattern: Pattern::Literal(Value::Bool(false)),
                        body: alternative,
                    },
                ],
            },
            ty: result,
            span: crate::resolve::trimmed_span(node),
        })
    }

    /// Check a function's body, and discharge the questions written in it.
    ///
    /// The body is what the function answers, so this is the outermost place a
    /// `?` can carry a failure to. A body with no written result type still
    /// gets a frame: the variable standing for that type is what `?`
    /// constrains, which is how an unannotated function infers through one.
    fn answering(&mut self, node: &SyntaxNode, result: Option<&Type>) -> Option<Expr> {
        self.branch(node, result, true)
    }

    /// Check one branch of a conditional or a match, and discharge the
    /// questions written in it.
    ///
    /// A branch is where `?` stops being hoistable. Everything above a branch
    /// — an argument, a scrutinee, a field — is evaluated whatever happens, so
    /// a question there can be lifted to the answer around it without changing
    /// when its subject runs. A branch is not: lifting a question out of one
    /// arm would evaluate its subject even when the other arm was taken. So
    /// the branch discharges its own questions, and can do so exactly when it
    /// is itself the function's answer — which is what `tail` records.
    fn branch(&mut self, node: &SyntaxNode, expected: Option<&Type>, tail: bool) -> Option<Expr> {
        let frame = match (tail, expected) {
            (true, Some(result)) => QuestionFrame::Open {
                result: result.clone(),
                asked: Vec::new(),
            },
            _ => QuestionFrame::Blocked,
        };
        self.questions.push(frame);
        self.tail = tail;
        let body = self.check(node, expected);
        let frame = self.questions.pop();
        self.discharge(frame?, body?)
    }

    /// Wrap an answer in the propagating matches the questions inside it asked
    /// for, outermost question first.
    ///
    /// The questions are discharged in the order they were written, which is
    /// the order their subjects are evaluated and therefore which failure a
    /// program reports when more than one thing goes wrong. Nothing else about
    /// the answer moves: each wrap is the two-arm `Result` match the surface
    /// already had, so what this returns is a term the author could have
    /// written, at the same cost.
    fn discharge(&self, frame: QuestionFrame, body: Expr) -> Option<Expr> {
        let QuestionFrame::Open { result, asked } = frame else {
            return Some(body);
        };
        if asked.is_empty() {
            return Some(body);
        }
        let result = self.unifier.resolve(&result);
        // Every question unified the answer with a sum before it was recorded,
        // so this holds by the time anything was recorded at all.
        let Type::Sum(value, error) = result.clone() else {
            return Some(body);
        };
        let mut answer = body;
        for question in asked.into_iter().rev() {
            let span = question.span;
            let failure = format!("{} failed", question.binder);
            answer = Expr {
                kind: ExprKind::Match {
                    scrutinee: Box::new(question.subject),
                    arms: vec![
                        CheckedArm {
                            pattern: Pattern::Ok(question.binder),
                            body: answer,
                        },
                        CheckedArm {
                            pattern: Pattern::Err(failure.clone()),
                            body: Expr {
                                kind: ExprKind::Injection {
                                    error: true,
                                    held: Box::new(Expr {
                                        kind: ExprKind::Name(failure),
                                        ty: error.as_ref().clone(),
                                        span,
                                    }),
                                    value_type: value.as_ref().clone(),
                                    error_type: error.as_ref().clone(),
                                },
                                ty: result.clone(),
                                span,
                            },
                        },
                    ],
                },
                ty: result.clone(),
                span,
            };
        }
        Some(answer)
    }

    /// `e?` — the success payload here, and the same failure out there.
    ///
    /// One meaning and no others: the subject is a `Result`, the answer around
    /// it is a `Result` with the identical error type, and the value of the
    /// whole thing is what the subject succeeded with. There is no conversion
    /// between error types, because a function's signature is supposed to say
    /// which failures can come out of it and a silent widening would make that
    /// sentence untrue.
    ///
    /// The expression this returns is a name — the binder the enclosing
    /// [`Checker::branch`] will bind in the `Ok` arm it wraps. The binder is
    /// unspellable, so nothing an author writes can capture it, and the
    /// subject appears once, as that match's scrutinee, so it is evaluated
    /// exactly once.
    fn question(&mut self, node: &SyntaxNode) -> Option<Expr> {
        let span = crate::resolve::trimmed_span(node);
        let subject_node = child_of(node, is_expr_node)?;
        let subject = self.check(&subject_node, None)?;
        let asked = self.unifier.resolve(&subject.ty);
        let Type::Sum(value, error) = asked else {
            let named = crate::infer::plain_one(&asked);
            let mut report = Diagnostic::error(
                Code::TypeMismatch,
                format!("`?` asks a `Result`, and this is `{named}`"),
            )
            .at(subject.span, format!("this has type `{named}`"));
            report = if matches!(asked, Type::Option(_)) {
                // An `Option` is the one near miss worth its own sentence: it
                // has a missing case but no failure to carry, so there is
                // nothing for `?` to propagate and the author has to say what
                // the absence means.
                report.help(
                    "an `Option` says only that a value is missing, not why; \
                     match on it and say what the missing case means",
                )
            } else {
                report.help("`?` carries a failure outward, so what it asks has to be able to fail")
            };
            self.resolver.report(report);
            self.failed = true;
            return None;
        };
        let answer = match self.questions.last() {
            Some(QuestionFrame::Open { result, .. }) => result.clone(),
            here => {
                let (told, help) = match here {
                    Some(QuestionFrame::Blocked) => (
                        "this branch is not what the function answers, so a failure has nowhere to go",
                        "a `?` carries a failure out of the function around it; write the `match` this branch means",
                    ),
                    _ => (
                        "there is no function here for a failure to leave",
                        "a `?` carries a failure out of the function around it; match on the `Result` instead",
                    ),
                };
                self.resolver.report(
                    Diagnostic::error(Code::TypeMismatch, "`?` has nowhere to carry a failure to")
                        .at(span, told)
                        .help(help),
                );
                self.failed = true;
                return None;
            }
        };
        // The answer is constrained to a `Result` failing the same way, which
        // is what makes an unannotated function infer through `?` instead of
        // demanding that its result be written down.
        let carried = Type::Sum(Box::new(self.unifier.fresh(Kind::Ordinary)), error);
        if self.unifier.unify(&answer, &carried).is_err() {
            let [answering, asking] =
                crate::infer::plain([&self.unifier.resolve(&answer), &self.unifier.resolve(&subject.ty)]);
            // Two different mistakes, and a reader can only act on the one
            // they made: an answer that is no `Result` at all needs a
            // different result type, and one that fails another way needs the
            // failure said in this language's words before it goes out.
            let report = if matches!(self.unifier.resolve(&answer), Type::Sum(_, _)) {
                Diagnostic::error(
                    Code::TypeMismatch,
                    format!("a failure of `{asking}` cannot leave an answer of `{answering}`"),
                )
                .at(span, format!("this fails with `{asking}`"))
                .help(
                    "`?` carries a failure unchanged and converts nothing; match on this one and \
                     say what it means here",
                )
            } else {
                Diagnostic::error(
                    Code::TypeMismatch,
                    format!("this answers `{answering}`, so a failure has no way out of it"),
                )
                .at(span, format!("this fails with `{asking}`"))
                .help(
                    "`?` carries a failure out of the answer around it, so that answer is a \
                     `Result` too; match on this one instead",
                )
            };
            self.resolver.report(report);
            self.failed = true;
            return None;
        }
        let binder = format!(" answer {}", self.asked);
        self.asked = self.asked.saturating_add(1);
        let Some(QuestionFrame::Open { asked, .. }) = self.questions.last_mut() else {
            return None;
        };
        asked.push(PendingQuestion {
            subject,
            binder: binder.clone(),
            span,
        });
        Some(Expr {
            kind: ExprKind::Name(binder),
            ty: value.as_ref().clone(),
            span,
        })
    }

    /// `p with { f = e, … }` — `p`, rebuilt with `f` replaced.
    ///
    /// The elaboration is a match on the subject that binds every field, and a
    /// construction that takes the written right-hand sides where they were
    /// given and the bound field everywhere else. This language has no surface
    /// projection (`docs/rules/language/02-core-calculus.md` §5.4), so binding
    /// by pattern *is* how the unmentioned fields are read; the subject is the
    /// match's scrutinee, so it is evaluated exactly once however many fields
    /// are carried over, and exactly one record is constructed, which is the
    /// charge prompt 127dcec asks for.
    ///
    /// The binders are unspellable — a leading space is not an identifier — so
    /// a right-hand side naming `dots` reads the `dots` in scope around the
    /// update and never the field of the same name. That is the contract's
    /// second clause, enforced by construction rather than by renaming.
    ///
    /// Fields are read in written order, so the first complaint about an
    /// update is about the leftmost thing wrong with it, and the record is
    /// built in declaration order, which is already what a constructor written
    /// with named fields does.
    fn record_update(&mut self, node: &SyntaxNode) -> Option<Expr> {
        let subject_node = child_of(node, is_expr_node)?;
        let subject = self.check(&subject_node, None)?;
        let span = crate::resolve::trimmed_span(node);
        let resolved = self.unifier.resolve(&subject.ty);
        let Type::Nominal(id, type_arguments) = resolved else {
            self.resolver.report(
                Diagnostic::error(
                    Code::TypeMismatch,
                    format!("`{resolved}` is not a record, so there is nothing to rebuild"),
                )
                .at(subject.span, "written here")
                .help("`with` rebuilds a value of a `data` type declared with one constructor"),
            );
            self.failed = true;
            return None;
        };
        let variants = self.world.variants(&id);
        let [(constructor, _)] = variants.as_slice() else {
            let mut report = Diagnostic::error(
                Code::TypeMismatch,
                format!(
                    "`{id}` has {} constructors, so which one to rebuild is not written",
                    variants.len()
                ),
            )
            .at(subject.span, "written here")
            .help("take it apart with `match`, which names the case, and rebuild inside the arm");
            if let Some(declared) = self.world.declared_at(&id) {
                report = report.also(declared, format!("`{id}` is declared here"));
            }
            self.resolver.report(report);
            self.failed = true;
            return None;
        };
        let constructor = constructor.clone();
        // Rebuilding is constructing, so a constructor this use may not write
        // is a value this use may not rebuild. Asked through the one place
        // that answers it, so the two readings cannot drift apart.
        let owner = self.scope.owner().map(str::to_owned);
        if let Some(crate::data::Constructing::Sealed {
            declared_in,
            ty,
            declared_at,
        }) = self.world.constructor(&constructor, owner.as_deref(), self.unifier)
        {
            self.resolver.report(
                Diagnostic::error(Code::UnknownName, format!("`{ty}` is private"))
                    .at(span, "rebuilt from outside the structure that declares it")
                    .also(declared_at, format!("`{declared_in}` declares `{ty}` here"))
                    .help(format!(
                        "rebuilding a value writes its constructor; `{declared_in}` has to expose a way to make a \
                         `{ty}`"
                    )),
            );
            self.failed = true;
            return None;
        }
        let declared = self.world.fields(&id, 0, &type_arguments);
        let mut given: Vec<Option<Expr>> = (0..declared.len()).map(|_| None).collect();
        let mut first_mention: IndexMap<usize, SourceSpan> = IndexMap::new();
        for field in node.children().filter(|child| child.kind() == SyntaxKind::FieldUpdate) {
            // The field's *name*, not the whole `name = value`: what is wrong
            // about an unknown or repeated field is the name, and the value
            // beside it is not part of the mistake.
            let named = field
                .children_with_tokens()
                .filter_map(SyntaxElement::into_token)
                .find(|token| token.kind() == SyntaxKind::Identifier)?;
            let at = token_span(&named);
            let named = named.text().to_owned();
            let Some(index) = declared.iter().position(|(name, _)| name == &named) else {
                let mut report = Diagnostic::error(Code::UnknownName, format!("`{id}` has no field `{named}`"))
                    .at(at, "unknown field")
                    .help(format!(
                        "`{id}` stores {}",
                        declared
                            .iter()
                            .map(|(name, _)| format!("`{name}`"))
                            .collect::<Vec<_>>()
                            .join(", ")
                    ));
                if let Some(declared_at) = self.world.declared_at(&id) {
                    report = report.also(declared_at, format!("`{id}` is declared here"));
                }
                self.resolver.report(report);
                self.failed = true;
                return None;
            };
            if let Some(already) = first_mention.get(&index) {
                self.resolver.report(
                    Diagnostic::error(Code::DuplicateName, format!("`{named}` is replaced twice"))
                        .at(at, "given again here")
                        .also(*already, "first given here")
                        .help("one update replaces each field once; the second value would silently win"),
                );
                self.failed = true;
                return None;
            }
            first_mention.insert(index, at);
            let value_node = child_of(&field, is_expr_node)?;
            let wanted = declared.get(index).map(|(_, ty)| ty.clone())?;
            let checked = self.check(&value_node, Some(&wanted))?;
            self.reconcile(&wanted, &checked.ty, checked.span)?;
            *given.get_mut(index)? = Some(checked);
        }
        let binders = (0..declared.len())
            .map(|index| format!(" field {index}"))
            .collect::<Vec<_>>();
        let fields = given
            .into_iter()
            .enumerate()
            .map(|(index, written)| match written {
                Some(value) => Some(value),
                None => Some(Expr {
                    kind: ExprKind::Name(binders.get(index)?.clone()),
                    ty: declared.get(index).map(|(_, ty)| ty.clone())?,
                    span,
                }),
            })
            .collect::<Option<Vec<_>>>()?;
        let result = Type::Nominal(id.clone(), type_arguments.clone());
        Some(Expr {
            kind: ExprKind::Match {
                scrutinee: Box::new(subject),
                arms: vec![CheckedArm {
                    pattern: Pattern::Constructor {
                        variant: 0,
                        fields: binders,
                    },
                    body: Expr {
                        kind: ExprKind::Construct {
                            id,
                            variant: 0,
                            arguments: type_arguments,
                            fields,
                        },
                        ty: result.clone(),
                        span,
                    },
                }],
            },
            ty: result,
            span,
        })
    }

    fn match_expression(&mut self, node: &SyntaxNode, expected: Option<&Type>, tail: bool) -> Option<Expr> {
        let scrutinee_node = child_of(node, is_expr_node)?;
        let scrutinee = self.check(&scrutinee_node, None)?;
        let mut coverage = IndexSet::new();
        let mut catch_all = false;
        let mut result = expected.cloned();
        // A `?` in the first arm needs a type to constrain before that arm has
        // been checked, and an unannotated match in answer position has none
        // yet. The variable is what the arm decides anyway, one step earlier.
        if tail && result.is_none() {
            result = Some(self.unifier.fresh(Kind::Ordinary));
        }
        let mut arms = Vec::new();
        for arm in node.children().filter(|child| child.kind() == SyntaxKind::MatchArm) {
            let pattern_node = arm.children().find(|child| child.kind() == SyntaxKind::Pattern)?;
            let (pattern, covered, bindings) = self.check_pattern(&pattern_node, &scrutinee.ty)?;
            if catch_all
                || uncovered(self.world, &scrutinee.ty, &coverage).is_none()
                || !coverage.insert(covered.clone())
            {
                self.resolver.report(
                    Diagnostic::error(Code::UnreachablePattern, "this match arm can never be selected")
                        .at(crate::resolve::trimmed_span(&pattern_node), "already covered above"),
                );
                self.failed = true;
            }
            catch_all |= covered == Coverage::CatchAll;
            let saved = self.locals.clone();
            self.locals
                .extend(bindings.into_iter().map(|(name, ty)| (name, Scheme::monomorphic(ty))));
            let body_node = child_of(&arm, is_expr_node)?;
            let body = self.branch(&body_node, result.as_ref(), tail);
            self.locals = saved;
            let body = body?;
            if result.is_none() {
                result = Some(body.ty.clone());
            }
            arms.push(CheckedArm { pattern, body });
        }
        if let Some(missing) = uncovered(self.world, &scrutinee.ty, &coverage) {
            self.resolver.report(
                Diagnostic::error(Code::NonExhaustiveMatch, "this match leaves a possible value uncovered")
                    .at(crate::resolve::trimmed_span(node), format!("add `{missing}`"))
                    .note(format!(
                        "the matched value has type `{}`",
                        crate::infer::plain_one(&scrutinee.ty)
                    )),
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
            // A constructor is asked about before a plain binding, because a
            // pattern that names one means that constructor: reading
            // `Silence` as a name that matches anything would silently make
            // the arm below it unreachable.
            if let Type::Nominal(id, arguments) = target
                && let Some(variant) = self.world.constructor_of(id, first.text())
            {
                let declared = self.world.fields(id, variant, arguments);
                let names: Vec<String> = tokens
                    .iter()
                    .skip(1)
                    .filter(|token| token.kind() == SyntaxKind::Identifier)
                    .map(|token| token.text().to_owned())
                    .collect();
                if names.len() != declared.len() {
                    return self.pattern_type_error(
                        span,
                        target,
                        &format!(
                            "`{}` binds {} field{}",
                            first.text(),
                            declared.len(),
                            if declared.len() == 1 { "" } else { "s" }
                        ),
                    );
                }
                for (name, (_, ty)) in names.iter().zip(&declared) {
                    if bindings.insert(name.clone(), ty.clone()).is_some() {
                        self.resolver.report(
                            Diagnostic::error(Code::DuplicateName, format!("pattern binding `{name}` is repeated"))
                                .at(span, "bind each field once"),
                        );
                        self.failed = true;
                        return None;
                    }
                }
                return Some((
                    Pattern::Constructor { variant, fields: names },
                    Coverage::Constructor(variant),
                    bindings,
                ));
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
        if matches!(first.kind(), SyntaxKind::OkKw | SyntaxKind::ErrKw) {
            let Type::Sum(value, error) = target else {
                return self.pattern_type_error(span, target, "`Ok` and `Err` need a result");
            };
            let wanted = first.kind() == SyntaxKind::ErrKw;
            let name = tokens
                .iter()
                .find(|token| token.kind() == SyntaxKind::Identifier)
                .map(|token| token.text().to_owned())?;
            let held = if wanted { error } else { value };
            bindings.insert(name.clone(), held.as_ref().clone());
            return Some(if wanted {
                (Pattern::Err(name), Coverage::Err, bindings)
            } else {
                (Pattern::Ok(name), Coverage::Ok, bindings)
            });
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
            | Value::Text(_)
            | Value::Duration(..)
            | Value::Position(..)
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
            | Value::Sum { .. }
            | Value::Option { .. }
            | Value::List { .. }
            | Value::Data { .. }
            | Value::Music(_)
            | Value::Closure(_)
            | Value::Primitive { .. }
            | Value::Machine { .. }
            | Value::Syntax(_)
            | Value::NodePath(_)
            | Value::BindingPath(_)
            | Value::SyntaxStep(_)
            | Value::Builtin(_) => Coverage::Literal(literal_key(&value)),
        };
        Some((Pattern::Literal(value), covered, bindings))
    }

    fn pattern_literal(&mut self, token: &SyntaxToken, target: &Type, span: SourceSpan) -> Option<Value> {
        let value = match (token.kind(), target) {
            (SyntaxKind::TrueKw, Type::Bool) => Value::Bool(true),
            (SyntaxKind::FalseKw, Type::Bool) => Value::Bool(false),
            (SyntaxKind::Integer, Type::Nat) => Value::Nat(parse_u64(self.resolver, token)?),
            (SyntaxKind::Integer, Type::Duration(coordinate)) => {
                Value::Duration(*coordinate, Ratio::from_integer(parse_i64(self.resolver, token)?))
            }
            (SyntaxKind::String, Type::Text) => Value::Text(musa_language::ast::unquote(token.text())),
            (SyntaxKind::Rational, Type::Ratio) => Value::Ratio(parse_ratio(self.resolver, token)?),
            (SyntaxKind::Rational, Type::Duration(coordinate)) => {
                Value::Duration(*coordinate, parse_ratio(self.resolver, token)?)
            }
            (SyntaxKind::PitchLiteral, Type::Pitch) => Value::Pitch(WrittenPitch::parse(token.text())?),
            (SyntaxKind::IntervalLiteral, Type::Interval) => Value::Interval(Interval::parse(token.text(), false)?),
            _ => return self.pattern_type_error(span, target, "this literal cannot match that value type"),
        };
        Some(value)
    }

    fn pattern_type_error<T>(&mut self, span: SourceSpan, target: &Type, message: &str) -> Option<T> {
        self.resolver.report(Diagnostic::error(Code::TypeMismatch, message).at(
            span,
            format!("the matched value has type `{}`", crate::infer::plain_one(target)),
        ));
        self.failed = true;
        None
    }

    fn application(&mut self, node: &SyntaxNode, expected: Option<&Type>) -> Option<Expr> {
        let mut children = node.children();
        let function_node = children.find(|child| is_expr_node(child.kind()))?;
        if let Some(builtin) = name_of(&function_node).as_deref().and_then(Builtin::named)
            && !builtin.is_track()
        {
            return self.builtin_application(node, builtin, expected);
        }
        // Only where the phase environment is in scope. Ordinary source reads
        // names through the line above and through `self.symbols`, neither of
        // which knows a syntax operation exists.
        if self.reading == Reading::Expansion
            && let Some(operation) = name_of(&function_node).as_deref().and_then(SyntaxOp::named)
        {
            return self.syntax_application(node, operation, expected);
        }
        if let Some(name) = name_of(&function_node) {
            if self.world.is_constructor(&name) {
                return self.construct(&name, node, &raw_arguments(node), crate::resolve::trimmed_span(node));
            }
            if self.world.is_fold(&name) {
                return self.fold_application(&name, node, expected);
            }
        }
        let function = self.check(&function_node, None)?;
        let raw_arguments = raw_arguments(node);
        let mut callee = self.unifier.resolve(&function.ty);
        // The thing being called may not have been decided yet: a parameter
        // used as a function, or a declaration whose own type this call helps
        // settle. What the call knows is the arity, so that is what it says —
        // one fresh variable per argument, and one for the result.
        if matches!(callee, Type::Var(_)) {
            let parameters: Vec<Type> = raw_arguments
                .iter()
                .map(|_| self.unifier.fresh(Kind::Ordinary))
                .collect();
            let result = self.unifier.fresh(Kind::Ordinary);
            let callable = Type::Function(parameters, Box::new(result));
            self.reconcile(&callable, &function.ty, function.span)?;
            callee = self.unifier.resolve(&function.ty);
        }
        let Type::Function(parameter_types, result) = callee.clone() else {
            self.resolver.report(
                Diagnostic::error(Code::TypeMismatch, format!("`{callee}` is not callable"))
                    .at(function.span, "this is a value, not a function"),
            );
            self.failed = true;
            return None;
        };
        let parameter_names = self.parameter_names(&function_node, &parameter_types);
        let mut occupied = IndexSet::new();
        let mut positional = 0usize;
        let mut arguments = Vec::with_capacity(raw_arguments.len());
        for argument in &raw_arguments {
            let named = argument_name(argument);
            let parameter = if let Some(named) = named {
                let Some(index) = parameter_names
                    .iter()
                    .position(|parameter| parameter.as_deref() == Some(&named))
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
        // A call supplies every parameter. What an under-applied call would
        // otherwise mean is a function of the ones left out, and that is the
        // one value the language will not let an argument list produce by
        // accident (`docs/rules/constitution.md` §9).
        let missing: Vec<String> = parameter_names
            .iter()
            .enumerate()
            .filter(|(index, _)| !occupied.contains(index))
            .map(|(index, name)| {
                name.clone()
                    .unwrap_or_else(|| format!("argument {}", index.saturating_add(1)))
            })
            .collect();
        if !missing.is_empty() {
            let names = missing.join(", ");
            let legacy = name_of(&function_node)
                .and_then(|name| self.symbols.get(&name))
                .and_then(|symbol| self.definitions.get(symbol.definition))
                .and_then(|definition| definition.role.as_ref());
            let message = match legacy {
                Some(role) => format!("{} `{}` needs a value for `{names}`", role.material.word(), role.name),
                None => format!("this call supplies no value for `{names}`"),
            };
            self.resolver.report(
                Diagnostic::error(Code::WrongArity, message)
                    .at(crate::resolve::trimmed_span(node), "not enough arguments")
                    .note("a call supplies every declared parameter, so there is no value a call with one missing could have")
                    .help("write the argument here, or declare a function that takes the ones you have"),
            );
            self.failed = true;
            return None;
        }
        Some(Expr {
            kind: ExprKind::Apply {
                function: Box::new(function),
                arguments,
            },
            ty: result.as_ref().clone(),
            span: crate::resolve::trimmed_span(node),
        })
    }

    fn builtin_application(&mut self, node: &SyntaxNode, builtin: Builtin, expected: Option<&Type>) -> Option<Expr> {
        let span = crate::resolve::trimmed_span(node);
        let raw = raw_arguments(node);
        if raw.iter().any(|argument| argument_name(argument).is_some()) {
            self.resolver.report(
                Diagnostic::error(
                    Code::WrongArity,
                    format!("`{}` uses positional arguments", builtin.name()),
                )
                .at(span, "named arguments are not part of this prelude operation"),
            );
            self.failed = true;
            return None;
        }
        let wanted = match builtin.family()? {
            Family::Delta { arguments, .. } => arguments.len(),
            Family::Eliminator(eliminator) => eliminator.arity(),
            // Unreached: the caller routes a track builtin to the general application path, where
            // its arrow type is what the arguments are checked against.
            Family::Track => builtin.parameters()?.len(),
            Family::Machine(operation) => operation.arity(),
        };
        if raw.len() != wanted {
            self.resolver.report(
                Diagnostic::error(
                    Code::WrongArity,
                    format!("`{}` takes {wanted} arguments, found {}", builtin.name(), raw.len()),
                )
                .at(span, "wrong number of arguments"),
            );
            self.failed = true;
            return None;
        }
        if self.meter.instantiate(builtin.name(), span).is_none() {
            self.failed = true;
            return None;
        }
        let nodes: Vec<_> = raw
            .iter()
            .filter_map(|argument| child_of(argument, is_expr_node))
            .collect();
        let family = builtin.family()?;
        if let Family::Delta { arguments, result } = family {
            let mut checked = Vec::with_capacity(arguments.len());
            for (index, shape) in arguments.iter().enumerate() {
                checked.push(self.check(nodes.get(index)?, Some(&shape.ty()))?);
            }
            return Some(Expr {
                kind: ExprKind::Builtin {
                    builtin,
                    arguments: checked,
                },
                ty: result.ty(),
                span,
            });
        }
        // The one machine builtin whose type is not written down: which unit
        // `primitive` makes is decided by the name and version it is applied
        // to, so the registry answers where a scheme would otherwise be.
        if family == Family::Machine(MachineOp::Primitive) {
            return self.registered_instance(&nodes, span, expected);
        }
        let scheme = match family {
            Family::Eliminator(eliminator) => eliminator.instantiate(self.unifier),
            Family::Machine(operation) => operation.instantiate(self.unifier)?,
            Family::Delta { .. } | Family::Track => return None,
        };
        self.applied_scheme(builtin, scheme, &nodes, span, expected)
    }

    /// Check a call against a declared rank-1 scheme.
    ///
    /// One unification in place of seven hand-written checks. The result is
    /// unified with what the position wants *before* the arguments are read, so
    /// that `nat_fold(0, step, n)` in a `List<Nat>` position complains about
    /// the zero rather than about the whole call.
    ///
    /// Shared by the eliminators, the machine builtins, and the phase-local
    /// syntax operations, which is one of the two places "there is no second
    /// checker" stops being a claim and starts being a fact about the code.
    fn applied_scheme(
        &mut self,
        builtin: Builtin,
        scheme: Type,
        nodes: &[SyntaxNode],
        span: SourceSpan,
        expected: Option<&Type>,
    ) -> Option<Expr> {
        let (parameters, result) = if let Type::Function(parameters, result) = scheme {
            (parameters, *result)
        } else {
            // A machine constant written with an empty argument list. `identity`
            // *is* a machine rather than a function to one, so `identity()` is
            // it, applied to nothing.
            (Vec::new(), scheme)
        };
        if let Some(expected) = expected {
            self.reconcile(expected, &result, span)?;
        }
        let mut arguments = Vec::with_capacity(parameters.len());
        for (index, parameter) in parameters.iter().enumerate() {
            arguments.push(self.check(nodes.get(index)?, Some(parameter))?);
        }
        let ty = self.unifier.resolve(&result);
        Some(Expr {
            kind: ExprKind::Builtin { builtin, arguments },
            ty,
            span,
        })
    }

    /// Check one use of a phase-local syntax operation.
    ///
    /// The same arity check, the same meter, and the same
    /// [`Self::applied_scheme`] the eliminators and the machine builtins go
    /// through. What is different is only which registry the name came from.
    fn syntax_application(&mut self, node: &SyntaxNode, operation: SyntaxOp, expected: Option<&Type>) -> Option<Expr> {
        let span = crate::resolve::trimmed_span(node);
        let raw = raw_arguments(node);
        let spelling = operation.spelling();
        if raw.iter().any(|argument| argument_name(argument).is_some()) {
            self.resolver.report(
                Diagnostic::error(Code::WrongArity, format!("`{spelling}` uses positional arguments"))
                    .at(span, "named arguments are not part of this phase operation"),
            );
            self.failed = true;
            return None;
        }
        let wanted = operation.arity();
        if raw.len() != wanted {
            self.resolver.report(
                Diagnostic::error(
                    Code::WrongArity,
                    format!("`{spelling}` takes {wanted} arguments, found {}", raw.len()),
                )
                .at(span, "wrong number of arguments"),
            );
            self.failed = true;
            return None;
        }
        if self.meter.instantiate(spelling, span).is_none() {
            self.failed = true;
            return None;
        }
        let nodes: Vec<_> = raw
            .iter()
            .filter_map(|argument| child_of(argument, is_expr_node))
            .collect();
        let scheme = operation.instantiate(self.unifier);
        self.applied_scheme(Builtin::Syntax(operation), scheme, &nodes, span, expected)
    }

    /// `primitive(name, version, configuration)` — one instance of a registered
    /// unit (`../across-stages/03-machine-calculus.md` §1).
    ///
    /// The name and version have to be *written*, not computed. A pair
    /// `(name, version)` selects exactly one state layout, configuration
    /// codec, start function, and step function, and a compiler that could not
    /// say which unit an expression named could not check its ports either —
    /// so this is the one place a builtin's argument is read as a literal
    /// rather than as a value of the right type.
    fn registered_instance(&mut self, nodes: &[SyntaxNode], span: SourceSpan, expected: Option<&Type>) -> Option<Expr> {
        let name = self.check(nodes.first()?, Some(&Type::Text))?;
        let version = self.check(nodes.get(1)?, Some(&Type::Nat))?;
        let (ExprKind::Literal(Value::Text(id)), ExprKind::Literal(Value::Nat(number))) = (&name.kind, &version.kind)
        else {
            self.resolver.report(
                Diagnostic::error(
                    Code::TypeMismatch,
                    "a registered unit is named by a written name and version",
                )
                .at(span, "the name or the version is computed here")
                .help("write both out: which unit this is decides its ports, and that has to be known while the piece is being checked"),
            );
            self.failed = true;
            return None;
        };
        let Some(descriptor) = u32::try_from(*number)
            .ok()
            .and_then(|number| crate::machine::descriptor(id, number))
        else {
            let versions = crate::machine::versions_of(id);
            let help = if versions.is_empty() {
                let registered = crate::machine::registered_ids();
                if registered.is_empty() {
                    "this build registers no units at all".to_owned()
                } else {
                    format!(
                        "this build registers {}",
                        registered
                            .iter()
                            .map(|name| format!("`{name}`"))
                            .collect::<Vec<_>>()
                            .join(", ")
                    )
                }
            } else {
                format!(
                    "`{id}` is registered at version {}",
                    versions.iter().map(u32::to_string).collect::<Vec<_>>().join(", ")
                )
            };
            self.resolver.report(
                Diagnostic::error(
                    Code::UnknownName,
                    format!("no registered unit `{id}` at version {number}"),
                )
                .at(span, "named here")
                .help(help),
            );
            self.failed = true;
            return None;
        };
        let configuration = self.check(nodes.get(2)?, Some(&descriptor.configuration().ty()))?;
        let ty = Type::Primitive {
            step: Box::new(Type::Step(descriptor.step())),
            input: Box::new(descriptor.input().ty()),
            output: Box::new(descriptor.output().ty()),
        };
        if let Some(expected) = expected {
            self.reconcile(expected, &ty, span)?;
        }
        Some(Expr {
            kind: ExprKind::Builtin {
                builtin: Builtin::Primitive,
                arguments: vec![name, version, configuration],
            },
            ty,
            span,
        })
    }

    /// Check one anonymous function.
    ///
    /// A declaration would have been checked by the same three steps —
    /// parameters into scope, body against the result, arrow out — and the
    /// only thing this adds is the capture list, because a declaration is
    /// written where its free names are declarations and a lambda is written
    /// where they may be someone's parameter.
    ///
    /// The expected type is read for the parameters and the result before the
    /// body is checked, so `map(fn (root) { schema_triad(collection, root) },
    /// roots)` needs no annotation: the position already says what `root` is.
    fn lambda(&mut self, node: &SyntaxNode, expected: Option<&Type>) -> Option<Expr> {
        let span = crate::resolve::trimmed_span(node);
        let (wanted_parameters, wanted_result) = match expected {
            Some(Type::Function(parameters, result)) => (parameters.clone(), Some(result.as_ref().clone())),
            _ => (Vec::new(), None),
        };
        let scope = self.world.scope();
        let written = musa_language::ast::LambdaExpr::cast(node.clone())?;
        let mut parameters = Vec::new();
        for (index, parameter) in written.params().iter().enumerate() {
            let ty = match child_of(parameter.syntax(), is_type_node) {
                Some(node) => parse_type(self.resolver, &scope, &node)?,
                None => wanted_parameters
                    .get(index)
                    .cloned()
                    .unwrap_or_else(|| self.unifier.fresh(Kind::Ordinary)),
            };
            parameters.push(CheckedParameter {
                name: parameter.name().unwrap_or_default(),
                ty,
            });
        }
        // A parameter's type sits inside the parameter list, so the one type
        // node written as a direct child is the result — the same reading a
        // declaration gets.
        let result = match node.children().find(|child| is_type_node(child.kind())) {
            Some(node) => parse_type(self.resolver, &scope, &node)?,
            None => wanted_result.unwrap_or_else(|| self.unifier.fresh(Kind::Ordinary)),
        };
        let body_node = child_of(node, is_expr_node)?;
        let saved = self.locals.clone();
        for parameter in &parameters {
            self.locals
                .insert(parameter.name.clone(), Scheme::monomorphic(parameter.ty.clone()));
        }
        self.mentioned.push(IndexSet::new());
        // An anonymous function is a function: a `?` in its body leaves *it*,
        // not whatever it was written inside of.
        let body = self.answering(&body_node, Some(&result));
        let read = self.mentioned.pop().unwrap_or_default();
        self.locals = saved;
        let body = body?;
        let captures = read
            .into_iter()
            .filter(|name| !parameters.iter().any(|parameter| &parameter.name == name))
            .collect();
        let ty = Type::Function(
            parameters.iter().map(|parameter| parameter.ty.clone()).collect(),
            Box::new(result.clone()),
        );
        Some(Expr {
            kind: ExprKind::Lambda {
                parameters,
                result,
                captures,
                body: Box::new(body),
            },
            ty,
            span,
        })
    }

    /// The name of each parameter, where the call can see one.
    ///
    /// A declaration lends its parameter names, which is what makes a named
    /// argument and a missing-argument diagnostic possible. Anything else —
    /// a parameter used as a function, a value of arrow type — has arity and
    /// no names, so the positions answer `None` and are reported by number.
    fn parameter_names(&self, function: &SyntaxNode, types: &[Type]) -> Vec<Option<String>> {
        let global = name_of(function)
            .and_then(|name| self.symbols.get(&name))
            .and_then(|symbol| self.definitions.get(symbol.definition));
        match global.map(|definition| &definition.kind) {
            Some(RawDefinitionKind::Function { parameters, .. } | RawDefinitionKind::Music { parameters, .. }) => {
                parameters
                    .iter()
                    .map(|parameter| Some(parameter.name.clone()))
                    .collect()
            }
            _ => types.iter().map(|_| None).collect(),
        }
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

/// The case a match has not covered, spelled the way an arm would spell it.
///
/// `None` means the match is exhaustive, so this is what both the
/// unreachable-arm check and the non-exhaustiveness diagnostic read: one
/// place decides what "covered" means, and the diagnostic can name what is
/// missing instead of asking the writer to work it out.
///
/// A type whose values are literals rather than constructors — a natural, a
/// pitch, a text — cannot be covered by naming them all
/// (`docs/rules/language/02-core-calculus.md` §5.6), so what is missing
/// there is the fallback itself.
fn uncovered(world: &World, target: &Type, coverage: &IndexSet<Coverage>) -> Option<String> {
    fn missing(coverage: &IndexSet<Coverage>, case: &Coverage, spelling: &str) -> Option<String> {
        (!coverage.contains(case)).then(|| spelling.to_owned())
    }
    if coverage.contains(&Coverage::CatchAll) {
        return Option::None;
    }
    match target {
        Type::Bool => {
            missing(coverage, &Coverage::True, "true").or_else(|| missing(coverage, &Coverage::False, "false"))
        }
        Type::Option(_) => {
            missing(coverage, &Coverage::None, "None").or_else(|| missing(coverage, &Coverage::Some, "Some(value)"))
        }
        Type::Sum(_, _) => {
            missing(coverage, &Coverage::Ok, "Ok(value)").or_else(|| missing(coverage, &Coverage::Err, "Err(reason)"))
        }
        // A declaration's constructors are what the declaration wrote, so what
        // is missing is named the way the arm would have to be written — with
        // one binding per field, since a constructor pattern binds by
        // position.
        Type::Nominal(id, _) => world
            .variants(id)
            .into_iter()
            .enumerate()
            .find_map(|(index, (name, fields))| {
                let bindings = (0..fields)
                    .map(|field| format!("field{}", field.saturating_add(1)))
                    .collect::<Vec<_>>()
                    .join(", ");
                let spelling = if fields == 0 {
                    name
                } else {
                    format!("{name}({bindings})")
                };
                missing(coverage, &Coverage::Constructor(index), &spelling)
            }),
        Type::List(_) => missing(coverage, &Coverage::EmptyList, "[]")
            .or_else(|| missing(coverage, &Coverage::Cons, "[head, ..tail]")),
        Type::Var(_)
        | Type::Unit
        | Type::Nat
        | Type::Ratio
        | Type::Text
        | Type::Duration(_)
        | Type::Position(_)
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
        | Type::Step(_)
        | Type::Primitive { .. }
        | Type::Machine { .. }
        | Type::Product(_)
        | Type::Syntax
        | Type::NodePath
        | Type::BindingPath
        | Type::SyntaxStep { .. }
        | Type::Function(_, _) => Some("_".to_owned()),
    }
}

fn literal_key(value: &Value) -> String {
    match value {
        Value::Bool(value) => format!("bool:{value}"),
        Value::Nat(value) => format!("nat:{value}"),
        Value::Ratio(value) => format!("ratio:{}/{}", value.numer(), value.denom()),
        // The quoted form, not the raw text: `quote` is text's exact
        // encoding, so two texts have one key exactly when they are one
        // text, and no text can spell another value's key.
        Value::Text(value) => format!("text:{}", musa_language::ast::quote(value)),
        Value::Duration(coordinate, value) => {
            format!("duration:{}:{}/{}", coordinate.spelling(), value.numer(), value.denom())
        }
        Value::Position(coordinate, value) => {
            format!("position:{}:{}/{}", coordinate.spelling(), value.numer(), value.denom())
        }
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
        | Value::Sum { .. }
        | Value::Option { .. }
        | Value::List { .. }
        | Value::Data { .. }
        | Value::Music(_)
        | Value::Closure(_)
        | Value::Primitive { .. }
        | Value::Machine { .. }
        | Value::Syntax(_)
        | Value::NodePath(_)
        | Value::BindingPath(_)
        | Value::SyntaxStep(_)
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

/// Evaluate the declaration graph in dependency order, as one configuration.
///
/// `run` is where the two language outcomes are told apart from the third that
/// is not one: `failed` is reported as the resource rejection it is, and
/// `Broken` is a compiler invariant failure, because a closed well-typed term
/// always steps (research `core-calculus/06-proof-outline.md` Theorem 2.3).
fn evaluate(
    resolver: &mut Resolver,
    definitions: &[CheckedDefinition],
    order: &[usize],
    meter: &mut WorkMeter,
) -> Option<IndexMap<String, Value>> {
    let mut broken = None;
    let outcome = meter.run(|meter| {
        let mut values = IndexMap::new();
        for index in order {
            let definition = definitions.get(*index)?;
            meter.output("scalar elaboration", 0, definition.span)?;
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
            }?;
            // Preservation (Theorem 2.2), checked rather than assumed: a value
            // whose type left its declaration's is a compiler fault, and the
            // one thing this must not do is publish it.
            if !crate::infer::admits(&definition.ty, &value.ty()) {
                broken = Some((definition.span, "the preservation invariant failed here"));
                return None;
            }
            let _normalization_witness = value.normalization_witness();
            values.insert(definition.name.clone(), value);
        }
        Some(values)
    });
    match outcome {
        Evaluation::Done(values) => Some(values),
        Evaluation::Failed(failure) => {
            report_resource_error(resolver, failure);
            None
        }
        Evaluation::Broken => {
            let (span, label) = broken.unwrap_or_else(|| {
                (
                    definitions.first().map_or_else(SourceSpan::default, |first| first.span),
                    "evaluation stopped here",
                )
            });
            resolver.report(
                Diagnostic::error(Code::TypeMismatch, "this checked expression could not be evaluated")
                    .at(span, label)
                    .note("this is a compiler invariant failure, not a recoverable language effect"),
            );
            None
        }
    }
}

fn eval(expression: &Expr, environment: &IndexMap<String, Value>, meter: &mut WorkMeter) -> Option<Value> {
    meter.step(Reduction::Expression, 1, expression.span)?;
    let value = match &expression.kind {
        ExprKind::Literal(value) => Some(value.clone()),
        ExprKind::Name(name) => environment.get(name).cloned(),
        // The closure a named `fn` becomes, built here instead of at the
        // declaration loop: what it closes over is whatever those names stand
        // for *at this point*, which is the whole difference between a
        // function written beside a value and one written beside a parameter.
        ExprKind::Lambda {
            parameters,
            result,
            captures,
            body,
        } => Some(Value::Closure(Box::new(Closure {
            parameters: parameters.clone(),
            result: result.clone(),
            body: body.as_ref().clone(),
            captures: captures
                .iter()
                .filter_map(|name| Some((name.clone(), environment.get(name)?.clone())))
                .collect(),
        }))),
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
        ExprKind::Injection {
            error,
            held,
            value_type,
            error_type,
        } => Some(Value::Sum {
            value_type: value_type.clone(),
            error_type: error_type.clone(),
            error: *error,
            held: Box::new(eval(held, environment, meter)?),
        }),
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
        ExprKind::Construct {
            id,
            variant,
            arguments,
            fields,
        } => {
            let fields = fields
                .iter()
                .map(|field| eval(field, environment, meter))
                .collect::<Option<Vec<_>>>()?;
            Some(Value::Data {
                id: id.clone(),
                arguments: arguments.clone(),
                variant: *variant,
                fields,
            })
        }
        // A fold is structural recursion over a finite value, so it is
        // written as recursion here: each field that is itself a group member
        // is folded first, and the case is applied to what came back. The
        // value is finite, so this terminates — which is the whole reason a
        // declaration has to be finite.
        ExprKind::Fold { cases, shape, value } => {
            let cases = cases
                .iter()
                .map(|case| eval(case, environment, meter))
                .collect::<Option<Vec<_>>>()?;
            let value = eval(value, environment, meter)?;
            fold_value(&cases, shape, &value, meter, expression.span)
        }
        ExprKind::Apply { function, arguments } => {
            let function = eval(function, environment, meter)?;
            match function {
                Value::Closure(closure) => {
                    let provided = supplied(closure.parameters.len(), arguments, environment, meter)?;
                    apply_closure(&closure, provided, meter, expression.span)
                }
                Value::Builtin(builtin) => {
                    let provided = supplied(builtin.parameters()?.len(), arguments, environment, meter)?;
                    apply_builtin(builtin, provided, expression.span)
                }
                Value::Primitive { .. }
                | Value::Machine { .. }
                | Value::Bool(_)
                | Value::Nat(_)
                | Value::Ratio(_)
                | Value::Text(_)
                | Value::Duration(..)
                | Value::Position(..)
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
                | Value::Sum { .. }
                | Value::Option { .. }
                | Value::List { .. }
                | Value::Data { .. }
                | Value::Syntax(_)
                | Value::NodePath(_)
                | Value::BindingPath(_)
                | Value::SyntaxStep(_)
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
                | Value::Text(_)
                | Value::Duration(..)
                | Value::Position(..)
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
                | Value::Sum { .. }
                | Value::Option { .. }
                | Value::List { .. }
                | Value::Data { .. }
                | Value::Music(_)
                | Value::Closure(_)
                | Value::Primitive { .. }
                | Value::Machine { .. }
                | Value::Syntax(_)
                | Value::NodePath(_)
                | Value::BindingPath(_)
                | Value::SyntaxStep(_)
                | Value::Builtin(_) => None,
            }
        }
        ExprKind::Builtin { builtin, arguments } => eval_builtin(*builtin, arguments, environment, meter, expression),
        ExprKind::Match { scrutinee, arms } => {
            let value = eval(scrutinee, environment, meter)?;
            let mut selected = None;
            for arm in arms {
                meter.step(Reduction::MatchArm, 1, expression.span)?;
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
                    Value::Duration(_, duration) => {
                        crate::resolve::BoundValue::Duration(crate::score::NotatedDuration::spelled(*duration))
                    }
                    Value::Bool(_)
                    | Value::Nat(_)
                    | Value::Ratio(_)
                    | Value::Position(..)
                    | Value::Text(_)
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
                    | Value::Sum { .. }
                    | Value::Option { .. }
                    | Value::List { .. }
                    | Value::Data { .. }
                    | Value::Music(_)
                    | Value::Closure(_)
                    | Value::Primitive { .. }
                    | Value::Machine { .. }
                    | Value::Syntax(_)
                    | Value::NodePath(_)
                    | Value::BindingPath(_)
                    | Value::SyntaxStep(_)
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
    let (nodes, bytes) = charged_shape(&expression.kind, &value);
    // A polymorphic function's body has the *declaration's* type, which is a
    // variable; the value flowing through it is whatever the caller chose.
    // `admits` is that distinction, and it is equality everywhere else.
    if !crate::infer::admits(&expression.ty, &value.ty()) {
        return None;
    }
    meter.construct("expression value", nodes, bytes, expression.span)?;
    Some(value)
}

/// Evaluate a note's pitch expression as far as the ambient scale allows.
fn pitch_term(expression: &Expr, environment: &IndexMap<String, Value>, meter: &mut WorkMeter) -> Option<PitchTerm> {
    match &expression.kind {
        ExprKind::Step { base, steps, down } => {
            meter.step(Reduction::ScaleStep, 1, expression.span)?;
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
        | ExprKind::Injection { .. }
        | ExprKind::Name(_)
        | ExprKind::Product(_)
        | ExprKind::Option(_)
        | ExprKind::List(_)
        | ExprKind::Apply { .. }
        | ExprKind::Lambda { .. }
        | ExprKind::Builtin { .. }
        | ExprKind::Match { .. }
        | ExprKind::Construct { .. }
        | ExprKind::Fold { .. }
        | ExprKind::Music(_)
        | ExprKind::KernelQuote(_) => {
            let Value::Pitch(pitch) = eval(expression, environment, meter)? else {
                return None;
            };
            Some(PitchTerm::Written(pitch))
        }
    }
}

/// One call's arguments, evaluated in the order they are written and read
/// back in parameter order.
///
/// A call supplies every parameter, so an empty slot here is not a partial
/// application waiting for the rest — it is a checker invariant that failed,
/// and the caller has no value to return.
fn supplied(
    parameters: usize,
    arguments: &[CallArgument],
    environment: &IndexMap<String, Value>,
    meter: &mut WorkMeter,
) -> Option<Vec<Value>> {
    let mut provided: Vec<Option<Value>> = vec![None; parameters];
    for argument in arguments {
        let slot = provided.get_mut(argument.parameter)?;
        *slot = Some(eval(&argument.value, environment, meter)?);
    }
    provided.into_iter().collect()
}

fn apply_closure(closure: &Closure, provided: Vec<Value>, meter: &mut WorkMeter, span: SourceSpan) -> Option<Value> {
    meter.step(Reduction::Application, 1, span)?;
    if provided.len() != closure.parameters.len() {
        return None;
    }
    let mut local = closure.captures.clone();
    for (parameter, value) in closure.parameters.iter().zip(provided) {
        if !crate::infer::admits(&parameter.ty, &value.ty()) {
            return None;
        }
        local.insert(parameter.name.clone(), value);
    }
    eval(&closure.body, &local, meter)
}

/// Apply a builtin, which happens once and completely.
///
/// There is no partly-applied builtin to return: `transpose` names the
/// operation and `transpose(P8, line)` names its result, and there is no
/// third thing in between.
fn apply_builtin(builtin: Builtin, provided: Vec<Value>, span: SourceSpan) -> Option<Value> {
    if provided.len() != builtin.parameters()?.len() {
        return None;
    }
    let mut arguments = provided.into_iter();
    let operation = match builtin {
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
        Builtin::Together => MusicOperation::Together {
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
        // A δ-builtin or an eliminator is never a value, so it never arrives here to be
        // applied as one; `parameters` above has already declined it.
        Builtin::NatFold
        | Builtin::ListFoldFromStart
        | Builtin::ListFoldFromEnd
        | Builtin::OptionFold
        | Builtin::Map
        | Builtin::Filter
        | Builtin::Range
        | Builtin::Repeat
        | Builtin::RatioAdd
        | Builtin::RatioSub
        | Builtin::RatioMul
        | Builtin::RatioDiv
        | Builtin::RatioLess
        | Builtin::RatioEqual
        | Builtin::TextEqual
        | Builtin::NatAdd
        | Builtin::NatMul
        | Builtin::NatSub
        | Builtin::DurationOf
        | Builtin::DurationRatio
        | Builtin::DurationAdd
        | Builtin::DurationScale
        | Builtin::DurationLess
        | Builtin::DurationEqual
        | Builtin::PositionOf
        | Builtin::PositionRatio
        | Builtin::PositionShift
        | Builtin::PositionBetween
        | Builtin::PositionLess
        | Builtin::PositionEqual
        | Builtin::IntervalAdd
        | Builtin::IntervalInverse
        | Builtin::PitchClassOf
        | Builtin::SignatureScale
        | Builtin::ScaleOn
        | Builtin::ScaleTonic
        | Builtin::ScaleSize
        | Builtin::ScalePitch
        | Builtin::ScaleClass
        | Builtin::ScaleChord
        | Builtin::PitchFrame
        | Builtin::FrameScale
        | Builtin::FrameTonic
        | Builtin::FramePitch
        | Builtin::DegreeOf
        | Builtin::DegreeStepUp
        | Builtin::DegreeStepDown
        | Builtin::DegreeRaised
        | Builtin::DegreeLowered
        | Builtin::ChordOn
        | Builtin::ChordRoot
        | Builtin::ChordBass
        | Builtin::ChordMembers
        | Builtin::ChordInversion
        | Builtin::ChordOver
        | Builtin::ChordTriad
        | Builtin::TriadChord
        | Builtin::TriadMajor
        | Builtin::RomanOf
        | Builtin::RomanOrdinal
        | Builtin::RomanSize
        | Builtin::RomanInversion
        | Builtin::VoicingOf
        | Builtin::VoicingPitches
        | Builtin::VoicingBass
        | Builtin::VoicingChord
        | Builtin::VoicingPosition
        | Builtin::CloseVoicing
        | Builtin::DropVoicing
        | Builtin::OmitVoicing
        | Builtin::Pc12Of
        | Builtin::Pc12Number
        | Builtin::Pc12Forget
        | Builtin::Pc12Transposed
        | Builtin::Pc12Inverted
        | Builtin::Pc12Spelled
        | Builtin::PcSet12Of
        | Builtin::PcSet12Members
        | Builtin::PcSet12Transposed
        | Builtin::PcSet12Inverted
        | Builtin::PcSet12Normal
        | Builtin::PcSet12Prime
        | Builtin::PcSet12Vector
        | Builtin::Row12Of
        | Builtin::Row12Pcs
        | Builtin::Row12Head
        | Builtin::Row12Transposed
        | Builtin::Row12Inverted
        | Builtin::Row12Retrograde
        | Builtin::Row12Matrix
        | Builtin::Row12Forms
        | Builtin::Row12Symmetries
        | Builtin::Row12Repeats
        | Builtin::Row12Missing
        | Builtin::Primitive
        | Builtin::Machine
        | Builtin::Identity
        | Builtin::Connect
        | Builtin::Beside
        | Builtin::Feedback
        | Builtin::Copy
        | Builtin::Drop
        | Builtin::Swap
        | Builtin::Syntax(_) => return None,
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
    let Value::Duration(_, value) = value else { return None };
    Some(*value)
}

fn position_value(value: &Value) -> Option<Ratio<i64>> {
    let Value::Position(_, value) = value else { return None };
    Some(*value)
}

/// The greatest common divisor of two magnitudes, by Euclid.
///
/// Written out because reduction happens in `i128` here: a sum of two
/// representable rationals need not be representable, so the arithmetic is
/// done wide, reduced, and only then asked whether it fits.
fn greatest_common_divisor(mut left: u128, mut right: u128) -> u128 {
    while right != 0 {
        let remainder = left.checked_rem(right).unwrap_or(0);
        left = right;
        right = remainder;
    }
    left
}

/// A wide numerator and denominator as an exact `Ratio<i64>`, or nothing.
///
/// Nothing means the reduced value does not fit, which every caller turns into
/// a stated failure rather than a stuck term: D2 forbids partiality anywhere
/// but the result type.
fn exact_ratio(numerator: i128, denominator: i128) -> Option<Ratio<i64>> {
    if denominator == 0 {
        return None;
    }
    let divisor = i128::try_from(greatest_common_divisor(
        numerator.unsigned_abs(),
        denominator.unsigned_abs(),
    ))
    .ok()?;
    let divisor = if divisor == 0 { 1 } else { divisor };
    let (numerator, denominator) = (numerator.checked_div(divisor)?, denominator.checked_div(divisor)?);
    let (numerator, denominator) = if denominator < 0 {
        (numerator.checked_neg()?, denominator.checked_neg()?)
    } else {
        (numerator, denominator)
    };
    Some(Ratio::new(
        i64::try_from(numerator).ok()?,
        i64::try_from(denominator).ok()?,
    ))
}

/// The four exact operations, named apart from the builtins that offer them.
///
/// Separate because the same four are reached from six builtins — a duration
/// sum is a rational sum, a position shift is one too — and because a match on
/// [`Builtin`] here would have to name every operation that is *not* one of
/// these four.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Exact {
    Add,
    Sub,
    Mul,
    Div,
}

/// `left · right`, computed wide and reduced before it is asked whether it
/// fits.
fn exact_arithmetic(left: Ratio<i64>, right: Ratio<i64>, operation: Exact) -> Option<Ratio<i64>> {
    let (a, b) = (i128::from(*left.numer()), i128::from(*left.denom()));
    let (c, d) = (i128::from(*right.numer()), i128::from(*right.denom()));
    match operation {
        Exact::Add => exact_ratio(a.checked_mul(d)?.checked_add(c.checked_mul(b)?)?, b.checked_mul(d)?),
        Exact::Sub => exact_ratio(a.checked_mul(d)?.checked_sub(c.checked_mul(b)?)?, b.checked_mul(d)?),
        Exact::Mul => exact_ratio(a.checked_mul(c)?, b.checked_mul(d)?),
        Exact::Div => exact_ratio(a.checked_mul(d)?, b.checked_mul(c)?),
    }
}

/// One injection of a `Result<τ, Text>`, which is how every arithmetic builtin
/// that can refuse says so.
fn answered(value_type: Type, held: Value) -> Value {
    Value::Sum {
        value_type,
        error_type: Type::Text,
        error: false,
        held: Box::new(held),
    }
}

/// The other injection, carrying the operation's own sentence about what it
/// was handed and could not answer for.
fn refused(value_type: Type, because: &str) -> Value {
    Value::Sum {
        value_type,
        error_type: Type::Text,
        error: true,
        held: Box::new(Value::Text(because.to_owned())),
    }
}

/// The written-time duration a nonnegative exact rational names.
fn written_duration(value: Ratio<i64>) -> Value {
    let ty = Type::Duration(Coordinate::WrittenTime);
    if value < Ratio::ZERO {
        return refused(ty, "a duration is nonnegative, and this exact rational is below zero");
    }
    answered(ty, Value::Duration(Coordinate::WrittenTime, value))
}

/// One of the four exact rational operations, on already-evaluated arguments.
///
/// Division by zero and an unrepresentable reduced result are two different
/// refusals, and the point of `Result` over `Option` is that the composer is
/// told which one happened.
fn ratio_arithmetic(operation: Exact, values: &[Value]) -> Option<Value> {
    let (left, right) = (ratio_value(values.first()?)?, ratio_value(values.get(1)?)?);
    if operation == Exact::Div && right == Ratio::ZERO {
        return Some(refused(Type::Ratio, "an exact rational is not divided by zero"));
    }
    Some(match exact_arithmetic(left, right, operation) {
        Some(value) => answered(Type::Ratio, Value::Ratio(value)),
        None => refused(
            Type::Ratio,
            "these exact rationals have no result this language can represent",
        ),
    })
}

fn eval_builtin(
    builtin: Builtin,
    arguments: &[Expr],
    environment: &IndexMap<String, Value>,
    meter: &mut WorkMeter,
    expression: &Expr,
) -> Option<Value> {
    let values = arguments
        .iter()
        .map(|argument| eval(argument, environment, meter))
        .collect::<Option<Vec<_>>>()?;
    match builtin {
        Builtin::Syntax(operation) => eval_syntax(operation, arguments, &values, meter, expression),
        Builtin::Pc12Of => Some(Value::Pc12(crate::pc12::Pc12::from_number(nat_value(values.first()?)?))),
        Builtin::Pc12Number => {
            let Value::Pc12(member) = values.first()? else {
                return None;
            };
            Some(Value::Nat(u64::from(member.number())))
        }
        Builtin::Pc12Forget => {
            let Value::PitchClass(spelled) = values.first()? else {
                return None;
            };
            Some(Value::Pc12(crate::pc12::Pc12::forgetting(*spelled)))
        }
        Builtin::Pc12Transposed | Builtin::Pc12Inverted => {
            let Value::Pc12(member) = values.first()? else {
                return None;
            };
            let index = nat_value(values.get(1)?)?;
            Some(Value::Pc12(if builtin == Builtin::Pc12Transposed {
                member.transposed(index)
            } else {
                member.inverted(index)
            }))
        }
        Builtin::Pc12Spelled => {
            let (Value::Pc12(member), Value::Scale(collection)) = (values.first()?, values.get(1)?) else {
                return None;
            };
            Some(optional(
                Type::PitchClass,
                member.spelled(*collection).map(Value::PitchClass),
            ))
        }
        Builtin::PcSet12Of => Some(Value::PcSet12(crate::pc12::PcSet12::of(pc12_list(values.first()?)?))),
        Builtin::PcSet12Members | Builtin::PcSet12Normal => {
            let Value::PcSet12(set) = values.first()? else {
                return None;
            };
            let members: Vec<crate::pc12::Pc12> = if builtin == Builtin::PcSet12Members {
                set.members().collect()
            } else {
                set.normal_order()
            };
            Some(pc12_values(members))
        }
        Builtin::PcSet12Transposed | Builtin::PcSet12Inverted => {
            let Value::PcSet12(set) = values.first()? else {
                return None;
            };
            let index = nat_value(values.get(1)?)?;
            Some(Value::PcSet12(if builtin == Builtin::PcSet12Transposed {
                (*set).transposed(index)
            } else {
                (*set).inverted(index)
            }))
        }
        Builtin::PcSet12Prime => {
            let Value::PcSet12(set) = values.first()? else {
                return None;
            };
            Some(Value::PcSet12(set.prime_form()))
        }
        Builtin::PcSet12Vector => {
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
        // The one builtin that says *which* way it failed. A caller used to
        // learn that from `row12_repeats` and `row12_missing`, run again on
        // the same input; the reason now comes back with the refusal.
        Builtin::Row12Of => {
            let pcs = pc12_list(values.first()?)?;
            let held = match crate::pc12::Row12::checked(&pcs) {
                Some(row) => Value::Row12(row),
                None => Value::Product(vec![
                    Value::List {
                        member: Type::Nat,
                        values: crate::pc12::repeated_positions(&pcs)
                            .into_iter()
                            .map(Value::Nat)
                            .collect(),
                    },
                    pc12_values(crate::pc12::missing_classes(&pcs)),
                ]),
            };
            Some(Value::Sum {
                value_type: Type::Row12,
                error_type: ROW_FAULT.ty(),
                error: !matches!(held, Value::Row12(_)),
                held: Box::new(held),
            })
        }
        Builtin::Row12Pcs => {
            let Value::Row12(row) = values.first()? else {
                return None;
            };
            Some(pc12_values(row.pcs().collect()))
        }
        Builtin::Row12Head => {
            let Value::Row12(row) = values.first()? else {
                return None;
            };
            Some(Value::Pc12(row.head()))
        }
        Builtin::Row12Transposed | Builtin::Row12Inverted => {
            let Value::Row12(row) = values.first()? else {
                return None;
            };
            let index = nat_value(values.get(1)?)?;
            Some(Value::Row12(if builtin == Builtin::Row12Transposed {
                row.transposed(index)
            } else {
                row.inverted(index)
            }))
        }
        Builtin::Row12Retrograde => {
            let Value::Row12(row) = values.first()? else {
                return None;
            };
            Some(Value::Row12(row.retrograde()))
        }
        Builtin::Row12Matrix => {
            let Value::Row12(row) = values.first()? else {
                return None;
            };
            Some(Value::List {
                member: Type::Row12,
                values: row.matrix().into_iter().map(Value::Row12).collect(),
            })
        }
        Builtin::Row12Forms | Builtin::Row12Symmetries => {
            let Value::Row12(row) = values.first()? else {
                return None;
            };
            let count = if builtin == Builtin::Row12Forms {
                row.forms()
            } else {
                row.symmetries()
            };
            Some(Value::Nat(u64::from(count)))
        }
        Builtin::Row12Repeats => {
            let pcs = pc12_list(values.first()?)?;
            Some(Value::List {
                member: Type::Nat,
                values: crate::pc12::repeated_positions(&pcs)
                    .into_iter()
                    .map(Value::Nat)
                    .collect(),
            })
        }
        Builtin::Row12Missing => Some(pc12_values(crate::pc12::missing_classes(&pc12_list(values.first()?)?))),
        Builtin::RatioAdd => ratio_arithmetic(Exact::Add, &values),
        Builtin::RatioSub => ratio_arithmetic(Exact::Sub, &values),
        Builtin::RatioMul => ratio_arithmetic(Exact::Mul, &values),
        Builtin::RatioDiv => ratio_arithmetic(Exact::Div, &values),
        Builtin::RatioLess => Some(Value::Bool(
            ratio_value(values.first()?)? < ratio_value(values.get(1)?)?,
        )),
        Builtin::RatioEqual => Some(Value::Bool(
            ratio_value(values.first()?)? == ratio_value(values.get(1)?)?,
        )),
        Builtin::TextEqual => {
            let (Value::Text(left), Value::Text(right)) = (values.first()?, values.get(1)?) else {
                return None;
            };
            Some(Value::Bool(left == right))
        }
        Builtin::NatAdd | Builtin::NatMul => {
            let (left, right) = (nat_value(values.first()?)?, nat_value(values.get(1)?)?);
            let held = if builtin == Builtin::NatAdd {
                left.checked_add(right)
            } else {
                left.checked_mul(right)
            };
            Some(match held {
                Some(value) => answered(Type::Nat, Value::Nat(value)),
                None => refused(
                    Type::Nat,
                    "these whole numbers have no result this language can represent",
                ),
            })
        }
        // Below zero is the *only* way this fails, so `Option` says everything
        // a `Result` would: there is no second reason to distinguish it from.
        Builtin::NatSub => {
            let (left, right) = (nat_value(values.first()?)?, nat_value(values.get(1)?)?);
            Some(Value::Option {
                member: Type::Nat,
                value: left.checked_sub(right).map(|held| Box::new(Value::Nat(held))),
            })
        }
        Builtin::DurationOf => Some(written_duration(ratio_value(values.first()?)?)),
        Builtin::DurationRatio => Some(Value::Ratio(duration_value(values.first()?)?)),
        Builtin::DurationAdd => {
            let (left, right) = (duration_value(values.first()?)?, duration_value(values.get(1)?)?);
            Some(match exact_arithmetic(left, right, Exact::Add) {
                // Two nonnegative durations sum to a nonnegative one, so the
                // only thing left to fail is representability. The
                // constructor is still asked, because the law that durations
                // are nonnegative is stated in one place.
                Some(value) => written_duration(value),
                None => refused(
                    Type::Duration(Coordinate::WrittenTime),
                    "these durations have no sum this language can represent",
                ),
            })
        }
        Builtin::DurationScale => {
            let (held, factor) = (duration_value(values.first()?)?, ratio_value(values.get(1)?)?);
            Some(match exact_arithmetic(held, factor, Exact::Mul) {
                Some(value) => written_duration(value),
                None => refused(
                    Type::Duration(Coordinate::WrittenTime),
                    "this duration and factor have no product this language can represent",
                ),
            })
        }
        Builtin::DurationLess => Some(Value::Bool(
            duration_value(values.first()?)? < duration_value(values.get(1)?)?,
        )),
        Builtin::DurationEqual => Some(Value::Bool(
            duration_value(values.first()?)? == duration_value(values.get(1)?)?,
        )),
        // Total, and that is the difference between a position and a
        // duration: an instant before the origin is an ordinary position, so
        // there is no refinement here to check.
        Builtin::PositionOf => Some(Value::Position(Coordinate::WrittenTime, ratio_value(values.first()?)?)),
        Builtin::PositionRatio => Some(Value::Ratio(position_value(values.first()?)?)),
        Builtin::PositionShift => {
            let (from, by) = (position_value(values.first()?)?, duration_value(values.get(1)?)?);
            let ty = Type::Position(Coordinate::WrittenTime);
            Some(match exact_arithmetic(from, by, Exact::Add) {
                Some(value) => answered(ty, Value::Position(Coordinate::WrittenTime, value)),
                None => refused(
                    ty,
                    "this position and duration have no result this language can represent",
                ),
            })
        }
        // The one operation the whole tagging exists for. Two positions do
        // not add — there is no name for that — and their difference is a
        // duration only when the second is not before the first.
        Builtin::PositionBetween => {
            let (from, to) = (position_value(values.first()?)?, position_value(values.get(1)?)?);
            let ty = Type::Duration(Coordinate::WrittenTime);
            if to < from {
                return Some(refused(
                    ty,
                    "the second position is before the first, and a duration is nonnegative",
                ));
            }
            Some(match exact_arithmetic(to, from, Exact::Sub) {
                Some(value) => written_duration(value),
                None => refused(ty, "these positions have no difference this language can represent"),
            })
        }
        Builtin::PositionLess => Some(Value::Bool(
            position_value(values.first()?)? < position_value(values.get(1)?)?,
        )),
        Builtin::PositionEqual => Some(Value::Bool(
            position_value(values.first()?)? == position_value(values.get(1)?)?,
        )),
        Builtin::IntervalAdd => {
            let Value::Interval(first) = values.first()? else {
                return None;
            };
            let Value::Interval(second) = values.get(1)? else {
                return None;
            };
            first.compose(*second).map(Value::Interval)
        }
        Builtin::IntervalInverse => {
            let Value::Interval(interval) = values.first()? else {
                return None;
            };
            interval.inverse().map(Value::Interval)
        }
        Builtin::PitchClassOf => {
            let Value::Pitch(pitch) = values.first()? else {
                return None;
            };
            Some(Value::PitchClass(pitch.pitch_class()))
        }
        Builtin::SignatureScale => {
            let Value::Key(key) = values.first()? else {
                return None;
            };
            Some(Value::Scale(crate::scale::signature_scale(*key)))
        }
        Builtin::ScaleOn => {
            let (Value::Scale(scale), Value::PitchClass(tonic)) = (values.first()?, values.get(1)?) else {
                return None;
            };
            Some(Value::Scale(scale.rooted_at(*tonic)))
        }
        Builtin::ScaleTonic => {
            let Value::Scale(scale) = values.first()? else {
                return None;
            };
            Some(Value::PitchClass(scale.tonic()))
        }
        Builtin::ScaleSize => {
            let Value::Scale(scale) = values.first()? else {
                return None;
            };
            Some(Value::Nat(u64::try_from(scale.size()).ok()?))
        }
        Builtin::ScalePitch => {
            let (Value::Scale(scale), Value::Pitch(pitch)) = (values.first()?, values.get(1)?) else {
                return None;
            };
            let located = crate::scale::Frame::around(*scale, *pitch).and_then(|frame| frame.locate(*pitch));
            Some(optional(Type::Degree, located.map(Value::Degree)))
        }
        Builtin::ScaleClass => {
            let (Value::Scale(scale), Value::Degree(degree)) = (values.first()?, values.get(1)?) else {
                return None;
            };
            Some(optional(Type::PitchClass, scale.class(*degree).map(Value::PitchClass)))
        }
        Builtin::ScaleChord => {
            let (Value::Scale(scale), Value::Degree(degree)) = (values.first()?, values.get(1)?) else {
                return None;
            };
            let members = usize::try_from(nat_value(values.get(2)?)?).ok()?;
            Some(optional(
                Type::ChordClass,
                scale.stacked(*degree, members).map(Value::ChordClass),
            ))
        }
        Builtin::PitchFrame => {
            let (Value::Scale(scale), Value::Pitch(tonic)) = (values.first()?, values.get(1)?) else {
                return None;
            };
            Some(optional(
                Type::Frame,
                crate::scale::Frame::new(*scale, *tonic).map(Value::Frame),
            ))
        }
        Builtin::FrameScale => {
            let Value::Frame(frame) = values.first()? else {
                return None;
            };
            Some(Value::Scale(frame.scale()))
        }
        Builtin::FrameTonic => {
            let Value::Frame(frame) = values.first()? else {
                return None;
            };
            Some(Value::Pitch(frame.tonic()))
        }
        Builtin::FramePitch => {
            let (Value::Frame(frame), Value::Degree(degree)) = (values.first()?, values.get(1)?) else {
                return None;
            };
            frame.pitch(*degree).map(Value::Pitch)
        }
        Builtin::DegreeOf => {
            // Degrees are written from one, as musicians write them, and
            // `Degree` counts from one as well: no adjustment belongs here.
            let ordinal = i64::try_from(nat_value(values.first()?)?).ok()?;
            Some(Value::Degree(crate::scale::Degree::new(ordinal)))
        }
        Builtin::DegreeStepUp | Builtin::DegreeStepDown => {
            let Value::Degree(degree) = values.first()? else {
                return None;
            };
            let steps = i64::try_from(nat_value(values.get(1)?)?).ok()?;
            let steps = if builtin == Builtin::DegreeStepDown {
                steps.checked_neg()?
            } else {
                steps
            };
            degree.step(steps).map(Value::Degree)
        }
        Builtin::DegreeRaised => {
            let Value::Degree(degree) = values.first()? else {
                return None;
            };
            degree.raised().map(Value::Degree)
        }
        Builtin::ChordOn => {
            let (Value::ChordClass(class), Value::PitchClass(root)) = (values.first()?, values.get(1)?) else {
                return None;
            };
            Some(Value::ChordClass(class.rooted_at(*root)))
        }
        Builtin::ChordRoot => {
            let Value::ChordClass(class) = values.first()? else {
                return None;
            };
            Some(Value::PitchClass(class.root()))
        }
        Builtin::ChordBass => {
            let Value::ChordClass(class) = values.first()? else {
                return None;
            };
            Some(optional(Type::PitchClass, class.bass().map(Value::PitchClass)))
        }
        Builtin::ChordMembers => {
            let Value::ChordClass(class) = values.first()? else {
                return None;
            };
            Some(Value::List {
                member: Type::Interval,
                values: class.members().iter().copied().map(Value::Interval).collect(),
            })
        }
        Builtin::ChordInversion => {
            let Value::ChordClass(class) = values.first()? else {
                return None;
            };
            let position = usize::try_from(nat_value(values.get(1)?)?).ok()?;
            Some(optional(
                Type::ChordClass,
                class.inverted(position).ok().map(Value::ChordClass),
            ))
        }
        Builtin::ChordOver => {
            let (Value::ChordClass(class), Value::PitchClass(bass)) = (values.first()?, values.get(1)?) else {
                return None;
            };
            Some(Value::ChordClass(class.over(*bass)))
        }
        Builtin::ChordTriad => {
            let Value::ChordClass(class) = values.first()? else {
                return None;
            };
            Some(optional(Type::Triad, crate::chord::Triad::of(*class).map(Value::Triad)))
        }
        Builtin::TriadChord => {
            let Value::Triad(triad) = values.first()? else {
                return None;
            };
            Some(Value::ChordClass(triad.class()))
        }
        Builtin::RomanOf => {
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
        Builtin::RomanOrdinal => {
            let Value::Roman(numeral) = values.first()? else {
                return None;
            };
            Some(Value::Nat(numeral.ordinal()))
        }
        Builtin::RomanSize => {
            let Value::Roman(numeral) = values.first()? else {
                return None;
            };
            Some(Value::Nat(numeral.members()))
        }
        Builtin::RomanInversion => {
            let Value::Roman(numeral) = values.first()? else {
                return None;
            };
            Some(Value::Nat(numeral.inversion()))
        }
        Builtin::TriadMajor => {
            let Value::Triad(triad) = values.first()? else {
                return None;
            };
            Some(Value::Bool(triad.is_major()))
        }
        Builtin::VoicingOf => {
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
                    | Value::Text(_)
                    | Value::Duration(..)
                    | Value::Position(..)
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
                    | Value::Sum { .. }
                    | Value::Option { .. }
                    | Value::List { .. }
                    | Value::Data { .. }
                    | Value::Music(_)
                    | Value::Closure(_)
                    | Value::Primitive { .. }
                    | Value::Machine { .. }
                    | Value::Syntax(_)
                    | Value::NodePath(_)
                    | Value::BindingPath(_)
                    | Value::SyntaxStep(_)
                    | Value::Builtin(_) => None,
                })
                .collect();
            Some(optional(
                Type::Voicing,
                crate::chord::Voicing::new(*class, written?).ok().map(Value::Voicing),
            ))
        }
        Builtin::VoicingPitches => {
            let Value::Voicing(voicing) = values.first()? else {
                return None;
            };
            Some(Value::List {
                member: Type::Pitch,
                values: voicing.pitches().map(Value::Pitch).collect(),
            })
        }
        Builtin::VoicingBass => {
            let Value::Voicing(voicing) = values.first()? else {
                return None;
            };
            Some(Value::Pitch(voicing.bass()))
        }
        Builtin::VoicingChord => {
            let Value::Voicing(voicing) = values.first()? else {
                return None;
            };
            Some(Value::ChordClass(voicing.class()))
        }
        Builtin::VoicingPosition => {
            let Value::Voicing(voicing) = values.first()? else {
                return None;
            };
            let position = voicing
                .inversion()
                .and_then(|position| u64::try_from(position).ok())
                .map(Value::Nat);
            Some(optional(Type::Nat, position))
        }
        Builtin::CloseVoicing => {
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
        Builtin::DropVoicing => {
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
        Builtin::OmitVoicing => {
            let Value::Voicing(voicing) = values.first()? else {
                return None;
            };
            let position = usize::try_from(nat_value(values.get(1)?)?).ok()?;
            Some(optional(
                Type::Voicing,
                voicing.omitting(position).ok().map(Value::Voicing),
            ))
        }
        Builtin::DegreeLowered => {
            let Value::Degree(degree) = values.first()? else {
                return None;
            };
            degree.lowered().map(Value::Degree)
        }
        Builtin::Range => {
            let count = nat_value(values.first()?)?;
            let nodes = count.saturating_add(1);
            let bytes = count.saturating_mul(8);
            meter.step(Reduction::Range, count, expression.span)?;
            meter.preflight_construct("range", nodes, bytes, expression.span)?;
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
        Builtin::Repeat => {
            let value = values.first()?.clone();
            let count = nat_value(values.get(1)?)?;
            let (value_nodes, value_bytes) = value_shape(&value);
            let nodes = value_nodes.saturating_mul(count).saturating_add(1);
            let bytes = value_bytes.saturating_mul(count);
            meter.step(Reduction::Repeat, count, expression.span)?;
            meter.preflight_construct("repeat", nodes, bytes, expression.span)?;
            let count = usize::try_from(count).ok()?;
            Some(Value::List {
                member: value.ty(),
                values: vec![value; count],
            })
        }
        Builtin::Map => {
            let Value::Closure(function) = values.first()? else {
                return None;
            };
            let Value::List { values: source, .. } = values.get(1)? else {
                return None;
            };
            meter.step(
                Reduction::Map,
                u64::try_from(source.len()).unwrap_or(u64::MAX),
                expression.span,
            )?;
            let mapped = source
                .iter()
                .cloned()
                .map(|value| apply_closure(function, vec![value], meter, expression.span))
                .collect::<Option<Vec<_>>>()?;
            let Type::List(member) = &expression.ty else {
                return None;
            };
            let member = member.as_ref().clone();
            Some(Value::List { member, values: mapped })
        }
        Builtin::Filter => {
            let Value::Closure(predicate) = values.first()? else {
                return None;
            };
            let Value::List { member, values } = values.get(1)? else {
                return None;
            };
            meter.step(
                Reduction::Filter,
                u64::try_from(values.len()).unwrap_or(u64::MAX),
                expression.span,
            )?;
            let mut kept = Vec::new();
            for value in values {
                let decision = apply_closure(predicate, vec![value.clone()], meter, expression.span)?;
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
        Builtin::NatFold => {
            let mut accumulator = values.first()?.clone();
            let Value::Closure(step) = values.get(1)? else {
                return None;
            };
            let count = nat_value(values.get(2)?)?;
            meter.step(Reduction::NatFold, count, expression.span)?;
            for index in 0..count {
                accumulator = apply_closure(step, vec![Value::Nat(index), accumulator], meter, expression.span)?;
            }
            Some(accumulator)
        }
        Builtin::ListFoldFromStart => {
            let mut accumulator = values.first()?.clone();
            let Value::Closure(step) = values.get(1)? else {
                return None;
            };
            let Value::List { values, .. } = values.get(2)? else {
                return None;
            };
            meter.step(
                Reduction::ListFoldFromStart,
                u64::try_from(values.len()).unwrap_or(u64::MAX),
                expression.span,
            )?;
            for value in values {
                accumulator = apply_closure(step, vec![value.clone(), accumulator], meter, expression.span)?;
            }
            Some(accumulator)
        }
        // The catamorphism. §5.6's equation is `s(x, fold(xs))`, so the step nearest the base case
        // is the one applied to the *last* member; iterating in reverse computes that without
        // building the recursive term, exactly as the other folds iterate rather than build.
        Builtin::ListFoldFromEnd => {
            let mut accumulator = values.first()?.clone();
            let Value::Closure(step) = values.get(1)? else {
                return None;
            };
            let Value::List { values, .. } = values.get(2)? else {
                return None;
            };
            meter.step(
                Reduction::ListFoldFromEnd,
                u64::try_from(values.len()).unwrap_or(u64::MAX),
                expression.span,
            )?;
            for value in values.iter().rev() {
                accumulator = apply_closure(step, vec![value.clone(), accumulator], meter, expression.span)?;
            }
            Some(accumulator)
        }
        Builtin::OptionFold => {
            let zero = values.first()?.clone();
            let Value::Closure(some_case) = values.get(1)? else {
                return None;
            };
            let Value::Option { value, .. } = values.get(2)? else {
                return None;
            };
            match value {
                Some(value) => {
                    meter.step(Reduction::OptionFold, 1, expression.span)?;
                    apply_closure(some_case, vec![value.as_ref().clone()], meter, expression.span)
                }
                None => Some(zero),
            }
        }
        // §2's forms, each building a description rather than running one. Evaluation is where a
        // configuration and a feedback value become exact bytes: the checker has already said
        // they are storable data, and this is the last moment a value can be read as the value it
        // was written as.
        Builtin::Primitive => {
            let (Value::Text(id), Value::Nat(number)) = (values.first()?, values.get(1)?) else {
                return None;
            };
            let descriptor = u32::try_from(*number)
                .ok()
                .and_then(|number| crate::machine::descriptor(id, number))?;
            let configuration = values.get(2)?.clone();
            let (nodes, bytes) = value_shape(&configuration);
            meter.preflight_construct(builtin.name(), nodes.saturating_add(1), bytes, expression.span)?;
            Some(Value::Primitive {
                descriptor,
                configuration: Box::new(configuration),
            })
        }
        Builtin::Machine => {
            let Value::Primitive {
                descriptor,
                configuration,
            } = values.first()?
            else {
                return None;
            };
            let mut encoded = Vec::new();
            encode_exactly(configuration, &mut encoded)?;
            machine_value(
                builtin,
                MachineTree::Primitive {
                    descriptor,
                    configuration: encoded,
                },
                meter,
                expression,
            )
        }
        Builtin::Identity => machine_value(builtin, MachineTree::Identity, meter, expression),
        Builtin::Copy => machine_value(builtin, MachineTree::Copy, meter, expression),
        Builtin::Drop => machine_value(builtin, MachineTree::Drop, meter, expression),
        Builtin::Swap => machine_value(builtin, MachineTree::Swap, meter, expression),
        Builtin::Connect | Builtin::Beside => {
            let (Value::Machine { tree: first, .. }, Value::Machine { tree: second, .. }) =
                (values.first()?, values.get(1)?)
            else {
                return None;
            };
            let (first, second) = (first.clone(), second.clone());
            let tree = if builtin == Builtin::Connect {
                MachineTree::Connect(first, second)
            } else {
                MachineTree::Beside(first, second)
            };
            machine_value(builtin, tree, meter, expression)
        }
        Builtin::Feedback => {
            let Value::Machine { tree: inner, .. } = values.get(1)? else {
                return None;
            };
            let mut initial = Vec::new();
            encode_exactly(values.first()?, &mut initial)?;
            machine_value(
                builtin,
                MachineTree::Feedback {
                    initial,
                    inner: inner.clone(),
                },
                meter,
                expression,
            )
        }
        // The track builtins are values with arrow types, performed by `apply_builtin` once
        // an application supplies their arguments. Elaboration never builds an
        // `ExprKind::Builtin` around one.
        Builtin::Transpose
        | Builtin::Stretch
        | Builtin::Retrograde
        | Builtin::Invert
        | Builtin::Shift
        | Builtin::Together
        | Builtin::MapNotePitches
        | Builtin::Play => None,
    }
}

/// Run one phase-local syntax operation.
///
/// Every case here is a function of the values it was handed and nothing else:
/// there is no counter, no allocation, and no compiler state to read, so two
/// runs of one transformer over one region agree exactly. The recursive cases
/// are [`SyntaxOp::Recurse`], [`SyntaxOp::Run`], and [`SyntaxOp::Fold`], all
/// three of which are [`recurse_syntax`], and each descent enters a strict
/// subtree of a finite value.
///
/// `arguments` is here for one reason: the recursor's group branch is handed a
/// `List<SyntaxStep<C, A>>`, and the member type of that list needs `C`, which
/// lives on the context argument's checked type and in no value.
fn eval_syntax(
    operation: SyntaxOp,
    arguments: &[Expr],
    values: &[Value],
    meter: &mut WorkMeter,
    expression: &Expr,
) -> Option<Value> {
    let syntax = |value: &Value| {
        if let Value::Syntax(held) = value {
            Some(held.as_ref().clone())
        } else {
            None
        }
    };
    let path = |value: &Value| {
        if let Value::NodePath(held) = value {
            Some(held.as_ref().clone())
        } else {
            None
        }
    };
    let binding = |value: &Value| {
        if let Value::BindingPath(held) = value {
            Some(held.as_ref().clone())
        } else {
            None
        }
    };
    let text = |value: &Value| {
        if let Value::Text(held) = value {
            Some(held.clone())
        } else {
            None
        }
    };
    let index = |value: &Value| u32::try_from(nat_value(value)?).ok();
    let built = |node: crate::syntax::Syntax, meter: &mut WorkMeter| {
        let (nodes, bytes) = node.shape();
        meter.preflight_construct(operation.spelling(), nodes, bytes, expression.span)?;
        Some(Value::Syntax(Box::new(node)))
    };
    match operation {
        SyntaxOp::Fold => {
            let subject = syntax(values.get(4)?)?;
            recurse_syntax(values, &Descent::FromLeaves, &subject, meter, expression)
        }
        SyntaxOp::Recurse => {
            let subject = syntax(values.get(5)?)?;
            let descent = Descent::Sealed(Box::new(Inherited {
                context: values.get(4)?.clone(),
                step_type: Type::SyntaxStep {
                    context: Box::new(arguments.get(4)?.ty.clone()),
                    answer: Box::new(expression.ty.clone()),
                },
            }));
            recurse_syntax(values, &descent, &subject, meter, expression)
        }
        SyntaxOp::Run => {
            let Value::SyntaxStep(step) = values.get(1)? else {
                return None;
            };
            run_syntax_step(values.first()?, step, meter, expression)
        }
        // The reader's own reading, handed back rather than re-derived. Both
        // numeric kinds the lexer distinguishes answer here and everything
        // else — an identifier, a group, a pitch literal, a token of some
        // other kind — is not a number and says so in the value.
        SyntaxOp::Number => {
            // Total, as every δ-style operation is: a token this compiler
            // cannot represent exactly answers "not a number" rather than
            // getting stuck, so the transformer sees one absence and not two
            // kinds of silence.
            let found = match syntax(values.first()?)? {
                crate::syntax::Syntax::Token { ref kind, ref text, .. } => match kind.as_str() {
                    "Integer" => text.parse::<i64>().ok().map(Ratio::from_integer),
                    "Rational" => text
                        .split_once('/')
                        .and_then(|(numerator, denominator)| {
                            Some((numerator.parse::<i128>().ok()?, denominator.parse::<i128>().ok()?))
                        })
                        .and_then(|(numerator, denominator)| exact_ratio(numerator, denominator)),
                    _ => None,
                },
                crate::syntax::Syntax::Missing(_)
                | crate::syntax::Syntax::Identifier { .. }
                | crate::syntax::Syntax::Group { .. } => None,
            };
            Some(optional(Type::Ratio, found.map(Value::Ratio)))
        }
        SyntaxOp::At => {
            let subject = syntax(values.first()?)?;
            let wanted = path(values.get(1)?)?;
            let found = subject.at(&wanted).cloned();
            if let Some(found) = &found {
                let (nodes, bytes) = found.shape();
                meter.preflight_construct(operation.spelling(), nodes, bytes, expression.span)?;
            }
            Some(optional(Type::Syntax, found.map(|node| Value::Syntax(Box::new(node)))))
        }
        // The number is built into a token rather than handed over as a `Nat`:
        // a transformer's answer is an expression, and the only way a number
        // reaches one is as a literal the ordinary parser reads. Handing back a
        // node is also what keeps §3.4 exact — the adapter gets something to
        // splice, and nothing to compare a range against.
        SyntaxOp::Anchor => {
            let subject = syntax(values.first()?)?;
            let wanted = path(values.get(1)?)?;
            let here = path(values.get(2)?)?;
            let found = subject
                .anchor(&wanted)
                .map(|anchor| crate::syntax::token(here, "Integer".to_owned(), anchor.to_string()));
            let held = match found {
                Some(node) => Some(built(node, meter)?),
                None => None,
            };
            Some(optional(Type::Syntax, held))
        }
        SyntaxOp::Built => Some(Value::NodePath(Box::new(
            path(values.first()?)?.built(index(values.get(1)?)?, index(values.get(2)?)?),
        ))),
        SyntaxOp::Binding => Some(Value::BindingPath(Box::new(
            path(values.first()?)?.binding(index(values.get(1)?)?),
        ))),
        SyntaxOp::Token => built(
            crate::syntax::token(path(values.first()?)?, text(values.get(1)?)?, text(values.get(2)?)?),
            meter,
        ),
        SyntaxOp::Identifier => built(
            crate::syntax::identifier(path(values.first()?)?, text(values.get(1)?)?),
            meter,
        ),
        SyntaxOp::Group => {
            let Value::List { values: children, .. } = values.get(2)? else {
                return None;
            };
            let children = children.iter().map(syntax).collect::<Option<Vec<_>>>()?;
            built(
                crate::syntax::group(path(values.first()?)?, text(values.get(1)?)?, children),
                meter,
            )
        }
        SyntaxOp::Binder => built(
            crate::syntax::binder(&binding(values.first()?)?, text(values.get(1)?)?),
            meter,
        ),
        SyntaxOp::Reference => built(
            crate::syntax::reference(path(values.first()?)?, &binding(values.get(1)?)?, text(values.get(2)?)?),
            meter,
        ),
        // The gate answers with a value either way, which is what keeps it
        // total: a transformer that builds badly gets a `Result` back and
        // decides what to say about it.
        SyntaxOp::Checked => {
            let subject = syntax(values.first()?)?;
            let held = match crate::syntax::check_expression(&subject) {
                Ok(()) => Value::Syntax(Box::new(subject)),
                Err(refusal) => Value::Text(refusal.to_string()),
            };
            Some(Value::Sum {
                value_type: Type::Syntax,
                error_type: Type::Text,
                error: matches!(held, Value::Text(_)),
                held: Box::new(held),
            })
        }
    }
}

/// How a group's children reach its branch.
///
/// The one difference between the two descents, and therefore the only thing
/// worth parameterizing. `docs/rules/language/02-core-calculus.md` §5.9's law
/// 9 is the statement that these two agree on the value when the context is
/// one nothing reads, and it is checkable here because there is one traversal
/// to compare against itself.
#[derive(Clone)]
enum Descent {
    /// Sealed steps, under an inherited context. The branch decides whether,
    /// in what order, and under what context each child is read. Boxed
    /// because the other descent carries nothing at all.
    Sealed(Box<Inherited>),
    /// Already-read answers, in source order: every child is read before its
    /// group's branch runs. The derived `syntax_fold_from_leaves`.
    FromLeaves,
}

/// What a sealed descent carries down.
#[derive(Clone)]
struct Inherited {
    /// The context this node is read under — §5.9's `c`.
    context: Value,
    /// `SyntaxStep<C, A>`, for the member type of the list a group branch is
    /// handed.
    step_type: Type,
}

/// The one descent into a syntax value.
///
/// Each branch receives the node's own path — read out of the node rather than
/// reconstructed — so a transformer cannot reach a node without also holding
/// the path it would build output from. That is blocker 1's repair: paths come
/// *from here*, and from deriving one already held.
///
/// Under [`Descent::Sealed`] the group branch is handed one
/// [`Value::SyntaxStep`] per child instead of one answer per child. Minting is
/// the only way such a value is made and [`SyntaxOp::Run`] the only way one is
/// consumed, so the child and the algebra a step names are decided here and
/// nowhere else — which is why a step carried into a nested recursor still
/// runs its own child under its own algebra, with no ownership check to
/// perform (§5.9).
///
/// Termination is not the local decrease alone: a branch may capture a step,
/// run it later, run it twice, or start a fresh recursor on any subject in
/// scope. The governing argument is §5.9's reducibility candidate and
/// fundamental lemma, which rest on the checker's definition acyclicity; this
/// function is that argument's implementation and not a substitute for it.
fn recurse_syntax(
    algebra: &[Value],
    descent: &Descent,
    subject: &crate::syntax::Syntax,
    meter: &mut WorkMeter,
    expression: &Expr,
) -> Option<Value> {
    meter.step(
        match descent {
            Descent::Sealed(_) => Reduction::SyntaxRecurse,
            // Unchanged from the fold this derives, so no shipped adapter's
            // budget moves when the name does.
            Descent::FromLeaves => Reduction::SyntaxFold,
        },
        1,
        expression.span,
    )?;
    let at = Value::NodePath(Box::new(subject.info().path().clone()));
    let mut arguments = match descent {
        Descent::Sealed(inherited) => vec![inherited.context.clone(), at],
        Descent::FromLeaves => vec![at],
    };
    let branch = match subject {
        crate::syntax::Syntax::Missing(_) => algebra.first()?,
        crate::syntax::Syntax::Token { kind, text, .. } => {
            arguments.push(Value::Text(kind.clone()));
            arguments.push(Value::Text(text.clone()));
            algebra.get(1)?
        }
        crate::syntax::Syntax::Identifier { name, .. } => {
            arguments.push(Value::Text(name.clone()));
            algebra.get(2)?
        }
        crate::syntax::Syntax::Group {
            delimiter, children, ..
        } => {
            arguments.push(Value::Text(delimiter.clone()));
            // The member type comes from the operation's own scheme and not
            // from the first value: a list whose member type was guessed from
            // what it happens to hold would be a different type when the group
            // is empty.
            let (member, values) = match descent {
                Descent::Sealed(inherited) => {
                    let mut minted = Vec::with_capacity(children.len());
                    for child in children {
                        meter.step(Reduction::SyntaxStepMint, 1, expression.span)?;
                        minted.push(Value::SyntaxStep(Box::new(SealedStep {
                            algebra: algebra.to_vec(),
                            child: Box::new(child.clone()),
                            ty: inherited.step_type.clone(),
                        })));
                    }
                    (inherited.step_type.clone(), minted)
                }
                Descent::FromLeaves => (
                    expression.ty.clone(),
                    children
                        .iter()
                        .map(|child| recurse_syntax(algebra, descent, child, meter, expression))
                        .collect::<Option<Vec<_>>>()?,
                ),
            };
            arguments.push(Value::List { member, values });
            algebra.get(3)?
        }
    };
    let Value::Closure(branch) = branch else {
        return None;
    };
    apply_closure(branch, arguments, meter, expression.span)
}

/// Resume one sealed step under `context`.
///
/// The whole of `run_syntax_step`: the step says which child and which
/// algebra, and the caller says only the context. There is nothing here that
/// could consult the recursor a call happens to stand inside, which is why the
/// association law needs no dynamic check and this returns a value rather than
/// a `Result`.
fn run_syntax_step(context: &Value, step: &SealedStep, meter: &mut WorkMeter, expression: &Expr) -> Option<Value> {
    meter.step(Reduction::SyntaxStepRun, 1, expression.span)?;
    recurse_syntax(
        &step.algebra,
        &Descent::Sealed(Box::new(Inherited {
            context: context.clone(),
            step_type: step.ty.clone(),
        })),
        &step.child,
        meter,
        expression,
    )
}

/// A finished description as a value, charged for what it holds.
///
/// The type comes from the expression rather than from the tree because the
/// tree says how the machine is wired and the type says what it is wired *for*:
/// the step and the two ports live only in the type, and the projection needs
/// all three.
fn machine_value(builtin: Builtin, tree: MachineTree, meter: &mut WorkMeter, expression: &Expr) -> Option<Value> {
    let (nodes, bytes) = tree.shape();
    meter.preflight_construct(builtin.name(), nodes, bytes, expression.span)?;
    Some(Value::Machine {
        ty: expression.ty.clone(),
        tree: Box::new(tree),
    })
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

/// Apply a fold's cases to one value, folding the fields that are themselves
/// group members before the case sees them.
///
/// `shape` says which constructor of which declaration each case answers for,
/// in the order [`crate::data::Folding`] laid them out, so this and the type
/// the checker gave the fold read the same list.
fn fold_value(
    cases: &[Value],
    shape: &[(crate::data::NominalId, usize)],
    value: &Value,
    meter: &mut WorkMeter,
    span: SourceSpan,
) -> Option<Value> {
    let Value::Data {
        id, variant, fields, ..
    } = value
    else {
        return None;
    };
    meter.step(Reduction::DataFold, 1, span)?;
    let index = shape
        .iter()
        .position(|(member, constructor)| member == id && constructor == variant)?;
    let arguments = fields
        .iter()
        .map(|field| {
            // A field of a group member is what the fold made of it; anything
            // else arrives as itself, because a fold replaces one constructor
            // layer and not what sits under a `List` or an `Option`.
            if let Value::Data { id, .. } = field
                && shape.iter().any(|(member, _)| member == id)
            {
                return fold_value(cases, shape, field, meter, span);
            }
            Some(field.clone())
        })
        .collect::<Option<Vec<_>>>()?;
    let case = cases.get(index)?;
    // A case that takes nothing is a value, not a function: a constructor with
    // no fields has nothing to hand one.
    if arguments.is_empty() {
        return Some(case.clone());
    }
    match case {
        Value::Closure(closure) => apply_closure(closure, arguments, meter, span),
        Value::Builtin(builtin) => apply_builtin(*builtin, arguments, span),
        // Every other value is not a function, and the checker has already
        // said so: a case of a constructor with fields has an arrow type.
        Value::Primitive { .. }
        | Value::Machine { .. }
        | Value::Bool(_)
        | Value::Nat(_)
        | Value::Ratio(_)
        | Value::Text(_)
        | Value::Duration(..)
        | Value::Position(..)
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
        | Value::Sum { .. }
        | Value::Option { .. }
        | Value::List { .. }
        | Value::Data { .. }
        | Value::Syntax(_)
        | Value::NodePath(_)
        | Value::BindingPath(_)
        | Value::SyntaxStep(_)
        | Value::Music(_) => None,
    }
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
        Pattern::Ok(name) | Pattern::Err(name) => {
            let wanted = matches!(pattern, Pattern::Err(_));
            if let Value::Sum { error, held, .. } = value
                && *error == wanted
            {
                bindings.insert(name.clone(), held.as_ref().clone());
                true
            } else {
                false
            }
        }
        Pattern::Constructor { variant, fields: names } => {
            if let Value::Data {
                variant: found, fields, ..
            } = value
                && found == variant
                && fields.len() == names.len()
            {
                for (name, field) in names.iter().zip(fields) {
                    bindings.insert(name.clone(), field.clone());
                }
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

/// Whether a literal pattern's value and the matched value are the same value.
///
/// Answered through [`literal_key`], which is the same identity the checker
/// already uses to tell two arms apart, so a pattern matches at run time
/// exactly when the checker counted it as covering. Two notions of "the same
/// literal" could disagree, and when they did the disagreement would be a file
/// that compiles and means something else.
///
/// This is also why the comparison is not a list of pairs. A pair list has a
/// last arm, that arm answers "not equal", and a value kind nobody remembered
/// to add falls into it — which is precisely what happened to [`Value::Text`],
/// where a text pattern never matched and the arm below it ran with nothing
/// reported. [`literal_key`] is an exhaustive match, so a value kind added
/// without a key is a compile error rather than a wrong answer.
fn literal_values_equal(left: &Value, right: &Value) -> bool {
    literal_key(left) == literal_key(right)
}

/// What one expression constructs, which is what the node and byte counters
/// charge (`02-core-calculus.md` §4).
///
/// A value is charged once, where it is built. Naming it, matching on it, or
/// returning it from a call builds nothing, so those charge nothing; the
/// reduction step they already charge is what bounds a program's length, and
/// charging the size of something an expression did not build would make a
/// project's cost the product of its data size and its program size. That is
/// not a conservative approximation of anything — prompt 127dcfa's staff
/// adapter, whose reader state is one value threaded through a region, could
/// not read six header lines and one bar under it.
///
/// Three cases, and the middle one is the one worth reading twice. An
/// aggregate built out of parts already evaluated is one cell whose fields
/// hold the addresses of those parts (Peyton Jones ch. 10 §10.3), so it is
/// charged its own node and its wiring and not its parts again. A value that
/// is new all the way down — a literal, a builtin's answer, a quotation — is
/// charged all the way down, because nothing else charged it.
fn charged_shape(kind: &ExprKind, value: &Value) -> (u64, u64) {
    match kind {
        // Selection: the value already existed.
        ExprKind::Name(_) | ExprKind::Match { .. } | ExprKind::Apply { .. } | ExprKind::Fold { .. } => (0, 0),
        // Wiring: one cell, and one field per part it points at.
        ExprKind::Product(_)
        | ExprKind::Option(_)
        | ExprKind::Injection { .. }
        | ExprKind::List(_)
        | ExprKind::Construct { .. }
        | ExprKind::Lambda { .. } => wiring_shape(value),
        // Fabrication: nothing else has charged this.
        ExprKind::Literal(_)
        | ExprKind::PitchAction { .. }
        | ExprKind::Step { .. }
        | ExprKind::Builtin { .. }
        | ExprKind::Music(_)
        | ExprKind::KernelQuote(_) => value_shape(value),
    }
}

/// One constructed cell and one field per part it holds.
fn wiring_shape(value: &Value) -> (u64, u64) {
    let fields = match value {
        Value::Product(members) => members.len(),
        Value::List { values, .. } => values.len(),
        Value::Data { fields, .. } => fields.len(),
        Value::Option { value, .. } => usize::from(value.is_some()),
        Value::Closure(closure) => closure.captures.len(),
        Value::Sum { .. } => 1,
        // Not built by a wiring expression; charged as a leaf if one ever is.
        Value::Bool(_)
        | Value::Nat(_)
        | Value::Ratio(_)
        | Value::Duration(..)
        | Value::Position(..)
        | Value::Pitch(_)
        | Value::PitchClass(_)
        | Value::Interval(_)
        | Value::Key(_)
        | Value::Degree(_)
        | Value::Scale(_)
        | Value::Frame(_)
        | Value::ChordClass(_)
        | Value::Triad(_)
        | Value::Roman(_)
        | Value::Pc12(_)
        | Value::PcSet12(_)
        | Value::Row12(_)
        | Value::Voicing(_)
        | Value::Text(_)
        | Value::Music(_)
        | Value::Builtin(_)
        | Value::Primitive { .. }
        | Value::Machine { .. }
        | Value::Syntax(_)
        | Value::NodePath(_)
        | Value::BindingPath(_)
        | Value::SyntaxStep(_) => 0,
    };
    let cells = u64::try_from(fields).unwrap_or(u64::MAX).saturating_add(1);
    (cells, cells)
}

fn value_shape(value: &Value) -> (u64, u64) {
    match value {
        Value::Bool(_) => (1, 1),
        Value::Nat(_) => (1, 8),
        Value::Ratio(_) | Value::Duration(..) | Value::Position(..) => (1, 16),
        Value::Pitch(_) | Value::PitchClass(_) | Value::Interval(_) | Value::Key(_) | Value::Degree(_) => (1, 12),
        Value::Scale(_) => (1, 24),
        Value::Frame(_) => (1, 36),
        Value::ChordClass(_) | Value::Triad(_) => (1, 24),
        Value::Roman(_) => (1, 12),
        Value::Pc12(_) => (1, 1),
        Value::PcSet12(_) => (1, 2),
        Value::Row12(_) => (1, 12),
        Value::Voicing(value) => (1, u64::try_from(value.size()).unwrap_or(u64::MAX).saturating_mul(12)),
        Value::Text(value) => (1, u64::try_from(value.len()).unwrap_or(u64::MAX)),
        Value::Product(members) => aggregate_shape(members.iter()),
        Value::Sum { held, .. } => {
            let (nodes, bytes) = value_shape(held);
            (nodes.saturating_add(1), bytes.saturating_add(1))
        }
        Value::Option { value, .. } => value.as_deref().map_or((1, 1), |value| {
            let (nodes, bytes) = value_shape(value);
            (nodes.saturating_add(1), bytes.saturating_add(1))
        }),
        Value::List { values, .. } => aggregate_shape(values.iter()),
        Value::Data { fields, .. } => {
            let (nodes, bytes) = aggregate_shape(fields.iter());
            (nodes.saturating_add(1), bytes.saturating_add(1))
        }
        Value::Music(music) => music_shape(music),
        // A closure's environment is its bindings, not its bindings'
        // contents. Capturing a name is a pointer to a value that already
        // exists and was already charged; §4's "closure environment size" is
        // that count.
        Value::Closure(closure) => {
            let bound = u64::try_from(closure.captures.len()).unwrap_or(u64::MAX);
            (bound.saturating_add(1), bound.saturating_add(1))
        }
        Value::Builtin(_) => (1, 1),
        // One node per description node, and its stored bytes as its size: a
        // configuration is what a machine value actually holds.
        Value::Primitive { configuration, .. } => {
            let (nodes, bytes) = value_shape(configuration);
            (nodes.saturating_add(1), bytes)
        }
        Value::Machine { tree, .. } => tree.shape(),
        Value::Syntax(held) => held.shape(),
        Value::NodePath(held) => held.shape(),
        Value::BindingPath(held) => held.shape(),
        // One cell and nothing under it. A step constructs nothing: its child
        // is already a node of the subject and its algebra is already in the
        // environment, both charged where they arrived. What minting costs is
        // a reduction, charged by `Reduction::SyntaxStepMint`, which is what
        // keeps §5.9's law 10 honest about capture.
        Value::SyntaxStep(_) => (1, 0),
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
        Some(MusicOperation::Together { left, right }) => {
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

/// Report the meter's failure, if it has one.
pub(crate) fn report_exhaustion(resolver: &mut Resolver, meter: &WorkMeter) {
    if let Some(failure) = meter.failure() {
        report_resource_error(resolver, failure);
    }
}

/// The one resource rejection, naming the operation, metric, attempted amount,
/// and limit (`docs/rules/language/02-core-calculus.md` §4).
///
/// The cost table's version is stated because it is half the answer: the same
/// source refused under version *n* is a claim about version *n*'s weights, and
/// a reader comparing two compilers needs to know which table said no.
pub(crate) fn report_resource_error(resolver: &mut Resolver, failure: ResourceError) {
    resolver.report(
        Diagnostic::error(
            Code::ResourceLimit,
            format!("`{}` exceeds the compilation budget", failure.operation),
        )
        .at(
            failure.span,
            format!(
                "attempted {} {}, limit {}",
                failure.attempted, failure.metric, failure.limit
            ),
        )
        .note(format!(
            "the expression is finite; Musa rejected its size before publishing a partial value (cost table {})",
            failure.cost_version
        ))
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
/// must not do (`docs/rules/language/00-semantics.md`, contextual closure).
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
    musa_language::ast::is_type(kind)
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
            | SyntaxKind::ResultExpr
            | SyntaxKind::ApplyExpr
            | SyntaxKind::LambdaExpr
            | SyntaxKind::PitchExpr
            | SyntaxKind::ChordExpr
            | SyntaxKind::ScaleExpr
            | SyntaxKind::KeyExpr
            | SyntaxKind::StepExpr
            | SyntaxKind::MatchExpr
            | SyntaxKind::IfExpr
            | SyntaxKind::RecordUpdateExpr
            | SyntaxKind::QuestionExpr
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

/// One adapter module, checked and evaluated in the phase environment.
///
/// **This is the phase environment**, and the only place [`Reading::Expansion`]
/// is ever set. Everything about it is the ordinary machinery: the same
/// declarations, the same Algorithm W, the same total evaluator, the same work
/// meter, and the same `data` world. What is phase-local is the *environment* —
/// [`SYNTAX_OWNERSHIP`] answers a name here and nowhere else, and
/// [`phase_type`] gives the phase's three types a written spelling here and
/// nowhere else — which is what keeps `02-core-calculus.md` §5's closed source
/// type grammar and its "no syntax value" sentence true of the language a
/// composer writes.
///
/// A module and not an expression. An adapter's operations read the module's
/// own `let`, `fn`, and `data`, because they are declarations of the module
/// those operations are declared in, and because a reader written without local
/// definitions is a reader nobody can follow (Peyton Jones ch. 3).
pub(crate) struct AdapterModule {
    values: IndexMap<String, Value>,
    types: IndexMap<String, Type>,
    /// The text of `print`, which is the one declaration not checked with the
    /// module.
    ///
    /// A printer's argument is the *package's* type — what its regions produce
    /// — and an adapter module imports nothing, so it cannot name that type and
    /// a standalone check of the printer would have nothing to settle its
    /// parameter against. So the printer is read where it is run, against the
    /// value it is handed ([`print_value`]), which is also why it is the one
    /// operation that never sees the phase environment. The cost is real and
    /// worth saying: a printer cannot call the module's other declarations.
    printer: Option<String>,
}

impl AdapterModule {
    /// The text a declaration holds, when it holds one.
    ///
    /// How `level` is read: the declared level is a `Text` the module
    /// evaluates to, so asking for it is asking the module for one of its own
    /// values rather than matching the shape of its source.
    pub(crate) fn text(&self, name: &str) -> Option<&str> {
        let Value::Text(held) = self.values.get(name)? else {
            return None;
        };
        Some(held)
    }

    /// Whether the module declares `name` at all.
    pub(crate) fn declares(&self, name: &str) -> bool {
        self.values.contains_key(name) || (name == "print" && self.printer.is_some())
    }

    /// The printer's source, for the one operation read at its use site.
    pub(crate) fn printer(&self) -> Option<&str> {
        self.printer.as_deref()
    }

    /// The operation `name`, if the module declares it at exactly `wanted`.
    ///
    /// Two ways to answer no, kept apart because they are different mistakes:
    /// a module that declares no `expand` is not the same as a module whose
    /// `expand` is not a transformer, and the second wants to say what type it
    /// found instead.
    fn operation(&self, name: &str, wanted: &Type) -> Result<&Closure, Option<&Type>> {
        let Some(found) = self.types.get(name) else {
            return Err(None);
        };
        if found != wanted {
            return Err(Some(found));
        }
        match self.values.get(name) {
            Some(Value::Closure(closure)) => Ok(closure),
            _ => Err(Some(found)),
        }
    }
}

/// Check and evaluate one adapter module, in the phase environment.
///
/// The diagnostics come back rather than being reported: they are about the
/// adapter package's own document, and publishing a span inside it as a span in
/// the composer's file is exactly what the source map exists to prevent. The
/// caller decides which of its own spans to restate them at.
pub(crate) fn read_adapter_module(source: &str) -> Result<AdapterModule, ModuleFault> {
    let mut meter = WorkMeter::default();
    read_adapter_module_metered(source, &mut Unifier::default(), &mut meter).map_err(|diagnostics| {
        if meter.failure().is_some() {
            ModuleFault::Stopped
        } else {
            ModuleFault::Broken(diagnostics)
        }
    })
}

/// Why a module could not be read as an adapter module.
///
/// Two cases and not one, for the reason [`ExpansionFailure`] separates the
/// same pair: a compilation that ran out of budget has said nothing about the
/// module, and reporting it as a broken adapter would make a narrowed budget
/// look like a package that does not compile.
pub(crate) enum ModuleFault {
    /// A compilation limit was crossed before the module finished checking.
    Stopped,
    /// It is not an adapter module, and these say why.
    Broken(Vec<Diagnostic>),
}

/// The same, charged to a run's own meter.
///
/// The phase reports what checking a module cost, because it is work the
/// compilation did: an adapter that is expensive to check is expensive whether
/// or not the region it reads is small.
fn read_adapter_module_metered(
    source: &str,
    unifier: &mut Unifier,
    meter: &mut WorkMeter,
) -> Result<AdapterModule, Vec<Diagnostic>> {
    let parsed = musa_language::parse(source);
    if let Some(error) = parsed.errors().first() {
        return Err(vec![Diagnostic::error(
            Code::Expansion,
            format!("it does not parse: {}", error.message()),
        )]);
    }
    let root = parsed.syntax();
    let Some(library) = musa_language::ast::LibraryDecl::from_root(&root) else {
        return Err(vec![
            Diagnostic::error(Code::Expansion, "an adapter module is a `library`").help(
                "write the module as `library { let level = …; let expand = …; }`, the way `stdlib/src/adapters/` does",
            ),
        ]);
    };
    // An adapter that imported would need the phase to resolve a package graph
    // before it can expand, and the phase runs before ordinary resolution. It
    // is refused rather than ignored: a module whose imports silently did
    // nothing would be a module whose author was misled.
    if root.descendants().any(|node| node.kind() == SyntaxKind::ImportStmt) {
        return Err(vec![
            Diagnostic::error(Code::Expansion, "an adapter module imports nothing").help(
                "expansion runs before ordinary resolution, so an adapter reads its own declarations and the phase's \
                 operations",
            ),
        ]);
    }
    // The adapter-free bootstrap, checked rather than assumed. An adapter whose
    // own definition needed an adapter would put the expansion order back into
    // a cycle, and this is the whole of what prevents it — which is also why
    // termination is structural and needs no rank arithmetic.
    if root.descendants().any(|node| node.kind() == SyntaxKind::SyntaxRegion) {
        return Err(vec![
            Diagnostic::error(Code::Expansion, "it is written with an adapter of its own")
                .help("an adapter is written in the adapter-free bootstrap: no region, no syntax import"),
        ]);
    }
    let mut resolver = Resolver::new();
    let owners = [library.syntax().clone()];
    let world = World::read_in_phase(&mut resolver, &owners);
    let modules = Modules::read(&mut resolver, &world, std::iter::once((None, library.syntax().clone())));
    let printer = printer_source(library.syntax());
    let program = check_and_evaluate_metered(
        &mut resolver,
        declarations(library.syntax(), None)
            .into_iter()
            .filter(|declaration| surface_identity(declaration).is_none_or(|(name, ..)| name != "print")),
        None,
        UnknownRootMusic::Reject,
        &modules,
        &world,
        Reading::Expansion,
        &phase_operations(),
        unifier,
        meter,
    );
    let Some(program) = program else {
        return Err(if resolver.diagnostics.is_empty() {
            vec![Diagnostic::error(Code::Expansion, "it does not check")]
        } else {
            resolver.diagnostics
        });
    };
    // A checked program is not yet a checked *module*: the declaration world is
    // read before the expressions are, and its refusals — a field storing an
    // arrow, a field storing a sealed step — are recorded there. Answering
    // `Ok` while the resolver holds an error would be an adapter running with
    // a declaration the checker had already refused.
    let refusals: Vec<Diagnostic> = resolver
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.severity == crate::diagnose::Severity::Error)
        .cloned()
        .collect();
    if !refusals.is_empty() {
        return Err(refusals);
    }
    Ok(AdapterModule {
        values: program.values,
        types: program.types,
        printer,
    })
}

/// The type of each operation the phase runs, by the name it is declared under.
///
/// The phase's interface, stated once. An adapter does not get to infer these:
/// `expand`'s error half is how it refuses, and an adapter that never refuses
/// would otherwise leave that half open and be told its own module does not say
/// what it holds — a complaint about a type that was never the module's.
fn phase_operations() -> IndexMap<String, Type> {
    IndexMap::from([
        (
            "expand".to_owned(),
            Type::Function(
                vec![Type::Syntax],
                Box::new(Type::Sum(
                    Box::new(Type::Syntax),
                    Box::new(Type::Product(vec![Type::Syntax, Type::Text])),
                )),
            ),
        ),
        (
            "edit".to_owned(),
            Type::Function(
                vec![Type::Syntax, Type::Text, Type::Nat, Type::Text],
                Box::new(Type::Sum(
                    Box::new(Type::List(Box::new(Type::Product(vec![Type::Nat, Type::Text])))),
                    Box::new(Type::Text),
                )),
            ),
        ),
    ])
}

/// The text of `let print = <this>;`, if the module declares one.
///
/// The one declaration read as text rather than as a value, for the reason
/// [`AdapterModule::printer`] gives. Everything between the `=` and the `;` and
/// nothing else: the name, the type, and the body's own tokens all lie between
/// the two marks rather than being one of them.
fn printer_source(library: &SyntaxNode) -> Option<String> {
    let declaration = root_nodes(library, SyntaxKind::LetDecl)
        .into_iter()
        .find(|declaration| declared_name(declaration).as_deref() == Some("print"))?;
    let mut equals = None;
    let mut semicolon = None;
    for token in declaration.children_with_tokens().filter_map(|it| it.into_token()) {
        if token.kind() == SyntaxKind::Equals && equals.is_none() {
            equals = Some(usize::try_from(u32::from(token.text_range().end())).ok()?);
        } else if token.kind() == SyntaxKind::Semicolon {
            semicolon = Some(usize::try_from(u32::from(token.text_range().start())).ok()?);
        }
    }
    let text = library.to_string();
    let base = usize::try_from(u32::from(library.text_range().start())).ok()?;
    let from = equals?.checked_sub(base)?;
    let to = semicolon?.checked_sub(base)?;
    Some(text.get(from..to)?.trim().to_owned())
}

/// The name a `let` declares.
fn declared_name(declaration: &SyntaxNode) -> Option<String> {
    declaration
        .children_with_tokens()
        .filter_map(|it| it.into_token())
        .find(|token| token.kind() == SyntaxKind::Identifier)
        .map(|token| token.text().to_owned())
}

/// What running a transformer over a region produced, or why it did not.
///
/// The three failures are different mistakes and a transformer author reading
/// one should be told which they made, which is why this is a type rather than
/// a `None`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ExpansionFailure {
    /// The adapter read the region and would not have it.
    ///
    /// Not a fault: a transformer that answers `Err` has *worked*, and what it
    /// says is the adapter package's sentence about the composer's text. The
    /// node is how it points — `26-language-design-decision.md` §3.4 gives a
    /// transformer no way to read a source range, so it hands back a node it
    /// was given and the range is read from that.
    Refused {
        /// The adapter's own sentence.
        message: String,
        /// Where it pointed, when it pointed at something the composer wrote.
        /// `None` means the adapter pointed at a node it built itself, which
        /// has no text under it.
        at: Option<SourceSpan>,
    },
    /// A compilation limit was crossed before the run finished.
    ///
    /// Not a fault in the adapter and not a fault in the region: a transformer
    /// is total, so this is the meter stopping a run rather than a run that
    /// would not have stopped. It is its own case because
    /// `docs/rules/language/00-semantics.md` §2 makes the difference matter —
    /// a stop must not read as a file that is not well-typed.
    Stopped,
    /// The transformer did not check as `Syntax -> Syntax`.
    NotATransformer(Vec<Diagnostic>),
    /// It checked and then did not answer — a budget crossed, or the evaluator
    /// and the checker disagreeing, which is a compiler fault rather than a
    /// language effect.
    NoAnswer,
    /// It answered with something that is not a well-formed expression.
    NotAnExpression(crate::syntax::NotAnExpression),
}

/// Run one transformer expression over one region, in the phase environment.
///
/// A test helper, and the one place a bare `expand` expression is still wrapped
/// into a module for the phase to read: a law about the fold or about a builder
/// is about that expression, and making each such test write a whole `library`
/// around it would bury the law in ceremony. Everything else — the compiler's
/// own path, and every test about an adapter *module* — hands the phase a
/// module.
///
/// `region` is source text, read by the fixed reader Musa already has: this
/// driver does not extend the lexer or the grouper.
#[cfg(test)]
pub(crate) fn expand_region(
    transformer: &str,
    region: &str,
    expansion: crate::syntax::ExpansionPath,
) -> Result<crate::syntax::Syntax, ExpansionFailure> {
    let subject = crate::syntax::read_region(&musa_language::parse(region).syntax(), expansion);
    expand_syntax(
        &format!("library {{\n    let level = \"readable\";\n\n    let expand = {transformer};\n}}\n"),
        subject,
    )
    .0
}

/// The refusal an `Err((node, message))` carries.
///
/// The span comes from the node the adapter handed back, never from anything
/// the adapter computed: `SourceInfo` has no eliminator, so an adapter can
/// point at a node it holds and cannot say where a node is.
fn refusal_of(held: Value) -> Option<ExpansionFailure> {
    let Value::Product(parts) = held else {
        return None;
    };
    let [Value::Syntax(node), Value::Text(message)] = parts.as_slice() else {
        return None;
    };
    let at = match node.info() {
        crate::syntax::SourceInfo::Original { span, .. } => Some(*span),
        crate::syntax::SourceInfo::Generated(_) => None,
    };
    Some(ExpansionFailure::Refused {
        message: message.clone(),
        at,
    })
}

/// A stop when the meter stopped, and `otherwise` when it did not.
fn stopped_or(meter: &WorkMeter, otherwise: ExpansionFailure) -> ExpansionFailure {
    if meter.failure().is_some() {
        ExpansionFailure::Stopped
    } else {
        otherwise
    }
}

/// What one expansion charged the phases it used.
///
/// Two of `CompilerLimits`' four counters (`26-language-design-decision.md`
/// §3.5); the other two are the phase's own and are counted by
/// [`crate::expand`]. Read off the ordinary meter and the ordinary unifier,
/// because a transformer is checked and evaluated by the ordinary machinery
/// and a separate accounting of the same work would be a second opinion about
/// it.
pub(crate) struct PhaseWork {
    pub(crate) type_constraints: u64,
    pub(crate) evaluation_steps: u64,
}

/// Why an adapter's `edit` produced no patch.
///
/// The same three-way distinction [`ExpansionFailure`] makes, for the same
/// reasons, minus the two cases an editor cannot reach: an editor answers with
/// replacements rather than with syntax, so there is nothing for the expression
/// gate to reject. It is its own type rather than a reuse because the sentences
/// a reader gets differ — "this is not a transformer" is not what to tell
/// somebody whose `edit` is wrong.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum EditFailure {
    /// The adapter read the command and would not serve it.
    ///
    /// The package's own sentence, exactly as a refusal during expansion is.
    /// It carries no node: a command an adapter does not know is not about a
    /// place in the region, and pointing at one would be inventing a subject.
    Refused(String),
    /// A compilation limit was crossed before the run finished.
    Stopped,
    /// The `edit` did not check as an editor.
    NotAnEditor(Vec<Diagnostic>),
    /// It checked and then did not answer.
    NoAnswer,
}

/// One replacement an adapter asked for: the anchor of the node to replace, and
/// the text to put there.
///
/// An anchor and not a range. An adapter has no operation for reading a source
/// range and gains none here — it names a node it was given, in the one
/// vocabulary it and an editor share (prompt 127dcc), and the compiler owns the
/// translation to bytes. That makes §4's locality a fact about the type rather
/// than a property the phase has to hope for and check afterwards.
pub(crate) type AdapterPatch = (u64, String);

/// Run one adapter's `edit` over one already-read region.
///
/// The second of `26-language-design-decision.md` §4's declared operations, in
/// the same phase environment [`expand_syntax`] runs in and by the same
/// machinery: one ordinary expression, checked by Algorithm W against a wanted
/// type, evaluated by the total evaluator, metered by the ordinary meter. The
/// command is spelled as its name, the anchor it is about, and one text
/// argument, because the phase is type-blind and may not learn a package's
/// command type.
pub(crate) fn edit_syntax(
    adapter_source: &str,
    subject: crate::syntax::Syntax,
    command: &str,
    anchor: u64,
    argument: &str,
) -> (Result<Vec<AdapterPatch>, EditFailure>, PhaseWork) {
    let mut unifier = Unifier::default();
    let mut meter = WorkMeter::default();
    let answer = run_editor(
        adapter_source,
        subject,
        command,
        anchor,
        argument,
        &mut unifier,
        &mut meter,
    );
    let work = PhaseWork {
        type_constraints: unifier.constraints(),
        evaluation_steps: meter.steps(),
    };
    (answer, work)
}

fn run_editor(
    adapter_source: &str,
    subject: crate::syntax::Syntax,
    command: &str,
    anchor: u64,
    argument: &str,
    unifier: &mut Unifier,
    meter: &mut WorkMeter,
) -> Result<Vec<AdapterPatch>, EditFailure> {
    // `edit : Syntax × Text × Nat × Text -> Result<List<(Nat, Text)>, Text>`.
    // The anchor arrives as a number rather than inside the argument text
    // because a command spelled as one string would be one an adapter could
    // not take apart.
    let wanted = phase_operations().swap_remove("edit").unwrap_or(Type::Unit);
    let module = match read_adapter_module_metered(adapter_source, unifier, meter) {
        Ok(module) => module,
        Err(diagnostics) => {
            if meter.failure().is_some() {
                return Err(EditFailure::Stopped);
            }
            return Err(EditFailure::NotAnEditor(diagnostics));
        }
    };
    let function = match module.operation("edit", &wanted) {
        Ok(function) => function.clone(),
        Err(found) => return Err(EditFailure::NotAnEditor(vec![not_the_operation("edit", found)])),
    };
    let arguments = vec![
        Value::Syntax(Box::new(subject)),
        Value::Text(command.to_owned()),
        Value::Nat(anchor),
        Value::Text(argument.to_owned()),
    ];
    let applied = apply_closure(&function, arguments, meter, SourceSpan::new(0, 0));
    let Some(Value::Sum { error, held, .. }) = applied else {
        return Err(edit_stopped_or(meter, EditFailure::NoAnswer));
    };
    if error {
        let Value::Text(message) = *held else {
            return Err(EditFailure::NoAnswer);
        };
        return Err(EditFailure::Refused(message));
    }
    let Value::List { values, .. } = *held else {
        return Err(edit_stopped_or(meter, EditFailure::NoAnswer));
    };
    values
        .into_iter()
        .map(|value| {
            let Value::Product(parts) = value else {
                return Err(EditFailure::NoAnswer);
            };
            let [Value::Nat(anchor), Value::Text(text)] = parts.as_slice() else {
                return Err(EditFailure::NoAnswer);
            };
            Ok((*anchor, text.clone()))
        })
        .collect()
}

/// A stop when the meter stopped, and `otherwise` when it did not.
fn edit_stopped_or(meter: &WorkMeter, otherwise: EditFailure) -> EditFailure {
    if meter.failure().is_some() {
        EditFailure::Stopped
    } else {
        otherwise
    }
}

/// Why an adapter's `print` produced no source text.
///
/// [`Self::Loss`] is `26-language-design-decision.md` §4's `PrintLoss`, and it
/// is the one of these that is not a fault at all: the printer was handed a
/// value carrying something it cannot write down, and said so. A printer that
/// quietly dropped that detail would make the round-trip law true by making the
/// value smaller, which is why the operation answers with a `Result` rather
/// than with text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum PrintFailure {
    /// The adapter read the value and could not spell it — its own sentence.
    Loss(String),
    /// A compilation limit was crossed before the run finished.
    Stopped,
    /// The `print` did not check against the value it was handed.
    NotAPrinter(Vec<Diagnostic>),
    /// It checked and then did not answer.
    NoAnswer,
}

/// Run one adapter's `print` over one ordinary value.
///
/// The third of `26-language-design-decision.md` §4's declared operations, and
/// the only one that does **not** run in the phase environment: its input is an
/// ordinary evaluated value rather than syntax, so it is an ordinary total
/// package function and is read under the ordinary reading. A printer that
/// could reach [`SYNTAX_OWNERSHIP`] would be a second way to build syntax, from
/// a value, outside the one place expansion happens.
///
/// `value` is the value as an ordinary expression, because that is the one
/// spelling of a value this crate shares with anything outside it. The printer
/// and the value are checked as one application, so `A` is settled by
/// unification rather than declared: the phase never learns the package's type
/// and does not need to.
pub(crate) fn print_value(adapter_source: &str, value: &str) -> Result<String, PrintFailure> {
    let mut unifier = Unifier::default();
    let mut meter = WorkMeter::default();
    let module = match read_adapter_module_metered(adapter_source, &mut unifier, &mut meter) {
        Ok(module) => module,
        Err(diagnostics) => return Err(PrintFailure::NotAPrinter(diagnostics)),
    };
    let Some(printer) = module.printer() else {
        return Err(PrintFailure::NotAPrinter(vec![not_the_operation("print", None)]));
    };
    run_printer(printer, value, &mut unifier, &mut meter)
}

fn run_printer(
    printer: &str,
    value: &str,
    unifier: &mut Unifier,
    meter: &mut WorkMeter,
) -> Result<String, PrintFailure> {
    let parsed = musa_language::parse(&format!(
        "piece \"print\" {{\n  let subject = {value};\n  let printer = {printer};\n}}"
    ));
    let mut resolver = Resolver::new();
    let declarations = root_nodes(&parsed.syntax(), SyntaxKind::LetDecl);
    let bodies: Vec<SyntaxNode> = declarations
        .iter()
        .filter_map(|declaration| child_of(declaration, is_expr_node))
        .collect();
    let [subject, printer] = bodies.as_slice() else {
        return Err(PrintFailure::NotAPrinter(resolver.diagnostics));
    };
    // The value first, and the printer against the type it turned out to have.
    // `A` is settled by unification rather than declared: what a package's
    // regions produce is the package's business, and a phase that had to be
    // told it would be a phase that knows a type.
    let Some(subject) = check_ordinary(subject, None, &mut resolver, unifier, meter) else {
        return Err(print_failure(meter, &resolver));
    };
    let wanted = Type::Function(
        vec![unifier.resolve(&subject.ty)],
        Box::new(Type::Sum(Box::new(Type::Text), Box::new(Type::Text))),
    );
    let Some(printer) = check_ordinary(printer, Some(&wanted), &mut resolver, unifier, meter) else {
        return Err(print_failure(meter, &resolver));
    };
    let environment = IndexMap::new();
    let Some(argument) = eval(&subject, &environment, meter) else {
        return Err(print_stopped_or(meter, PrintFailure::NoAnswer));
    };
    let Some(Value::Closure(function)) = eval(&printer, &environment, meter) else {
        return Err(print_stopped_or(meter, PrintFailure::NoAnswer));
    };
    let applied = apply_closure(&function, vec![argument], meter, printer.span);
    let Some(Value::Sum { error, held, .. }) = applied else {
        return Err(print_stopped_or(meter, PrintFailure::NoAnswer));
    };
    let Value::Text(text) = *held else {
        return Err(print_stopped_or(meter, PrintFailure::NoAnswer));
    };
    if error { Err(PrintFailure::Loss(text)) } else { Ok(text) }
}

/// Check one expression under the ordinary reading, with nothing else in scope.
///
/// The environment a printer and the value it is handed are both read in: no
/// definitions, no module scope, and — the part that matters —
/// [`Reading::Foreign`] rather than [`Reading::Expansion`], so
/// [`SYNTAX_OWNERSHIP`] is not in scope. A printer that could build syntax
/// would be a second way to make an expansion, out of a value, away from the
/// one place expansion happens.
fn check_ordinary(
    node: &SyntaxNode,
    wanted: Option<&Type>,
    resolver: &mut Resolver,
    unifier: &mut Unifier,
    meter: &mut WorkMeter,
) -> Option<Expr> {
    let span = crate::resolve::trimmed_span(node);
    let mut checker = Checker {
        resolver,
        definitions: &[],
        symbols: &IndexMap::new(),
        locals: IndexMap::new(),
        unifier,
        dependencies: IndexMap::new(),
        mentioned: Vec::new(),
        reading: Reading::Foreign,
        failed: false,
        meter,
        music_role: None,
        definition_span: span,
        deferred_pitch: false,
        scope: crate::module::NameScope::empty(),
        modules: &Modules::default(),
        world: &World::default(),
        questions: Vec::new(),
        asked: 0,
        tail: false,
    };
    checker.check(node, wanted)
}

/// A stop when the meter stopped, and the checker's complaints when it did not.
fn print_failure(meter: &WorkMeter, resolver: &Resolver) -> PrintFailure {
    if meter.failure().is_some() {
        PrintFailure::Stopped
    } else {
        PrintFailure::NotAPrinter(resolver.diagnostics.clone())
    }
}

/// A stop when the meter stopped, and `otherwise` when it did not.
fn print_stopped_or(meter: &WorkMeter, otherwise: PrintFailure) -> PrintFailure {
    if meter.failure().is_some() {
        PrintFailure::Stopped
    } else {
        otherwise
    }
}

/// The text an ordinary expression evaluates to, for a law that compares two
/// values rather than two spellings of one.
///
/// The round-trip law of §4 is about *values*, and the fixture that carries it
/// states its equality as text equality — which is the fixture's own equality
/// function, not a structural comparison of the printed source, because
/// printing is allowed to normalize.
#[cfg(test)]
pub(crate) fn evaluate_text(expression: &str) -> Option<String> {
    let parsed = musa_language::parse(&format!("piece \"value\" {{\n  let it = {expression}\n}}"));
    let mut unifier = Unifier::default();
    let mut meter = WorkMeter::default();
    let mut resolver = Resolver::new();
    let body = root_nodes(&parsed.syntax(), SyntaxKind::LetDecl)
        .first()
        .and_then(|declaration| child_of(declaration, is_expr_node))?;
    let span = crate::resolve::trimmed_span(&body);
    let mut checker = Checker {
        resolver: &mut resolver,
        definitions: &[],
        symbols: &IndexMap::new(),
        locals: IndexMap::new(),
        unifier: &mut unifier,
        dependencies: IndexMap::new(),
        mentioned: Vec::new(),
        reading: Reading::Foreign,
        failed: false,
        meter: &mut meter,
        music_role: None,
        definition_span: span,
        deferred_pitch: false,
        scope: crate::module::NameScope::empty(),
        modules: &Modules::default(),
        world: &World::default(),
        questions: Vec::new(),
        asked: 0,
        tail: false,
    };
    let checked = checker.check(&body, Some(&Type::Text))?;
    let environment = IndexMap::new();
    let Value::Text(text) = eval(&checked, &environment, &mut meter)? else {
        return None;
    };
    Some(text)
}

/// Run one transformer over one already-read region, in the phase environment.
///
/// The work is reported whichever way the run came out, because what a run
/// cost does not depend on what it answered: an adapter that reads a whole
/// region and then refuses it has read a whole region, and a phase that
/// charged nothing for that would let a file buy unbounded reading by
/// arranging to be refused.
pub(crate) fn expand_syntax(
    adapter_source: &str,
    subject: crate::syntax::Syntax,
) -> (Result<crate::syntax::Syntax, ExpansionFailure>, PhaseWork) {
    let mut unifier = Unifier::default();
    let mut meter = WorkMeter::default();
    let answer = run_transformer(adapter_source, subject, &mut unifier, &mut meter);
    let work = PhaseWork {
        type_constraints: unifier.constraints(),
        evaluation_steps: meter.steps(),
    };
    (answer, work)
}

fn run_transformer(
    adapter_source: &str,
    subject: crate::syntax::Syntax,
    unifier: &mut Unifier,
    meter: &mut WorkMeter,
) -> Result<crate::syntax::Syntax, ExpansionFailure> {
    // `expand : Syntax -> Result<Syntax, (Syntax, Text)>`, which is
    // `26-language-design-decision.md` §3.4's operation with both halves. One
    // shape and not two: a phase that took either would be two interfaces
    // wearing one name.
    let wanted = phase_operations().swap_remove("expand").unwrap_or(Type::Unit);
    let module = match read_adapter_module_metered(adapter_source, unifier, meter) {
        Ok(module) => module,
        Err(diagnostics) => {
            if meter.failure().is_some() {
                return Err(ExpansionFailure::Stopped);
            }
            return Err(ExpansionFailure::NotATransformer(diagnostics));
        }
    };
    let function = match module.operation("expand", &wanted) {
        Ok(function) => function.clone(),
        Err(found) => {
            return Err(ExpansionFailure::NotATransformer(vec![not_the_operation(
                "expand", found,
            )]));
        }
    };
    let applied = apply_closure(
        &function,
        vec![Value::Syntax(Box::new(subject))],
        meter,
        SourceSpan::new(0, 0),
    );
    let Some(Value::Sum { error, held, .. }) = applied else {
        return Err(stopped_or(meter, ExpansionFailure::NoAnswer));
    };
    if error {
        return Err(refusal_of(*held).unwrap_or(ExpansionFailure::NoAnswer));
    }
    let Value::Syntax(produced) = *held else {
        return Err(stopped_or(meter, ExpansionFailure::NoAnswer));
    };
    // The gate again, here rather than only in `checked_expression`: a
    // transformer that never called the builtin has still produced output the
    // rest of the compiler will have to anchor diagnostics against.
    crate::syntax::check_expression(&produced).map_err(ExpansionFailure::NotAnExpression)?;
    Ok(*produced)
}

/// The complaint for a module that declares the wrong thing under a name the
/// phase runs, or nothing at all.
///
/// It says the type it found, because "this is not a transformer" is not
/// advice: the author wrote a function and wants to know which function the
/// phase was expecting.
fn not_the_operation(name: &str, found: Option<&Type>) -> Diagnostic {
    match found {
        None => Diagnostic::error(Code::Expansion, format!("it declares no `{name}`")),
        Some(found) => Diagnostic::error(
            Code::Expansion,
            format!("its `{name}` has type `{}`", crate::infer::plain_one(found)),
        ),
    }
    .help(match name {
        "expand" => {
            "an adapter declares `let expand = fn (region) { … };`, answering `Ok(syntax)` or `Err((node, why))`"
        }
        "edit" => "an adapter declares `let edit = fn (region, command, anchor, argument) { … };`",
        _ => "an adapter declares `let print = fn (value) { … };`, answering `Ok(text)` or `Err(loss)`",
    })
}

#[cfg(test)]
// A law suite reports a violated law by failing, which is what `panic!` and `expect` are for here }
// the crate's integration tests carry the same allowances for the same reason.
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    /// One binding's value, from a piece that declares only that binding.
    fn only(declarations: &str, name: &str) -> Value {
        let source = format!("piece \"law\" {{ {declarations} }}");
        let bindings = values(&source).unwrap_or_else(|| panic!("well-typed source was rejected: {source}"));
        bindings
            .get(name)
            .unwrap_or_else(|| panic!("`{name}` was not bound by: {source}"))
            .clone()
    }

    /// The value half of a `Result`, or a panic naming the refusal.
    fn accepted(value: &Value) -> Value {
        let Value::Sum { error, held, .. } = value else {
            panic!("not a `Result`")
        };
        assert!(!*error, "refused: {}", literal_key(held));
        held.as_ref().clone()
    }

    /// The error half's sentence, or a panic if the operation answered.
    fn refusal(value: &Value) -> String {
        let Value::Sum { error, held, .. } = value else {
            panic!("not a `Result`")
        };
        assert!(
            *error,
            "answered where the law expects a refusal: {}",
            literal_key(held)
        );
        let Value::Text(because) = held.as_ref() else {
            panic!("a refusal that is not a sentence")
        };
        because.clone()
    }

    /// A negative exact rational, written the only way this language can
    /// write one: no literal carries a sign, so `0 - 1/4` is subtraction, and
    /// its `Result` is opened by the ordinary match every `Result` is opened
    /// by. That the language has no negative *literal* is a separate question
    /// from whether it has negative values, and it has them.
    const NEGATIVE: &str = "let before: Ratio = match ratio_sub(0, 1/4) { Ok(found) -> found, Err(why) -> 0 };";

    /// The exact rational a source expression evaluates to, through
    /// whichever projection its type needs.
    fn exact(declarations: &str, name: &str) -> Ratio<i64> {
        let held = only(declarations, name);
        let (Value::Ratio(value) | Value::Duration(_, value) | Value::Position(_, value)) = held else {
            panic!("not an exact value: {}", literal_key(&held))
        };
        value
    }

    /// The exact arithmetic answers what the arithmetic says it answers.
    ///
    /// Stated as source rather than as calls to [`eval_builtin`], because the
    /// operation a composer can reach is the one under test: a builtin that
    /// evaluated correctly and could not be written down would pass a law
    /// written the other way.
    #[test]
    fn exact_arithmetic_answers_what_the_arithmetic_says() {
        for (expression, expected) in [
            ("ratio_add(3/4, 1/4)", Ratio::new(1, 1)),
            ("ratio_sub(3/4, 1/4)", Ratio::new(1, 2)),
            ("ratio_mul(3/4, 2/3)", Ratio::new(1, 2)),
            ("ratio_div(3/4, 3/2)", Ratio::new(1, 2)),
            ("duration_of(3/8)", Ratio::new(3, 8)),
            ("duration_add(one_eighth, one_eighth)", Ratio::new(1, 4)),
            ("duration_scale(one_eighth, 3)", Ratio::new(3, 8)),
            ("position_shift(here, one_eighth)", Ratio::new(9, 8)),
            ("position_between(here, later)", Ratio::new(3, 4)),
        ] {
            let source = format!(
                "let one_eighth: Duration<WrittenTime> = 1/8; \
                 let here: Position<WrittenTime> = position_of(1); \
                 let later: Position<WrittenTime> = position_of(7/4); \
                 let answer = {expression};"
            );
            let held = accepted(&only(&source, "answer"));
            let (Value::Ratio(actual) | Value::Duration(_, actual) | Value::Position(_, actual)) = held else {
                panic!("`{expression}` answered with {}", literal_key(&held))
            };
            assert_eq!(actual, expected, "`{expression}`");
        }
        assert_eq!(
            exact("let answer: Ratio = duration_ratio(3/8);", "answer"),
            Ratio::new(3, 8)
        );
        assert_eq!(
            exact(
                &format!("{NEGATIVE} let answer: Ratio = position_ratio(position_of(before));"),
                "answer"
            ),
            Ratio::new(-1, 4),
            "a position is signed: an instant before the origin is an ordinary position"
        );
    }

    /// Whole-number arithmetic, and the one way subtraction fails.
    #[test]
    fn whole_number_arithmetic_says_when_it_would_go_below_zero() {
        assert!(matches!(
            accepted(&only("let answer = nat_add(2, 3);", "answer")),
            Value::Nat(5)
        ));
        assert!(matches!(
            accepted(&only("let answer = nat_mul(2, 3);", "answer")),
            Value::Nat(6)
        ));
        assert!(matches!(
            only("let answer = nat_sub(5, 3);", "answer"),
            Value::Option {
                value: Some(held),
                ..
            } if matches!(*held, Value::Nat(2))
        ));
        assert!(
            matches!(
                only("let answer = nat_sub(3, 5);", "answer"),
                Value::Option { value: None, .. }
            ),
            "below zero is not a whole number, and the absence says so"
        );
    }

    /// Comparison, which is where an ordering that would otherwise have to be
    /// reconstructed from subtraction comes from.
    #[test]
    fn exact_values_compare_without_being_subtracted() {
        for (expression, expected) in [
            ("ratio_less(1/3, 1/2)", true),
            ("ratio_less(1/2, 1/3)", false),
            ("ratio_equal(2/4, 1/2)", true),
            ("duration_less(1/8, 1/4)", true),
            ("duration_equal(2/8, 1/4)", true),
            ("position_less(position_of(before), position_of(0))", true),
            ("position_equal(position_of(3/2), position_of(6/4))", true),
        ] {
            let source = format!("{NEGATIVE} let answer: Bool = {expression};");
            assert!(
                matches!(only(&source, "answer"), Value::Bool(found) if found == expected),
                "`{expression}`"
            );
        }
    }

    /// Every partial operation says which way it failed, in its own sentence.
    ///
    /// D2 puts partiality in the result type, so each of these is a value
    /// rather than a diagnostic — and a `Result` rather than an `Option`
    /// wherever there is more than one way to fail, because "it did not work"
    /// is not what a composer needs to read.
    #[test]
    fn every_partial_time_operation_states_its_own_refusal() {
        for (expression, expected) in [
            ("ratio_div(3/4, 0)", "an exact rational is not divided by zero"),
            (
                "duration_of(before)",
                "a duration is nonnegative, and this exact rational is below zero",
            ),
            (
                "duration_scale(1/4, before)",
                "a duration is nonnegative, and this exact rational is below zero",
            ),
            (
                "position_between(position_of(2), position_of(1))",
                "the second position is before the first, and a duration is nonnegative",
            ),
        ] {
            let source = format!("{NEGATIVE} let answer = {expression};");
            assert_eq!(refusal(&only(&source, "answer")), expected, "`{expression}`");
        }
    }

    /// The whole reason the coordinate is carried.
    ///
    /// A written beat and a number of seconds are different types, so no
    /// program can add one to the other — and the refusal is a type error at
    /// the site rather than a wrong answer later.
    #[test]
    fn a_written_duration_and_a_physical_one_are_not_the_same_type() {
        let mut unifier = Unifier::default();
        assert!(
            unifier
                .unify(
                    &Type::Duration(Coordinate::WrittenTime),
                    &Type::Duration(Coordinate::PhysicalTime)
                )
                .is_err(),
            "two coordinates unified, so the tag distinguishes nothing"
        );
        assert!(
            unifier
                .unify(
                    &Type::Duration(Coordinate::WrittenTime),
                    &Type::Position(Coordinate::WrittenTime)
                )
                .is_err(),
            "a duration unified with a position, which is the distinction §1 exists to draw"
        );
        assert!(
            !refusals("piece \"law\" { let d: Duration<PhysicalTime> = 1/4; let sum = duration_add(d, 1/4); }")
                .is_empty(),
            "a written duration was added to a physical one"
        );
    }

    /// There is no name for adding two positions, and that is the point.
    ///
    /// Beat 3 plus three beats is a position; beat 3 plus beat 5 is nothing.
    /// The way to forbid the second is to register no operation with that
    /// signature, so the law reads the registry rather than trying the
    /// spelling — a spelling test would pass the day someone added
    /// `position_plus`.
    #[test]
    fn no_compiler_owned_operation_adds_two_positions() {
        let position = Shape::Base(Base::Position(Coordinate::WrittenTime)).ty();
        for entry in &BUILTIN_OWNERSHIP {
            let Family::Delta { arguments, result } = entry.family else {
                continue;
            };
            let positions = arguments.iter().filter(|shape| shape.ty() == position).count();
            if positions < 2 {
                continue;
            }
            let answers_a_position = result.ty() == position
                || matches!(result, Shape::Result(value, _) if value.ty() == position)
                || matches!(result, Shape::Option(value) if value.ty() == position);
            assert!(
                !answers_a_position,
                "`{}` takes two positions and answers with one, which is addition on an affine space",
                entry.spelling
            );
        }
    }

    /// The vocabulary the language offers, the types this module reads, and
    /// the spellings it writes back are one vocabulary or they are three.
    #[test]
    fn every_offered_type_name_is_read_and_written_the_same_way() {
        for (name, _) in musa_language::BASE_TYPES {
            let read = named_type(name);
            assert!(read.is_some(), "`{name}` is offered to the composer but is not a type");
            assert_eq!(
                read.map(|ty| ty.to_string()).as_deref(),
                Some(*name),
                "`{name}` is read as a type that prints under another name"
            );
        }
    }

    /// The literal forms of the offered vocabulary, one per type that has one.
    ///
    /// Written as source and compiled rather than listed as [`Type`]s, so that
    /// "this type has a written form" is checked against the parser and the
    /// checker instead of asserted here. Everything absent — `NoteName`,
    /// `Degree`, `Frame`, `Triad`, `Roman`, `Voicing`, and the twelve-tone
    /// domains — is reached by applying a δ-builtin, which is the other half
    /// of the law below.
    const WRITTEN_LITERALS: &[(&str, &str)] = &[
        ("Bool", "true"),
        ("Nat", "3"),
        ("Ratio", "3/2"),
        ("Text", "\"a title\""),
        ("Duration<WrittenTime>", "1/2"),
        ("Pitch", "c4"),
        ("Interval", "P5"),
        ("Scale", "scale c major"),
        ("Key", "key c major"),
        ("ChordClass", "chord c major"),
        ("Music", "music { c4/4 }"),
    ];

    /// Every base type some δ-builtin answers with, at any depth of its result.
    ///
    /// A domain reached only inside an `Option` or a `Result` still counts as
    /// produced: `pitch_frame` is the only way to make a `Frame`, and it says
    /// *whether* it made one, so a `Frame` is a value a `match` arm holds even
    /// though no closed expression is annotated with it.
    fn produced_by_a_builtin(shape: Shape, found: &mut IndexSet<String>) {
        match shape {
            Shape::Base(base) => {
                found.insert(base.ty().to_string());
            }
            Shape::Option(inner) | Shape::List(inner) => produced_by_a_builtin(*inner, found),
            Shape::Result(value, error) => {
                produced_by_a_builtin(*value, found);
                produced_by_a_builtin(*error, found);
            }
            Shape::Product(members) => {
                for member in members {
                    produced_by_a_builtin(*member, found);
                }
            }
        }
    }

    /// `Unit` is the one type this language offers that nothing written
    /// produces, and that is a decision rather than an omission.
    ///
    /// `Unit` is offered because the compiler *prints* it: `drop`'s output port
    /// and `count`'s input port are typed `Unit`, so a composer reading a
    /// machine's type meets the word and must be able to write it in an
    /// annotation — which is what the round-trip law above is for. What a port
    /// type says is what flows, and `Unit` says nothing flows. It is not a
    /// value a composer ever holds, so the surface has no literal for it and
    /// [`crate::machine::PortShape::is_writable`] keeps it out of the one
    /// place a registered unit's type would demand one.
    ///
    /// The argument, and the two spellings refused, are in
    /// `docs/notes/research/core-calculus/19-unit-has-no-surface-value.md`. If
    /// a later prompt introduces the constructor
    /// `docs/rules/language/02-core-calculus.md` §5.3 leaves open, this law is
    /// where that reversal is stated.
    #[test]
    fn unit_is_the_one_offered_type_no_written_expression_produces() {
        use std::fmt::Write as _;

        let mut source = String::from("piece \"law\" { ");
        for (index, (ty, literal)) in WRITTEN_LITERALS.iter().enumerate() {
            write!(source, "let witness{index}: {ty} = {literal}; ").expect("a string accepts what is written to it");
        }
        source.push('}');
        let complaints = refusals(&source);
        assert!(
            complaints.is_empty(),
            "a literal this law calls written did not compile at the type it claims: {}",
            complaints
                .iter()
                .map(|complaint| complaint.message.clone())
                .collect::<Vec<_>>()
                .join("; ")
        );

        let mut produced: IndexSet<String> = WRITTEN_LITERALS.iter().map(|(ty, _)| (*ty).to_string()).collect();
        for entry in &BUILTIN_OWNERSHIP {
            if let Family::Delta { result, .. } = entry.family {
                produced_by_a_builtin(result, &mut produced);
            }
        }

        let unproduced: Vec<&str> = musa_language::BASE_TYPES
            .iter()
            .map(|(name, _)| *name)
            .filter(|name| !produced.contains(*name))
            .collect();
        assert_eq!(
            unproduced,
            ["Unit"],
            "a type the language offers must be one a composer can obtain a value of, by a literal or by a \
             builtin that answers with it; `Unit` is the one deliberate exception, and a second one is a \
             vocabulary entry nobody can use"
        );
    }

    /// Advice is for crossing a distinction, not for every mismatch. A slip
    /// between two unrelated types has nothing to say beyond the two names,
    /// and a help line there would be noise on the common case; a category
    /// error names the operation that crosses, one layer of container deep.
    #[test]
    fn only_a_kept_distinction_earns_a_help_line() {
        assert_eq!(
            crossing_help(&Type::Nat, &Type::Duration(Coordinate::WrittenTime)),
            None
        );
        assert_eq!(crossing_help(&Type::Music, &Type::Music), None);
        assert!(crossing_help(&Type::Scale, &Type::Key).is_some());
        assert!(crossing_help(&Type::Pitch, &Type::Degree).is_some());
        assert_eq!(
            crossing_help(
                &Type::Option(Box::new(Type::ChordClass)),
                &Type::Option(Box::new(Type::Roman)),
            ),
            crossing_help(&Type::ChordClass, &Type::Roman),
            "a numeral inside an `Option` is the same confusion as a numeral"
        );
        assert!(
            crossing_help(&Type::Nat, &Type::Function(Vec::new(), Box::new(Type::Nat))).is_some(),
            "a nullary function written bare is the commonest category error of all"
        );
    }

    /// Every spelling the language removed still reaches the type it named,
    /// so a file written against the old vocabulary gets the parser's one
    /// complaint and not a second one from here.
    #[test]
    fn every_removed_type_name_still_reaches_its_type() {
        for (was, now) in musa_language::RESPELLED_TYPES {
            if matches!(*now, "Option" | "List" | "Result") {
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

    /// How many value nodes evaluating `declarations` charges.
    ///
    /// The four tests below are about §4's charging locus, which is a claim
    /// about this number and not about whether one particular file fits: a
    /// program that names a value ten times must cost what one that names it
    /// once costs, and a chain of `n` constructions must grow by a constant
    /// per link. Reading the counter says that; watching an accept/reject
    /// boundary would only say that one hand-picked size happened to fit.
    fn charged_nodes(text: &str) -> u64 {
        let source = format!("piece \"law\" {{ {text} }}");
        let parsed = musa_language::parse(&source);
        let piece = musa_language::ast::PieceDecl::from_root(&parsed.syntax()).expect("a piece");
        let mut resolver = Resolver::new();
        let mut meter = WorkMeter::default();
        let mut unifier = Unifier::default();
        let world = World::read(&mut resolver, &[piece.syntax().clone()]);
        let program = check_and_evaluate_metered(
            &mut resolver,
            declarations(piece.syntax(), None).into_iter(),
            Some(piece.syntax()),
            UnknownRootMusic::Reject,
            &Modules::default(),
            &world,
            Reading::Source,
            &IndexMap::new(),
            &mut unifier,
            &mut meter,
        );
        assert!(
            program.is_some(),
            "well-typed source was rejected: {source}\n{:?}",
            resolver.diagnostics.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
        meter.nodes()
    }

    /// One binding's value from a piece that declares its own `data` types.
    ///
    /// [`values`] reads an empty world, which is right for pieces built only
    /// from the core types; a piece that declares a data type needs the world
    /// [`charged_nodes`] builds, or its constructors resolve to nothing.
    fn value_from_declared_data(text: &str, name: &str) -> Value {
        let source = format!("piece \"law\" {{ {text} }}");
        let parsed = musa_language::parse(&source);
        let piece = musa_language::ast::PieceDecl::from_root(&parsed.syntax()).expect("a piece");
        let mut resolver = Resolver::new();
        let world = World::read(&mut resolver, &[piece.syntax().clone()]);
        let program = check_and_evaluate(
            &mut resolver,
            declarations(piece.syntax(), None).into_iter(),
            Some(piece.syntax()),
            UnknownRootMusic::Reject,
            &Modules::default(),
            &world,
            Reading::Source,
        )
        .unwrap_or_else(|| {
            panic!(
                "well-typed source was rejected: {source}\n{:?}",
                resolver.diagnostics.iter().map(|d| &d.message).collect::<Vec<_>>()
            )
        });
        program
            .values
            .get(name)
            .unwrap_or_else(|| panic!("`{name}` was not bound by: {source}"))
            .clone()
    }

    /// Naming a value builds nothing, so it charges nothing.
    ///
    /// The version-1 charge was the value's whole shape at every expression it
    /// passed through, which made this program cost ten lists rather than one.
    /// That is what stopped prompt 127dcfa's staff adapter: a reader state
    /// threaded through a region is one value mentioned a great many times.
    #[test]
    fn naming_a_value_costs_nothing_beyond_building_it() {
        let once = charged_nodes("let held = [1, 2, 3, 4, 5, 6, 7, 8]; let one = held;");
        let ten = charged_nodes(
            "let held = [1, 2, 3, 4, 5, 6, 7, 8]; let one = held; let two = held; let three = held; \
             let four = held; let five = held; let six = held; let seven = held; let eight = held; \
             let nine = held; let ten = held;",
        );
        assert_eq!(
            once, ten,
            "naming a value nine more times charged more nodes, so a mention is being charged as a construction"
        );
    }

    /// A constructor holds its fields; it does not build them again.
    #[test]
    fn holding_a_value_in_a_constructor_costs_one_cell() {
        const BOX: &str = "data Box {\n    Box(held: List<Nat>)\n}\n";
        let short = charged_nodes(&format!("{BOX} let held = [1, 2]; let put = Box(held);"))
            - charged_nodes(&format!("{BOX} let held = [1, 2];"));
        let long = charged_nodes(&format!(
            "{BOX} let held = [1, 2, 3, 4, 5, 6, 7, 8]; let put = Box(held);"
        )) - charged_nodes(&format!("{BOX} let held = [1, 2, 3, 4, 5, 6, 7, 8];"));
        assert_eq!(short, long, "wrapping a longer list in the same constructor cost more");
        assert_eq!(
            short, 2,
            "a one-field constructor should cost its own cell and its one field"
        );
    }

    /// A chain of constructions is linear in its length.
    ///
    /// The right-nested chain is the shape a notation package gives a sequence
    /// of items, so this is the growth an adapter that reads a page pays. Under
    /// the version-1 charge each link re-charged the whole tail and the chain
    /// was quadratic.
    #[test]
    fn a_chain_of_constructions_grows_by_a_constant_per_link() {
        fn chain(links: usize) -> u64 {
            let mut source = "data Chain {\n    End,\n    Link(next: Chain),\n}\n let held0 = End;".to_owned();
            for link in 1..=links {
                use std::fmt::Write as _;
                let _ = write!(source, " let held{link} = Link(held{});", link - 1);
            }
            charged_nodes(&source)
        }
        let early = (chain(8) - chain(4)) / 4;
        let late = (chain(16) - chain(8)) / 8;
        assert_eq!(
            early, late,
            "a chain's cost per link changed with its length, so a link is re-charging its tail"
        );
        assert_eq!(early, 2, "a one-field link should cost its own cell and its one field");
    }

    /// A closure's environment is its bindings, not their contents.
    #[test]
    fn capturing_a_value_costs_the_binding_and_not_the_value() {
        const TAKE: &str = " let take = fn (ignored: Nat) { held };";
        let short = "let held = [1, 2];";
        let long = "let held = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12];";
        let capturing_short = charged_nodes(&format!("{short}{TAKE}")) - charged_nodes(short);
        let capturing_long = charged_nodes(&format!("{long}{TAKE}")) - charged_nodes(long);
        assert_eq!(
            capturing_short, capturing_long,
            "the closure charged more for capturing a longer list, so a capture is being charged as a copy"
        );
        assert_eq!(
            capturing_short, 2,
            "a closure over one name should cost its own cell and its one binding"
        );
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
            &World::default(),
            Reading::Source,
        )
        .map(|program| program.values)
    }

    #[test]
    fn every_compiler_owned_operation_names_its_hidden_information() {
        let entries = BUILTIN_OWNERSHIP
            .iter()
            .map(|entry| (entry.spelling, entry.hidden_information))
            .collect::<Vec<_>>();
        assert_eq!(
            entries.len(),
            111,
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

    // --- `docs/rules/language/02-core-calculus.md` §5.8: the conservative-extension laws ---
    //
    // Theorem 5 holds for any base type with no eliminator and any δ-builtins satisfying D1–D4.
    // These laws check its premises against the implementation, so that a later musical domain
    // costs a registry entry rather than a new induction and cannot be added without discharging
    // them. They say nothing about whether the musical content is right; that is what the domain
    // law suites in `crates/musa-compiler/tests/` are for.

    /// How far the natural samples run. Thirteen covers every ordinal, member count, inversion,
    /// transposition index, and voice position the current domains accept, plus the first value
    /// past each — which is the half that matters, since D2 is about total answers on the whole
    /// declared domain rather than correct answers on the intended one.
    const SAMPLED_NATS: u64 = 13;

    /// How many times the sample pool is closed under the δ-builtins. Three rounds is what it
    /// takes to reach every constructed domain from the seeds: degrees and pitch classes appear in
    /// the first, frames, chords, rows and sets in the second, triads and voicings in the third.
    /// Durations and positions arrive in the first too, out of the exact rationals the seed piece
    /// writes: `duration_of` and `position_of` are the only ways to make one, and both answer from
    /// a `Ratio`.
    const SAMPLE_ROUNDS: usize = 3;

    /// How many applications each δ-builtin is sampled at. The pool is deliberately not
    /// exhausted combinatorially: a three-argument builtin over a pool of forty would be sixty
    /// thousand evaluations for no additional coverage of the property being checked. The budget
    /// is spent per *builtin* rather than per argument position so that a unary builtin sees
    /// its whole domain — which is how all twelve pitch classes reach the pool.
    const SAMPLED_APPLICATIONS: usize = 64;

    /// How long a synthesized list sample is. Twelve, because the row domain's whole point is that
    /// a row is an ordering of all twelve pitch classes, and a sampler that never offered twelve
    /// distinct classes would only ever see `row12_of` decline.
    const SAMPLED_LIST_LENGTH: usize = 12;

    /// How many values one argument position draws, given the builtin's arity, so that the
    /// product stays within [`SAMPLED_APPLICATIONS`].
    const fn samples_per_argument(arity: usize) -> usize {
        match arity {
            0 | 1 => SAMPLED_APPLICATIONS,
            2 => 8,
            _ => 4,
        }
    }

    /// The type of an evaluated value, for the shapes a δ-builtin can return.
    ///
    /// Absent for `music`, closures, and builtins, none of which a δ signature can name — which is
    /// itself part of what the totality law checks.
    fn value_type(value: &Value) -> Option<Type> {
        Some(match value {
            Value::Bool(_) => Type::Bool,
            Value::Nat(_) => Type::Nat,
            Value::Ratio(_) => Type::Ratio,
            Value::Duration(coordinate, _) => Type::Duration(*coordinate),
            Value::Position(coordinate, _) => Type::Position(*coordinate),
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
            Value::Text(_) => Type::Text,
            Value::Option { member, .. } => Type::Option(Box::new(member.clone())),
            Value::Sum {
                value_type: value,
                error_type: error,
                ..
            } => Type::Sum(Box::new(value.clone()), Box::new(error.clone())),
            Value::List { member, .. } => Type::List(Box::new(member.clone())),
            Value::Product(members) => Type::Product(members.iter().map(value_type).collect::<Option<Vec<_>>>()?),
            // A declared value has no place in the builtin registry's
            // sample pool: no δ-builtin's signature can name one, because a
            // library declares it and the registry is the compiler's own.
            Value::Data { .. }
            | Value::Music(_)
            | Value::Closure(_)
            | Value::Primitive { .. }
            | Value::Machine { .. }
            // A phase-local value has no place in the source builtin
            // registry's sample pool either: no δ-builtin's signature can
            // name a syntax type, which is what keeps §5.8's four families
            // the four families of ordinary source.
            | Value::Syntax(_)
            | Value::NodePath(_)
            | Value::BindingPath(_)
            | Value::SyntaxStep(_)
            | Value::Builtin(_) => return None,
        })
    }

    /// The values every base type the seeds cannot reach is later constructed from.
    ///
    /// Only the domains with surface literals are seeded here. Everything else — degrees, frames,
    /// triads, numerals, voicings, and the twelve-tone domains — is reached by *applying the
    /// builtins*, which is why the closure below doubles as the totality check rather than
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
             let a_ratio: Ratio = 3/2; \
             let b_ratio: Ratio = 1/3; \
             let c_ratio: Ratio = 0; \
             let d_ratio: Ratio = 4; \
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
    /// reachable from any result: `pcset12_of` wants a `list[pc12]`, and the only builtin that
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
            if matches!(
                member,
                Type::Option(_) | Type::List(_) | Type::Sum(_, _) | Type::Product(_)
            ) {
                continue;
            }
            let ty = Type::List(Box::new(member.clone())).to_string();
            // An empty list and a full one: the first is the edge case every list builtin has to
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

    /// Evaluate one δ-builtin on already-evaluated arguments.
    fn apply(builtin: Builtin, arguments: &[(Type, Value)]) -> Option<Value> {
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
        eval_builtin(builtin, &exprs, &IndexMap::new(), &mut WorkMeter::default(), &site)
    }

    /// Every δ-builtin applied to every sampled argument tuple, with what it answered.
    fn sampled_applications() -> Vec<(Builtin, Shape, Option<Value>)> {
        let mut pool = sample_seeds();
        let mut observed = Vec::new();
        for _ in 0..SAMPLE_ROUNDS {
            let grouped = by_type(&pool);
            let mut discovered = Vec::new();
            for entry in &BUILTIN_OWNERSHIP {
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
                        // An option's, sum's, or list's members are themselves samples, which is
                        // how the pool reaches domains no seed can spell — `Row12` among them,
                        // since `row12_of` is the only way to make one and it answers with a sum.
                        if let Value::Option { value: Some(inner), .. } = value {
                            discovered.push(inner.as_ref().clone());
                        } else if let Value::Sum { held, .. } = value {
                            discovered.push(held.as_ref().clone());
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
        // One table, one fold: "classified exactly once" is the statement that every entry falls
        // into one of these arms, and a match is what makes that true rather than checked.
        let families = BUILTIN_OWNERSHIP.iter().map(|entry| entry.family);
        let (delta, eliminator, track, machine) = families.fold((0, 0, 0, 0), |(d, e, t, m), family| match family {
            Family::Delta { .. } => (d + 1, e, t, m),
            Family::Eliminator(_) => (d, e + 1, t, m),
            Family::Track => (d, e, t + 1, m),
            Family::Machine(_) => (d, e, t, m + 1),
        });
        assert_eq!(delta + eliminator + track + machine, BUILTIN_OWNERSHIP.len());
        assert_eq!(
            eliminator, 8,
            "the structural eliminators of §5.6 are nat_fold, list_fold_from_start, list_fold_from_end, option_fold, \
             map, filter, range, and repeat"
        );
        assert_eq!(
            track, 8,
            "the track builtins of §5.7 are transpose, stretch, retrograde, invert, shift, together, \
             map_note_pitches, and play"
        );
        assert_eq!(
            machine, 9,
            "the machine builtins of §2 are primitive, machine, identity, connect, beside, feedback, copy, \
             drop, and swap"
        );
        assert_eq!(
            delta + eliminator + track + machine,
            111,
            "a new compiler operation must be classified before it is admitted"
        );
    }

    /// The phase environment is a *language*, not a hole an expression is
    /// dropped into: an adapter module's own declarations are in scope in its
    /// operations, and the phase's types have a spelling there.
    ///
    /// This is prompt 127dc's own sentence — "an adapter definition is checked
    /// and evaluated in the phase environment, where the syntax types are in
    /// scope" — held to by a test rather than left to a comment, because the
    /// first implementation of it was a text splice with no scope at all and
    /// nothing noticed for four prompts.
    #[test]
    fn an_adapter_module_reads_its_own_declarations_and_spells_the_phases_types() {
        let module = "library {
    let level = \"readable\";

    data Seen {
        Nothing,
        One(node: Syntax),
    }

    let held = fn (node: Syntax) { One(node) };

    let first = fn (found: Seen, later: Seen) {
        match found {
            One(node) -> One(node),
            Nothing -> later,
        }
    };

    let expand = fn (region) {
        match first(held(region), Nothing) {
            One(node) -> Ok(node),
            Nothing -> Err((region, \"a region is always a node\")),
        }
    };
}
";
        let subject = crate::syntax::read_region(
            &musa_language::parse("let melody = c4").syntax(),
            crate::syntax::ExpansionPath::at(vec![0]),
        );
        assert!(
            expand_syntax(module, subject).0.is_ok(),
            "a `data` holding a `Syntax`, a parameter annotated `Syntax`, and a sibling `fn` are all the module's own"
        );
    }

    /// An adapter reads its region and imports nothing: expansion runs before
    /// ordinary resolution, so there is no package graph for it to reach into.
    #[test]
    fn an_adapter_module_that_imports_is_refused_with_a_sentence() {
        let module = "library {\n    import std::notation::staff;\n\n    let level = \"readable\";\n}\n";
        let Err(ModuleFault::Broken(diagnostics)) = read_adapter_module(module) else {
            panic!("an adapter that imports is refused")
        };
        assert!(
            diagnostics
                .first()
                .is_some_and(|first| first.message.contains("imports nothing")),
            "and it says which rule it broke: {diagnostics:?}"
        );
    }

    /// A mistake inside an adapter module is the *adapter's* mistake, and its
    /// diagnostics say so rather than landing on the composer's region.
    #[test]
    fn a_diagnostic_inside_an_adapter_module_stays_in_that_module() {
        let module =
            "library {\n    let level = \"readable\";\n\n    let expand = fn (region) { Ok(nowhere(region)) };\n}\n";
        let Err(ModuleFault::Broken(diagnostics)) = read_adapter_module(module) else {
            panic!("an unbound name is a broken module")
        };
        assert!(
            diagnostics
                .iter()
                .any(|diagnostic| diagnostic.message.contains("nowhere")),
            "the module's own diagnostics are what come back: {diagnostics:?}"
        );
    }

    /// `syntax_number` hands over the reading the lexer already performed, for
    /// both numeric token kinds and for nothing else.
    ///
    /// Read through a refusal, because a refusal's sentence is the one `Text` a
    /// transformer's answer can carry out of the phase.
    #[test]
    fn the_reader_hands_a_transformer_the_number_it_already_read() {
        let read = |region: &str| {
            let transformer = "fn (region) {
                Err((region, syntax_fold_from_leaves(
                    fn (here) { \"none\" },
                    fn (here, kind, text) {
                        option_fold(\"none\", fn (node) {
                            option_fold(\"none\", fn (value) {
                                match ratio_equal(value, 3/8) {
                                    true -> \"three eighths\",
                                    false -> \"another number\",
                                }
                            }, syntax_number(node))
                        }, syntax_at(region, here))
                    },
                    fn (here, name) { \"none\" },
                    fn (here, delimiter, children) {
                        list_fold_from_start(\"none\", fn (child, found) {
                            match text_equal(found, \"none\") { true -> child, false -> found }
                        }, children)
                    },
                    region
                )))
            }";
            match expand_region(transformer, region, crate::syntax::ExpansionPath::at(vec![0])) {
                Err(ExpansionFailure::Refused { message, .. }) => message,
                other => panic!("this transformer refuses: {other:?}"),
            }
        };
        assert_eq!(
            read("let held = 3/8"),
            "three eighths",
            "a rational token the lexer kept whole"
        );
        assert_eq!(
            read("let held = 4"),
            "another number",
            "an integer token is a number too"
        );
        assert_eq!(
            read("let held = c5"),
            "none",
            "and a token that is not a number is not one"
        );
    }

    /// `text_equal` is an ordinary δ-builtin: `Text` is a base type, and its
    /// equality was simply never registered.
    #[test]
    fn two_texts_compare_in_ordinary_source() {
        let same = only(
            "let same = text_equal(\"f5\", \"f5\"); let apart = text_equal(\"f5\", \"f#5\");",
            "same",
        );
        assert!(matches!(same, Value::Bool(true)), "two spellings of one text agree");
        let apart = only(
            "let same = text_equal(\"f5\", \"f5\"); let apart = text_equal(\"f5\", \"f#5\");",
            "apart",
        );
        assert!(matches!(apart, Value::Bool(false)), "and two different ones do not");
    }

    /// `02-core-calculus.md` §5 closes the source type grammar and says the
    /// source language has no syntax value; §5.8 fixes four builtin families.
    /// This prompt adds a calculus without touching either, and the way it
    /// does so is structural rather than promised: the phase types have no
    /// written spelling at all, and the phase builtins are looked up in a
    /// registry ordinary source never reads.
    #[test]
    fn ordinary_source_can_neither_name_a_syntax_type_nor_obtain_a_syntax_value() {
        for name in ["Syntax", "NodePath", "BindingPath"] {
            assert!(
                named_type(name).is_none(),
                "`{name}` is nameable from ordinary source, which §5's grammar forbids"
            );
            assert!(
                !musa_language::BASE_TYPES.iter().any(|(offered, _)| *offered == name),
                "`{name}` is offered to the composer as a type"
            );
        }
        for entry in &SYNTAX_OWNERSHIP {
            assert!(
                Builtin::named(entry.spelling).is_none(),
                "`{}` is reachable from the name table ordinary source reads",
                entry.spelling
            );
            let source = format!("piece \"one\" {{\n  let refused = {}(1)\n}}", entry.spelling);
            assert!(
                !refusals(&source).is_empty(),
                "`{}` resolved in ordinary source, so a piece can obtain a syntax value",
                entry.spelling
            );
        }
        // And the type names, through the checker rather than through the
        // table: `phase_type` answers only under `Reading::Expansion`, so a
        // piece annotating a parameter `Syntax` gets the same "cannot find" any
        // other unbound type earns.
        for name in ["Syntax", "NodePath", "BindingPath", "SyntaxStep<Nat, Nat>"] {
            let source = format!("piece \"one\" {{\n  let refused = fn (node: {name}) {{ node }}\n}}");
            assert!(
                !refusals(&source).is_empty(),
                "ordinary source annotated a parameter `{name}`"
            );
        }
    }

    /// The phase registry is a second registry, not a fifth family.
    #[test]
    fn the_phase_registry_is_separate_and_classified() {
        assert_eq!(
            SYNTAX_OWNERSHIP.len(),
            14,
            "a new phase operation must enter the phase registry"
        );
        let spellings = SYNTAX_OWNERSHIP
            .iter()
            .map(|entry| entry.spelling)
            .collect::<IndexSet<_>>();
        assert_eq!(
            spellings.len(),
            SYNTAX_OWNERSHIP.len(),
            "phase operation spellings must be unique"
        );
        assert!(
            SYNTAX_OWNERSHIP
                .iter()
                .all(|entry| !entry.hidden_information.trim().is_empty()),
            "a phase operation earns its place by hiding something a library could not"
        );
        for entry in &SYNTAX_OWNERSHIP {
            assert!(
                !BUILTIN_OWNERSHIP.iter().any(|source| source.spelling == entry.spelling),
                "`{}` is in both registries, so §5.8's four families would have gained a member",
                entry.spelling
            );
        }
        let (folds, builders) = SYNTAX_OWNERSHIP
            .iter()
            .fold((0_usize, 0_usize), |(f, b), entry| match entry.family {
                PhaseFamily::Fold => (f.saturating_add(1), b),
                PhaseFamily::Builder => (f, b.saturating_add(1)),
            });
        // Three, and all three are one function: `recurse_syntax` descends,
        // `run_syntax_step` resumes a descent it did not start, and
        // `syntax_fold_from_leaves` is the first at a context nothing reads.
        // Nothing else in the registry enters a syntax value at all.
        assert_eq!(folds, 3, "descent into a syntax value happens in exactly one place");
        assert_eq!(folds + builders, SYNTAX_OWNERSHIP.len());
    }

    /// Every refusal one adapter module earns, by the sentence it earns it
    /// with. The module is checked whole: a declaration the checker refused is
    /// a refused module even where the expressions around it check.
    fn adapter_refusals(module: &str) -> Vec<String> {
        match read_adapter_module(module) {
            Ok(_) => Vec::new(),
            Err(ModuleFault::Broken(diagnostics)) => {
                diagnostics.into_iter().map(|diagnostic| diagnostic.message).collect()
            }
            Err(_) => vec!["it is not a module at all".to_owned()],
        }
    }

    /// One adapter module around `body`, which is spliced in above `expand`.
    fn adapter_module(body: &str) -> String {
        format!(
            "library {{\n    let level = \"readable\";\n\n{body}\n\n    let expand = fn (region) {{ Ok(region) }};\n}}\n"
        )
    }

    /// Law 1: a sealed step is minted by the recursor and by nothing else.
    ///
    /// The type is *spellable* — a group branch is worth factoring out, and
    /// `fn read_group(state, here, delimiter, kids: List<SyntaxStep<C, A>>)`
    /// is a definition an adapter should be able to write. What no adapter can
    /// do is make one: there is no constructor of that name, no other type
    /// coerces to it, and it cannot be stored in a declaration and carried out
    /// of the traversal that sealed it.
    #[test]
    fn law_1_a_sealed_step_is_minted_by_the_recursor_and_by_nothing_else() {
        // Spellable, and usable where one is already in hand.
        assert!(
            adapter_refusals(&adapter_module(
                "    let run_it = fn (sealed: SyntaxStep<Text, Text>) { run_syntax_step(\"c\", sealed) };"
            ))
            .is_empty(),
            "an adapter may name the type of the value its group branch is handed"
        );
        // Written at the wrong size it names no type at all, so the mistake is
        // reported where it was written rather than at a later mismatch.
        assert!(
            adapter_refusals(&adapter_module(
                "    let run_it = fn (sealed: SyntaxStep<Text>) { sealed };"
            ))
            .iter()
            .any(|message| message.contains("takes 2 type arguments")),
            "`SyntaxStep` written with one argument is refused where it is written"
        );
        // No constructor: the name is a type and not a term.
        assert!(
            adapter_refusals(&adapter_module(
                "    let forged = fn (node: Syntax) { SyntaxStep(node) };"
            ))
            .iter()
            .any(|message| message.contains("cannot find `SyntaxStep`")),
            "a step has no source constructor"
        );
        // And nothing else is one. A region is a `Syntax`, which is what the
        // recursor descends into — not what running a step resumes.
        assert!(
            adapter_refusals(
                "library {\n    let level = \"readable\";\n\n    let expand = fn (region) { Ok(run_syntax_step(\"c\", \
                 region)) };\n}\n"
            )
            .iter()
            .any(|message| message.contains("expected `SyntaxStep")),
            "a raw region is not a step, so minting stays the only introduction"
        );
        // Not storable, directly or inside a container: a stored step would
        // outlive the traversal that sealed it, which is what sealing is for.
        for field in ["SyntaxStep<Text, Text>", "List<SyntaxStep<Text, Text>>"] {
            let module = adapter_module(&format!(
                "    data Held {{\n        Nothing,\n        One(held: {field}),\n    }}"
            ));
            assert!(
                adapter_refusals(&module)
                    .iter()
                    .any(|message| message.contains("may not be a sealed step")),
                "a `data` field of type `{field}` was accepted"
            );
        }
    }

    /// Law 8: a step tells nothing about itself, and running it is the only
    /// question it answers.
    ///
    /// Stated over the registry rather than over a list of attempts, because
    /// the property is about what operations *exist*: one takes a step, none
    /// answers with one, and none turns one back into the node, the path, or
    /// the algebra it was sealed with.
    #[test]
    fn law_8_the_only_question_a_sealed_step_answers_is_the_one_that_runs_it() {
        fn holds_a_step(ty: &Type) -> bool {
            matches!(ty, Type::SyntaxStep { .. }) || crate::infer::member_types(ty).into_iter().any(holds_a_step)
        }
        let mut takes_one = Vec::new();
        let mut answers_one = Vec::new();
        for entry in &SYNTAX_OWNERSHIP {
            let mut unifier = Unifier::default();
            let Type::Function(arguments, result) = entry.operation.instantiate(&mut unifier) else {
                panic!("`{}` is written with arguments", entry.spelling)
            };
            if arguments
                .iter()
                .any(|argument| matches!(argument, Type::SyntaxStep { .. }))
            {
                takes_one.push(entry.spelling);
            }
            if holds_a_step(&result) {
                answers_one.push(entry.spelling);
            }
        }
        assert_eq!(
            takes_one,
            ["run_syntax_step"],
            "exactly one operation takes a step, and it is the one that runs it"
        );
        assert!(
            answers_one.is_empty(),
            "no operation answers with a step: minting happens inside the recursor, into the branch it hands it to \
             ({answers_one:?})"
        );
        // And from the adapter's side: a step is not a node, so every
        // operation that reads a node refuses one.
        let branch = |reading: &str| {
            format!(
                "library {{\n    let level = \"readable\";\n\n    let expand = fn (region) {{ Ok(recurse_syntax(\n     \
                    fn (c, here) {{ region }},\n        fn (c, here, kind, text) {{ region }},\n        fn (c, here, \
                 name) {{ region }},\n        fn (c, here, delimiter, kids) {{ {reading} }},\n        \"\", region)) \
                 }};\n}}\n"
            )
        };
        for reading in [
            "option_fold(region, fn (node) { node }, syntax_at(kids, here))",
            "option_fold(region, fn (node) { node }, syntax_at(map(fn (kid) { kid }, kids), here))",
        ] {
            assert!(
                !adapter_refusals(&branch(reading)).is_empty(),
                "a step reached an operation that reads a node: {reading}"
            );
        }
    }

    /// Check and evaluate one expression under one reading. Everything else —
    /// the terms, the checker, the evaluator — is the same either way.
    fn phase_nat(source: &str, reading: Reading) -> Option<u64> {
        let parsed = musa_language::parse(&format!("piece \"one\" {{\n  let answer = {source}\n}}"));
        let body = root_nodes(&parsed.syntax(), SyntaxKind::LetDecl)
            .first()
            .and_then(|declaration| child_of(declaration, is_expr_node))?;
        let mut resolver = Resolver::new();
        let mut unifier = Unifier::default();
        let mut meter = WorkMeter::default();
        let span = crate::resolve::trimmed_span(&body);
        let checked = {
            let mut checker = Checker {
                resolver: &mut resolver,
                definitions: &[],
                symbols: &IndexMap::new(),
                locals: IndexMap::new(),
                unifier: &mut unifier,
                dependencies: IndexMap::new(),
                mentioned: Vec::new(),
                reading,
                failed: false,
                meter: &mut meter,
                music_role: None,
                definition_span: span,
                deferred_pitch: false,
                scope: crate::module::NameScope::empty(),
                modules: &Modules::default(),
                world: &World::default(),
                questions: Vec::new(),
                asked: 0,
                tail: false,
            };
            checker.check(&body, None)
        }?;
        if let Some(Value::Nat(found)) = eval(&checked, &IndexMap::new(), &mut meter) {
            Some(found)
        } else {
            None
        }
    }

    /// Blocker 2 is closed by stating what prompt 127b already decided.
    /// Exhaustive `match` is one explicit form in the private evaluation core
    /// and there is no join point, jump, or switch target beside it, so
    /// transformer code and ordinary source run on one match semantics. The
    /// phase environment offers more *names*; it does not offer another way
    /// to take a value apart.
    #[test]
    fn there_is_exactly_one_match_evaluator() {
        const CASES: [&str; 4] = [
            "match Some(2) { None -> 0, Some(n) -> n }",
            "match Some(2) { None -> 0, Some(n) -> nat_fold(n, fn (a, b) { a }, 4) }",
            "match range(3) { [] -> 0, [head, ..tail] -> head }",
            "match range(2) { [] -> 5, [head, ..tail] -> list_fold_from_start(head, fn (m, a) { a }, tail) }",
        ];
        for case in CASES {
            let ordinary = phase_nat(case, Reading::Source);
            assert!(ordinary.is_some(), "`{case}` did not run in ordinary source");
            assert_eq!(
                ordinary,
                phase_nat(case, Reading::Expansion),
                "`{case}` ran differently once the phase environment was in scope"
            );
        }
    }

    #[test]
    fn no_first_order_signature_mentions_a_function() {
        for entry in &BUILTIN_OWNERSHIP {
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
            Type::Sum(value, error) => mentions_function(value) || mentions_function(error),
            Type::Product(members) => members.iter().any(mentions_function),
            // A machine's ports are refused an arrow where the machine is built, but this law
            // reads a written signature rather than an inferred one, so it looks for itself.
            Type::Primitive { step, input, output } | Type::Machine { step, input, output } => {
                mentions_function(step) || mentions_function(input) || mentions_function(output)
            }
            // A sealed step hides an algebra of source closures, so it answers
            // the way an arrow does and for a stronger reason: an arrow *is* a
            // function, and this *holds* four of them.
            Type::SyntaxStep { .. } => true,
            // Listed rather than wildcarded: a new *type former* would otherwise be assumed
            // arrow-free, and this law is the only thing standing between that assumption and
            // §5.8's no-arrow premise. A registry signature is written, not inferred, so a
            // variable never reaches here — and if one did, it is not an arrow.
            Type::Var(_)
            | Type::Unit
            | Type::Bool
            | Type::Nat
            | Type::Ratio
            | Type::Duration(_)
            | Type::Position(_)
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
            | Type::Text
            // A declared type holds only what its fields hold, and a field
            // holding an arrow is refused where the declaration is written.
            | Type::Nominal(_, _)
            | Type::Step(_)
            | Type::Syntax
            | Type::NodePath
            | Type::BindingPath
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

    /// Two machines whose steps count different things do not connect
    /// (`docs/rules/across-stages/03-machine-calculus.md` §2).
    ///
    /// This law is stated here rather than beside the rest of them in
    /// `tests/suite/machine_laws.rs` because it needs two step tags and the
    /// governing grammar names one. A second tag exists only under `cfg(test)`,
    /// and an integration test links the ordinary build, where every registered
    /// unit counts frames and the law would pass without ever being asked.
    #[test]
    fn machines_that_count_different_things_do_not_connect() {
        let joined = refusals(
            "piece \"law\" { let m = connect(machine(primitive(\"scale\", 1, 3/2)), \
             machine(primitive(\"other_step\", 1, 3/2))); }",
        );
        assert!(
            joined.iter().any(|diagnostic| diagnostic.code == Code::TypeMismatch),
            "two machines with unlike steps were connected: {joined:?}"
        );

        let alone = refusals("piece \"law\" { let m = machine(primitive(\"other_step\", 1, 3/2)); }");
        assert!(
            alone.is_empty(),
            "the unit itself is well-formed; only the joining is refused: {alone:?}"
        );
    }

    /// The two list folds agree for addition and disagree for projection.
    ///
    /// Associativity and commutativity with a common unit are sufficient for
    /// the direction to be unobservable, but their absence is not an exact
    /// classification of every operation and input. The projection step below
    /// is a concrete witness that the two source operations are distinguishable.
    #[test]
    fn the_two_list_folds_agree_for_addition_and_disagree_for_projection() {
        const SUMS: &str = "fn plus(member: Nat, running: Nat) -> Nat { \
             match nat_add(member, running) { Ok(sum) -> sum, Err(why) -> 0 } } \
             let from_start: Nat = list_fold_from_start(0, plus, [1, 2, 3, 4]); \
             let from_end: Nat = list_fold_from_end(0, plus, [1, 2, 3, 4]);";
        assert_eq!(
            literal_key(&only(SUMS, "from_start")),
            literal_key(&only(SUMS, "from_end")),
            "addition is associative and commutative, so the direction must not be observable"
        );

        // `fn (member, running) { member }` keeps whichever member the step saw
        // last, so it reports the end the fold finished at.
        const KEEPS_THE_LAST_MEMBER_SEEN: &str = "fn latest(member: Nat, running: Nat) -> Nat { member } \
             let from_start: Nat = list_fold_from_start(0, latest, [1, 2, 3, 4]); \
             let from_end: Nat = list_fold_from_end(0, latest, [1, 2, 3, 4]);";
        assert_eq!(literal_key(&only(KEEPS_THE_LAST_MEMBER_SEEN, "from_start")), "nat:4");
        assert_eq!(literal_key(&only(KEEPS_THE_LAST_MEMBER_SEEN, "from_end")), "nat:1");
    }

    /// Both folds are total on the empty list, and both answer the seed.
    #[test]
    fn both_list_folds_are_total_on_the_empty_list() {
        const EMPTY: &str = "fn latest(member: Nat, running: Nat) -> Nat { member } \
             fn reject(member: Nat) -> Bool { false } \
             let empty: List<Nat> = filter(reject, range(3)); \
             let from_start: Nat = list_fold_from_start(7, latest, empty); \
             let from_end: Nat = list_fold_from_end(7, latest, empty);";
        assert_eq!(literal_key(&only(EMPTY, "from_start")), "nat:7");
        assert_eq!(literal_key(&only(EMPTY, "from_end")), "nat:7");
    }

    /// A generated `data` fold and `list_fold_from_end` are the same law read
    /// on two types, and `list_fold_from_start` is not.
    ///
    /// §5.6's equations are what a fold *denotes*, and a generated fold is the
    /// catamorphism of its declaration by construction: a case sees its
    /// recursive field already folded. `list` is a `data` declaration in
    /// everything but spelling, so its catamorphism has to answer what the
    /// generated one answers over the same members and the same step. Before
    /// this prompt only the accumulator fold was reachable from source, so the
    /// comparison had no left-hand side to run — the generated law was checked
    /// against the term written out by hand and never against the list
    /// eliminator that stands for it.
    #[test]
    fn a_generated_fold_and_the_list_catamorphism_answer_alike() {
        const DECLARATION: &str = "data Chain {\n    End,\n    Link(first: Nat, later: Chain),\n}\n \
             fn keep(first: Nat, later: Nat) -> Nat { first } \
             let built = list_fold_from_end(End, fn (member: Nat, later: Chain) -> Chain { Link(member, later) }, \
             [1, 2, 3, 4]); \
             let by_declaration: Nat = chain_fold(0, keep, built); \
             let by_catamorphism: Nat = list_fold_from_end(0, keep, [1, 2, 3, 4]); \
             let by_accumulator: Nat = list_fold_from_start(0, keep, [1, 2, 3, 4]);";
        assert_eq!(
            literal_key(&value_from_declared_data(DECLARATION, "by_declaration")),
            literal_key(&value_from_declared_data(DECLARATION, "by_catamorphism")),
            "the generated fold and the list catamorphism must be the same law on two types"
        );
        assert_ne!(
            literal_key(&value_from_declared_data(DECLARATION, "by_declaration")),
            literal_key(&value_from_declared_data(DECLARATION, "by_accumulator")),
            "if the accumulator fold answered this too the comparison would prove nothing"
        );
    }

    /// `list_fold_from_end` builds the same right-nested value as the closure
    /// chain it replaces, and costs strictly fewer nodes doing it.
    ///
    /// The chain is what `stdlib/src/adapters/staff.musa` wrote before this
    /// prompt: fold to a `Pending -> Pending` and apply it. Prompt 127dcec
    /// measured that at one closure and one extra application per element, and
    /// this is the same measurement against the eliminator that removes it.
    #[test]
    fn folding_from_the_end_builds_the_nesting_the_closure_chain_built() {
        // `rest` is spoken for by the notation, so the tail field is `later`.
        const DECLARATION: &str = "data Chain {\n    End,\n    Link(first: Nat, later: Chain),\n}\n \
             fn link(member, later) { Link(member, later) } ";
        const DIRECT: &str = "let built = list_fold_from_end(End, link, [1, 2, 3, 4, 5, 6, 7, 8]);";
        const CHAINED: &str = "let built = list_fold_from_start( \
             fn (later: Chain) -> Chain { later }, \
             fn (member: Nat, sofar: Chain -> Chain) -> Chain -> Chain { \
             fn (later: Chain) -> Chain { sofar(Link(member, later)) } }, \
             [1, 2, 3, 4, 5, 6, 7, 8])(End);";

        assert_eq!(
            literal_key(&value_from_declared_data(&format!("{DECLARATION}{DIRECT}"), "built")),
            literal_key(&value_from_declared_data(&format!("{DECLARATION}{CHAINED}"), "built")),
            "the eliminator must build exactly the value the closure chain built"
        );
        let direct = charged_nodes(&format!("{DECLARATION}{DIRECT}"));
        let chained = charged_nodes(&format!("{DECLARATION}{CHAINED}"));
        assert!(
            direct < chained,
            "the eliminator charged {direct} nodes and the closure chain {chained}; \
             the point of the eliminator is that it does not allocate a closure per element"
        );
    }

    /// The bare `list_fold` resolves to nothing, and says which name preserves
    /// the meaning the call already had.
    ///
    /// It is not an alias and not a deprecation. A direction change is the one
    /// break a reader cannot see — every call would keep compiling and start
    /// answering differently wherever the step is not symmetric — so the name
    /// is deleted and this diagnostic is the migration.
    #[test]
    fn the_bare_list_fold_is_rejected_with_the_name_that_keeps_its_meaning() {
        let refused = refusals("piece \"law\" { let n: Nat = list_fold(0, fn (m, a) { m }, [1, 2]); }");
        let named = refused
            .iter()
            .find(|diagnostic| diagnostic.code == Code::UnknownName)
            .unwrap_or_else(|| panic!("`list_fold` was not refused: {refused:?}"));
        assert!(
            named
                .help
                .as_ref()
                .is_some_and(|help| help.contains("list_fold_from_start") && help.contains("list_fold_from_end")),
            "the diagnostic must name both replacements: {named:?}"
        );
        assert!(
            named
                .fixes
                .iter()
                .any(|fix| fix.edits.iter().any(|edit| edit.replacement == "list_fold_from_start")),
            "the applicable fix must be the one that preserves the old meaning: {named:?}"
        );
    }

    /// A conditional and the boolean match it elaborates to are the same
    /// program — same answers, and the same number of charged nodes.
    ///
    /// This is the whole claim prompt 127dcfab makes. `if` adds no core term,
    /// no typing rule, no reduction, no measure case, and no cost-table entry,
    /// which is only true if the elaborated form is indistinguishable from the
    /// match an author would have written by hand. Equal values would allow a
    /// conditional that reached the same answer by a longer route; equal charge
    /// is what says it is the same route. The ladder is included because
    /// `else if` is not a form of its own — it is the alternative being another
    /// conditional — so a three-rung ladder must cost what three nested matches
    /// cost, not one frame more.
    #[test]
    fn a_conditional_is_the_boolean_match_it_elaborates_to() {
        const CONDITIONAL: &str = "fn count(said: Text) -> Nat { \
             if text_equal(said, \"none\") { 0 } \
             else if text_equal(said, \"one\") { 1 } \
             else { 9 } } \
             let of_none: Nat = count(\"none\"); \
             let of_one: Nat = count(\"one\"); \
             let of_other: Nat = count(\"four\");";
        const BY_HAND: &str = "fn count(said: Text) -> Nat { \
             match text_equal(said, \"none\") { \
             true -> 0, \
             false -> match text_equal(said, \"one\") { true -> 1, false -> 9 }, \
             } } \
             let of_none: Nat = count(\"none\"); \
             let of_one: Nat = count(\"one\"); \
             let of_other: Nat = count(\"four\");";

        let keyed = |source: &str| {
            values(&format!("piece \"law\" {{ {source} }}"))
                .unwrap_or_else(|| panic!("well-typed source was rejected: {source}"))
                .iter()
                .map(|(name, value)| (name.clone(), literal_key(value)))
                .collect::<Vec<_>>()
        };
        let conditional = keyed(CONDITIONAL);
        assert_eq!(
            conditional,
            [
                ("count".to_owned(), "constructor".to_owned()),
                ("of_none".to_owned(), "nat:0".to_owned()),
                ("of_one".to_owned(), "nat:1".to_owned()),
                ("of_other".to_owned(), "nat:9".to_owned()),
            ]
        );
        assert_eq!(conditional, keyed(BY_HAND), "the conditional answered differently");

        assert_eq!(
            charged_nodes(CONDITIONAL),
            charged_nodes(BY_HAND),
            "the conditional cost more than the match it is"
        );
    }

    /// A conditional has one result type, and the branch that disagrees is the
    /// one the diagnostic points at.
    ///
    /// The consequent is checked first and fixes the type; the alternative is
    /// then checked against it. That order is why the span lands on the second
    /// branch rather than on the whole `if`, which would ask the reader to work
    /// out which half was meant.
    #[test]
    fn a_conditional_whose_branches_disagree_names_the_branch_that_disagrees() {
        let source = "piece \"law\" { let m: Nat = if true { 1 } else { \"two\" }; }";
        let refused = refusals(source);
        let mismatch = refused
            .iter()
            .find(|diagnostic| diagnostic.message.contains("expected `Nat`"))
            .unwrap_or_else(|| panic!("branches of different types were accepted: {refused:?}"));
        let at = mismatch
            .labels
            .iter()
            .find(|label| label.primary)
            .expect("a type error points somewhere");
        assert_eq!(
            &source[at.span.start as usize..at.span.end as usize],
            "\"two\"",
            "the diagnostic did not point at the disagreeing branch: {mismatch:?}"
        );
    }

    /// What a conditional asks must be a `Bool`, and the refusal lands on the
    /// condition rather than on the conditional around it.
    #[test]
    fn a_condition_that_is_not_a_bool_is_refused_where_it_is_written() {
        let source = "piece \"law\" { let m: Nat = if 3 { 1 } else { 2 }; }";
        let refused = refusals(source);
        let mismatch = refused
            .iter()
            .find(|diagnostic| diagnostic.message.contains("expected `Bool`"))
            .unwrap_or_else(|| panic!("a non-boolean condition was accepted: {refused:?}"));
        let at = mismatch
            .labels
            .iter()
            .find(|label| label.primary)
            .expect("a type error points somewhere");
        assert_eq!(
            &source[at.span.start as usize..at.span.end as usize],
            "3",
            "the diagnostic did not point at the condition: {mismatch:?}"
        );
    }

    /// A record update and the construction it elaborates to are the same
    /// program — same fields afterwards, and the same number of charged nodes.
    ///
    /// This is the whole claim prompt 127dcfac makes. `with` adds no core term:
    /// it is a one-arm match on the subject and a use of the declaration's own
    /// constructor. Equal fields would allow an update that reached the same
    /// record by building an intermediate one; equal charge is what says it
    /// built exactly one. The many-field case is here for the same reason the
    /// ladder is in the conditional law — if a mentioned right-hand side were
    /// evaluated twice, or the subject rebuilt once per carried field, the
    /// meter would say so and inspection would not.
    #[test]
    fn an_update_is_the_construction_it_elaborates_to() {
        const DATA: &str = "data Pending {\n    Pending(read: Nat, length: Nat, dots: Nat)\n}\n";
        const UPDATED: &str = "let start = Pending(1, 2, 3); \
             let one = start with { dots = 9 }; \
             let many = start with { read = 7, dots = 8 }; \
             let one_read = match one { Pending(read, length, dots) -> read }; \
             let one_length = match one { Pending(read, length, dots) -> length }; \
             let one_dots = match one { Pending(read, length, dots) -> dots }; \
             let many_read = match many { Pending(read, length, dots) -> read }; \
             let many_length = match many { Pending(read, length, dots) -> length }; \
             let many_dots = match many { Pending(read, length, dots) -> dots };";
        const BY_HAND: &str = "let start = Pending(1, 2, 3); \
             let one = match start { Pending(read, length, dots) -> Pending(read, length, 9) }; \
             let many = match start { Pending(read, length, dots) -> Pending(7, length, 8) }; \
             let one_read = match one { Pending(read, length, dots) -> read }; \
             let one_length = match one { Pending(read, length, dots) -> length }; \
             let one_dots = match one { Pending(read, length, dots) -> dots }; \
             let many_read = match many { Pending(read, length, dots) -> read }; \
             let many_length = match many { Pending(read, length, dots) -> length }; \
             let many_dots = match many { Pending(read, length, dots) -> dots };";

        let read_back = |source: &str| {
            [
                "one_read",
                "one_length",
                "one_dots",
                "many_read",
                "many_length",
                "many_dots",
            ]
            .map(|name| literal_key(&value_from_declared_data(&format!("{DATA}{source}"), name)))
        };
        let updated = read_back(UPDATED);
        assert_eq!(
            updated,
            [
                // Mentioned, carried over, mentioned.
                "nat:1", "nat:2", "nat:9", "nat:7", "nat:2", "nat:8",
            ]
            .map(str::to_owned),
            "an unmentioned field was not carried over unchanged"
        );
        assert_eq!(
            updated,
            read_back(BY_HAND),
            "the update answered differently from the construction it is"
        );
        assert_eq!(
            charged_nodes(&format!("{DATA}{UPDATED}")),
            charged_nodes(&format!("{DATA}{BY_HAND}")),
            "the update cost more than the construction it is"
        );
    }

    /// An update charges one construction, and carrying a field over does not
    /// charge for what is in it.
    ///
    /// Prompt 127dcec charges a value where it is built. A carried-over field
    /// is not built again — the elaboration binds it and hands the same value
    /// back — so an update of a record holding a long list must cost what an
    /// update of one holding a short list costs. Under a copying reading it
    /// would not, and the `holding_*` functions this form replaces would have
    /// been the cheaper way to write it.
    #[test]
    fn an_update_charges_one_construction_and_does_not_copy_what_it_carries() {
        const BOX: &str = "data Box {\n    Box(held: List<Nat>, tag: Nat)\n}\n";
        let cost = |held: &str, built: &str| {
            let start = format!("{BOX} let held = {held}; let start = Box(held, 0);");
            charged_nodes(&format!("{start} let moved = {built};")) - charged_nodes(&start)
        };
        let short = cost("[1, 2]", "start with { tag = 1 }");
        let long = cost("[1, 2, 3, 4, 5, 6, 7, 8]", "start with { tag = 1 }");
        assert_eq!(
            short, long,
            "updating a record holding a longer list cost more, so a carried field is being copied"
        );
        assert_eq!(
            short,
            cost("[1, 2]", "Box(held, 1)"),
            "an update cost more than writing the same record out, so it is building more than one"
        );
    }

    /// A right-hand side reads the scope around the update, not the field of
    /// the same name.
    ///
    /// An update is not a `let` over the old fields. The binders the
    /// elaboration introduces are unspellable, so this is true by construction
    /// rather than by a renaming rule — but it is the part of the contract a
    /// reader is most likely to assume the other way, so it is checked rather
    /// than asserted.
    #[test]
    fn a_right_hand_side_reads_the_surrounding_binding_and_not_the_field() {
        const SOURCE: &str = "data Pending {\n    Pending(dots: Nat, tying: Nat)\n}\n \
             let dots = 5; \
             let start = Pending(1, 2); \
             let moved = start with { tying = dots }; \
             let carried = match moved { Pending(dots, tying) -> tying };";
        assert_eq!(
            literal_key(&value_from_declared_data(SOURCE, "carried")),
            "nat:5",
            "the right-hand side read the subject's field instead of the surrounding binding"
        );
    }

    /// A field named twice is refused, and both mentions are named.
    ///
    /// The second value would silently win, and nothing in the spelling says
    /// which one was meant — so the diagnostic points at the repeat and says
    /// where the first one was, in the shape every other duplicate diagnostic
    /// uses.
    #[test]
    fn a_field_named_twice_is_refused_at_both_mentions() {
        let source = "piece \"law\" { data Pending {\n    Pending(dots: Nat, tying: Nat)\n}\n \
             let start = Pending(1, 2); \
             let moved = start with { dots = 3, dots = 4 }; }";
        let refused = refusals_from_declared_data(source);
        let repeated = refused
            .iter()
            .find(|diagnostic| diagnostic.message.contains("dots"))
            .unwrap_or_else(|| panic!("a field named twice was accepted: {refused:?}"));
        let spans = repeated
            .labels
            .iter()
            .map(|label| {
                (
                    label.primary,
                    source[label.span.start as usize..label.span.end as usize].to_owned(),
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(
            spans,
            [(true, "dots".to_owned()), (false, "dots".to_owned())],
            "a duplicate field must point at the repeat and name the first: {repeated:?}"
        );
        assert_ne!(
            repeated.labels.first().map(|label| label.span.start),
            repeated.labels.get(1).map(|label| label.span.start),
            "both labels landed on the same mention: {repeated:?}"
        );
    }

    /// Naming a field the declaration does not have is refused at that name,
    /// and the diagnostic names the declaration.
    ///
    /// Including the case where the name is a real field — of some *other*
    /// declaration. The update is nominal: the subject's type fixes the field
    /// set, so a name borrowed from another record is exactly as unknown as an
    /// invented one, and saying which declaration was being updated is what
    /// tells the author that.
    #[test]
    fn an_unknown_field_is_refused_at_its_name_and_names_the_declaration() {
        for field in ["missing", "elsewhere"] {
            let source = format!(
                "piece \"law\" {{ data Pending {{\n    Pending(dots: Nat)\n}}\n \
                 data Other {{\n    Other(elsewhere: Nat)\n}}\n \
                 let start = Pending(1); \
                 let moved = start with {{ {field} = 3 }}; }}"
            );
            let refused = refusals_from_declared_data(&source);
            let unknown = refused
                .iter()
                .find(|diagnostic| diagnostic.message.contains(field))
                .unwrap_or_else(|| panic!("`{field}` was accepted as a field: {refused:?}"));
            let at = unknown
                .labels
                .iter()
                .find(|label| label.primary)
                .expect("an unknown name points somewhere");
            assert_eq!(
                &source[at.span.start as usize..at.span.end as usize],
                field,
                "the diagnostic did not point at the field name: {unknown:?}"
            );
            assert!(
                unknown.message.contains("Pending"),
                "the diagnostic must name the declaration being updated: {unknown:?}"
            );
        }
    }

    /// Only a single-constructor nominal record can be updated.
    ///
    /// A `Nat` has no fields; a sum has fields only once the case is known, and
    /// knowing the case is what `match` is for. An update that skipped it would
    /// have to fail where the value turned out to be the other constructor, and
    /// this language has no run-time failure to fail with.
    #[test]
    fn updating_something_that_is_not_a_single_constructor_record_is_refused() {
        for source in [
            "piece \"law\" { let held = 3; let moved = held with { dots = 1 }; }",
            "piece \"law\" { data Held {\n    One(dots: Nat),\n    Two(dots: Nat),\n}\n \
             let start = One(1); let moved = start with { dots = 2 }; }",
        ] {
            let refused = refusals_from_declared_data(source);
            assert!(
                !refused.is_empty(),
                "updating a value with no single constructor was accepted: {source}"
            );
        }
    }

    /// A question and the `Result` match it elaborates to are the same
    /// program — same answers, and the same number of charged nodes.
    ///
    /// This is the whole claim prompt 127dcfad makes. `?` adds no core term,
    /// no typing rule, no reduction, no measure case, and no cost-table entry,
    /// which is only true if the elaborated form is indistinguishable from the
    /// match an author would have written. The subject here builds an
    /// eight-member list, so a second evaluation of it would be several nodes
    /// wide: equal charge against a match that visibly evaluates its scrutinee
    /// once is what says the question does too. Two questions are chained
    /// because they nest rather than sit side by side, and a chain that cost
    /// one frame more than its nesting would say the elaboration had invented
    /// something.
    #[test]
    fn a_question_is_the_result_match_it_elaborates_to() {
        const MADE: &str = "fn made(said: Text) -> Result<List<Nat>, Text> { \
             if text_equal(said, \"none\") { Err(\"nothing made\") } else { Ok([1, 2, 3, 4, 5, 6, 7, 8]) } } \
             fn kept(held: List<Nat>) -> Result<Nat, Text> { Ok(4) } ";
        const ASKED: &str = "fn used(said: Text) -> Result<Nat, Text> { Ok(nat_add(kept(made(said)?)?, 1)?) } \
             let good: Nat = match used(\"some\") { Ok(v) -> v, Err(why) -> 99 }; \
             let bad: Text = match used(\"none\") { Ok(v) -> \"no\", Err(why) -> why };";
        const BY_HAND: &str = "fn used(said: Text) -> Result<Nat, Text> { \
             match made(said) { \
             Ok(one) -> match kept(one) { \
             Ok(two) -> match nat_add(two, 1) { Ok(three) -> Ok(three), Err(why) -> Err(why) }, \
             Err(why) -> Err(why), \
             }, \
             Err(why) -> Err(why), \
             } } \
             let good: Nat = match used(\"some\") { Ok(v) -> v, Err(why) -> 99 }; \
             let bad: Text = match used(\"none\") { Ok(v) -> \"no\", Err(why) -> why };";

        let keyed = |source: &str| {
            values(&format!("piece \"law\" {{ {MADE}{source} }}"))
                .unwrap_or_else(|| panic!("well-typed source was rejected: {source}"))
                .iter()
                .filter(|(name, _)| name.as_str() == "good" || name.as_str() == "bad")
                .map(|(name, value)| (name.clone(), literal_key(value)))
                .collect::<Vec<_>>()
        };
        let asked = keyed(ASKED);
        assert_eq!(
            asked,
            [
                ("good".to_owned(), "nat:5".to_owned()),
                ("bad".to_owned(), "text:\"nothing made\"".to_owned()),
            ]
        );
        assert_eq!(asked, keyed(BY_HAND), "the question answered differently");
        assert_eq!(
            charged_nodes(&format!("{MADE}{ASKED}")),
            charged_nodes(&format!("{MADE}{BY_HAND}")),
            "the question cost more than the match it is, so its subject is not evaluated once"
        );
    }

    /// A failure comes out exactly as it went in, payload and all.
    ///
    /// The error here is the pair an adapter refuses with — where it happened
    /// and what to say — because prompt 127dcb makes the node part of the
    /// answer. Rebuilding the `Err` is what the elaboration does, so this is
    /// the law that says rebuilding it changes nothing about it.
    #[test]
    fn a_question_carries_the_failure_it_was_given_unchanged() {
        const SOURCE: &str = "fn refusing(said: Text) -> Result<Nat, (Nat, Text)> { Err((7, said)) } \
             fn carrying(said: Text) -> Result<Nat, (Nat, Text)> { Ok(refusing(said)?) } \
             let where_from: Nat = match carrying(\"say so\") { \
             Ok(v) -> 0, \
             Err(refusal) -> match refusal { (node, told) -> node }, \
             }; \
             let told: Text = match carrying(\"say so\") { \
             Ok(v) -> \"\", \
             Err(refusal) -> match refusal { (node, told) -> told }, \
             };";
        let bound = values(&format!("piece \"law\" {{ {SOURCE} }}")).expect("well-typed source");
        assert_eq!(
            [
                bound.get("where_from").map(literal_key),
                bound.get("told").map(literal_key)
            ],
            [Some("nat:7".to_owned()), Some("text:\"say so\"".to_owned())],
            "the propagated failure is not the one that was given"
        );
    }

    /// A function that writes no result type still infers through a `?`, named
    /// or anonymous.
    ///
    /// This is the part of prompt 127dcfad most easily got wrong. `?`
    /// constrains the enclosing answer to a `Result` failing the same way, and
    /// a constraint is discharged by unification like every other one — so
    /// requiring the annotation would have made this the one construct that
    /// demands one, and made "principal rank-1 inference" false as a
    /// language-wide sentence.
    #[test]
    fn an_unannotated_function_infers_through_a_question() {
        const SOURCE: &str = "fn made(said: Text) -> Result<Nat, Text> { \
             if text_equal(said, \"none\") { Err(\"nothing made\") } else { Ok(4) } } \
             fn named(said) { Ok(nat_add(made(said)?, 1)?) } \
             let by_name: Nat = match named(\"some\") { Ok(v) -> v, Err(why) -> 99 }; \
             let anonymous = fn (said: Text) { Ok(nat_add(made(said)?, 2)?) }; \
             let by_lambda: Nat = match anonymous(\"some\") { Ok(v) -> v, Err(why) -> 99 }; \
             let anonymous_bad: Text = match anonymous(\"none\") { Ok(v) -> \"no\", Err(why) -> why };";
        let bound = values(&format!("piece \"law\" {{ {SOURCE} }}")).expect("well-typed source");
        assert_eq!(
            [
                bound.get("by_name").map(literal_key),
                bound.get("by_lambda").map(literal_key),
                bound.get("anonymous_bad").map(literal_key),
            ],
            [
                Some("nat:5".to_owned()),
                Some("nat:6".to_owned()),
                Some("text:\"nothing made\"".to_owned()),
            ]
        );
    }

    /// Chained questions run left to right and the first failure is the one
    /// reported.
    ///
    /// The elaboration nests them in written order, which is what decides this
    /// — so a program with two things wrong with it complains about the
    /// leftmost, the way a reader reads.
    #[test]
    fn chained_questions_stop_at_the_first_failure() {
        const SOURCE: &str = "fn named(said: Text) -> Result<Nat, Text> { \
             if text_equal(said, \"fine\") { Ok(1) } else { Err(said) } } \
             fn both(first: Text, second: Text) -> Result<Nat, Text> { \
             Ok(nat_add(named(first)?, named(second)?)?) } \
             let leftmost: Text = match both(\"one bad\", \"two bad\") { Ok(v) -> \"no\", Err(why) -> why }; \
             let rightmost: Text = match both(\"fine\", \"two bad\") { Ok(v) -> \"no\", Err(why) -> why }; \
             let neither: Nat = match both(\"fine\", \"fine\") { Ok(v) -> v, Err(why) -> 99 };";
        let bound = values(&format!("piece \"law\" {{ {SOURCE} }}")).expect("well-typed source");
        assert_eq!(
            [
                bound.get("leftmost").map(literal_key),
                bound.get("rightmost").map(literal_key),
                bound.get("neither").map(literal_key),
            ],
            [
                Some("text:\"one bad\"".to_owned()),
                Some("text:\"two bad\"".to_owned()),
                Some("nat:2".to_owned()),
            ],
            "a chain must stop at the first failure, in written order"
        );
    }

    /// A `?` is refused where the answer around it cannot fail, and the
    /// refusal lands on the question rather than on a match nobody wrote.
    ///
    /// Three ways to have nowhere to carry a failure to, and each says which
    /// one it is: an answer that is no `Result`, an answer that fails another
    /// way, and a branch whose value is not the function's answer at all. The
    /// last is the one this elaboration cannot do without a `return`, and
    /// refusing it is why there is no `return`.
    #[test]
    fn a_question_with_nowhere_to_go_is_refused_at_its_own_span() {
        const MADE: &str = "fn made(said: Text) -> Result<Nat, Text> { Ok(4) } \
             fn kept(held: Nat) -> Result<Nat, Text> { Ok(held) } \
             fn maybe(said: Text) -> Option<Nat> { Some(4) } ";
        for (body, expected, at) in [
            (
                "fn plain(said: Text) -> Nat { made(said)? }",
                "this answers `Nat`, so a failure has no way out of it",
                "made(said)?",
            ),
            (
                "fn other(said: Text) -> Result<Nat, Nat> { Ok(made(said)?) }",
                "a failure of `Result<Nat, Text>` cannot leave an answer of `Result<Nat, Nat>`",
                "made(said)?",
            ),
            (
                "fn buried(said: Text) -> Result<Nat, Text> { \
                 kept(match text_equal(said, \"x\") { true -> made(said)?, false -> 1 }) }",
                "`?` has nowhere to carry a failure to",
                "made(said)?",
            ),
            (
                "fn absent(said: Text) -> Result<Nat, Text> { Ok(maybe(said)?) }",
                "`?` asks a `Result`, and this is `Option<Nat>`",
                "maybe(said)",
            ),
        ] {
            let source = format!("piece \"law\" {{ {MADE}{body} }}");
            let refused = refusals(&source);
            let named = refused
                .iter()
                .find(|diagnostic| diagnostic.message == expected)
                .unwrap_or_else(|| panic!("expected `{expected}`, saw: {refused:?}"));
            let span = named
                .labels
                .iter()
                .find(|label| label.primary)
                .expect("a refusal points somewhere");
            assert_eq!(
                source[span.span.start as usize..span.span.end as usize].trim(),
                at,
                "the refusal did not point at the question: {named:?}"
            );
        }
    }

    /// Why a piece that declares its own `data` types was refused.
    ///
    /// [`refusals`] reads an empty world, which cannot resolve a declared
    /// constructor, so a piece with a `data` declaration needs the world
    /// [`charged_nodes`] builds or every name in it is unknown for the wrong
    /// reason.
    fn refusals_from_declared_data(source: &str) -> Vec<Diagnostic> {
        let parsed = musa_language::parse(source);
        let Some(piece) = musa_language::ast::PieceDecl::from_root(&parsed.syntax()) else {
            return Vec::new();
        };
        let mut resolver = Resolver::new();
        let world = World::read(&mut resolver, &[piece.syntax().clone()]);
        check_and_evaluate(
            &mut resolver,
            declarations(piece.syntax(), None).into_iter(),
            Some(piece.syntax()),
            UnknownRootMusic::Reject,
            &Modules::default(),
            &world,
            Reading::Source,
        );
        resolver.diagnostics
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
            &World::default(),
            Reading::Source,
        );
        resolver.diagnostics
    }

    #[test]
    fn every_first_order_builtin_is_total_on_its_declared_domain() {
        let observed = sampled_applications();
        assert!(
            observed.len() > 1_000,
            "the sample must actually exercise the domains, saw {} applications",
            observed.len()
        );
        let mut exercised: Vec<Builtin> = Vec::new();
        for (builtin, result, answer) in observed {
            if !exercised.contains(&builtin) {
                exercised.push(builtin);
            }
            let Some(value) = answer else {
                panic!(
                    "`{}` returned no value on a well-typed argument tuple; D2 requires partiality \
                     to be declared in the result type, not reported by the evaluator",
                    builtin.name()
                );
            };
            let actual = value_type(&value)
                .unwrap_or_else(|| panic!("`{}` returned a value with no first-order type", builtin.name()));
            assert_eq!(
                actual,
                result.ty(),
                "`{}` returned a `{actual}` where its signature declares `{}`",
                builtin.name(),
                result.ty()
            );
            if !result.admits_absence() {
                assert!(
                    !matches!(value, Value::Option { value: None, .. }),
                    "`{}` reported absence at a result type that cannot express it",
                    builtin.name()
                );
            }
        }
        let declared = BUILTIN_OWNERSHIP
            .iter()
            .filter(|entry| matches!(entry.family, Family::Delta { .. }))
            .count();
        assert_eq!(
            exercised.len(),
            declared,
            "every δ-builtin must be reached by the sample; unreached: {:?}",
            BUILTIN_OWNERSHIP
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
    fn finite_builtins_agree_with_small_reference_folds() {
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
                 let by_list: Nat = list_fold_from_start(0, item, mapped); \
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
