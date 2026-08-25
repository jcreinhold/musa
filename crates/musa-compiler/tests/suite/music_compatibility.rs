//! Legacy material declarations are roles on the contextual-music path, not
//! an independently interpreted language.

#![allow(clippy::expect_used)]

use musa_compiler::{CompileOptions, SourceDocument, compile};

use musa_score::ScoreEventKind;

fn sounding_facts(declaration: &str, uses: &str) -> Vec<String> {
    let source = format!("piece \"compatibility\" {{ {declaration} score {{ part p {{ voice v {{ {uses} }} }} }} }}");
    let compilation = compile(
        &SourceDocument::new(source, "music-compatibility.musa"),
        &CompileOptions::default(),
    );
    assert!(!compilation.has_errors(), "{:?}", compilation.diagnostics());
    compilation
        .snapshot()
        .expect("compatible form has a score")
        .parts()
        .iter()
        .flat_map(|(_, part)| part.voices())
        .flat_map(|(_, voice)| voice.events())
        .map(|event| {
            let kind = match &event.kind {
                ScoreEventKind::Note { pitch } => format!("note:{pitch}"),
                ScoreEventKind::Rest => "rest".to_owned(),
                ScoreEventKind::Chord { pitches } => format!("chord:{pitches:?}"),
            };
            format!(
                "{}|{}|{}",
                event.onset.as_ratio(),
                event.notated_duration.value.as_ratio(),
                kind
            )
        })
        .collect()
}

#[test]
fn motif_sugar_and_a_music_function_have_the_same_score_facts() {
    let legacy = sounding_facts("motif turn(root: Pitch) { root/4 d4/4 }", "use turn(c4); use turn(e4);");
    let general = sounding_facts(
        "fn turn(root: Pitch) -> EventTrack(WrittenTime) { music { root/4 d4/4 } }",
        "use turn(c4); use turn(e4);",
    );
    assert_eq!(legacy, general);
}

#[test]
fn fragment_sugar_and_a_music_binding_have_the_same_score_facts() {
    let legacy = sounding_facts("fragment answer { g4/8 a4/8 }", "use answer;");
    let general = sounding_facts(
        "let answer: EventTrack(WrittenTime) = music { g4/8 a4/8 };",
        "use answer;",
    );
    assert_eq!(legacy, general);
}
