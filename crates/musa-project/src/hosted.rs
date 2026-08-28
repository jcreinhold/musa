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

use musa_dsp::{
    AudioRouteKind, AuditionControlRefusal, AuditionEvent, AuditionOutcome, MidiAuditionInputKind, PreparedAudio,
    PreparedAuditionTarget,
};

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
    /// The parameter-address table a restored document carried, encoded by
    /// [`HostedControlTable::encode`].
    ///
    /// `None` is a component that has never been saved. Anything else is a
    /// promise a host already made to its automation lanes, and preparation
    /// keeps it: an address that was assigned stays assigned to the same
    /// source identity, and one whose control is gone stays retired rather
    /// than being handed to a different control.
    pub table: Option<String>,
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

/// The version of the parameter-address table this crate writes.
///
/// It changes when the table's encoding or its address derivation changes,
/// which is what makes a table from a future component recognizable as one
/// this component must not silently reinterpret.
pub const HOSTED_TABLE_VERSION: u32 = 1;

const HOSTED_TABLE_HEADER: &str = "musa-control-table";

/// One admitted control, projected as a host parameter.
///
/// Rule D2: this is a source-declared exposed control and nothing else. The
/// address is stable across formatting, summary edits, tree rebuilds, and
/// extension restarts, because it is assigned from the control's canonical
/// source identity through a table that is saved with the document.
#[derive(Clone, Debug, PartialEq)]
pub struct HostedControl {
    /// The 64-bit address a host automates. Never zero.
    pub address: u64,
    /// The canonical source identity: `namespace::name`.
    pub identity: String,
    /// The name the source key declares, for a host to display.
    pub display: String,
    /// The sentence the source key declares, for an accessible description.
    pub summary: String,
    /// The source value kind.
    pub kind: String,
    /// The source update rate, spelled as the declaration spells it.
    pub update_rate: String,
    /// The inclusive ends of the control's declared domain.
    pub minimum: f32,
    /// The upper end of that domain.
    pub maximum: f32,
    /// The declared default, inside that domain.
    pub default: f32,
    /// Whether the source declares this control continuous, and so whether a
    /// host ramp means anything for it.
    pub continuous: bool,
}

/// One source-declared control this boundary cannot carry, and why.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HostedControlLoss {
    /// The canonical source identity of the control that stayed behind.
    pub identity: String,
    /// Its source value kind.
    pub kind: String,
    /// One clause naming the refusal.
    pub reason: String,
}

impl std::fmt::Display for HostedControlLoss {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "`{}` ({}) is not a host parameter: {}",
            self.identity, self.kind, self.reason
        )
    }
}

/// One output a host may negotiate, in stable source order.
///
/// Bus zero is always the main output — what a single-output host hears, and
/// what every host hears on bus zero whether or not it enabled the others.
/// The rest are the declared points this part's signal reaches on its way
/// there. None of them is a mixer track: `08-performance-and-sound.md` §6
/// keeps part, bus, and track three different things, and this projection
/// renames nothing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HostedOutput {
    /// The source name, or `main` for bus zero.
    pub name: String,
    /// What the source declared this to be.
    pub role: HostedOutputRole,
}

/// What one projected output is, in source terms.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HostedOutputRole {
    /// The main output: everything this part's routing delivers.
    Main,
    /// This part's own instrument chain, before any bus it sends to.
    Part,
    /// A named bus or room the part reaches by a declared edge.
    Bus,
}

/// One entry of the parameter-address table.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HostedControlAddress {
    /// The canonical source identity the address belongs to.
    pub identity: String,
    /// The address, which is never zero and never shared.
    pub address: u64,
}

/// The versioned, collision-detecting parameter-address table.
///
/// A hash is how an address is *proposed*; this table is what makes it
/// unique. It holds every identity that has ever been assigned an address in
/// this document, including controls the source no longer declares, so that a
/// removed control's address is retired rather than handed to a different
/// control that a host would then automate by mistake.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HostedControlTable {
    entries: Vec<HostedControlAddress>,
}

impl HostedControlTable {
    /// Every entry, sorted by identity.
    #[must_use]
    pub fn entries(&self) -> &[HostedControlAddress] {
        &self.entries
    }

