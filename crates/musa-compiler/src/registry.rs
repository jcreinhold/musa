//! The compiler's own operations, said in `musa-core`'s terms.
//!
//! `02-core-calculus.md` §5.8 splits every compiler-owned operation into four
//! families and gives each its own admissibility argument. Prompts 141b, 141c,
//! and 141d built the mechanism for them; this module fills its δ half. What was
//! a [`crate::core::Type`] and a [`crate::core::Shape`] becomes a
//! [`musa_core::Term`], and what was an arm of the old evaluator becomes a rule
//! over [`Datum`].
//!
//! # Nothing here decides anything new
//!
//! Every source signature is read off [`crate::core::BUILTIN_OWNERSHIP`] rather
//! than retyped, and every phase signature off [`crate::core::SyntaxOp`]'s own
//! scheme, because 106 signatures written a second time by hand is 106 chances
//! to write a different type from the one the old checker enforced. The rules are
//! the same arithmetic on the same domain modules; only the unwrapping of an
//! argument and the wrapping of an answer changed.
//!
//! # Why a base type and not a family
//!
//! A base type is *inert*: it contributes no ι-rule, so two closed values of it
//! are convertible exactly when the host says the payloads agree. That is the
//! right shape for a domain whose representation the compiler owns and no source
//! program takes apart — a `Pitch`, a `Scale`, a `Row12`. It is the wrong shape
//! for anything source pattern-matches on, which is why `Bool`, `Nat`, `Option`,
//! `List`, and `Result` are declared in [`crate::prelude`] instead.
//!
//! `Coordinate` and `Cat` are base types too, and that is D3 deciding rather than
//! taste. A δ-rule is a `fn` pointer, so the only type it can *write* is one it
//! can build with no context — a [`Base`] and a [`Literal`], both of which are
//! plain constructor calls, and never a family's constructor, which exists only
//! inside a [`Cx`]. `duration_of` answers a `Duration ⟨written⟩` and takes only a
//! `Ratio`, so if the index were a constructor it could not write down the type
//! of its own answer.
//!
//! # The order the context is built in
//!
//! Four stages, each needing the one before it, and written out because the
//! dependency is real rather than stylistic:
//!
//! 1. the structural families, which name no base type;
//! 2. a registry holding every base type and no builtin, so that a base type is
//!    nameable;
//! 3. the musical families, whose fields may name a base type — `RowFault` holds
//!    a `List Pc12`;
//! 4. the builtins, whose signatures may name anything above, and the full
//!    registry.
//!
//! # What is registered, and what is named and left
//!
//! `BUILTIN_OWNERSHIP` has 117 rows and 92 of them are δ. The other 25 are
//! §5.8's three remaining families and none is registrable here: the eight
//! collection eliminators are traversals over `Nat`, `List`, and `Option`, which
//! are *declared*, so [`musa_core::declare`] already generated their recursors
//! and [`Registry::new`] refuses a structural target that is not a base type; the
//! eight track and nine machine builtins need `EventTrack` and `Machine`, which
//! prompt 142 reshapes when it deletes contextual `Music`.
//!
//! `SYNTAX_OWNERSHIP` has 17 rows and sixteen are registered: fourteen δ builders
//! here, and the two traversals in [`traversal`], which are §5.8's *second*
//! family — a structural eliminator over `Syntax`, which is a base type. The
//! seventeenth, `run_syntax_step`, is [`run_syntax_step`]: a projection, not a
//! compiler-owned operation, and its target is a declared family that
//! [`Registry::new`] would refuse. [`crate::registry::rules::UNREGISTERED`]
//! counts what is left, and the suite counts it again off the tables themselves.

#[cfg(test)]
mod laws;
mod rules;
mod traversal;

use std::any::Any;
use std::fmt;
use std::sync::Arc;

use musa_core::{
    Base, Builtin, Cx, Datum, ElabError, Index, Level, Literal, Origin, Payload, Raw, RawArm, RawPattern, Refusal,
    Registry, Term,
};

