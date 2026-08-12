//! Laws for contextual `music`: it is composed while open, instantiated at
//! each `use`, and only then closed into one checked kernel term.

#![allow(clippy::expect_used)]

use musa_compiler::{
    Code, CompileOptions, MusicalDuration, MusicalTime, Realization, ScoreEvent, ScoreSnapshot, SourceDocument,
    check_kernel_text, compile, kernel_text,
};
use num_rational::Ratio;

fn compiled(source: &str) -> musa_compiler::Compilation {
    compile(
        &SourceDocument::new(source, "music-laws.musa"),
        &CompileOptions::default(),
    )
}

fn snapshot(source: &str) -> ScoreSnapshot {
    let compilation = compiled(source);
    assert!(!compilation.has_errors(), "{:?}", compilation.diagnostics());
    compilation.into_snapshot().expect("valid contextual music has a score")
}

fn voices(score: &ScoreSnapshot) -> Vec<Vec<ScoreEvent>> {
    score
        .parts()
        .iter()
        .flat_map(|(_, part)| part.voices())
        .map(|(_, voice)| voice.events().to_vec())
        .collect()
}

#[test]
fn sequence_adds_extent_and_each_use_reads_its_own_placement() {
    let source = "piece \"music\" {
        fn figure(root: Pitch) -> Music { music { root/4 d4/4 } }
        let subject: Music = figure(c4);
        score { part p { voice v {
            use subject;
            rest/4
            use subject;
        } } }
    }";
    let score = snapshot(source);
    let lanes = voices(&score);
    let events = lanes.first().expect("one voice");
    assert_eq!(events.len(), 5);
    let onsets = events.iter().map(|event| event.onset).collect::<Vec<_>>();
    assert_eq!(
        onsets,
        vec![
            MusicalTime::ZERO,
            MusicalTime::new(Ratio::new(1, 4)),
            MusicalTime::new(Ratio::new(1, 2)),
            MusicalTime::new(Ratio::new(3, 4)),
            MusicalTime::new(Ratio::new(1, 1)),
        ]
    );
    assert_eq!(
        score.parts().iter().next().map(|(_, part)| part.span()),
        Some(MusicalDuration::new(Ratio::new(5, 4)))
    );
}

#[test]
fn voice_overlay_takes_the_maximum_of_contextual_sequence_extents() {
    let source = "piece \"overlay\" {
        let cell: Music = music { c4/2 };
        score { part p {
            voice upper { use cell; }
            voice lower { use cell; use cell; }
        } }
    }";
    let score = snapshot(source);
    let lanes = score
        .parts()
        .iter()
        .next()
        .map(|(_, part)| part.voices().map(|(_, voice)| voice.span()).collect::<Vec<_>>())
        .unwrap_or_default();
    assert_eq!(
        lanes,
        vec![
            MusicalDuration::new(Ratio::new(1, 2)),
            MusicalDuration::new(Ratio::new(1, 1)),
        ]
    );
    assert_eq!(
        score.parts().iter().next().map(|(_, part)| part.span()),
        Some(MusicalDuration::new(Ratio::new(1, 1)))
    );
}

#[test]
fn contextual_music_is_neutral_with_respect_to_structural_context() {
    let compilation = compiled(
        "piece \"neutral\" { let cell: Music = music { meter 3/4; c4/4 }; \
         score { part p { voice v { use cell; } } } }",
    );
    assert!(compilation.has_errors());
    assert!(compilation.snapshot().is_none());
    assert!(
        compilation
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code == Code::Misplaced)
    );
}

#[test]
fn shared_instantiations_are_closed_and_keep_definition_and_call_provenance() {
    let source = "piece \"sharing\" {
        let cell: Music = music { c4/4 d4/4 };
        score { part p { voice v { use cell; use cell; } } }
    }";
    let score = snapshot(source);
    let lanes = voices(&score);
    let events = lanes.first().expect("one voice");
    assert_eq!(events.len(), 4);
    let first = events.first().expect("first use");
    let second = events.get(2).expect("second use");
    assert_eq!(first.origin.definition_span, second.origin.definition_span);
    assert_ne!(first.origin.source_span, second.origin.source_span);
    assert_eq!(first.origin.expansion_path.len(), 1);
    assert_eq!(second.origin.expansion_path.len(), 1);

    let printed = kernel_text(
        &SourceDocument::new(source, "music-laws.musa"),
        &Realization::deterministic(),
    )
    .expect("contextual music closes to a term");
    assert!(printed.contains("let shared0 ="), "{printed}");
    let checked = check_kernel_text(&printed).expect("the emitted term is closed and checked");
    assert_eq!(checked.occurrences, 5, "four notes plus the piece's meter fact");
}

#[test]
fn an_oversized_music_value_is_rejected_before_repeat_expansion() {
    let compilation = compiled(
        "piece \"finite\" { let cell: Music = music { c4/4 }; \
         score { part p { voice v { repeat 1000001 { use cell; } } } } }",
    );
    assert!(compilation.snapshot().is_none());
    assert!(
        compilation
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code == Code::ResourceLimit)
    );
}
