//! Laws for higher-order contextual music.

#![allow(clippy::expect_used)]
#![allow(clippy::indexing_slicing)]
#![allow(clippy::panic)]

use musa_compiler::{CompileOptions, SourceDocument, compile};

use musa_score::{
    Code, ExpansionStep, MusicalDuration, MusicalTime, ScoreEvent, ScoreEventKind, ScoreSnapshot, WrittenPitch,
};
use num_rational::Ratio;

fn compile_text(source: &str) -> musa_compiler::Compilation {
    compile(
        &SourceDocument::new(source, "higher-order-music.musa"),
        &CompileOptions::default(),
    )
}

fn snapshot(source: &str) -> ScoreSnapshot {
    let compilation = compile_text(source);
    assert!(!compilation.has_errors(), "{:?}", compilation.diagnostics());
    compilation
        .into_snapshot()
        .expect("valid higher-order music has a score")
}

fn voices(score: &ScoreSnapshot) -> Vec<Vec<ScoreEvent>> {
    score
        .parts()
        .iter()
        .flat_map(|(_, part)| part.voices())
        .map(|(_, voice)| voice.events().to_vec())
        .collect()
}

fn shape(events: &[ScoreEvent]) -> Vec<(MusicalTime, MusicalDuration, ScoreEventKind)> {
    events
        .iter()
        .map(|event| (event.onset, event.notated_duration.value, event.kind.clone()))
        .collect()
}

#[test]
fn a_delayed_canon_accepts_a_partially_applied_answer_and_has_maximum_extent() {
    let score = snapshot(include_str!("../../../../examples/canon-functions.musa"));
    let lanes = voices(&score);
    let events = lanes.first().expect("one canon voice");
    assert_eq!(events.len(), 4);
    assert_eq!(events[2].onset, MusicalTime::new(Ratio::new(1, 2)));
    assert_eq!(events[3].onset, MusicalTime::new(Ratio::new(3, 4)));
    assert_eq!(
        events[2].kind,
        ScoreEventKind::Note {
            pitch: WrittenPitch::parse("c5").expect("pitch")
        }
    );
    assert_eq!(
        score.parts().iter().next().map(|(_, part)| part.span()),
        Some(MusicalDuration::new(Ratio::ONE))
    );
}

#[test]
fn function_identity_and_composition_hold_for_contextual_music() {
    let score = snapshot(
        "piece \"unchanged\" {
            let subject: EventTrack<WrittenTime> = music { c4/4 d4/4 };
            fn unchanged(value: EventTrack<WrittenTime>) -> EventTrack<WrittenTime> { value }
            fn compose(f: EventTrack<WrittenTime> -> EventTrack<WrittenTime>, g: EventTrack<WrittenTime> -> EventTrack<WrittenTime>, value: EventTrack<WrittenTime>) -> EventTrack<WrittenTime> { f(g(value)) }
            score { part p {
                voice direct { use subject; }
                voice unchanged { use unchanged(subject); }
                voice composed { use compose(unchanged, unchanged, subject); }
            } }
        }",
    );
    let lanes = voices(&score);
    assert_eq!(shape(&lanes[0]), shape(&lanes[1]));
    assert_eq!(shape(&lanes[0]), shape(&lanes[2]));
}

#[test]
fn pitch_mapping_preserves_support_and_non_pitch_fields() {
    let score = snapshot(
        "piece \"mapping\" {
            let subject: EventTrack<WrittenTime> = music {
                grace { d5 e5 }
                c4/4 staccato
                rest/4
            };
            fn unchanged(p: Pitch) -> Pitch { p }
            fn pedal(_: Pitch) -> Pitch { g3 }
            score { part p {
                voice original { use subject; }
                voice same { use map_note_pitches(unchanged, subject); }
                voice pedal { use map_note_pitches(pedal, subject); }
            } }
        }",
    );
    let lanes = voices(&score);
    assert_eq!(shape(&lanes[0]), shape(&lanes[1]), "unchanged changes no musical fact");
    assert_eq!(
        lanes[2]
            .iter()
            .map(|event| (event.onset, event.notated_duration.value))
            .collect::<Vec<_>>(),
        lanes[0]
            .iter()
            .map(|event| (event.onset, event.notated_duration.value))
            .collect::<Vec<_>>(),
        "mapping preserves exact temporal support"
    );
    assert_eq!(
        lanes[2][0].kind,
        ScoreEventKind::Note {
            pitch: WrittenPitch::parse("g3").expect("pitch")
        }
    );
    assert_eq!(lanes[2][1].kind, ScoreEventKind::Rest);
    assert_eq!(score.annotations().graces().len(), 6);
    assert_eq!(
        score.annotations().graces()[4].pitch,
        WrittenPitch::parse("g3").expect("pitch")
    );
    assert!(
        lanes[2][0]
            .origin
            .expansion_path
            .contains(&ExpansionStep::MapNotePitches)
    );
}

