//! The `AudioEngine` facade: device negotiation, stream lifecycle, and the
//! control-side ends of the real-time queues.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use cpal::traits::{DeviceTrait, HostTrait};

use crate::core::{
    AuditionEvent, AuditionMessage, AuditionTarget, CallbackCore, Message, PreparedPlaybackPlan, TransportCommand,
};
use crate::error::EngineError;

/// Engine configuration.
///
/// Device selection follows the zero-setup rule: the default output
/// device, negotiated for stereo f32 at `sample_rate`. There is no
/// resampler yet — if the device cannot run the requested rate, `open`
/// fails with [`EngineError::UnsupportedStreamConfig`] rather than silently
/// picking a different rate (a resampler is a later measured feature).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EngineConfig {
    /// Requested stream sample rate (the plan's rate; 48 kHz by default).
    pub sample_rate: u32,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self { sample_rate: 48_000 }
    }
}

/// Capacity of both real-time queues.
///
/// Sizing them equally is what makes the callback's no-drop guarantee
/// structural. Only an `Install` retires a plan, [`AudioEngine::install`]
/// drains the retirement queue before it enqueues one, and the command queue
/// holds at most `COMMAND_CAPACITY` messages — so at most `COMMAND_CAPACITY`
/// retirements can be outstanding, and the retirement queue cannot fill. The
/// callback's hold-back path is therefore unreachable in this configuration
/// and exists only so that misuse degrades into back-pressure rather than
/// into a prepared audio machine being destroyed on the audio thread.
const COMMAND_CAPACITY: usize = 64;
const AUDITION_CAPACITY: usize = 2_048;

/// The audio engine. Owns the stream and queue ends; the rest of
/// the application never sees CPAL types.
pub struct AudioEngine {
    /// Kept alive for the stream's lifetime; dropping stops the stream.
    _stream: cpal::Stream,
    /// Both control-side queue ends under one lock, so an install drains and
    /// enqueues atomically. Taken only on the control thread — never in the
    /// callback.
    channels: Mutex<Channels>,
    position: Arc<AtomicU64>,
    playing: Arc<AtomicBool>,
}

/// The control thread's ends of the two real-time queues.
struct Channels {
    commands: rtrb::Producer<Message>,
    audition: rtrb::Producer<AuditionMessage>,
    retired: rtrb::Consumer<Box<PreparedPlaybackPlan>>,
}

impl Channels {
    /// Drop every plan the callback has handed back.
    fn drain_retired(&mut self) {
        while self.retired.pop().is_ok() {}
    }
}

impl AudioEngine {
    /// Open the default output device and start the stream.
    ///
    /// # Errors
    /// [`EngineError::NoOutputDevice`] when no device exists,
    /// [`EngineError::UnsupportedStreamConfig`] when the requested shape is
    /// unsupported, [`EngineError::Stream`] on stream failures.
    pub fn open(config: EngineConfig) -> Result<Self, EngineError> {
        let span = tracing::info_span!("engine_open", requested_rate = config.sample_rate);
        let _entered = span.enter();
        let host = cpal::default_host();
        let device = host.default_output_device().ok_or(EngineError::NoOutputDevice)?;
        let stream_config = negotiate(&device, config.sample_rate)?;
        // The device musa actually got and the shape CPAL actually agreed to.
        // A person reporting "no sound" cannot see either, and every other
        // question about playback is downstream of these two.
        //
        // The name is the one field here that is not already to hand — CPAL
        // builds the string on demand — so it is asked for only when something
        // is listening. The stream configuration is free.
        if tracing::enabled!(tracing::Level::INFO) {
            tracing::info!(
                device = %device,
                rate = stream_config.sample_rate,
                channels = stream_config.channels,
                "opened the output device"
            );
        }

        let (command_producer, command_consumer) = rtrb::RingBuffer::<Message>::new(COMMAND_CAPACITY);
        let (retired_producer, retired_consumer) = rtrb::RingBuffer::<Box<PreparedPlaybackPlan>>::new(COMMAND_CAPACITY);
        let (audition_producer, audition_consumer) = rtrb::RingBuffer::<AuditionMessage>::new(AUDITION_CAPACITY);
        let position = Arc::new(AtomicU64::new(0));
        let playing = Arc::new(AtomicBool::new(false));
        let mut core = CallbackCore::new_with_audition(
            command_consumer,
            retired_producer,
            audition_consumer,
            Arc::clone(&position),
            Arc::clone(&playing),
        );

        let stream = device
            .build_output_stream(
                stream_config,
                move |data: &mut [f32], _: &cpal::OutputCallbackInfo| core.process(data),
                |error| tracing::error!(%error, "audio stream error"),
                None,
            )
            .map_err(|error| EngineError::Stream(error.to_string()))?;
        cpal::traits::StreamTrait::play(&stream).map_err(|error| EngineError::Stream(error.to_string()))?;
        Ok(Self {
            _stream: stream,
            channels: Mutex::new(Channels {
                commands: command_producer,
                audition: audition_producer,
                retired: retired_consumer,
            }),
            position,
            playing,
        })
    }

