//! The expressive notation layer's contracts (docs/prompts/27).
//!
//! Two laws carry the semantics, and both are properties rather than
//! examples because both are claims about *every* ratio and *every* pair of
//! written values:
//!
//! - a tuplet redistributes time without creating or destroying it: `n/d` of
//!   a written value spans exactly `d/n` times what those values would have
//!   spanned written plainly;
//! - a tie merges two statements into one sounding event whose span is their
//!   sum and whose provenance is the first one's — the composer wrote one
//!   note, and it starts where the first notehead does.

// Test helpers use expect() on statically-valid inputs: a failure is a bug in
// the test itself, and panicking is the correct behavior there.
#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]
// Rational sums over exact musical time (see musa-compiler/src/time.rs).
#![allow(clippy::arithmetic_side_effects)]

use musa_compiler::{
    ArticulationMark, CompileOptions, DynamicMark, MusicalDuration, ScoreEvent, ScoreSnapshot, SourceDocument, compile,
};
use num_rational::Ratio;
use proptest::prelude::*;

const TUPLET_FIXTURE: &str = include_str!("../../../examples/tuplet-fixture.musa");

fn score_of(text: &str) -> ScoreSnapshot {
    let compilation = compile(&SourceDocument::new(text, "test.musa"), &CompileOptions::default());
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
    compile(&SourceDocument::new(text, "test.musa"), &CompileOptions::default())
        .diagnostics()
        .iter()
        .filter(|diagnostic| diagnostic.severity == musa_compiler::Severity::Error)
        .map(|diagnostic| diagnostic.message.clone())
        .collect()
}

/// The first voice of the first part, in source order.
fn first_voice(snapshot: &ScoreSnapshot) -> Vec<ScoreEvent> {
    snapshot
        .parts()
        .iter()
        .next()
        .and_then(|(_, part)| part.voices().map(|(_, voice)| voice).next())
        .map(|voice| voice.events().to_vec())
        .unwrap_or_default()
}

fn voice_source(body: &str) -> String {
    format!("piece \"x\" {{ meter 4/4; score {{ part p {{ voice v {{ {body} }} }} }} }}")
}

// --- Ties ------------------------------------------------------------------

#[test]
fn a_tie_makes_one_event_spelled_as_two_noteheads() {
    let snapshot = score_of(&voice_source("c4 1/4 ~; c4 1/8; rest 5/8;"));
    let events = first_voice(&snapshot);
    assert_eq!(events.len(), 2, "the tied pair is one sounding event, plus the rest");
    let tied = events.first().expect("tied event");
    assert_eq!(tied.notated_duration.value.as_ratio(), Ratio::new(3, 8));
    assert_eq!(
        tied.notated_duration.pieces,
        vec![
            MusicalDuration::new(Ratio::new(1, 4)),
            MusicalDuration::new(Ratio::new(1, 8)),
        ],
        "the composer asked for two noteheads, not a dotted quarter"
    );
    assert_eq!(tied.notated_duration.spelling, "1/4 ~ 1/8");
}

#[test]
fn a_tie_to_a_different_pitch_is_an_error() {
    let errors = errors_of(&voice_source("c4 1/4 ~; d4 1/4; rest 1/2;"));
    assert_eq!(
        errors,
        vec!["a tie must be followed by the same pitch or chord".to_string()]
    );
}

#[test]
fn a_tie_with_nothing_after_it_is_an_error() {
    let errors = errors_of(&voice_source("c4 1;"));
    assert!(errors.is_empty(), "control: an untied whole note is fine");
    let errors = errors_of(&voice_source("c4 1 ~;"));
    assert_eq!(errors, vec!["this tie has no note after it".to_string()]);
}

// --- Tuplets ---------------------------------------------------------------

