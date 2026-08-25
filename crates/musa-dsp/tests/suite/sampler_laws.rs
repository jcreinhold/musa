//! Native sample maps remain source values; only their checked projection and
//! fixed real-time state live here.
#![allow(clippy::arithmetic_side_effects)]
#![allow(clippy::expect_used)]

use std::sync::Arc;

use musa_compiler::{CompileOptions, SourceDocument, checked_source_value, compile, lower_gestures};
use musa_dsp::{PreparedSampleMap, SamplerLimits, decode_sample_map, prepare_sample_map, sample_map_schema};

use crate::schedule::EventHandle;

const RATE: u32 = 48_000;

fn region(
    asset: &str,
    trigger: &str,
    pedal: &str,
    sequence_group: u64,
    sequence_position: u64,
    sequence_length: u64,
    weight: u64,
    priority: u64,
    loop_mode: &str,
    loop_start: u64,
    loop_end: u64,
) -> String {
    format!(
        r#"SampleRegion {{
            asset = "{asset}", key_low = 0, key_high = 127, root_key = 60,
            expression_low = 0/1, expression_high = 1/1, technique = "", connection = AnyConnection,
            trigger = {trigger}, pedal = {pedal},
            sequence_group = {sequence_group}, sequence_position = {sequence_position},
            sequence_length = {sequence_length}, weight = {weight}, priority = {priority},
            tune_cents = 0/1, gain = 1/1, pan = 0/1,
            start_frame = 0, end_frame = 64, loop_start = {loop_start}, loop_end = {loop_end},
            loop_mode = {loop_mode},
            envelope = SampleEnvelope {{
                attack_seconds = 0/1, decay_seconds = 0/1,
                sustain_level = 1/1, release_seconds = 1/1000
            }},
            exclusive_group = 0
        }}"#
    )
}

fn artifact_with_regions(selection: &str, regions: &[String]) -> musa_compiler::CheckedSource {
    let regions = regions.join(",\n");
    let source = format!(
        r#"import std::sound::sample;
let test_map: SampleMapArtifact = SampleMapArtifact {{
    schema_version = 1,
    sample_map = SampleMap {{
        declaration_id = "tests.native_sampler@1",
        selection = {selection},
        voices = 2,
        regions = [{regions}]
    }}
}};
piece "Native sampler" {{
    meter 4/4; key c major;
    score {{ part proof {{ voice observed {{ c4/4 }} }} }}
}}
"#
    );
    checked_source_value(
        &SourceDocument::new(&source, "native-sampler-map.musa"),
        &CompileOptions::default(),
        "test_map",
        &sample_map_schema(),
    )
    .expect("sample map source value should check")
}

fn artifact(selection: &str) -> musa_compiler::CheckedSource {
    artifact_with_regions(
        selection,
        &[
            region(
                "a.wav",
                "AttackTrigger",
                "AnyPedal",
                1,
                1,
                2,
                1,
                10,
                "ForwardLoop",
                8,
                48,
            ),
            region(
                "b.wav",
                "AttackTrigger",
                "AnyPedal",
                1,
                2,
                2,
                3,
                10,
                "ForwardLoop",
                8,
                48,
            ),
            region("up.wav", "ReleaseTrigger", "PedalUp", 2, 1, 1, 1, 10, "NoLoop", 0, 0),
            region(
                "down.wav",
                "ReleaseTrigger",
                "PedalDown",
                3,
                1,
                1,
                1,
                10,
                "NoLoop",
                0,
                0,
            ),
        ],
    )
}

fn wav(value: f32) -> Arc<[u8]> {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: RATE,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };
    let mut bytes = std::io::Cursor::new(Vec::new());
    {
        let mut writer = hound::WavWriter::new(&mut bytes, spec).expect("wav writer");
        for _ in 0..64 {
            writer.write_sample(value).expect("sample");
        }
        writer.finalize().expect("wav finish");
    }
    bytes.into_inner().into()
}

