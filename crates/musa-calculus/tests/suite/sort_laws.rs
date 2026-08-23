//! §1's universe hierarchy: predicative, non-cumulative, and polymorphic.
//!
//! Four claims, one per law below, and each is a property the hierarchy would
//! quietly lose to an ordinary-looking edit:
//!
//! - **The normal form is one.** `max` and `+1` normalize as they build, so
//!   there is no un-normalized level to compare by mistake, and normalizing a
//!   normal form changes nothing.
//! - **Equality is decidable.** `0 | u | ℓ+1 | max ℓ ℓ'` with no `imax` is a
//!   complete decision procedure — the incompleteness Lean lives with comes
//!   from the former musa does not have.
//! - **`Type u` and `Type (u+1)` are two types.** Non-cumulativity, stated
//!   where a subtyping rule would have to break it.
//! - **One definition serves two levels.** Which is what pays for
//!   non-cumulativity: the ergonomics cumulativity buys are bought here by
//!   writing the definition polymorphically instead.

use std::sync::Arc;

use musa_calculus::{
    Cx, Levels, Malformed, Origin, Program, Raw, RawProgram, RawTopLevel, Refusal, Role, Shape, Sort, SortVar, Term,
    Visibility, convertible_types,
};

use crate::programs::{WRITTEN, unit, unit_type};

/// Where the levels this file builds by hand say they were written.
const HERE: Origin = Origin::node(950);

/// A level variable of the caller's own numbering.
///
/// High-numbered on purpose: a host-built variable and an elaborator's are two
/// numberings, and `SortVar::new`'s documentation says not to mix them. Nothing
/// below hands one to an elaboration that could invent the same identity.
fn u() -> SortVar {
    SortVar::new(9000, HERE)
}

/// §1: every level is built in normal form, so normalizing one is a no-op.
///
/// Written as "the operations agree with themselves" rather than as a
/// `normalize` call, because there is no `normalize` to call — the absence is
/// the design, and this is what checks it. `max` is idempotent, commutative and
/// associative on the representation; `succ` distributes over it; a variable
/// appears once however many times it is written.
#[test]
fn a_level_is_already_in_normal_form() {
    let zero = Sort::ZERO;
    let one = Sort::ONE;
    let u = Sort::var(u());
    let cases = [
        zero.clone(),
        one.clone(),
        u.clone(),
        u.succ(),
        u.max(&one),
        u.succ().max(&u),
    ];
    for level in &cases {
        assert_eq!(*level, level.max(level), "`max ℓ ℓ` is `ℓ` for `{level}`");
        assert_eq!(level.succ(), level.max(level).succ(), "`succ` reads a normal form");
    }
    for left in &cases {
        for right in &cases {
            assert_eq!(left.max(right), right.max(left), "`max` is commutative");
            assert_eq!(
                left.max(right).succ(),
                left.succ().max(&right.succ()),
                "`succ` distributes over `max`"
            );
            for third in &cases {
                assert_eq!(
                    left.max(&right.max(third)),
                    left.max(right).max(third),
                    "`max` is associative"
                );
            }
        }
    }
    // A variable written twice under two offsets is one term at the larger,
    // which is the deduplication the normal form is *for*.
    assert_eq!(u.max(&u.succ()), u.succ(), "`max u (u+1)` is `u+1`");
    assert_eq!(zero.max(&u), u, "`max 0 u` is `u`");
}

/// §1: level equality is decided, not searched — and decided completely.
///
/// The four formers with no `imax` have a normal form that is a *complete*
/// invariant: two levels are equal exactly when their normal forms are, for
/// every valuation of their variables. Checked here on the pairs a solver would
/// have to get right, including the ones that differ only by a variable.
#[test]
fn level_equality_is_decidable() {
    let u = Sort::var(u());
    let v = Sort::var(SortVar::new(9001, HERE));
    let same: [(Sort, Sort); 5] = [
        (Sort::ZERO, Sort::constant(0)),
        (Sort::ZERO.succ(), Sort::ONE),
        (u.max(&v), v.max(&u)),
        (u.succ().succ(), Sort::constant(2).max(&u.succ().succ())),
        (u.max(&Sort::ZERO), u.clone()),
    ];
    for (left, right) in &same {
        assert_eq!(left, right, "`{left}` and `{right}` are one level");
    }
    let different: [(Sort, Sort); 4] = [
        (Sort::ZERO, Sort::ONE),
        (u.clone(), u.succ()),
        (u.clone(), v.clone()),
        (u.max(&v), u),
    ];
    for (left, right) in &different {
        assert_ne!(left, right, "`{left}` and `{right}` are two levels");
    }
}

