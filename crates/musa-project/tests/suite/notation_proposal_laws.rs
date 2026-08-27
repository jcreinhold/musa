//! Laws for prompt 205c's checked notation proposal facade.
//!
//! Every assertion here runs through `ProjectSession::propose_notation`, never
//! through the private stages: the proposal names its take/revision/policy,
//! composes voices, spellings, and written ends, derives each note to captured
//! event ids, and ships a source preview only after it has been parsed and
//! compiled. The corpus admission thresholds are asserted on the composed
//! proposal, with each note aligned to the corpus through its derivation.

#![allow(clippy::arithmetic_side_effects)]
#![allow(clippy::expect_used)]
#![allow(clippy::indexing_slicing)]
#![allow(clippy::panic)]

use std::collections::{BTreeSet, HashMap};

use musa_playback::{MidiClockCalibrator, MidiInputEvent, MidiMessageKind};
use musa_project::{CapturedMidiEvent, MidiPairingFact, ProjectSession, ProposalLoss, ProposalNote, TakeClock};

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
    pitch: u8,
    onset_micros: u64,
    release_micros: u64,
    sounding_end_micros: u64,
    expected: Option<Expected>,
}

#[derive(Clone, Copy, serde::Deserialize)]
struct Expected {
    duration_ticks: u32,
    voice: u8,
    group: u16,
}

fn session() -> ProjectSession {
    ProjectSession::from_text(
        r#"piece "Proposal laws" {
    meter 4/4;
    key c major;
    score { part p { voice v { rest/1 } } }
}
"#,
        "proposal-laws.musa",
    )
}

fn raw(kind: MidiMessageKind, channel: u8, data: u8, value: i16, micros: u64) -> MidiInputEvent {
    MidiInputEvent {
        device_micros: micros,
        callback_micros: micros,
        cable: 0,
        channel,
        kind,
        data,
        value,
    }
}

fn captured(id: u64, kind: MidiMessageKind, data: u8, value: i16, micros: u64) -> CapturedMidiEvent {
    let raw = raw(kind, 0, data, value, micros);
    CapturedMidiEvent {
        id,
        raw,
        project_micros: micros,
        calibration: MidiClockCalibrator::default().calibration(),
        voice: None,
        pairing: MidiPairingFact::NotANote,
    }
}

/// Synthesize the raw MIDI of one corpus fixture's *expected* notes, in a time
/// order the pairing stage admits. Pedal-extended notes get one sustain pedal
/// held from the first press to the last release, so `complete()` reports the
/// key release and the pedal-extended sounding end as separate facts exactly as
/// the corpus does.
fn synthesize(fixture: &Fixture) -> Vec<CapturedMidiEvent> {
    let expected = fixture
        .notes
        .iter()
        .enumerate()
        .filter_map(|(index, note)| note.expected.map(|_| (index, note)))
        .collect::<Vec<_>>();

    let mut events = Vec::new();
    let mut pedal_press: Option<u64> = None;
    let mut pedal_release: u64 = 0;
    for (frame, (_, note)) in expected.iter().enumerate() {
        events.push(captured(
            u64::try_from(frame * 2).unwrap_or(u64::MAX),
            MidiMessageKind::NoteOn,
            note.pitch,
            90,
            note.onset_micros,
        ));
        events.push(captured(
            u64::try_from(frame * 2 + 1).unwrap_or(u64::MAX),
            MidiMessageKind::NoteOff,
            note.pitch,
            0,
            note.release_micros,
        ));
        if note.sounding_end_micros > note.release_micros {
            pedal_press = Some(pedal_press.map_or(note.onset_micros, |p| p.min(note.onset_micros)));
            pedal_release = pedal_release.max(note.sounding_end_micros);
        }
    }
    if let Some(press) = pedal_press {
        events.push(captured(1_000_000, MidiMessageKind::ControlChange, 64, 127, press));
        events.push(captured(
            1_000_001,
            MidiMessageKind::ControlChange,
            64,
            0,
            pedal_release,
        ));
    }
    events.sort_by_key(|event| (event.project_micros, event.id));
    events
}

