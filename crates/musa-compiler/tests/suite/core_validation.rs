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
        ("let value: Nat = true;", Code::TypeMismatch),
        ("let value: unknown = 1;", Code::UnknownName),
        ("let value: Nat = absent;", Code::UnknownName),
        ("let value: Ratio = 999999999999999999999/1;", Code::OutOfRange),
        ("fn one(x: Nat) -> Nat { x } let value: Nat = one();", Code::WrongArity),
        ("let value: Nat = 1; let value: Nat = 2;", Code::DuplicateName),
        (
            "fn left(x: Nat) -> Nat { right(x) } fn right(x: Nat) -> Nat { left(x) }",
            Code::DependencyCycle,
        ),
        ("let value: Music = music { meter 3/4; c4/1 };", Code::Misplaced),
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

#[test]
fn named_arguments_must_name_a_parameter_once() {
    let unknown = diagnostics("fn one(x: Nat) -> Nat { x } let value: Nat = one(y: 1);");
    assert!(unknown.contains(&Code::WrongArity), "{unknown:?}");
    let repeated = diagnostics("fn one(x: Nat) -> Nat { x } let value: Nat = one(x: 1, x: 2);");
    assert!(repeated.contains(&Code::WrongArity), "{repeated:?}");
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