#[test]
fn mapping_composition_agrees_with_nested_mapping() {
    let score = snapshot(
        "piece \"composition\" {
            let subject: EventTrack<WrittenTime> = music { c4/2 };
            fn pedal(_: Pitch) -> Pitch { e3 }
            fn twice(f: Pitch -> Pitch, p: Pitch) -> Pitch { f(f(p)) }
            fn pedal_twice(p: Pitch) -> Pitch { twice(pedal, p) }
            score { part p {
                voice nested { use map_note_pitches(pedal, map_note_pitches(pedal, subject)); }
                voice composed { use map_note_pitches(pedal_twice, subject); }
            } }
        }",
    );
    let lanes = voices(&score);
    assert_eq!(shape(&lanes[0]), shape(&lanes[1]));
}

#[test]
fn block_and_function_transpose_agree_musically_but_keep_provenance() {
    let score = snapshot(
        "piece \"agreement\" {
            let subject: EventTrack<WrittenTime> = music { c4/4 e4/4 };
            score { part p {
                voice block { transpose up P8 { use subject; } }
                voice function { use transpose(P8, subject); }
            } }
        }",
    );
    let lanes = voices(&score);
    assert_eq!(shape(&lanes[0]), shape(&lanes[1]));
    assert!(lanes[0].iter().all(|event| {
        event
            .origin
            .expansion_path
            .iter()
            .any(|step| matches!(step, ExpansionStep::Transposition(_)))
    }));
    assert!(lanes[1].iter().all(|event| {
        event
            .origin
            .expansion_path
            .iter()
            .any(|step| matches!(step, ExpansionStep::Transposition(_)))
    }));
    assert_ne!(lanes[0][0].origin.source_span, lanes[1][0].origin.source_span);
}

#[test]
fn every_existing_transform_has_one_block_and_function_meaning() {
    let score = snapshot(
        "piece \"transform functions\" {
            let subject: EventTrack<WrittenTime> = music { c4/4 e4/4 };
            let broader: EventTrack<WrittenTime> -> EventTrack<WrittenTime> = fn (line: EventTrack<WrittenTime>) -> EventTrack<WrittenTime> { stretch(2/1, line) };
            let backwards: EventTrack<WrittenTime> -> EventTrack<WrittenTime> = retrograde;
            let mirror: EventTrack<WrittenTime> -> EventTrack<WrittenTime> = fn (line: EventTrack<WrittenTime>) -> EventTrack<WrittenTime> { invert(c4, line) };
            score { part p {
                voice stretch_block { stretch 2 { use subject; } }
                voice stretch_function { use broader(subject); }
                voice retrograde_block { retrograde { use subject; } }
                voice retrograde_function { use backwards(subject); }
                voice invert_block { invert around c4 { use subject; } }
                voice invert_function { use mirror(subject); }
            } }
        }",
    );
    let lanes = voices(&score);
    for pair in lanes.as_chunks::<2>().0 {
        assert_eq!(shape(&pair[0]), shape(&pair[1]));
    }
}

#[test]
fn wrong_higher_order_arguments_are_rejected_statically() {
    for declaration in [
        "fn wrong(n: Nat) -> Nat { n } let bad: EventTrack<WrittenTime> = map_note_pitches(wrong, music { c4/1 });",
        "fn answer_pitch(p: Pitch) -> Pitch { p } let bad: EventTrack<WrittenTime> = map_note_pitches(answer_pitch, 1);",
    ] {
        let source = format!("piece \"wrong\" {{ {declaration} score {{ part p {{ voice v {{ c4/1 }} }} }} }}");
        let compilation = compile_text(&source);
        assert!(compilation.has_errors());
        assert!(
            compilation
                .diagnostics()
                .iter()
                .any(|diagnostic| diagnostic.code == Code::ConversionMismatch),
            "{:?}",
            compilation.diagnostics()
        );
    }
}

#[test]
fn distinct_mappers_and_call_sites_do_not_alias_shared_instantiations() {
    let score = snapshot(
        "piece \"cache separation\" {
            let subject: EventTrack<WrittenTime> = music { c4/4 };
            fn low(_: Pitch) -> Pitch { c3 }
            fn high(_: Pitch) -> Pitch { c5 }
            score { part p { voice v {
                use map_note_pitches(low, subject);
                use map_note_pitches(high, subject);
                rest/2
            } } }
        }",
    );
    let events = &voices(&score)[0];
    assert_eq!(
        events[0].kind,
        ScoreEventKind::Note {
            pitch: WrittenPitch::parse("c3").expect("pitch")
        }
    );
    assert_eq!(
        events[1].kind,
        ScoreEventKind::Note {
            pitch: WrittenPitch::parse("c5").expect("pitch")
        }
    );
    assert_ne!(events[0].origin.source_span, events[1].origin.source_span);
}
