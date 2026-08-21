//! The score → performance → graph → audio benchmark.
//!
//! `with_inputs` prepares a fresh render plan outside the timed region, so
//! the measurement is render time and allocation rather than fixture setup.

#![allow(clippy::expect_used)]

use musa_audio::{GraphOptions, RenderPlan, compile_graph, lower_studio, render_offline};
use musa_compiler::{CompileOptions, SourceDocument, compile};
use musa_score::{PerformanceEvent, PerformanceOptions, lower_performance};

#[global_allocator]
static ALLOC: divan::AllocProfiler = divan::AllocProfiler::system();

const SOURCE: &str = include_str!("../../../tests/fixtures/audio-bridge.musa");
const SAMPLE_RATE: u32 = 48_000;
const BLOCK_SIZE: usize = 128;

fn prepare() -> (RenderPlan, Vec<PerformanceEvent>, u64) {
    let compilation = compile(
        &SourceDocument::new(SOURCE, "tests/fixtures/audio-bridge.musa"),
        &CompileOptions::default(),
    );
    let score = compilation.snapshot().expect("fixture compiles");
    let performance = lower_performance(
        score,
        &PerformanceOptions {
            sample_rate: SAMPLE_RATE,
            ..PerformanceOptions::default()
        },
    )
    .expect("fixture schedules");
    let mut events: Vec<PerformanceEvent> = performance
        .lanes()
        .iter()
        .flat_map(|lane| lane.events().iter().cloned())
        .collect();
    events.sort_by_key(PerformanceEvent::frame);
    let frames = events
        .last()
        .map_or(0, PerformanceEvent::frame)
        .saturating_add(u64::from(SAMPLE_RATE));
    let options = GraphOptions {
        sample_rate: SAMPLE_RATE,
        block_size: BLOCK_SIZE,
    };
    let (graph, _) = lower_studio(compilation.studio(), &options);
    let plan = compile_graph(&graph, &options).expect("graph compiles");
    (plan, events, frames)
}

#[divan::bench(sample_count = 30)]
fn render_audio_bridge(bencher: divan::Bencher<'_, '_>) {
    bencher
        .with_inputs(prepare)
        .bench_values(|(mut plan, events, frames)| render_offline(&mut plan, &events, frames));
}

fn main() {
    divan::main();
}
