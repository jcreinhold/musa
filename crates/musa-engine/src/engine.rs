//! The `AudioEngine` facade: device negotiation, stream lifecycle, and the
//! control-side ends of the real-time queues (roadmap §13.2, §14.8, §15.6).

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use cpal::traits::{DeviceTrait, HostTrait};

use crate::core::{CallbackCore, Message, PreparedPlaybackPlan, TransportCommand};
use crate::error::EngineError;

/// Engine configuration.
///
/// Device selection follows §14.8's zero-setup rule: the default output
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

/// The audio engine (§15.6). Owns the stream and queue ends; the rest of
/// the application never sees CPAL types.
pub struct AudioEngine {
    /// Kept alive for the stream's lifetime; dropping stops the stream.
    _stream: cpal::Stream,
    /// Control-side lock (never taken in the callback, §13.2).
    commands: Mutex<rtrb::Producer<Message>>,
    retired: rtrb::Consumer<Box<PreparedPlaybackPlan>>,
    position: Arc<AtomicU64>,
    playing: Arc<AtomicBool>,
}

impl AudioEngine {
    /// Open the default output device and start the stream.
    ///
    /// # Errors
    /// [`EngineError::NoOutputDevice`] when no device exists,
    /// [`EngineError::UnsupportedStreamConfig`] when the requested shape is
    /// unsupported, [`EngineError::Stream`] on stream failures.
    pub fn open(config: EngineConfig) -> Result<Self, EngineError> {
        let host = cpal::default_host();
        let device = host.default_output_device().ok_or(EngineError::NoOutputDevice)?;
        let stream_config = negotiate(&device, config.sample_rate)?;

        let (command_producer, command_consumer) = rtrb::RingBuffer::<Message>::new(64);
        let (retired_producer, retired_consumer) = rtrb::RingBuffer::<Box<PreparedPlaybackPlan>>::new(8);
        let position = Arc::new(AtomicU64::new(0));
        let playing = Arc::new(AtomicBool::new(false));
        let mut core = CallbackCore::new(
            command_consumer,
            retired_producer,
            Arc::clone(&position),
            Arc::clone(&playing),
        );

        let stream = device
            .build_output_stream(
                &stream_config,
                move |data: &mut [f32], _: &cpal::OutputCallbackInfo| core.process(data),
                |error| tracing::error!(%error, "audio stream error"),
                None,
            )
            .map_err(|error| EngineError::Stream(error.to_string()))?;
        cpal::traits::StreamTrait::play(&stream).map_err(|error| EngineError::Stream(error.to_string()))?;
        Ok(Self {
            _stream: stream,
            commands: Mutex::new(command_producer),
            retired: retired_consumer,
            position,
            playing,
        })
    }

    /// Install a prepared plan (replaces the current one; the replaced plan
    /// returns on the retired queue and is dropped on the control side,
    /// latest when the engine drops).
    ///
    /// # Errors
    /// [`EngineError::QueueFull`] if the command queue is full.
    pub fn install(&self, plan: PreparedPlaybackPlan) -> Result<(), EngineError> {
        self.push(Message::Install(Box::new(plan)))
    }

    /// Send a transport command.
    ///
    /// # Errors
    /// [`EngineError::QueueFull`] if the command queue is full.
    pub fn command(&self, command: TransportCommand) -> Result<(), EngineError> {
        self.push(Message::Transport(command))
    }

    /// Push onto the command queue (control-side lock).
    fn push(&self, message: Message) -> Result<(), EngineError> {
        let mut producer = self
            .commands
            .lock()
            .map_err(|_| EngineError::Stream("command lock poisoned".to_string()))?;
        producer.push(message).map_err(|_| EngineError::QueueFull)
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
        while self.retired.pop().is_ok() {}
    }
}

/// Find a stereo f32 stream config at `rate` on `device` (§14.8 fallback
/// order: exact match only; mismatches are explicit errors).
fn negotiate(device: &cpal::Device, rate: u32) -> Result<cpal::StreamConfig, EngineError> {
    let mut configs = device
        .supported_output_configs()
        .map_err(|error| EngineError::Stream(error.to_string()))?;
    let found = configs.find_map(|range| {
        if range.channels() == 2 && range.sample_format() == cpal::SampleFormat::F32 {
            range.try_with_sample_rate(cpal::SampleRate(rate))
        } else {
            None
        }
    });
    found
        .map(|config| config.config())
        .ok_or(EngineError::UnsupportedStreamConfig { rate })
}
