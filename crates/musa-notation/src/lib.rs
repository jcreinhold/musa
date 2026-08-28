//! Backend-neutral notation planning (roadmap §12.1) plus MEI, `LilyPond`,
//! `MusicXML`, and MIDI export.
//!
//! Owns: the `NotationPlan` derived from a `ScoreSnapshot` (measures, voice
//! lanes, beaming, tie decomposition, key/time signatures), and every
//! text/file backend that consumes it. Must never contain: musical semantics
//! (it reads `ScoreSnapshot`, never reinterprets it), DSP, or respelling —
//! written pitch spelling passes through verbatim (§6.3).
//!
//! Facade: [`plan_notation`], `render_mei`, `render_lilypond`,
//! `render_musicxml`, `render_midi`.
//!
//! Invariants: the plan is semantic, not typographic — no line breaks or
//! spacing; every notated item carries the `EventId` it came from, so tie
//! pieces of one event stay linked; unsupported constructs are explicit
//! [`NotationError`]s, never raw backend escapes (§7.2).

mod error;
mod ly;
mod mei;
mod midi;
mod musicxml;
mod plan;
mod render;

pub use crate::error::{NotationError, RenderError};
pub use crate::midi::{
    MidiLoss, MidiMeta, MidiMode, MidiOptions, MidiOrigin, MidiPart, MidiSchedule, MidiTrack, RenderedMidi,
    ScheduledMessage, ScheduledMeta, midi_schedule, render_midi, render_midi_with_origins, write_schedule,
};
pub use crate::plan::{
    ARTICULATION_PLACEMENT, BeamGroup, DYNAMIC_PLACEMENT, FrontMatter, HairpinMark, HairpinRange, KeySignature,
    MeasurePlan, NotatedItem, NotatedKind, NotationOptions, NotationPlan, PhraseMark, PhraseRange, Placement,
    PositionedMark, SLUR_PLACEMENT, SlurRange, StaffPlan, TempoText, TupletMark, VoiceLane, plan_notation,
};
pub use crate::render::{NotationTarget, RenderedNotation, render_notation};
