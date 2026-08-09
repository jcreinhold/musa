use musa_compiler::{Code, CompileOptions, SourceDocument, compile};

#[test]
fn expressions_are_valid_syntax_but_stop_at_the_owned_stage() {
    let source = SourceDocument::new(
        "piece \"staged\" { let answer: nat = 42; score { part p { voice v { c4/1 } } } }",
        "staged.musa",
    );
    let compilation = compile(&source, &CompileOptions::default());
    assert!(compilation.has_errors());
    assert!(compilation.snapshot().is_none());
    assert_eq!(compilation.diagnostics().len(), 1);
    assert_eq!(
        compilation.diagnostics().first().map(|diagnostic| diagnostic.code),
        Some(Code::UnsupportedLanguageStage)
    );
}
