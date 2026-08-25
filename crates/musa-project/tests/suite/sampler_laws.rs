//! The project owns verified bytes; DSP owns decoding and sampler state.
#![allow(clippy::expect_used)]

use std::path::Path;

use musa_project::{ProjectSession, lock_assets};

fn write(path: &Path, bytes: impl AsRef<[u8]>) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("fixture directory");
    }
    std::fs::write(path, bytes).expect("fixture file");
}

fn wav(value: f32) -> Vec<u8> {
    let mut bytes = std::io::Cursor::new(Vec::new());
    {
        let mut writer = hound::WavWriter::new(
            &mut bytes,
            hound::WavSpec {
                channels: 1,
                sample_rate: 48_000,
                bits_per_sample: 32,
                sample_format: hound::SampleFormat::Float,
            },
        )
        .expect("wav writer");
        for _ in 0..64 {
            writer.write_sample(value).expect("sample");
        }
        writer.finalize().expect("wav finish");
    }
    bytes.into_inner()
}

fn limits() -> musa_dsp::SamplerLimits {
    musa_dsp::SamplerLimits {
        sample_rate: 48_000,
        max_voices: 8,
        max_regions: 32,
        max_decoded_bytes: 1 << 20,
        max_selection_work: 32,
        max_step_work: 1_000,
    }
}

fn project(root: &Path) {
    write(
        &root.join("musa.toml"),
        r#"[project]
name = "Native sample map"

[assets."assets/tone.wav"]
kind = "audio"
adapter = "wav@1"
max_bytes = 1048576
"#,
    );
    write(
        &root.join("piece.musa"),
        r#"import std::sound::sample;
let fixture_map: SampleMapArtifact = SampleMapArtifact {
    schema_version = 1,
    sample_map = SampleMap {
        declaration_id = "tests.project_sampler@1",
        selection = FirstRegion,
        voices = 2,
        regions = [SampleRegion {
            asset = "assets/tone.wav",
            key_low = 0, key_high = 127, root_key = 60,
            expression_low = 0/1, expression_high = 1/1,
            technique = "", connection = AnyConnection, trigger = AttackTrigger, pedal = AnyPedal,
            sequence_group = 0, sequence_position = 0, sequence_length = 0,
            weight = 1, priority = 1, tune_cents = 0/1,
            gain = 1/1, pan = 0/1,
            start_frame = 0, end_frame = 64,
            loop_start = 8, loop_end = 48, loop_mode = ForwardLoop,
            envelope = SampleEnvelope {
                attack_seconds = 0/1, decay_seconds = 0/1,
                sustain_level = 1/1, release_seconds = 1/100
            },
            exclusive_group = 0
        }]
    }
};
piece "Native map" {
    meter 4/4; key c major;
    score { part p { voice v { c4/4 } } }
}
"#,
    );
    write(&root.join("assets/tone.wav"), wav(0.25));
}

#[test]
fn a_project_prepares_only_a_checked_map_from_the_exact_verified_read() {
    let directory = tempfile::tempdir().expect("temporary project");
    project(directory.path());
    lock_assets(directory.path()).expect("asset lock");
    let session = ProjectSession::open(directory.path().join("piece.musa")).expect("open project");
    let prepared = session
        .prepare_sample_map("fixture_map", limits())
        .expect("sample preparation");
    assert_eq!(prepared.resources().decoded_pcm_bytes, 64 * size_of::<f32>());

    let asset = directory.path().join("assets/tone.wav");
    write(&asset, wav(0.75));
    let error = session
        .prepare_sample_map("fixture_map", limits())
        .err()
        .expect("the bytes changed after inventory verification");
    assert!(error.to_string().contains("changed content after verification"));
}
