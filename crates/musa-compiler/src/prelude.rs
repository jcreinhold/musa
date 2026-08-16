//! The declarations every Musa program is read against, and the context that
//! holds them.
//!
//! `musa-core` is a leaf that knows nothing about pitch, time, or notation
//! (roadmap §15.12), so *something* has to say that `Pitch` is an inert base
//! type and that `Option` is a family with two constructors. This module is that
//! something. It is the host half of `02-core-calculus.md` §5.8: the core owns
//! the mechanism, and the table of what is registered lives here, where the
//! musical domains already do.
//!
//! # Why the families are declared before the registry is built
//!
//! §5.8's D1 admits an argument or result type that is "a base type **or a
//! finite constructor over base types**", and 41% of the δ-builtins in
//! [`crate::core`]'s table answer an `Option`, a `Result`, or a `List`. A
//! signature that says so has to *name* those families, and a family constant
//! exists only after [`musa_core::declare`] has run. So the order is fixed and
//! not a preference: declare the families in a bare context, read their
//! constants back out of it, build the registry against them, and only then hand
//! the whole thing to the rest of the compiler.
//!
//! # Which types are families and which are base types
//!
//! A base type is *inert*: it contributes no ι-rule, so two closed values of it
//! are convertible exactly when the host says the payloads agree. That is the
//! right shape for a domain whose representation the compiler owns and no source
//! program takes apart — a `Pitch`, a `Scale`, a `Row12`.
//!
//! It is the wrong shape for anything source code pattern-matches on, and
//! `02-core-calculus.md` decides two of those by name. `Nat` "is in the language
//! to be the *inductive* numeric type: `zero | succ` is well founded, so its
//! recursor terminates by construction" — a `Nat` with no recursor would leave
//! `nat_fold` with nothing to fold. `Bool` is the same argument one constructor
//! shorter, and `Option`, `List`, and `Result` are what §1's `τ + τ` and its
//! relatives become once families exist. All five are declared here rather than
//! registered, and the eight collection eliminators prompt 141c left out of the
//! core's registry are the traversals over three of them.
//!
//! `Ratio` stays a base type, and that is not an inconsistency with `Nat`: an
//! exact rational has no least element to descend to, which is the same reason
//! `02-core-calculus.md` gives for admitting no signed integer. Arithmetic on it
//! is δ, not ι.
//!
//! # What is *not* declared here, and why
//!
//! `Coordinate` and `Cat` look like enumerations and are registered as base
//! types with literal values instead. They index `Duration`, `Position`, and
//! `Syntax`, and a δ-rule is a `fn` pointer: it can build a
//! [`musa_core::Literal`] with no context and cannot build a constructor at all,
//! so `duration_of` — which answers a `Duration` and takes only a `Ratio` —
//! could not write down the type of its own answer if the index were a
//! constructor. Nothing is lost by it, because neither is pattern-matched by
//! source, which is §1's own test for inertness. See [`crate::registry`].

use std::sync::Arc;

use musa_core::{
    Cx, ElabError, Level, ModuleId, Origin, Raw, RawBinder, RawConstructor, RawData, RawFamily, Term, Visibility,
};

/// Where a declaration this module writes comes from.
///
/// Every term here is compiler-owned: no source file wrote `data Nat`, and a
/// diagnostic that pointed at one would be pointing at a line that does not
/// exist. [`Origin::UNKNOWN`] is the honest answer, and the same one
/// `musa-core`'s own suites use for a term the host assembled.
const HERE: Origin = Origin::UNKNOWN;

/// `data Bool { False, True }`.
///
/// Declared rather than registered because `if` is a `match`, and a `match`
/// needs constructors to split on. A base type with two opaque values would make
/// every conditional in the language a δ-builtin.
fn bool_data() -> RawData {
    data(
        Vec::new(),
        vec![family(
            "Bool",
            Vec::new(),
            vec![constructor("False", Vec::new()), constructor("True", Vec::new())],
        )],
    )
}

/// `data Nat { Zero, Succ(Nat) }`.
///
/// `02-core-calculus.md`'s own argument for keeping `Nat` and refusing `Int`:
/// `zero | succ` is well founded, so the recursor terminates by construction.
/// Everything the language counts with — repeat counts, list lengths, range
/// bounds — is nonnegative, so nothing musical is lost by having no predecessor
/// below zero.
fn nat_data() -> RawData {
    data(
        Vec::new(),
        vec![family(
            "Nat",
            Vec::new(),
            vec![
                constructor("Zero", Vec::new()),
                constructor("Succ", vec![binder("earlier", var("Nat"))]),
            ],
        )],
    )
}

