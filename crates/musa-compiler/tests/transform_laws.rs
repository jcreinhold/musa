//! The variation transformations' contracts (docs/prompts/34).
//!
//! `stretch`, `retrograde`, and `invert` are elaboration-time functions over
//! an already-elaborated timeline, not kernel constructors (course correction
//! §29: no new primitive without semantic necessity). What earns them that
//! status is that each obeys an algebraic law relating it to the kernel's
//! own operations, and those laws are what this suite pins down:
//!
//! - `stretch 1 { x } ≡ x` — the identity factor changes nothing, including
//!   the way durations are written;
//! - `stretch f { a b } ≡ stretch f { a } stretch f { b }` — stretching
//!   distributes over sequence (over overlay it is already a kernel law,
//!   `musa-kernel/tests/laws.rs`, L13);
//! - `retrograde { retrograde { x } } ≡ x` — retrograde is an involution,
//!   ties included;
//! - `retrograde { a b } ≡ retrograde { b } retrograde { a }` — retrograde is
//!   an anti-homomorphism for sequence, which is the precise sense in which
//!   it reverses time rather than merely reordering noteheads;
//! - `invert around p { invert around p { x } } ≡ x` — inversion about a
//!   fixed axis is an involution wherever it is spellable at all.
//!
//! Specialization has no law of that kind: a `with` clause is a finite,
//! positional edit of one occurrence. Its contract is that it touches exactly
//! the note it names, in exactly the occurrence that names it, and that every
//! way of naming no such note is an error rather than a silent no-op.

// Test helpers use expect() on statically-valid inputs: a failure is a bug in
// the test itself, and panicking is the correct behavior there.
#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]

use musa_compiler::{
    CompileOptions, ExpansionStep, MusicalTime, NotatedDuration, ScoreEvent, ScoreEventKind, ScoreSnapshot, Severity,
    SourceDocument, WrittenPitch, compile,
};
use proptest::prelude::*;

const VARIATION: &str = include_str!("../../../examples/variation.musa");

/// A piece with two motifs, `a` and `b`, and `body` as its only voice.
fn piece(body: &str) -> String {
    format!(
        "piece \"transform\" {{\n\
         tempo 1/4 = 60;\n\
         meter 4/4;\n\
         key c major;\n\
         motif a() {{ c5/4 e5/4 }}\n\
         motif b() {{ g5/8 f5/4. }}\n\
         score {{ part p {{ clef treble; voice v {{ {body} }} }} }}\n\
         }}"
    )
}

fn snapshot_of(text: &str) -> ScoreSnapshot {
    let compilation = compile(&SourceDocument::new(text, "transform.musa"), &CompileOptions::default());
    let messages: Vec<String> = compilation
        .diagnostics()
        .iter()
        .map(|diagnostic| diagnostic.message.clone())
        .collect();
    compilation
        .into_snapshot()
        .unwrap_or_else(|| panic!("expected a snapshot; got {}", messages.join("; ")))
}

fn errors_of(text: &str) -> Vec<String> {
    compile(&SourceDocument::new(text, "transform.musa"), &CompileOptions::default())
        .diagnostics()
        .iter()
        .filter(|diagnostic| diagnostic.severity == Severity::Error)
        .map(|diagnostic| diagnostic.message.clone())
        .collect()
}

/// What the laws compare: what sounds, when, written how. Identity and
/// provenance are deliberately excluded — two sources that say the same
/// music in different words *should* differ in where their notes came from.
type Shape = (MusicalTime, NotatedDuration, ScoreEventKind);

fn shape(event: &ScoreEvent) -> Shape {
    (event.onset, event.notated_duration.clone(), event.kind.clone())
}

/// The first voice of the first part, as shapes in score order.
fn music(body: &str) -> Vec<Shape> {
    let snapshot = snapshot_of(&piece(body));
    snapshot
        .parts()
        .iter()
        .next()
        .and_then(|(_, part)| part.voices().map(|(_, voice)| voice).next())
        .map(|voice| voice.events().iter().map(shape).collect())
        .unwrap_or_default()
}

/// Both bodies say the same music.
fn same(left: &str, right: &str) {
    let left_music = music(left);
    assert!(!left_music.is_empty(), "the left body produced no music: {left}");
    assert_eq!(left_music, music(right), "`{left}` and `{right}` disagree");
}

// --- Stretch -----------------------------------------------------------

