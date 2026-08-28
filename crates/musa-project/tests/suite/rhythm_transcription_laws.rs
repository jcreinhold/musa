//! Laws for prompt 204b's rhythm-transcription report facade.
//!
//! These run through `ProjectSession::transcribe_rhythm`, never through the
//! private optimizer: the facade carries the take/policy identities, the
//! published cost-field order, and the storage/beam measurements as read-only
//! facts, and the corpus admission thresholds are asserted on the report.

#![allow(clippy::arithmetic_side_effects)]
#![allow(clippy::expect_used)]
#![allow(clippy::indexing_slicing)]
#![allow(clippy::panic)]

use musa_project::{ProjectSession, Refusal, RhythmEvent, Take, TakeClock};

const CORPUS: &str = include_str!("../fixtures/transcription/corpus.json");

#[derive(serde::Deserialize)]
struct Corpus {
    fixtures: Vec<Fixture>,
}

#[derive(serde::Deserialize)]
struct Fixture {
    id: String,
    clock: Clock,
    notes: Vec<Note>,
}

#[derive(serde::Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum Clock {
    Known { quarter_micros: u64, origin_micros: u64 },
    Free,
    Unmeasured,
}

#[derive(serde::Deserialize)]
struct Note {
    id: u32,
    onset_micros: u64,
    release_micros: u64,
    sounding_end_micros: u64,
    expected: Option<Expected>,
}

#[derive(serde::Deserialize)]
struct Expected {
    onset_ticks: u32,
    duration_ticks: u32,
}

fn session() -> ProjectSession {
    ProjectSession::from_text(
        r#"piece "Transcription laws" {
    meter 4/4;
    key c major;
    score { part p { voice v { rest/1 } } }
}
"#,
        "transcription-laws.musa",
    )
}

fn known_take(name: &str, ticks: &[u32], quarter_micros: u64, origin_micros: u64) -> Take {
    let events = ticks
        .iter()
        .copied()
        .enumerate()
        .map(|(index, tick)| {
            let onset = u64::from(tick).saturating_mul(quarter_micros) / 24;
            RhythmEvent {
                id: u32::try_from(index).unwrap_or(u32::MAX),
                onset_micros: onset,
                release_micros: onset.saturating_add(quarter_micros / 2),
                sounding_end_micros: onset.saturating_add(quarter_micros / 2),
            }
        })
        .collect();
    Take {
        name: name.to_owned(),
        clock: TakeClock::Known {
            quarter_micros,
            origin_micros,
        },
        bar_ticks: 96,
        events,
        pins: Vec::new(),
    }
}

#[test]
fn the_report_names_its_constituents_and_publishes_the_cost_order() {
    let session = session();
    let take = known_take("unit-take", &[0, 12, 24], 500_000, 0);
    let report = session.transcribe_rhythm(&take, "standard");
    assert_eq!(report.report_version(), 1);
    assert_eq!(report.take_name(), "unit-take");
    assert_eq!(report.policy_name(), "standard");
    assert_eq!(report.cost_version(), 1);
    assert_eq!(
        report.cost_fields(),
        [
            "onset-displacement",
            "duration-displacement",
            "tempo-smoothness",
            "notation-complexity",
            "rests",
            "ties",
            "tuplets",
            "syncopation-preservation",
            "user-constraints",
        ]
    );
    assert!(report.policy_error().is_none());
    assert!(report.candidates().is_some());
}

#[test]
fn a_known_clock_keeps_its_origin_and_reports_the_exact_onset() {
    let session = session();
    // A pickup starting at tick 18 with a calibrated origin at 0.
    let take = known_take("pickup", &[18, 24, 42], 520_000, 0);
    let report = session.transcribe_rhythm(&take, "standard");
    let candidates = report.candidates().expect("must rank");
    assert_eq!(candidates[0].onsets, vec![18, 24, 42]);
}