#[test]
fn a_triplet_eighth_is_exactly_one_twelfth() {
    let snapshot = score_of(&voice_source("tuplet 3/2 { c4 1/8; d4 1/8; e4 1/8; } rest 3/4;"));
    let events = first_voice(&snapshot);
    for event in events.iter().take(3) {
        assert_eq!(event.notated_duration.value.as_ratio(), Ratio::new(1, 12));
    }
    let tuplets = snapshot.annotations().tuplets();
    assert_eq!(tuplets.len(), 1);
    let tuplet = tuplets.first().expect("tuplet");
    assert_eq!((tuplet.num, tuplet.den), (3, 2));
    assert_eq!(tuplet.from.0, 0);
    assert_eq!(tuplet.to.0, 2);
}

#[test]
fn a_tuplet_across_a_barline_is_an_error() {
    let errors = errors_of(&voice_source("rest 7/8; tuplet 3/2 { c4 1/4; d4 1/4; e4 1/4; }"));
    assert!(
        errors.contains(&"a tuplet must fit inside one measure".to_string()),
        "got {errors:?}"
    );
}

// --- Slurs, dynamics, articulations ---------------------------------------

#[test]
fn a_slur_names_the_events_it_joins() {
    let snapshot = score_of(&voice_source("slur { c4 1/4; d4 1/4; e4 1/4; } f4 1/4;"));
    let slurs = snapshot.annotations().slurs();
    assert_eq!(slurs.len(), 1);
    let slur = slurs.first().expect("slur");
    assert_eq!((slur.from.0, slur.to.0), (0, 2), "the slurred run, not the whole voice");
}

#[test]
fn a_dynamic_belongs_to_the_note_after_it() {
    let snapshot = score_of(&voice_source("c4 1/4; dynamic mf; d4 1/4; rest 1/2;"));
    let dynamics = snapshot.annotations().dynamics();
    assert_eq!(dynamics.len(), 1);
    let dynamic = dynamics.first().expect("dynamic");
    assert_eq!(dynamic.mark, DynamicMark::Mf);
    assert_eq!(dynamic.at.0, 1, "the second event, not the first");
}

#[test]
fn a_dynamic_reaches_into_the_block_that_follows_it() {
    let snapshot = score_of(&voice_source("dynamic p; repeat 2 { c4 1/4; d4 1/4; }"));
    let dynamics = snapshot.annotations().dynamics();
    assert_eq!(dynamics.len(), 1);
    assert_eq!(dynamics.first().expect("dynamic").at.0, 0);
}

#[test]
fn a_dynamic_with_nothing_after_it_is_an_error() {
    let errors = errors_of(&voice_source("c4 1; dynamic p;"));
    assert_eq!(errors, vec!["this dynamic marking has no note after it".to_string()]);
}

#[test]
fn articulations_keep_their_written_order_and_reject_unknown_names() {
    let snapshot = score_of(&voice_source("c4 1 accent staccato;"));
    let marks: Vec<ArticulationMark> = snapshot
        .annotations()
        .articulations()
        .iter()
        .map(|articulation| articulation.mark)
        .collect();
    assert_eq!(marks, vec![ArticulationMark::Accent, ArticulationMark::Staccato]);
    let errors = errors_of(&voice_source("c4 1 sideways;"));
    assert_eq!(errors, vec!["unknown articulation `sideways`".to_string()]);
}

#[test]
fn an_expanded_motif_carries_its_marks_once_per_use() {
    let source = "piece \"x\" { meter 4/4; motif m() { slur { c4 1/4 accent; d4 1/4; } } \
                  score { part p { voice v { use m(); use m(); } } } }";
    let snapshot = score_of(source);
    assert_eq!(snapshot.annotations().slurs().len(), 2, "one slur per expansion");
    assert_eq!(snapshot.annotations().articulations().len(), 2);
    let slurs = snapshot.annotations().slurs();
    let spans: Vec<(u64, u64)> = slurs.iter().map(|slur| (slur.from.0, slur.to.0)).collect();
    assert_eq!(spans, vec![(0, 1), (2, 3)]);
}

// --- The fixture -----------------------------------------------------------

