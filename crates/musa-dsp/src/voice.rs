//! The voice allocator: a fixed voice pool turning `NoteOn`/`NoteOff` into
//! per-voice gate + frequency control (roadmap §13.5). Steal-oldest policy.
//!
//! The per-voice chain is §13.5's: allocator → pitch-to-frequency (done by
//! the scheduler, which hands over Hz) → oscillator → amplitude ADSR →
//! per-voice gain, where the gain is the interpreted loudness the
//! performance profiles produce. The pool's sum carries a fixed headroom
//! ([`VoiceAllocator::scale`]) so ordinary polyphony reaches the master
//! limiter under its ceiling, not over it. The envelope is a real ADSR ([`crate::envelope`]) whose
//! default shape is the ramp that preceded it, so a piece that asks for no
//! envelope still gets the same short, uninterpretive fade in and out.
//!
//! Envelope and oscillator arithmetic per sample is total and bounded.
#![allow(clippy::arithmetic_side_effects)]

use crate::envelope::{Adsr, AdsrSettings, AdsrSteps};
use crate::schedule::EventHandle;

#[derive(Clone, Debug, PartialEq, Eq)]
enum VoiceIdentity {
    Scheduled(EventHandle),
    Live(u32),
}

/// One synthesizer voice: oscillator phase, frequency, envelope, and
/// allocation bookkeeping.
#[derive(Clone, Debug)]
struct Voice {
    instance: Option<VoiceIdentity>,
    frequency: f32,
    phase: f64,
    envelope: Adsr,
    /// The shape this voice is playing: the patch's, with the note's own
    /// attack request substituted when it made one.
    steps: AdsrSteps,
    /// The attack the note asked for, in seconds; `0` means "as written".
    /// Kept so a later change to the patch's envelope can be re-applied
    /// without losing what the note asked for.
    attack_request: f32,
    /// Whether the note is still held (a released voice is stealable first).
    held: bool,
    /// The interpreted loudness the note-on asked for; `1.0` is neutral, so
    /// a profile's reading is applied to the voice exactly as stated. (The
    /// pool's own headroom is separate — see [`VoiceAllocator::scale`].)
    amplitude: f32,
    /// Frames since (re)trigger — the steal-oldest criterion.
    age: u64,
}

impl Voice {
    const FREE: Self = Self {
        instance: None,
        frequency: 0.0,
        phase: 0.0,
        envelope: Adsr::IDLE,
        steps: AdsrSteps::DEFAULT,
        attack_request: 0.0,
        held: false,
        amplitude: 1.0,
        age: 0,
    };

    fn is_free(&self) -> bool {
        self.envelope.is_idle()
    }
}

/// A fixed voice pool with a steal-oldest policy (§13.5).
#[derive(Clone, Debug)]
pub(crate) struct VoiceAllocator {
    voices: Vec<Voice>,
    settings: AdsrSettings,
    steps: AdsrSteps,
    sample_rate: u32,
    /// The second partial's frequency ratio to the note.
    ratio: f64,
    /// How much of that partial is mixed in; `0` is one sine per voice.
    blend: f32,
    /// Additional partial energy contributed by source expression mapping.
    expression_timbre: f32,
    /// Instrument-global gain reached only through a checked control mapping.
    gain: f32,
    /// Additional release time selected by typed phrase connection mapping.
    connection_release: f32,
    /// Fixed headroom applied to the pool sum: `1/√voices`, so the typical
    /// (incoherent-phase) sum of a full pool lands near full scale instead
    /// of `voices` times over it. Per-voice `amplitude` stays neutral — the
    /// profile's interpretation is not rescaled — but the *sum* of sixteen
    /// full-scale sines is 24 dB over 0 dBFS, and leaving that for the
    /// master limiter made the limiter the mix bus: engaged continuously,
    /// its per-sample gain was audible as scratchy distortion. The limiter
    /// is for the rare coherent peak, not for every chord.
    scale: f32,
}

impl VoiceAllocator {
    /// A pool of `voices` voices with the default envelope (a 5 ms attack and
    /// a 50 ms release — enough not to click, short enough not to be an
    /// interpretation).
    pub(crate) fn new(voices: u8, sample_rate: u32) -> Self {
        let settings = AdsrSettings::default();
        Self {
            voices: vec![Voice::FREE; usize::from(voices)],
            settings,
            steps: AdsrSteps::new(settings, sample_rate),
            sample_rate,
            ratio: 2.0,
            blend: 0.0,
            expression_timbre: 0.0,
            gain: 1.0,
            connection_release: 0.0,
            // `max(1)`: an empty pool renders silence whatever the scale is.
            scale: f64::from(voices.max(1)).sqrt().recip() as f32,
        }
    }

