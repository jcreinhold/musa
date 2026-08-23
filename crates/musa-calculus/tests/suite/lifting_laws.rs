//! A `rec` written inside a term, and the definition it becomes.
//!
//! `kernel/program.rs`'s module doc states the premise: a definition is a
//! **global name**, a de Bruijn binder refers outward only, and §1.3 refuses the
//! fixed point that would let a local recursion stand without a name. So a `rec`
//! that captures the binders around it is lifted out — abstracted over its whole
//! context, declared as an auxiliary member of the program, and left behind as
//! that member applied to the binders it abstracted (Peyton Jones ch. 13, and
//! ch. 14 for the recursive case "without using Y").
//!
//! ```text
//! meaning:     the lift relocates and nothing else — the captured binders still
//!              reach the body, and the definition still computes
//! membership:  the program holds a member no source line wrote, and says so
//!              without a reader having to look at how the name is punctuated
//! reach:       that member is private to the module its parent was written in
//! audit:       the kernel re-checks a term naming it
//! ```
//!
//! `termination_laws.rs` is the other half of this file and is deliberately not
//! touched: a `rec` there is elaborated by a bare `check`, with no program to
//! join, and it must go on meaning exactly what it meant. If a law there had to
//! change, the lift would have changed a recursion rather than moved one.

use musa_calculus::{Cx, ModuleId, Raw, RawArm, RawPattern, RawProgram, RawTopLevel, Refusal, Term, Visibility};

use crate::coverage_laws::nat_vec_context;
use crate::family_laws::{apply, var};
use crate::programs::{WRITTEN, refusal};

/// The module the fixture below is written in.
const INSIDE: ModuleId = ModuleId::new(1);

/// A module that is not [`INSIDE`].
const OUTSIDE: ModuleId = ModuleId::new(2);

/// What the lift calls the `rec` in [`bump`], given the definition it is in.
const LIFTED: &str = "bump#walk";

fn arrow(domain: Raw, codomain: Raw) -> Raw {
    Raw::pi(WRITTEN, "_", domain, codomain)
}

fn arm(patterns: Vec<RawPattern>, body: Raw) -> RawArm {
    RawArm { patterns, body }
}

fn number(count: u32) -> Raw {
    (0..count).fold(var("Nat.Zero"), |built, _| apply(var("Nat.Succ"), [built]))
}

/// `bump : Nat → Nat → Nat = λn. rec walk : Nat → Nat = λm. match m { … }`.
///
/// The `rec` is in **term position** — it stands under `n`, and its `Zero` arm
/// answers with `n`, so the body genuinely captures a binder and "lift it and
/// abstract nothing" is not available. Written in [`INSIDE`] so the reach law
/// has an outside to ask from.
fn bump() -> RawTopLevel {
    let walk_ty = arrow(var("Nat"), var("Nat"));
    RawTopLevel {
        origin: WRITTEN,
        name: "bump".into(),
        visibility: Visibility::Public,
        module: Some(INSIDE),
        ty: Some(arrow(var("Nat"), walk_ty.clone())),
        value: Raw::lam(
            WRITTEN,
            "n",
            Raw::rec(
                WRITTEN,
                "walk",
                walk_ty,
                Raw::lam(
                    WRITTEN,
                    "m",
                    Raw::match_on(
                        WRITTEN,
                        [var("m")],
                        vec![
                            arm(vec![RawPattern::constructor(WRITTEN, "Nat.Zero", [])], var("n")),
                            arm(
                                vec![RawPattern::constructor(
                                    WRITTEN,
                                    "Nat.Succ",
                                    [RawPattern::bind(WRITTEN, "k")],
                                )],
                                apply(var("Nat.Succ"), [apply(var("walk"), [var("k")])]),
                            ),
                        ],
                    ),
                ),
            ),
        ),
    }
}

