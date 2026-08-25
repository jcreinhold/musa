//! Audio-side compatibility oracle for the elaboration migration fixture.
//!
//! Refresh intentionally with `UPDATE_ELABORATION_BASELINE=1 cargo test
//! -p musa-compiler --test suite` in a clean worktree.

#![allow(clippy::expect_used)]

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use musa_compiler::{CompileOptions, SourceDocument, compile};
use musa_score::{GesturePlan, lower_gestures};

use super::audio_support::{options, prepare_gestures};

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

fn scheduled() -> Result<(musa_dsp::StudioSpec, GesturePlan, String)> {
    let compilation = compile(
        &SourceDocument::new(SOURCE, "tests/fixtures/audio-bridge.musa"),
        &CompileOptions::default(),
    );
    assert!(!compilation.has_errors(), "the audio bridge fixture must compile");
    let score = compilation.snapshot().expect("the fixture has a score");
    let performance = lower_gestures(score)?;
    let mut lane_summary = String::new();
    for lane in performance.lanes() {
        let mut event_text = String::new();
        for occurrence in lane.track().occurrences() {
            let _ = writeln!(event_text, "{occurrence:?}");
        }
        let _ = writeln!(
            lane_summary,
            "lane={}:{}:{}:{:016x}",
            lane.part().0,
            lane.name(),
            lane.track().occurrences().len(),
            digest(event_text.as_bytes())
        );
    }
    Ok((compilation.into_parts().1, performance, lane_summary))
}

fn manifest() -> Result<String> {
    let (studio, gestures, lanes) = scheduled()?;
    let mut out = String::from(
        "# musa audio-bridge compatibility manifest v2\n\
         # Test oracle only; timing and allocation samples live in docs/rules/language/06-elaboration-baseline.md.\n",
    );
    out.push_str(&lanes);
    let _ = writeln!(out, "sample-rate={SAMPLE_RATE}");
    let _ = writeln!(
        out,
        "gestures={}",
        gestures
            .lanes()
            .iter()
            .map(|lane| lane.track().occurrences().len())
            .sum::<usize>()
    );

    for block_size in BLOCK_SIZES {
        let mut audio = prepare_gestures(&gestures, &studio, options(u64::from(SAMPLE_RATE)))?;
        let frames = audio.total_frames();
        let mut samples = vec![0.0; usize::try_from(frames.saturating_mul(2))?];
        for chunk in samples.chunks_mut(block_size.saturating_mul(2)) {
            audio.render(chunk);
        }
        let _ = writeln!(out, "block-size={block_size}");
        let _ = writeln!(out, "frames={frames}");
        let wav = wav_bytes(&samples)?;
        let _ = writeln!(out, "wav={:016x}", digest(&wav));
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