#[test]
fn reports_are_deterministic_and_ordered() {
    let session = session();
    let take = known_take("det", &[0, 12, 24, 36, 48], 500_000, 0);
    let first = session.transcribe_rhythm(&take, "standard");
    let second = session.transcribe_rhythm(&take, "standard");
    assert_eq!(first, second);
    let candidates = first.candidates().expect("must rank");
    assert!(candidates.len() <= 5);
    for pair in candidates.windows(2) {
        assert!(pair[0].cost.rank_total() <= pair[1].cost.rank_total());
    }
}

#[test]
fn a_matched_pin_is_local_and_idempotent() {
    let session = session();
    let take = known_take("pin", &[0, 12, 24, 48], 500_000, 0);
    let top = session.transcribe_rhythm(&take, "standard").candidates().expect("rank")[0]
        .onsets
        .clone();
    let before = session.transcribe_rhythm(&take, "standard");
    let mut pinned = take;
    pinned.pins = vec![(2, top[2])];
    let after = session.transcribe_rhythm(&pinned, "standard");
    assert_eq!(
        before.candidates(),
        after.candidates(),
        "pinning to the tick an event already had must not perturb the rank"
    );
}

#[test]
fn unmeasured_scope_is_a_write_source_refusal() {
    let session = session();
    let take = Take {
        name: "unmeasured".to_owned(),
        clock: TakeClock::Unmeasured,
        bar_ticks: 96,
        events: vec![RhythmEvent {
            id: 0,
            onset_micros: 0,
            release_micros: 100_000,
            sounding_end_micros: 100_000,
        }],
        pins: Vec::new(),
    };
    let report = session.transcribe_rhythm(&take, "standard");
    assert_eq!(report.refusal(), Some(Refusal::WriteSource));
}

#[test]
fn a_128_note_take_completes_within_the_published_bounds() {
    let session = session();
    let ticks = (0..128)
        .map(|index| u32::try_from(index).unwrap_or(u32::MAX).saturating_mul(12))
        .collect::<Vec<_>>();
    let take = known_take("stress", &ticks, 500_000, 0);
    let started = std::time::Instant::now();
    let report = session.transcribe_rhythm(&take, "standard");
    assert!(report.candidates().is_some(), "the stress take must rank");
    println!(
        "128-note take: {} peak states, {} peak storage bytes, {:.1} ms",
        report.peak_states(),
        report.peak_storage_bytes(),
        started.elapsed().as_secs_f64() * 1000.0
    );
    assert!(report.peak_states() <= 96);
    assert!(
        report.peak_storage_bytes() < 131_072,
        "storage {} exceeds 128 KiB",
        report.peak_storage_bytes()
    );
    // A generous CI-safe ceiling; the exact 50 ms reference number is the
    // benchmark's job.
    assert!(
        started.elapsed().as_secs_f64() < 5.0,
        "the 128-note reference must stay far below the trial's budget"
    );
}

