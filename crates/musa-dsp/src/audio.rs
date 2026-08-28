//! Opaque preparation and one-frame execution for production audio.
//!
//! The public operation consumes exact performed gestures and an authored
//! studio value. It schedules first, prepares the closed native primitive
//! graph as a one-frame machine, and returns the one stateful object shared
//! by offline rendering and the live callback.
#![allow(clippy::arithmetic_side_effects)]

use musa_calculus::CheckedSource;
use musa_events::{Duration, PerformedTime, PhysicalTime, Position, empty};
use musa_score::{Gesture, GesturePlan, PartId, Tuning};
use num_rational::Ratio;

use crate::intent::StudioSpec;
use crate::plan::{PreparedParameterEvent, PreparedParameterId, RenderPlan, prepare_routed_plan_with_external};
use crate::primitive::{AudioLimits, resources};
use crate::schedule::{
    AudioFormat, ChannelLayout, Schedule, ScheduleError, SchedulePolicy, ScheduledSource, TimeDecision, TimeMap,
    merge_schedules, schedule,
};
use crate::spec::GraphOptions;
use crate::studio::{TapRole, lower_studio, lower_studio_for_sources};

/// Opaque prepared event input for one score part's audition instrument.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PreparedAuditionTarget(usize);

/// Which declared route a tap reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AudioTapRole {
    /// One part's own instrument chain, before any bus it sends to.
    Part,
    /// One named bus or return, after its own chain.
    Bus,
}

/// One point on the routing the source declared, readable beside the master.
///
/// A tap is an identity, not a mixer channel: it names a route that already
/// exists and carries no level, mute, or solo of its own. Reading one cannot
/// change what the master hears.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct AudioTap {
    role: AudioTapRole,
    name: String,
}

impl AudioTap {
    /// Whether this reads a part output or a bus output.
    pub const fn role(&self) -> AudioTapRole {
        self.role
    }

    /// The part or bus name the source wrote.
    pub fn name(&self) -> &str {
        &self.name
    }
}

/// Whether a declared edge carries the whole signal or a share of it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AudioRouteKind {
    /// `route a -> b`: the signal goes there instead.
    Route,
    /// `send a -> b`: a share of the signal goes there as well.
    Send,
}

/// One declared routing edge, reported so a consumer can read the projection
/// the taps sit on.
///
/// This is what makes the taps legible without promising they add up: a send
/// duplicates signal and a return may share nonlinear processing, so the
/// edges say where the signal went rather than how to reconstruct a mix.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct AudioRoute {
    kind: AudioRouteKind,
    source: String,
    destination: String,
}

impl AudioRoute {
    /// Whether the edge is a route or a send.
    pub const fn kind(&self) -> AudioRouteKind {
        self.kind
    }

    /// The part or bus the signal leaves.
    pub fn source(&self) -> &str {
        &self.source
    }

    /// The bus it arrives at, or `master`.
    pub fn destination(&self) -> &str {
        &self.destination
    }
}

/// One callback-safe event for ephemeral selected-instrument audition.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AuditionEvent {
    /// Begin one already-paired live voice.
    NoteOn {
        /// Control-side identity pairing this attack with its release.
        voice: u32,
        /// MIDI note number, realized only at this device edge.
        note: u8,
        /// Raw attack velocity.
        velocity: u8,
    },
    /// Release one already-paired live voice.
    NoteOff {
        /// Control-side identity of the attack being released.
        voice: u32,
        /// Raw release velocity.
        velocity: u8,
    },
    /// Apply one retained MIDI dimension through source audition policy.
    Input {
        /// MIDI wire dimension, not a musical meaning.
        input: crate::MidiAuditionInputKind,
        /// Raw signed value (`-8192..=8191` for bend, otherwise seven-bit).
        value: i16,
        /// Key for a key-scoped dimension.
        key: Option<u8>,
    },
}

/// One source-declared exposed control a host may drive directly.
///
/// The value is the *semantic* control of `08-performance-and-sound.md` §3 —
/// an exact normalized intention — and never a private DSP parameter.
/// `06-daw-boundary.md` Rule D2 admits exactly these and nothing else, so a
/// consumer projecting a parameter tree reads this list and stops.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuditionControl {
    namespace: String,
    name: String,
    summary: String,
    kind: String,
    update_rate: String,
    default_ratio: Ratio<i64>,
}

impl AuditionControl {
    /// The namespace the source key declares.
    pub fn namespace(&self) -> &str {
        &self.namespace
    }

    /// The name the source key declares.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The one sentence the source key declares about what this means.
    pub fn summary(&self) -> &str {
        &self.summary
    }

    /// The source value kind. Every admitted control is `Normalized`; the
    /// field is carried anyway because a consumer that recorded the kind can
    /// say what changed when a later edition admits another one.
    pub fn kind(&self) -> &str {
        &self.kind
    }

    /// The source update rate, spelled as the declaration spells it.
    pub fn update_rate(&self) -> &str {
        &self.update_rate
    }

    /// The exact declared default, in the control's own normalized domain.
    pub const fn default_ratio(&self) -> Ratio<i64> {
        self.default_ratio
    }

    /// Whether the declaration says this control changes continuously.
    ///
    /// A per-note or per-transition control still accepts a host value; what
    /// it does not accept is a ramp, because the source does not say the
    /// quantity moves between its statements.
    pub fn is_continuous(&self) -> bool {
        self.update_rate == "Continuous"
    }
}

/// Why one source-declared control is not projected to a host.
///
/// `06-daw-boundary.md` §6: a presentation may lose information and may not
/// lose it silently. The control keeps working inside Musa; what is recorded
/// here is that this *boundary* cannot carry it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuditionControlLoss {
    namespace: String,
    name: String,
    kind: String,
    refusal: AuditionControlRefusal,
}

impl AuditionControlLoss {
    /// The namespace of the control that is not projected.
    pub fn namespace(&self) -> &str {
        &self.namespace
    }

    /// The name of the control that is not projected.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The source value kind that could not be carried.
    pub fn kind(&self) -> &str {
        &self.kind
    }

