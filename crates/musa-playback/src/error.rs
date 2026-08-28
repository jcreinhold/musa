//! Engine errors: explicit, never silent.

/// A failure to open or command the engine.
#[derive(Debug, thiserror::Error)]
pub enum EngineError {
    /// No usable audio output device exists (CI machines, headless servers).
    #[error("no audio output device available")]
    NoOutputDevice,
    /// The device cannot run the requested stream shape. There is no
    /// resampler yet (a later measured feature), so the plan's sample rate
    /// must be a device-supported rate.
    #[error("the output device does not support {rate} Hz stereo f32 output")]
    UnsupportedStreamConfig {
        /// The requested rate.
        rate: u32,
    },
    /// The stream failed to build or start.
    #[error("audio stream failure: {0}")]
    Stream(String),
    /// The command queue is full (the control side is producing faster than
    /// the audio side consumes).
    #[error("engine command queue is full")]
    QueueFull,
    /// This platform publishes no MIDI output at all. macOS has `CoreMIDI`;
    /// nothing else does, and saying so is better than a silent no-op.
    #[error("this platform has no MIDI output")]
    NoMidiOutput,
    /// A destination was named that the host does not currently offer.
    #[error("no MIDI destination `{id}` is connected")]
    UnknownMidiDestination {
        /// The stable identifier that was asked for.
        id: String,
    },
    /// The platform refused to publish, open, or run MIDI output.
    #[error("MIDI output failure: {0}")]
    MidiOutput(String),
}
