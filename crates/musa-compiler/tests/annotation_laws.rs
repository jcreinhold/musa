//! The annotation layer's contracts (docs/prompts/35).
//!
//! Phrases, form markers, and the harmony lane are *about* the music without
//! being part of it, and that is the whole property under test here: adding
//! any of them changes no sounding event, and removing them loses no note.
//!
//! Chord symbols get a parse table rather than examples. A symbol is parsed
//! (roadmap §8.2) so a later theory library reads structure instead of four
//! letters, which makes the reading itself the contract: `c7` and `cmaj7`
//! share a triad and differ in one note, and a table is the only honest way
//! to say which spellings mean what. Nothing here derives notes from a
//! symbol — that restraint is the feature, not an omission.
//!
//! Positions are the other half: a `measure:beat` coordinate is meaningful
//! only against a meter, so naming one past the end of the piece is an error
//! rather than a marker nobody will ever reach.

// Test helpers use expect() on statically-valid inputs: a failure is a bug in
// the test itself, and panicking is the correct behavior there.
#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]

use musa_compiler::{
    ChordQuality, ChordSymbol, CompileOptions, IntegratedTempoMap, PerformanceOptions, ScoreSnapshot, Severity,
    SourceDocument, compile,
};

const ANNOTATED: &str = include_str!("../../../examples/annotated.musa");

fn snapshot_of(source: &str) -> ScoreSnapshot {
    compile(&SourceDocument::new(source, "test.musa"), &CompileOptions::default())
        .into_snapshot()
        .expect("compiles")
}

fn errors_of(source: &str) -> Vec<String> {
    compile(&SourceDocument::new(source, "test.musa"), &CompileOptions::default())
        .diagnostics()
        .iter()
        .filter(|diagnostic| diagnostic.severity == Severity::Error)
        .map(|diagnostic| diagnostic.message.clone())
        .collect()
}

/// A piece whose voice is the eight quarter notes of two 4/4 measures, plus
/// whatever score-level annotations the caller writes above the part.
fn piece(annotations: &str) -> String {
    format!(
        "piece \"P\" {{ tempo 1/4 = 60; meter 4/4; key c major; score {{ {annotations}
            part piano {{ voice one {{ c4 1/4; d4 1/4; e4 1/4; f4 1/4; g4 1/4; a4 1/4; b4 1/4; c5 1/4; }} }} }} }}"
    )
}

/// The events of a snapshot as `(onset, kind)`, which is everything an
/// annotation is forbidden to change.
fn music(snapshot: &ScoreSnapshot) -> Vec<String> {
    let mut out = Vec::new();
    for (_, part) in snapshot.parts().iter() {
        for (_, voice) in part.voices() {
            for event in voice.events() {
                out.push(format!("{} {:?}", event.onset.as_ratio(), event.kind));
            }
        }
    }
    out
}

#[test]
fn annotating_a_piece_changes_no_note() {
    let plain = snapshot_of(&piece(""));
    let annotated = snapshot_of(&piece("section \"A\" at 1:1; harmony { at 1:1 c; at 1:3 g7; }"));
    assert_eq!(music(&plain), music(&annotated));
    assert_eq!(annotated.annotations().sections().len(), 1);
    assert_eq!(annotated.annotations().harmony().len(), 2);
}

#[test]
fn a_phrase_brackets_the_events_written_inside_it() {
    let snapshot = snapshot_of(&piece(""));
    let plain = music(&snapshot);
    let source = "piece \"P\" { tempo 1/4 = 60; meter 4/4; key c major; score {
        part piano { voice one { phrase \"A\" { c4 1/4; d4 1/4; } e4 1/4; f4 1/4;
            g4 1/4; a4 1/4; b4 1/4; c5 1/4; } } } }";
    let phrased = snapshot_of(source);
    assert_eq!(plain, music(&phrased));

    let phrases = phrased.annotations().phrases();
    let [phrase] = phrases else {
        panic!("expected exactly one phrase, got {phrases:?}");
    };
    assert_eq!(phrase.name, "A");
    // The bracket covers the first two events and stops there: the notes
    // after the closing brace are outside it.
    assert_eq!(phrase.to.0.saturating_sub(phrase.from.0), 1);
}

