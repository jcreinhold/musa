use std::fmt::Write as _;

use musa_compiler::{Code, CompileOptions, SourceDocument, compile};

fn compile_declarations(declarations: &str) -> musa_compiler::Compilation {
    compile(
        &SourceDocument::new(
            format!("piece \"Resource validation\" {{ {declarations} score {{ part p {{ voice v {{ c4/1 }} }} }} }}"),
            "resource-validation.musa",
        ),
        &CompileOptions::default(),
    )
}

fn alphabetic_name(mut index: usize) -> String {
    let mut suffix = String::new();
    loop {
        let letter = u8::try_from(index % 26).unwrap_or_default();
        suffix.push(char::from(b'a'.saturating_add(letter)));
        index /= 26;
        if index == 0 {
            break;
        }
    }
    format!("value{suffix}")
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
fn monomorphization_has_its_own_finite_limit() {
    let mut declarations = String::new();
    for index in 0..2049 {
        let _ = write!(declarations, "let {}: List<Nat> = range(0);", alphabetic_name(index));
    }
    let compilation = compile_declarations(&declarations);
    let diagnostic = compilation
        .diagnostics()
        .iter()
        .find(|diagnostic| diagnostic.code == Code::ResourceLimit);
    assert!(
        matches!(diagnostic, Some(found) if found.labels.iter().any(|label| {
            label.text.contains("monomorphized prelude instances") && label.text.contains("limit 2048")
        })),
        "{:?}",
        compilation.diagnostics()
    );
}

#[test]
fn matches_reject_missing_and_unreachable_cases_separately() {
    let missing = compile_declarations("fn choose(value: Option<Nat>) -> Nat { match value { None -> 0 } }");
    assert!(
        missing
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code == Code::NonExhaustiveMatch)
    );

    let unreachable = compile_declarations("fn choose(value: Bool) -> Nat { match value { _ -> 0, true -> 1 } }");
    assert!(
        unreachable
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code == Code::UnreachablePattern)
    );
}

/// A `match` over a sum has to answer for both injections, and the refusal
/// says which one is missing and at what type. A sum whose second case can be
/// forgotten silently is a second error channel wearing a value's clothes —
/// which is the whole reason `Result` is a sum here rather than a convention.
#[test]
fn a_match_over_a_sum_must_answer_for_both_injections() {
    for (declarations, missing) in [
        (
            "fn taken(outcome: Result<Nat, Text>) -> Nat { match outcome { Ok(found) -> found } }",
            "Err(reason)",
        ),
        (
            "fn taken(outcome: Result<Nat, Text>) -> Nat { match outcome { Err(said) -> 0 } }",
            "Ok(value)",
        ),
    ] {
        let compilation = compile_declarations(declarations);
        let diagnostic = compilation
            .diagnostics()
            .iter()
            .find(|diagnostic| diagnostic.code == Code::NonExhaustiveMatch);
        assert!(
            diagnostic.is_some(),
            "accepted a partial match: {:?}",
            compilation.diagnostics()
        );
        let Some(found) = diagnostic else { continue };
        assert!(
            found.labels.iter().any(|label| label.text.contains(missing)),
            "the refusal must name `{missing}`: {:?}",
            found.labels
        );
        assert!(
            found
                .note
                .as_ref()
                .is_some_and(|note| note.contains("Result<Nat, Text>")),
            "and say at what type: {:?}",
            found.note
        );
    }

    let both = compile_declarations(
        "fn taken(outcome: Result<Nat, Text>) -> Nat { match outcome { Ok(found) -> found, Err(said) -> 0 } }",
    );
    assert!(!both.has_errors(), "{:?}", both.diagnostics());
}