/// §1: the hierarchy is **non-cumulative**. `Type u` and `Type (u+1)` are two
/// types, and no comparison anywhere lets one stand for the other.
///
/// The negative control for cumulativity. A `≼` admitted "just for records"
/// would make this pass in one direction, so both directions are asked.
#[test]
fn a_universe_is_not_the_one_above_it() {
    let cx = crate::programs::cx();
    let u = Sort::var(u());
    let below = Term::universe(HERE, u.clone());
    let above = Term::universe(HERE, u.succ());
    assert!(convertible_types(&cx, &below, &below).expect("a level is itself"));
    assert!(
        !convertible_types(&cx, &below, &above).expect("the comparison completes"),
        "`Type u` is not `Type (u+1)`"
    );
    assert!(
        !convertible_types(&cx, &above, &below).expect("the comparison completes"),
        "`Type (u+1)` is not `Type u`, either"
    );
    // And the closed pair the ceiling used to be about, now two ordinary types
    // rather than a program with nowhere to go.
    assert!(
        !convertible_types(&cx, &Term::universe(HERE, Sort::ZERO), &Term::universe(HERE, Sort::ONE))
            .expect("the comparison completes"),
        "`Type 0` is not `Type 1`"
    );
}

/// §1: one polymorphic definition, instantiated at two different levels in one
/// program.
///
/// This is what pays for non-cumulativity, so it is the law that has to hold
/// for the trade to be the one the design claims. `id` is written once with no
/// level anywhere in it; generalization gives it one parameter; the two uses
/// pick `0` and `1` for themselves, and the terms record which.
#[test]
fn a_polymorphic_identity_is_used_at_two_levels() {
    let cx = crate::programs::cx();
    let program = declared(&cx);
    let inside = cx.defining(&program);

    // `id {} {}` — the argument is a type at `Type 0`, so the parameter is `0`.
    let at_zero = use_of("id", [unit_type(), unit()]);
    let (term, _) = musa_calculus::infer(&inside, &at_zero).expect("`id` applies at `Type 0`");
    // `id Type {}` — the argument is `Type 0`, which is a type at `Type 1`.
    let at_one = use_of("id", [Raw::universe(WRITTEN, Sort::ZERO), unit_type()]);
    let (raised, _) = musa_calculus::infer(&inside, &at_one).expect("`id` applies at `Type 1`");

    let low = levels_of(&term);
    let high = levels_of(&raised);
    assert_eq!(low, vec![Sort::ZERO], "the first use instantiates `id` at level 0");
    assert_eq!(high, vec![Sort::ONE], "the second use instantiates `id` at level 1");
    assert_ne!(low, high, "one definition, two instantiations");
}

/// §1, and this prompt's obligation on the re-checker: a use that instantiated
/// a declaration's level parameters wrongly derives a type the term around it
/// does not accept.
///
/// A positive and a negative control over one term shape, because the claim is
/// that the *levels* decide it and nothing else: `id.{0}` applied to a type at
/// `Type 0` re-checks, and the same application with the level argument moved
/// to `1` does not. The kernel is not asked to re-infer the levels — they ride
/// on the term — so this is a verification and not a second elaboration.
#[test]
fn the_rechecker_reads_the_levels_a_use_names() {
    let cx = crate::programs::cx();
    let program = declared(&cx);
    let inside = cx.defining(&program);
    let core_unit = crate::programs::core_unit_type();
    let identity_on_unit = Term::pi(WRITTEN, "_", core_unit.clone(), core_unit.clone());

    let at = |level: Sort| {
        Term::app(
            WRITTEN,
            Term::named_at(WRITTEN, "id", Role::Defined, Levels::of([level])),
            core_unit.clone(),
        )
    };
    let checked = musa_calculus::Checked::try_from(at(Sort::ZERO)).expect("the term holds no metavariable");
    musa_calculus::recheck(&inside, &identity_on_unit, &checked).expect("`id.{0}` applies to a type at `Type 0`");

    let checked = musa_calculus::Checked::try_from(at(Sort::ONE)).expect("the term holds no metavariable");
    let outcome = musa_calculus::recheck(&inside, &identity_on_unit, &checked);
    let Err(musa_calculus::CoreError::Malformed(fault)) = outcome else {
        panic!("the kernel accepted `id.{{1}}` applied to a type at `Type 0`: {outcome:?}");
    };
    assert!(matches!(fault, Malformed::Mistyped { .. }), "{fault}");
}