fn clock_of(clock: &Clock) -> TakeClock {
    match *clock {
        Clock::Known {
            quarter_micros,
            origin_micros,
        } => TakeClock::Known {
            quarter_micros,
            origin_micros,
        },
        Clock::Free => TakeClock::Free,
        Clock::Unmeasured => TakeClock::Unmeasured,
    }
}

/// Map a proposal note back to its corpus fixture note via the derivation.
fn frame_of(note: &ProposalNote) -> usize {
    usize::try_from(note.derivation[0] / 2).unwrap_or(usize::MAX)
}

#[test]
fn corpus_admission_thresholds_hold_through_the_proposal_facade() {
    let session = session();
    let body: Corpus = serde_json::from_str(CORPUS).expect("decode the checked-in corpus");
    let mut voice_correct = 0_usize;
    let mut voice_total = 0_usize;
    let mut grouping_correct = 0_usize;
    let mut grouping_total = 0_usize;
    let mut duration_correct = 0_usize;
    let mut duration_total = 0_usize;

    for fixture in &body.fixtures {
        if matches!(fixture.clock, Clock::Unmeasured) {
            continue;
        }
        let expected = fixture
            .notes
            .iter()
            .filter_map(|note| note.expected)
            .collect::<Vec<_>>();
        let expected_pairs = fixture
            .notes
            .iter()
            .filter_map(|note| note.expected.map(|e| (note, e)))
            .collect::<Vec<_>>();
        if expected.is_empty() {
            continue;
        }
        let proposal = session
            .propose_notation(
                &fixture.id,
                &synthesize(fixture),
                clock_of(&fixture.clock),
                96,
                "4/4",
                None,
                "standard",
                &[],
            )
            .unwrap_or_else(|error| panic!("{}: {:?}", fixture.id, error));

        // Align each proposal note to the corpus through its derivation, then
        // score voice labels under the best one-to-one permutation.
        let predicted = proposal.notes().iter().map(|note| note.voice).collect::<Vec<_>>();
        let intended = proposal
            .notes()
            .iter()
            .map(|note| expected[frame_of(note)].voice)
            .collect::<Vec<_>>();
        voice_correct = voice_correct.saturating_add(best_label_matches(&predicted, &intended));
        voice_total = voice_total.saturating_add(intended.len());

        // Grouping pairs over aligned notes, membership read from the
        // proposal's own group list rather than a private per-note channel.
        let group_of = proposal
            .groups()
            .iter()
            .enumerate()
            .flat_map(|(group, members)| members.notes.iter().map(move |note| (*note, group)))
            .collect::<HashMap<_, _>>();
        for left in 0..proposal.notes().len() {
            for right in left.saturating_add(1)..proposal.notes().len() {
                let expected_same = expected[frame_of(&proposal.notes()[left])].group
                    == expected[frame_of(&proposal.notes()[right])].group;
                let observed_same = group_of[&left] == group_of[&right];
                grouping_total = grouping_total.saturating_add(1);
                if expected_same == observed_same {
                    grouping_correct = grouping_correct.saturating_add(1);
                }
            }
        }

        // Written ends and the pedal fact: pedal-extended sound is a reported
        // fact beside the key duration, never scored as a longer written end.
        // Align by derivation frame, never by output order.
        for proposal_note in proposal.notes() {
            let frame = frame_of(proposal_note);
            let Some(&(corpus_note, e)) = expected_pairs.get(frame) else {
                continue;
            };
            duration_total = duration_total.saturating_add(1);
            if proposal_note.written_end_ticks == e.duration_ticks {
                duration_correct = duration_correct.saturating_add(1);
            }
            if corpus_note.sounding_end_micros > corpus_note.release_micros {
                assert!(
                    proposal_note.pedal_extended,
                    "{}: pedal-extended sound must be reported",
                    fixture.id
                );
            }
        }
    }

    assert!(
        voice_correct >= 54 && voice_total == 58,
        "voice labels {voice_correct}/{voice_total}"
    );
    assert!(
        grouping_correct >= 166 && grouping_total == 166,
        "grouping pairs {grouping_correct}/{grouping_total}"
    );
    assert!(
        duration_correct >= 47 && duration_total == 58,
        "key-release durations {duration_correct}/{duration_total}"
    );
}

