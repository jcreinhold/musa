//! Written freedom that is not a count: mobile form, free duration, and an
//! improvised frame (prompt 68, `docs/kernel/11-realization.md`).
//!
//! Each of the three is checked twice — once for what it *sounds*, because a
//! freedom that produces no music is a comment, and once for what it *says*,
//! because a freedom the page cannot print is a freedom the performer never
//! learns about. Prompt 58's rule, three times over.

// A failure of these is a bug in the fixture, not in a caller's input.
#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]

use musa_compiler::{
    Compilation, CompileOptions, Decision, OpenKind, Realization, ScoreSnapshot, SourceDocument, compile,
};

const IN_C: &str = include_str!("../../../examples/in-c.musa");

fn under(source: &str, seed: u64) -> Compilation {
    compile(
        &SourceDocument::new(source, "open"),
        &CompileOptions {
            realization: Realization::seeded(seed),
            ..CompileOptions::default()
        },
    )
}

fn snapshot_of(source: &str, seed: u64) -> ScoreSnapshot {
    let compiled = under(source, seed);
    assert!(!compiled.has_errors(), "{:?}", compiled.diagnostics());
    compiled.into_snapshot().expect("a piece that compiles has a score")
}

/// The pitches a piece sounds, in time order, across every voice.
fn pitches(snapshot: &ScoreSnapshot) -> Vec<String> {
    let mut events: Vec<(num_rational::Ratio<i64>, String)> = Vec::new();
    for (_, part) in snapshot.parts().iter() {
        for (_, voice) in part.voices() {
            for event in voice.events() {
                events.push((event.onset.as_ratio(), format!("{:?}", event.kind)));
            }
        }
    }
    events.sort_by_key(|(onset, _)| *onset);
    events.into_iter().map(|(_, kind)| kind).collect()
}

fn mobile(fragments: &str, names: &str) -> String {
    format!(
        "piece \"M\" {{ tempo 1/4 = 60; meter 4/4; key c major; {fragments} score {{ part p {{ voice v {{ mobile {{ {names} }} }} }} }} }}"
    )
}

const THREE: &str = "fragment alpha { c5 1; } fragment beta { d5 1; } fragment gamma { e5 1; }";

/// A mobile plays all of its material, once each: what is chosen is the order
/// and nothing else.
#[test]
fn a_mobile_plays_every_fragment_once() {
    for seed in 0..32u64 {
        let snapshot = snapshot_of(&mobile(THREE, "alpha; beta; gamma;"), seed);
        let mut played = pitches(&snapshot);
        assert_eq!(played.len(), 3, "seed {seed}");
        played.sort();
        let mut expected = pitches(&snapshot_of(&mobile(THREE, "alpha; beta; gamma;"), 0));
        expected.sort();
        assert_eq!(played, expected, "seed {seed}: a fragment was dropped or doubled");
    }
}

/// R1 for an order: the same reading twice, and more than one reading across
/// seeds. Without the second half the mobile would be a fixed sequence with a
/// keyword in front of it.
#[test]
fn the_seed_decides_the_order_and_decides_it_once() {
    let source = mobile(THREE, "alpha; beta; gamma;");
    assert_eq!(under(&source, 5).identity(), under(&source, 5).identity());
    let orders: std::collections::BTreeSet<Vec<u32>> = (0..64u64)
        .filter_map(|seed| {
            under(&source, seed)
                .decisions()
                .iter()
                .find_map(|(_, decision)| match decision {
                    Decision::Order(order) => Some(order.clone()),
                    Decision::Count(_) | Decision::Duration(_) => None,
                })
        })
        .collect();
    assert!(orders.len() > 1, "64 seeds produced one order");
    for order in &orders {
        let mut sorted = order.clone();
        sorted.sort_unstable();
        assert_eq!(sorted, vec![0, 1, 2], "{order:?} is not a permutation");
    }
}

/// The page has to print the instruction, not only the reading — otherwise the
/// performer is handed one arrangement and never told it was a choice.
#[test]
fn a_mobile_says_on_the_page_that_it_is_one() {
    let snapshot = snapshot_of(&mobile(THREE, "alpha; beta; gamma;"), 3);
    let [region] = snapshot.annotations().open() else {
        panic!("expected one open region: {:?}", snapshot.annotations().open());
    };
    let OpenKind::Mobile { fragments, order } = &region.kind else {
        panic!("expected a mobile: {:?}", region.kind);
    };
    assert_eq!(fragments, &["alpha", "beta", "gamma"]);
    assert_eq!(order.len(), 3);
    assert_eq!(region.start.as_ratio(), num_rational::Ratio::ZERO);
    assert_eq!(region.end.as_ratio(), num_rational::Ratio::from_integer(3));
}

/// One fragment in any order is the fragment. The refusal is the useful part:
/// a `mobile` with one name is almost always a half-finished edit.
#[test]
fn a_mobile_of_one_fragment_is_refused() {
    let compiled = under(&mobile(THREE, "alpha;"), 1);
    assert!(
        compiled
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.message.contains("at least two fragments")),
        "{:?}",
        compiled.diagnostics()
    );
}

