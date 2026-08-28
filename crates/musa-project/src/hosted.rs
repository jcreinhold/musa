//! One checked instrument, prepared for a host to render from its own MIDI.
//!
//! The session's own playback path prepares a whole piece and then *plays* it:
//! transport advances, and the score's occurrences arrive at their scheduled
//! frames. A workstation hosting Musa as an Audio Unit wants the other half of
//! the same preparation — the instrument, live, driven by MIDI the host
//! decides — which `musa-dsp` has always exposed as audition and which nothing
//! outside this crate could reach.
//!
//! So this module is a second door onto one preparation rather than a second
//! preparation. [`open`] compiles a project, verifies its assets, prepares
//! audio exactly as [`crate::playback::build`] does, and resolves one part to
//! the opaque audition target behind it. What comes back renders from events
//! and never from a transport: `06-daw-boundary.md` §3's hosted crossing is
//! the host's timeline, and a Music Device that sought a composition would be
//! claiming otherwise.
//!
//! Everything past preparation is real-time. [`HostedInstrument::note_on`],
//! [`HostedInstrument::note_off`], [`HostedInstrument::input`], and
//! [`HostedInstrument::step`] allocate nothing, take no lock, and touch only
//! prepared state; dropping a [`HostedInstrument`] frees a large graph and so
//! belongs off the render thread.

use std::path::{Path, PathBuf};

use musa_dsp::{AuditionEvent, AuditionOutcome, MidiAuditionInputKind, PreparedAudio, PreparedAuditionTarget};

use crate::error::ProjectError;

/// What a host asked to be prepared.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HostedRequest {
    /// A `.musa` piece, or the project directory holding it.
    pub project: PathBuf,
    /// Which piece of that project, by the name the contents gives it.
    /// `None` takes the project's first piece, which is the whole of it when
    /// the path named a file.
    pub piece: Option<String>,
    /// Which part of that piece supplies the instrument to render.
    pub part: String,
    /// The rate the host will render at. Preparation is exact for this rate
    /// and for no other: a host that changes it asks for a new preparation.
    pub sample_rate: u32,
}

/// What a restored document must name for a preparation to be the same one.
///
/// Every field is exact and compared exactly. `06-daw-boundary.md` Rule D1
/// makes the source canonical, so a component that cannot prove it prepared
/// *this* closure has to say so rather than render something adjacent.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HostedIdentity {
    /// The compiled music's semantic identity.
    pub music: String,
    /// The verified asset closure's identity, hex-encoded.
    pub assets: String,
    /// The piece within the project.
    pub piece: String,
    /// The part whose instrument this is.
    pub part: String,
    /// The rate this preparation is exact for.
    pub sample_rate: u32,
}

/// One MIDI wire dimension the prepared instrument's source actually binds.
///
/// Not a parameter and not a control: it is the set of things a host's MIDI
/// stream can say to *this* instrument, so that everything else it sends can
/// be reported as a loss instead of silently ignored.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HostedInput {
    /// Note-on attack velocity.
    AttackVelocity,
    /// Note-off release velocity.
    ReleaseVelocity,
    /// Damper pedal (CC64).
    SustainPedal,
    /// Sostenuto pedal (CC66).
    SostenutoPedal,
    /// Soft pedal (CC67).
    SoftPedal,
    /// Fourteen-bit channel pitch bend.
    PitchBend,
    /// Channel pressure.
    ChannelPressure,
    /// Polyphonic key pressure.
    KeyPressure,
    /// A named seven-bit controller.
    Controller(u8),
}

impl HostedInput {
    const fn to_dsp(self) -> MidiAuditionInputKind {
        match self {
            Self::AttackVelocity => MidiAuditionInputKind::AttackVelocity,
            Self::ReleaseVelocity => MidiAuditionInputKind::ReleaseVelocity,
            Self::SustainPedal => MidiAuditionInputKind::SustainPedal,
            Self::SostenutoPedal => MidiAuditionInputKind::SostenutoPedal,
            Self::SoftPedal => MidiAuditionInputKind::SoftPedal,
            Self::PitchBend => MidiAuditionInputKind::PitchBend,
            Self::ChannelPressure => MidiAuditionInputKind::ChannelPressure,
            Self::KeyPressure => MidiAuditionInputKind::KeyPressure,
            Self::Controller(number) => MidiAuditionInputKind::Controller(number),
        }
    }

