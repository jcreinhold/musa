//! The kernel as an oracle over programs nobody wrote.
//!
//! Every other law in this suite is stated over terms somebody chose, so each
//! covers a failure somebody thought of. This one covers the ones nobody did.
//! `TRUST.md`'s claim has a shape that makes it testable without an expected
//! output: **if elaboration accepts a program, the kernel accepts what
//! elaboration produced.** So a generator needs no oracle of its own — a
//! disagreement is a bug in *us* by construction, whatever the program was.
//!
//! That is the whole reason prompt 158 owns this file rather than a later one.
//! The re-checker is the first thing in the crate that can say a generated
//! program is wrong, and the failure it is pointed at — a de Bruijn index that
//! names the wrong binder — is exactly the kind that hand-written fixtures miss,
//! because a person writing a fixture writes the term they were already
//! thinking about.
//!
//! **The generator is the coverage.** `proptest` explores what it was written
//! to reach and nothing else, so a construct absent from [`Sketch`] is untested
//! here — which is why this file is a complement to `recheck_laws.rs`'s
//! negative-control table and not a replacement for it. The prompt records what
//! that trades away against a coverage-guided fuzzer, and names the escalation.

use proptest::prelude::{Just, Strategy, prop_oneof};
use proptest::{prop_assert, proptest};

use musa_calculus::{Checked, CoreError, Cx, Raw};

use crate::programs::{self, WRITTEN};

/// A program shape, with binders counted rather than named.
///
/// Names are assigned when the sketch is rendered, from the scope it is
/// rendered under, so a generated variable always names a binder that encloses
/// it. That is deliberate and it is not cheating: the law is about the terms
/// *elaboration* produces, and a raw term whose variable resolves to nothing is
/// refused by name resolution long before the kernel could have an opinion.
/// Generating them would spend the budget proving that name resolution works.
#[derive(Debug, Clone)]
enum Sketch {
    /// `Unit.Unit`.
    Unit,
    /// `Unit`.
    UnitType,
    /// `Type 0`.
    Universe,
    /// The binder this many steps out, counted from the innermost.
    Var(usize),
    /// `λx. body`.
    Lam(Box<Self>),
    /// `f a`.
    App(Box<Self>, Box<Self>),
    /// `let x = value in body`, annotated at `Unit` so the `let` has a type to
    /// state.
    Let(Box<Self>, Box<Self>),
    /// `{ ty = …, val = … }` at the corpus's one dependent family.
    Cell(Box<Self>, Box<Self>),
    /// `subject.val`.
    Project(Box<Self>),
    /// `(term : Unit)`.
    Annotated(Box<Self>),
}

/// Sketches up to a small depth, weighted towards the shapes that elaborate.
///
/// Skewed on purpose. An unweighted grammar over this alphabet produces almost
/// nothing well typed, and a run in which elaboration never accepts anything is
/// a run in which the oracle was never asked —
/// [`the_generator_reaches_programs_elaboration_accepts`] is the law that stops
/// that from passing silently.
fn sketch() -> impl Strategy<Value = Sketch> {
    let leaf = prop_oneof![
        4 => Just(Sketch::Unit),
        2 => Just(Sketch::UnitType),
        1 => Just(Sketch::Universe),
        4 => (0_usize..4).prop_map(Sketch::Var),
    ];
    leaf.prop_recursive(4, 24, 3, |inner| {
        prop_oneof![
            3 => inner.clone().prop_map(|body| Sketch::Lam(Box::new(body))),
            3 => (inner.clone(), inner.clone())
                .prop_map(|(function, argument)| Sketch::App(Box::new(function), Box::new(argument))),
            2 => (inner.clone(), inner.clone())
                .prop_map(|(value, body)| Sketch::Let(Box::new(value), Box::new(body))),
            2 => (inner.clone(), inner.clone())
                .prop_map(|(ty, val)| Sketch::Cell(Box::new(ty), Box::new(val))),
            2 => inner.clone().prop_map(|subject| Sketch::Project(Box::new(subject))),
            1 => inner.prop_map(|term| Sketch::Annotated(Box::new(term))),
        ]
    })
}

/// The binder names a rendered sketch may write, innermost last.
const BINDERS: [&str; 4] = ["a", "b", "c", "d"];

/// The name at `depth`, wrapping round [`BINDERS`].
///
/// Total, and written without `%` or an index because the workspace lints
/// forbid both — deliberately, and the reason applies here as much as anywhere:
/// a generator that panicked on its own scope bookkeeping would report a bug in
/// this harness as a bug in the kernel, which is the one failure an oracle must
/// not have.
fn binder(depth: usize) -> &'static str {
    BINDERS.iter().copied().cycle().nth(depth).unwrap_or("a")
}

