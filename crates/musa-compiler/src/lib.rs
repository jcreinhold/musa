//! Semantic core: from parsed source to immutable musical snapshots.
//!
//! Pipeline (course correction §26, prompt 12): CST → expansion-aware
//! elaboration → temporal kernel (`musa-kernel`) → `ScoreSnapshot` adapter.
//! The direct lowerer of prompts 05–06 remains as the differential
//! regression oracle; the kernel — not the surface grammar — defines the
//! ontology.
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
mod harmony;
mod imports;
mod lower;
mod origin;
mod performance;
mod pitch;
mod profile;
mod score;
mod studio;
mod time;

pub use crate::compile::{Compilation, CompileOptions, Diagnostic, Elaboration, Severity, SourceDocument, compile};
#[doc(hidden)]
pub use crate::elaborate::kernel_normal_form;
pub use crate::harmony::{ChordQuality, ChordSymbol, Seventh};
pub use crate::imports::{ImportSources, resolve_import};
pub use crate::origin::{DeclarationId, ExpansionStep, Interval, Origin, SourceSpan};
pub use crate::performance::{
    IntegratedTempoMap, ParameterId, PerformanceError, PerformanceEvent, PerformanceLane, PerformanceOptions,
    PerformancePlan, PerformedNote, TempoSegment, Tuning, VoiceInstanceId, lower_performance,
};
pub use crate::pitch::{Accidental, Letter, PitchClass, WrittenPitch};
pub use crate::profile::{ArticulationRealization, PerformanceProfile, ProfileSet};
pub use crate::score::{
    AnnotationStore, ArticulationMark, ArticulationMarking, Clef, DynamicMark, DynamicMarking, EventId, HairpinSpan,
    HarmonyMark, KeyMap, MeterMap, Mode, MotifDeclaration, NotatedDuration, Part, PartId, PartMap, PhraseSpan,
    ScoreEvent, ScoreEventKind, ScoreSnapshot, SectionMark, SlurSpan, TempoChange, TempoMap, TupletSpan, Voice,
    VoiceId,
};
pub use crate::studio::{
    Assignment, Modulation, NodeIndex, ParamSpec, Patch, Processor, Route, Send, StudioNode, StudioSpec, Unit, Value,
};
pub use crate::time::{MusicalDuration, MusicalTime};
