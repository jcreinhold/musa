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

use musa_compiler::{Code, CompileOptions, SourceDocument, compile};

fn compile_declarations(declarations: &str) -> musa_compiler::Compilation {
    compile(
        &SourceDocument::new(
            format!(
                "piece \"Resource validation\" {{ import std::nat; import std::list; {declarations} \
                 score {{ part p {{ voice v {{ c4/1 }} }} }} }}"
            ),
            "resource-validation.musa",
        ),
        &CompileOptions::default(),
    )
}

#[test]
fn finite_large_work_is_accepted_but_the_deterministic_boundary_is_not() {
    let accepted = compile_declarations(
        "fn keep(index: Nat, accumulator: Nat) -> Nat { accumulator } \
         let value: Nat = nat_fold(0, keep, 50000);",
    );
    assert!(!accepted.has_errors(), "{:?}", accepted.diagnostics());

    let rejected = compile_declarations(
        "fn keep(index: Nat, accumulator: Nat) -> Nat { accumulator } \
         let value: Nat = nat_fold(0, keep, 200000);",
    );
    let diagnostic = rejected
        .diagnostics()
        .iter()
        .find(|diagnostic| diagnostic.code == Code::ResourceLimit);
    assert!(matches!(diagnostic, Some(found) if found.message.contains("nat_fold")));
    assert!(rejected.snapshot().is_none());
}

#[test]
fn aggregate_allocation_is_rejected_before_it_is_built() {
    let compilation = compile_declarations("let values: List<Nat> = range(100001);");
    let diagnostic = compilation
        .diagnostics()
        .iter()
        .find(|diagnostic| diagnostic.code == Code::ResourceLimit);
    assert!(matches!(diagnostic, Some(found) if found.message.contains("range")));
    assert!(matches!(
        diagnostic.and_then(|found| found.labels.first()),
        Some(label) if label.text.contains("constructed value nodes") && label.text.contains("limit 100000")
    ));
}

#[test]
fn logical_value_bytes_have_a_limit_distinct_from_node_count() {
    let compilation = compile_declarations("let values: List<Ratio> = repeated(1/2, 65536);");
    let diagnostic = compilation
        .diagnostics()
        .iter()
        .find(|diagnostic| diagnostic.code == Code::ResourceLimit);
    assert!(matches!(
        diagnostic.and_then(|found| found.labels.first()),
        Some(label) if label.text.contains("constructed value bytes") && label.text.contains("limit 1048576")
    ));
}

#[test]
fn matches_reject_missing_and_unreachable_cases_separately() {
    let missing = compile_declarations("fn choose(value: Option<Nat>) -> Nat { match value { None -> 0 } }");
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
/// diagnostics are prompt 144's, and this is one of them.
#[test]
fn a_match_over_a_sum_must_answer_for_both_injections() {
    for (declarations, missing) in [
        (
            "fn taken(outcome: Result<Nat, Text>) -> Nat { match outcome { Ok(found) -> found } }",
            "Result.Err",
        ),
        (
            "fn taken(outcome: Result<Nat, Text>) -> Nat { match outcome { Err(said) -> 0 } }",
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
        "fn taken(outcome: Result<Nat, Text>) -> Nat { match outcome { Ok(found) -> found, Err(said) -> 0 } }",
    );
    assert!(!both.has_errors(), "{:?}", both.diagnostics());
}
