//! Prepared recorded-media laws: bounded decode, fit semantics, deterministic
//! absolute-frame seek, and one offline/live reference operation.

#![allow(clippy::arithmetic_side_effects)]
#![allow(clippy::expect_used)]
#![allow(clippy::float_cmp)]

use std::sync::Arc;

use musa_compiler::{CompileOptions, SourceDocument, compile, lower_gestures};
use musa_dsp::{MediaLimits, prepare_media};

use crate::suite::audio_support::{RATE, options};

fn wav(samples: &[f32]) -> Arc<[u8]> {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: RATE,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };
    let mut bytes = std::io::Cursor::new(Vec::new());
    {
        let mut writer = hound::WavWriter::new(&mut bytes, spec).expect("wav writer");
        for sample in samples {
            writer.write_sample(*sample).expect("sample");
        }
        writer.finalize().expect("wav finish");
    }
    bytes.into_inner().into()
}

fn source(declaration: &str) -> String {
    format!(
        r#"piece "Recorded" {{
    meter 4/4;
    {declaration}
    score {{
        cue recording at 1:1;
        part proof {{ voice observed {{ rest/1 }} }}
    }}
}}"#
    )
}

fn prepared(declaration: &str, samples: &[f32]) -> (musa_compiler::Compilation, musa_dsp::PreparedMedia) {
    prepared_with_limits(declaration, samples, 64, 512)
}

fn prepared_with_limits(
    declaration: &str,
    samples: &[f32],
    max_decoded_frames: u64,
    max_decoded_bytes: u64,
) -> (musa_compiler::Compilation, musa_dsp::PreparedMedia) {
    let compilation = compile(
        &SourceDocument::new(source(declaration), "media.musa"),
        &CompileOptions::default(),
    );
    assert!(!compilation.has_errors(), "{:#?}", compilation.diagnostics());
    let score = compilation.snapshot().expect("score");
    let gestures = lower_gestures(score).expect("gestures");
    let media = prepare_media(
        score,
        gestures.tempo(),
        options(0).format,
        |_| Ok(wav(samples)),
        MediaLimits {
            max_assets: 1,
            max_decoded_frames,
            max_decoded_bytes,
        },
    )
    .expect("prepared media");
    (compilation, media)
}

pub(crate) fn fixed_audio(samples: &[f32]) -> musa_dsp::PreparedAudio {
    let (compilation, media) = prepared_with_limits(
        r#"fixed_media recording from "recording.wav";"#,
        samples,
        samples.len().saturating_mul(2) as u64,
        samples.len().saturating_mul(16) as u64,
    );
    let gestures = lower_gestures(compilation.snapshot().expect("score")).expect("gestures");
    let instruments = musa_compiler::checked_standard_instruments().expect("instruments");
    let machine = musa_compiler::checked_standard_instrument_machine().expect("instrument machine");
    let execution = musa_dsp::decode_studio_execution(compilation.studio_source().expect("studio source"))
        .expect("studio execution");
    musa_dsp::prepare_execution_with_media(&gestures, &instruments, &machine, &execution, media, options(0))
        .expect("audio")
}