    /// The eight fixed dimensions, in the order a report lists them.
    const FIXED: [Self; 8] = [
        Self::AttackVelocity,
        Self::ReleaseVelocity,
        Self::SustainPedal,
        Self::SostenutoPedal,
        Self::SoftPedal,
        Self::PitchBend,
        Self::ChannelPressure,
        Self::KeyPressure,
    ];

    /// How this dimension is written in a loss report or a saved document.
    #[must_use]
    pub fn name(self) -> String {
        match self {
            Self::AttackVelocity => "attack-velocity".to_owned(),
            Self::ReleaseVelocity => "release-velocity".to_owned(),
            Self::SustainPedal => "sustain-pedal".to_owned(),
            Self::SostenutoPedal => "sostenuto-pedal".to_owned(),
            Self::SoftPedal => "soft-pedal".to_owned(),
            Self::PitchBend => "pitch-bend".to_owned(),
            Self::ChannelPressure => "channel-pressure".to_owned(),
            Self::KeyPressure => "key-pressure".to_owned(),
            Self::Controller(number) => format!("controller-{number}"),
        }
    }
}

/// What applying one event did, as the host is told it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HostedOutcome {
    /// The instrument's source binds this, and it was applied.
    Applied,
    /// A note sounded, but the expression carried with it is not bound.
    NoteWithoutExpression,
    /// The instrument's source binds nothing for this dimension. The event is
    /// a loss, not an error: the host is entitled to send it.
    Unbound,
}

impl HostedOutcome {
    const fn from_dsp(outcome: AuditionOutcome) -> Self {
        match outcome {
            AuditionOutcome::Applied => Self::Applied,
            AuditionOutcome::NoteWithoutExpression => Self::NoteWithoutExpression,
            AuditionOutcome::UnsupportedInput => Self::Unbound,
        }
    }
}

/// A prepared instrument, ready for a host to render.
///
/// Immutable in everything a host may not change: the closure it was prepared
/// from, the part it renders, and the rate it is exact for are all fixed at
/// [`open`]. What moves is DSP state, and only through the real-time methods.
pub struct HostedInstrument {
    audio: PreparedAudio,
    target: PreparedAuditionTarget,
    identity: HostedIdentity,
    inputs: Vec<HostedInput>,
}

impl std::fmt::Debug for HostedInstrument {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HostedInstrument")
            .field("identity", &self.identity)
            .field("inputs", &self.inputs.len())
            .finish_non_exhaustive()
    }
}

impl HostedInstrument {
    /// What this was prepared from.
    #[must_use]
    pub const fn identity(&self) -> &HostedIdentity {
        &self.identity
    }

    /// The MIDI dimensions this instrument's source binds, in report order.
    ///
    /// A host stream carrying anything else is carrying something Musa
    /// declines to guess at, and the component says so once here rather than
    /// per event.
    #[must_use]
    pub fn inputs(&self) -> &[HostedInput] {
        &self.inputs
    }

    /// Begin one voice. Real-time safe.
    pub fn note_on(&mut self, voice: u32, note: u8, velocity: u8) -> HostedOutcome {
        HostedOutcome::from_dsp(
            self.audio
                .audition(self.target, AuditionEvent::NoteOn { voice, note, velocity }),
        )
    }

    /// Release one voice. Real-time safe.
    pub fn note_off(&mut self, voice: u32, velocity: u8) -> HostedOutcome {
        HostedOutcome::from_dsp(
            self.audio
                .audition(self.target, AuditionEvent::NoteOff { voice, velocity }),
        )
    }