    /// Which refusal this is, as a fact rather than a sentence.
    pub const fn refusal(&self) -> AuditionControlRefusal {
        self.refusal
    }
}

/// The named reasons a declared control is not admitted at a host boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AuditionControlRefusal {
    /// The value kind is a typed constructor set rather than a quantity —
    /// `phrase` is a grouping relation, and coercing it to a float would be
    /// exactly the invention `08-performance-and-sound.md` §3 forbids.
    NotScalar,
    /// The value kind is a quantity, but the source declares no minimum and
    /// maximum for it. A host parameter without a domain is a control whose
    /// ends nobody agreed on.
    NoDeclaredDomain,
    /// The selected implementation maps this control to nothing, so a host
    /// parameter for it would move and be inaudible.
    Unmapped,
}

impl AuditionControlRefusal {
    /// The refusal as one clause, for a consumer that has to print it.
    pub const fn reason(self) -> &'static str {
        match self {
            Self::NotScalar => "its value kind is a typed relation rather than a quantity",
            Self::NoDeclaredDomain => "its source declaration states no value domain",
            Self::Unmapped => "the selected implementation maps it to nothing",
        }
    }
}

/// Whether source policy could interpret an audition event's expressive part.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AuditionOutcome {
    /// The note or control was applied through prepared source policy.
    Applied,
    /// The note sounded, but its hardware expression had no source binding.
    NoteWithoutExpression,
    /// The selected instrument declares no binding for this dimension.
    UnsupportedInput,
}

/// Every product-level choice that can change audio preparation or its finite
/// playback extent. There is deliberately no default.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AudioOptions {
    /// Physical frame lattice.
    pub format: AudioFormat,
    /// Exact-to-frame and finite-resource policy.
    pub schedule: SchedulePolicy,
    /// Explicit pitch-to-frequency interpretation.
    pub tuning: Tuning,
    /// Explicit seed for every stochastic primitive in this render.
    pub render_seed: u64,
    /// Native primitive memory and per-frame work bounds.
    pub limits: AudioLimits,
    /// Frames retained after the last scheduled boundary for release tails.
    pub tail_frames: u64,
    /// Greatest accepted schedule plus tail.
    pub max_total_frames: u64,
}

/// A stable failure before any machine reaches a real-time thread.
#[derive(Clone, Debug, PartialEq, thiserror::Error)]
pub enum AudioPrepareError {
    /// Checked frame scheduling failed.
    #[error("gesture scheduling failed: {0}")]
    Schedule(#[from] ScheduleError),
    /// Native primitive graph validation or preparation failed.
    #[error("audio primitive preparation failed: {0}")]
    Primitive(String),
    /// Exact studio intent failed conversion or a private DSP range check.
    #[error("studio value preparation failed: {0}")]
    StudioValue(String),
    /// Checked source instrument declarations failed schema or contract checks.
    #[error("instrument contract preparation failed: {0}")]
    InstrumentContract(String),
    /// The requested tail makes the finite playback extent unacceptable.
    #[error("audio extent {actual} frames exceeds explicit limit {limit}")]
    FrameLimit { actual: u64, limit: u64 },
    /// The production machine has one stereo-frame reference meaning.
    #[error("unsupported audio layout {0:?}; production audio requires stereo")]
    UnsupportedLayout(ChannelLayout),
    /// Pitch interpretation must produce positive finite frequencies.
    #[error("concert A must be a positive finite frequency, got {0}")]
    InvalidTuning(f64),
    /// A checked native resource exceeds the product's explicit bound.
    #[error("audio {resource} requirement {actual} exceeds explicit limit {limit}")]
    ResourceLimit {
        /// Resource class with stable spelling.
        resource: &'static str,
        /// Conservatively priced requirement.
        actual: u64,
        /// Product-supplied bound.
        limit: u64,
    },
}

/// A fully scheduled, allocated audio machine. Its graph, state, buffers,
/// event cursor, and handle tables are intentionally inaccessible.
pub struct PreparedAudio {
    // Retain the complete exact checked argument. A digest or the queried
    // fields below cannot stand in for preparation identity (R1).
    _instrument_contracts: crate::InstrumentContracts,
    // The source instrument component is prepared through the governing
    // machine calculus beside the source-derived native graph projection.
    _instrument_machine: crate::PreparedMachine,
    // Exact source identities retained beside their compact prepared slots.
    // Equal declarations deliberately still have distinct instance ids.
    instrument_instances: Vec<InstrumentInstanceBinding>,
    audition: Vec<PreparedInstrumentAudition>,
    plan: RenderPlan,
    schedule: Schedule<PerformedTime, Gesture>,
    lanes: Vec<PreparedLane>,
    media: Option<crate::PreparedMedia>,
    media_inputs: Vec<String>,
    // Tap i in this list is buffer tap i of the plan. The compact index is
    // private; what crosses the facade is the source identity beside it.
    taps: Vec<AudioTap>,
    routes: Vec<AudioRoute>,
    tuning: Tuning,
    sample_rate: u32,
    total_frames: u64,
    position: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct PreparedInstrumentId(usize);

struct InstrumentInstanceBinding {
    _part: PartId,
    name: String,
    _declaration: String,
    _instance: PreparedInstrumentId,
}

struct PreparedInstrumentAudition {
    bindings: Vec<PreparedAuditionBinding>,
    controls: Vec<PreparedAuditionControl>,
    losses: Vec<AuditionControlLoss>,
}

/// One admitted control beside the private targets its source mapping names.
struct PreparedAuditionControl {
    control: AuditionControl,
    default_value: f32,
    mappings: Box<[PreparedControlTarget]>,
}

/// One private parameter a control reaches, and the source transfer to it.
struct PreparedControlTarget {
    target: PreparedParameterId,
    transfer: Option<(f32, f32, bool)>,
}

struct PreparedAuditionBinding {
    input: crate::MidiAuditionInputKind,
    scope: crate::MidiAuditionScope,
    input_minimum: f32,
    input_maximum: f32,
    output_minimum: f32,
    output_maximum: f32,
    dead_zone: f32,
    switch_threshold: Option<f32>,
    mappings: Box<[PreparedAuditionMapping]>,
}

struct PreparedAuditionMapping {
    target: PreparedParameterId,
    transfer: Option<(f32, f32, bool)>,
    per_note_attack: bool,
}

struct PreparedLane {
    instance: PreparedInstrumentId,
    source: ScheduledSource<Gesture>,
    controls: Vec<PreparedControlBatch>,
    control_cursor: usize,
}

struct PreparedControlBatch {
    frame: u64,
    events: Box<[PreparedParameterEvent]>,
}

impl PreparedAudio {
    /// Resolve a score part name to its opaque prepared audition input.
    pub fn audition_target(&self, part: &str) -> Option<PreparedAuditionTarget> {
        self.instrument_instances
            .iter()
            .position(|binding| binding.name == part)
            .map(PreparedAuditionTarget)
    }