    /// Set one voice parameter by the name its descriptor declares —
    /// envelope or oscillator bank. Unknown names are ignored: the plan only
    /// ever passes declared ones.
    ///
    /// Sounding voices follow: a modulated envelope that only took effect on
    /// the next note would be a control that does nothing while you listen.
    pub(crate) fn set_voice(&mut self, name: &str, value: f32) {
        match name {
            "attack" => self.settings.attack = value,
            "decay" => self.settings.decay = value,
            "sustain" => self.settings.sustain = value,
            "release" => self.settings.release = value,
            "ratio" => {
                self.ratio = f64::from(value);
                return;
            }
            "blend" => {
                self.blend = value;
                return;
            }
            "expression_timbre" => {
                self.expression_timbre = value;
                return;
            }
            "gain" => {
                self.gain = value;
                return;
            }
            "connection_release" => {
                self.connection_release = value;
            }
            _ => return,
        }
        self.steps = AdsrSteps::new(self.settings, self.sample_rate);
        let (settings, rate) = (self.settings, self.sample_rate);
        for voice in &mut self.voices {
            voice.steps = shape(settings, rate, voice.attack_request, self.connection_release);
        }
    }

    /// Gate a voice for `instance` at `frequency` and `amplitude`, stealing
    /// the oldest when the pool is full. The envelope restarts from its
    /// current value — a stolen voice never clicks.
    ///
    /// `attack` is the time in seconds the *performance* asked for (§6.4's
    /// profile), which is a request rather than an envelope: it replaces the
    /// patch's attack for this note and leaves the rest of the shape alone.
    /// Zero means the note asked for nothing.
    pub(crate) fn note_on(&mut self, instance: &EventHandle, frequency: f32, amplitude: f32, attack: f32) {
        self.note_on_identity(VoiceIdentity::Scheduled(instance.clone()), frequency, amplitude, attack);
    }

    /// Gate one ephemeral audition voice without allocating an event handle.
    pub(crate) fn live_note_on(&mut self, voice: u32, frequency: f32, amplitude: f32, attack: f32) {
        self.note_on_identity(VoiceIdentity::Live(voice), frequency, amplitude, attack);
    }

    fn note_on_identity(&mut self, instance: VoiceIdentity, frequency: f32, amplitude: f32, attack: f32) {
        let slot = self.voices.iter().position(Voice::is_free).or_else(|| {
            self.voices
                .iter()
                .enumerate()
                .max_by_key(|(_, voice)| voice.age)
                .map(|(index, _)| index)
        });
        if let Some(voice) = slot.and_then(|i| self.voices.get_mut(i)) {
            voice.instance = Some(instance);
            voice.frequency = frequency;
            voice.amplitude = amplitude;
            voice.phase = 0.0;
            voice.age = 0;
            voice.held = true;
            voice.attack_request = attack;
            voice.steps = shape(self.settings, self.sample_rate, attack, self.connection_release);
            voice.envelope.gate(&voice.steps);
        }
    }

    /// Release the voice sounding `instance`, if any.
    pub(crate) fn note_off(&mut self, instance: &EventHandle) {
        for voice in &mut self.voices {
            if voice.held
                && matches!(
                    voice.instance.as_ref(),
                    Some(VoiceIdentity::Scheduled(handle)) if handle == instance
                )
            {
                voice.held = false;
                voice.envelope.release(&voice.steps);
            }
        }
    }

    /// Release one ephemeral audition voice.
    pub(crate) fn live_note_off(&mut self, instance: u32) {
        for voice in &mut self.voices {
            if voice.held && voice.instance == Some(VoiceIdentity::Live(instance)) {
                voice.held = false;
                voice.envelope.release(&voice.steps);
            }
        }
    }

    /// Silence every voice at once, held or decaying.
    ///
    /// Not a release: a release is what a player asks for, and this is what a
    /// *host* asks for when it stops, loops, or hands the component a
    /// discontinuity. A tail crossing that seam would be this instrument
    /// sounding in a place the host did not put it.
    pub(crate) fn silence(&mut self) {
        for voice in &mut self.voices {
            voice.instance = None;
            voice.held = false;
            voice.phase = 0.0;
            voice.envelope = Adsr::IDLE;
        }
    }

    /// Voices currently making sound (gated or decaying).
    #[cfg(test)]
    fn sounding(&self) -> usize {
        self.voices.iter().filter(|voice| !voice.is_free()).count()
    }