#[test]
fn stretching_by_one_changes_nothing() {
    same("stretch 1 { use a(); use b(); }", "use a(); use b();");
    same("stretch 1/1 { c5/4 rest/8 }", "c5/4 rest/8");
}

proptest! {
    /// Stretching a sequence is stretching each of its parts. This is the
    /// law that makes `stretch` safe to push inward or pull outward when
    /// editing, and it must hold for every factor, not the tidy ones.
    #[test]
    fn stretch_distributes_over_sequence(numerator in 1i64..8, denominator in 1i64..8) {
        let factor = format!("{numerator}/{denominator}");
        let together = format!("stretch {factor} {{ use a(); use b(); }}");
        let apart = format!("stretch {factor} {{ use a(); }} stretch {factor} {{ use b(); }}");
        prop_assert_eq!(music(&together), music(&apart));
    }
}

#[test]
fn stretching_rewrites_the_durations_it_scales() {
    // Augmentation is a notational act as much as a temporal one: a bar of
    // quarters stretched by two is written as halves, not as quarters that
    // secretly last twice as long.
    let stretched = music("stretch 2 { use a(); }");
    let spellings: Vec<&str> = stretched
        .iter()
        .map(|(_, duration, _)| duration.spelling.as_str())
        .collect();
    assert_eq!(spellings, ["1/2", "1/2"]);
    let onsets: Vec<String> = stretched.iter().map(|(onset, _, _)| onset.to_string()).collect();
    assert_eq!(onsets, ["0", "1/2"]);
}

// --- Retrograde --------------------------------------------------------

#[test]
fn retrograde_is_its_own_inverse() {
    same("retrograde { retrograde { use a(); use b(); } }", "use a(); use b();");
    // Ties are relations between notes, so reversing twice has to put them
    // back on the note that opened them.
    same("retrograde { retrograde { c5/4 ~ c5/8 e5/4 } }", "c5/4 ~ c5/8 e5/4");
}

#[test]
fn retrograde_reverses_a_sequence_rather_than_distributing_over_it() {
    same(
        "retrograde { use a(); use b(); }",
        "retrograde { use b(); } retrograde { use a(); }",
    );
}

#[test]
fn retrograde_puts_the_last_note_first() {
    let backwards = music("retrograde { c5/4 e5/8 g5/2 }");
    let forwards = music("g5/2 e5/8 c5/4");
    assert_eq!(backwards, forwards);
}

// --- Inversion ---------------------------------------------------------

proptest! {
    /// Mirroring twice about the same axis returns the original spelling —
    /// wherever the mirror lands on a note the language can write at all.
    #[test]
    fn inversion_about_a_fixed_axis_is_an_involution(
        axis in prop::sample::select(vec!["c5", "d5", "e5", "f5", "g5", "a4", "b4"]),
    ) {
        let once = format!("invert around {axis} {{ use a(); use b(); }}");
        let twice = format!("invert around {axis} {{ {once} }}");
        prop_assert!(errors_of(&piece(&once)).is_empty());
        prop_assert_eq!(music(&twice), music("use a(); use b();"));
    }
}

#[test]
fn inversion_mirrors_diatonically_and_spells_the_result() {
    // `c5 e5 g5` about `c5`: the rising third becomes a falling one, and the
    // key's own spelling is not assumed — the mirror of a major third down
    // from c is a-flat, not g-sharp.
    let mirrored = music("invert around c5 { c5/4 e5/4 g5/4 }");
    let pitches: Vec<ScoreEventKind> = mirrored.iter().map(|(_, _, kind)| kind.clone()).collect();
    let expected: Vec<ScoreEventKind> = ["c5", "ab4", "f4"]
        .into_iter()
        .map(|text| ScoreEventKind::Note {
            pitch: WrittenPitch::parse(text).expect("a pitch literal"),
        })
        .collect();
    assert_eq!(pitches, expected);
}

#[test]
fn a_mirror_image_may_produce_an_exact_triple_accidental() {
    // `b##4` mirrored about `c5` lands a diatonic step above the axis and
    // three semitones below it: D triple-flat. The integer pitch algebra keeps
    // that spelling instead of refusing or approximating it.
    let source = "invert around c5 { b##4/4 }";
    assert!(errors_of(&piece(source)).is_empty());
    let event = music(source).into_iter().next().expect("one note event");
    assert_eq!(
        event.2,
        ScoreEventKind::Note {
            pitch: WrittenPitch::parse("dbbb5").expect("a pitch literal"),
        }
    );
}

// --- Specialization ----------------------------------------------------