/// A motif takes arguments and a bar is a measure; neither is material a
/// performance is invited to shuffle, and saying so is cheaper than the
/// mistake.
#[test]
fn a_mobile_cannot_arrange_a_motif() {
    let source = mobile("motif alpha() { c5 1; } fragment beta { d5 1; }", "alpha; beta;");
    let compiled = under(&source, 1);
    assert!(
        compiled
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.message.contains("is a motif, not a fragment")),
        "{:?}",
        compiled.diagnostics()
    );
}

/// Roadmap §2's row, with both values present: the symbol keeps the written
/// value, the timeline gets the decided one.
#[test]
fn a_held_note_is_drawn_at_its_written_value_and_sounds_the_decision() {
    let source = "piece \"H\" { tempo 1/4 = 60; meter 4/4; key c major;
        score { part p { voice v { c5 1/4 to 2/1; } } } }";
    for seed in 0..32u64 {
        let snapshot = snapshot_of(source, seed);
        let events: Vec<_> = snapshot
            .parts()
            .iter()
            .flat_map(|(_, part)| part.voices())
            .flat_map(|(_, voice)| voice.events())
            .cloned()
            .collect();
        let [event] = events.as_slice() else {
            panic!("expected one event");
        };
        let free = event.free.as_ref().expect("a held note carries its bounds");
        assert_eq!(free.least.as_ratio(), num_rational::Ratio::new(1, 4));
        assert_eq!(free.most.as_ratio(), num_rational::Ratio::from_integer(2));
        // What the engraver draws is what was written.
        assert_eq!(event.notated_duration.spelling, "1/4");
        // What the timeline holds is what was decided, and it is in range.
        let sounds = event.notated_duration.value.as_ratio();
        assert!(
            (free.least.as_ratio()..=free.most.as_ratio()).contains(&sounds),
            "seed {seed}: {sounds} is outside the range"
        );
    }
}

/// A range that counts downwards is a mistake about the music, and is refused
/// rather than silently swapped.
#[test]
fn a_backwards_hold_is_refused() {
    let source = "piece \"H\" { tempo 1/4 = 60; meter 4/4; key c major;
        score { part p { voice v { c5 2/1 to 1/4; } } } }";
    let compiled = under(source, 1);
    assert!(
        compiled
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.message.contains("counts upwards")),
        "{:?}",
        compiled.diagnostics()
    );
}

/// An improvised frame occupies its length so everything after it lands where
/// it should, and it sounds nothing, because musa does not invent notes.
#[test]
fn an_improvised_frame_takes_its_time_and_plays_no_notes() {
    let source = "piece \"I\" { tempo 1/4 = 60; meter 4/4; key c major;
        score { part p { voice v { improvise 2/1 over \"Dm7 | G7\"; c5 1; } } } }";
    let snapshot = snapshot_of(source, 1);
    let onsets: Vec<num_rational::Ratio<i64>> = snapshot
        .parts()
        .iter()
        .flat_map(|(_, part)| part.voices())
        .flat_map(|(_, voice)| voice.events())
        .map(|event| event.onset.as_ratio())
        .collect();
    assert_eq!(onsets, [num_rational::Ratio::from_integer(2)], "the frame sounded");
    let [region] = snapshot.annotations().open() else {
        panic!("expected one open region");
    };
    assert_eq!(
        region.kind,
        OpenKind::Improvise {
            over: Some("Dm7 | G7".to_owned())
        }
    );
    assert_eq!(region.start.as_ratio(), num_rational::Ratio::ZERO);
    assert_eq!(region.end.as_ratio(), num_rational::Ratio::from_integer(2));
}

/// Path identity at the scale that would break an ordinal-based one: fifty-three
/// decision sites, and an edit to one of them.
///
/// A site inside a fragment is named by the fragment, so rewriting the notes of
/// the fortieth figure leaves the other fifty-two decisions exactly as they
/// were. This is the test prompt 68 said *In C* had to be written for.
#[test]
fn editing_one_figure_leaves_the_other_fifty_two_decisions_alone() {
    let before = under(IN_C, 42);
    assert!(!before.has_errors(), "{:?}", before.diagnostics());
    assert_eq!(
        before.decisions().len(),
        53,
        "one per figure — the pulse repeats a fixed number of times and decides nothing"
    );

    let edited = IN_C.replace(
        "fragment figure_forty {",
        "fragment figure_forty { g5 1/4; g5 1/4; g5 1/4; g5 1/4;",
    );
    assert_ne!(edited, IN_C, "the edit did not apply");
    let after = under(&edited, 42);
    assert!(!after.has_errors(), "{:?}", after.diagnostics());
    assert_eq!(
        before.decisions(),
        after.decisions(),
        "editing one figure re-rolled another"
    );
    assert_ne!(before.identity(), after.identity(), "the edit changed no music");
}