fn gesture_plan_for(pitch: &str) -> musa_score::GesturePlan {
    let source =
        format!("piece \"gesture\" {{ meter 4/4; key c major; score {{ part p {{ voice v {{ {pitch}/4 }} }} }} }}");
    let compilation = compile(
        &SourceDocument::new(source, "sampler-gesture.musa"),
        &CompileOptions::default(),
    );
    assert!(!compilation.has_errors(), "{:?}", compilation.diagnostics());
    lower_gestures(compilation.snapshot().expect("score")).expect("gestures")
}

fn quiet_gesture_plan(pitch: &str) -> musa_score::GesturePlan {
    let source = format!(
        "piece \"gesture\" {{ tempo 1/4 = 60; meter 4/4; key c major; \
         performance {{ profile quiet {{ dynamic p {{ amplitude = 1/4; }} }} }} \
         score {{ part p {{ profile quiet; voice v {{ dynamic p; {pitch}/4 }} }} }} }}"
    );
    let compilation = compile(
        &SourceDocument::new(source, "quiet-sampler-gesture.musa"),
        &CompileOptions::default(),
    );
    assert!(!compilation.has_errors(), "{:?}", compilation.diagnostics());
    lower_gestures(compilation.snapshot().expect("score")).expect("gestures")
}

pub(crate) fn gesture_plan() -> musa_score::GesturePlan {
    gesture_plan_for("c4")
}

pub(crate) fn first_gesture(plan: &musa_score::GesturePlan) -> &musa_score::Gesture {
    plan.lanes()
        .first()
        .and_then(|lane| lane.track().occurrences().first())
        .map(musa_events::Occurrence::payload)
        .expect("one gesture")
}

fn limits() -> SamplerLimits {
    SamplerLimits {
        sample_rate: RATE,
        max_voices: 8,
        max_regions: 32,
        max_decoded_bytes: 1 << 20,
        max_selection_work: 32,
        max_step_work: 1_000,
    }
}

pub(crate) fn prepared() -> PreparedSampleMap {
    let map = decode_sample_map(&artifact("RoundRobin")).expect("checked map projection");
    prepare_sample_map(
        map,
        |path| {
            Ok(match path {
                "a.wav" => wav(0.25),
                "b.wav" => wav(0.75),
                "up.wav" => wav(-0.25),
                "down.wav" => wav(-0.75),
                _ => return Err(format!("unknown asset {path}")),
            })
        },
        limits(),
    )
    .expect("prepared map")
}

#[test]
fn checked_source_is_the_complete_sample_map_authority() {
    let source = artifact("RoundRobin");
    let map = decode_sample_map(&source).expect("projection");
    assert_eq!(map.declaration_id(), "tests.native_sampler@1");
    assert_eq!(map.region_count(), 4);
    assert_eq!(map.exact_source_bytes(), source.exact_bytes());
    let prepared = prepared();
    assert_eq!(prepared.exact_source_bytes(), source.exact_bytes());
    assert_eq!(prepared.decoded_bytes(), 4 * 64 * size_of::<f32>());
    assert_eq!(prepared.resources().decoded_pcm_bytes, prepared.decoded_bytes());
    assert_eq!(prepared.resources().voices, 2);
}

#[test]
fn round_robin_is_per_instance_and_block_partition_cannot_change_sound() {
    let prepared = prepared();
    let gestures = gesture_plan();
    let gesture = first_gesture(&gestures);
    let mut selector = prepared.selector(7, 11);
    let first = selector.select(gesture, "").expect("first token");
    let second = selector.select(gesture, "").expect("second token");
    assert_ne!(first, second, "the sequence position belongs to the instance");
    assert_eq!(
        first,
        prepared.selector(7, 11).select(gesture, "").expect("fresh instance")
    );

    let handle = EventHandle::root(0);
    let mut whole = prepared.runtime();
    whole.note_on(&handle, first);
    let mut one = vec![0.0; 512];
    whole.render(&mut one);

    let mut split = prepared.runtime();
    split.note_on(&handle, first);
    let mut many = vec![0.0; 512];
    for chunk in many.chunks_mut(14) {
        split.render(chunk);
    }
    assert_eq!(one, many);
    assert!(one.iter().all(|sample| sample.is_finite()));
}