/// `id : (A : Type) → A → A = λA. λx. x`, declared./// `id : (A : Type) → A → A = λA. λx. x`, declared.
///
/// # Panics
///
/// When the group does not declare, which would mean the polymorphic
/// declaration itself was refused.
fn declared(cx: &Cx) -> Arc<Program> {
    let ty = Raw::pi(
        WRITTEN,
        "A",
        Raw::any_universe(WRITTEN),
        Raw::pi(WRITTEN, "_", Raw::var(WRITTEN, "A"), Raw::var(WRITTEN, "A")),
    );
    let value = Raw::lam(WRITTEN, "A", Raw::lam(WRITTEN, "x", Raw::var(WRITTEN, "x")));
    let program = RawProgram {
        definitions: vec![RawTopLevel {
            origin: WRITTEN,
            name: "id".into(),
            visibility: Visibility::Public,
            module: None,
            ty: Some(ty),
            value,
        }],
    };
    musa_calculus::declare_program(cx, &program).expect("`id` is declared")
}

/// `name` applied to `arguments`.
fn use_of(name: &str, arguments: impl IntoIterator<Item = Raw>) -> Raw {
    arguments.into_iter().fold(Raw::var(WRITTEN, name), |built, argument| {
        Raw::app(WRITTEN, built, argument)
    })
}

/// The levels the leftmost name in `term` was instantiated at.
///
/// # Panics
///
/// When the term's head is not a name, which would mean the use did not
/// elaborate to the reference §6 says it is.
fn levels_of(term: &Term) -> Vec<Sort> {
    let mut head = term;
    while let Shape::App { function, .. } = head.shape() {
        head = function;
    }
    let Shape::Named { levels, .. } = head.shape() else {
        panic!("a use of a definition is a name");
    };
    levels.as_slice().to_vec()
}

/// The program that reaches [`Refusal::LevelMismatch`], for
/// `elaboration_laws`'s coverage gate.
///
/// A host-written level is what it takes, and that is the finding rather than a
/// weakness of the corpus: every level an *author* writes is an unknown, and an
/// unknown that nothing determines is defaulted rather than refused. So the one
/// way to write down two levels that cannot be made equal is to write them
/// down, which is what a host handing the core a type with an explicit level in
/// it does.
pub(crate) fn refused_levels() -> Vec<(&'static str, Term, Raw, fn(&Refusal) -> bool)> {
    let u = Sort::var(u());
    vec![(
        "a universe checked one level above itself",
        Term::universe(HERE, u.clone()),
        Raw::universe(WRITTEN, u.succ()),
        |refusal| matches!(refusal, Refusal::LevelMismatch { .. }),
    )]
}

/// The malformation a term reaches by naming a definition at the wrong number
/// of levels, for `malformed_laws`'s coverage gate.
///
/// # Panics
///
/// When the kernel accepts the term, or answers something else.
pub(crate) fn level_faults() -> Vec<(&'static str, Malformed)> {
    let cx = crate::programs::cx();
    let program = declared(&cx);
    let inside = cx.defining(&program);
    // `id` has one level parameter and this use names none, which no
    // elaboration produces: the instantiation walk creates one level per
    // parameter or it creates none at all.
    let term = Term::named_at(WRITTEN, "id", Role::Defined, Levels::NONE);
    let checked = musa_calculus::Checked::try_from(term).expect("the term holds no metavariable");
    let outcome = musa_calculus::recheck(&inside, &Term::universe(WRITTEN, Sort::ZERO), &checked);
    let fault = match outcome {
        Ok(()) => panic!("the kernel accepted a use at the wrong level arity"),
        Err(musa_calculus::CoreError::Malformed(fault)) => fault,
        Err(other) => panic!("reached `{other}` rather than a malformation"),
    };
    assert!(matches!(fault, Malformed::LevelArity(_)), "{fault}");
    vec![("a definition named at the wrong level arity", fault)]
}