#[test]
fn a_binary_melody_previews_and_compiles_with_exact_round_trip() {
    let session = session();
    let events = [
        captured(0, MidiMessageKind::NoteOn, 60, 90, 0),
        captured(1, MidiMessageKind::NoteOff, 60, 0, 500_000),
        captured(2, MidiMessageKind::NoteOn, 62, 90, 505_000),
        captured(3, MidiMessageKind::NoteOff, 62, 0, 1_005_000),
        captured(4, MidiMessageKind::NoteOn, 64, 90, 993_000),
        captured(5, MidiMessageKind::NoteOff, 64, 0, 1_493_000),
        captured(6, MidiMessageKind::NoteOn, 65, 90, 1_508_000),
        captured(7, MidiMessageKind::NoteOff, 65, 0, 2_008_000),
    ];
    let proposal = session
        .propose_notation(
            "melody",
            &events,
            TakeClock::Known {
                quarter_micros: 500_000,
                origin_micros: 0,
            },
            96,
            "4/4",
            None,
            "standard",
            &[],
        )
        .expect("a straight melody must propose");
    assert_eq!(proposal.voice_count(), 1);
    assert_eq!(
        proposal
            .notes()
            .iter()
            .map(|note| note.pitch.as_str())
            .collect::<Vec<_>>(),
        ["c4", "d4", "e4", "f4"]
    );
    let source = proposal.source().expect("binary durations are spellable here");
    assert!(source.source().contains("c4/4"));
    assert!(source.source().contains("d4/4"));
    assert!(source.source().contains("e4/4"));
    assert!(source.source().contains("f4/4"));
}

#[test]
fn a_block_chord_previews_as_separate_voices() {
    let session = session();
    // A C-major triad held for one full 4/4 bar: each voice carries a whole
    // note, so the bar is complete.
    let events = [
        captured(0, MidiMessageKind::NoteOn, 60, 90, 0),
        captured(1, MidiMessageKind::NoteOn, 64, 90, 0),
        captured(2, MidiMessageKind::NoteOn, 67, 90, 0),
        captured(3, MidiMessageKind::NoteOff, 60, 0, 2_000_000),
        captured(4, MidiMessageKind::NoteOff, 64, 0, 2_000_000),
        captured(5, MidiMessageKind::NoteOff, 67, 0, 2_000_000),
    ];
    let proposal = session
        .propose_notation(
            "chord",
            &events,
            TakeClock::Known {
                quarter_micros: 500_000,
                origin_micros: 0,
            },
            96,
            "4/4",
            None,
            "standard",
            &[],
        )
        .expect("a three-note block chord must propose");
    assert_eq!(proposal.voice_count(), 3);
    let source = proposal
        .source()
        .expect("a block chord of binary whole notes is spellable");
    for pitch in ["c4/1", "e4/1", "g4/1"] {
        assert!(source.source().contains(pitch), "the preview must carry {pitch}");
    }
}

#[test]
fn chromatic_spelling_is_explicit_and_round_trips() {
    let session = session();
    // One full bar, first note a chromatically-spelled C-sharp.
    let events = [
        captured(0, MidiMessageKind::NoteOn, 61, 90, 0),
        captured(1, MidiMessageKind::NoteOff, 61, 0, 500_000),
        captured(2, MidiMessageKind::NoteOn, 62, 90, 505_000),
        captured(3, MidiMessageKind::NoteOff, 62, 0, 1_005_000),
        captured(4, MidiMessageKind::NoteOn, 64, 90, 993_000),
        captured(5, MidiMessageKind::NoteOff, 64, 0, 1_493_000),
        captured(6, MidiMessageKind::NoteOn, 65, 90, 1_508_000),
        captured(7, MidiMessageKind::NoteOff, 65, 0, 2_008_000),
    ];
    let proposal = session
        .propose_notation(
            "chromatic",
            &events,
            TakeClock::Known {
                quarter_micros: 500_000,
                origin_micros: 0,
            },
            96,
            "4/4",
            None,
            "standard",
            &[],
        )
        .expect("a chromatic melody must propose");
    assert_eq!(proposal.notes()[0].pitch, "c#4");
    assert_eq!(proposal.notes()[0].alternatives, vec!["db4".to_owned()]);
    let source = proposal.source().expect("a chromatic quarter is spellable");
    assert!(source.source().contains("c#4/4"));
}