    /// The address assigned to one identity, if it has ever had one.
    #[must_use]
    pub fn address_of(&self, identity: &str) -> Option<u64> {
        self.entries
            .binary_search_by(|entry| entry.identity.as_str().cmp(identity))
            .ok()
            .and_then(|index| self.entries.get(index))
            .map(|entry| entry.address)
    }

    /// Extend this table to cover `identities`, keeping every promise it made.
    ///
    /// An identity already in the table keeps its address. A new one is given
    /// the first address its own derivation proposes that no entry already
    /// holds — the collision detection the design asks for, and the reason a
    /// hash alone is not enough. Nothing is ever removed.
    #[must_use]
    pub fn extended(&self, identities: &[String]) -> Self {
        let mut entries = self.entries.clone();
        let mut used: std::collections::BTreeSet<u64> = entries.iter().map(|entry| entry.address).collect();
        for identity in identities {
            if entries.iter().any(|entry| &entry.identity == identity) {
                continue;
            }
            let mut salt = 0u64;
            let mut address = derive_address(identity, salt);
            while used.contains(&address) {
                salt = salt.wrapping_add(1);
                address = derive_address(identity, salt);
            }
            used.insert(address);
            entries.push(HostedControlAddress {
                identity: identity.clone(),
                address,
            });
        }
        entries.sort_by(|left, right| left.identity.cmp(&right.identity));
        Self { entries }
    }

    /// The table as one property-list-safe string.
    #[must_use]
    pub fn encode(&self) -> String {
        use std::fmt::Write as _;
        let mut text = format!("{HOSTED_TABLE_HEADER} {HOSTED_TABLE_VERSION}\n");
        for entry in &self.entries {
            let _ = writeln!(text, "{:016x}\t{}", entry.address, entry.identity);
        }
        text
    }

    /// Read a table a document saved.
    ///
    /// # Errors
    /// Returns the reason as a sentence when the header, the version, or the
    /// body is not what this component writes, and — the point of the type —
    /// when two entries share an address or an identity. A table that failed
    /// its own uniqueness check is not a table a host's automation can be
    /// restored against.
    pub fn decode(text: &str) -> Result<Self, String> {
        let mut lines = text.lines();
        let header = lines.next().ok_or_else(|| "the control table is empty".to_owned())?;
        let (name, version) = header
            .split_once(' ')
            .ok_or_else(|| format!("`{header}` is not a control-table header"))?;
        if name != HOSTED_TABLE_HEADER {
            return Err(format!("`{name}` is not a control table"));
        }
        let version: u32 = version
            .parse()
            .map_err(|_| format!("`{version}` is not a control-table version"))?;
        if version != HOSTED_TABLE_VERSION {
            return Err(format!(
                "this table is version {version}; this component writes version {HOSTED_TABLE_VERSION}"
            ));
        }
        let mut entries = Vec::new();
        for line in lines.filter(|line| !line.is_empty()) {
            let (address, identity) = line
                .split_once('\t')
                .ok_or_else(|| format!("`{line}` is not an address and an identity"))?;
            let address = u64::from_str_radix(address, 16).map_err(|_| format!("`{address}` is not an address"))?;
            if address == 0 {
                return Err(format!("`{identity}` is recorded at address zero, which names nothing"));
            }
            if entries
                .iter()
                .any(|entry: &HostedControlAddress| entry.address == address)
            {
                return Err(format!("address {address:016x} is recorded twice"));
            }
            if entries.iter().any(|entry| entry.identity == identity) {
                return Err(format!("`{identity}` is recorded twice"));
            }
            entries.push(HostedControlAddress {
                identity: identity.to_owned(),
                address,
            });
        }
        entries.sort_by(|left, right| left.identity.cmp(&right.identity));
        Ok(Self { entries })
    }
}

