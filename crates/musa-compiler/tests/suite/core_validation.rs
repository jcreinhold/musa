use musa_compiler::{Code, CompileOptions, ImportSources, SourceDocument, compile};

fn diagnostics(declarations: &str) -> Vec<Code> {
    let source = SourceDocument::new(
        format!("piece \"Core validation\" {{ {declarations} score {{ part p {{ voice v {{ c4/1 }} }} }} }}"),
        "core-validation.musa",
    );
    compile(&source, &CompileOptions::default())
        .diagnostics()
        .iter()
        .map(|diagnostic| diagnostic.code)
        .collect()
}

#[test]
fn each_static_failure_has_a_stable_diagnostic_code() {
    for (declarations, expected) in [
        // `ConversionMismatch`, not `TypeMismatch`: the two types are compared
        // by normalizing both and deciding convertibility, so the refusal names
        // that question rather than a shape mismatch in a unifier.
        // `diagnose.rs` introduced the code as distinct from `TypeMismatch`,
        // "which the rank-1 checker raises".
        ("let value: Nat = true;", Code::ConversionMismatch),
        ("let value: unknown = 1;", Code::UnknownName),
        ("let value: Nat = absent;", Code::UnknownName),
        ("let value: Ratio = 999999999999999999999/1;", Code::OutOfRange),
        ("fn one(x: Nat) -> Nat { x } let value: Nat = one();", Code::WrongArity),
        ("let value: Nat = 1; let value: Nat = 2;", Code::DuplicateName),
        (
            "fn left(x: Nat) -> Nat { right(x) } fn right(x: Nat) -> Nat { left(x) }",
            Code::DependencyCycle,
        ),
        (
            "let value: EventTrack<WrittenTime> = music { meter 3/4; c4/1 };",
            Code::Misplaced,
        ),
    ] {
        let actual = diagnostics(declarations);
        assert!(
            actual.contains(&expected),
            "expected {expected:?} for {declarations}, got {actual:?}"
        );
    }
}

#[test]
fn a_parameter_cannot_be_bound_twice() {
    let actual = diagnostics("fn choose(value: Nat, value: Nat) -> Nat { value }");
    assert!(actual.contains(&Code::DuplicateName), "{actual:?}");
}

/// An argument is not labelled at all, and the refusal says so.
///
/// This law used to check that a label naming no parameter, or naming one
/// twice, was refused as `WrongArity`. There are no labels to get wrong now:
/// prompt 130's rewrite of `01-surface.md` left the name at the declaration —
/// §1.2 names fields where the name does work — and every argument passes by
/// position. Checking a label at a use site would need the declaration in hand,
/// and a label that agrees with the position it is already in earns nothing for
/// the second way to pass an argument that it costs.
///
/// So the law changes from "a label must be right" to "a label is not a thing",
/// and it is still worth stating: the two programs below are what an author
/// migrating from the old language writes first, and they get told the feature
/// is gone rather than a mismatch further downstream.
#[test]
fn an_argument_cannot_be_labelled() {
    for written in [
        "fn one(x: Nat) -> Nat { x } let value: Nat = one(y: 1);",
        "fn one(x: Nat) -> Nat { x } let value: Nat = one(x: 1, x: 2);",
    ] {
        let said = diagnostics(written);
        assert!(said.contains(&Code::UnsupportedLanguageStage), "{said:?}");
    }
}

#[test]
fn an_imported_core_error_is_located_at_the_local_use() {
    let mut imports = ImportSources::default();
    imports.insert("broken.musa", "library { let answer: Nat = false; }");
    let source_text = "piece \"Imported error\" { import \"broken.musa\"; score { part p { voice v { c4/1 } } } }";
    let compilation = compile(
        &SourceDocument::new(source_text, "piece.musa"),
        &CompileOptions {
            imports,
            ..CompileOptions::default()
        },
    );
    let diagnostic = compilation
        .diagnostics()
        .iter()
        .find(|diagnostic| diagnostic.code == Code::Import);
    assert!(matches!(diagnostic, Some(found) if found.message.contains("broken.musa")));
    assert!(matches!(
        diagnostic.and_then(|found| found.labels.iter().find(|label| label.primary)),
        Some(label) if &source_text[label.span.start as usize..label.span.end as usize] == "import \"broken.musa\";"
    ));
}
