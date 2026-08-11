//! Native tests for the wasm shell's core. The boundary carries
//! plain data, so the interesting behavior is all testable without a browser;
//! DOM behavior is Playwright's job.
//!
//! Test helpers use `expect()` on corpus fixtures: a failure is a bug in the
//! test or the pipeline, not a case to handle.
#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]

use musa_compiler::{CompileOptions, SourceDocument, compile};
use musa_render::{NotationOptions, NotationTarget, render_notation};
use musa_wasm::{typeset_impl, validate_impl};

/// A fixture from the corpus, read relative to the workspace root.
fn example(name: &str) -> String {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("examples")
        .join(name);
    std::fs::read_to_string(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

#[test]
fn canon_typesets_to_provenance_mei() {
    let result = typeset_impl(&example("canon.musa"));
    let mei = result.mei.as_deref().expect("canon must typeset");
    assert!(mei.contains("<mei"), "expected MEI output");
    assert!(mei.contains("event-"), "MEI must carry the xml:id contract");
    assert!(
        result.diagnostics.iter().all(|d| d.severity != "error"),
        "canon must compile without errors: {:?}",
        result.diagnostics
    );
}

#[test]
fn broken_source_yields_diagnostics_and_no_mei() {
    let result = typeset_impl(&example("broken/bar-too-long.musa"));
    assert_eq!(result.mei, None);
    let error = result
        .diagnostics
        .iter()
        .find(|d| d.severity == "error")
        .expect("expected an error diagnostic");
    assert!(!error.message.is_empty());
    assert!(!error.code.is_empty());
    assert!(
        error.labels.iter().any(|label| label.primary),
        "the error must point somewhere: {error:?}"
    );
}

#[test]
fn material_document_is_not_an_error() {
    let source = "library {\n    motif rise(root: Pitch = c5) {\n        root/4\n        d5/4\n    }\n}\n";
    let result = typeset_impl(source);
    assert_eq!(result.mei, None, "a library declares; it does not sound");
    assert!(
        result.diagnostics.iter().all(|d| d.severity != "error"),
        "material is not a failure: {:?}",
        result.diagnostics
    );
}

#[test]
fn the_shell_adds_nothing_to_the_pipeline() {
    let source = example("canon.musa");
    let compilation = compile(
        &SourceDocument::new(&source, "snippet.musa"),
        &CompileOptions::default(),
    );
    let snapshot = compilation.snapshot().expect("canon must compile");
    let direct = render_notation(snapshot, NotationTarget::Mei, &NotationOptions::default())
        .expect("direct render must succeed");
    assert_eq!(typeset_impl(&source).mei.as_deref(), Some(direct.text()));
}

#[test]
fn validate_matches_typeset_diagnostics() {
    let source = example("broken/bar-too-long.musa");
    assert_eq!(validate_impl(&source), typeset_impl(&source).diagnostics);
}
