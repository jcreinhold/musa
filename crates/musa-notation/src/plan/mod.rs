//! The `NotationPlan` (roadmap §12.1): measures, voice lanes, beaming, and
//! tie decomposition derived from a `ScoreSnapshot`. Semantic, not
//! typographic: no line breaks, spacing, or engraving decisions.
//!
//! Duration decomposition and measure splitting use exact rational
//! arithmetic; the workspace arithmetic lint is allowed at module scope (see
//! `musa-compiler/src/time.rs` for the totality argument).
//!
//! # By file
//!
//! - [`score`] — the plan tree: the plan, its front matter, staves, measures,
//!   and voice lanes.
//! - [`marks`] — everything a plan hangs at a position or over a range.
//! - [`items`] — one notated item: what it is, what it carries, and the edges
//!   that join it to its neighbours.
//! - [`build`] — deriving a plan from a score snapshot.
//! - [`fold`] — the repeat structure the derivation reads time through.
//! - [`collect`] — the marks gathered from the snapshot before planning.
//! - [`staff`] — planning one staff's measures.
//! - [`lane`] — planning one voice lane: items, beams, and tie decomposition.
#![allow(clippy::arithmetic_side_effects)]

mod build;
mod collect;
mod fold;
mod items;
mod lane;
mod marks;
mod score;
mod staff;

pub use build::plan_notation;
pub use items::{NotatedItem, NotatedKind};
pub use marks::{
    ARTICULATION_PLACEMENT, BeamGroup, DYNAMIC_PLACEMENT, HairpinMark, HairpinRange, KeySignature, PhraseMark,
    PhraseRange, Placement, PositionedMark, SLUR_PLACEMENT, SlurRange, TempoText, TupletMark,
};
pub use score::{FrontMatter, MeasurePlan, NotationOptions, NotationPlan, StaffPlan, VoiceLane};

// Named by the exporters inside this crate but not part of its public surface.
pub(crate) use marks::{ClefChange, OpenShape, PointMark, SpanMark, VoltaMark};
