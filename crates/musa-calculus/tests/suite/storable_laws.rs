//! `02-core-calculus.md` §1.2's `Storable`, stated as tests.
//!
//! # Why there is a suite for one constraint
//!
//! Because one is all there is. Prompt 146 deleted the class mechanism, and
//! `Storable` is the single constraint that survived it — not as an exception,
//! but because nothing about it was ever a table lookup. A machine port's
//! signature says its type stores data, and the checker *computes* the answer by
//! walking the type; the evidence is the empty record, since the walk is the
//! evidence. There is no instance to find, so there was nothing for the deleted
//! mechanism to do.
//!
//! # Why the fixture is a registry
//!
//! No author writes a constraint. [`musa_calculus::requiring_storable`] is the
//! only way to build the form and its one caller is `musa-compiler`'s machine
//! registry, which writes the schemes for constructors whose ports carry data
//! across the process boundary — so a `Raw` corpus cannot state this question at
//! all. What the laws below do is what that caller does: register a δ whose
//! signature demands a storable port, and then write a program that applies it.
//! The constraint is answered where a use site meets it, which is the spine.

use std::sync::Arc;

use musa_calculus::{Builtin, Cx, Family, Origin, Raw, Refusal, Registry, Term, check, requiring_storable};

use crate::family_laws::{core_constant, nat_context, var};
use crate::programs::WRITTEN;

/// Where this module's own terms are written.
const HERE: Origin = WRITTEN;

/// `domain → codomain`, as a core term.
fn arrow(domain: Term, codomain: Term) -> Term {
    Term::pi(HERE, "_", domain, codomain)
}

/// `name : [Storable τ] → τ → Nat`, a port that demands data.
///
/// [`Family::Machine`] rather than [`Family::Delta`], and the reason is the
/// registry's own rule: a δ's signature holds finite data and no arrows, so a δ
/// could not have a port at `Nat → Nat` to ask about. A machine builtin is the
/// family whose signatures carry constraints, which is the caller this fixture
/// stands in for.
///
/// The rule never fires. What is under test is the *signature*, and a builtin
/// that answered would only add a computation the constraint has to be
/// discharged before reaching.
fn port(name: &'static str, ported: Term, nat: Term) -> Builtin {
    Builtin::new(
        name,
        requiring_storable(HERE, ported.clone(), arrow(ported, nat)),
        Family::Machine,
        |_| None,
    )
}

/// `Nat` declared, with two ports over it: one at `Nat`, one at `Nat → Nat`.
///
/// Two rather than one because a constraint that refused everything would pass
/// the refusal law and say nothing. The pair is the smallest fixture in which
/// the walk has to *decide*.
///
/// # Panics
///
/// If the registry refuses its own signatures, which would be a defect in this
/// crate rather than a property of any law.
fn ported() -> Cx {
    let (cx, _) = nat_context();
    let nat = core_constant(&cx, "Nat");
    let function = arrow(nat.clone(), nat.clone());
    cx.with_externs(Arc::new(
        Registry::new(
            Vec::new(),
            vec![
                port("stores_data", nat.clone(), nat.clone()),
                port("stores_a_function", function, nat),
            ],
        )
        .expect("a port's scheme is a signature"),
    ))
}

/// `λn. n`, the smallest inhabitant of `Nat → Nat`.
fn identity() -> Raw {
    Raw::lam(WRITTEN, "n", var("n"))
}

/// §1.2, computed rather than assumed: a function is not storable, and the
/// refusal names the type.
///
/// The whole difference between this and the mechanism it outlived. A class
/// would answer "no instance `Storable (Nat → Nat)`", which tells the author
/// about a table; the walk answers with the type it walked into, which tells
/// them about their program.
#[test]
fn a_port_that_carries_a_function_is_refused_and_names_the_type() {
    let cx = ported();
    let nat = core_constant(&cx, "Nat");
    let Err(error) = check(&cx, &nat, &Raw::app(WRITTEN, var("stores_a_function"), identity())) else {
        panic!("a port that carries a function must be refused");
    };
    let refusal = crate::programs::refusal("a function type asked to be storable", error);
    assert!(
        matches!(refusal, Refusal::NotStorable { .. }),
        "refused, but as `{refusal}`"
    );
    assert!(
        refusal.to_string().contains("is not storable data"),
        "the refusal says what the walk found: {refusal}"
    );
}

/// The other side of §1.2, so that the law above is about *functions* rather
/// than about a constraint that refuses everything.
///
/// A declared family of data is storable, and the evidence is supplied without
/// the program mentioning it: nothing in the source below names a constraint, an
/// instance, or an argument for one.
#[test]
fn a_port_that_carries_a_declared_family_is_answered_without_being_written() {
    let cx = ported();
    let nat = core_constant(&cx, "Nat");
    check(&cx, &nat, &Raw::app(WRITTEN, var("stores_data"), var("Nat.Zero")))
        .expect("`Nat` stores only `Nat`s, and the walk says so");
}

/// The refusal this suite owns, for `elaboration_laws.rs`'s coverage gate.
///
/// It carries its own context because the question needs a registry, which no
/// other corpus has a reason to build.
pub(crate) fn refused_ports() -> Vec<(&'static str, Cx, Term, Raw, fn(&Refusal) -> bool)> {
    let cx = ported();
    let nat = core_constant(&cx, "Nat");
    vec![(
        "a function type asked to be storable",
        cx,
        nat,
        Raw::app(WRITTEN, var("stores_a_function"), identity()),
        |refusal| matches!(refusal, Refusal::NotStorable { .. }),
    )]
}
