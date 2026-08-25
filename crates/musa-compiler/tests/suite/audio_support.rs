#![allow(clippy::expect_used)]
#![allow(clippy::arithmetic_side_effects)]

use musa_compiler::{
    CompileOptions, SourceDocument, checked_standard_instrument_machine, checked_standard_instruments, compile,
};
use musa_dsp::{
    AudioFormat, AudioLimits, AudioOptions, ChannelLayout, CollapsePolicy, FrameRounding, MessageKind, PreparedAudio,
    ScheduleLimits, SchedulePolicy, StudioSpec,
};
use musa_score::{ScoreSnapshot, Tuning, lower_gestures};

pub(crate) const RATE: u32 = 48_000;

pub(crate) fn parts(source: &str) -> (ScoreSnapshot, StudioSpec) {
    let compilation = compile(&SourceDocument::new(source, "test.musa"), &CompileOptions::default());
    assert!(!compilation.has_errors(), "{:?}", compilation.diagnostics());
    let (score, studio) = compilation.into_parts();
    (score.expect("score"), studio)
}

pub(crate) fn options(tail_frames: u64) -> AudioOptions {
    let maximum = u64::from(RATE).saturating_mul(60 * 60);
    AudioOptions {
        format: AudioFormat::new(
            std::num::NonZeroU32::new(RATE).expect("nonzero rate"),
            ChannelLayout::Stereo,
        ),
        schedule: SchedulePolicy::new(
            1,
            FrameRounding::NearestTiesLater,
            CollapsePolicy::Ordered,
            [MessageKind::End, MessageKind::Point, MessageKind::Begin],
            ScheduleLimits {
                max_frame: maximum,
                max_time_map_entries: 20_001,
                max_occurrences: 10_000,
                max_messages: 20_000,
                max_batches: 20_000,
            },
        )
        .expect("policy"),
        tuning: Tuning::default(),
        render_seed: 0x4D55_5341,
        limits: AudioLimits {
            max_primitives: 10_000,
            max_state_bytes: 1 << 30,
            max_step_work: 10_000_000,
        },
        tail_frames,
        max_total_frames: maximum.saturating_add(tail_frames),
    }
}

pub(crate) fn prepare(source: &str, tail_frames: u64) -> PreparedAudio {
    let (score, studio) = parts(source);
    let gestures = lower_gestures(&score).expect("gestures");
    prepare_gestures(&gestures, &studio, options(tail_frames)).expect("audio")
}

pub(crate) fn prepare_gestures(
    gestures: &musa_score::GesturePlan,
    studio: &StudioSpec,
    options: AudioOptions,
) -> Result<PreparedAudio, musa_dsp::AudioPrepareError> {
    let diagnostic = |diagnostics: Vec<musa_score::Diagnostic>| {
        musa_dsp::AudioPrepareError::InstrumentContract(format!("{diagnostics:#?}"))
    };
    let instruments = checked_standard_instruments().map_err(diagnostic)?;
    let instrument_machine = checked_standard_instrument_machine().map_err(diagnostic)?;
    musa_dsp::prepare_execution(gestures, &instruments, &instrument_machine, studio, options)
}

pub(crate) fn render_source(source: &str, frames: usize) -> Vec<f32> {
    let mut audio = prepare(source, frames as u64);
    let mut output = vec![0.0; frames.saturating_mul(2)];
    audio.render(&mut output);
    output
}