    /// Voices currently gated (for tests and future voice-count telemetry).
    #[cfg(test)]
    fn gated(&self) -> usize {
        self.voices.iter().filter(|voice| voice.held).count()
    }

    /// Render `count` samples of the whole pool summed, mono into `output`
    /// (the oscillator bank of §13.5; the poly synth processor mixes the
    /// result to stereo). The sum carries the pool's fixed headroom — see
    /// [`Self::scale`].
    pub(crate) fn render(&mut self, output: &mut [f32], count: usize, sample_rate: f64) {
        for i in 0..count {
            let mut mix = 0.0f32;
            for voice in &mut self.voices {
                if voice.is_free() {
                    continue;
                }
                voice.age = voice.age.saturating_add(1);
                // The bank's second partial rides the same phase, so it is
                // exactly `ratio` times the note with no second accumulator
                // and no drift between the two. A blend of zero skips it
                // rather than adding a scaled zero.
                let mut sample = (2.0 * std::f64::consts::PI * voice.phase).sin() as f32;
                let partial_mix = (self.blend + self.expression_timbre).clamp(0.0, 1.0);
                if partial_mix != 0.0 {
                    let partial = (2.0 * std::f64::consts::PI * voice.phase * self.ratio).sin() as f32;
                    sample = partial_mix.mul_add(partial, sample);
                }
                voice.phase += f64::from(voice.frequency) / sample_rate;
                if voice.phase >= 1.0 {
                    voice.phase -= 1.0;
                }
                let level = voice.envelope.tick(&voice.steps);
                if voice.envelope.is_idle() {
                    voice.held = false;
                    voice.instance = None;
                }
                // Fused: one rounding for the scale-and-accumulate, which is
                // both cheaper and closer than the two-step form. With the
                // neutral amplitude of 1 it is bit-identical to a plain add,
                // so an unprofiled piece is not quietly rescaled.
                mix = (sample * level).mul_add(voice.amplitude, mix);
            }
            if let Some(slot) = output.get_mut(i) {
                *slot = mix * self.scale * self.gain;
            }
        }
    }
}

/// The shape a voice plays: the patch's, with a positive attack request
/// substituted for the written one.
fn shape(settings: AdsrSettings, sample_rate: u32, attack: f32, connection_release: f32) -> AdsrSteps {
    let mut settings = if attack > 0.0 {
        AdsrSettings { attack, ..settings }
    } else {
        settings
    };
    settings.release += connection_release;
    AdsrSteps::new(settings, sample_rate)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::arithmetic_side_effects)]

    use super::*;

    const RATE: u32 = 48_000;

    #[test]
    fn allocator_steals_oldest_and_matches_only_live_handles() {
        let mut allocator = VoiceAllocator::new(2, RATE);
        let [first, second, third] = [EventHandle::root(0), EventHandle::root(1), EventHandle::root(2)];
        allocator.note_on(&first, 220.0, 1.0, 0.0);
        let mut buffer = [0.0; 64];
        allocator.render(&mut buffer, 64, f64::from(RATE));
        allocator.note_on(&second, 330.0, 1.0, 0.0);
        allocator.note_on(&third, 440.0, 1.0, 0.0);
        assert_eq!(allocator.gated(), 2);
        allocator.note_off(&first);
        assert_eq!(allocator.gated(), 2);
        allocator.note_off(&second);
        assert_eq!(allocator.gated(), 1);
        allocator.note_off(&third);
        assert_eq!(allocator.gated(), 0);
    }

    #[test]
    fn allocator_reuses_a_fully_released_voice() {
        let mut allocator = VoiceAllocator::new(1, RATE);
        let first = EventHandle::root(0);
        allocator.note_on(&first, 440.0, 1.0, 0.0);
        allocator.note_off(&first);
        let mut buffer = vec![0.0; 4_800];
        allocator.render(&mut buffer, 4_800, f64::from(RATE));
        assert_eq!(allocator.sounding(), 0);
        allocator.note_on(&EventHandle::root(1), 440.0, 1.0, 0.0);
        assert_eq!(allocator.gated(), 1);
    }

    #[test]
    fn pool_sum_carries_fixed_headroom() {
        let mut allocator = VoiceAllocator::new(16, RATE);
        allocator.note_on(&EventHandle::root(0), 440.0, 1.0, 0.0);
        let mut buffer = vec![0.0; RATE as usize / 2];
        let count = buffer.len();
        allocator.render(&mut buffer, count, f64::from(RATE));
        let peak = buffer.iter().fold(0.0_f32, |worst, sample| worst.max(sample.abs()));
        assert!((peak - 0.25).abs() < 0.01, "got {peak}");
    }
}