#[test]
fn looping_release_pedal_and_voice_stealing_are_bounded_and_deterministic() {
    let prepared = prepared();
    let gestures = gesture_plan();
    let gesture = first_gesture(&gestures);
    let mut selector = prepared.selector(9, 3);
    let first = selector.select(gesture, "").expect("token");
    let second = selector.select(gesture, "").expect("token");
    let first_handle = EventHandle::root(0);
    let second_handle = EventHandle::root(1);
    let mut runtime = prepared.runtime();
    runtime.note_on(&first_handle, first);
    let mut beyond_asset = vec![0.0; 512];
    runtime.render(&mut beyond_asset);
    assert!(
        beyond_asset.iter().any(|sample| *sample != 0.0),
        "held loop must survive the source end"
    );

    runtime.set_pedal(true);
    runtime.note_off(&first_handle);
    let sustained = runtime.step();
    assert!(sustained.iter().any(|sample| sample.abs() > f32::EPSILON));
    runtime.set_pedal(false);
    runtime.note_on(&second_handle, second);
    runtime.note_on(&EventHandle::root(2), first);
    let mut tail = vec![0.0; 512];
    runtime.render(&mut tail);
    assert!(tail.iter().all(|sample| sample.is_finite()));
}

#[test]
fn stable_weighted_selection_is_seeded_by_realization_and_semantic_identity() {
    let map = decode_sample_map(&artifact("StableWeighted")).expect("projection");
    let prepared =
        prepare_sample_map(map, |path| Ok(wav(if path == "a.wav" { 0.25 } else { 0.75 })), limits()).expect("prepared");
    let gestures = gesture_plan();
    let gesture = first_gesture(&gestures);
    let left = prepared.selector(17, 5).select(gesture, "").expect("selection");
    let right = prepared.selector(17, 5).select(gesture, "").expect("selection");
    assert_eq!(left, right);
}

#[test]
fn pitch_rate_and_instrument_swapping_change_private_audio_not_source_gestures() {
    let map = decode_sample_map(&artifact("FirstRegion")).expect("projection");
    let prepare = |a_scale: f32| {
        prepare_sample_map(
            map.clone(),
            |path| {
                if path == "a.wav" {
                    let spec = hound::WavSpec {
                        channels: 1,
                        sample_rate: RATE,
                        bits_per_sample: 32,
                        sample_format: hound::SampleFormat::Float,
                    };
                    let mut bytes = std::io::Cursor::new(Vec::new());
                    {
                        let mut writer = hound::WavWriter::new(&mut bytes, spec).expect("wav writer");
                        for frame in 0..64 {
                            writer.write_sample(frame as f32 / 64.0 * a_scale).expect("sample");
                        }
                        writer.finalize().expect("wav finish");
                    }
                    Ok(bytes.into_inner().into())
                } else {
                    Ok(wav(0.0))
                }
            },
            limits(),
        )
        .expect("prepared")
    };
    let root = gesture_plan_for("c4");
    let octave = gesture_plan_for("c5");
    let root_gesture = first_gesture(&root);
    let octave_gesture = first_gesture(&octave);
    let first = prepare(1.0);
    let root_token = first.selector(1, 1).select(root_gesture, "").expect("root");
    let octave_token = first.selector(1, 1).select(octave_gesture, "").expect("octave");
    let render = |prepared: &PreparedSampleMap, token| {
        let mut runtime = prepared.runtime();
        runtime.note_on(&EventHandle::root(0), token);
        let mut output = vec![0.0; 32];
        runtime.render(&mut output);
        output
    };
    let at_root = render(&first, root_token);
    let at_octave = render(&first, octave_token);
    assert_ne!(
        at_root, at_octave,
        "the sounding/root ratio must set the resampling rate"
    );

    let replacement = prepare(0.5);
    let replacement_token = replacement
        .selector(1, 1)
        .select(root_gesture, "")
        .expect("replacement token");
    assert_ne!(at_root, render(&replacement, replacement_token));
    assert_eq!(
        root_gesture,
        first_gesture(&root),
        "instrument preparation cannot rewrite gestures"
    );
}

