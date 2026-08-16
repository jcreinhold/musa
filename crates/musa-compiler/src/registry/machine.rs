//! `../../rules/across-stages/03-machine-calculus.md` §2's machine forms, as
//! core constructors.
//!
//! §5.8's *fourth* family, and the one that reduces least: §2 gives its forms
//! typing rules and **no reductions**, so a machine's application is its value
//! and two machines are the same machine exactly when they were built the same
//! way. [`musa_core::Builtin::constructor`] is how a table says that, and this
//! module is its first caller.
//!
//! # Why none of these could have been a rule
//!
//! Every form is polymorphic in its step tag and its ports, so a *type* stands
//! on every one of their spines. [`musa_core::eval`]'s `canonical` answers
//! `None` at a universe, so a δ-rule registered here would block forever and
//! [`musa_core::Malformed::BuiltinStuck`] would never even get the chance to
//! report it. That is not a mechanism to route around: a machine has nothing to
//! compute until §3 gives it a step, and §3 is prompts 150–153.
//!
//! # What the ports are, and what is not checked here
//!
//! §2's premises read `data A`: a port holds storable data, never a function.
//! The old checker enforced it by giving every port a [`crate::infer::Kind::Data`]
//! variable, whose [`crate::infer::Unifier::bind`] refuses an arrow at any
//! depth. Half of that survives the restatement exactly and half does not, and
//! which is which is worth writing down rather than discovering:
//!
//! - **The step tag survives.** It is one implicit binder shared by every
//!   argument of a form, so `connect`'s two machines must agree on it and §2's
//!   "the step tag `K` prevents machines whose steps mean different things from
//!   being connected" is a property of the signature.
//! - **Storability does not.** `02-core-calculus.md` §1.2 states it as the
//!   `Storable` constraint, and a constraint is discharged during elaboration by
//!   resolving a dictionary — a [`musa_core::Builtin`]'s type is a
//!   [`Term`], which has explicit and implicit binders and no constraint binder.
//!   Writing one as an ordinary Π at the empty dictionary type would leave an
//!   unsolved metavariable at every use, which is a worse answer than none.
//!
//! So `Machine ⟨step⟩ (Nat → Nat) Nat` is writable in the core today and
//! refused by nothing in this module. The check belongs to the elaboration that
//! has the constraint solver — prompt 142, which is also where the surface
//! learns to spell these — and preparation refuses what is left
//! (§5: "a well-typed machine may still fail preparation").
//!
//! # The ninth form is not here
//!
//! `primitive` is §1's, not §2's: §2's grammar starts at `machine(p)` and gives
//! `primitive` no typing rule at all, because its type is read out of the
//! build-local registry rather than written down. See [`UNREGISTERED`].

use musa_core::{Builtin, Cx, ElabError, Index, Term};

use super::{HERE, ported, type0};

/// The eight forms registered here, in `BUILTIN_OWNERSHIP`'s order.
pub(super) const SPELLINGS: [&str; 8] = [
    "machine", "identity", "connect", "beside", "feedback", "copy", "drop", "swap",
];

/// The ninth machine row, which is registered nowhere and is prompt 142's.
///
/// `primitive(name, version, configuration)` is typed by a registry rather than
/// by a signature. [`crate::core::MachineOp::instantiate`] returns `None` for
/// it and the old checker's `registered_instance` answers instead: the written
/// name and version select a descriptor from [`crate::machine`], and *that*
/// supplies the step, the two ports, **and the type of the configuration
/// argument** — which differs per unit, so it is not one Π short of writable,
/// it is a different type per registered pair.
///
/// The registrable alternative was to take all of it explicitly —
/// `(step input output configuration : Type 0) → Text → Nat → configuration →
/// Primitive step input output` — and it was refused. Four type arguments the
/// name and version already decide are four chances for a program to say
/// something the registry contradicts, and a signature that admits them is a
/// second way to type a `primitive`: the audits' own smell. What is registered
/// would not be §1's operation, only an operation that shares its spelling.
///
/// It goes to prompt 142 because that is where elaboration replaces the old
/// checker and therefore where a build-local lookup can happen at all. Counted
/// in [`super::rules::UNREGISTERED`] so the accounting stays honest in the
/// meantime.
pub(super) const UNREGISTERED: [&str; 1] = ["primitive"];

