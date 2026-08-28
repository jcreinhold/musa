//! The compile → exact gestures → prepared-audio chain, used both ways.
//!
//! Roadmap §13.8: an offline render and a live performance must come from
//! the same preparation, or "export what I hear" is a lie. [`build`] is that
//! single preparation; [`prepare`] hands it to the engine and [`to_wav`]
//! runs it offline.

use musa_compiler::lower_gestures;
use musa_playback::PreparedPlaybackPlan;
use musa_score::{PerformanceOptions, ScoreSnapshot};

use crate::error::ProjectError;

/// The sample rate at which checked scheduling and audio preparation agree.
pub(crate) fn sample_rate() -> u32 {
    PerformanceOptions::default().sample_rate
}

/// The seed every render of this project is taken with.
///
/// Named rather than spelled in place because an export that records the
/// arguments it was produced under has to be able to state it.
pub(crate) const RENDER_SEED: u64 = 0x4D55_5341;

/// Exact gesture lowering → checked scheduling → prepared audio machine.
pub(crate) fn build(
    score: &ScoreSnapshot,
    studio: &musa_dsp::StudioExecution,
    assets: &crate::assets::AssetInventory,
) -> Result<musa_dsp::PreparedAudio, ProjectError> {
    let gestures = lower_gestures(score).map_err(|e| ProjectError::Performance(e.to_string()))?;
    let instruments = musa_compiler::checked_standard_instruments()
        .map_err(|diagnostics| ProjectError::Performance(format!("{diagnostics:#?}")))?;
    let instrument_machine = musa_compiler::checked_standard_instrument_machine()
        .map_err(|diagnostics| ProjectError::Performance(format!("{diagnostics:#?}")))?;
    let sample_rate = sample_rate();
    let rate = std::num::NonZeroU32::new(sample_rate)
        .ok_or_else(|| ProjectError::Performance("the audio sample rate must be nonzero".to_owned()))?;
    let format = musa_dsp::AudioFormat::new(rate, musa_dsp::ChannelLayout::Stereo);
    let media = musa_dsp::prepare_media(
        score,
        gestures.tempo(),
        format,
        |logical| assets.read_verified(logical).map_err(|error| error.to_string()),
        musa_dsp::MediaLimits {
            max_assets: 256,
            max_decoded_frames: u64::from(sample_rate).saturating_mul(60 * 60),
            max_decoded_bytes: 2_u64.pow(31),
        },
    )
    .map_err(|error| ProjectError::Performance(error.to_string()))?;
    let maximum = u64::from(sample_rate).saturating_mul(60 * 60 * 24);
    let policy = musa_dsp::SchedulePolicy::new(
        1,
        musa_dsp::FrameRounding::NearestTiesLater,
        musa_dsp::CollapsePolicy::Ordered,
        [
            musa_dsp::MessageKind::End,
            musa_dsp::MessageKind::Point,
            musa_dsp::MessageKind::Begin,
        ],
        musa_dsp::ScheduleLimits {
            max_frame: maximum,
            max_time_map_entries: 2_000_001,
            max_occurrences: 1_000_000,
            max_messages: 2_000_000,
            max_batches: 2_000_000,
        },
    )
    .map_err(|error| ProjectError::Performance(error.to_string()))?;
    musa_dsp::prepare_execution_with_media(
        &gestures,
        &instruments,
        &instrument_machine,
        studio,
        media,
        musa_dsp::AudioOptions {
            format,
            schedule: policy,
            tuning: PerformanceOptions::default().tuning,
            render_seed: RENDER_SEED,
            limits: musa_dsp::AudioLimits {
                max_primitives: 10_000,
                max_state_bytes: 1 << 30,
                // The scheduling policy admits two million same-frame
                // messages; preparation prices that adversarial batch even
                // though ordinary scores spread them over time.
                max_step_work: 1_000_000_000,
            },
            // A minimum second beyond the studio-declared release remains an
            // explicit product export/playback policy.
            tail_frames: u64::from(sample_rate),
            max_total_frames: maximum.saturating_add(u64::from(sample_rate) * 10),
        },
    )
    .map_err(|error| ProjectError::Performance(error.to_string()))
}

/// The chain prepared for the engine.
///
/// # Errors
/// [`ProjectError::Performance`] if lowering or audio preparation fails.
pub(crate) fn prepare(
    score: &ScoreSnapshot,
    studio: &musa_dsp::StudioExecution,
    assets: &crate::assets::AssetInventory,
) -> Result<PreparedPlaybackPlan, ProjectError> {
    Ok(PreparedPlaybackPlan::new(build(score, studio, assets)?))
}

/// The same chain rendered offline to 32-bit float stereo WAV bytes.
///
/// # Errors
/// [`ProjectError::Performance`] if lowering, audio preparation, or WAV
/// encoding fails.
pub(crate) fn to_wav(
    score: &ScoreSnapshot,
    studio: &musa_dsp::StudioExecution,
    assets: &crate::assets::AssetInventory,
) -> Result<Vec<u8>, ProjectError> {
    let mut prepared = build(score, studio, assets)?;
    let audio = musa_dsp::render_offline(&mut prepared);
    wav_bytes(&audio)
}