    /// Whether the selected instrument has a source binding for one input.
    pub fn supports_audition_input(&self, target: PreparedAuditionTarget, input: crate::MidiAuditionInputKind) -> bool {
        self.audition
            .get(target.0)
            .is_some_and(|prepared| prepared.bindings.iter().any(|binding| binding.input == input))
    }

    /// How many source-declared controls this instrument exposes to a host.
    pub fn audition_control_count(&self, target: PreparedAuditionTarget) -> usize {
        self.audition
            .get(target.0)
            .map_or(0, |prepared| prepared.controls.len())
    }

    /// One admitted control, in the order the source signature declares them.
    ///
    /// The order is the declaration's, so a consumer that keeps an index is
    /// keeping a source fact and not a preparation accident.
    pub fn audition_control_at(&self, target: PreparedAuditionTarget, index: usize) -> Option<&AuditionControl> {
        self.audition
            .get(target.0)?
            .controls
            .get(index)
            .map(|prepared| &prepared.control)
    }

    /// The exact declared default of one admitted control, as a DSP value.
    pub fn audition_control_default(&self, target: PreparedAuditionTarget, index: usize) -> Option<f32> {
        Some(self.audition.get(target.0)?.controls.get(index)?.default_value)
    }

    /// Every declared control this boundary cannot carry, and why.
    pub fn audition_control_losses(&self, target: PreparedAuditionTarget) -> &[AuditionControlLoss] {
        self.audition.get(target.0).map_or(&[][..], |prepared| &prepared.losses)
    }

    /// Drive one admitted control directly, as an ephemeral overlay.
    ///
    /// This is the host-parameter half of audition. The value is the semantic
    /// control's own normalized quantity; what reaches the private graph is
    /// whatever the instrument's declared mapping says, exactly as a
    /// scheduled control statement would reach it. Nothing here writes source
    /// (Rule D1) and nothing here addresses a DSP node (Rule D2).
    ///
    /// `ramp_frames` is honoured only for a control the source declares
    /// continuous; a per-note or per-transition control takes the value as a
    /// point change, because the declaration does not say the quantity moves
    /// between its statements. Allocation-free.
    pub fn audition_control(
        &mut self,
        target: PreparedAuditionTarget,
        index: usize,
        value: f32,
        ramp_frames: u64,
    ) -> AuditionOutcome {
        let Some(prepared) = self.audition.get(target.0) else {
            return AuditionOutcome::UnsupportedInput;
        };
        let Some(control) = prepared.controls.get(index) else {
            return AuditionOutcome::UnsupportedInput;
        };
        let semantic = value.clamp(0.0, 1.0);
        let ramp_frames = if control.control.is_continuous() {
            ramp_frames
        } else {
            0
        };
        for mapping in &control.mappings {
            let value = mapping.transfer.map_or(semantic, |(minimum, maximum, inverse)| {
                let along = if inverse { 1.0 - semantic } else { semantic };
                (maximum - minimum).mul_add(along, minimum)
            });
            self.plan.apply_parameter_events(&[PreparedParameterEvent {
                target: mapping.target,
                value,
                ramp_frames,
            }]);
        }
        AuditionOutcome::Applied
    }

    /// Apply one ephemeral event without advancing musical transport time.
    ///
    /// This operation allocates nothing and touches only prepared state.
    pub fn audition(&mut self, target: PreparedAuditionTarget, event: AuditionEvent) -> AuditionOutcome {
        match event {
            AuditionEvent::NoteOn { voice, note, velocity } => {
                let (supported, attack) = self.apply_audition_input(
                    target,
                    crate::MidiAuditionInputKind::AttackVelocity,
                    i16::from(velocity),
                    Some(note),
                );
                self.plan
                    .live_note_on(target.0, voice, note, 1.0, attack.unwrap_or(0.0), self.tuning);
                if supported {
                    AuditionOutcome::Applied
                } else {
                    AuditionOutcome::NoteWithoutExpression
                }
            }
            AuditionEvent::NoteOff { voice, velocity } => {
                let (supported, _) = self.apply_audition_input(
                    target,
                    crate::MidiAuditionInputKind::ReleaseVelocity,
                    i16::from(velocity),
                    None,
                );
                self.plan.live_note_off(target.0, voice);
                if supported {
                    AuditionOutcome::Applied
                } else {
                    AuditionOutcome::NoteWithoutExpression
                }
            }
            AuditionEvent::Input { input, value, key } => {
                if self.apply_audition_input(target, input, value, key).0 {
                    AuditionOutcome::Applied
                } else {
                    AuditionOutcome::UnsupportedInput
                }
            }
        }
    }

    /// Silence every live voice of one prepared instrument at once.
    ///
    /// Allocation-free, and immediate rather than released: a host that
    /// stops, loops, or reports a discontinuity is saying this instrument
    /// stops *there*, and a release tail crossing that seam would be Musa
    /// sounding somewhere the host did not put it. Scheduled playback is
    /// untouched — this is the audition half of the plan.
    pub fn silence_audition(&mut self, target: PreparedAuditionTarget) {
        self.plan.live_silence(target.0);
    }

    /// Advance the prepared DSP state once without advancing the score.
    ///
    /// This is the audition counterpart of [`Self::step`]: release tails and
    /// effects move while transport remains stopped.
    pub fn audition_step(&mut self) -> [f32; 2] {
        self.plan.finish_step()
    }