/// The eight forms, read off [`crate::core::MachineOp::instantiate`] rather than
/// retyped.
///
/// ```text
/// machine  : {K A B   : Type 0} → Primitive K A B → Machine K A B
/// identity : {K A     : Type 0} → Machine K A A
/// connect  : {K A B C : Type 0} → Machine K A B → Machine K B C → Machine K A C
/// beside   : {K A B C D : Type 0} → Machine K A B → Machine K C D
///                                 → Machine K (Pair A C) (Pair B D)
/// feedback : {K A B F : Type 0} → F → Machine K (Pair A F) (Pair B F)
///                               → Machine K A B
/// copy     : {K A     : Type 0} → Machine K A (Pair A A)
/// drop     : {K A     : Type 0} → Machine K A Unit
/// swap     : {K A B   : Type 0} → Machine K (Pair A B) (Pair B A)
/// ```
///
/// Implicit binders throughout, and that is what makes `identity` a machine
/// rather than a function to one: §2 names four of the eight rather than
/// applying them, and an implicit argument is one a use site does not write.
/// The ports of a nullary form are then solved from the position it stands in,
/// which is the same thing the old checker's fresh unification variables did.
///
/// # Errors
///
/// [`ElabError`] when `Pair` or `Unit` is not declared in `cx`, which is a
/// compiler defect.
pub(super) fn builtins(cx: &Cx) -> Result<Vec<Builtin>, ElabError> {
    let words = Words {
        pair: crate::prelude::constant(cx, "Pair")?,
        unit: crate::prelude::constant(cx, "Unit")?,
    };
    Ok([
        machine_type(),
        identity_type(),
        connect_type(),
        beside_type(&words),
        feedback_type(&words),
        copy_type(&words),
        drop_type(&words),
        swap_type(&words),
    ]
    .into_iter()
    .zip(SPELLINGS)
    .map(|(ty, spelling)| Builtin::constructor(spelling, ty, musa_core::Family::Machine))
    .collect())
}

/// The declared constants a signature here names, resolved once.
///
/// Two, and both from [`crate::prelude`]: §2 writes its ports over `(A, D)` and
/// `Unit`, and those are `Pair` and `Unit`. Resolved through the context that
/// declared them for [`super::traversal`]'s reason — the term a signature names
/// and the term a source program's `Pair` denotes are one term, not two that
/// agree today.
struct Words {
    pair: Term,
    unit: Term,
}

// ---- the eight signatures ---------------------------------------------------
//
// Each is written in terms of the *positions* its binders stand at, counted from
// the outermost, and `at` turns a position into the index it has at a given
// depth. That is [`super::traversal`]'s idiom and it is here for the same
// reason: `step` is `Index(3)` in `connect`'s first argument and `Index(5)` in
// its result, and a signature that wrote those numbers would be a signature
// nobody could check by reading.

/// The step tag, which every form binds first.
const STEP: usize = 0;

fn machine_type() -> Term {
    let (step, input, output) = (STEP, 1, 2);
    let bound = 3;
    scheme(
        &["step", "input", "output"],
        vec![ported_type("Primitive", bound, step, input, output)],
        ported_type("Machine", bound + 1, step, input, output),
    )
}

fn identity_type() -> Term {
    let (step, port) = (STEP, 1);
    scheme(
        &["step", "port"],
        Vec::new(),
        ported_type("Machine", 2, step, port, port),
    )
}

fn connect_type() -> Term {
    let (step, input, middle, output) = (STEP, 1, 2, 3);
    let bound = 4;
    scheme(
        &["step", "input", "middle", "output"],
        vec![
            ported_type("Machine", bound, step, input, middle),
            ported_type("Machine", bound + 1, step, middle, output),
        ],
        ported_type("Machine", bound + 2, step, input, output),
    )
}

