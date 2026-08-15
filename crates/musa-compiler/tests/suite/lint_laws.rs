//! The lint pass's laws (`docs/rules/style-guide.md`).
//!
//! Each rule fires on a minimal fixture and stays silent where the guide
//! says the spelling is honest; each waiver works where it is written and
//! nowhere else; each offered fix, applied, leaves the piece compiling and
//! the rule silent. The last test is the one that keeps the rules honest:
//! every valid fixture in `examples/` compiles with no lint warning at all —
//! the examples are the executable form of the style the guide describes.

// Test helpers use expect() on statically-valid inputs: a failure is a bug in
// the test itself, and panicking is the correct behavior there.
#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]

use musa_compiler::{Code, CompileOptions, Diagnostic, Severity, SourceDocument, compile};

/// A piece with the parts every lint fixture needs, with `body` for the
/// voice's items and `extra` for declarations ahead of the score.
fn piece(extra: &str, body: &str) -> String {
    format!(
        "piece \"Lint\" {{
    tempo 1/4 = 96;
    meter 4/4;
    key c major;

{extra}

    score {{
        part piano {{
            voice right {{
                bar {{ c4/4 d4/4 e4/4 f4/4 }}
{body}
            }}
        }}
    }}
}}
"
    )
}

fn warnings_of(source: &str) -> Vec<Diagnostic> {
    let compilation = compile(&SourceDocument::new(source, "lint.musa"), &CompileOptions::default());
    let errors: Vec<String> = compilation
        .diagnostics()
        .iter()
        .filter(|diagnostic| diagnostic.severity == Severity::Error)
        .map(|diagnostic| diagnostic.message.clone())
        .collect();
    assert!(errors.is_empty(), "fixture must compile: {errors:?}");
    compilation
        .diagnostics()
        .iter()
        .filter(|diagnostic| diagnostic.severity == Severity::Warning)
        .cloned()
        .collect()
}

/// The warnings with one code, from one source.
fn coded(source: &str, code: Code) -> Vec<Diagnostic> {
    warnings_of(source)
        .into_iter()
        .filter(|diagnostic| diagnostic.code == code)
        .collect()
}

#[test]
fn an_unused_motif_is_named_a_rumour() {
    let source = piece("motif answer() {\n    g4/4 a4/4 e4/4 f4/4\n}", "");
    let lints = coded(&source, Code::UnusedMaterial);
    let [lint] = lints.as_slice() else {
        panic!("expected one unused-material warning: {lints:?}");
    };
    assert!(lint.message.contains("never used"), "{}", lint.message);
    // The fix deletes the declaration; applying it leaves a compiling piece
    // and a silent rule.
    let fix = lint.fixes.first().expect("a fix");
    let mut fixed = source;
    for edit in &fix.edits {
        fixed.replace_range(edit.span.start as usize..edit.span.end as usize, &edit.replacement);
    }
    assert!(coded(&fixed, Code::UnusedMaterial).is_empty(), "fix left: {fixed}");
}

#[test]
fn a_used_motif_is_silent() {
    let source = piece(
        "motif answer() {\n    g4/4 a4/4 e4/4 f4/4\n}",
        "                use answer();",
    );
    assert!(coded(&source, Code::UnusedMaterial).is_empty());
}

#[test]
fn a_named_bar_is_an_address_not_a_rumour() {
    // A named bar plays where it stands; its name is for edit sites and
    // provenance, not for `use`. Unused ones are silent by design.
    let source = piece(
        "",
        "                bar head { g4/4 a4/4 e4/4 f4/4 }\n                bar { c4/4 c4/4 c4/4 c4/4 }",
    );
    assert!(
        coded(&source, Code::UnusedMaterial).is_empty(),
        "{:?}",
        warnings_of(&source)
    );
}

#[test]
fn an_unassigned_patch_is_dead_wiring() {
    let source = piece(
        "studio {\n    patch pad {\n        oscillator(sine) |> output;\n    }\n}",
        "",
    );
    let lints = coded(&source, Code::UnassignedPatch);
    let [lint] = lints.as_slice() else {
        panic!("expected one unassigned-patch warning: {lints:?}");
    };
    assert!(lint.message.contains("realizes no part"), "{}", lint.message);
}

