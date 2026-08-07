//! The voice allocator: a fixed voice pool turning `NoteOn`/`NoteOff` into
//! per-voice gate + frequency control (roadmap §13.5). Steal-oldest policy.
//!
//! Envelope arithmetic per sample is total and bounded.
#![allow(clippy::arithmetic_side_effects)]

use musa_compiler::VoiceInstanceId;

/// One voice's state.
#[derive(Clone, Copy, Debug, PartialEq)]
enum VoiceState {
    /// Silent and available.
    Free,
    /// Gated (note held).
    Active,
    /// Released (note off; envelope decaying).
    Release,
}

/// One synthesizer voice: oscillator phase, frequency, placeholder envelope,
/// and allocation bookkeeping.
#[derive(Clone, Copy, Debug)]
struct Voice {
    state: VoiceState,
    instance: VoiceInstanceId,
    frequency: f32,
    phase: f64,
    envelope: f32,
    /// Frames since (re)trigger — the steal-oldest criterion.
    age: u64,
}

impl Voice {
    const FREE: Self = Self {
        state: VoiceState::Free,
        instance: VoiceInstanceId(0),
        frequency: 0.0,
        phase: 0.0,
        envelope: 0.0,
        age: 0,
    };
}

/// A fixed voice pool with a steal-oldest policy (§13.5). The envelope is a
/// short linear attack/release ramp — a placeholder to avoid clicks, NOT an
/// articulation model (a real ADSR arrives in prompt 25).
#[derive(Clone, Debug)]
pub struct VoiceAllocator {
    voices: Vec<Voice>,
    /// Attack ramp length in frames.
    attack: u64,
    /// Release ramp length in frames.
    release: u64,
}

impl VoiceAllocator {
    /// A pool of `voices` voices. Attack/release are fixed placeholder ramps
    /// (5 ms / 50 ms at `sample_rate`).
    pub fn new(voices: u8, sample_rate: u32) -> Self {
        Self {
            voices: vec![Voice::FREE; usize::from(voices)],
            attack: (u64::from(sample_rate) / 200).max(1),
            release: (u64::from(sample_rate) / 20).max(1),
        }
    }

    /// Gate a voice for `instance` at `frequency`, stealing the oldest when
    /// the pool is full. The envelope restarts from its current value — a
    /// stolen voice never clicks.
    pub fn note_on(&mut self, instance: VoiceInstanceId, frequency: f32) {
        let slot = self
            .voices
            .iter()
            .position(|voice| voice.state == VoiceState::Free)
            .or_else(|| {
                self.voices
                    .iter()
                    .enumerate()
                    .max_by_key(|(_, voice)| voice.age)
                    .map(|(index, _)| index)
            });
        if let Some(voice) = slot.and_then(|i| self.voices.get_mut(i)) {
            voice.state = VoiceState::Active;
            voice.instance = instance;
            voice.frequency = frequency;
            voice.phase = 0.0;
            voice.age = 0;
        }
    }

    /// Release the voice sounding `instance`, if any.
    pub fn note_off(&mut self, instance: VoiceInstanceId) {
        for voice in &mut self.voices {
            if voice.state == VoiceState::Active && voice.instance == instance {
                voice.state = VoiceState::Release;
            }
        }
    }

    /// Voices currently making sound (gated or decaying).
    pub fn sounding(&self) -> usize {
        self.voices
            .iter()
            .filter(|voice| voice.state != VoiceState::Free)
            .count()
    }

    /// Voices currently gated (for tests and future voice-count telemetry).
    pub fn gated(&self) -> usize {
        self.voices
            .iter()
            .filter(|voice| voice.state == VoiceState::Active)
            .count()
    }

    /// Render `count` samples of the whole pool summed, mono into `output`
    /// (the oscillator bank of §13.5; the poly synth processor mixes the
    /// result to stereo).
    pub fn render(&mut self, output: &mut [f32], count: usize, sample_rate: f64) {
        let attack_step = 1.0 / self.attack as f32;
        let release_step = 1.0 / self.release as f32;
        for i in 0..count {
            let mut mix = 0.0f32;
            for voice in &mut self.voices {
                if voice.state == VoiceState::Free {
                    continue;
                }
                voice.age = voice.age.saturating_add(1);
                let sample = (2.0 * std::f64::consts::PI * voice.phase).sin() as f32;
                voice.phase += f64::from(voice.frequency) / sample_rate;
                if voice.phase >= 1.0 {
                    voice.phase -= 1.0;
                }
                match voice.state {
                    VoiceState::Active => {
                        voice.envelope = (voice.envelope + attack_step).min(1.0);
                    }
                    VoiceState::Release => {
                        voice.envelope -= release_step;
                        if voice.envelope <= 0.0 {
                            voice.envelope = 0.0;
                            voice.state = VoiceState::Free;
                        }
                    }
                    VoiceState::Free => {}
                }
                mix += sample * voice.envelope;
            }
            if let Some(slot) = output.get_mut(i) {
                *slot = mix;
            }
        }
    }
}
