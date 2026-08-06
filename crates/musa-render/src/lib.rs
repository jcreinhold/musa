//! Notation planning and interchange export.
//!
//! Owns: the backend-neutral `NotationPlan` (measures, beaming, tie
//! decomposition, voice allocation), MEI generation with `EventId`-carrying
//! `xml:id`s, `MusicXML` generation, the typed `LilyPond` document model and its
//! deterministic pretty-printer, MIDI-file export, deterministic text/XML
//! serialization, and source maps for rendered artifacts — design roadmap
//! §15.4.
//!
//! Must never expose: `LilyPond`/`MusicXML`/MEI document internals (they stay
//! behind the `NotationTarget` facade); must never contain: composition
//! semantics (that is `musa-compiler`'s), backend assumptions leaking into
//! the score model, or raw backend escape hatches (roadmap §7.2).
//!
//! Intended facade (roadmap §15.4), to be implemented by prompts 07–09, 18,
//! and 22:
//!
//! ```text
//! pub enum NotationTarget { Mei, MusicXml, LilyPond }
//! pub fn render_notation(score: &ScoreSnapshot, target: NotationTarget,
//!     options: &NotationOptions) -> Result<RenderedNotation, RenderError>;
//! pub fn render_midi(performance: &PerformancePlan, options: &MidiOptions)
//!     -> Result<Vec<u8>, RenderError>;
//! ```
//!
//! Invariants: output is deterministic (byte-identical for identical input);
//! every notated item traces back to a `ScoreEvent` id; unsupported notation
//! produces an explicit diagnostic, never a silent drop.
