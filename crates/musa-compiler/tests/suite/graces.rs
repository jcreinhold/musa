//! Grace notes: the page writes them, the profile plays them.
//!
//! A grace note is the sharpest case roadmap §2 has. It has *no* written
//! duration — it is a point occurrence, start equal to end — so performance
//! cannot read a length off the page even in principle; it has to take one
//! from a neighbour, and which neighbour is a question two centuries of
//! practice answer differently. `MusicXML` puts the answer in the file.
//! musa puts it in the profile, and these tests are what "puts it in the
//! profile" means:
//!
//! - `the_page_does_not_say_how_a_grace_is_played` — two profiles, one
//!   engraving, byte for byte.
//! - `the_profile_says_how_a_grace_is_played` — the same two profiles, two
//!   performances.
//!
//! The third one that matters is `order_survives_normalization`. Every grace
//! in a group stands at one instant, and docs/rules/kernel/05 N2 orders occurrences
//! by span and then by payload key — so a span they all share settles nothing,
//! and two different pieces of music would compile to one term unless the
//! written order rides in the payload.

// A failure of these is a bug in the fixture, not in a caller's input.
#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]
#![allow(clippy::indexing_slicing)]
// Exact rational comparisons throughout; see `time.rs`.
#![allow(clippy::arithmetic_side_effects)]

use musa_compiler::{CompileOptions, SourceDocument, compile};

use musa_score::{PerformanceEvent, PerformanceOptions, ScoreSnapshot, lower_performance};

/// A piece whose only variable is what the profile says about grace notes.
///
/// Two plain quarters before the grace, so `from = previous` has something
/// behind the beat to take from and the two readings can actually differ.
fn piece(grace: &str) -> String {
    format!(
        "piece \"Leaning\" {{ tempo 1/4 = 60; meter 4/4; key c major;
            performance {{ profile band {{ {grace} }} }}
            score {{ part p {{ profile band; voice v {{
                c5/4 d5/4
                grace {{ b4 }}
                c5/2
            }} }} }} }}"
    )
}

fn score_of(text: &str) -> ScoreSnapshot {
    compile(&SourceDocument::new(text, "graces.musa"), &CompileOptions::default())
        .into_snapshot()
        .expect("compiles")
}

/// Every note-on frame with the pitch that starts there, in time order.
fn attacks(score: &ScoreSnapshot) -> Vec<(u64, String)> {
    let plan = lower_performance(score, &PerformanceOptions::default()).expect("lowers");
    let mut attacks: Vec<(u64, String)> = plan
        .lanes()
        .iter()
        .flat_map(|lane| lane.events().iter())
        .filter_map(|event| match event {
            PerformanceEvent::NoteOn { frame, note, .. } => Some((*frame, note.pitch.to_string())),
            PerformanceEvent::NoteOff { .. } | PerformanceEvent::Parameter { .. } => None,
        })
        .collect();
    attacks.sort();
    attacks
}

const ON_THE_BEAT: &str = "grace { steal = 1/8; from = principal; }";
const AHEAD_OF_IT: &str = "grace { steal = 1/8; from = previous; }";

// The other half of the pair — one engraving from two profiles — lives in
// `musa-notation`'s `graces.rs`, because the dependency runs that way and a
// notation backend is what has to be shown not to care.

#[test]
fn the_profile_says_how_a_grace_is_played() {
    let on_the_beat = attacks(&score_of(&piece(ON_THE_BEAT)));
    let ahead_of_it = attacks(&score_of(&piece(AHEAD_OF_IT)));
    assert_ne!(on_the_beat, ahead_of_it);
    // Concretely: at 60bpm a quarter is one second, so the principal's written
    // onset is two seconds in. Stealing from the principal moves *it* and
    // leaves the grace on the beat; stealing from the previous note moves the
    // *grace* and leaves the principal alone.
    let quarter = u64::from(PerformanceOptions::default().sample_rate);
    assert!(on_the_beat.contains(&(2 * quarter, "b4".to_owned())), "{on_the_beat:?}");
    assert!(ahead_of_it.contains(&(2 * quarter, "c5".to_owned())), "{ahead_of_it:?}");
    // And each reading leaves the other note off the beat by the stolen 1/8.
    let eighth = quarter / 2;
    assert!(
        on_the_beat.contains(&(2 * quarter + eighth, "c5".to_owned())),
        "{on_the_beat:?}"
    );
    assert!(
        ahead_of_it.contains(&(2 * quarter - eighth, "b4".to_owned())),
        "{ahead_of_it:?}"
    );
}