#[test]
fn fixed_media_retains_natural_physical_duration_and_mono_becomes_stereo() {
    let (_, media) = prepared(r#"fixed_media recording from "recording.wav";"#, &[0.25, 0.5, -0.25]);
    assert_eq!(media.finish_frame(), 3);
    assert_eq!(media.frame(0), [0.25, 0.25]);
    assert_eq!(media.frame(1), [0.5, 0.5]);
    assert_eq!(media.frame(2), [-0.25, -0.25]);
    assert_eq!(media.frame(3), [0.0, 0.0]);
}

#[test]
fn crop_loop_and_rate_have_distinct_finite_readings() {
    // Quarter=120 makes 1/24000 whole notes four output frames.
    let (_, crop) = prepared(
        r#"clip recording from "recording.wav" fit 1/24000 by crop;"#,
        &[0.25, 0.5],
    );
    let (_, looping) = prepared(
        r#"clip recording from "recording.wav" fit 1/24000 by loop;"#,
        &[0.25, 0.5],
    );
    let (_, rate) = prepared(
        r#"clip recording from "recording.wav" fit 1/24000 by rate;"#,
        &[0.25, 0.5],
    );
    assert_eq!(
        (0..4).map(|frame| crop.frame(frame)[0]).collect::<Vec<_>>(),
        [0.25, 0.5, 0.0, 0.0]
    );
    assert_eq!(
        (0..4).map(|frame| looping.frame(frame)[0]).collect::<Vec<_>>(),
        [0.25, 0.5, 0.25, 0.5]
    );
    assert_eq!(
        (0..4).map(|frame| rate.frame(frame)[0]).collect::<Vec<_>>(),
        [0.25, 0.375, 0.5, 0.5]
    );
}

#[test]
fn prepared_audio_seek_reads_the_same_absolute_media_frame() {
    let samples = vec![0.1; 512];
    let mut audio = fixed_audio(&samples);
    let mut rendered = vec![0.0; 512 * 2];
    audio.render(&mut rendered);
    assert!(rendered.iter().any(|sample| sample.abs() > 0.05));

    audio.seek(128);
    let sought = audio.step();
    assert!(sought[0] > 0.09 && sought[1] > 0.09);
}

#[test]
fn media_render_is_independent_of_host_block_partition() {
    let samples = (0..512).map(|frame| frame as f32 / 512.0 - 0.5).collect::<Vec<_>>();
    let mut whole = fixed_audio(&samples);
    let mut partitioned = fixed_audio(&samples);
    let mut expected = vec![0.0; 1024];
    whole.render(&mut expected);
    let mut actual = Vec::with_capacity(expected.len());
    for frames in [17_usize, 3, 91, 1, 256, 144] {
        let mut block = vec![0.0; frames.saturating_mul(2)];
        partitioned.render(&mut block);
        actual.extend(block);
    }
    actual.truncate(expected.len());
    assert_eq!(actual, expected);
}

#[test]
fn decoded_memory_is_refused_before_a_machine_is_built() {
    let compilation = compile(
        &SourceDocument::new(source(r#"fixed_media recording from "recording.wav";"#), "media.musa"),
        &CompileOptions::default(),
    );
    let score = compilation.snapshot().expect("score");
    let gestures = lower_gestures(score).expect("gestures");
    let error = prepare_media(
        score,
        gestures.tempo(),
        options(0).format,
        |_| Ok(wav(&[0.0; 4])),
        MediaLimits {
            max_assets: 1,
            max_decoded_frames: 3,
            max_decoded_bytes: 512,
        },
    )
    .err()
    .expect("bound refusal");
    assert!(error.to_string().contains("decoded frame count"));
}

#[test]
fn an_unavailable_asset_is_refused_during_off_thread_preparation() {
    let compilation = compile(
        &SourceDocument::new(source(r#"fixed_media recording from "recording.wav";"#), "media.musa"),
        &CompileOptions::default(),
    );
    let score = compilation.snapshot().expect("score");
    let gestures = lower_gestures(score).expect("gestures");
    let error = prepare_media(
        score,
        gestures.tempo(),
        options(0).format,
        |_| Err("verified bytes were removed".to_owned()),
        MediaLimits {
            max_assets: 1,
            max_decoded_frames: 64,
            max_decoded_bytes: 512,
        },
    )
    .err()
    .expect("asset refusal");
    assert!(error.to_string().contains("recording.wav"));
    assert!(error.to_string().contains("verified bytes were removed"));
}

#[test]
fn repeated_fixed_cues_overlap_by_deterministic_addition() {
    let source = r#"piece "Overlapping recordings" {
    meter 4/4;
    fixed_media recording from "recording.wav";
    score {
        cue recording at 1:1;
        cue recording at 1:1;
        part proof { voice observed { rest/1 } }
    }
}"#;
    let compilation = compile(
        &SourceDocument::new(source, "overlapping-media.musa"),
        &CompileOptions::default(),
    );
    assert!(!compilation.has_errors(), "{:#?}", compilation.diagnostics());
    let score = compilation.snapshot().expect("score");
    let gestures = lower_gestures(score).expect("gestures");
    let media = prepare_media(
        score,
        gestures.tempo(),
        options(0).format,
        |_| Ok(wav(&[0.25, 0.5])),
        MediaLimits {
            max_assets: 1,
            max_decoded_frames: 2,
            max_decoded_bytes: 16,
        },
    )
    .expect("media");

    assert_eq!(media.frame(0), [0.5, 0.5]);
    assert_eq!(media.frame(1), [1.0, 1.0]);
    assert_eq!(media.finish_frame(), 2);
}

#[test]
fn named_media_uses_the_authored_route_and_send_graph() {
    fn render(studio: &str) -> Vec<f32> {
        let source = format!(
            r#"piece "Routed recording" {{
    meter 4/4;
    fixed_media recording from "recording.wav";
    score {{ cue recording at 1:1; part proof {{ voice observed {{ rest/1 }} }} }}
    studio {{ {studio} }}
}}"#
        );
        let compilation = compile(
            &SourceDocument::new(source, "routed-media.musa"),
            &CompileOptions::default(),
        );
        assert!(!compilation.has_errors(), "{:#?}", compilation.diagnostics());
        let score = compilation.snapshot().expect("score");
        let gestures = lower_gestures(score).expect("gestures");
        let media = prepare_media(
            score,
            gestures.tempo(),
            options(0).format,
            |_| Ok(wav(&[0.25; 1024])),
            MediaLimits {
                max_assets: 1,
                max_decoded_frames: 2048,
                max_decoded_bytes: 16_384,
            },
        )
        .expect("media");
        let execution = musa_dsp::decode_studio_execution(compilation.studio_source().expect("studio source"))
            .expect("studio execution");
        let instruments = musa_compiler::checked_standard_instruments().expect("instruments");
        let machine = musa_compiler::checked_standard_instrument_machine().expect("machine");
        let mut audio =
            musa_dsp::prepare_execution_with_media(&gestures, &instruments, &machine, &execution, media, options(0))
                .expect("audio");
        let mut samples = vec![0.0; 2048];
        audio.render(&mut samples);
        samples
    }

    let direct = render("route recording -> master;");
    let sent = render("bus quiet { gain(-6 dB) |> output; } send recording -> quiet at -6 dB; route quiet -> master;");
    let peak = |samples: &[f32]| samples.iter().fold(0.0_f32, |peak, sample| peak.max(sample.abs()));
    assert!(peak(&direct) > 0.2);
    assert!(peak(&sent) > 0.04);
    assert!(peak(&sent) < peak(&direct) * 0.6);
}
