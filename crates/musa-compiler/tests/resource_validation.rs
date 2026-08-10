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
        "fn keep(index: nat, accumulator: nat) -> nat = accumulator; \
         let value: nat = nat_fold(0, keep, 50000);",
    );
    assert!(!accepted.has_errors(), "{:?}", accepted.diagnostics());

    let rejected = compile_declarations(
        "fn keep(index: nat, accumulator: nat) -> nat = accumulator; \
         let value: nat = nat_fold(0, keep, 200000);",
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
    let compilation = compile_declarations("let values: list[nat] = range(100001);");
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
    let compilation = compile_declarations("let values: list[ratio] = repeat(1/2, 65536);");
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
        let _ = write!(declarations, "let {}: list[nat] = range(0);", alphabetic_name(index));
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
    let missing = compile_declarations("fn choose(value: option[nat]) -> nat = match value { none -> 0 };");
    assert!(
        missing
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code == Code::NonExhaustiveMatch)
    );

    let unreachable = compile_declarations("fn choose(value: bool) -> nat = match value { _ -> 0, true -> 1 };");
    assert!(
        unreachable
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code == Code::UnreachablePattern)
    );
}