/// `data Option (A : Type 0) { None, Some(A) }`.
fn option_data() -> RawData {
    data(
        vec![binder("A", type0())],
        vec![family(
            "Option",
            Vec::new(),
            vec![
                constructor("None", Vec::new()),
                constructor("Some", vec![binder("value", var("A"))]),
            ],
        )],
    )
}

/// `data List (A : Type 0) { Empty, Cons(A, List A) }`.
fn list_data() -> RawData {
    data(
        vec![binder("A", type0())],
        vec![family(
            "List",
            Vec::new(),
            vec![
                constructor("Empty", Vec::new()),
                constructor(
                    "Cons",
                    vec![binder("first", var("A")), binder("rest", applied("List", [var("A")]))],
                ),
            ],
        )],
    )
}

/// `data Result (T : Type 0) (E : Type 0) { Ok(T), Err(E) }`.
///
/// §1's binary sum `τ + τ`, with the spelling it is actually used for. A builtin
/// with more than one way to fail says which one happened by answering this;
/// `Option` says only that it did.
fn result_data() -> RawData {
    data(
        vec![binder("T", type0()), binder("E", type0())],
        vec![family(
            "Result",
            Vec::new(),
            vec![
                constructor("Ok", vec![binder("value", var("T"))]),
                constructor("Err", vec![binder("error", var("E"))]),
            ],
        )],
    )
}

/// `data RowFault { Fault(List Nat, List Pc12) }`.
///
/// Why the twelve-tone row's failure has a name rather than a tuple: a rule
/// answers a [`musa_core::Datum`], which is a literal or a constructor, and an
/// anonymous pair is neither. That is the mechanism noticing something true —
/// the pair was a domain concept wearing a tuple. *Open Music Theory*
/// `108-basics-of-twelve-tone-theory.md` says what the two halves are: the order
/// positions whose pitch class already appeared, and the pitch classes the
/// sequence never names. Both, rather than a choice between them, because a
/// sequence of the wrong length can have either without the other.
///
/// It is the one declaration here that is musical rather than structural, and it
/// belongs to the compiler for the same reason `Pitch` does.
fn row_fault_data() -> RawData {
    data(
        Vec::new(),
        vec![family(
            "RowFault",
            Vec::new(),
            vec![constructor(
                "Fault",
                vec![
                    binder("repeated", applied("List", [var("Nat")])),
                    binder("missing", applied("List", [var("Pc12")])),
                ],
            )],
        )],
    )
}

/// `data SyntaxStep (Context : Type 0) (Answer : Type 0) { private Step(run : Context -> Answer) }`.
///
/// One suspended recursive call into a proper child, sealed —
/// `11-quotation.md` §1's sealed step, moved off a base type and onto a family
/// whose constructor is private to [`PHASE`].
///
/// # Why a family, and why it holds a function
///
/// A step has to *capture*: which child it descends to, and under which
/// algebra. The only sound capture in this calculus is a closure, because the
/// core evaluates a rewrite's answer in the environment of the spine's
/// arguments and a λ in that answer becomes a value holding them. A base type
/// cannot hold a closure — its payload is opaque host data, and a payload
/// carrying a de Bruijn index would be meaningless the moment it left the spine
/// it was minted in. A declared family can hold a field of function type, so it
/// does.
///
/// # Why the seal survives
///
/// It used to be "this type is opaque because the compiler says so". It becomes
/// "this constructor is private to the phase module", which is 136a's own
/// mechanism, checked by the same filter as every other `private`. What a
/// transformer may do is unchanged: it receives steps and runs them, and there
/// is no spelling with which it could mint one for a node it chose. What is
/// *gained* is that the claim is now checked rather than asserted.
///
/// The family itself is public, because a transformer's own signature has to be
/// able to say `List (SyntaxStep C A)`. Hiding the type as well would hide the
/// argument type of the branch that receives it.
fn syntax_step_data() -> RawData {
    data(
        vec![binder("Context", type0()), binder("Answer", type0())],
        vec![family(
            "SyntaxStep",
            Vec::new(),
            vec![sealed(
                "Step",
                vec![binder("run", Raw::pi(HERE, "context", var("Context"), var("Answer")))],
            )],
        )],
    )
}