#[test]
fn a_grace_with_nothing_behind_it_takes_from_the_note_it_leans_on() {
    // The first note of a voice: `from = previous` has nothing to take from,
    // and the alternative to falling back is a performance that starts before
    // the piece does.
    let score = score_of(
        "piece \"First\" { tempo 1/4 = 60; meter 4/4; key c major;
            performance { profile band { grace { steal = 1/8; from = previous; } } }
            score { part p { profile band; voice v {
                grace { b4 }
                c5/1
            } } } }",
    );
    let attacks = attacks(&score);
    let eighth = u64::from(PerformanceOptions::default().sample_rate) / 2;
    assert_eq!(attacks[0], (0, "b4".to_owned()), "{attacks:?}");
    assert_eq!(attacks[1], (eighth, "c5".to_owned()), "{attacks:?}");
}

#[test]
fn a_grace_cannot_swallow_the_note_it_leans_on() {
    // A profile asking for more than the principal has gets equal shares of
    // half of it. The bound is not politeness: without it the principal's
    // note-off precedes its note-on and the plan is not a performance.
    let score = score_of(
        "piece \"Greedy\" { tempo 1/4 = 60; meter 4/4; key c major;
            performance { profile band { grace { steal = 1/1; from = principal; } } }
            score { part p { profile band; voice v {
                grace { b4 c5 }
                d5/4
            } } } }",
    );
    let plan = lower_performance(&score, &PerformanceOptions::default()).expect("lowers");
    let mut open: std::collections::HashMap<u32, u64> = std::collections::HashMap::new();
    for event in plan.lanes()[0].events() {
        match event {
            PerformanceEvent::NoteOn { frame, instance, .. } => {
                open.insert(instance.0, *frame);
            }
            PerformanceEvent::NoteOff { frame, instance } => {
                let on = open.remove(&instance.0).expect("an off follows its on");
                assert!(*frame >= on, "note {} ends at {frame} and starts at {on}", instance.0);
            }
            PerformanceEvent::Parameter { .. } => {}
        }
    }
}

#[test]
fn order_survives_normalization() {
    // The two sources are the same length on purpose: their occurrences carry
    // source spans, so equal-length sources make the *order* the only thing
    // that can differ.
    let piece = |graces: &str| {
        format!(
            "piece \"Order\" {{ tempo 1/4 = 60; meter 4/4; key c major;
                score {{ part p {{ voice v {{ grace {{ {graces} }} c5/1 }} }} }} }}"
        )
    };
    let kernel = |graces: &str| {
        musa_compiler::kernel_text(
            &SourceDocument::new(piece(graces), "order.musa"),
            &musa_score::Realization::default(),
            &musa_compiler::ImportSources::default(),
        )
        .expect("projects")
    };
    assert_ne!(kernel("d5 e5"), kernel("e5 d5"));
}

#[test]
fn a_grace_note_is_not_an_event_in_the_measure() {
    // It has no written duration, so it occupies no time on the page: the bar
    // it stands in is full without it, and the notated duration of the note it
    // leans on is what the source wrote.
    let score = score_of(&piece(ON_THE_BEAT));
    let part = score.parts().iter().next().expect("one part").1;
    let voice = part.voices().next().expect("one voice").1;
    assert_eq!(voice.events().len(), 3, "a grace is not an event");
    let principal = voice.events().last().expect("three events");
    assert_eq!(
        principal.notated_duration.value.as_ratio(),
        num_rational::Ratio::new(1, 2)
    );
    assert_eq!(score.annotations().graces().len(), 1);
}

#[test]
fn a_grace_with_no_note_to_lean_on_is_reported() {
    let result = compile(
        &SourceDocument::new(
            "piece \"Orphan\" { tempo 1/4 = 60; meter 4/4; key c major;
                score { part p { voice v { c5/1 grace { b4 } } } } }",
            "orphan.musa",
        ),
        &CompileOptions::default(),
    );
    let said: String = result.diagnostics().iter().map(|d| d.message.clone()).collect();
    assert!(said.contains("no note to lean on"), "{said}");
}
