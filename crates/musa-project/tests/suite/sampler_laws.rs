//! The project owns verified bytes; DSP owns decoding and sampler state.
#![allow(clippy::expect_used)]

use std::fmt::Write as _;
use std::path::Path;

use musa_compiler::{CompileOptions, SourceDocument, compile, lower_gestures};
use musa_dsp::{
    AudioFormat, ChannelLayout, CollapsePolicy, EventMessage, FrameRounding, MessageKind, ScheduleLimits,
    SchedulePolicy, TimeMap, schedule,
};
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
    schema_version = 3,
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
            gain = LinearGain(1/1), expression_gain = LinearExpressionGain, pan = 0/1,
            start_frame = 0, end_frame = 64,
            loop_start = 8, loop_end = 48, loop_mode = ForwardSustainLoop,
            envelope = SampleEnvelope {
                delay = ExactSeconds(0/1), attack = ExactSeconds(0/1), hold = ExactSeconds(0/1),
                decay = ExactSeconds(0/1), sustain = LinearLevel(1/1), release = ExactSeconds(1/100),
                hold_key_timecents = 0/1, decay_key_timecents = 0/1, curve = LinearEnvelope
            },
            filter = SampleFilter { cutoff_cents = 13500/1, resonance_centibels = 0/1 },
            modulations = [],
            group = 0, off_by = 0, off_mode = FastOff
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

fn sfz_limits() -> musa_project::SfzLimits {
    musa_project::SfzLimits {
        max_file_bytes: 64 * 1024,
        max_regions: 32,
        max_opcodes: 256,
        max_value_bytes: 1024,
    }
}

fn sfz_sampler_limits() -> musa_dsp::SamplerLimits {
    musa_dsp::SamplerLimits {
        max_voices: 64,
        max_step_work: 64 * 48,
        ..limits()
    }
}

fn sfz_project(root: &Path, fixture: &str) {
    let mut manifest = String::from(
        r#"[project]
name = "SFZ adapter fixture"

[assets."assets/instrument.sfz"]
kind = "sfz"
adapter = "sfz@1"
max_bytes = 65536
"#,
    );
    for name in [
        "round-a",
        "round-b",
        "layer",
        "release",
        "pedal-release",
        "key-release",
        "loop",
        "tone",
    ] {
        writeln!(
            manifest,
            "\n[assets.\"assets/samples/{name}.wav\"]\nkind = \"audio\"\nadapter = \"wav@1\"\nmax_bytes = 1048576"
        )
        .expect("manifest string write");
        write(&root.join(format!("assets/samples/{name}.wav")), wav(0.125));
    }
    write(&root.join("musa.toml"), manifest);
    write(
        &root.join("piece.musa"),
        r#"instrument imported from "assets/instrument.sfz" conforms note_instrument;
piece "SFZ fixture" { meter 4/4; key c major; score { part p { voice v { c4/4 } } } }
"#,
    );
    write(&root.join("assets/instrument.sfz"), fixture);
}