/// Positions are read against the meter, so the same coordinate means a
/// different time under a different meter — and the lane is stored in the
/// order the marks are reached, not the order they were typed.
#[test]
fn a_position_is_measured_against_the_meter() {
    let snapshot = snapshot_of(&piece("harmony { at 2:1 g; at 1:3 f; at 1:1 c; }"));
    let times: Vec<String> = snapshot
        .annotations()
        .harmony()
        .iter()
        .map(|mark| format!("{} {}", mark.at.as_ratio(), mark.symbol.text))
        .collect();
    assert_eq!(times, ["0 c", "1/2 f", "1 g"]);
}

#[test]
fn a_position_past_the_end_of_the_piece_is_an_error() {
    let errors = errors_of(&piece("section \"Coda\" at 9:1;"));
    assert!(
        errors
            .iter()
            .any(|message| message.contains("the piece ends before `9:1`")),
        "expected a diagnostic naming the position, got {errors:?}"
    );
}

/// A piece that ends in silence still ends where the silence ends.
///
/// Prompt 40 replaced "the maximum over event ends" with the timeline's own
/// extent, and the two agree only because a written rest is an occurrence
/// (prompt 39). Stated as a fixture rather than as reasoning: a position
/// inside the trailing rest is reachable, and one past it is not.
#[test]
fn a_piece_that_ends_in_a_rest_ends_where_the_rest_ends() {
    let ending_in_silence = |annotations: &str| {
        format!(
            "piece \"P\" {{ tempo 1/4 = 60; meter 4/4; score {{ {annotations}
                part piano {{ voice one {{ c4 1/4; rest 1/4; rest 1/2; rest 1; }} }} }} }}"
        )
    };
    // The music stops at 1/4; the rests carry the piece to the end of bar 2.
    assert!(errors_of(&ending_in_silence("section \"Fade\" at 2:1;")).is_empty());
    let errors = errors_of(&ending_in_silence("section \"Gone\" at 3:1;"));
    assert!(
        errors
            .iter()
            .any(|message| message.contains("the piece ends before `3:1`")),
        "expected a diagnostic naming the position, got {errors:?}"
    );
}

#[test]
fn a_beat_past_the_end_of_its_measure_is_an_error() {
    let errors = errors_of(&piece("section \"Late\" at 1:9;"));
    assert!(
        errors
            .iter()
            .any(|message| message.contains("the piece ends before `1:9`")),
        "expected a diagnostic naming the position, got {errors:?}"
    );
}

#[test]
fn a_symbol_that_is_not_a_chord_is_reported_rather_than_stored() {
    let errors = errors_of(&piece("harmony { at 1:1 hq13; }"));
    assert!(
        errors
            .iter()
            .any(|message| message.contains("`hq13` is not a chord symbol musa reads")),
        "expected a diagnostic naming the symbol, got {errors:?}"
    );
}

#[test]
fn every_chord_goes_in_one_harmony_lane() {
    let errors = errors_of(&piece("harmony { at 1:1 c; } harmony { at 1:3 g; }"));
    assert!(
        errors
            .iter()
            .any(|message| message.contains("already has a harmony lane")),
        "expected a diagnostic about the second lane, got {errors:?}"
    );
}

/// The chord grammar, as a table. Each row is `(written, root, quality,
/// seventh, extension)` — the reading a theory library would get, not the
/// notes, which musa never computes (§8.2).
#[test]
fn the_chord_grammar_reads_what_a_lead_sheet_writes() {
    let table: [(&str, &str, ChordQuality, Option<&str>, Option<u8>); 14] = [
        ("c", "c", ChordQuality::Major, None, None),
        ("am", "a", ChordQuality::Minor, None, None),
        ("cmin", "c", ChordQuality::Minor, None, None),
        ("efmaj", "ef", ChordQuality::Major, None, None),
        ("fsdim", "fs", ChordQuality::Diminished, None, None),
        ("gaug", "g", ChordQuality::Augmented, None, None),
        ("dsus4", "d", ChordQuality::Suspended4, None, None),
        ("dsus2", "d", ChordQuality::Suspended2, None, None),
        ("c6", "c", ChordQuality::Major, None, Some(6)),
        ("g7", "g", ChordQuality::Major, Some("minor"), None),
        ("fmaj7", "f", ChordQuality::Major, Some("major"), None),
        ("bm7", "b", ChordQuality::Minor, Some("minor"), None),
        ("cmmaj7", "c", ChordQuality::Minor, Some("major"), None),
        ("bfdim7", "bf", ChordQuality::Diminished, Some("diminished"), None),
    ];
    for (written, root, quality, seventh, extension) in table {
        let chord = ChordSymbol::parse(written).unwrap_or_else(|| panic!("`{written}` should parse"));
        let spelled = format!("{}{}", letter_of(&chord), accidental_of(&chord));
        assert_eq!(spelled, root, "root of `{written}`");
        assert_eq!(chord.quality, quality, "quality of `{written}`");
        assert_eq!(chord.seventh.map(seventh_name), seventh, "seventh of `{written}`");
        assert_eq!(chord.extension, extension, "extension of `{written}`");
        assert_eq!(chord.text, written, "`{written}` keeps what was written");
    }
}