#[test]
fn key_expression_and_custom_technique_predicates_select_only_applicable_regions() {
    let low = region("a.wav", "AttackTrigger", "AnyPedal", 0, 0, 0, 1, 1, "NoLoop", 0, 0)
        .replace(
            "key_low = 0, key_high = 127, root_key = 60",
            "key_low = 0, key_high = 60, root_key = 60",
        )
        .replace(
            "expression_low = 0/1, expression_high = 1/1",
            "expression_low = 0/1, expression_high = 1/2",
        )
        .replace("technique = \"\"", "technique = \"pizzicato\"");
    let high = region("b.wav", "AttackTrigger", "AnyPedal", 0, 0, 0, 1, 1, "NoLoop", 0, 0)
        .replace(
            "key_low = 0, key_high = 127, root_key = 60",
            "key_low = 61, key_high = 127, root_key = 72",
        )
        .replace(
            "expression_low = 0/1, expression_high = 1/1",
            "expression_low = 1/2, expression_high = 1/1",
        );
    let map = decode_sample_map(&artifact_with_regions("FirstRegion", &[low, high])).expect("layered projection");
    let prepared = prepare_sample_map(map, |path| Ok(wav(if path == "a.wav" { 0.25 } else { 0.75 })), limits())
        .expect("layered map");
    let quiet = quiet_gesture_plan("c4");
    let quiet = first_gesture(&quiet);
    let no_technique = prepared
        .selector(1, 1)
        .select(quiet, "")
        .expect("empty selection token");
    let pizzicato = prepared
        .selector(1, 1)
        .select(quiet, "pizzicato")
        .expect("pizzicato token");
    let render = |token| {
        let mut runtime = prepared.runtime();
        runtime.note_on(&EventHandle::root(0), token);
        runtime.step()
    };
    assert!(render(no_technique).iter().all(|sample| sample.abs() <= f32::EPSILON));
    assert!(render(pizzicato).iter().any(|sample| sample.abs() > f32::EPSILON));

    let loud_high = gesture_plan_for("c5");
    let high_token = prepared
        .selector(1, 1)
        .select(first_gesture(&loud_high), "")
        .expect("high layer");
    let high_frame = render(high_token);
    assert!(high_frame.iter().all(|sample| *sample > 0.4));
}

#[test]
fn malformed_assets_and_every_preload_budget_fail_before_runtime() {
    let map = decode_sample_map(&artifact("RoundRobin")).expect("projection");
    let error = prepare_sample_map(map.clone(), |_| Ok(Arc::from([0_u8; 8])), limits())
        .err()
        .expect("bad wav");
    assert!(error.to_string().contains("deterministic PCM"));
    let mut bounded = limits();
    bounded.max_decoded_bytes = 1;
    let error = prepare_sample_map(map, |_| Ok(wav(0.0)), bounded)
        .err()
        .expect("memory bound");
    assert!(error.to_string().contains("decoded PCM bytes"));

    let map = decode_sample_map(&artifact("RoundRobin")).expect("projection");
    bounded = limits();
    bounded.max_step_work = 1;
    let error = prepare_sample_map(map, |_| Ok(wav(0.0)), bounded)
        .err()
        .expect("step-work bound");
    assert!(error.to_string().contains("one-frame work"));
}