#[test]
fn an_override_changes_one_note_of_one_occurrence() {
    let specialized = music("use a(); use a() with { note 2 = f5; }");
    let plain = music("use a(); c5/4 f5/4");
    assert_eq!(specialized, plain);
}

#[test]
fn an_override_of_a_note_the_occurrence_does_not_have_is_an_error() {
    let errors = errors_of(&piece("use a() with { note 5 = f5; }"));
    assert!(
        errors
            .iter()
            .any(|message| message.contains("this occurrence has 2 notes")),
        "expected an out-of-range override error; got {errors:?}"
    );
}

#[test]
fn overriding_the_same_note_twice_is_an_error() {
    let errors = errors_of(&piece("use a() with { note 1 = f5; note 1 = g5; }"));
    assert!(
        errors
            .iter()
            .any(|message| message.contains("twice") || message.contains("already")),
        "expected a duplicate-override error; got {errors:?}"
    );
}

// --- Provenance --------------------------------------------------------

#[test]
fn the_variation_fixture_says_where_every_note_came_from() {
    // The point of the fixture is not the notes but the paths: a stretched
    // note knows it was stretched *and* which motif it was written in, and a
    // specialized note names the override that respelled it. Provenance is a
    // layer above the kernel, and the kernel's normalization does not erase
    // it (course correction §20).
    let snapshot = snapshot_of(VARIATION);
    let mut dump = String::new();
    for (_, part) in snapshot.parts().iter() {
        for (name, voice) in part.voices() {
            let header = format!("voice {name:?}:\n");
            dump.push_str(&header);
            for event in voice.events() {
                let sounds = match &event.kind {
                    ScoreEventKind::Note { pitch } => pitch.to_string(),
                    ScoreEventKind::Rest => "rest".to_owned(),
                    ScoreEventKind::Chord { pitches } => {
                        pitches.iter().map(ToString::to_string).collect::<Vec<_>>().join("+")
                    }
                };
                let path: Vec<String> = event
                    .origin
                    .expansion_path
                    .iter()
                    .map(|step| match step {
                        ExpansionStep::MotifApplication { .. } => "motif".to_owned(),
                        ExpansionStep::RepeatIteration(index) => format!("repeat {index}"),
                        ExpansionStep::Transposition(interval) => {
                            format!("transpose {}/{}", interval.diatonic_steps, interval.semitones)
                        }
                        ExpansionStep::Stretch(factor) => format!("stretch {factor}"),
                        ExpansionStep::Retrograde => "retrograde".to_owned(),
                        ExpansionStep::Inversion { axis } => format!("invert around {axis}"),
                        ExpansionStep::MapNotePitches => "map note pitches".to_owned(),
                        ExpansionStep::ScaleContext { scale } => format!("in {scale}"),
                        ExpansionStep::Assertion { claim } => format!("assert {claim}"),
                        ExpansionStep::Specialization { .. } => "specialized".to_owned(),
                        ExpansionStep::TemplateInstance { alias, .. } => format!("make {alias}"),
                    })
                    .collect();
                let line = format!(
                    "  {} {} {sounds} [{}]\n",
                    event.onset,
                    event.notated_duration.spelling,
                    path.join(" → ")
                );
                dump.push_str(&line);
            }
        }
    }
    insta::assert_snapshot!("variation_fixture", dump);
}

#[test]
fn an_override_cannot_name_a_rest_or_a_chord() {
    // Positions count what the score inspector counts, so a rest holds one
    // and a chord holds one — and neither is a thing a pitch can be written
    // onto. Both say so rather than skipping the position silently, which
    // would renumber every note after it.
    let with_rests = |clause: &str| {
        errors_of(&format!(
            "piece \"x\" {{ tempo 1/4 = 60; meter 4/4; key c major;\n\
             motif c() {{ rest/4 [c5 e5]/4 g5/4 }}\n\
             score {{ part p {{ voice v {{ use c() with {{ {clause} }} }} }} }} }}"
        ))
    };
    assert!(
        with_rests("note 1 = f5;")
            .iter()
            .any(|message| message.contains("rest")),
        "expected a rest-position error; got {:?}",
        with_rests("note 1 = f5;")
    );
    assert!(
        with_rests("note 2 = f5;")
            .iter()
            .any(|message| message.contains("chord")),
        "expected a chord-position error; got {:?}",
        with_rests("note 2 = f5;")
    );
    // And the note after them is still reachable at the position the score
    // shows it at.
    assert!(with_rests("note 3 = f5;").is_empty());
}
