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
    Budget, Builtin, CoreError, Cx, Datum, ElabError, Family, Metric, Origin, Raw, RawArm, RawPattern, Registry, Role,
    Sort, Term, convertible, convertible_types, normalize_type,
};
use std::sync::Arc;

use crate::family_laws::{apply, nat_context, var};
use crate::fixtures::{Sample, corpus, corpus_at};
use crate::programs::WRITTEN;

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

/// `rec add : Nat → Nat → Nat = λa. λb. match a { Zero => b; Succ k => Succ (add k b) }`.
///
/// `termination_laws.rs`'s definition, written again here because what these two
/// laws measure is not whether it is admitted — that law is over there — but
/// what running it *costs*. It is the smallest definition that recurses once per
/// unit of its argument, so `add n 0` is exactly `n` recursive calls and nothing
/// else, which is what makes the counts below countable.
fn add() -> Raw {
    let arrow = |domain: Raw, codomain: Raw| Raw::pi(WRITTEN, "_", domain, codomain);
    Raw::rec(
        WRITTEN,
        "add",
        arrow(var("Nat"), arrow(var("Nat"), var("Nat"))),
        Raw::lam(
            WRITTEN,
            "a",
            Raw::lam(
                WRITTEN,
                "b",
                Raw::match_on(
                    WRITTEN,
                    [var("a")],
                    vec![
                        RawArm {
                            patterns: vec![RawPattern::constructor(WRITTEN, "Nat.Zero", [])],
                            body: var("b"),
                        },
                        RawArm {
                            patterns: vec![RawPattern::constructor(
                                WRITTEN,
                                "Nat.Succ",
                                [RawPattern::bind(WRITTEN, "k")],
                            )],
                            body: apply(var("Nat.Succ"), [apply(var("add"), [var("k"), var("b")])]),
                        },
                    ],
                ),
            ),
        ),
    )
}

/// `add count 0`, as a raw term with [`add`]'s type written down.
fn adding(count: u64) -> Raw {
    let arrow = |domain: Raw, codomain: Raw| Raw::pi(WRITTEN, "_", domain, codomain);
    apply(
        Raw::annot(WRITTEN, add(), arrow(var("Nat"), arrow(var("Nat"), var("Nat")))),
        [Raw::numeral(WRITTEN, "Nat", count), Raw::numeral(WRITTEN, "Nat", 0)],
    )
}

/// `Nat`, and a context that has it.
///
/// # Panics
///
/// If `Nat` is not a type, which would be a defect in this crate.
fn nat() -> (Cx, Term) {
    let (cx, _) = nat_context();
    let nat = musa_calculus::infer(&cx, &var("Nat")).expect("`Nat` is a type").0;
    (cx, nat)
}

/// §4.1, a definition calling itself is *work* and not *depth*.
///
/// The law prompt 165a exists to make statable. Until it ran, a recursive call
/// was evaluated inside the enclosing evaluation and held its level until the
/// call beneath it finished, so [`Budget::NESTING`] bounded recursion as well as
/// term depth: measured at the commit before the change, `add 105 0` answered
/// and `add 106 0` was refused at 321 of 320 nesting levels — about three levels
/// a call, which is 155a's bisection seen from the other end.
///
/// Three thousand calls now, which is not a bigger limit but a different metric:
/// the evaluator's pending work is heap data, so the depth a recursion reaches
/// is a length rather than a stack. What bounds it is [`Metric::Steps`], and the
/// companion law below is where that is stated.
///
/// **Eight nesting levels, and the number is the point.** Not "under the limit"
/// — under a budget two orders of magnitude below the recursion's own depth, so
/// that no reading of the result can be that 320 happened to be enough. Three is
/// the measured minimum, and what refuses at two is *unification*, which is the
/// elaborator's structural work on the definition's type and has nothing to do
/// with how far the recursion went. The five levels between are slack for that
/// structural work rather than for the recursion.
#[test]
fn a_definition_recursing_far_past_the_nesting_limit_is_accepted() {
    let (cx, nat) = nat();
    let narrow = Cx::with_budget(Budget::LANGUAGE.nesting(8)).declaring(&nat_context().1);
    let calls = 3_000;
    assert!(
        calls > Budget::NESTING,
        "the law is only a law if the recursion is past the limit"
    );
    let term = musa_calculus::check(&narrow, &nat, &adding(calls)).expect("three thousand calls elaborate");
    let answer = musa_calculus::normalize(&narrow, &nat, &term).expect("and run, inside eight nesting levels");
    assert_eq!(
        answer,
        musa_calculus::check(&cx, &nat, &Raw::numeral(WRITTEN, "Nat", calls)).expect("a numeral")
    );
}

