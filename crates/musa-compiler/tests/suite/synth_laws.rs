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

use super::audio_support::render_source;

const RATE: u32 = 48_000;

// --- Synthesis ------------------------------------------------------------------

/// A single A4 quarter note.
fn a4_source() -> &'static str {
    "piece \"a4\" { tempo quarter = 60; meter 4/4; key c major; score { part p { voice v { a4/1 } } } }"
}

#[test]
fn golden_frequency_a4() {
    let output = render_source(a4_source(), RATE as usize);
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
    let output = render_source(a4_source(), RATE as usize * 2);
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
    let a = render_source(a4_source(), 8192);
    let b = render_source(a4_source(), 8192);
    assert_eq!(a, b);
}