fn beside_type(words: &Words) -> Term {
    let (step, input, output, other_input, other_output) = (STEP, 1, 2, 3, 4);
    let bound = 5;
    let at_depth = bound + 2;
    scheme(
        &["step", "input", "output", "other_input", "other_output"],
        vec![
            ported_type("Machine", bound, step, input, output),
            ported_type("Machine", bound + 1, step, other_input, other_output),
        ],
        applied(
            ported("Machine").term(HERE),
            [
                at(at_depth, step),
                pair(words, at(at_depth, input), at(at_depth, other_input)),
                pair(words, at(at_depth, output), at(at_depth, other_output)),
            ],
        ),
    )
}

fn feedback_type(words: &Words) -> Term {
    let (step, input, output, stored) = (STEP, 1, 2, 3);
    let bound = 4;
    let inner = bound + 1;
    scheme(
        &["step", "input", "output", "stored"],
        vec![
            at(bound, stored),
            applied(
                ported("Machine").term(HERE),
                [
                    at(inner, step),
                    pair(words, at(inner, input), at(inner, stored)),
                    pair(words, at(inner, output), at(inner, stored)),
                ],
            ),
        ],
        ported_type("Machine", bound + 2, step, input, output),
    )
}

fn copy_type(words: &Words) -> Term {
    let (step, port) = (STEP, 1);
    let bound = 2;
    scheme(
        &["step", "port"],
        Vec::new(),
        applied(
            ported("Machine").term(HERE),
            [
                at(bound, step),
                at(bound, port),
                pair(words, at(bound, port), at(bound, port)),
            ],
        ),
    )
}

fn drop_type(words: &Words) -> Term {
    let (step, port) = (STEP, 1);
    let bound = 2;
    scheme(
        &["step", "port"],
        Vec::new(),
        applied(
            ported("Machine").term(HERE),
            [at(bound, step), at(bound, port), words.unit.clone()],
        ),
    )
}

fn swap_type(words: &Words) -> Term {
    let (step, first, second) = (STEP, 1, 2);
    let bound = 3;
    scheme(
        &["step", "first", "second"],
        Vec::new(),
        applied(
            ported("Machine").term(HERE),
            [
                at(bound, step),
                pair(words, at(bound, first), at(bound, second)),
                pair(words, at(bound, second), at(bound, first)),
            ],
        ),
    )
}

// ---- writing a signature ----------------------------------------------------

/// `{v₁ … vₙ : Type 0} → α₁ → … → αₘ → ρ`.
///
/// Every argument must already be written at the depth its own position gives it
/// — `n` for the first and `n + k` for the k-th — and the result at `n + m`.
/// Folding from the right is what makes that true, exactly as in
/// [`super::traversal`]'s telescope.
fn scheme(binders: &[&'static str], arguments: Vec<Term>, result: Term) -> Term {
    let applied = arguments
        .into_iter()
        .rev()
        .fold(result, |built, argument| Term::pi(HERE, "argument", argument, built));
    binders
        .iter()
        .rev()
        .fold(applied, |built, name| Term::implicit_pi(HERE, *name, type0(), built))
}

/// `Machine step input output` or `Primitive step input output`, where all three
/// are bound variables read at `depth`.
///
/// Six of the eight signatures need nothing else; the two that pair a port reach
/// for [`applied`] and [`pair`] directly.
fn ported_type(name: &'static str, depth: usize, step: usize, input: usize, output: usize) -> Term {
    applied(
        ported(name).term(HERE),
        [at(depth, step), at(depth, input), at(depth, output)],
    )
}

/// `Pair first second`.
fn pair(words: &Words, first: Term, second: Term) -> Term {
    applied(words.pair.clone(), [first, second])
}

/// `head a b …`.
fn applied(head: Term, arguments: impl IntoIterator<Item = Term>) -> Term {
    arguments
        .into_iter()
        .fold(head, |function, argument| Term::app(HERE, function, argument))
}

/// The variable bound at `position`, counted from the outermost, read at
/// `depth`.
fn at(depth: usize, position: usize) -> Term {
    let index = depth.saturating_sub(position).saturating_sub(1);
    Term::var(HERE, Index(u32::try_from(index).unwrap_or_default()))
}

#[cfg(test)]
mod laws;