#[test]
fn an_assigned_patch_is_silent() {
    let source = piece(
        "studio {\n    patch pad {\n        oscillator(sine) |> output;\n    }\n\n    assign piano -> pad;\n}",
        "",
    );
    assert!(
        coded(&source, Code::UnassignedPatch).is_empty(),
        "{:?}",
        warnings_of(&source)
    );
}

#[test]
fn an_aimless_ramp_is_an_error_not_a_lint() {
    // Style guide §3's dishonest middle — a metronome'd ramp with no
    // arrival — is the compiler's own error, which is why the lint pass has
    // no rule for it: lints warn on what compiles.
    let source = piece("", "                tempo 1/4 = 72 over 2/1;");
    let compilation = compile(&SourceDocument::new(source, "lint.musa"), &CompileOptions::default());
    assert!(
        compilation
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.severity == Severity::Error && diagnostic.message.contains("goes nowhere")),
        "{:?}",
        compilation.diagnostics()
    );
}

#[test]
fn a_worded_ramp_and_a_named_arrival_are_honest() {
    // The word leaves the speed to the player; the `to` names the arrival.
    let worded = piece("", "                tempo \"rit.\" over 1/1;");
    assert!(warnings_of(&worded).is_empty(), "{:?}", warnings_of(&worded));
    let named = piece("", "                tempo 1/4 = 72 to 48 over 2/1 \"rit.\";");
    assert!(warnings_of(&named).is_empty(), "{:?}", warnings_of(&named));
}

#[test]
fn a_marking_that_says_what_is_already_true_is_redundant() {
    // The header's own tempo, meter, and key, said again in the voice.
    let source = piece(
        "",
        "                tempo 1/4 = 96;\n                meter 4/4;\n                key c major;",
    );
    let lints = coded(&source, Code::RedundantMarking);
    assert_eq!(lints.len(), 3, "{lints:?}");
    // And one said twice in the voice itself.
    let twice = piece(
        "",
        "                tempo 1/4 = 120;\n                bar { g4/4 g4/4 g4/4 g4/4 }\n                tempo 1/4 = 120;",
    );
    assert_eq!(
        coded(&twice, Code::RedundantMarking).len(),
        1,
        "{:?}",
        warnings_of(&twice)
    );
}

#[test]
fn a_change_is_not_redundant_and_a_ramp_hides_the_clock() {
    // Different values are changes; after a ramp the in-force tempo is the
    // arrival, not any text, so a repeated header tempo is let go.
    let changing = piece("", "                tempo 1/4 = 120;\n                meter 3/4;");
    assert!(
        coded(&changing, Code::RedundantMarking).is_empty(),
        "{:?}",
        warnings_of(&changing)
    );
    let ramped = piece(
        "",
        "                tempo 1/4 = 96 to 60 over 1/1;\n                tempo 1/4 = 96;",
    );
    assert!(
        coded(&ramped, Code::RedundantMarking).is_empty(),
        "{:?}",
        warnings_of(&ramped)
    );
}

#[test]
fn a_senza_resets_the_meter_record() {
    // `senza` changes the meter and restores it without a statement, so the
    // meter said again afterwards is a change the text cannot see.
    let source = piece(
        "",
        "                senza {\n                    c4/1\n                }\n                meter 4/4;",
    );
    assert!(
        coded(&source, Code::RedundantMarking).is_empty(),
        "{:?}",
        warnings_of(&source)
    );
}

#[test]
fn three_identical_bars_are_a_motif_not_yet_named() {
    let body = "                bar { g4/4 a4/4 g4/4 f4/4 }\n                bar { g4/4 a4/4 g4/4 f4/4 }\n                bar { g4/4 a4/4 g4/4 f4/4 }";
    let lints = coded(&piece("", body), Code::CopiedBars);
    let [lint] = lints.as_slice() else {
        panic!("expected one copied-bars warning: {lints:?}");
    };
    assert!(lint.message.contains("3 times"), "{}", lint.message);
    assert_eq!(lint.labels.len(), 3, "the original plus two copies: {:?}", lint.labels);
}

