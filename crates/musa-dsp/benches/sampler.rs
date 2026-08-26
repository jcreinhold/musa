//! Preloaded sample-instrument frame-step benchmark.
#![allow(clippy::arithmetic_side_effects)]
#![allow(clippy::expect_used)]

use std::sync::Arc;

use musa_compiler::{CompileOptions, SourceDocument, checked_source_value, compile, lower_gestures};
use musa_dsp::{
    AudioFormat, ChannelLayout, CollapsePolicy, EventMessage, FrameRounding, MessageKind, SampleRuntime, SamplerLimits,
    ScheduleLimits, SchedulePolicy, TimeMap, decode_sample_map, prepare_sample_map, sample_map_schema, schedule,
};

#[global_allocator]
static ALLOC: divan::AllocProfiler = divan::AllocProfiler::system();

const RATE: u32 = 48_000;
const SOURCE: &str = r#"import std::sound::sample;
let benchmark_map: SampleMapArtifact = SampleMapArtifact {
    schema_version = 2,
    sample_map = SampleMap {
        declaration_id = "bench.sampler@1", selection = FirstRegion, voices = 16,
        regions = [SampleRegion {
            asset = "tone.wav", key_low = 0, key_high = 127, root_key = 60,
            expression_low = 0/1, expression_high = 1/1, technique = "", connection = AnyConnection,
            trigger = AttackTrigger, pedal = AnyPedal,
            sequence_group = 0, sequence_position = 0, sequence_length = 0,
            weight = 1, priority = 1, tune_cents = 0/1, gain = LinearGain(1/1), pan = 0/1,
            start_frame = 0, end_frame = 2048, loop_start = 128, loop_end = 1920,
            loop_mode = ForwardSustainLoop,
            envelope = SampleEnvelope {
                attack_seconds = 1/1000, decay_seconds = 1/100,
                sustain_level = 4/5, release_seconds = 1/20, curve = LinearEnvelope
            }, group = 0, off_by = 0, off_mode = FastOff
        }]
    }
};
piece "Sampler benchmark" { meter 4/4; key c major; score { part p { voice v { c4/1 } } } }
"#;

fn wav() -> Arc<[u8]> {
    let mut bytes = std::io::Cursor::new(Vec::new());
    {
        let mut writer = hound::WavWriter::new(
            &mut bytes,
            hound::WavSpec {
                channels: 1,
                sample_rate: RATE,
                bits_per_sample: 32,
                sample_format: hound::SampleFormat::Float,
            },
        )
        .expect("wav writer");
        for frame in 0..2048 {
            let phase = std::f32::consts::TAU * 220.0 * frame as f32 / RATE as f32;
            writer.write_sample(phase.sin() * 0.25).expect("sample");
        }
        writer.finalize().expect("wav finish");
    }
    bytes.into_inner().into()
}

fn input() -> (SampleRuntime, Box<[f32]>) {
    let document = SourceDocument::new(SOURCE, "sampler-benchmark.musa");
    let checked = checked_source_value(
        &document,
        &CompileOptions::default(),
        "benchmark_map",
        &sample_map_schema(),
    )
    .expect("checked sampler map");
    let map = decode_sample_map(&checked).expect("sample map projection");
    let prepared = prepare_sample_map(
        map,
        |_| Ok(wav()),
        SamplerLimits {
            sample_rate: RATE,
            max_voices: 32,
            max_regions: 32,
            max_decoded_bytes: 1 << 20,
            max_selection_work: 32,
            max_step_work: 10_000,
        },
    )
    .expect("sample preparation");
    let compilation = compile(&document, &CompileOptions::default());
    let gestures = lower_gestures(compilation.snapshot().expect("score")).expect("gestures");
    let lane = gestures.lanes().first().expect("lane");
    let gesture = lane.track().occurrences().first().expect("occurrence").payload();
    let token = prepared.selector(1, 1).select(gesture, "").expect("selection");
    let format = AudioFormat::new(
        std::num::NonZeroU32::new(RATE).expect("sample rate"),
        ChannelLayout::Stereo,
    );
    let policy = SchedulePolicy::new(
        1,
        FrameRounding::NearestTiesLater,
        CollapsePolicy::Ordered,
        [MessageKind::End, MessageKind::Point, MessageKind::Begin],
        ScheduleLimits {
            max_frame: u64::from(RATE) * 60,
            max_time_map_entries: 8,
            max_occurrences: 8,
            max_messages: 16,
            max_batches: 16,
        },
    )
    .expect("policy");
    let assignments = lane
        .track()
        .occurrences()
        .iter()
        .flat_map(|occurrence| [occurrence.span().start(), occurrence.span().end()])
        .chain([lane.track().duration().reach()])
        .map(|position| (position, lane.physical(position)));
    let scheduled = schedule(format, policy, &TimeMap::new(1, assignments), lane.track()).expect("schedule");
    let handle = scheduled
        .batches()
        .flat_map(|(_, batch)| batch.messages())
        .find_map(|message| match message {
            EventMessage::Begin(handle, _) => Some(handle.clone()),
            EventMessage::End(_) | EventMessage::Point(_, _) => None,
        })
        .expect("begin handle");
    let mut runtime = prepared.runtime();
    runtime.note_on(&handle, &token);
    (runtime, vec![0.0; 8192].into_boxed_slice())
}

#[divan::bench(sample_count = 30)]
fn render_preloaded_sampler(bencher: divan::Bencher<'_, '_>) {
    bencher.with_inputs(input).bench_values(|(mut sampler, mut output)| {
        sampler.render(&mut output);
        divan::black_box(output);
    });
}

fn main() {
    divan::main();
}