#[test]
fn an_unspellable_duration_is_a_deferral_not_a_wrong_source() {
    let session = session();
    // One 5-tick written end at a 480 ms quarter: not a binary or dotted
    // value, not a single tuplet member, so the written speller declines and
    // the proposal declares the loss rather than rounding it.
    let events = [
        captured(0, MidiMessageKind::NoteOn, 60, 90, 0),
        captured(1, MidiMessageKind::NoteOff, 60, 0, 100_000),
    ];
    let proposal = session
        .propose_notation(
            "odd-length",
            &events,
            TakeClock::Known {
                quarter_micros: 480_000,
                origin_micros: 0,
            },
            96,
            "4/4",
            None,
            "standard",
            &[],
        )
        .expect("an odd-length take still proposes facts");
    assert!(proposal.source().is_none(), "an unspellable duration defers the source");
    assert_eq!(
        proposal.losses(),
        &[ProposalLoss::DeferredDuration { note_index: 0 }],
        "the deferral loss must name the note"
    );
}

#[test]
fn the_corpus_triplet_fixture_previews_as_bracketed_tuplets() {
    let session = session();
    let body: Corpus = serde_json::from_str(CORPUS).expect("decode the checked-in corpus");
    let fixture = body
        .fixtures
        .iter()
        .find(|fixture| fixture.id == "triplet-known")
        .expect("the corpus carries the triplet fixture");
    let proposal = session
        .propose_notation(
            &fixture.id,
            &synthesize(fixture),
            clock_of(&fixture.clock),
            96,
            "4/4",
            None,
            "standard",
            &[],
        )
        .expect("the triplet fixture must propose");
    // Every measured written end equals the corpus's expected notated
    // duration: five triplet eighths and a closing quarter.
    let expected = fixture
        .notes
        .iter()
        .filter_map(|note| note.expected.map(|e| e.duration_ticks))
        .collect::<Vec<_>>();
    assert_eq!(expected, vec![8, 8, 8, 8, 8, 24]);
    assert_eq!(
        proposal
            .notes()
            .iter()
            .map(|note| note.written_end_ticks)
            .collect::<Vec<_>>(),
        expected,
        "the triplet fixture's written ends must match its expected durations"
    );
    let source = proposal
        .source()
        .expect("ternary divisions are spellable as tuplets")
        .source()
        .to_owned();
    assert!(
        source.contains("tuplet 3/2 { c4/8 d4/8 e4/8 }"),
        "the first quarter's run is one bracket: {source}"
    );
    assert!(
        source.contains("tuplet 3/2 { f4/8 g4/8 }"),
        "the second quarter's run is its own bracket: {source}"
    );
    assert!(source.contains("a4/4"), "the closing quarter is a quarter: {source}");
}

#[test]
fn the_corpus_syncopation_fixture_previews_with_dots_and_a_barline_tie() {
    let session = session();
    let body: Corpus = serde_json::from_str(CORPUS).expect("decode the checked-in corpus");
    let fixture = body
        .fixtures
        .iter()
        .find(|fixture| fixture.id == "syncopated-known")
        .expect("the corpus carries the syncopation fixture");
    let proposal = session
        .propose_notation(
            &fixture.id,
            &synthesize(fixture),
            clock_of(&fixture.clock),
            96,
            "4/4",
            None,
            "standard",
            &[],
        )
        .expect("the syncopation fixture must propose");
    let expected = fixture
        .notes
        .iter()
        .filter_map(|note| note.expected.map(|e| e.duration_ticks))
        .collect::<Vec<_>>();
    assert_eq!(expected, vec![12, 24, 6, 18, 24, 24]);
    assert_eq!(
        proposal
            .notes()
            .iter()
            .map(|note| note.written_end_ticks)
            .collect::<Vec<_>>(),
        expected,
        "the syncopation fixture's written ends must match its expected durations"
    );
    let source = proposal
        .source()
        .expect("displaced binary notes and a dotted eighth are spellable")
        .source()
        .to_owned();
    assert!(
        source.contains("c4/8 d4/4 e4/16 f4/8. g4/4"),
        "off-beat notes stay displaced and 18 ticks is a dotted eighth: {source}"
    );
    assert!(
        source.contains("a4/8 ~"),
        "the quarter from tick 84 is split into tied halves at the bar line: {source}"
    );
}