#[test]
fn two_identical_bars_is_an_accident_and_a_repeat_is_honest() {
    let two = piece(
        "",
        "                bar { g4/4 a4/4 g4/4 f4/4 }\n                bar { g4/4 a4/4 g4/4 f4/4 }",
    );
    assert!(coded(&two, Code::CopiedBars).is_empty(), "{:?}", warnings_of(&two));
    // The same bar inside a `repeat` is the honest spelling of repetition.
    let repeated = piece(
        "",
        "                repeat 3 {\n                    g4/4 a4/4 g4/4 f4/4\n                }",
    );
    assert!(
        coded(&repeated, Code::CopiedBars).is_empty(),
        "{:?}",
        warnings_of(&repeated)
    );
}

#[test]
fn a_waiver_lives_next_to_the_sin() {
    let waived = piece(
        "// musa:allow(unused-material) — kept for the B section\nmotif answer() {\n    g4/4 a4/4 e4/4 f4/4\n}",
        "",
    );
    assert!(
        coded(&waived, Code::UnusedMaterial).is_empty(),
        "{:?}",
        warnings_of(&waived)
    );
    // A waiver names its codes; another rule's name does not waive this one.
    let wrong_code = piece(
        "// musa:allow(copied-bars)\nmotif answer() {\n    g4/4 a4/4 e4/4 f4/4\n}",
        "",
    );
    assert_eq!(coded(&wrong_code, Code::UnusedMaterial).len(), 1);
    // And it waives the construct it stands above, not a neighbour's.
    let neighbour = piece(
        "motif answer() {\n    g4/4 a4/4 e4/4 f4/4\n}\n\n// musa:allow(unused-material)\nmotif reply() {\n    e4/4 d4/4 c4/4 g4/4\n}",
        "",
    );
    assert_eq!(
        coded(&neighbour, Code::UnusedMaterial).len(),
        1,
        "{:?}",
        warnings_of(&neighbour)
    );
}

/// Guide §6: a field is read after its record and a case after its type, so a
/// name that repeats the declaration says it twice.
///
/// The trait and inherent-impl halves of the same rule are unit tests in
/// `lint.rs`: `trait` and `impl` are surface the checker does not resolve yet,
/// so a fixture holding one cannot compile and this file's fixtures must.
#[test]
fn a_field_and_a_case_do_not_repeat_what_they_belong_to() {
    let source = piece(
        "record Duration {\n        duration_beats: Nat;\n    }\n\n    enum Decision {\n        DecisionYes,\n        No,\n    }",
        "",
    );
    let helps: Vec<Option<String>> = coded(&source, Code::RedundantNamePrefix)
        .into_iter()
        .map(|lint| lint.help)
        .collect();
    assert_eq!(
        helps,
        vec![
            Some("name it `beats` — a use already says `Duration` before the name arrives".to_owned()),
            Some("name it `Yes` — a use already says `Decision` before the name arrives".to_owned()),
        ],
        "the field and the case, each named in its own convention, and not the case that reads clean"
    );
}

/// The same declarations spelled the way §6 asks for.
#[test]
fn a_name_that_says_itself_once_is_silent() {
    let source = piece(
        "record Duration {\n        beats: Nat;\n    }\n\n    enum Decision {\n        Yes,\n        No,\n    }",
        "",
    );
    assert!(coded(&source, Code::RedundantNamePrefix).is_empty());
}

#[test]
fn the_examples_are_lint_clean() {
    let mut checked = 0_u32;
    for directory in ["examples", "examples/album/pieces", "examples/analysis"] {
        let mut paths: Vec<_> = std::fs::read_dir(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../..")
                .join(directory),
        )
        .expect("read examples")
        .map(|entry| entry.expect("an entry").path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "musa"))
        .collect();
        paths.sort();
        for path in paths {
            let source = std::fs::read_to_string(&path).expect("read fixture");
            let compilation = compile(
                &SourceDocument::new(source, path.display().to_string()),
                &CompileOptions::default(),
            );
            let lints: Vec<String> = compilation
                .diagnostics()
                .iter()
                .filter(|diagnostic| {
                    diagnostic.severity == Severity::Warning
                        && matches!(
                            diagnostic.code,
                            Code::UnusedMaterial | Code::UnassignedPatch | Code::RedundantMarking | Code::CopiedBars
                        )
                })
                .map(|diagnostic| format!("{}: {}", diagnostic.code, diagnostic.message))
                .collect();
            assert!(lints.is_empty(), "{}: {lints:?}", path.display());
            checked = checked.saturating_add(1);
        }
    }
    assert!(checked >= 20, "the corpus law must have a corpus: {checked}");
}
