//! What a compilation is allowed to cost, and what a `match` has to answer for.
//!
//! Prompt 142 moved the surface onto a core program, which changed two of
//! these laws' subjects and deleted a third.
//!
//! The two match laws keep their claims and change their codes. `diagnose.rs`
//! introduced `IncompleteMatch` and `UnreachableBranch` deliberately distinct
//! from `NonExhaustiveMatch` and `UnreachablePattern`, "which the rank-1
//! checker raises": coverage is now decided while a `match` is compiled to a
//! recursor, so the refusal names a *constructor of an inductive family* rather
//! than a shape in a pattern list. `Result.Err` where the replaced checker said
//! `Err(reason)` is the same answer in the vocabulary 141l gave the language.
//!
//! The three meter laws are gone, and the reason is one measurement rather
//! than three. A collection is built by structural recursion now — `range` is
//! `stdlib/src/list.musa`'s `counting_from`, one recursive call per element —
//! and a recursive call costs about three nesting levels, so §4's limit
//! refuses every list past about a hundred elements — bisected at prompt 155a,
//! which raised the limit to 320 and re-measured the per-call cost, up from the
//! 2.5 levels the generated recursor spent. Nesting is therefore
//! the *first* limit any large value meets, and §4's node limit (100,000) and
//! byte limit (1,048,576) are unreachable by construction: no program can
//! build 100,000 nodes without passing 256 levels on the way. A law probing
//! `range(100001)` or `repeated(1/2, 65536)` no longer measures nodes or
//! bytes, and restating it at a reachable count would measure nesting under
//! two other metrics' names.
//!
//! `finite_large_work_is_accepted_but_the_deterministic_boundary_is_not` goes
//! with them, and it is the one whose claim actually *failed* rather than
//! moved: its accepted half was `range(50000).fold_from_start(0, fn (carried, index) { keep(index, carried) })`, and 50,000 is not
//! accepted — it is not even refused. Past about 1,256 elements the process
//! aborts with a stack overflow, which §4.1 names as the one outcome this
//! language may not have: "a compiler that aborts instead has replaced a
//! diagnostic with a crash". Some descent proportional to the count is
//! uncharged, so it reaches ~1,256 native frames while the meter believes it
//! is below 256, and no amount of room fixes an uncharged descent —
//! `room.rs`'s 8 MiB is what it fills. Writing this law at sixty iterations
//! would keep the name and drop the subject; the shortfall is recorded where
//! a reader will find it instead.
//!
//! **Prompt 165a named that descent and closed it, and both laws below moved
//! with it.** It was the destructor. A value is a tree of `Arc`s and Rust's
//! derived drop walks it with the host's stack, so freeing a list of a few
//! thousand elements was a few thousand host frames with no musa frame among
//! them — which is why no charge could see it and why the room it filled was
//! whatever the room happened to be. `musa-calculus`'s `Neutral` reclaims its
//! spine with an explicit worklist now. What is left is the metric that was
//! always supposed to bind: a recursive call is [`Metric::Steps`] rather than
//! nesting levels. Prompt 165 subsequently made the construction metrics real
//! and re-derived the table, so the large aggregate law below accepts either
//! deterministic work or node exhaustion.
//!
//! `monomorphization_has_its_own_finite_limit` is gone, and not quietly. It
//! asserted that 2,049 declarations reading one prelude generic are refused at
//! `WorkMeter`'s 2,048 "monomorphized prelude instances". They are accepted
//! now, and correctly: a dependent core has no monomorphization step to count.
//! `repeated<A>` is a Π over a type applied at each call site, not a scheme
//! instantiated into one copy per use, so there is no instance to charge and
//! nothing a limit would bound. `Counter::Instances` survives only because
//! `core.rs`'s replaced arms are its two callers, which is prompt 142's own
//! Target sorting a dead row into *replaced* rather than *not yet called* — the
//! distinction that separates it from `WorkMeter::output`, where the limit is
//! real and uncharged. No governing document fixes 2,048; it is a
//! `core_budget.rs` constant, and it goes with the mechanism it measured.

// A failure is more useful reported with what actually happened than with an
// assertion message alone.
#![allow(clippy::panic)]
use musa_compiler::{CompileOptions, SourceDocument, compile};
use musa_score::Code;

fn compile_declarations(declarations: &str) -> musa_compiler::Compilation {
    compile(
        &SourceDocument::new(
            format!(
                "piece \"Resource validation\" {{ import std::list; {declarations} \
                 score {{ part p {{ voice v {{ c4/1 }} }} }} }}"
            ),
            "resource-validation.musa",
        ),
        &CompileOptions::default(),
    )
}