/// An extension implies the seventh under it: `c9` is a dominant ninth, and a
/// library that wants the notes must not have to guess that.
#[test]
fn an_extension_implies_the_seventh_below_it() {
    for (written, extension) in [("c9", 9u8), ("c11", 11), ("c13", 13)] {
        let chord = ChordSymbol::parse(written).unwrap_or_else(|| panic!("`{written}` should parse"));
        assert_eq!(chord.extension, Some(extension));
        assert_eq!(chord.seventh.map(seventh_name), Some("minor"), "`{written}`");
    }
}

#[test]
fn a_symbol_outside_the_grammar_does_not_parse() {
    for written in ["h", "c4", "cmaj8", "", "c#", "cm7b5"] {
        assert!(
            ChordSymbol::parse(written).is_none(),
            "`{written}` should not parse as a chord symbol"
        );
    }
}

/// The fixture is the executable specification: every annotation in it
/// survives compilation with its position and its name.
#[test]
fn the_annotated_fixture_records_every_annotation() {
    let snapshot = snapshot_of(ANNOTATED);
    let sections: Vec<String> = snapshot
        .annotations()
        .sections()
        .iter()
        .map(|mark| format!("{} {}", mark.at.as_ratio(), mark.name))
        .collect();
    assert_eq!(sections, ["0 Exposition", "2 Development"]);

    let harmony: Vec<String> = snapshot
        .annotations()
        .harmony()
        .iter()
        .map(|mark| format!("{} {}", mark.at.as_ratio(), mark.symbol.text))
        .collect();
    assert_eq!(harmony, ["0 am", "1 fmaj7", "2 e7", "3 am"]);

    let phrases: Vec<String> = snapshot
        .annotations()
        .phrases()
        .iter()
        .map(|phrase| phrase.name.clone())
        .collect();
    assert_eq!(phrases, ["antecedent", "consequent"]);
}

/// A section is a place, and the performance layer says when that place is
/// reached — which is what a navigation pane needs and what an annotation,
/// being not-an-event, would otherwise not have.
#[test]
fn a_form_marker_lands_at_a_time_the_performance_agrees_with() {
    let snapshot = snapshot_of(ANNOTATED);
    let [_, development] = snapshot.annotations().sections() else {
        panic!("expected two sections");
    };
    let tempo = IntegratedTempoMap::new(&snapshot, &PerformanceOptions::default());
    // 1/4 = 96 and two 4/4 measures before it: five seconds in.
    let seconds = tempo.frames(development.at) / u64::from(tempo.sample_rate());
    assert_eq!(seconds, 5);
}

fn letter_of(chord: &ChordSymbol) -> &'static str {
    match chord.letter {
        musa_compiler::Letter::C => "c",
        musa_compiler::Letter::D => "d",
        musa_compiler::Letter::E => "e",
        musa_compiler::Letter::F => "f",
        musa_compiler::Letter::G => "g",
        musa_compiler::Letter::A => "a",
        musa_compiler::Letter::B => "b",
    }
}

fn accidental_of(chord: &ChordSymbol) -> &'static str {
    match chord.accidental.0 {
        -2 => "ff",
        -1 => "f",
        1 => "s",
        2 => "ss",
        _ => "",
    }
}

fn seventh_name(seventh: musa_compiler::Seventh) -> &'static str {
    match seventh {
        musa_compiler::Seventh::Minor => "minor",
        musa_compiler::Seventh::Major => "major",
        musa_compiler::Seventh::Diminished => "diminished",
    }
}
