use std::collections::BTreeSet;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;
use std::time::{Duration, Instant};

use divan::black_box;
use midly::{MetaMessage, MidiMessage, Smf, Timing, TrackEventKind};
use musa_playback::{MidiInput, MidiMessageKind};
use musa_project::transcription_trial::{TrialCorpus, generated_corpus, generated_stress_corpus, run};
use serde::Serialize;

const ASAP_COMMIT: &str = "fad8d1e8078d0ae47ad2f280b5d022bd2de24784";
const ASAP_CASES: [(&str, &str); 3] = [
    ("Bach/Fugue/bwv_846/Shi05M.mid", "Bach/Fugue/bwv_846/midi_score.mid"),
    (
        "Mozart/Piano_Sonatas/12-3/WuuE04M.mid",
        "Mozart/Piano_Sonatas/12-3/midi_score.mid",
    ),
    (
        "Debussy/Images_Book_1/1_Reflets_dans_lEau/Kleisen11M.mid",
        "Debussy/Images_Book_1/1_Reflets_dans_lEau/midi_score.mid",
    ),
];

fn main() -> Result<(), Box<dyn Error>> {
    if let Some(root) = std::env::var_os("MUSA_ASAP_ROOT") {
        let summary = external_summary(Path::new(&root))?;
        eprintln!("{}", serde_json::to_string_pretty(&summary)?);
    }
    if let Some(seconds) = std::env::var_os("MUSA_LIVE_TRIAL_SECONDS") {
        let seconds = seconds.to_string_lossy().parse()?;
        eprintln!("{}", serde_json::to_string_pretty(&live_summary(seconds)?)?);
    }
    if let Some(note_count) = std::env::var_os("MUSA_STRESS_REPORT") {
        let note_count = note_count.to_string_lossy().parse()?;
        eprintln!(
            "{}",
            serde_json::to_string_pretty(&run(&generated_stress_corpus(note_count)))?
        );
    }
    divan::main();
    Ok(())
}

#[derive(Serialize)]
struct LiveSummary {
    device: String,
    completed_notes: usize,
    evaluated_notes: usize,
    report: musa_project::transcription_trial::TrialReport,
}

fn live_summary(seconds: u64) -> Result<LiveSummary, Box<dyn Error>> {
    let devices = MidiInput::devices();
    let preferred = match devices.as_slice() {
        [device] => Some(device.id.as_str()),
        [] => return Err("no MIDI input is connected".into()),
        _ => return Err("more than one MIDI input is connected; leave only the trial keyboard connected".into()),
    };
    let mut input = MidiInput::open(preferred);
    let device = input
        .device()
        .map(|device| device.name.clone())
        .ok_or("the MIDI input could not be opened")?;
    eprintln!(
        "Live trial on {device}: play one ascending major scale as even eighth notes; recording for {seconds} seconds"
    );
    let started = Instant::now();
    let mut held = std::collections::BTreeMap::new();
    let mut notes = Vec::new();
    while started.elapsed() < Duration::from_secs(seconds) {
        while let Some(event) = input.poll() {
            match event.kind {
                MidiMessageKind::NoteOn if event.value > 0 => {
                    held.insert((event.channel, event.data), event.callback_micros);
                }
                MidiMessageKind::NoteOff | MidiMessageKind::NoteOn => {
                    if let Some(onset) = held.remove(&(event.channel, event.data)) {
                        notes.push((event.data, onset, event.callback_micros.max(onset)));
                    }
                }
                MidiMessageKind::ControlChange
                | MidiMessageKind::PitchBend
                | MidiMessageKind::ChannelPressure
                | MidiMessageKind::KeyPressure
                | MidiMessageKind::ProgramChange => {}
            }
        }
        thread::sleep(Duration::from_millis(1));
    }
    notes.sort_by_key(|(_, onset, _)| *onset);
    let scale = notes.get(..8).ok_or("fewer than eight completed notes were captured")?;
    let root = scale.first().map(|(pitch, _, _)| *pitch).ok_or("the take is empty")?;
    let expected_intervals = [0_u8, 2, 4, 5, 7, 9, 11, 12];
    if !scale
        .iter()
        .zip(expected_intervals)
        .all(|((pitch, _, _), interval)| *pitch == root.saturating_add(interval))
    {
        return Err("the first eight notes were not one ascending major scale".into());
    }
    let first = scale.first().map_or(0, |(_, onset, _)| *onset);
    let trial_notes = scale
        .iter()
        .enumerate()
        .map(|(index, (pitch, onset, release))| {
            serde_json::json!({
                "id": index,
                "pitch": pitch,
                "onset_micros": onset.saturating_sub(first),
                "release_micros": release.saturating_sub(first),
                "sounding_end_micros": release.saturating_sub(first),
                "expected": {
                    "onset_ticks": index.saturating_mul(12),
                    "duration_ticks": 12,
                    "voice": 0,
                    "group": index,
                }
            })
        })
        .collect::<Vec<_>>();
    let corpus = TrialCorpus::read(&serde_json::to_string(&serde_json::json!({
        "schema": 1,
        "ticks_per_quarter": 24,
        "fixtures": [{
            "id": "consented-local-major-scale",
            "source": "aggregate-only",
            "tags": ["human", "local", "free"],
            "clock": { "kind": "free" },
            "notes": trial_notes,
            "controls": [],
            "expected_review": "tap_pulse"
        }]
    }))?)?;
    Ok(LiveSummary {
        device,
        completed_notes: notes.len(),
        evaluated_notes: scale.len(),
        report: run(&corpus),
    })
}