/// The old table's name for one inert domain, renamed on the way in.
///
/// `Base` means two things in this module — the registration `musa-core` holds,
/// and the leaf the signature table names — and this is the one that is going
/// away when prompt 142 deletes the old checker. Naming it for what it is keeps
/// the translation's two sides legible in a single line.
use crate::core::Base as Leaf;
use crate::core::{BUILTIN_OWNERSHIP, Coordinate, Family, SYNTAX_OWNERSHIP, Shape, Type};

/// Where a term this module builds comes from.
///
/// Compiler-owned, like [`crate::prelude`]'s: no source file wrote `pitch_of`'s
/// signature, so there is no line for a diagnostic to point at.
const HERE: Origin = Origin::UNKNOWN;

/// A musical value as a core literal's payload.
///
/// One implementation rather than one per domain. [`Payload`]'s three methods
/// each need one thing of the value it wraps — `PartialEq` for [`Payload::same`],
/// [`fmt::Display`] for [`Payload::shown`], and `'static` for
/// [`Payload::as_any`] — and every inert domain in this compiler already has all
/// three. Adding a domain is therefore adding a row to [`bases`] and nothing
/// else.
///
/// `Send + Sync` is inherited rather than chosen: a literal rides inside a
/// [`Term`], and terms cross threads wherever more than one document compiles at
/// once.
#[derive(Debug, PartialEq, Eq)]
struct Domain<T>(T);

