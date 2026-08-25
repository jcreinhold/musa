//! Laws for prompt 181's contextual musician-facing sound surface.

#![allow(clippy::expect_used)]

use musa_compiler::{CompileOptions, SourceDocument, compile};
use musa_score::{Severity, diagnose::Code};

fn compile_source(source: &str) -> musa_compiler::Compilation {
    compile(
        &SourceDocument::new(source, "sound-surface.musa"),
        &CompileOptions::default(),
    )
}

#[test]
fn one_sound_sentence_selects_profile_instrument_and_master_route() {
    let source = r#"piece "Sound" {
    meter 4/4;
    key c major;
    performance { profile clear { dynamic f { amplitude = 1; } } }
    instrument glass conforms note_instrument {
        implementation graph { oscillator(sine) |> output; }
    }
    score { part lead { sound glass using clear; voice one { c4/1 } } }
}"#;
    let compilation = compile_source(source);
    let errors: Vec<_> = compilation
        .diagnostics()
        .iter()
        .filter(|diagnostic| diagnostic.severity == Severity::Error)
        .collect();
    assert!(errors.is_empty(), "{errors:#?}");
    assert!(
        compilation.studio_source().is_some(),
        "the selection must reach checked source"
    );
}

#[test]
fn room_is_a_named_shared_effect_path() {
    let source = r#"piece "Room" {
    meter 4/4;
    key c major;
    score { part lead { voice one { c4/1 } } }
    studio { room hall { reverb(room: 0.8); } route hall -> master; }
}"#;
    let compilation = compile_source(source);
    let errors: Vec<_> = compilation
        .diagnostics()
        .iter()
        .filter(|diagnostic| diagnostic.severity == Severity::Error)
        .collect();
    assert!(errors.is_empty(), "{errors:#?}");
}

#[test]
fn compatibility_patch_has_one_exact_instrument_fix() {
    let source = r#"piece "Old" {
    meter 4/4;
    key c major;
    score { part lead { voice one { c4/1 } } }
    studio { patch old { oscillator(sine) |> output; } assign lead -> old; route lead -> master; }
}"#;
    let compilation = compile_source(source);
    let warning = compilation
        .diagnostics()
        .iter()
        .find(|diagnostic| diagnostic.severity == Severity::Warning && diagnostic.code == Code::Syntax)
        .expect("compatibility warning");
    assert_eq!(warning.fixes.len(), 1, "one exact fix: {warning:#?}");
    let fix = warning.fixes.first().expect("the one fix");
    let edit = fix.edits.first().expect("the fix's one edit");
    assert!(edit.replacement.starts_with("instrument old conforms note_instrument"));
}

#[test]
fn contextual_sound_words_remain_ordinary_names_elsewhere() {
    let source = r#"record Terms { instrument: Text; room: Ratio; from: Text; }
piece "Names" { meter 4/4; key c major; score { part p { voice v { rest/1 } } } }"#;
    let compilation = compile_source(source);
    assert!(
        compilation
            .diagnostics()
            .iter()
            .all(|diagnostic| diagnostic.severity != Severity::Error),
        "{:#?}",
        compilation.diagnostics()
    );
}

#[test]
fn removed_q_parameter_has_one_certain_resonance_fix() {
    let source = r#"piece "Q" {
    meter 4/4;
    key c major;
    instrument glass conforms note_instrument {
        implementation graph { oscillator(sine) |> lowpass(cutoff: 1200 Hz, q: 0.7) |> output; }
    }
    score { part lead { sound glass using neutral; voice one { c4/1 } } }
}"#;
    let compilation = compile_source(source);
    let diagnostic = compilation
        .diagnostics()
        .iter()
        .find(|diagnostic| diagnostic.message.contains("`q` is no longer a parameter"))
        .expect("removed spelling diagnostic");
    assert_eq!(diagnostic.fixes.len(), 1, "one exact fix: {diagnostic:#?}");
    let fix = diagnostic.fixes.first().expect("the one fix");
    let edit = fix.edits.first().expect("the fix's one edit");
    assert_eq!(edit.replacement, "resonance");
}