#[divan::bench]
fn complete_generated_trial() {
    let corpus = generated_corpus();
    black_box(run(black_box(&corpus)));
}

#[divan::bench(args = [32, 64, 128, 256])]
fn regular_phrase_scaling(note_count: usize) {
    let corpus = generated_stress_corpus(note_count);
    black_box(run(black_box(&corpus)));
}

#[derive(Serialize)]
struct ExternalSummary {
    dataset_commit: &'static str,
    cases: Vec<ExternalCase>,
    matched_notes: u64,
    exact_anchor_normalized_onsets: u64,
    absolute_tick_error: u64,
}

#[derive(Serialize)]
struct ExternalCase {
    fixture: &'static str,
    performance_notes: usize,
    score_notes: usize,
    matched_notes: u64,
    exact_onsets: u64,
    absolute_tick_error: u64,
}

fn external_summary(root: &Path) -> Result<ExternalSummary, Box<dyn Error>> {
    verify_checkout(root)?;
    let mut cases = Vec::new();
    for (performance, score) in ASAP_CASES {
        cases.push(compare_case(root, performance, score)?);
    }
    Ok(ExternalSummary {
        dataset_commit: ASAP_COMMIT,
        matched_notes: cases.iter().map(|case| case.matched_notes).sum(),
        exact_anchor_normalized_onsets: cases.iter().map(|case| case.exact_onsets).sum(),
        absolute_tick_error: cases.iter().map(|case| case.absolute_tick_error).sum(),
        cases,
    })
}

fn verify_checkout(root: &Path) -> Result<(), Box<dyn Error>> {
    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(root)
        .output()?;
    let actual = String::from_utf8(output.stdout)?;
    if !output.status.success() || actual.trim() != ASAP_COMMIT {
        return Err(format!("expected ASAP v1.1 commit {ASAP_COMMIT}, found {}", actual.trim()).into());
    }
    Ok(())
}