/// The module the expansion phase's own declarations are written in.
///
/// One module and one number, because there is one seal. It is `1` rather than
/// `0` so that a context which has not said where it is standing — [`ModuleId`]
/// is an `Option` in a context — is never confused with this one by arithmetic.
pub(crate) const PHASE: ModuleId = ModuleId::new(1);

/// The families the expansion phase declares, in the module that seals them.
///
/// Separate from [`structural`] not because the elaboration differs but because
/// the *context* does: these must be declared by a `Cx` standing in [`PHASE`],
/// or `private` would be a word with nothing behind it — 136a's own rule is that
/// a declaration written in no module hides from nobody.
pub(crate) fn phase() -> Vec<RawData> {
    vec![syntax_step_data()]
}

/// The families that name no base type, in dependency order.
///
/// These are declarable in a bare context, which is what makes them first: a
/// base type's *kind* may mention one — `Duration : Coordinate → Type 0` does —
/// so the registry cannot exist until these do.
///
/// Order matters within the group as well, because a later declaration may
/// mention an earlier one, and each is its own `data` group rather than one
/// large one: a group is the unit of mutual recursion, and putting seven
/// unrelated families in one would make positivity a question about all of them
/// at once.
pub(crate) fn structural() -> Vec<RawData> {
    vec![bool_data(), nat_data(), option_data(), list_data(), result_data()]
}

/// The families that name a base type, and so must be declared after one exists.
///
/// One today. It is a separate list rather than a comment on an ordering because
/// the ordering is a real constraint that would otherwise be discoverable only
/// by breaking it: `RowFault` holds a `List Pc12`, and `Pc12` is registered
/// rather than declared.
pub(crate) fn musical() -> Vec<RawData> {
    vec![row_fault_data()]
}

/// The term naming `name` in `cx`, for a caller assembling a builtin's type.
///
/// This is how a δ signature spells `Option Ratio`: the family constant is read
/// back out of the context that declared it, exactly as a source program's
/// `Option` is, so there is one path from a spelling to a constant and not two.
///
/// # Errors
///
/// As [`musa_core::infer`] — in practice [`musa_core::Refusal::Unbound`] for a
/// name this module did not declare.
pub(crate) fn constant(cx: &Cx, name: &str) -> Result<Term, ElabError> {
    musa_core::infer(cx, &Raw::var(HERE, name)).map(|(term, _)| term)
}

// ---- raw-syntax helpers ----
//
// The same shapes `musa-core`'s own suites build, spelled once here so that the
// declarations above read as declarations rather than as struct literals.

fn binder(name: &str, ty: Raw) -> RawBinder {
    RawBinder {
        name: Arc::from(name),
        ty,
    }
}

/// A constructor with no index arguments.
///
/// Every family here is parameterized at most, never indexed, so the index list
/// is empty in all of them and is not a parameter of this helper. The first
/// indexed family the compiler declares will want [`RawConstructor`] directly.
fn constructor(name: &str, fields: Vec<RawBinder>) -> RawConstructor {
    RawConstructor {
        origin: HERE,
        name: Arc::from(name),
        visibility: Visibility::Public,
        fields,
        indices: Vec::new(),
    }
}

/// A constructor nothing outside its own module may write.
///
/// The one place `private` appears in this module, and the only kind of seal
/// this compiler has now: [`syntax_step_data`]'s `Step`.
fn sealed(name: &str, fields: Vec<RawBinder>) -> RawConstructor {
    RawConstructor {
        visibility: Visibility::Private,
        ..constructor(name, fields)
    }
}

fn family(name: &str, indices: Vec<RawBinder>, constructors: Vec<RawConstructor>) -> RawFamily {
    RawFamily {
        name: Arc::from(name),
        visibility: Visibility::Public,
        indices,
        constructors,
    }
}

fn data(params: Vec<RawBinder>, families: Vec<RawFamily>) -> RawData {
    RawData {
        origin: HERE,
        params,
        families,
    }
}

fn var(name: &str) -> Raw {
    Raw::var(HERE, name)
}

fn applied(head: &str, arguments: impl IntoIterator<Item = Raw>) -> Raw {
    arguments
        .into_iter()
        .fold(var(head), |function, argument| Raw::app(HERE, function, argument))
}