/// The same score as a Standard MIDI File, with what SMF could not say
/// about it.
///
/// # Errors
/// [`ProjectError::Performance`] if lowering fails, or
/// [`ProjectError::Notation`] if a written pitch is outside MIDI's range.
pub(crate) fn to_midi(
    score: &ScoreSnapshot,
    mode: musa_notation::MidiMode,
) -> Result<(Vec<u8>, Vec<String>), ProjectError> {
    let performance = lower_gestures(score).map_err(|error| ProjectError::Performance(error.to_string()))?;
    let options = musa_notation::MidiOptions {
        mode,
        ..musa_notation::MidiOptions::default()
    };
    // SMF has one tempo track and one time-signature track for the whole
    // file. Every note is written at the frame it is actually played at, so
    // the file *sounds* exactly right; what it says about itself is the
    // reference part's. Sonically exact, notationally wrong, and said here
    // rather than discovered (`docs/rules/events/07-backend-contract.md`).
    let warnings = midi_losses(score, &performance);
    let bytes = musa_notation::render_midi(&performance, &options)
        .map_err(|error| ProjectError::Notation(error.to_string()))?;
    Ok((bytes, warnings.into_iter().map(|(_, said)| said).collect()))
}

/// What a Standard MIDI File cannot say about this score, each classified by
/// the fact it loses.
///
/// The kind is here rather than at the reader because this is where the
/// question is decided; a consumer that has to recognize a loss by its
/// wording has been handed prose where it needed a fact.
pub(crate) fn midi_losses(score: &ScoreSnapshot, performance: &musa_score::GesturePlan) -> Vec<(&'static str, String)> {
    let mut losses = Vec::new();
    if performance.is_polytempo() {
        losses.push((
            "polytempo",
            "SMF has one tempo track: the parts play at their own speeds and every note is written at \
             the moment it sounds, but the tempo the file states is the piece's and not theirs"
                .to_owned(),
        ));
    }
    if polymetric(score) {
        losses.push((
            "polymeter",
            "SMF has one time-signature track: the parts are barred differently and the file states \
             the piece's meter, which is the barlines of one of them"
                .to_owned(),
        ));
    }
    losses
}

/// Whether any part is barred differently from the piece.
fn polymetric(score: &ScoreSnapshot) -> bool {
    let piece = score.meter_at(musa_score::Scope::Piece, musa_score::MusicalTime::ZERO);
    score
        .parts()
        .iter()
        .any(|(id, _)| score.meter_at(musa_score::Scope::Part { part: id.0 }, musa_score::MusicalTime::ZERO) != piece)
}

/// Encode rendered audio as a 32-bit float stereo WAV (§13.8).
pub(crate) fn wav_bytes(audio: &musa_dsp::RenderedAudio) -> Result<Vec<u8>, ProjectError> {
    let spec = hound::WavSpec {
        channels: 2,
        sample_rate: audio.sample_rate(),
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };
    let mut cursor = std::io::Cursor::new(Vec::new());
    {
        let mut writer =
            hound::WavWriter::new(&mut cursor, spec).map_err(|e| ProjectError::Performance(e.to_string()))?;
        for sample in audio.samples() {
            writer
                .write_sample(*sample)
                .map_err(|e| ProjectError::Performance(e.to_string()))?;
        }
        writer
            .finalize()
            .map_err(|e| ProjectError::Performance(e.to_string()))?;
    }
    Ok(cursor.into_inner())
}

/// The performance lowering, formatted as the `render --to performance`
/// debug dump. Lives here because it is the one place that already knows the
/// exact performed-gesture vocabulary; frame assignment remains private to
/// audio preparation.
///
/// # Errors
/// [`ProjectError::Performance`] if lowering fails.
pub(crate) fn performance_dump(score: &ScoreSnapshot) -> Result<String, ProjectError> {
    use std::fmt::Write as _;

    let performance = lower_gestures(score).map_err(|e| ProjectError::Performance(e.to_string()))?;
    let mut out = String::new();
    for lane in performance.lanes() {
        let _ = writeln!(out, "lane {}:", lane.name());
        for occurrence in lane.track().occurrences() {
            let span = occurrence.span();
            let gesture = occurrence.payload();
            let lineage = lane.lineage(gesture.instance()).ok_or_else(|| {
                ProjectError::Performance(format!("gesture {} has no written lineage", gesture.instance()))
            })?;
            let _ = writeln!(
                out,
                "  {}..{} {} amplitude={} event-{:x}",
                span.start(),
                span.end(),
                gesture.pitch(),
                gesture.amplitude(),
                lineage.event().0
            );
        }
    }
    Ok(out)
}