    /// Install a prepared plan, replacing the current one.
    ///
    /// The replaced plan returns on the retirement queue and is destroyed
    /// here on the control thread, never in the callback. Draining
    /// before enqueuing is what keeps that queue from ever filling — see
    /// [`COMMAND_CAPACITY`] — and it frees the previous plan's graph promptly
    /// rather than at engine shutdown, which matters once the GUI reinstalls
    /// a plan after every edit.
    ///
    /// # Errors
    /// [`EngineError::QueueFull`] if the command queue is full.
    pub fn install(&self, plan: PreparedPlaybackPlan) -> Result<(), EngineError> {
        // The control side of the boundary, and the last place anything may
        // be said: past the queue the callback runs under the real-time rules
        // and logs nothing at any level (roadmap §13).
        tracing::debug!(frames = plan.total_frames(), "installing a playback plan");
        let mut channels = self.lock()?;
        channels.drain_retired();
        channels
            .commands
            .push(Message::Install(Box::new(plan)))
            .map_err(|_| EngineError::QueueFull)
    }

    /// Send a transport command.
    ///
    /// # Errors
    /// [`EngineError::QueueFull`] if the command queue is full.
    pub fn command(&self, command: TransportCommand) -> Result<(), EngineError> {
        tracing::debug!(?command, "transport");
        self.lock()?
            .commands
            .push(Message::Transport(command))
            .map_err(|_| EngineError::QueueFull)
    }

    /// Queue one already-paired selected-instrument audition event.
    ///
    /// # Errors
    /// [`EngineError::QueueFull`] when the bounded audition queue is full.
    pub fn audition(&self, target: AuditionTarget, event: AuditionEvent) -> Result<(), EngineError> {
        self.lock()?
            .audition
            .push(AuditionMessage { target, event })
            .map_err(|_| EngineError::QueueFull)
    }

    /// Take the control-side lock.
    fn lock(&self) -> Result<std::sync::MutexGuard<'_, Channels>, EngineError> {
        self.channels
            .lock()
            .map_err(|_| EngineError::Stream("command lock poisoned".to_string()))
    }

    /// The transport's current frame (telemetry; eventually consistent).
    pub fn position(&self) -> u64 {
        self.position.load(Ordering::Relaxed)
    }

    /// Whether playback is running.
    pub fn is_playing(&self) -> bool {
        self.playing.load(Ordering::Relaxed)
    }
}

impl Drop for AudioEngine {
    fn drop(&mut self) {
        // The stream is torn down after this, so any plan still in flight is
        // released here rather than leaked.
        if let Ok(channels) = self.channels.get_mut() {
            channels.drain_retired();
        }
    }
}

/// Find a stereo f32 stream config at `rate` on `device` (exact match
/// only; mismatches are explicit errors).
fn negotiate(device: &cpal::Device, rate: u32) -> Result<cpal::StreamConfig, EngineError> {
    let mut configs = device
        .supported_output_configs()
        .map_err(|error| EngineError::Stream(error.to_string()))?;
    let found = configs.find_map(|range| {
        if range.channels() == 2 && range.sample_format() == cpal::SampleFormat::F32 {
            range.try_with_sample_rate(rate)
        } else {
            None
        }
    });
    found
        .map(|config| config.config())
        .ok_or(EngineError::UnsupportedStreamConfig { rate })
}
