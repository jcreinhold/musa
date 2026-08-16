//! The budget: `docs/rules/language/02-core-calculus.md` §4's three outcomes,
//! and the independence law that keeps the third from quietly becoming the
//! second.
//!
//! > A budget may end a conversion and may never change one that finished.
//!
//! These are language acceptance limits and not wall-clock timeouts: no charge
//! reads the clock, the allocator, or machine load, so the tests below are
//! deterministic rather than flaky-by-construction. That is the property being
//! protected — a timeout here would make acceptance a property of the host.

use musa_core::{Budget, CoreError, Cx, Level, Metric, Origin, Term, convertible, convertible_types, normalize_type};

use crate::fixtures::{Sample, corpus, corpus_at};

/// As in `conversion_laws.rs`: origins are this file's *input*, not its subject.
const HERE: Origin = Origin::node(920);

/// The divisors the independence law is checked at.
///
/// Chosen to straddle the corpus: the first few leave room, the last leaves the
/// nesting limit at zero so that even building a context exhausts. Both ends are
/// the point — a law tested only where nothing runs out is not tested.
const DIVISORS: [u64; 6] = [1, 2, 10, 100, 1_000, 100_000];

/// §4, the independence law.
///
/// Under a narrower budget every question either exhausts or gives the answer
/// the language budget gave. What must never happen is a *different* answer:
/// that would make a resource limit decide a program's meaning.
#[test]
fn a_narrower_budget_exhausts_or_agrees() {
    let expected = corpus();
    for divisor in DIVISORS {
        let Ok(narrowed) = corpus_at(Budget::LANGUAGE.scaled(divisor)) else {
            // The context itself ran out, which is exhaustion and so is
            // permitted. Nothing was answered, so nothing can disagree.
            continue;
        };
        assert_eq!(narrowed.len(), expected.len(), "the corpus is the same questions");
        for (narrow, wide) in narrowed.into_iter().zip(&expected) {
            let Sample {
                name,
                cx,
                ty,
                left,
                right,
                ..
            } = narrow;
            match convertible(&cx, &ty, &left, &right) {
                Ok(answer) => assert_eq!(
                    answer, wide.equal,
                    "{name} at 1/{divisor} of the budget: an answer, and the wrong one"
                ),
                Err(CoreError::Exhausted(_)) => {}
                Err(CoreError::Malformed(malformed)) => {
                    panic!("{name} at 1/{divisor} of the budget: {malformed}")
                }
                Err(CoreError::Refused { message, .. }) => {
                    panic!("{name} at 1/{divisor} of the budget: a rule refused: {message}")
                }
            }
        }
    }
}

/// §4, the language budget answers the whole corpus.
///
/// The other half of the law above: a budget that exhausted on ordinary terms
/// would make every "or exhausts" clause vacuous.
#[test]
fn the_language_budget_answers_every_question_in_the_corpus() {
    for Sample {
        name,
        cx,
        ty,
        left,
        right,
        equal,
    } in corpus()
    {
        assert_eq!(convertible(&cx, &ty, &left, &right), Ok(equal), "{name}");
    }
}

/// §4, exhaustion is deterministic.
///
/// The same term under the same budget runs out at the same operation against
/// the same metric, every time. A refusal that moved between runs would not be a
/// property of the language.
#[test]
fn exhaustion_names_the_same_operation_and_metric_every_time() {
    let narrow = Cx::with_budget(Budget::LANGUAGE.scaled(200_000));
    let deep = nested_lets(64);
    let first = convertible_types(&narrow, &deep, &Term::universe(HERE, Level::ZERO));
    let second = convertible_types(&narrow, &deep, &Term::universe(HERE, Level::ZERO));
    match (first, second) {
        (Err(CoreError::Exhausted(one)), Err(CoreError::Exhausted(two))) => {
            assert_eq!(one.operation, two.operation);
            assert_eq!(one.metric, two.metric);
            assert_eq!(one.limit, two.limit);
            assert_eq!(one.attempted, two.attempted);
        }
        (one, two) => panic!("expected two identical exhaustions, got {one:?} and {two:?}"),
    }
}

/// §4, a budget that runs out is downward closed.
///
/// Once a term exhausts at some share of the language budget it exhausts at
/// every smaller share. A limit that let a term through only in the middle of
/// the range would mean the charges do not accumulate monotonically, which is
/// the assumption every refusal message rests on.
#[test]
fn exhaustion_is_monotone_in_the_budget() {
    let deep = nested_lets(200);
    let type0 = Term::universe(HERE, Level::ZERO);
    let mut exhausted = false;
    for divisor in DIVISORS {
        let cx = Cx::with_budget(Budget::LANGUAGE.scaled(divisor));
        match convertible_types(&cx, &deep, &type0) {
            Ok(answer) => {
                assert!(!exhausted, "a smaller budget answered where a larger one ran out");
                assert!(answer, "two hundred lets around Type 0 is Type 0");
            }
            Err(CoreError::Exhausted(_)) => exhausted = true,
            Err(CoreError::Malformed(malformed)) => panic!("{malformed}"),
            Err(CoreError::Refused { message, .. }) => panic!("a rule refused: {message}"),
        }
    }
    assert!(exhausted, "some share of the budget must be too small for this term");
}

/// §4.1, nesting is the one metric that is given back.
///
/// A record with three hundred fields is three hundred *siblings*, not three
/// hundred levels, and normalizing it walks every field twice — once to
/// evaluate, once to η-expand under quotation. It must fit inside a nesting
/// limit of 256, or the limit is measuring width.
#[test]
fn a_wide_term_is_not_a_deep_one() {
    let names: Vec<String> = (0..300).map(|field| format!("f{field}")).collect();
    let one_up = Term::universe(HERE, Level::ZERO.succ());
    let type0 = Term::universe(HERE, Level::ZERO);
    let wide = Term::record_type(HERE, names.iter().map(|name| (name.as_str(), one_up.clone())));

    let cx = Cx::new();
    let normal = normalize_type(&cx, &wide).expect("width is not depth");
    assert_eq!(normal, wide);

    let value = Term::record(HERE, names.iter().map(|name| (name.as_str(), type0.clone())));
    assert_eq!(convertible(&cx, &wide, &value, &value), Ok(true));
}

/// §4.1, depth *is* depth: past the nesting limit a term is refused rather than
/// overflowing the host's stack.
///
/// `NbE` gives the machine a second way to stand inside itself — `quote` walks a
/// value the way `eval` walks a term — and a total language may refuse but may
/// not crash.
#[test]
fn a_term_nested_past_the_limit_is_refused() {
    let cx = Cx::new();
    let too_deep = nested_lets(1_000);
    match normalize_type(&cx, &too_deep) {
        Err(CoreError::Exhausted(error)) => assert_eq!(error.metric, Metric::Nesting),
        other => panic!("expected a nesting refusal, got {other:?}"),
    }
}

/// `let z : Type 0 = Type 0 in … Type 0`, nested `depth` deep.
///
/// A `let` is the cheapest term that costs one nesting level and one step
/// without needing a context, which is what makes it the right shape for
/// measuring the meter rather than the language.
fn nested_lets(depth: u32) -> Term {
    let type0 = Term::universe(HERE, Level::ZERO);
    (0..depth).fold(type0.clone(), |body, _| {
        Term::bind(HERE, "z", type0.clone(), type0.clone(), body)
    })
}
