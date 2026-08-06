//! Semantic core: from parsed source to immutable musical snapshots.
//!
//! Owns: name resolution, unit checking, semantic diagnostics, the high-level
//! compositional model (motifs, transformations, references), motif expansion,
//! exact rational musical time, score normalization, performance lowering,
//! provenance (`Origin`/`ExpansionStep`), and the public immutable snapshots
//! (`ScoreSnapshot`, `PerformancePlan`, `StudioSpec`) — design roadmap §15.3.
//!
//! Must never expose: compiler pass internals (resolution tables, expansion
//! machinery); transient `slotmap` keys as serialized identities. Must never
//! contain: notation planning, DSP, MIDI numbers in the score, or floating-
//! point musical time.
//!
//! Facade (roadmap §15.3): [`compile`] (prompt 05) and `lower_performance`
//! (prompt 10).
//!
//! Invariants: a `ScoreSnapshot` is finite, sorted by onset, immutable, and
//! keeps written pitch spelling (D♯ ≠ E♭); every expanded event carries the
//! full provenance path explaining why it exists; expansion always terminates
//! (the language has no recursion).

mod compile;
mod elaborate;
mod lower;
mod origin;
mod pitch;
mod score;
mod time;

pub use crate::compile::{Compilation, CompileOptions, Diagnostic, Elaboration, Severity, SourceDocument, compile};
#[doc(hidden)]
pub use crate::elaborate::kernel_normal_form;
pub use crate::origin::{DeclarationId, ExpansionStep, Interval, Origin, SourceSpan};
pub use crate::pitch::{Accidental, Letter, PitchClass, WrittenPitch};
pub use crate::score::{
    AnnotationStore, Clef, EventId, KeyMap, MeterMap, Mode, NotatedDuration, Part, PartId, PartMap, ScoreEvent,
    ScoreEventKind, ScoreSnapshot, TempoMap, Voice, VoiceId,
};
pub use crate::time::{MusicalDuration, MusicalTime};