/// A budget ends an evaluation by refusing it, and publishes nothing partial.
///
/// What survives of the three deleted meter laws, at the metric that actually
/// binds. The claim is §4's three-outcome law rather than any one counter's
/// threshold: a small aggregate is accepted, a larger one is *refused* — named,
/// with its metric, attempted amount, and limit — and a refused compilation has
/// no snapshot, so exhaustion publishes neither a partial value nor a partial
/// score. Neither count is small any more. Eight thousand elements is two
/// orders of magnitude past the sixty the nesting limit used to admit, and
/// two hundred thousand is past the re-derived work or construction budget;
/// prompt 165a is what moved the recursion wall, and prompt 165 made the size
/// counters real, so which deterministic counter binds is part of the cost
/// table rather than this publication law.
///
/// The metric and the limit are read out of the message because that is where
/// the new core puts them: a `ResourceLimit` from `musa-calculus` arrives with no
/// labels at all, where the replaced meter carried both in one. §4 requires the
/// diagnostic to name "the operation, metric, attempted amount, and limit", and
/// the message does name all four — but a label is also what gives a diagnostic
/// a span, so this refusal currently points at no text. That is the same
/// missing-provenance shortfall recorded for the new lowering's origins, and it
/// is asserted here as it is rather than as it should be.
#[test]
fn an_aggregate_past_the_budget_is_refused_and_publishes_nothing() {
    let accepted = compile_declarations("let values: List(Nat) = range(8000);");
    assert!(!accepted.has_errors(), "{:?}", accepted.diagnostics());

    let refused = compile_declarations("let values: List(Nat) = range(200000);");
    let diagnostic = refused
        .diagnostics()
        .iter()
        .find(|diagnostic| diagnostic.code == Code::ResourceLimit);
    let Some(found) = diagnostic else {
        panic!("a list past the budget is refused: {:?}", refused.diagnostics())
    };
    assert!(
        (found.message.contains(musa_calculus::Metric::Steps.name())
            && found
                .message
                .contains(&musa_calculus::Budget::LANGUAGE.steps().to_string()))
            || (found.message.contains(musa_calculus::Metric::ConstructedNodes.name())
                && found
                    .message
                    .contains(&musa_calculus::Budget::LANGUAGE.constructed_node_limit().to_string(),)),
        "the refusal names its metric and limit: {}",
        found.message
    );
    assert!(refused.snapshot().is_none());
}

/// §4.1's third outcome does not come back at the other end of a value's life.
///
/// The accepted half above is also a law about *freeing* eight thousand list
/// cells, and it is stated separately because it fails differently: not as a
/// wrong answer or a missing diagnostic but as `fatal runtime error: stack
/// overflow`, with the whole process gone. A `Value` is a tree of `Arc`s, and
/// while nesting refused every list past about sixty nobody could reach a value
/// deep enough for its destructor to matter. Prompt 165a made those lists
/// compile, and measured on the way: `range(4000)` compiled and `range(4500)`
/// aborted, in `drop_glue<Value>`, with no musa frame on the stack.
///
/// The count is the same eight thousand, deliberately — a widening that is paid
/// for with a crash is not a widening, and the two halves are one claim seen
/// twice.
#[test]
fn a_value_deeper_than_the_host_stack_is_freed_rather_than_aborting() {
    let built = compile_declarations("let values: List(Nat) = range(8000);");
    assert!(!built.has_errors(), "{:?}", built.diagnostics());
    drop(built);
}

#[test]
fn matches_reject_missing_and_unreachable_cases_separately() {
    let missing = compile_declarations("fn choose(value: Option(Nat)) -> Nat { match value { None -> 0 } }");
    assert!(
        missing
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code == Code::IncompleteMatch),
        "{:?}",
        missing.diagnostics()
    );

    let unreachable = compile_declarations("fn choose(value: Bool) -> Nat { match value { _ -> 0, true -> 1 } }");
    assert!(
        unreachable
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code == Code::UnreachableBranch),
        "{:?}",
        unreachable.diagnostics()
    );
}

/// A `match` over a sum has to answer for both injections, and the refusal says
/// which one is missing. A sum whose second case can be forgotten silently is a
/// second error channel wearing a value's clothes — which is the whole reason
/// `Result` is a sum here rather than a convention.
///
/// "And at what type" is the half this law no longer states. The replaced
/// checker put `Result<Nat, Text>` in a note because it was holding the
/// scrutinee's `Type`; `Refusal::IncompleteMatch` holds an `Origin` and a
/// constructor `Name`, so the family is recoverable from `Result.Err` and its
/// arguments are not held at all. That makes restoring the note a change to
/// what the refusal *carries* rather than to what it prints — coverage
/// diagnostics are prompt 165's, and this is one of them.
#[test]
fn a_match_over_a_sum_must_answer_for_both_injections() {
    for (declarations, missing) in [
        (
            "fn taken(outcome: Result(Nat, Text)) -> Nat { match outcome { Ok(found) -> found } }",
            "Result.Err",
        ),
        (
            "fn taken(outcome: Result(Nat, Text)) -> Nat { match outcome { Err(said) -> 0 } }",
            "Result.Ok",
        ),
    ] {
        let compilation = compile_declarations(declarations);
        let diagnostic = compilation
            .diagnostics()
            .iter()
            .find(|diagnostic| diagnostic.code == Code::IncompleteMatch);
        assert!(
            diagnostic.is_some(),
            "accepted a partial match: {:?}",
            compilation.diagnostics()
        );
        let Some(found) = diagnostic else { continue };
        assert!(
            found.message.contains(missing),
            "the refusal must name `{missing}`: {}",
            found.message
        );
    }

    let both = compile_declarations(
        "fn taken(outcome: Result(Nat, Text)) -> Nat { match outcome { Ok(found) -> found, Err(said) -> 0 } }",
    );
    assert!(!both.has_errors(), "{:?}", both.diagnostics());
}