/// Propose one address for a canonical identity.
///
/// FNV-1a over the version, the salt, and the identity. Written out rather
/// than taken from a hasher whose output is documented as unstable: this
/// number is saved in host documents, so it has to mean the same thing in
/// next year's build.
fn derive_address(identity: &str, salt: u64) -> u64 {
    const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0000_0100_0000_01b3;
    let mut hash = OFFSET;
    for byte in u64::from(HOSTED_TABLE_VERSION)
        .to_be_bytes()
        .into_iter()
        .chain(salt.to_be_bytes())
        .chain(identity.bytes())
    {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(PRIME);
    }
    // Zero is reserved: a host that reads an address of zero has read an
    // uninitialized field, and it must not find a parameter there.
    if hash == 0 { PRIME } else { hash }
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
    controls: Vec<HostedControl>,
    losses: Vec<HostedControlLoss>,
    table: HostedControlTable,
    /// Address to control index, sorted, so the render side finds a
    /// parameter by binary search rather than by scanning names.
    addresses: Vec<(u64, usize)>,
    outputs: Vec<HostedOutput>,
    /// Tap index per projected output past bus zero, parallel to
    /// `outputs[1..]`.
    output_taps: Vec<usize>,
    /// Preallocated room for one frame of every prepared tap, so that
    /// reporting extra buses allocates nothing on the render side.
    tap_frame: Vec<[f32; 2]>,
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

    /// The source-declared controls this instrument exposes as parameters.
    ///
    /// Generated, in the order the source signature declares them. There is
    /// no catalogue anywhere else — an interface that added a control here
    /// would be adding one to the instrument, which is a source edit.
    #[must_use]
    pub fn controls(&self) -> &[HostedControl] {
        &self.controls
    }

    /// Every declared control this boundary could not carry, and why.
    #[must_use]
    pub fn control_losses(&self) -> &[HostedControlLoss] {
        &self.losses
    }

    /// The complete parameter-address table, to be saved with the document.
    #[must_use]
    pub const fn control_table(&self) -> &HostedControlTable {
        &self.table
    }

    /// The outputs a host may negotiate, bus zero first.
    #[must_use]
    pub fn outputs(&self) -> &[HostedOutput] {
        &self.outputs
    }

    /// Drive one exposed control by its host address. Real-time safe.
    ///
    /// This is an ephemeral performance overlay and nothing more: it writes
    /// no source, changes no declaration, and survives nothing. `ramp_frames`
    /// asks for a linear approach over that many frames and is honoured only
    /// where the source declares the control continuous.
    ///
    /// An address this instrument does not publish is [`HostedOutcome::Unbound`]
    /// rather than an error — a host restoring an old automation lane is
    /// entitled to send one, and the honest answer is that nothing here
    /// answers to it.
    pub fn set_control(&mut self, address: u64, value: f32, ramp_frames: u64) -> HostedOutcome {
        let Ok(found) = self.addresses.binary_search_by_key(&address, |entry| entry.0) else {
            return HostedOutcome::Unbound;
        };
        let Some(&(_, index)) = self.addresses.get(found) else {
            return HostedOutcome::Unbound;
        };
        HostedOutcome::from_dsp(self.audio.audition_control(self.target, index, value, ramp_frames))
    }

    /// Advance one frame and also report every projected output past bus zero.
    ///
    /// `into` is filled from the front, one stereo frame per entry of
    /// [`Self::outputs`] after the first; the return value is bus zero. It is
    /// the same frame [`Self::step`] would have produced — the extra outputs
    /// are reads of buffers this frame already wrote — so a host that
    /// negotiates one bus and a host that negotiates all of them hear the
    /// same main output. Real-time safe.
    pub fn step_outputs(&mut self, into: &mut [[f32; 2]]) -> [f32; 2] {
        let master = self.audio.audition_step_with_taps(&mut self.tap_frame);
        for (slot, tap) in into.iter_mut().zip(self.output_taps.iter()) {
            *slot = self.tap_frame.get(*tap).copied().unwrap_or([0.0; 2]);
        }
        if into.len() > self.output_taps.len() {
            for slot in into.iter_mut().skip(self.output_taps.len()) {
                *slot = [0.0; 2];
            }
        }
        master
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
    let losses = audio
        .audition_control_losses(target)
        .iter()
        .map(|loss| HostedControlLoss {
            identity: format!("{}::{}", loss.namespace(), loss.name()),
            kind: loss.kind().to_owned(),
            reason: refusal_reason(loss.refusal()).to_owned(),
        })
        .collect();
    // The table a document carried is a promise, so a table this component
    // cannot read is a refusal rather than a fresh start: silently assigning
    // new addresses would leave the host automating parameters that moved.
    let restored = match request.table.as_deref() {
        Some(text) => HostedControlTable::decode(text)
            .map_err(|reason| ProjectError::Performance(format!("the restored control table is unusable: {reason}")))?,
        None => HostedControlTable::default(),
    };
    let identities: Vec<String> = (0..audio.audition_control_count(target))
        .filter_map(|index| audio.audition_control_at(target, index))
        .map(|control| format!("{}::{}", control.namespace(), control.name()))
        .collect();
    let table = restored.extended(&identities);
    let mut controls = Vec::with_capacity(identities.len());
    for (index, identity) in identities.iter().enumerate() {
        let (Some(control), Some(default)) = (
            audio.audition_control_at(target, index),
            audio.audition_control_default(target, index),
        ) else {
            continue;
        };
        let address = table.address_of(identity).ok_or_else(|| {
            ProjectError::Performance(format!("`{identity}` was admitted but given no parameter address"))
        })?;
        controls.push(HostedControl {
            address,
            identity: identity.clone(),
            display: control.name().to_owned(),
            summary: control.summary().to_owned(),
            kind: control.kind().to_owned(),
            update_rate: control.update_rate().to_owned(),
            minimum: 0.0,
            maximum: 1.0,
            default,
            continuous: control.is_continuous(),
        });
    }
    let mut addresses: Vec<(u64, usize)> = controls
        .iter()
        .enumerate()
        .map(|(index, control)| (control.address, index))
        .collect();
    addresses.sort_unstable_by_key(|entry| entry.0);
    let (outputs, output_taps) = project_outputs(&audio, &request.part);
    let tap_frame = vec![[0.0; 2]; audio.taps().len()];
    Ok(HostedInstrument {
        audio,
        target,
        identity,
        inputs,
        controls,
        losses,
        table,
        addresses,
        outputs,
        output_taps,
        tap_frame,
    })
}

const fn refusal_reason(refusal: AuditionControlRefusal) -> &'static str {
    refusal.reason()
}