    /// Apply one retained MIDI dimension. Real-time safe.
    ///
    /// `value` is raw wire data — seven-bit, or `-8192..=8191` for bend — and
    /// `key` names the note a polyphonic pressure belongs to.
    pub fn input(&mut self, input: HostedInput, value: i16, key: Option<u8>) -> HostedOutcome {
        HostedOutcome::from_dsp(self.audio.audition(
            self.target,
            AuditionEvent::Input {
                input: input.to_dsp(),
                value,
                key,
            },
        ))
    }

    /// Advance the prepared graph one frame, returning stereo output.
    ///
    /// Real-time safe, and the only way frames are produced: there is no
    /// transport to start, because the host owns the timeline.
    pub fn step(&mut self) -> [f32; 2] {
        self.audio.audition_step()
    }

    /// Stop every voice at once. Real-time safe.
    ///
    /// A host calls this when it stops, loops, or hands the component a
    /// discontinuity, and what it must not do is leave a voice sounding
    /// across the seam.
    ///
    /// It stops the *instrument*, not the studio. Whatever the source's
    /// declared processing already holds keeps decaying, because truncating
    /// that would be this crossing overriding what the source says — and it
    /// is bounded by the release the studio declares. So this is not
    /// re-instantiation: an instrument that has been reset does not render
    /// frame-for-frame what a freshly prepared one does, and a host that
    /// needs that asks for a new preparation.
    pub fn reset(&mut self) {
        self.audio.silence_audition(self.target);
    }
}

/// Compile, verify, and prepare one part's instrument for a host.
///
/// # Errors
/// [`ProjectError::Io`] if the project cannot be read,
/// [`ProjectError::NoValidScore`] if the source does not compile or its assets
/// are not verified, [`ProjectError::Assets`] if the named part has no
/// prepared instrument behind it, and [`ProjectError::Performance`] if
/// preparation itself fails.
pub fn open_hosted_instrument(request: &HostedRequest) -> Result<HostedInstrument, ProjectError> {
    if request.sample_rate == 0 {
        return Err(ProjectError::Performance(
            "the audio sample rate must be nonzero".to_owned(),
        ));
    }
    let mut project = crate::Project::open(&request.project)?;
    if let Some(piece) = request.piece.as_deref() {
        project.show(piece)?;
    }
    let piece = project.showing().to_owned();
    let session = project.current();
    let valid = session.hosted_artifacts().ok_or(ProjectError::NoValidScore)?;
    let audio = crate::playback::build_at(
        &valid.score,
        &valid.studio_execution,
        session.hosted_assets(),
        request.sample_rate,
    )?;
    let target = audio.audition_target(&request.part).ok_or_else(|| {
        ProjectError::Assets(format!(
            "the piece has no part named `{}` with an instrument behind it",
            request.part
        ))
    })?;
    let mut inputs: Vec<HostedInput> = HostedInput::FIXED
        .into_iter()
        .filter(|input| audio.supports_audition_input(target, input.to_dsp()))
        .collect();
    inputs.extend((0..=127u8).filter_map(|number| {
        let input = HostedInput::Controller(number);
        audio.supports_audition_input(target, input.to_dsp()).then_some(input)
    }));
    let identity = HostedIdentity {
        music: valid.identity.to_string(),
        assets: hex(&session.hosted_assets().identity()),
        piece,
        part: request.part.clone(),
        sample_rate: request.sample_rate,
    };
    Ok(HostedInstrument {
        audio,
        target,
        identity,
        inputs,
    })
}

/// The same, for a project already located by a directory alone.
///
/// # Errors
/// As [`open_hosted_instrument`].
pub fn open_hosted_instrument_at(
    project: impl AsRef<Path>,
    part: &str,
    sample_rate: u32,
) -> Result<HostedInstrument, ProjectError> {
    open_hosted_instrument(&HostedRequest {
        project: project.as_ref().to_path_buf(),
        piece: None,
        part: part.to_owned(),
        sample_rate,
    })
}

fn hex(bytes: &[u8; 32]) -> String {
    use std::fmt::Write as _;
    bytes.iter().fold(String::with_capacity(64), |mut text, byte| {
        let _ = write!(text, "{byte:02x}");
        text
    })
}