#[test]
fn sfz_inheritance_layers_sequences_and_release_regions_cross_one_checked_map() {
    let directory = tempfile::tempdir().expect("temporary project");
    sfz_project(directory.path(), include_str!("../fixtures/sfz/layered.sfz"));
    lock_assets(directory.path()).expect("asset lock");
    let session = ProjectSession::open(directory.path().join("piece.musa")).expect("open project");
    assert!(session.snapshot().compiles(), "{:?}", session.snapshot().diagnostics());
    let (prepared, facts) = session
        .prepare_sfz_instrument("imported", sfz_limits(), sfz_sampler_limits())
        .expect("strict SFZ adaptation");
    assert_eq!(facts.adapter, "sfz@1");
    assert_eq!(facts.regions, 7);
    assert_eq!(facts.warnings.len(), 2);
    assert_eq!(prepared.resources().regions, 7);
    assert!(!prepared.exact_source_bytes().is_empty());

    let compilation = compile(
        &SourceDocument::new(
            "piece \"gesture\" { meter 4/4; key c major; score { part p { voice v { c4/4 } } } }",
            "sfz-gesture.musa",
        ),
        &CompileOptions::default(),
    );
    let gestures = lower_gestures(compilation.snapshot().expect("gesture score")).expect("gesture lowering");
    let lane = gestures.lanes().first().expect("one lane");
    let gesture = lane
        .track()
        .occurrences()
        .first()
        .map(|occurrence| occurrence.payload())
        .expect("one gesture");
    let token = prepared.selector(7, 11).select(gesture, "").expect("selection token");
    let schedule_policy = SchedulePolicy::new(
        1,
        FrameRounding::NearestTiesLater,
        CollapsePolicy::Ordered,
        [MessageKind::End, MessageKind::Point, MessageKind::Begin],
        ScheduleLimits {
            max_frame: 48_000,
            max_time_map_entries: 8,
            max_occurrences: 8,
            max_messages: 16,
            max_batches: 16,
        },
    )
    .expect("schedule policy");
    let assignments = lane
        .track()
        .occurrences()
        .iter()
        .flat_map(|occurrence| [occurrence.span().start(), occurrence.span().end()])
        .chain([lane.track().duration().reach()])
        .map(|position| (position, lane.physical(position)));
    let scheduled = schedule(
        AudioFormat::new(
            std::num::NonZeroU32::new(48_000).expect("sample rate"),
            ChannelLayout::Stereo,
        ),
        schedule_policy,
        &TimeMap::new(1, assignments),
        lane.track(),
    )
    .expect("gesture schedule");
    let handle = scheduled
        .batches()
        .flat_map(|(_, batch)| batch.messages())
        .find_map(|message| match message {
            EventMessage::Begin(handle, _) => Some(handle.clone()),
            EventMessage::End(_) | EventMessage::Point(_, _) => None,
        })
        .expect("begin handle");
    let render = || {
        let mut runtime = prepared.runtime();
        runtime.note_on(&handle, &token);
        let mut output = vec![0.0; 128];
        runtime.render(&mut output);
        output
    };
    let first = render();
    assert_eq!(first, render(), "the adapted source has one deterministic runtime");
    assert!(
        first.first().is_some_and(|sample| *sample > 0.07),
        "the unsequenced microphone region layers with one round-robin region"
    );
}

#[test]
fn unsupported_sound_changing_sfz_opcodes_are_positioned_errors() {
    let directory = tempfile::tempdir().expect("temporary project");
    sfz_project(directory.path(), include_str!("../fixtures/sfz/unsupported.sfz"));
    lock_assets(directory.path()).expect("asset lock");
    let session = ProjectSession::open(directory.path().join("piece.musa")).expect("open project");
    let error = session
        .prepare_sfz_instrument("imported", sfz_limits(), sfz_sampler_limits())
        .err()
        .expect("cutoff is outside sfz@1");
    let message = error.to_string();
    assert!(message.contains("cutoff"), "{message}");
    assert!(message.contains("at 1:"), "{message}");
}

#[test]
fn sfz_directives_and_sample_paths_cannot_escape_the_strict_adapter_boundary() {
    for (fixture, expected) in [
        ("#include \"other.sfz\"\n", "directives, includes, macros"),
        (
            "<region> sample=../../outside.wav key=60\n",
            "escapes its owning asset root",
        ),
        (
            "<region> sample=samples/tone.wav key=60 ampeg_release=-0.1\n",
            "`ampeg_release` must not be negative",
        ),
    ] {
        let directory = tempfile::tempdir().expect("temporary project");
        sfz_project(directory.path(), fixture);
        lock_assets(directory.path()).expect("asset lock");
        let session = ProjectSession::open(directory.path().join("piece.musa")).expect("open project");
        let message = session
            .prepare_sfz_instrument("imported", sfz_limits(), sfz_sampler_limits())
            .err()
            .expect("the strict adapter must reject the foreign construct")
            .to_string();
        assert!(message.contains(expected), "{message}");
    }
}

#[test]
fn every_sfz_parser_resource_has_an_explicit_enforced_bound() {
    let fixture = include_str!("../fixtures/sfz/layered.sfz");
    for (limits, expected) in [
        (
            musa_project::SfzLimits {
                max_file_bytes: 1,
                ..sfz_limits()
            },
            "explicit SFZ limit",
        ),
        (
            musa_project::SfzLimits {
                max_regions: 1,
                ..sfz_limits()
            },
            "regions, above",
        ),
        (
            musa_project::SfzLimits {
                max_opcodes: 1,
                ..sfz_limits()
            },
            "opcode count",
        ),
        (
            musa_project::SfzLimits {
                max_value_bytes: 1,
                ..sfz_limits()
            },
            "value for",
        ),
    ] {
        let directory = tempfile::tempdir().expect("temporary project");
        sfz_project(directory.path(), fixture);
        lock_assets(directory.path()).expect("asset lock");
        let session = ProjectSession::open(directory.path().join("piece.musa")).expect("open project");
        let message = session
            .prepare_sfz_instrument("imported", limits, sfz_sampler_limits())
            .err()
            .expect("the narrowed bound must refuse adaptation")
            .to_string();
        assert!(message.contains(expected), "{message}");
    }
}
