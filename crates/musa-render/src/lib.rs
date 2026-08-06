//! Backend-neutral notation planning (roadmap §12.1) plus MEI, `LilyPond`,
//! `MusicXML`, and MIDI export.
//!
//! Owns: the `NotationPlan` derived from a `ScoreSnapshot` (measures, voice
//! lanes, beaming, tie decomposition, key/time signatures), and every
//! text/file backend that consumes it. Must never contain: musical semantics
//! (it reads `ScoreSnapshot`, never reinterprets it), DSP, or respelling —
//! written pitch spelling passes through verbatim (§6.3).
//!
//! Facade (roadmap §15.4): [`plan_notation`] (prompt 07); `render_mei`
//! (prompt 08), `render_lilypond` (prompt 09), MIDI/MusicXML later.
//!
//! Invariants: the plan is semantic, not typographic — no line breaks or
//! spacing; every notated item carries the `EventId` it came from, so tie
//! pieces of one event stay linked; unsupported constructs are explicit
//! [`NotationError`]s, never raw backend escapes (§7.2).

mod error;
mod ly;
mod mei;
mod plan;
mod render;

pub use crate::error::{NotationError, RenderError};
pub use crate::plan::{
    BeamGroup, KeySignature, MeasurePlan, NotatedItem, NotatedKind, NotationOptions, NotationPlan, StaffPlan,
    VoiceLane, plan_notation,
};
pub use crate::render::{NotationTarget, RenderedNotation, render_notation};