    /// The same audition frame, also reporting what each tap wrote during it.
    ///
    /// The audition counterpart of [`Self::step_with_taps`], and identical to
    /// [`Self::audition_step`] in every effect it has: the taps are reads of
    /// buffers this frame already wrote, so the master returned here is the
    /// master `audition_step` would have returned. That is what lets a host
    /// enable or ignore extra output buses without changing the main one.
    pub fn audition_step_with_taps(&mut self, into: &mut [[f32; 2]]) -> [f32; 2] {
        let master = self.plan.finish_step();
        for (index, slot) in into.iter_mut().enumerate() {
            *slot = if index < self.taps.len() {
                self.plan.tap_frame(index)
            } else {
                [0.0; 2]
            };
        }
        master
    }

    fn apply_audition_input(
        &mut self,
        target: PreparedAuditionTarget,
        input: crate::MidiAuditionInputKind,
        raw: i16,
        key: Option<u8>,
    ) -> (bool, Option<f32>) {
        let Some(prepared) = self.audition.get(target.0) else {
            return (false, None);
        };
        let mut supported = false;
        let mut attack = None;
        for binding in prepared.bindings.iter().filter(|binding| binding.input == input) {
            if binding.scope == crate::MidiAuditionScope::PerKey && key.is_none() {
                continue;
            }
            let raw = f32::from(raw);
            let normalized = if let Some(threshold) = binding.switch_threshold {
                if raw >= threshold { 1.0 } else { 0.0 }
            } else {
                let centered = if raw.abs() <= binding.dead_zone { 0.0 } else { raw };
                ((centered - binding.input_minimum) / (binding.input_maximum - binding.input_minimum)).clamp(0.0, 1.0)
            };
            let semantic =
                (binding.output_maximum - binding.output_minimum).mul_add(normalized, binding.output_minimum);
            for mapping in &binding.mappings {
                let value = mapping.transfer.map_or(semantic, |(minimum, maximum, inverse)| {
                    let along = if inverse { 1.0 - semantic } else { semantic };
                    (maximum - minimum).mul_add(along, minimum)
                });
                if mapping.per_note_attack {
                    attack = Some(value);
                } else {
                    self.plan.apply_parameter_events(&[PreparedParameterEvent {
                        target: mapping.target,
                        value,
                        ramp_frames: 0,
                    }]);
                }
            }
            supported = true;
        }
        (supported, attack)
    }

    /// Complete auditable time decisions for all gesture lanes.
    pub fn decisions(&self) -> &[TimeDecision<PerformedTime>] {
        self.schedule.decisions()
    }

    /// Finite playback extent, including the explicit release tail.
    pub const fn total_frames(&self) -> u64 {
        self.total_frames
    }

    /// Prepared physical sample rate.
    pub const fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// Frame that will be produced by the next call to [`Self::step`].
    pub const fn position(&self) -> u64 {
        self.position
    }

    /// Move the event source to an absolute frame. DSP state is retained,
    /// matching transport seek rather than reconstructing the instrument.
    pub fn seek(&mut self, frame: u64) {
        self.position = frame.min(self.total_frames);
        for lane in &mut self.lanes {
            lane.source.seek(self.position);
            lane.control_cursor = lane.controls.partition_point(|batch| batch.frame < self.position);
        }
    }

    /// Every readable point on the routing this studio declared, in the
    /// order a tapped step reports them: part outputs first, then buses.
    ///
    /// The list is derived from checked source routing alone. It is empty for
    /// a studio that declares nothing, and it never contains the master —
    /// which [`Self::step`] already returns.
    ///
    /// A tap reports what its route delivered towards the master. A part the
    /// studio routes nowhere delivers nothing, so its tap is silent; that is
    /// the studio's statement about the part, not a failure to read it.
    pub fn taps(&self) -> &[AudioTap] {
        &self.taps
    }

    /// Every declared routing edge, so a consumer can read what the taps sit
    /// on without being told the taps sum to the master. They do not: a send
    /// duplicates signal, and a nonlinear master chain is not the sum of what
    /// reaches it.
    pub fn routes(&self) -> &[AudioRoute] {
        &self.routes
    }

    /// Execute exactly one reference frame, reading this frame's event batch
    /// before producing its stereo output. No allocation, lock, I/O, or log
    /// occurs on this path.
    pub fn step(&mut self) -> [f32; 2] {
        if self.position >= self.total_frames {
            return [0.0; 2];
        }
        for lane in &mut self.lanes {
            if let Some(batch) = lane.controls.get(lane.control_cursor)
                && batch.frame == self.position
            {
                self.plan.apply_parameter_events(&batch.events);
                lane.control_cursor = lane.control_cursor.saturating_add(1);
            }
            let (_, messages) = lane.source.step();
            self.plan.apply_events(lane.instance.0, messages, self.tuning);
        }
        if let Some(media) = &self.media {
            for slot in 0..self.media_inputs.len() {
                self.plan
                    .apply_external_frame(slot, media.slot_frame(slot, self.position));
            }
        }
        let frame = self.plan.finish_step();
        self.position = self.position.saturating_add(1);
        frame
    }

    /// Execute the same one reference frame and also report what each tap
    /// wrote during it.
    ///
    /// `into` is filled from the front, one stereo frame per entry of
    /// [`Self::taps`]; entries beyond that are zeroed, and a shorter slice
    /// simply reports fewer taps. Allocation-free, and identical to
    /// [`Self::step`] in every effect it has on state — the taps are reads of
    /// buffers the frame already wrote, so the master returned here is the
    /// master `step` would have returned.
    pub fn step_with_taps(&mut self, into: &mut [[f32; 2]]) -> [f32; 2] {
        // Past the finite extent nothing is stepped, so the plan's buffers
        // still hold the last frame that was. Reporting them again would
        // invent signal after the piece ended.
        let sounding = self.position < self.total_frames;
        let master = self.step();
        for (index, slot) in into.iter_mut().enumerate() {
            *slot = if sounding && index < self.taps.len() {
                self.plan.tap_frame(index)
            } else {
                [0.0; 2]
            };
        }
        master
    }