/// The source with every run of whitespace collapsed to one space, so
/// assertions see content and not the formatter's line wrapping.
fn normalized(source: &str) -> String {
    source.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[test]
fn the_corpus_pedal_fixture_previews_key_releases_never_sounding_ends() {
    let session = session();
    let body: Corpus = serde_json::from_str(CORPUS).expect("decode the checked-in corpus");
    let fixture = body
        .fixtures
        .iter()
        .find(|fixture| fixture.id == "pedal-repetition-mistake")
        .expect("the corpus carries the pedal fixture");
    let proposal = session
        .propose_notation(
            &fixture.id,
            &synthesize(fixture),
            clock_of(&fixture.clock),
            96,
            "4/4",
            None,
            "standard",
            &[],
        )
        .expect("the pedal fixture must propose");
    // Every written end is the quantized key release (9 ticks, a dotted
    // sixteenth), never the pedal-extended sounding end (~81 ticks).
    for note in proposal.notes() {
        assert_eq!(note.written_end_ticks, 9, "the written end is the key release");
        assert!(note.pedal_extended, "the pedal fact is reported beside it");
    }
    let source = normalized(
        proposal
            .source()
            .expect("dotted sixteenths and binary rests are spellable")
            .source(),
    );
    // The exact deterministic line: five dotted-sixteenth key releases with
    // spelled gaps, closing the bar — no whole, half, or dotted-half anywhere.
    assert!(
        source.contains(
            "c4/16. rest/8 rest/32 c4/16. rest/8 rest/32 e4/16. rest/32 f4/16. rest/32 g4/16. rest/8 rest/32"
        ),
        "the staccato key releases are dotted sixteenths with spelled gaps: {source}"
    );
}

#[test]
fn a_grace_tier_note_previews_as_a_grace_on_its_principal() {
    let session = session();
    // A quarter, a 3-tick gesture, another quarter: the gesture is below the
    // ordinary tier, so the page carries it as a grace standing on the
    // following quarter, donating its time to that principal.
    let events = [
        captured(0, MidiMessageKind::NoteOn, 60, 90, 0),
        captured(1, MidiMessageKind::NoteOff, 60, 0, 500_000),
        captured(2, MidiMessageKind::NoteOn, 62, 90, 505_000),
        captured(3, MidiMessageKind::NoteOff, 62, 0, 565_000),
        captured(4, MidiMessageKind::NoteOn, 64, 90, 570_000),
        captured(5, MidiMessageKind::NoteOff, 64, 0, 1_070_000),
    ];
    let proposal = session
        .propose_notation(
            "grace-gesture",
            &events,
            TakeClock::Known {
                quarter_micros: 500_000,
                origin_micros: 0,
            },
            96,
            "4/4",
            None,
            "standard",
            &[],
        )
        .expect("a grace-like gesture must propose");
    assert!(proposal.notes()[1].grace, "the 3-tick gesture is grace tier");
    let source = proposal
        .source()
        .expect("a grace on its principal is spellable")
        .source()
        .to_owned();
    assert!(
        source.contains("c4/4 grace { d4 } e4/4"),
        "the gesture leans on the following quarter: {source}"
    );
}

#[test]
fn mixed_rests_fill_gaps_and_the_final_bar_exactly() {
    let session = session();
    // A quarter, a 15-tick gap, a dotted eighth, a 3-tick gap, an eighth: the
    // gaps spell as greedy binary rests and the bar closes itself.
    let events = [
        captured(0, MidiMessageKind::NoteOn, 60, 90, 0),
        captured(1, MidiMessageKind::NoteOff, 60, 0, 500_000),
        captured(2, MidiMessageKind::NoteOn, 62, 90, 810_000),
        captured(3, MidiMessageKind::NoteOff, 62, 0, 1_185_000),
        captured(4, MidiMessageKind::NoteOn, 64, 90, 1_250_000),
        captured(5, MidiMessageKind::NoteOff, 64, 0, 1_500_000),
    ];
    let proposal = session
        .propose_notation(
            "mixed-rests",
            &events,
            TakeClock::Known {
                quarter_micros: 500_000,
                origin_micros: 0,
            },
            96,
            "4/4",
            None,
            "standard",
            &[],
        )
        .expect("a gapped melody must propose");
    let source = normalized(
        proposal
            .source()
            .expect("binary and dotted rests are spellable")
            .source(),
    );
    assert!(
        source.contains("c4/4 rest/8 rest/32 d4/8. rest/32 e4/8 rest/4"),
        "a 15-tick gap is rest/8 rest/32, a 3-tick gap rest/32, and the bar\ncloses with the greedy fill: {source}"
    );
}

#[test]
fn rhythmic_spelling_is_deterministic_through_the_facade() {
    let session = session();
    let body: Corpus = serde_json::from_str(CORPUS).expect("decode the checked-in corpus");
    let mut non_binary = 0_usize;
    for fixture in &body.fixtures {
        if matches!(fixture.clock, Clock::Unmeasured) {
            continue;
        }
        let events = synthesize(fixture);
        let clock = clock_of(&fixture.clock);
        let first = session
            .propose_notation(&fixture.id, &events, clock, 96, "4/4", None, "standard", &[])
            .unwrap_or_else(|error| panic!("{}: {:?}", fixture.id, error));
        let second = session
            .propose_notation(&fixture.id, &events, clock, 96, "4/4", None, "standard", &[])
            .unwrap_or_else(|error| panic!("{}: {:?}", fixture.id, error));
        assert_eq!(first, second, "{} must compose identically twice", fixture.id);
        if first.source().is_some() {
            non_binary += 1;
        }
    }
    assert!(
        non_binary >= 3,
        "at least the triplet, syncopation, and pedal fixtures must preview, saw {non_binary}"
    );
}

#[test]
fn the_proposal_names_its_constituents_and_is_deterministic() {
    let session = session();
    let events = [
        captured(0, MidiMessageKind::NoteOn, 60, 90, 0),
        captured(1, MidiMessageKind::NoteOff, 60, 0, 500_000),
        captured(2, MidiMessageKind::NoteOn, 62, 90, 505_000),
        captured(3, MidiMessageKind::NoteOff, 62, 0, 1_005_000),
        captured(4, MidiMessageKind::NoteOn, 64, 90, 993_000),
        captured(5, MidiMessageKind::NoteOff, 64, 0, 1_493_000),
        captured(6, MidiMessageKind::NoteOn, 65, 90, 1_508_000),
        captured(7, MidiMessageKind::NoteOff, 65, 0, 2_008_000),
    ];
    let clock = TakeClock::Known {
        quarter_micros: 500_000,
        origin_micros: 0,
    };
    let first = session
        .propose_notation("constants", &events, clock, 96, "4/4", None, "standard", &[])
        .expect("must propose");
    let second = session
        .propose_notation("constants", &events, clock, 96, "4/4", None, "standard", &[])
        .expect("must propose again");
    assert_eq!(first, second);
    assert_eq!(first.take_name(), "constants");
    assert_eq!(first.policy_name(), "standard");
    assert_eq!(first.meter(), "4/4");
    assert_eq!(first.proposal_version(), 1);
    assert_eq!(first.cost_version(), 1);
    assert_eq!(
        first.cost_fields(),
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
    assert_eq!(first.rhythm_candidate().onsets.len(), 4);
}

fn best_label_matches(predicted: &[u8], intended: &[u8]) -> usize {
    let labels = intended
        .iter()
        .copied()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let mut permutation = (0..labels.len())
        .map(|index| u8::try_from(index).unwrap_or(u8::MAX))
        .collect::<Vec<_>>();
    let mut best = 0;
    permute(&mut permutation, 0, &mut |mapping| {
        let matched = predicted
            .iter()
            .zip(intended)
            .filter(|(predicted, intended)| {
                labels
                    .iter()
                    .position(|label| label == *intended)
                    .and_then(|index| mapping.get(index))
                    .is_some_and(|mapped| mapped == *predicted)
            })
            .count();
        best = best.max(matched);
    });
    best
}

fn permute(values: &mut [u8], at: usize, visit: &mut impl FnMut(&[u8])) {
    if at == values.len() {
        visit(values);
        return;
    }
    for index in at..values.len() {
        values.swap(at, index);
        permute(values, at.saturating_add(1), visit);
        values.swap(at, index);
    }
}