#[test]
fn corpus_admission_thresholds_hold_through_the_facade() {
    let session = session();
    let body: Corpus = serde_json::from_str(CORPUS).expect("decode the checked-in corpus");
    let mut onset_correct = 0_usize;
    let mut onset_total = 0_usize;
    let mut top_k_recall = 0_usize;
    let mut top_k_total = 0_usize;
    let mut duration_correct = 0_usize;
    let mut duration_total = 0_usize;
    let mut edits = 0_usize;

    for fixture in &body.fixtures {
        let clock = match fixture.clock {
            Clock::Known {
                quarter_micros,
                origin_micros,
            } => TakeClock::Known {
                quarter_micros,
                origin_micros,
            },
            Clock::Free => TakeClock::Free,
            Clock::Unmeasured => continue,
        };
        let expected = fixture
            .notes
            .iter()
            .filter_map(|note| note.expected.as_ref().map(|expected| (note, expected)))
            .collect::<Vec<_>>();
        let events = expected
            .iter()
            .map(|(note, _)| RhythmEvent {
                id: note.id,
                onset_micros: note.onset_micros,
                release_micros: note.release_micros,
                sounding_end_micros: note.sounding_end_micros,
            })
            .collect::<Vec<_>>();
        let take = Take {
            name: fixture.id.clone(),
            clock,
            bar_ticks: 96,
            events,
            pins: Vec::new(),
        };
        let report = session.transcribe_rhythm(&take, "standard");
        let Some(candidates) = report.candidates() else {
            panic!("{} must rank", fixture.id);
        };
        let Some(best) = candidates.first() else {
            continue;
        };
        let intended_onsets = expected
            .iter()
            .map(|(_, expected)| expected.onset_ticks)
            .collect::<Vec<_>>();
        top_k_total = top_k_total.saturating_add(1);
        if candidates.iter().any(|candidate| candidate.onsets == intended_onsets) {
            top_k_recall = top_k_recall.saturating_add(1);
        }
        for ((_, expected), onset) in expected.iter().zip(&best.onsets) {
            onset_total = onset_total.saturating_add(1);
            if expected.onset_ticks == *onset {
                onset_correct = onset_correct.saturating_add(1);
            }
        }
        let intended_durations = expected
            .iter()
            .map(|(_, expected)| expected.duration_ticks)
            .collect::<Vec<_>>();
        for ((_, expected), duration) in expected.iter().zip(&best.durations) {
            duration_total = duration_total.saturating_add(1);
            if expected.duration_ticks == *duration {
                duration_correct = duration_correct.saturating_add(1);
            }
        }
        edits = edits.saturating_add(token_edit_distance(
            &tokens(&intended_onsets, &intended_durations),
            &tokens(&best.onsets, &best.durations),
        ));
    }

    // Reported, not only asserted: a floor tells the next reader the model did
    // not regress, and the measurement tells them how much room it has.
    println!(
        "corpus through the facade: onsets {onset_correct}/{onset_total}, top-5 {top_k_recall}/{top_k_total}, \
         key durations {duration_correct}/{duration_total}, source-token edits {edits}"
    );
    assert!(onset_total >= 58, "at least 58 expected onsets, saw {onset_total}");
    assert!(
        onset_correct >= 54,
        "exact onsets regressed: {onset_correct}/{onset_total}"
    );
    assert_eq!(top_k_total, 9);
    assert!(
        top_k_recall >= 8,
        "top-five recall regressed: {top_k_recall}/{top_k_total}"
    );
    assert!(
        duration_correct >= 47,
        "key durations regressed: {duration_correct}/{duration_total}"
    );
    assert!(edits <= 23, "source-token edits regressed: {edits}");
}

fn token_edit_distance(left: &[String], right: &[String]) -> usize {
    let mut previous = (0..=right.len()).collect::<Vec<_>>();
    for (left_index, left_token) in left.iter().enumerate() {
        let mut current = Vec::with_capacity(right.len().saturating_add(1));
        current.push(left_index.saturating_add(1));
        for (right_index, right_token) in right.iter().enumerate() {
            let insertion = current.last().copied().unwrap_or(usize::MAX).saturating_add(1);
            let deletion = previous
                .get(right_index.saturating_add(1))
                .copied()
                .unwrap_or(usize::MAX)
                .saturating_add(1);
            let replacement = previous
                .get(right_index)
                .copied()
                .unwrap_or(usize::MAX)
                .saturating_add(usize::from(left_token != right_token));
            current.push(insertion.min(deletion).min(replacement));
        }
        previous = current;
    }
    previous.last().copied().unwrap_or(0)
}

fn tokens(onsets: &[u32], durations: &[u32]) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut previous_end: Option<u32> = None;
    for (onset, duration) in onsets.iter().copied().zip(durations.iter().copied()) {
        if let Some(end) = previous_end
            && onset > end
        {
            tokens.push(format!("rest@{end}/{}", onset.saturating_sub(end)));
        }
        tokens.push(format!("note@{onset}/{duration}"));
        previous_end = Some(previous_end.map_or_else(
            || onset.saturating_add(duration),
            |end| end.max(onset.saturating_add(duration)),
        ));
    }
    tokens
}