    /// Fill an interleaved stereo host block by repeated reference steps.
    /// Host partitioning therefore cannot alter state or sound.
    pub fn render(&mut self, output: &mut [f32]) {
        let (frames, remainder) = output.as_chunks_mut::<2>();
        for frame in frames {
            let [left, right] = self.step();
            *frame = [left, right];
        }
        remainder.fill(0.0);
    }
}

/// Prepare exact gestures, checked scheduling, instruments, routing, effects,
/// and finite playback extent as one audio machine.
///
/// # Errors
/// [`AudioPrepareError`] names the scheduling, primitive, or extent check that
/// refused preparation.
pub(crate) fn prepare_projected_execution(
    gestures: &GesturePlan,
    instruments: &CheckedSource,
    instrument_machine: &musa_score::MachineSpec,
    studio: &StudioSpec,
    media: Option<crate::PreparedMedia>,
    options: AudioOptions,
) -> Result<PreparedAudio, AudioPrepareError> {
    let contracts = crate::decode_instrument_contracts(instruments)
        .map_err(|error| AudioPrepareError::InstrumentContract(error.to_string()))?;
    let basic = contracts.declaration("std.sound.basic_sine@1").ok_or_else(|| {
        AudioPrepareError::InstrumentContract("the edition-one basic instrument is absent".to_owned())
    })?;
    if basic.channels() != 2 {
        return Err(AudioPrepareError::InstrumentContract(
            "the edition-one basic instrument must promise stereo output".to_owned(),
        ));
    }
    if !basic
        .controls()
        .iter()
        .any(|control| control.namespace() == "std.performance" && control.name() == "expression")
    {
        return Err(AudioPrepareError::InstrumentContract(
            "the edition-one basic instrument does not accept standard expression".to_owned(),
        ));
    }
    let instrument_machine = crate::prepare_machine(instrument_machine)
        .map_err(|error| AudioPrepareError::InstrumentContract(error.to_string()))?;
    prepare_audio(gestures, contracts, instrument_machine, studio, media, options)
}

/// Prepare audio from the one checked production studio value.
///
/// # Errors
/// Refuses malformed checked source before any primitive is allocated, then
/// reports the same scheduling, primitive, and extent errors as
/// [`AudioPrepareError`].
pub fn prepare_execution(
    gestures: &GesturePlan,
    instruments: &CheckedSource,
    instrument_machine: &musa_score::MachineSpec,
    studio: &crate::StudioExecution,
    options: AudioOptions,
) -> Result<PreparedAudio, AudioPrepareError> {
    let projection = studio.preparation_projection().ok_or_else(|| {
        AudioPrepareError::StudioValue("checked studio processor has no native preparation witness".to_owned())
    })?;
    prepare_projected_execution(gestures, instruments, instrument_machine, &projection, None, options)
}

/// Prepare the production graph with already decoded recorded media.
///
/// The media value can only be obtained from [`crate::prepare_media`],
/// keeping verified-byte loading and decoding on the control side.
///
/// # Errors
/// Reports the same checked preparation failures as [`prepare_execution`].
pub fn prepare_execution_with_media(
    gestures: &GesturePlan,
    instruments: &CheckedSource,
    instrument_machine: &musa_score::MachineSpec,
    studio: &crate::StudioExecution,
    media: crate::PreparedMedia,
    options: AudioOptions,
) -> Result<PreparedAudio, AudioPrepareError> {
    let projection = studio.preparation_projection().ok_or_else(|| {
        AudioPrepareError::StudioValue("checked studio processor has no native preparation witness".to_owned())
    })?;
    prepare_projected_execution(
        gestures,
        instruments,
        instrument_machine,
        &projection,
        Some(media),
        options,
    )
}

fn prepare_audio(
    gestures: &GesturePlan,
    instrument_contracts: crate::InstrumentContracts,
    instrument_machine: crate::PreparedMachine,
    studio: &StudioSpec,
    media: Option<crate::PreparedMedia>,
    options: AudioOptions,
) -> Result<PreparedAudio, AudioPrepareError> {
    let basic = instrument_contracts
        .declaration("std.sound.basic_sine@1")
        .ok_or_else(|| {
            AudioPrepareError::InstrumentContract("the edition-one basic instrument is absent".to_owned())
        })?;
    if options.format.layout() != ChannelLayout::Stereo {
        return Err(AudioPrepareError::UnsupportedLayout(options.format.layout()));
    }
    if !options.tuning.concert_a.is_finite() || options.tuning.concert_a <= 0.0 {
        return Err(AudioPrepareError::InvalidTuning(options.tuning.concert_a));
    }
    let sample_rate = options.format.sample_rate().get();
    let graph_options = GraphOptions {
        sample_rate,
        render_seed: options.render_seed,
    };
    let part_names = gestures
        .lanes()
        .iter()
        .map(musa_score::GestureLane::name)
        .collect::<Vec<_>>();
    let media_names = media.as_ref().map_or(&[][..], crate::PreparedMedia::names);
    let media_name_refs = media_names.iter().map(String::as_str).collect::<Vec<_>>();
    let (graph, lowering, part_inputs, prepared_media_inputs) = if part_names.is_empty() && media_name_refs.is_empty() {
        let (graph, lowering) = lower_studio(studio, &graph_options);
        (graph, lowering, Vec::new(), Vec::new())
    } else {
        lower_studio_for_sources(studio, &part_names, &media_name_refs, &graph_options)
    };
    if !lowering.errors.is_empty() {
        return Err(AudioPrepareError::StudioValue(lowering.errors.join("; ")));
    }
    if graph.output_kind() != Some(crate::spec::PortKind::Audio { channels: 2 }) {
        return Err(AudioPrepareError::Primitive(
            "native graph output must be one stereo frame".to_owned(),
        ));
    }
    let mut required = resources(&graph, sample_rate, options.schedule.limits().max_messages).ok_or_else(|| {
        AudioPrepareError::Primitive("native primitive is absent from the closed registry".to_owned())
    })?;
    required.state_bytes = required
        .state_bytes
        .saturating_add(media_names.len().saturating_mul(std::mem::size_of::<[f32; 2]>()));
    required.step_work = required.step_work.saturating_add(media_names.len() as u64 * 2);
    check_resources(required, options.limits)?;
    let mut instrument_instances = Vec::with_capacity(gestures.lanes().len());
    let mut event_inputs = Vec::with_capacity(gestures.lanes().len());
    for (instance, lane) in gestures.lanes().iter().enumerate() {
        let input = part_inputs
            .iter()
            .find(|input| input.part == lane.name())
            .ok_or_else(|| {
                AudioPrepareError::StudioValue(format!("part `{}` has no prepared instrument instance", lane.name()))
            })?;
        event_inputs.push((input.node, instance));
        instrument_instances.push(InstrumentInstanceBinding {
            _part: lane.part(),
            name: lane.name().to_owned(),
            _declaration: input.declaration.clone(),
            _instance: PreparedInstrumentId(instance),
        });
    }
    let external_inputs = prepared_media_inputs
        .iter()
        .enumerate()
        .map(|(slot, input)| (input.node, slot))
        .collect::<Vec<_>>();
    let tap_nodes = lowering.taps.iter().map(|tap| tap.node).collect::<Vec<_>>();
    let mut plan =
        prepare_routed_plan_with_external(&graph, &graph_options, &event_inputs, &external_inputs, &tap_nodes)
            .map_err(|error| AudioPrepareError::Primitive(error.to_string()))?;
    let (schedule, lane_schedules) = schedule_gestures(gestures, options)?;
    let mut lane_controls = Vec::with_capacity(lane_schedules.len());
    let mut audition = Vec::with_capacity(lane_schedules.len());
    for (instance, lane_schedule) in lane_schedules.iter().enumerate() {
        let input = part_inputs.get(instance).ok_or_else(|| {
            AudioPrepareError::StudioValue("a scheduled lane has no prepared instrument input".to_owned())
        })?;
        let declaration = instrument_contracts.declaration(&input.declaration).unwrap_or(basic);
        lane_controls.push(prepare_control_batches(
            &mut plan,
            input.node,
            declaration,
            lane_schedule,
        )?);
        audition.push(prepare_audition_bindings(&mut plan, input.node, declaration)?);
    }
    let studio_tail = (f64::from(lowering.release_tail) * f64::from(sample_rate)) as u64;
    let tail_frames = options.tail_frames.saturating_add(studio_tail);
    let sounding_finish = media.as_ref().map_or_else(
        || schedule.finish_frame(),
        |media| media.finish_frame().max(schedule.finish_frame()),
    );
    let total_frames = sounding_finish
        .checked_add(tail_frames)
        .filter(|frames| *frames <= options.max_total_frames)
        .ok_or_else(|| AudioPrepareError::FrameLimit {
            actual: sounding_finish.saturating_add(tail_frames),
            limit: options.max_total_frames,
        })?;
    let lanes = lane_schedules
        .iter()
        .zip(lane_controls)
        .enumerate()
        .map(|(instance, (schedule, controls))| PreparedLane {
            instance: PreparedInstrumentId(instance),
            source: schedule.source(),
            controls,
            control_cursor: 0,
        })
        .collect();
    let taps = lowering
        .taps
        .iter()
        .map(|tap| AudioTap {
            role: match tap.role {
                TapRole::Part => AudioTapRole::Part,
                TapRole::Bus => AudioTapRole::Bus,
            },
            name: tap.name.clone(),
        })
        .collect();
    let routes = studio
        .routes()
        .iter()
        .map(|route| AudioRoute {
            kind: AudioRouteKind::Route,
            source: route.source.clone(),
            destination: route.destination.clone(),
        })
        .chain(studio.sends().iter().map(|send| AudioRoute {
            kind: AudioRouteKind::Send,
            source: send.source.clone(),
            destination: send.bus.clone(),
        }))
        .collect();
    Ok(PreparedAudio {
        _instrument_contracts: instrument_contracts,
        _instrument_machine: instrument_machine,
        instrument_instances,
        audition,
        plan,
        schedule,
        lanes,
        media,
        media_inputs: prepared_media_inputs.into_iter().map(|input| input.name).collect(),
        taps,
        routes,
        tuning: options.tuning,
        sample_rate,
        total_frames,
        position: 0,
    })
}

struct ResolvedControlMapping {
    kind: String,
    namespace: String,
    name: String,
    continuous: bool,
    target: PreparedParameterId,
    transfer: Option<(Ratio<i64>, Ratio<i64>, bool)>,
    connection_values: Option<[Ratio<i64>; 3]>,
}

fn prepare_audition_bindings(
    plan: &mut RenderPlan,
    node: crate::spec::NodeId,
    declaration: &crate::InstrumentContract,
) -> Result<PreparedInstrumentAudition, AudioPrepareError> {
    let mut bindings = Vec::with_capacity(declaration.audition_bindings().len());
    for source in declaration.audition_bindings() {
        let mut mappings = Vec::new();
        for mapping in declaration.mappings().iter().filter(|mapping| {
            mapping.kind() == "Normalized"
                && mapping.namespace() == source.namespace()
                && mapping.name() == source.name()
        }) {
            if mapping.node() != "voice" {
                return Err(AudioPrepareError::InstrumentContract(format!(
                    "audition control `{}::{}` reaches unsupported private target `{}`",
                    source.namespace(),
                    source.name(),
                    mapping.node()
                )));
            }
            let Some(target) = plan
                .resolve_parameter(node, mapping.parameter())
                .map_err(|error| AudioPrepareError::InstrumentContract(error.to_string()))?
            else {
                continue;
            };
            let per_note_attack = source.scope() == crate::MidiAuditionScope::PerKey
                && source.input() == crate::MidiAuditionInputKind::AttackVelocity
                && plan.parameter_name(target) == Some("attack");
            if source.scope() == crate::MidiAuditionScope::PerKey && !per_note_attack {
                return Err(AudioPrepareError::InstrumentContract(format!(
                    "key-scoped audition control `{}::{}` does not reach a prepared per-note target",
                    source.namespace(),
                    source.name()
                )));
            }
            let transfer = mapping
                .transfer()
                .map(|(minimum, maximum, inverse)| {
                    Ok::<_, AudioPrepareError>((exact_f32(minimum)?, exact_f32(maximum)?, inverse))
                })
                .transpose()?;
            mappings.push(PreparedAuditionMapping {
                target,
                transfer,
                per_note_attack,
            });
        }
        if mappings.is_empty() {
            continue;
        }
        let (input_minimum, input_maximum) = source.input_range();
        let (output_minimum, output_maximum) = source.output_range();
        bindings.push(PreparedAuditionBinding {
            input: source.input(),
            scope: source.scope(),
            input_minimum: exact_f32(input_minimum)?,
            input_maximum: exact_f32(input_maximum)?,
            output_minimum: exact_f32(output_minimum)?,
            output_maximum: exact_f32(output_maximum)?,
            dead_zone: exact_f32(source.dead_zone())?,
            switch_threshold: source.switch_threshold().map(exact_f32).transpose()?,
            mappings: mappings.into_boxed_slice(),
        });
    }
    let (controls, losses) = prepare_audition_controls(plan, node, declaration)?;
    Ok(PreparedInstrumentAudition {
        bindings,
        controls,
        losses,
    })
}

/// Admit the source-declared controls a host may drive, and record the rest.
///
/// Admission is a question about the *declaration*, not about what would be
/// convenient: a control is admitted when its source value kind is a quantity
/// with a declared domain and the selected implementation maps it somewhere.
/// Everything else stays available inside Musa and leaves a named loss, which
/// is `06-daw-boundary.md` §6 rather than a policy invented here.
fn prepare_audition_controls(
    plan: &mut RenderPlan,
    node: crate::spec::NodeId,
    declaration: &crate::InstrumentContract,
) -> Result<(Vec<PreparedAuditionControl>, Vec<AuditionControlLoss>), AudioPrepareError> {
    let mut controls = Vec::new();
    let mut losses = Vec::new();
    for control in declaration.controls() {
        let refuse = |refusal| AuditionControlLoss {
            namespace: control.namespace().to_owned(),
            name: control.name().to_owned(),
            kind: control.kind().to_owned(),
            refusal,
        };
        if control.kind() != "Normalized" {
            losses.push(refuse(if control.kind() == "PhraseConnection" {
                AuditionControlRefusal::NotScalar
            } else {
                AuditionControlRefusal::NoDeclaredDomain
            }));
            continue;
        }
        let Some(default_ratio) = control.default_ratio() else {
            losses.push(refuse(AuditionControlRefusal::NoDeclaredDomain));
            continue;
        };
        let mut mappings = Vec::new();
        for mapping in declaration.mappings().iter().filter(|mapping| {
            mapping.kind() == control.kind()
                && mapping.namespace() == control.namespace()
                && mapping.name() == control.name()
        }) {
            let Some(target) = plan
                .resolve_parameter(node, mapping.parameter())
                .map_err(|error| AudioPrepareError::InstrumentContract(error.to_string()))?
            else {
                continue;
            };
            let transfer = mapping
                .transfer()
                .map(|(minimum, maximum, inverse)| {
                    Ok::<_, AudioPrepareError>((exact_f32(minimum)?, exact_f32(maximum)?, inverse))
                })
                .transpose()?;
            mappings.push(PreparedControlTarget { target, transfer });
        }
        if mappings.is_empty() {
            losses.push(refuse(AuditionControlRefusal::Unmapped));
            continue;
        }
        controls.push(PreparedAuditionControl {
            control: AuditionControl {
                namespace: control.namespace().to_owned(),
                name: control.name().to_owned(),
                summary: control.summary().to_owned(),
                kind: control.kind().to_owned(),
                update_rate: control.update_rate().to_owned(),
                default_ratio,
            },
            default_value: exact_f32(default_ratio)?,
            mappings: mappings.into_boxed_slice(),
        });
    }
    Ok((controls, losses))
}

fn exact_f32(value: Ratio<i64>) -> Result<f32, AudioPrepareError> {
    let value = *value.numer() as f64 / *value.denom() as f64;
    if !value.is_finite() || value < f64::from(f32::MIN) || value > f64::from(f32::MAX) {
        return Err(AudioPrepareError::InstrumentContract(
            "audition transfer is outside finite DSP bounds".to_owned(),
        ));
    }
    Ok(value as f32)
}

fn prepare_control_batches(
    plan: &mut RenderPlan,
    node: crate::spec::NodeId,
    declaration: &crate::InstrumentContract,
    schedule: &Schedule<PerformedTime, Gesture>,
) -> Result<Vec<PreparedControlBatch>, AudioPrepareError> {
    use std::collections::BTreeMap;

    let mut mappings = Vec::with_capacity(declaration.mappings().len());
    let mut defaults = Vec::new();
    for mapping in declaration.mappings() {
        if mapping.node() != "voice" {
            return Err(AudioPrepareError::InstrumentContract(format!(
                "private target `{}` is not supplied by the native instrument instance",
                mapping.node()
            )));
        }
        let Some(target) = plan
            .resolve_parameter(node, mapping.parameter())
            .map_err(|error| AudioPrepareError::InstrumentContract(error.to_string()))?
        else {
            continue;
        };
        let requirement = declaration
            .controls()
            .iter()
            .find(|control| {
                control.kind() == mapping.kind()
                    && control.namespace() == mapping.namespace()
                    && control.name() == mapping.name()
            })
            .ok_or_else(|| {
                AudioPrepareError::InstrumentContract(format!(
                    "mapping for `{}::{}` has no matching exposed control",
                    mapping.namespace(),
                    mapping.name()
                ))
            })?;
        let default = mapped_control_value(
            requirement.default_ratio(),
            requirement.default_symbol(),
            mapping.transfer(),
            mapping.connection_values(),
        )?;
        if !plan.parameter_is_written(target) {
            defaults.push(PreparedParameterEvent {
                target,
                value: default,
                ramp_frames: 0,
            });
        }
        mappings.push(ResolvedControlMapping {
            kind: mapping.kind().to_owned(),
            namespace: mapping.namespace().to_owned(),
            name: mapping.name().to_owned(),
            continuous: requirement.update_rate() == "Continuous",
            target,
            transfer: mapping.transfer(),
            connection_values: mapping.connection_values(),
        });
    }
    plan.apply_parameter_events(&defaults);

    let mut points: BTreeMap<PreparedParameterId, Vec<(u64, f32, bool)>> = BTreeMap::new();
    for (frame, batch) in schedule.batches() {
        for message in batch.messages() {
            let crate::schedule::EventMessage::Begin(_, gesture) = message else {
                continue;
            };
            for control in gesture.controls() {
                let accepted = declaration.controls().iter().any(|requirement| {
                    requirement.kind() == control.kind()
                        && requirement.namespace() == control.namespace()
                        && requirement.name() == control.name()
                });
                if !accepted {
                    return Err(AudioPrepareError::InstrumentContract(format!(
                        "instrument `{}` does not accept control `{}::{}` at source kind `{}`",
                        declaration.declaration_id(),
                        control.namespace(),
                        control.name(),
                        control.kind()
                    )));
                }
                for mapping in mappings.iter().filter(|mapping| {
                    mapping.kind == control.kind()
                        && mapping.namespace == control.namespace()
                        && mapping.name == control.name()
                }) {
                    points.entry(mapping.target).or_default().push((
                        frame,
                        mapped_control_value(
                            control.ratio(),
                            control.symbol(),
                            mapping.transfer,
                            mapping.connection_values,
                        )?,
                        mapping.continuous,
                    ));
                }
            }
        }
    }

    let mut batches: BTreeMap<u64, Vec<PreparedParameterEvent>> = BTreeMap::new();
    for (target, values) in points {
        for (index, &(frame, value, continuous)) in values.iter().enumerate() {
            batches.entry(frame).or_default().push(PreparedParameterEvent {
                target,
                value,
                ramp_frames: 0,
            });
            if continuous
                && let Some(&(next_frame, next_value, _)) = values.get(index.saturating_add(1))
                && next_frame > frame
            {
                batches.entry(frame).or_default().push(PreparedParameterEvent {
                    target,
                    value: next_value,
                    ramp_frames: next_frame.saturating_sub(frame),
                });
            }
        }
    }
    Ok(batches
        .into_iter()
        .map(|(frame, events)| PreparedControlBatch {
            frame,
            events: events.into_boxed_slice(),
        })
        .collect())
}

fn mapped_value(value: Ratio<i64>, transfer: Option<(Ratio<i64>, Ratio<i64>, bool)>) -> Result<f32, AudioPrepareError> {
    let exact = match transfer {
        Some((minimum, maximum, inverse)) => {
            let along = if inverse { Ratio::ONE - value } else { value };
            minimum + (maximum - minimum) * along
        }
        None => value,
    };
    let value = *exact.numer() as f64 / *exact.denom() as f64;
    if !value.is_finite() || value < f64::from(f32::MIN) || value > f64::from(f32::MAX) {
        return Err(AudioPrepareError::InstrumentContract(format!(
            "mapped exact value {}/{} is not a finite DSP parameter",
            exact.numer(),
            exact.denom()
        )));
    }
    Ok(value as f32)
}

fn mapped_control_value(
    ratio: Option<Ratio<i64>>,
    symbol: Option<&str>,
    transfer: Option<(Ratio<i64>, Ratio<i64>, bool)>,
    connection_values: Option<[Ratio<i64>; 3]>,
) -> Result<f32, AudioPrepareError> {
    if let Some(value) = ratio {
        return mapped_value(value, transfer);
    }
    let values = connection_values.ok_or_else(|| {
        AudioPrepareError::InstrumentContract("a symbolic control has no declared transfer".to_owned())
    })?;
    let exact = match symbol {
        Some("Detached") => values[0],
        Some("Ordinary") => values[1],
        Some("Legato") => values[2],
        _ => {
            return Err(AudioPrepareError::InstrumentContract(
                "a phrase-connection mapping received an unsupported source constructor".to_owned(),
            ));
        }
    };
    mapped_value(exact, None)
}

fn check_resources(actual: crate::primitive::AudioResources, limits: AudioLimits) -> Result<(), AudioPrepareError> {
    let checks = [
        (
            "primitive count",
            actual.primitives as u64,
            limits.max_primitives as u64,
        ),
        (
            "retained state bytes",
            actual.state_bytes as u64,
            limits.max_state_bytes as u64,
        ),
        ("one-frame work", actual.step_work, limits.max_step_work),
    ];
    for (resource, actual, limit) in checks {
        if actual > limit {
            return Err(AudioPrepareError::ResourceLimit {
                resource,
                actual,
                limit,
            });
        }
    }
    Ok(())
}

fn schedule_gestures(
    gestures: &GesturePlan,
    options: AudioOptions,
) -> Result<(Schedule<PerformedTime, Gesture>, Vec<Schedule<PerformedTime, Gesture>>), ScheduleError> {
    let lane_schedules = gestures
        .lanes()
        .iter()
        .map(|lane| schedule_lane(lane, options))
        .collect::<Result<Vec<_>, _>>()?;
    let mut schedules = lane_schedules.iter();
    let mut combined = match schedules.next() {
        Some(schedule) => schedule.clone(),
        None => {
            let track = empty(Duration::ZERO);
            schedule(
                options.format,
                options.schedule,
                &TimeMap::new(1, [(Position::ZERO, Position::<PhysicalTime>::ZERO)]),
                &track,
            )?
        }
    };
    for next in schedules {
        combined = merge_schedules(options.schedule, &combined, next)?;
    }
    Ok((combined, lane_schedules))
}

fn schedule_lane(
    lane: &musa_score::GestureLane,
    options: AudioOptions,
) -> Result<Schedule<PerformedTime, Gesture>, ScheduleError> {
    let track = lane.track();
    let assignments = track
        .occurrences()
        .iter()
        .flat_map(|occurrence| [occurrence.span().start(), occurrence.span().end()])
        .chain([track.duration().reach()])
        .map(|position| (position, lane.physical(position)));
    schedule(options.format, options.schedule, &TimeMap::new(1, assignments), track)
}
