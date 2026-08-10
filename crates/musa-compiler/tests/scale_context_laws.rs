//! Scales, degrees, register, and the lexical pitch context.
//!
//! The domain values are crate-private, so these laws are stated the way a
//! composer states them: in source text, through the pitches a piece
//! elaborates to. That is also the only statement that matters — a law about
//! a private struct that the surface language does not honour is not a law.

use musa_compiler::{CompileOptions, ScoreEventKind, SourceDocument, compile};

/// Every sounding pitch of a compiled piece, in order, as written.
fn pitches(source: &str) -> Vec<String> {
    let compilation = compile(
        &SourceDocument::new(source, "scale-laws.musa"),
        &CompileOptions::default(),
    );
    assert!(!compilation.has_errors(), "{:#?}", compilation.diagnostics());
    compilation
        .snapshot()
        .map(|snapshot| {
            snapshot
                .parts()
                .iter()
                .flat_map(|(_, part)| part.voices())
                .flat_map(|(_, voice)| voice.events())
                .filter_map(|event| match &event.kind {
                    ScoreEventKind::Note { pitch } => Some(pitch.to_string()),
                    ScoreEventKind::Rest | ScoreEventKind::Chord { .. } => None,
                })
                .collect()
        })
        .unwrap_or_default()
}

/// The errors a piece reports, as one string.
fn errors(source: &str) -> String {
    let compilation = compile(
        &SourceDocument::new(source, "scale-laws.musa"),
        &CompileOptions::default(),
    );
    compilation
        .diagnostics()
        .iter()
        .map(|diagnostic| format!("{diagnostic:?}"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// A piece whose single voice holds `body`.
fn piece(body: &str) -> String {
    format!("piece \"Laws\" {{\n    score {{ part p {{ voice v {{\n{body}\n    }} }} }}\n}}\n")
}

/// A piece with declarations before the score.
fn piece_with(declarations: &str, body: &str) -> String {
    format!("piece \"Laws\" {{\n{declarations}\n    score {{ part p {{ voice v {{\n{body}\n    }} }} }}\n}}\n")
}

#[test]
fn a_scale_walks_its_own_reference_map() {
    // C major from the tonic: the collection's ordered offsets, in register.
    assert_eq!(
        pitches(&piece(
            "in scale c major { c4/8 (c4 step 1)/8 (c4 step 2)/8 (c4 step 3)/8 (c4 step 4)/8 (c4 step 5)/8 (c4 step 6)/8 (c4 step 7)/8 }"
        )),
        ["c4", "d4", "e4", "f4", "g4", "a4", "b4", "c5"]
    );
    // C Dorian differs from C major exactly where the collections differ.
    assert_eq!(
        pitches(&piece("in scale c dorian { c4/8 (c4 step 2)/8 (c4 step 6)/8 }")),
        ["c4", "eb4", "bb4"]
    );
}

#[test]
fn one_bound_phrase_elaborates_differently_under_two_scales() {
    // The open-binding law: `subject` is evaluated once, and each use reads
    // the scale in force where it is written. Saving a phrase does not freeze
    // its coordinates.
    let source = piece_with(
        "    fn figure() -> music = music {\n        c5/8\n        (c5 step 1)/8\n        (c5 step 2)/4\n    };\n\n    let subject: music = figure();",
        "        in scale c major { use subject; }\n        in scale c dorian { use subject; }",
    );
    assert_eq!(
        pitches(&source),
        ["c5", "d5", "e5", "c5", "d5", "eb5"],
        "the same music value must read the scale at each use site"
    );
}

#[test]
fn a_nested_scale_shadows_the_one_around_it_and_restores_it() {
    assert_eq!(
        pitches(&piece(
            "in scale c major {\n            (c4 step 2)/4\n            in scale c dorian { (c4 step 2)/4 }\n            (c4 step 2)/4\n        }"
        )),
        ["e4", "eb4", "e4"]
    );
}

#[test]
fn the_scale_distributes_over_sequence_and_overlay() {
    // Sequenced: every item of the block reads the same scale.
    let sequenced = pitches(&piece(
        "in scale c dorian { (c4 step 2)/4 (c4 step 6)/4 (c4 step 2)/4 }",
    ));
    assert_eq!(sequenced, ["eb4", "bb4", "eb4"]);
    // Overlaid: a `use` inside the context reads it too, in both branches.
    let overlaid = pitches(&piece_with(
        "    let low: music = music { (c4 step 2)/2 };\n    let high: music = music { (c5 step 2)/2 };",
        "        in scale c dorian { use overlay(low, high); }",
    ));
    assert_eq!(overlaid, ["eb4", "eb5"]);
}

#[test]
fn the_scale_is_independent_of_the_rest_of_the_environment() {
    // Transposition moves the sounding result; it does not move the
    // coordinates `step` reads, which stay the ones written in the source.
    let plain = pitches(&piece("in scale c dorian { (c4 step 2)/4 }"));
    let transposed = pitches(&piece("in scale c dorian { transpose up P8 { (c4 step 2)/4 } }"));
    assert_eq!(plain, ["eb4"]);
    assert_eq!(transposed, ["eb5"]);
    // A stretch changes durations only.
    let stretched = pitches(&piece("in scale c dorian { stretch 2/1 { (c4 step 2)/8 } }"));
    assert_eq!(stretched, plain);
}

#[test]
fn a_numbered_degree_realizes_in_the_frame_that_registers_it() {
    // Degrees are written from one: degree 1 of a C major frame rooted on c4
    // is c4 itself, and 1/3/5 spell that frame's triad in its own register.
    let source = piece_with(
        "    use std::scale;\n\n    fn triad(register: frame) -> music = music {\n        (frame_degree(register, 1))/4\n        (frame_degree(register, 3))/4\n        (frame_degree(register, 5))/4\n    };\n\n    let anchored: music = option_fold(music { rest/4 }, triad, frame_on(scale c major, c4));",
        "        use anchored;",
    );
    assert_eq!(pitches(&source), ["c4", "e4", "g4"]);
}

#[test]
fn degrees_are_periodic_and_stepping_is_additive() {
    // A whole period up is the written period up, spelling included.
    assert_eq!(
        pitches(&piece("in scale c major { (c4 step 7)/4 (d4 step 7)/4 }")),
        ["c5", "d5"]
    );
    // Five-note collections have a five-note period, not a seven-note one.
    assert_eq!(
        pitches(&piece(
            "in scale c major_pentatonic { c4/4 (c4 step 5)/4 (c4 step 2)/4 }"
        )),
        ["c4", "c5", "e4"]
    );
    // Eight-note collections have an eight-note period.
    assert_eq!(
        pitches(&piece(
            "in scale c octatonic_half_whole { c4/4 (c4 step 8)/4 (c4 step 1)/4 }"
        )),
        ["c4", "c5", "db4"]
    );
    // Stepping is additive: two steps then three is five.
    assert_eq!(
        pitches(&piece("in scale c major { ((c4 step 2) step 3)/4 (c4 step 5)/4 }")),
        ["a4", "a4"]
    );
    // Down undoes up.
    assert_eq!(
        pitches(&piece("in scale c major { ((c4 step 3) step down 3)/4 }")),
        ["c4"]
    );
}

#[test]
fn the_minor_collections_are_distinct_values() {
    // The seventh degree is what separates them, and the sixth separates
    // melodic minor from the other two.
    assert_eq!(
        pitches(&piece("in scale c natural_minor { (c4 step 5)/4 (c4 step 6)/4 }")),
        ["ab4", "bb4"]
    );
    assert_eq!(
        pitches(&piece("in scale c harmonic_minor { (c4 step 5)/4 (c4 step 6)/4 }")),
        ["ab4", "b4"]
    );
    assert_eq!(
        pitches(&piece("in scale c melodic_minor { (c4 step 5)/4 (c4 step 6)/4 }")),
        ["a4", "b4"]
    );
    // Descending melodic minor has the natural collection's pitches and is a
    // value of its own, which is what lets a passage name the direction.
    assert_eq!(
        pitches(&piece(
            "in scale c descending_melodic_minor { (c4 step 5)/4 (c4 step 6)/4 }"
        )),
        ["ab4", "bb4"]
    );
}

#[test]
fn chromatic_motion_and_scale_stepping_do_not_commute() {
    // Step-then-transpose and transpose-then-step are different operations,
    // and the counterexample is written out rather than asserted in prose.
    let step_then_move = pitches(&piece("in scale c major { ((c4 step 1) up m2)/4 }"));
    assert_eq!(step_then_move, ["eb4"]);
    // `db4` is not in C major, so the other order has no coordinate at all —
    // the operations do not commute, and one of them is not even defined here.
    let reported = errors(&piece("in scale c major { ((c4 up m2) step 1)/4 }"));
    assert!(
        reported.contains("is not a member of"),
        "expected a membership diagnostic, got: {reported}"
    );
}

#[test]
fn a_key_supplies_the_default_collection_and_a_scale_overrides_it() {
    // `key c minor` prints three flats and suggests the natural collection.
    let defaulted = pitches(&piece_with("    key c minor;", "        c4/4\n        (c4 step 6)/4"));
    assert_eq!(defaulted, ["c4", "bb4"]);
    // Naming a collection changes the coordinates and nothing else. The key
    // signature is still the one the header wrote.
    let overridden = pitches(&piece_with(
        "    key c minor;",
        "        in scale c harmonic_minor { c4/4 (c4 step 6)/4 }",
    ));
    assert_eq!(overridden, ["c4", "b4"]);
    let compilation = compile(
        &SourceDocument::new(
            piece_with(
                "    key c minor;",
                "        in scale c harmonic_minor { c4/4 (c4 step 6)/4 }",
            ),
            "scale-laws.musa",
        ),
        &CompileOptions::default(),
    );
    let keys = compilation
        .snapshot()
        .map(|snapshot| {
            snapshot
                .parts()
                .iter()
                .flat_map(|(_, part)| part.voices())
                .flat_map(|(_, voice)| voice.events())
                .filter(|event| matches!(event.kind, ScoreEventKind::Note { .. }))
                .count()
        })
        .unwrap_or_default();
    assert_eq!(keys, 2, "`in scale` must not add or remove score events");
}

#[test]
fn an_absent_scale_is_a_diagnostic_rather_than_c_major() {
    let reported = errors(&piece("c4/4 (c4 step 1)/4"));
    assert!(
        reported.contains("needs a scale in force"),
        "expected the missing-scale diagnostic, got: {reported}"
    );
}

#[test]
fn a_note_outside_the_scale_has_no_coordinate() {
    let reported = errors(&piece("in scale c major { f#5/4 (f#5 step 1)/4 }"));
    assert!(
        reported.contains("is not a member of"),
        "expected the membership diagnostic, got: {reported}"
    );
}

#[test]
fn a_scale_context_records_itself_in_the_origin() {
    let compilation = compile(
        &SourceDocument::new(piece("in scale c dorian { (c4 step 2)/4 }"), "scale-laws.musa"),
        &CompileOptions::default(),
    );
    assert!(!compilation.has_errors(), "{:#?}", compilation.diagnostics());
    let steps: Vec<String> = compilation
        .snapshot()
        .map(|snapshot| {
            snapshot
                .parts()
                .iter()
                .flat_map(|(_, part)| part.voices())
                .flat_map(|(_, voice)| voice.events())
                .flat_map(|event| event.origin.expansion_path.clone())
                .map(|step| format!("{step:?}"))
                .collect()
        })
        .unwrap_or_default();
    assert!(
        steps.iter().any(|step| step.contains("ScaleContext")),
        "expected a ScaleContext origin step, got: {steps:?}"
    );
}

#[test]
fn a_scale_emits_no_key_signature() {
    let compilation = compile(
        &SourceDocument::new(piece("in scale c dorian { (c4 step 2)/4 }"), "scale-laws.musa"),
        &CompileOptions::default(),
    );
    assert!(!compilation.has_errors(), "{:#?}", compilation.diagnostics());
    let stated = compilation
        .snapshot()
        .and_then(|snapshot| snapshot.key_at(musa_compiler::Scope::Piece, musa_compiler::MusicalTime::ZERO));
    assert!(
        stated.is_none(),
        "`in scale` must not write a key signature: {stated:?}"
    );
}