fn compare_case(root: &Path, performance: &'static str, score: &'static str) -> Result<ExternalCase, Box<dyn Error>> {
    let performance_path = root.join(performance);
    let score_path = root.join(score);
    let performed = note_ons(&performance_path)?;
    let intended = note_ons(&score_path)?;
    let performed_beats = annotation_beats(annotation_path(&performance_path))?;
    let score_beats = annotation_beats(annotation_path(&score_path))?;
    let intended_beats = intended
        .iter()
        .filter_map(|(pitch, seconds)| beat_position(*seconds, &score_beats).map(|beat| (*pitch, beat)))
        .collect::<Vec<_>>();
    let mut used = BTreeSet::new();
    let mut matched = 0_u64;
    let mut exact = 0_u64;
    let mut error = 0_u64;
    for &(pitch, performed_seconds) in &performed {
        let Some(performed_beat) = beat_position(performed_seconds, &performed_beats) else {
            continue;
        };
        let Some((index, (_, score_beat))) = intended_beats
            .iter()
            .enumerate()
            .filter(|(index, (candidate, score_beat))| {
                !used.contains(index) && *candidate == pitch && (performed_beat - *score_beat).abs() <= 2.0
            })
            .min_by(|(_, (_, left)), (_, (_, right))| {
                (performed_beat - *left)
                    .abs()
                    .total_cmp(&(performed_beat - *right).abs())
            })
        else {
            continue;
        };
        used.insert(index);
        let performed_tick = (performed_beat * 24.0).round();
        let score_tick = (*score_beat * 24.0).round();
        let tick_error = (performed_tick - score_tick).abs() as u64;
        matched = matched.saturating_add(1);
        exact = exact.saturating_add(u64::from(tick_error == 0));
        error = error.saturating_add(tick_error);
    }
    Ok(ExternalCase {
        fixture: performance,
        performance_notes: performed.len(),
        score_notes: intended.len(),
        matched_notes: matched,
        exact_onsets: exact,
        absolute_tick_error: error,
    })
}

fn annotation_path(midi: &Path) -> PathBuf {
    let stem = midi.file_stem().and_then(|stem| stem.to_str()).unwrap_or_default();
    midi.with_file_name(format!("{stem}_annotations.txt"))
}

fn annotation_beats(path: PathBuf) -> Result<Vec<f64>, Box<dyn Error>> {
    let mut beats = Vec::new();
    for line in fs::read_to_string(path)?.lines() {
        let mut fields = line.split('\t');
        let Some(seconds) = fields.next() else { continue };
        let _duplicate = fields.next();
        let Some(label) = fields.next() else { continue };
        let beat_label = label.split(',').next().unwrap_or_default();
        if matches!(beat_label, "b" | "db" | "bR") {
            beats.push(seconds.parse()?);
        }
    }
    Ok(beats)
}

fn beat_position(seconds: f64, beats: &[f64]) -> Option<f64> {
    let index = beats.partition_point(|beat| *beat <= seconds).saturating_sub(1);
    let (&left, &right) = beats.get(index).zip(beats.get(index.saturating_add(1)))?;
    let fraction = (seconds - left) / (right - left);
    Some(index as f64 + fraction)
}

fn note_ons(path: &Path) -> Result<Vec<(u8, f64)>, Box<dyn Error>> {
    let bytes = fs::read(path)?;
    let midi = Smf::parse(&bytes)?;
    let ticks_per_quarter = match midi.header.timing {
        Timing::Metrical(ticks) => u64::from(ticks.as_int()),
        Timing::Timecode(_, _) => return Err(format!("{} uses SMPTE timing", path.display()).into()),
    };
    let mut events = Vec::new();
    for track in &midi.tracks {
        let mut tick = 0_u64;
        for event in track {
            tick = tick.saturating_add(u64::from(event.delta.as_int()));
            events.push((tick, event.kind));
        }
    }
    events.sort_by_key(|(tick, _)| *tick);
    let mut tempo = 500_000_u64;
    let mut previous_tick = 0_u64;
    let mut micros = 0_u128;
    let mut notes = Vec::new();
    for (tick, event) in events {
        let delta = tick.saturating_sub(previous_tick);
        let elapsed = u128::from(delta)
            .saturating_mul(u128::from(tempo))
            .checked_div(u128::from(ticks_per_quarter.max(1)))
            .unwrap_or(0);
        micros = micros.saturating_add(elapsed);
        previous_tick = tick;
        match event {
            TrackEventKind::Meta(MetaMessage::Tempo(next)) => tempo = u64::from(next.as_int()),
            TrackEventKind::Midi {
                message: MidiMessage::NoteOn { key, vel },
                ..
            } if vel.as_int() > 0 => notes.push((key.as_int(), micros as f64 / 1_000_000.0)),
            TrackEventKind::Midi { .. }
            | TrackEventKind::SysEx(_)
            | TrackEventKind::Escape(_)
            | TrackEventKind::Meta(_) => {}
        }
    }
    Ok(notes)
}