/// Which declared points this part's signal reaches, in stable source order.
///
/// Bus zero is the main output, and it is always there. The rest are the taps
/// the studio already publishes, kept only where a declared edge actually
/// carries this part to them: a Music Device rendering `violin` has no
/// business publishing `strings`, and reachability rather than a hand-written
/// list is what says so.
fn project_outputs(audio: &PreparedAudio, part: &str) -> (Vec<HostedOutput>, Vec<usize>) {
    use std::collections::BTreeSet;

    let mut reached: BTreeSet<&str> = BTreeSet::new();
    reached.insert(part);
    // The routing is finite and acyclic in the source, but this walk does not
    // need to know that: it stops when a pass adds nothing.
    loop {
        let mut grew = false;
        for route in audio.routes() {
            let carries = matches!(route.kind(), AudioRouteKind::Route | AudioRouteKind::Send);
            if carries && reached.contains(route.source()) && !reached.contains(route.destination()) {
                reached.insert(route.destination());
                grew = true;
            }
        }
        if !grew {
            break;
        }
    }
    let mut outputs = vec![HostedOutput {
        name: "main".to_owned(),
        role: HostedOutputRole::Main,
    }];
    let mut taps = Vec::new();
    for (index, tap) in audio.taps().iter().enumerate() {
        if !reached.contains(tap.name()) {
            continue;
        }
        outputs.push(HostedOutput {
            name: tap.name().to_owned(),
            role: match tap.role() {
                musa_dsp::AudioTapRole::Part => HostedOutputRole::Part,
                musa_dsp::AudioTapRole::Bus => HostedOutputRole::Bus,
            },
        });
        taps.push(index);
    }
    (outputs, taps)
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
        table: None,
    })
}

fn hex(bytes: &[u8; 32]) -> String {
    use std::fmt::Write as _;
    bytes.iter().fold(String::with_capacity(64), |mut text, byte| {
        let _ = write!(text, "{byte:02x}");
        text
    })
}
