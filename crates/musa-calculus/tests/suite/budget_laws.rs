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

use musa_calculus::{
    Budget, CoreError, Cx, ElabError, Metric, Origin, Raw, Role, Sort, Term, convertible, convertible_types,
    normalize_type,
};

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
    let first = convertible_types(&narrow, &deep, &Term::universe(HERE, Sort::ZERO));
    let second = convertible_types(&narrow, &deep, &Term::universe(HERE, Sort::ZERO));
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
    let type0 = Term::universe(HERE, Sort::ZERO);
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
    let type0 = Term::universe(HERE, Sort::ZERO);
    // `data Wide { Wide(f0: Type 0, …, f299: Type 0) }`. A record after prompt
    // 157, so the width that used to sit in one `RecordType` node now sits in a
    // 300-argument constructor application — and η at a one-constructor family
    // still reads all 300 fields back.
    let declared = crate::family_laws::data(
        Vec::new(),
        vec![crate::family_laws::family(
            "Wide",
            vec![crate::family_laws::constructor(
                "Wide",
                names
                    .iter()
                    .map(|name| crate::family_laws::binder(name, Raw::universe(HERE, Sort::ZERO)))
                    .collect(),
            )],
        )],
    );
    let cx = Cx::new();
    let group = musa_calculus::declare(&cx, &declared).expect("width is not depth in a declaration either");
    let cx = cx.declaring(&group);

    let wide = Term::named(HERE, "Wide", Role::TypeConstructor);
    let normal = normalize_type(&cx, &wide).expect("width is not depth");
    assert_eq!(normal, wide);

    let value = names
        .iter()
        .fold(Term::named(HERE, "Wide.Wide", Role::Constructor), |built, _| {
            Term::app(HERE, built, type0.clone())
        });
    assert_eq!(convertible(&cx, &wide, &value, &value), Ok(true));
}

/// §4.1, depth *is* depth: past the nesting limit a term is refused rather than
/// overflowing the host's stack.
///
/// `NbE` gives the machine a second way to stand inside itself — `quote` walks a
/// value the way `eval` walks a term — and a total language may refuse but may
/// not crash.
///
/// Stated on [`smallest_host`], because a law about not crashing that is only
/// checked where the stack happens to be generous is a law about the checker's
/// luck.
#[test]
fn a_term_nested_past_the_limit_is_refused() {
    on_the_smallest_host(|| {
        let cx = Cx::new();
        let too_deep = nested_lets(1_000);
        match normalize_type(&cx, &too_deep) {
            Err(CoreError::Exhausted(error)) => assert_eq!(error.metric, Metric::Nesting),
            other => panic!("expected a nesting refusal, got {other:?}"),
        }
    });
}

/// §4.1's *second* half: the same law over the chain a program actually drives.
///
/// The one above descends through `eval` alone, which is the cheapest way into
/// the meter and so the measurement that reads lowest. A program reaches the
/// same counter through `infer → check → eval → quote`, which is five times the
/// frame, and that is the chain that aborted: elaborating a tower of 220
/// constructors ended the process at about 215 levels — thirty short of the
/// limit that was supposed to refuse it, with `Metric::Nesting` never consulted
/// because nothing was left alive to consult it.
///
/// What makes this pass is not a larger limit but the room under it: §4.1 owes
/// `nesting limit × frame ceiling` bytes of stack, `musa-calculus`'s `room` module
/// arranges them, and the budget does not move, so the same terms are accepted
/// and refused as before. A regression here shows up as an *abort* rather than
/// as a failed assertion, which is the point — that is what the defect looked
/// like.
///
/// **Sixty-four past the limit and not a thousand**, because the law is only
/// this strong today. The elaborator descends the whole raw term before the
/// counter — charged on the way back up, where the values are evaluated —
/// reaches the limit, and that descent is charged nothing at all. So the room
/// covers a term some way past the limit and not one arbitrarily past it:
/// measured on this shape in a debug build, 756 levels refuse and 1,256 abort.
/// Charging `Elaborator::check` and `Elaborator::infer` is what closes that,
/// and it is prompt 165's third step rather than this repair's, because at 256
/// it would refuse programs that compile today — a cost-table version bump,
/// which §4.1 says is argued in the specification and never made to pass a
/// test.
#[test]
fn elaborating_a_term_nested_past_the_limit_is_refused() {
    on_the_smallest_host(|| {
        let cx = Cx::new();
        let too_deep = nested_raw_lets(NESTING_LIMIT.saturating_add(64));
        match musa_calculus::infer(&cx, &too_deep) {
            Err(ElabError::Exhausted(error)) => assert_eq!(error.metric, Metric::Nesting),
            other => panic!("expected a nesting refusal, got {other:?}"),
        }
    });
}

/// [`Budget::NESTING`], as the `u32` a term's depth is counted in.
const NESTING_LIMIT: u32 = if Budget::NESTING > u32::MAX as u64 {
    u32::MAX
} else {
    Budget::NESTING as u32
};

/// The stack of the smallest host this workspace runs the checker on, in bytes.
///
/// Rust's default for a spawned thread. `cargo nextest` gives a test that much,
/// and `apps/musa-desktop/src-tauri/src/session.rs` gives the session thread
/// exactly that and no more — so a law stated here is stated at the size a
/// musician's machine actually has, rather than at the 8 MiB the main thread
/// happens to start with.
const SMALLEST_HOST: usize = 2 * 1024 * 1024;

/// Run `law` on a thread the size of [`SMALLEST_HOST`], and fail with its panic.
///
/// A stack overflow inside is not catchable and does not want to be: it aborts
/// the process, the run reports `SIGABRT`, and that is the honest report of the
/// thing these two laws exist to forbid.
fn on_the_smallest_host(law: impl FnOnce() + Send) {
    let mut ran = false;
    std::thread::scope(|scope| {
        let run = || {
            law();
            ran = true;
        };
        let running = std::thread::Builder::new()
            .stack_size(SMALLEST_HOST)
            .spawn_scoped(scope, run)
            .expect("a host that can spawn a thread");
        if let Err(panic) = running.join() {
            std::panic::resume_unwind(panic);
        }
    });
    assert!(ran, "the law never ran");
}

/// `let z : Type 0 = Type 0 in … Type 0`, nested `depth` deep.
///
/// A `let` is the cheapest term that costs one nesting level and one step
/// without needing a context, which is what makes it the right shape for
/// measuring the meter rather than the language.
fn nested_lets(depth: u32) -> Term {
    let type0 = Term::universe(HERE, Sort::ZERO);
    (0..depth).fold(type0.clone(), |body, _| {
        Term::bind(HERE, "z", type0.clone(), type0.clone(), body)
    })
}

/// The same nest, written as the *raw* term an author would have written.
///
/// Not the same measurement as [`nested_lets`] and that is why both are here:
/// this one is elaborated rather than evaluated, so one level of it is a level
/// of `infer` standing inside a level of `check` standing inside `eval`, which
/// is the chain §4.1's frame ceiling is measured on.
fn nested_raw_lets(depth: u32) -> Raw {
    // Unannotated: with two fixed universes an annotation is checked as a type
    // and the one this nest used no longer is one. The unannotated form keeps
    // the chain the law measures — one level is still an `infer` standing
    // inside a `definition` standing inside an `eval`.
    (0..depth).fold(Raw::universe(HERE, Sort::ZERO), |value, _| {
        Raw::bind(HERE, "z", value, Raw::var(HERE, "z"))
    })
}