#[test]
fn the_fixture_compiles_and_carries_every_annotation_kind() {
    let snapshot = score_of(TUPLET_FIXTURE);
    let annotations = &snapshot.annotations();
    assert_eq!(annotations.tuplets().len(), 2);
    assert_eq!(annotations.slurs().len(), 3);
    assert_eq!(annotations.dynamics().len(), 3);
    // accent, staccato, marcato, the cello's tenuto, and one more from the
    // single `use turn(a4)` expansion.
    assert_eq!(annotations.articulations().len(), 5);
    insta::assert_snapshot!("tuplet_fixture", format!("{snapshot:#?}"));
}

// --- Laws ------------------------------------------------------------------

/// A tuplet ratio worth writing: `n` values in the time of `d`, with `n > d`
/// so the notes are compressed, and small enough to stay inside one bar.
fn tuplet_ratio() -> impl Strategy<Value = (u32, u32)> {
    (2u32..=9, 1u32..=8).prop_filter("n in the time of d, compressed", |(num, den)| num > den)
}

/// A written value the group can be built from, as `1/2^k`.
fn written_denominator() -> impl Strategy<Value = u32> {
    prop::sample::select(vec![8u32, 16, 32])
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(96))]

    /// Tuplet elaboration sums to the notated span: `n` written `1/w` values
    /// under `n/d` last exactly `d/w`.
    #[test]
    fn tuplet_elaboration_sums_to_the_notated_span((num, den) in tuplet_ratio(), w in written_denominator()) {
        let body = format!("c4 1/{w}; ").repeat(num as usize);
        let expected = Ratio::new(i64::from(den), i64::from(w));
        // Padded to a whole number of measures so the only diagnostic that
        // could fire is the one this law is about.
        let source = format!(
            "piece \"x\" {{ meter 4/4; score {{ part p {{ voice v {{ tuplet {num}/{den} {{ {body}}} rest {}/{}; }} }} }} }}",
            i64::from(w) - i64::from(den),
            w,
        );
        let snapshot = score_of(&source);
        let events = first_voice(&snapshot);
        let sounded: Ratio<i64> = events
            .iter()
            .take(num as usize)
            .map(|event| event.notated_duration.value.as_ratio())
            .sum();
        prop_assert_eq!(sounded, expected);
    }

    /// Tie merging preserves total span and provenance: the merged event
    /// starts where the first notehead did, lasts as long as both, and
    /// explains itself with the first statement's origin.
    #[test]
    fn tie_merging_preserves_span_and_provenance(first in 1u32..=8, second in 1u32..=8) {
        let tied = voice_source(&format!("c4 1/{first} ~; c4 1/{second};"));
        let apart = voice_source(&format!("c4 1/{first}; c4 1/{second};"));
        let merged = first_voice(&score_of(&tied));
        let separate = first_voice(&score_of(&apart));
        prop_assert_eq!(merged.len(), 1);
        prop_assert_eq!(separate.len(), 2);
        let (Some(merged), Some(head), Some(tail)) = (merged.first(), separate.first(), separate.get(1)) else {
            return Err(TestCaseError::fail("missing events"));
        };
        prop_assert_eq!(merged.onset, head.onset);
        prop_assert_eq!(
            merged.notated_duration.value.as_ratio(),
            head.notated_duration.value.as_ratio() + tail.notated_duration.value.as_ratio()
        );
        // The merged event points at the first statement. Its span is wider by
        // the `~` token the untied source does not have, so the comparable
        // facts are where the provenance starts and what declared it.
        prop_assert_eq!(merged.origin.source_span.start, head.origin.source_span.start);
        prop_assert_eq!(merged.origin.declaration, head.origin.declaration);
        prop_assert_eq!(&merged.origin.expansion_path, &head.origin.expansion_path);
        prop_assert_eq!(
            merged.notated_duration.pieces.clone(),
            vec![head.notated_duration.value, tail.notated_duration.value]
        );
    }
}
