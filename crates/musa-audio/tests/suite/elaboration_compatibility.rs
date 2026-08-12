//! Audio-side compatibility oracle for the elaboration migration fixture.
//!
//! Refresh intentionally with `UPDATE_ELABORATION_BASELINE=1 cargo test
//! -p musa-audio --test suite` in a clean worktree.

#![allow(clippy::expect_used)]

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use musa_audio::{GraphOptions, compile_graph, lower_studio, render_offline};
use musa_compiler::{
    CompileOptions, ParameterId, PerformanceEvent, PerformanceOptions, SourceDocument, compile, lower_performance,
};

type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

const UPDATE: &str = "UPDATE_ELABORATION_BASELINE";
const SOURCE: &str = include_str!("../../../../tests/fixtures/audio-bridge.musa");
const BLOCK_SIZES: [usize; 2] = [64, 256];
const SAMPLE_RATE: u32 = 48_000;

fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn digest(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(0x0000_0100_0000_01b3)
    })
}

fn wav_bytes(samples: &[f32]) -> Result<Vec<u8>> {
    let spec = hound::WavSpec {
        channels: 2,
        sample_rate: SAMPLE_RATE,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };
    let mut cursor = std::io::Cursor::new(Vec::new());
    {
        let mut writer = hound::WavWriter::new(&mut cursor, spec)?;
        for sample in samples {
            writer.write_sample(*sample)?;
        }
        writer.finalize()?;
    }
    Ok(cursor.into_inner())
}

fn scheduled() -> Result<(musa_compiler::StudioSpec, Vec<PerformanceEvent>, u64, String)> {
    let compilation = compile(
        &SourceDocument::new(SOURCE, "tests/fixtures/audio-bridge.musa"),
        &CompileOptions::default(),
    );
    assert!(!compilation.has_errors(), "the audio bridge fixture must compile");
    let score = compilation.snapshot().expect("the fixture has a score");
    let performance = lower_performance(
        score,
        &PerformanceOptions {
            sample_rate: SAMPLE_RATE,
            ..PerformanceOptions::default()
        },
    )?;
    let mut lane_summary = String::new();
    let mut events = Vec::new();
    for lane in performance.lanes() {
        let mut event_text = String::new();
        for event in lane.events() {
            let _ = writeln!(event_text, "{event:?}");
            events.push(event.clone());
        }
        let _ = writeln!(
            lane_summary,
            "lane={}:{}:{}:{:016x}",
            lane.part().0,
            lane.name(),
            lane.events().len(),
            digest(event_text.as_bytes())
        );
    }
    events.sort_by_key(PerformanceEvent::frame);
    let last = events.last().map_or(0, PerformanceEvent::frame);
    let frames = last.saturating_add(u64::from(SAMPLE_RATE));
    Ok((compilation.into_parts().1, events, frames, lane_summary))
}

fn manifest() -> Result<String> {
    let (studio, events, frames, lanes) = scheduled()?;
    let mut out = String::from(
        "# musa audio-bridge compatibility manifest v1\n\
         # Test oracle only; timing and allocation samples live in docs/rules/language/06-performance.md.\n",
    );
    out.push_str(&lanes);
    let _ = writeln!(out, "sample-rate={SAMPLE_RATE}");
    let _ = writeln!(out, "frames={frames}");
    let _ = writeln!(out, "events={}", events.len());

    for block_size in BLOCK_SIZES {
        let options = GraphOptions {
            sample_rate: SAMPLE_RATE,
            block_size,
        };
        let (graph, lowering) = lower_studio(&studio, &options);
        let graph_debug = format!("{graph:#?}");
        let _ = writeln!(out, "block-size={block_size}");
        let _ = writeln!(out, "graph={:016x}", digest(graph_debug.as_bytes()));
        let _ = writeln!(out, "lowering-notes={:?}", lowering.notes);
        let _ = writeln!(out, "release-tail-bits={:08x}", lowering.release_tail.to_bits());

        let mut plan = compile_graph(&graph, &options)?;
        let audio = render_offline(&mut plan, &events, frames);
        let wav = wav_bytes(audio.samples())?;
        let _ = writeln!(out, "wav={:016x}", digest(&wav));

        let mut with_parameter = events.clone();
        with_parameter.push(PerformanceEvent::Parameter {
            frame: u64::from(SAMPLE_RATE),
            target: ParameterId(0),
            value: 0.125,
        });
        with_parameter.sort_by_key(PerformanceEvent::frame);
        let mut parameter_plan = compile_graph(&graph, &options)?;
        let parameter_audio = render_offline(&mut parameter_plan, &with_parameter, frames);
        let _ = writeln!(
            out,
            "ignored-parameter-event={}",
            parameter_audio.samples() == audio.samples()
        );
    }
    Ok(out)
}

#[test]
fn score_to_graph_to_wav_matches_the_migration_oracle() -> Result {
    let actual = manifest()?;
    let path = repository().join("tests/fixtures/audio-bridge-audio.compat");
    let expected = std::fs::read_to_string(&path).ok();
    if expected.as_deref() == Some(actual.as_str()) {
        return Ok(());
    }
    if std::env::var_os(UPDATE).is_some() {
        std::fs::write(path, actual)?;
        return Ok(());
    }
    Err(format!(
        "{} differs — rerun in a clean worktree with {UPDATE}=1 and review the WAV/graph diff",
        path.display()
    )
    .into())
}