/// `sketch` as a raw term, under `depth` binders already introduced.
fn render(sketch: &Sketch, depth: usize) -> Raw {
    match *sketch {
        Sketch::Unit => programs::unit(),
        Sketch::UnitType => programs::unit_type(),
        Sketch::Universe => Raw::universe(WRITTEN, musa_calculus::Sort::ZERO),
        // Nothing is in scope at the top level, so a variable there is written
        // as the one closed term that always elaborates.
        Sketch::Var(back) => match depth.checked_sub(back.checked_rem(depth).unwrap_or(0).saturating_add(1)) {
            Some(position) => Raw::var(WRITTEN, binder(position)),
            None => programs::unit(),
        },
        Sketch::Lam(ref body) => {
            let name = binder(depth);
            Raw::annotated_lam(
                WRITTEN,
                name,
                programs::unit_type(),
                render(body, depth.saturating_add(1)),
            )
        }
        Sketch::App(ref function, ref argument) => Raw::app(WRITTEN, render(function, depth), render(argument, depth)),
        Sketch::Let(ref value, ref body) => {
            let name = binder(depth);
            Raw::annotated_bind(
                WRITTEN,
                name,
                programs::unit_type(),
                render(value, depth),
                render(body, depth.saturating_add(1)),
            )
        }
        Sketch::Cell(ref ty, ref val) => Raw::record(WRITTEN, [("ty", render(ty, depth)), ("val", render(val, depth))]),
        Sketch::Project(ref subject) => Raw::project(WRITTEN, render(subject, depth), "val"),
        Sketch::Annotated(ref term) => Raw::annot(WRITTEN, render(term, depth), programs::unit_type()),
    }
}

/// Whether elaboration accepted this sketch, having required the kernel to
/// agree wherever it did.
///
/// The oracle, and the two answers it does not treat as disagreement are the
/// same two every other law here excludes: a program elaboration refused says
/// nothing about the kernel, and a budget that ran out is not a verdict either
/// way.
fn agrees(cx: &Cx, sketch: &Sketch) -> Result<bool, String> {
    let raw = render(sketch, 0);
    let Ok((term, ty)) = musa_calculus::infer(cx, &raw) else {
        return Ok(false);
    };
    let checked = Checked::try_from(term).map_err(|fault| {
        format!("elaboration accepted a term still holding an unsolved metavariable: {fault} — {sketch:?}")
    })?;
    match musa_calculus::recheck(cx, &ty, &checked) {
        Ok(()) | Err(CoreError::Exhausted(_)) => Ok(true),
        Err(fault) => Err(format!(
            "the kernel rejects a term elaboration accepted: {fault} — {sketch:?}"
        )),
    }
}

proptest! {
    /// `TRUST.md`'s claim, over programs nobody chose.
    ///
    /// The assertion is one-sided because the claim is: a program elaboration
    /// *refused* is not evidence about the kernel, so the only way to fail is
    /// for [`agrees`] to report a disagreement.
    #[test]
    fn the_kernel_accepts_whatever_elaboration_produced(sketch in sketch()) {
        match agrees(&programs::cx(), &sketch) {
            Ok(_) => {}
            Err(report) => prop_assert!(false, "{}", report),
        }
    }
}

/// The law that stops the one above from passing vacuously.
///
/// A generator that produces nothing elaboration accepts asks the oracle
/// nothing, and every run would be green. This is the same argument
/// `recheck_laws.rs` makes about its negative controls, pointed at the
/// generator instead: a property nobody has watched *fire* is a property that
/// does not hold anything up.
#[test]
fn the_generator_reaches_programs_elaboration_accepts() {
    use proptest::strategy::ValueTree;
    use proptest::test_runner::{Config, TestRunner};

    let cx = programs::cx();
    let mut runner = TestRunner::new(Config {
        cases: 512,
        ..Config::default()
    });
    let strategy = sketch();
    let mut accepted = 0_u32;
    for _ in 0..512 {
        let Ok(tree) = strategy.new_tree(&mut runner) else {
            continue;
        };
        let sketch = tree.current();
        match agrees(&cx, &sketch) {
            Ok(true) => accepted = accepted.saturating_add(1),
            Ok(false) => {}
            Err(report) => panic!("{report}"),
        }
    }
    assert!(
        accepted >= 16,
        "the generator reached only {accepted} programs elaboration accepts in 512 draws, \
         so the oracle above is close to never being asked"
    );
}