fn type0() -> Raw {
    Raw::universe(HERE, Level::ZERO)
}

#[cfg(test)]
mod tests {
    #![expect(
        clippy::expect_used,
        clippy::panic,
        reason = "a law that cannot fail loudly is not a law"
    )]

    use musa_core::{Cx, Level, Raw, Term};

    use super::{HERE, applied, constant, structural, var};

    /// The structural families elaborate in a bare context.
    ///
    /// Worth a test rather than a comment because positivity, universe levels,
    /// and constructor-field scoping are all checked by `musa-core` and all
    /// silently absent until something declares them. It is also the claim
    /// [`structural`] makes by being separate from [`super::musical`]: these
    /// name no base type, so no registry is needed to declare them.
    ///
    /// The musical group is not tested here, because it is not declarable here.
    /// Its laws are in the registry's suite, where a base type exists.
    #[test]
    fn the_structural_families_elaborate_with_no_registry() {
        let mut cx = Cx::new();
        for declaration in structural() {
            let group = musa_core::declare(&cx, &declaration).expect("a compiler declaration elaborates");
            cx = cx.declaring(&group);
        }
        for name in ["Bool", "Nat", "Option", "List", "Result"] {
            constant(&cx, name).unwrap_or_else(|_| panic!("`{name}` is declared"));
        }
    }

    /// A constructor of each structural family is nameable by its qualified
    /// spelling.
    ///
    /// `Datum::Case` names a constructor as `Family.Case` and nothing else, so a
    /// δ-rule answering `Option.Some` is writing this string. If the qualified
    /// spelling here and the one `musa-core` builds ever disagreed, every such
    /// rule would answer `MisfitAnswer` at reduction rather than failing to
    /// build.
    #[test]
    fn each_constructor_is_nameable_by_its_qualified_spelling() {
        let mut cx = Cx::new();
        for declaration in structural() {
            let group = musa_core::declare(&cx, &declaration).expect("a compiler declaration elaborates");
            cx = cx.declaring(&group);
        }
        for name in [
            "Bool.True",
            "Nat.Succ",
            "Option.Some",
            "List.Cons",
            "Result.Ok",
            "Result.Err",
        ] {
            constant(&cx, name).unwrap_or_else(|_| panic!("`{name}` is declared"));
        }
    }

    /// The prelude's families are writable the way source writes them: a
    /// constructor names its case and supplies its fields, and never its
    /// family's parameters.
    ///
    /// `02-core-calculus.md` §2 states the rule and `musa-core`'s own suite
    /// proves it over families that suite declares. What this law adds is that
    /// the families *this* module declares are the shape it fires on. Three of
    /// the five carry parameters, and a parameterized family whose constructors
    /// could only be written `Option.Some Nat 0` is one no `.musa` file could
    /// use — `None` and `Some(register)` are what
    /// `stdlib/src/context.musa` actually writes.
    #[test]
    fn a_prelude_constructor_is_written_without_its_family_s_parameters() {
        let mut cx = Cx::new();
        for declaration in structural() {
            let group = musa_core::declare(&cx, &declaration).expect("a compiler declaration elaborates");
            cx = cx.declaring(&group);
        }
        let ty = |raw: &Raw| {
            musa_core::check(&cx, &Term::universe(HERE, Level::ZERO), raw).expect("a prelude family is a type")
        };
        let zero = var("Nat.Zero");
        let programs: &[(&str, Raw, Raw)] = &[
            ("None", applied("Option", [var("Nat")]), var("None")),
            (
                "Some(0)",
                applied("Option", [var("Nat")]),
                applied("Some", [zero.clone()]),
            ),
            (
                "Ok(0)",
                applied("Result", [var("Nat"), var("Bool")]),
                applied("Ok", [zero.clone()]),
            ),
            (
                "Err(True)",
                applied("Result", [var("Nat"), var("Bool")]),
                applied("Err", [var("Bool.True")]),
            ),
            (
                "List.Cons(0, List.Cons(0, List.Empty))",
                applied("List", [var("Nat")]),
                applied(
                    "List.Cons",
                    [zero.clone(), applied("List.Cons", [zero, var("List.Empty")])],
                ),
            ),
        ];
        for (name, at, program) in programs {
            musa_core::check(&cx, &ty(at), program).unwrap_or_else(|error| panic!("{name}: {error}"));
        }
    }
}