/// §4, what does bound a recursion: the step budget, at the price the frames
/// charged.
///
/// The other half of the law above, and the one that keeps it from being a hole.
/// Taking recursion off the nesting metric is only sound if every recursive step
/// is still charged *something* that runs out, and if the explicit control stack
/// charges what the host frames it replaced charged. Both are stated here as one
/// number: `add n 0` costs `9 + 12n` reduction steps, exactly, for every `n`.
///
/// **The twelve is measured against the frame-based evaluator, not derived from
/// the new one.** At commit `3846ec72`, the last before the control stack,
/// `add 1 0` cost 21 steps, `add 2 0` cost 33, `add 10 0` cost 129 and
/// `add 100 0` cost 1,209 — the four numbers below, byte for byte. A hand charge
/// that drifted from what the frames charged would be a silent acceptance
/// change, and this is what notices.
///
/// So the wall moved from a depth to a length: at the language budget's 200,000
/// steps the last count that answers is 16,665, where the frame-based evaluator
/// stopped at 105.
#[test]
fn a_recursion_costs_the_same_steps_the_frames_charged() {
    let (cx, nat) = nat();
    for (calls, steps) in [(1_u64, 21_u64), (2, 33), (10, 129), (100, 1_209)] {
        let term = musa_calculus::check(&cx, &nat, &adding(calls)).expect("a call elaborates");
        let (_, spend) = musa_calculus::normalize_metered(&cx, &nat, &term).expect("and runs");
        assert_eq!(spend.steps, steps, "add {calls} 0");
        assert_eq!(spend.steps, 9 + 12 * calls, "add {calls} 0 against the closed form");
    }

    let past = musa_calculus::check(&cx, &nat, &adding(20_000)).expect("twenty thousand calls elaborate");
    match musa_calculus::normalize(&cx, &nat, &past) {
        Err(CoreError::Exhausted(error)) => {
            assert_eq!(
                error.metric,
                Metric::Steps,
                "a recursion runs out of work, not of depth"
            );
            assert_eq!(error.limit, Budget::LANGUAGE.steps());
        }
        other => panic!("expected a step refusal, got {other:?}"),
    }
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

/// `let z : Type 0 = (let z : Type 0 = … in Type 0) in Type 0`, nested `depth`
/// deep **in the value**.
///
/// A `let` is the cheapest term that costs one nesting level and one step
/// without needing a context, which is what makes it the right shape for
/// measuring the meter rather than the language.
///
/// The nest is in the value and not in the body because those are two different
/// measurements now that the evaluator carries its own control stack. A `let`'s
/// body is its *tail*: the value is already computed when the body is entered,
/// so nothing of the enclosing term is still being descended into and the level
/// is given back — a chain of a million lets in body position is a loop, and
/// costs one level. The value is the descent, so it is the shape that measures
/// depth.
fn nested_lets(depth: u32) -> Term {
    let type0 = Term::universe(HERE, Sort::ZERO);
    (0..depth).fold(type0.clone(), |value, _| {
        Term::bind(HERE, "z", type0.clone(), value, type0.clone())
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

/// The δ path a long list actually travels, as a context and two builtins.
///
/// `Vec` under `Nat`, `spun : Nat → Vec Nat` building a list of the length it is
/// given, and `drained : Vec Nat → Nat` reading one back. Between them they are
/// the two walks prompt 165b took off the nesting metric: `spun`'s answer is
/// realized from a [`Datum`] into a value, and `drained`'s argument is read from
/// a value into a [`Datum`].
///
/// **The length has to arrive as a number rather than as a term**, which is why
/// `spun` exists at all. Writing the list out — `Vec.Cons Nat 0 (Vec.Cons …)` —
/// would make it a *term* six hundred deep, and a term's depth is exactly what
/// the nesting metric is derived to bound. What the staff adapter does, and what
/// this fixture does, is build a deep value out of a shallow term.
///
/// # Panics
///
/// If the declarations or the registry are refused, which would be a defect in
/// this crate rather than a property of any law.
fn list_host() -> Cx {
    let (cx, _) = crate::family_laws::nat_context();
    let group = musa_calculus::declare(&cx, &crate::family_laws::vec()).expect("Vec is a declaration");
    let cx = cx.declaring(&group);
    let nat = crate::family_laws::core_constant(&cx, "Nat");
    let vec_nat = Term::app(HERE, crate::family_laws::core_constant(&cx, "Vec"), nat.clone());
    let arrow = |domain: Term, codomain: Term| Term::pi(HERE, "_", domain, codomain);
    let registry = Registry::new(
        Vec::new(),
        vec![
            Builtin::new(
                "spun",
                arrow(nat.clone(), vec_nat.clone()),
                Family::Delta,
                |arguments| match *arguments {
                    [Datum::Count { count, .. }] => Some(
                        (0..count)
                            .fold(
                                Datum::Case {
                                    constructor: "Vec.Nil".into(),
                                    fields: Vec::new(),
                                },
                                |rest, at| Datum::Case {
                                    constructor: "Vec.Cons".into(),
                                    fields: vec![
                                        Datum::Count {
                                            family: "Nat".into(),
                                            count: at,
                                        },
                                        rest,
                                    ],
                                },
                            )
                            .into(),
                    ),
                    _ => None,
                },
            ),
            Builtin::new("drained", arrow(vec_nat, nat), Family::Delta, |arguments| {
                let [ref subject] = *arguments else { return None };
                let mut count: u64 = 0;
                let mut here = subject;
                while let Datum::Case {
                    ref constructor,
                    ref fields,
                } = *here
                {
                    let [_, ref rest] = *fields.as_slice() else { break };
                    if **constructor != *"Vec.Cons" {
                        break;
                    }
                    count = count.saturating_add(1);
                    here = rest;
                }
                Some(
                    Datum::Count {
                        family: "Nat".into(),
                        count,
                    }
                    .into(),
                )
            }),
        ],
    )
    .expect("the two rules register");
    cx.with_externs(Arc::new(registry))
}

/// `drained (spun n)` as a raw term: shallow, whatever `n` is.
fn round_trip(count: u64) -> Raw {
    crate::family_laws::apply(
        crate::family_laws::var("drained"),
        [crate::family_laws::apply(
            crate::family_laws::var("spun"),
            [Raw::numeral(HERE, "Nat", count)],
        )],
    )
}

/// §4.1, the metric counts descent through a term and through a value, and not
/// through *data*.
///
/// A δ-rule handed a list two thousand long builds it and reads it back at a
/// nesting limit of [`Budget::NESTING`], which is 320. Before prompt 165b the
/// two walks charged one nesting level per level of the list, so this refused at
/// the three hundred and twenty-first element — the size of one argument charged
/// against a stack-safety guard.
///
/// Stated on [`on_the_smallest_host`], because the other half of the change is
/// that the walks stopped using host frames as well as stopped charging for
/// them. Removing the charge alone would have turned a refusal into an abort,
/// and an abort is the one outcome §4.1 does not allow. A regression here shows
/// up as `SIGABRT` rather than as a failed assertion, which is the honest report.
#[test]
fn data_far_past_the_nesting_limit_is_built_and_read() {
    on_the_smallest_host(|| {
        let cx = list_host();
        let nat = crate::family_laws::core_constant(&cx, "Nat");
        let checked = musa_calculus::check(&cx, &nat, &round_trip(2_000)).expect("the round trip checks");
        let normal = musa_calculus::normalize(&cx, &nat, &checked).expect("two thousand is not two thousand levels");
        assert_eq!(
            musa_calculus::canonical(&cx, &normal),
            Some(Datum::Count {
                family: "Nat".into(),
                count: 2_000,
            }),
            "what was built is what was read"
        );
    });
}

/// §4.1's other half: what replaced the charge is the step budget, one step a
/// node, which is what the frame-based reading charged one level a node for.
///
/// Counted by hand rather than asserted from a recorded total. Each extra
/// element of the list is one more node in `spun`'s answer and one more in
/// `drained`'s reading, so the cost of the round trip is affine in the length
/// and the slope is the per-element constant. Two differences pin both: they
/// must be equal to each other — the walk charges per node and not per anything
/// else — and their common value is the number of charges one element costs.
///
/// The slope is asserted exactly rather than bounded, because the point of the
/// law is that the count did not quietly change when the metric did. If a later
/// change to the walk moves it, that is a cost-table question for §4 and not a
/// number to re-fit here.
#[test]
fn the_data_walk_charges_one_step_a_node() {
    let cx = list_host();
    let nat = crate::family_laws::core_constant(&cx, "Nat");
    let spend = |count: u64| {
        let checked = musa_calculus::check(&cx, &nat, &round_trip(count)).expect("the round trip checks");
        let (_, spend) = musa_calculus::normalize_metered(&cx, &nat, &checked).expect("the round trip normalizes");
        spend.steps
    };
    let (ten, twenty, thirty) = (spend(10), spend(20), spend(30));
    let first = twenty.saturating_sub(ten);
    let second = thirty.saturating_sub(twenty);
    assert_eq!(first, second, "the walk charges per node: {first} then {second}");
    assert_eq!(
        first.checked_div(10),
        Some(PER_ELEMENT),
        "one element of the list costs {first} charges over ten elements"
    );
}

/// What one element of the list above costs, in steps.
///
/// Twelve, measured. A handful rather than one because an element is more than
/// a node: `realize` charges the `Vec.Cons` node and its `Nat` field, evaluates
/// each of the two declared field types — `A` is one node and `Vec A` is three
/// — and applies the constructor to each field, and `canonical` charges the two
/// nodes again on the way back. What the law claims is not the twelve but that
/// it is the *same* twelve for every element, which is what "one charge a node"
/// means when the node is read once and written once.
const PER_ELEMENT: u64 = 12;