impl<T> Payload for Domain<T>
where
    T: PartialEq + fmt::Debug + fmt::Display + Send + Sync + 'static,
{
    /// Two payloads agree when they are the same domain and the same value.
    ///
    /// The downcast can fail even though [`Literal`] compares types first,
    /// because two domains could in principle share a base type's name.
    /// Answering `false` there is the behaviour [`Payload::same`] documents and
    /// is an equivalence relation either way.
    fn same(&self, other: &dyn Payload) -> bool {
        other.as_any().downcast_ref::<Self>().is_some_and(|other| self == other)
    }

    fn shown(&self) -> String {
        self.0.to_string()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// The literal holding `value` at base type `ty`.
pub(crate) fn literal<T>(ty: Term, value: T) -> Literal
where
    T: PartialEq + fmt::Debug + fmt::Display + Send + Sync + 'static,
{
    Literal::new(ty, Arc::new(Domain(value)))
}

/// The value inside a literal `Datum`, if it is of domain `T`.
///
/// The inverse of [`literal`], and the only way a rule reads its argument. It
/// answers `None` rather than panicking for the reason a rule answers `None` at
/// all: a builtin applied to something its signature does not admit is a caller
/// defect that the core reports, not a crash.
fn domain<T>(datum: &Datum) -> Option<&T>
where
    T: PartialEq + fmt::Debug + fmt::Display + Send + Sync + 'static,
{
    let Datum::Lit(ref value) = *datum else {
        return None;
    };
    held(value)
}

/// The value inside a literal, if it is of domain `T`.
///
/// The same reading as [`domain`], one layer in. A δ-rule is handed a [`Datum`]
/// and a structural rewrite is handed the [`Literal`] it fired on, so both
/// spellings of "the target" reach the same downcast rather than two.
fn held<T>(value: &Literal) -> Option<&T>
where
    T: PartialEq + fmt::Debug + fmt::Display + Send + Sync + 'static,
{
    value.payload().as_any().downcast_ref::<Domain<T>>().map(|held| &held.0)
}

/// The context every Musa program is elaborated in.
///
/// Answers the context and nothing else, for [`crate::prelude::constant`]'s
/// reason: a caller that wants `Option` or `Pitch` asks for it by name, the same
/// way source code does, rather than being handed a struct of resolved terms it
/// would have to keep in step.
///
/// # Errors
///
/// [`ElabError`] when a declaration or a signature in this compiler is wrong,
/// which is a compiler defect rather than a program's, and is returned rather
/// than panicked on for the reason `musa-core` returns
/// [`musa_core::Malformed`].
pub(crate) fn owned() -> Result<Cx, ElabError> {
    let mut cx = Cx::new();
    for declaration in crate::prelude::structural() {
        let group = musa_core::declare(&cx, &declaration)?;
        cx = cx.declaring(&group);
    }
    // Declared *by* a context standing in the phase module and added to one that
    // is not, which is what makes `SyntaxStep.Step` private: the group carries
    // the module it was written in, and the compiler's own context — standing
    // nowhere — is inside every module and may still mint a step.
    for declaration in crate::prelude::phase() {
        let group = musa_core::declare(&cx.in_module(crate::prelude::PHASE), &declaration)?;
        cx = cx.declaring(&group);
    }
    let bases = bases();
    cx = cx.with_externs(Arc::new(Registry::new(bases.clone(), Vec::new())?));
    for declaration in crate::prelude::musical() {
        let group = musa_core::declare(&cx, &declaration)?;
        cx = cx.declaring(&group);
    }
    let builtins = builtins(&cx)?;
    Ok(cx.with_externs(Arc::new(Registry::new(bases, builtins)?)))
}

/// A base type at `Type 0`.
///
/// The one place a plain base type's kind is written, so that the registration
/// and the type a rule builds for its own answer are the same expression rather
/// than two that agree today.
fn plain(name: &'static str) -> Base {
    Base::new(name, type0())
}

/// A base type indexed by the values of another base type.
fn indexed(name: &'static str, index: &'static str) -> Base {
    Base::new(name, Term::pi(HERE, "index", plain(index).term(HERE), type0()))
}

/// Every inert domain, with the kind it is registered at.
///
/// Needs no context, which is the point: a rule reaches the same three helpers
/// to write the type of its own answer, so a registered base type and a
/// rule-built one cannot be two different terms.
fn bases() -> Vec<Base> {
    vec![
        // The two numeric leaves §1 keeps inert. `Ratio` is signed and has no
        // least element to descend to, so it has no recursor and its arithmetic
        // is δ; `Text` is opaque printable text, which is what lets it be the
        // error half of a `Result` without giving a builtin a second way to say
        // what went wrong.
        plain("Ratio"),
        plain("Text"),
        // The index on the two tagged rationals: an enumeration in everything
        // but its representation, inert because nothing matches on it and a
        // literal because a `fn` rule can build one.
        plain("Coordinate"),
        // The two tagged rationals, indexed by their coordinate so that a
        // written beat and a number of seconds do not add.
        indexed("Duration", "Coordinate"),
        indexed("Position", "Coordinate"),
        // The written domains.
        plain("Pitch"),
        plain("PitchClass"),
        plain("Interval"),
        plain("Key"),
        plain("Scale"),
        plain("Degree"),
        plain("Frame"),
        plain("ChordClass"),
        plain("Triad"),
        plain("Roman"),
        plain("Voicing"),
        // The twelve-tone domains.
        plain("Pc12"),
        plain("PcSet12"),
        plain("Row12"),
        // The phase-local domains (§5.9). Registered beside the musical ones
        // rather than in a second registry, because a base type is a base type;
        // what §5.9 keeps separate is the *operations*, and those are two tables
        // in [`builtins`].
        plain("Cat"),
        indexed("Syntax", "Cat"),
        plain("TokenKind"),
        plain("Delimiter"),
        plain("NodePath"),
        plain("BindingPath"),
    ]
}

/// `Type 0`, which every base type but the two indexed ones is registered at.
fn type0() -> Term {
    Term::universe(HERE, Level::ZERO)
}

/// The term naming a plain base type.
pub(crate) fn plain_type(name: &'static str) -> Term {
    plain(name).term(HERE)
}

/// The term naming `Duration` or `Position` at one coordinate.
pub(crate) fn tagged_type(name: &'static str, which: Coordinate) -> Term {
    Term::app(
        HERE,
        indexed(name, "Coordinate").term(HERE),
        literal(plain_type("Coordinate"), which).term(HERE),
    )
}

/// The term naming `Syntax` at one category.
pub(crate) fn syntax_type(cat: crate::syntax::Cat) -> Term {
    Term::app(
        HERE,
        indexed("Syntax", "Cat").term(HERE),
        literal(plain_type("Cat"), cat).term(HERE),
    )
}

/// The core type a signature shape denotes.
///
/// This is the whole of "the signature table is reused rather than retyped".
/// [`Shape`] already says what each of the 92 δ entries takes and answers, with
/// no arrow and with storability checked where the table is written; what it
/// lacks is a spelling in the core's terms, and that is one recursion rather
/// than 92 declarations.
///
/// A base type applied to an index is an application, and nothing about it is
/// special-cased: `Duration` is registered at `Coordinate → Type 0`, so
/// `DURATION` denotes `Duration ⟨written⟩` by the same rule that makes
/// `Option Ratio` an application of a declared family.
fn shape_type(cx: &Cx, shape: Shape) -> Result<Term, ElabError> {
    match shape {
        Shape::Base(leaf) => base_type(cx, leaf),
        Shape::Option(member) => applied(cx, "Option", [shape_type(cx, *member)?]),
        Shape::List(member) => applied(cx, "List", [shape_type(cx, *member)?]),
        Shape::Result(value, error) => applied(cx, "Result", [shape_type(cx, *value)?, shape_type(cx, *error)?]),
        Shape::Fault => crate::prelude::constant(cx, "RowFault"),
    }
}

/// The core type one inert domain denotes.
///
/// The two tagged rationals are applications and every other leaf is a constant,
/// which is [`bases`]'s kinds read back the other way round.
fn base_type(cx: &Cx, leaf: Leaf) -> Result<Term, ElabError> {
    Ok(match leaf {
        Leaf::Ratio => plain_type("Ratio"),
        Leaf::Text => plain_type("Text"),
        Leaf::Duration(which) => tagged_type("Duration", which),
        Leaf::Position(which) => tagged_type("Position", which),
        Leaf::Pitch => plain_type("Pitch"),
        Leaf::PitchClass => plain_type("PitchClass"),
        Leaf::Interval => plain_type("Interval"),
        Leaf::Key => plain_type("Key"),
        Leaf::Scale => plain_type("Scale"),
        Leaf::Degree => plain_type("Degree"),
        Leaf::Frame => plain_type("Frame"),
        Leaf::ChordClass => plain_type("ChordClass"),
        Leaf::Triad => plain_type("Triad"),
        Leaf::Roman => plain_type("Roman"),
        Leaf::Voicing => plain_type("Voicing"),
        Leaf::Pc12 => plain_type("Pc12"),
        Leaf::PcSet12 => plain_type("PcSet12"),
        Leaf::Row12 => plain_type("Row12"),
        // Declared rather than registered, so they are read out of the prelude's
        // context and not out of this registry. §1 decides both by name: a `Nat`
        // with no recursor would leave `nat_fold` with nothing to fold, and
        // `Bool` is the same argument one constructor shorter.
        Leaf::Bool => crate::prelude::constant(cx, "Bool")?,
        Leaf::Nat => crate::prelude::constant(cx, "Nat")?,
    })
}

/// The core type one phase-operation type denotes.
///
/// The phase table has no [`Shape`], because its three traversals take function
/// arguments and `Shape` has no arrow; its signatures are written in the old
/// checker's [`Type`] instead. This is the same reuse by a different door — the
/// 14 builders' types are read off `SyntaxOp`'s own scheme rather than declared a
/// second time — and it covers exactly the first-order fragment those 14 use.
///
/// # Errors
///
/// [`unnameable`] for a type outside that fragment, which is how a traversal
/// reaches here by mistake: it is a defect in the caller's filter rather than in
/// the signature.
fn phase_type(cx: &Cx, ty: &Type) -> Result<Term, ElabError> {
    Ok(match *ty {
        Type::Bool => crate::prelude::constant(cx, "Bool")?,
        Type::Nat => crate::prelude::constant(cx, "Nat")?,
        Type::Ratio => plain_type("Ratio"),
        Type::Text => plain_type("Text"),
        Type::Syntax(cat) => syntax_type(cat),
        Type::TokenKind => plain_type("TokenKind"),
        Type::Delimiter => plain_type("Delimiter"),
        Type::NodePath => plain_type("NodePath"),
        Type::BindingPath => plain_type("BindingPath"),
        Type::Option(ref member) => applied(cx, "Option", [phase_type(cx, member)?])?,
        Type::List(ref member) => applied(cx, "List", [phase_type(cx, member)?])?,
        Type::Sum(ref value, ref error) => applied(cx, "Result", [phase_type(cx, value)?, phase_type(cx, error)?])?,
        Type::Function(ref arguments, ref result) => {
            let mut built = phase_type(cx, result)?;
            for argument in arguments.iter().rev() {
                built = Term::pi(HERE, "argument", phase_type(cx, argument)?, built);
            }
            built
        }
        // Spelled out rather than wildcarded, because the list is the claim.
        // Every one of these is a type the *old* checker has and the core is
        // deliberately not being given: a type variable and a unit belong to the
        // checker prompt 142 deletes, the musical domains are registered from
        // the `Shape` table rather than from this one, and `Music`, `Step`,
        // `Primitive`, `Machine`, and `SyntaxStep` are the shapes prompt 142
        // reshapes. A variant added to `Type` should stop here and be decided,
        // not fall through a `_`.
        Type::Var(_)
        | Type::Unit
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
        | Type::Product(_)
        | Type::Nominal(..)
        | Type::Music
        | Type::Step(_)
        | Type::Primitive { .. }
        | Type::Machine { .. }
        | Type::SyntaxStep { .. } => return Err(unnameable("a phase type with no core spelling")),
    })
}

/// `head` applied to `arguments`.
fn applied(cx: &Cx, head: &str, arguments: impl IntoIterator<Item = Term>) -> Result<Term, ElabError> {
    let head = crate::prelude::constant(cx, head)?;
    Ok(arguments
        .into_iter()
        .fold(head, |function, argument| Term::app(HERE, function, argument)))
}

/// The signature of a δ entry: its arguments, then its result, as one Π type.
fn delta_type(cx: &Cx, arguments: &[Shape], result: Shape) -> Result<Term, ElabError> {
    let mut ty = shape_type(cx, result)?;
    for argument in arguments.iter().rev() {
        ty = Term::pi(HERE, "argument", shape_type(cx, *argument)?, ty);
    }
    Ok(ty)
}

/// Every compiler-owned δ operation, as a core builtin.
///
/// Two tables, kept two. `BUILTIN_OWNERSHIP` is the source language's and
/// `SYNTAX_OWNERSHIP` is the expansion phase's, and §5.9 makes that separation
/// load-bearing: nothing in the phase registry is looked up when ordinary source
/// reads a name. What they share is this one loop, because a δ-builtin is a
/// δ-builtin whichever table names it.
///
/// A row whose rule is missing is a registration defect and refuses the whole
/// registry, rather than being skipped: a table with a hole in it would type-check
/// a program the evaluator then got stuck on.
fn builtins(cx: &Cx) -> Result<Vec<Builtin>, ElabError> {
    let mut built = Vec::with_capacity(rules::REGISTERED);
    for entry in &BUILTIN_OWNERSHIP {
        let Family::Delta { arguments, result } = entry.family else {
            continue;
        };
        let rule = rules::source(entry.operation).ok_or_else(|| missing(entry.spelling))?;
        built.push(Builtin::new(
            entry.spelling,
            delta_type(cx, arguments, result)?,
            musa_core::Family::Delta,
            rule,
        ));
    }
    let mut unifier = crate::infer::Unifier::default();
    for entry in &SYNTAX_OWNERSHIP {
        let Some(rule) = rules::phase(entry.operation) else {
            continue;
        };
        built.push(Builtin::new(
            entry.spelling,
            phase_type(cx, &entry.operation.instantiate(&mut unifier))?,
            musa_core::Family::Delta,
            rule,
        ));
    }
    built.extend(traversal::eliminators(cx)?);
    Ok(built)
}

/// `run_syntax_step`, as a definition rather than a registration.
///
/// ```text
/// run_syntax_step
///   : (Context : Type 0) → (Answer : Type 0)
///   → Context → SyntaxStep Context Answer → Answer
/// run_syntax_step = fn (C, A, context, step) {
///     match step { SyntaxStep.Step(run) => run(context) }
/// }
/// ```
///
/// Seventeen rows in, sixteen registrations and this out. `run_syntax_step`
/// leaves the phase registry because a projection is not a compiler-owned
/// operation: it hides nothing, which is the test every `SYNTAX_OWNERSHIP` row
/// already states for itself. [`Registry::new`] would have refused it in any
/// case — its target is a *declared* family, and a rewrite over a declared family
/// is the second ι-rule that check exists to catch. That the ownership test and
/// the core's own check agree is evidence the design is right rather than a
/// coincidence to route around.
///
/// The step's `run` field is private to the phase module, so the `match` here
/// resolves only because this context stands in no module and is therefore inside
/// every one. A transformer's own module is not, which is the seal.
///
/// # Errors
///
/// [`ElabError`] when `SyntaxStep` is not declared in `cx`, or when the
/// definition does not check at its own type — a compiler defect either way.
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "prompt 142 is where elaboration binds the phase's spellings; until then the laws are the caller"
    )
)]
pub(crate) fn run_syntax_step(cx: &Cx) -> Result<Definition, ElabError> {
    let sealed = crate::prelude::constant(cx, "SyntaxStep")?;
    let step = Term::app(
        HERE,
        Term::app(HERE, sealed, Term::var(HERE, Index(2))),
        Term::var(HERE, Index(1)),
    );
    let ty = Term::pi(
        HERE,
        "Context",
        type0(),
        Term::pi(
            HERE,
            "Answer",
            type0(),
            Term::pi(
                HERE,
                "context",
                Term::var(HERE, Index(1)),
                Term::pi(HERE, "step", step, Term::var(HERE, Index(2))),
            ),
        ),
    );
    let run = RawArm {
        patterns: vec![RawPattern::constructor(
            HERE,
            "SyntaxStep.Step",
            [RawPattern::bind(HERE, "run")],
        )],
        body: Raw::app(HERE, Raw::var(HERE, "run"), Raw::var(HERE, "context")),
    };
    let body = Raw::match_on(HERE, [Raw::var(HERE, "step")], vec![run]);
    let written = ["Context", "Answer", "context", "step"]
        .into_iter()
        .rev()
        .fold(body, |built, name| Raw::lam(HERE, name, built));
    let value = musa_core::check(cx, &ty, &written)?;
    Ok(Definition { ty, value })
}

/// A compiler-owned definition: what it is, and what it means.
///
/// Both halves, because a definition is both — [`Cx::define`] takes a type and a
/// value, and a λ has no inferable type, so handing back the value alone would
/// leave every caller to reconstruct the type this function already wrote.
pub(crate) struct Definition {
    /// Its type.
    pub(crate) ty: Term,
    /// Its meaning, checked at that type.
    pub(crate) value: Term,
}

/// A row of a compiler-owned table with no rule behind it.
fn missing(spelling: &str) -> ElabError {
    unnameable(spelling)
}

/// Something this module was asked for and has no core spelling of.
///
/// A compiler defect either way, and [`Refusal::UnknownName`] rather than a
/// [`musa_core::Malformed`] because what went wrong is a *name* with nothing
/// behind it — the same sentence a program earns for a name that is not in
/// scope, which is exactly the shape of this failure one level up.
fn unnameable(what: &str) -> ElabError {
    Refusal::UnknownName {
        name: Arc::from(what),
        at: HERE,
    }
    .into()
}
