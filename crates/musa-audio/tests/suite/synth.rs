//! Polysynth tests (roadmap §13.5, §17.5): voice-allocator stealing and
//! note-off matching, click-free placeholder ramps, golden-frequency
//! accuracy, and offline-render determinism.

// Test helpers use expect() on statically-valid inputs: a failure is a bug in
// the test itself, and panicking is the correct behavior there.
#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
// Test buffers are sized to the slices they hand the renderer.
#![allow(clippy::indexing_slicing)]
// Sample arithmetic in tests is small and total.
#![allow(clippy::arithmetic_side_effects)]

use musa_audio::{GraphOptions, VoiceAllocator, compile_graph, poly_sine_spec, render_offline};
use musa_compiler::{
    PerformanceEvent, PerformanceOptions, ScoreSnapshot, SourceDocument, VoiceInstanceId, compile, lower_performance,
};

const RATE: u32 = 48_000;

/// Compile and lower a `.musa` source, returning its merged event list.
fn events_of(source: &str) -> Vec<PerformanceEvent> {
    let compilation = compile(
        &SourceDocument::new(source, "test"),
        &musa_compiler::CompileOptions::default(),
    );
    let score: ScoreSnapshot = compilation.into_snapshot().expect("compiles");
    let plan = lower_performance(&score, &PerformanceOptions::default()).expect("lowers");
    let mut events: Vec<PerformanceEvent> = plan
        .lanes()
        .iter()
        .flat_map(|lane| lane.events().iter().cloned())
        .collect();
    events.sort_by_key(PerformanceEvent::frame);
    events
}

fn render_events(events: &[PerformanceEvent], frames: u64) -> Vec<f32> {
    let options = GraphOptions {
        sample_rate: RATE,
        block_size: 128,
    };
    let mut plan = compile_graph(&poly_sine_spec(16), &options).expect("graph");
    render_offline(&mut plan, events, frames).samples().to_vec()
}

// --- Allocator ----------------------------------------------------------------

#[test]
fn allocator_steals_the_oldest_voice() {
    let mut allocator = VoiceAllocator::new(2, RATE);
    allocator.note_on(VoiceInstanceId(0), 220.0, 1.0, 0.0);
    // Age the first voice so it is the steal candidate.
    let mut buffer = vec![0.0f32; 64];
    allocator.render(&mut buffer, 64, f64::from(RATE));
    allocator.note_on(VoiceInstanceId(1), 330.0, 1.0, 0.0);
    allocator.note_on(VoiceInstanceId(2), 440.0, 1.0, 0.0);
    assert_eq!(allocator.gated(), 2, "pool is full");
    // Note-off for the stolen instance must match nothing...
    allocator.note_off(VoiceInstanceId(0));
    assert_eq!(allocator.gated(), 2, "stolen voice's off matches nothing");
    // ...while the live instances release exactly their own voices.
    allocator.note_off(VoiceInstanceId(1));
    assert_eq!(allocator.gated(), 1);
    allocator.note_off(VoiceInstanceId(2));
    assert_eq!(allocator.gated(), 0);
}

#[test]
fn allocator_reuses_fully_released_voices() {
    let mut allocator = VoiceAllocator::new(1, RATE);
    allocator.note_on(VoiceInstanceId(0), 440.0, 1.0, 0.0);
    allocator.note_off(VoiceInstanceId(0));
    // Render past the 50 ms release: the voice returns to the pool.
    let mut buffer = vec![0.0f32; 4800];
    allocator.render(&mut buffer, 4800, f64::from(RATE));
    assert_eq!(allocator.sounding(), 0);
    allocator.note_on(VoiceInstanceId(1), 440.0, 1.0, 0.0);
    assert_eq!(allocator.gated(), 1, "released voice is reusable");
}

// --- Headroom -----------------------------------------------------------------

/// The pool sum is scaled by `1/√voices`: one voice in a 16-voice pool peaks
/// at 0.25, not at full scale. Without that, sixteen full-scale sines sum to
/// 24 dB over 0 dBFS and the master limiter becomes the mix bus.
#[test]
fn the_pool_sum_carries_fixed_headroom() {
    let mut allocator = VoiceAllocator::new(16, RATE);
    allocator.note_on(VoiceInstanceId(0), 440.0, 1.0, 0.0);
    let mut buffer = vec![0.0f32; RATE as usize / 2];
    let count = buffer.len();
    allocator.render(&mut buffer, count, f64::from(RATE));
    let peak = buffer.iter().fold(0.0f32, |worst, sample| worst.max(sample.abs()));
    assert!((peak - 0.25).abs() < 0.01, "one voice peaks at 1/√16, got {peak}");
}

/// A full pool of spread-phase voices stays near full scale: 12 dB under the
/// raw `voices`-times-full-scale the sum used to reach, and never quieter
/// than a single voice. Note-ons are staggered the way real polyphony is;
/// sample-aligned onsets are phase-coherent by construction (`note_on`
/// resets phase), peak at exactly `voices·scale = 4.0`, and are the
/// transient the master's lookahead limiter exists to absorb.
#[test]
fn a_full_pool_stays_near_full_scale() {
    let mut allocator = VoiceAllocator::new(16, RATE);
    let mut buffer = vec![0.0f32; RATE as usize];
    for i in 0..16u32 {
        allocator.note_on(VoiceInstanceId(i), 37.0f32.mul_add(i as f32, 200.0), 1.0, 0.0);
        let start = i as usize * 100;
        allocator.render(&mut buffer[start..start + 100], 100, f64::from(RATE));
    }
    allocator.render(&mut buffer[1600..], RATE as usize - 1600, f64::from(RATE));
    let peak = buffer.iter().fold(0.0f32, |worst, sample| worst.max(sample.abs()));
    assert!(
        peak < 2.0,
        "sixteen voices without headroom peaked near 16.0, got {peak}"
    );
    assert!(peak > 0.25, "a full pool is not quieter than one voice: {peak}");
}

// --- Synthesis ------------------------------------------------------------------

/// A single A4 quarter note.
fn a4_events() -> Vec<PerformanceEvent> {
    events_of("piece \"a4\" { tempo quarter = 60; meter 4/4; key c major; score { part p { voice v { a4/1 } } } }")
}

#[test]
fn golden_frequency_a4() {
    let events = a4_events();
    let output = render_events(&events, RATE.into());
    let left: Vec<f32> = output.iter().step_by(2).copied().collect();
    let crossings = left
        .windows(2)
        .filter(|pair| matches!(pair, [prev, next] if *prev <= 0.0 && *next > 0.0))
        .count();
    assert!(
        (438..=442).contains(&crossings),
        "A4 should cross zero ~440 times in 1 s, counted {crossings}"
    );
}

#[test]
fn note_on_and_off_are_click_free() {
    let events = a4_events();
    let output = render_events(&events, 2 * u64::from(RATE));
    let max_step = output
        .windows(2)
        .filter_map(|pair| pair.first().zip(pair.get(1)).map(|(a, b)| (b - a).abs()))
        .fold(0.0f32, f32::max);
    // A sine at amplitude ~1 can legitimately move 2πf/sr ≈ 0.06 per sample
    // at 440 Hz; the placeholder ramps must keep transitions in that league.
    assert!(max_step < 0.1, "max sample-to-sample step {max_step}");
}

#[test]
fn render_is_deterministic() {
    let events = a4_events();
    let a = render_events(&events, 8192);
    let b = render_events(&events, 8192);
    assert_eq!(a, b);
}