/// The fixture's context, and the program it declares.
///
/// # Panics
///
/// When the group refuses, which every law here says it does not.
pub(crate) fn declared() -> (Cx, std::sync::Arc<musa_calculus::Program>) {
    let cx = nat_vec_context();
    let program = musa_calculus::declare_program(
        &cx,
        &RawProgram {
            definitions: vec![bump()],
        },
    )
    .unwrap_or_else(|error| panic!("the lifting fixture declares: {error}"));
    let cx = cx.defining(&program);
    (cx, program)
}

/// The type `Nat`, as a core term.
///
/// # Panics
///
/// If `Nat` is not in scope, which is a defect in the fixture.
fn nat(cx: &Cx) -> Term {
    musa_calculus::infer(cx, &var("Nat")).expect("Nat is declared").0
}

/// The lift relocates: the captured binder still reaches the body, and the
/// definition still computes.
///
/// `bump 2 3` is `5` only if `n` — a binder of the *parent*, abstracted by the
/// lift and applied back at the use site — arrives in the `Zero` arm. A lift
/// that dropped it, or that abstracted the binders in the other order, would
/// answer something else rather than fail to elaborate.
#[test]
fn a_rec_that_captures_its_enclosing_binders_computes_through_the_lift() {
    let (cx, _) = declared();
    let nat = nat(&cx);
    let applied = apply(var("bump"), [number(2), number(3)]);
    let term = musa_calculus::check(&cx.in_module(INSIDE), &nat, &applied).expect("the application checks");
    let answer = musa_calculus::normalize(&cx, &nat, &term).expect("it computes");
    let expected = musa_calculus::check(&cx, &nat, &number(5)).expect("five checks");
    let expected = musa_calculus::normalize(&cx, &nat, &expected).expect("five computes");
    assert_eq!(answer, expected, "bump 2 3 is 5");
}

/// The program holds a member nothing in the source wrote, and says which.
///
/// Read off the flag the lift set rather than off the spelling of the name: a
/// caller that told the two apart by looking for a `#` would be a second rule
/// to keep in step with the first.
#[test]
fn the_lifted_definition_is_a_member_of_the_program_and_is_private() {
    let (_, program) = declared();
    let lifted = program.lifted();
    assert_eq!(
        lifted,
        vec![(std::sync::Arc::from(LIFTED), Visibility::Private)],
        "one definition was lifted, and it is private"
    );
}

/// No program reaches a lifted definition from outside the module its parent
/// was written in.
///
/// The other half of "no program can name one" is not this crate's to state:
/// `#` is not an identifier character, so no `.musa` source can spell the name
/// at all. What the core owes is that the name is not a back door for a caller
/// that can spell it, and that is visibility.
#[test]
fn a_lifted_definition_is_out_of_reach_from_another_module() {
    let (cx, _) = declared();
    let outside = cx.in_module(OUTSIDE);
    let error = musa_calculus::infer(&outside, &var(LIFTED)).expect_err("the lifted name is not in reach");
    assert!(
        matches!(
            refusal("a lifted definition is private", error),
            Refusal::UnknownName { .. } | Refusal::Private { .. }
        ),
        "naming a lifted definition from outside its module is refused"
    );
}

/// The kernel re-checks a term that names a definition nothing in the source
/// wrote.
///
/// A lifted member is an ordinary definition and its use is an ordinary
/// application, so the existing arms cover both — but they had never been
/// handed a member the source did not write, and a re-checker is only known to
/// walk what it has walked.
#[test]
fn the_kernel_rechecks_a_term_naming_a_lifted_definition() {
    let (cx, _) = declared();
    let inside = cx.in_module(INSIDE);
    let nat = nat(&cx);
    let applied = apply(var(LIFTED), [number(1), number(4)]);
    let term = musa_calculus::check(&inside, &nat, &applied).expect("the lifted definition is applicable");
    let checked = musa_calculus::Checked::try_from(term).expect("a finished term");
    musa_calculus::recheck(&inside, &nat, &checked).expect("the kernel agrees");
}
