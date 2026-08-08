//! Semantic core: from parsed source to immutable musical snapshots.
//!
//! Pipeline (course correction §26, prompt 12): CST → expansion-aware
//! elaboration → temporal kernel (`musa-kernel`) → `ScoreSnapshot` adapter.
//! One semantic path: the direct lowerer of prompts 05–06 was the migration's
//! regression oracle and was deleted at prompt 41. The kernel — not the
//! surface grammar — defines the ontology.
//!
//! Owns: name resolution, unit checking, semantic diagnostics, the high-level
//! compositional model (motifs, transformations, references), motif expansion,
//! exact rational musical time, score normalization, performance resolver,
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

/// Measurement seams for the benchmark suite. Not an interface: see the
/// module docs (roadmap §17.7).
#[doc(hidden)]
pub mod bench;
mod compile;
mod diagnose;
mod elaborate;
mod factext;
mod harmony;
mod imports;
mod kernel_text;
mod origin;
mod performance;
mod pitch;
mod profile;
mod project;
mod resolve;
mod score;
mod studio;
mod time;

pub use crate::compile::{Compilation, CompileOptions, SourceDocument, compile};
pub use crate::diagnose::{Code, Diagnostic, Fix, FixEdit, Label, Severity};
#[doc(hidden)]
pub use crate::elaborate::kernel_normal_form;
pub use crate::harmony::{ChordQuality, ChordSymbol, Seventh};
pub use crate::imports::{ImportSources, resolve_import};
#[doc(hidden)]
pub use crate::kernel_text::{
    KernelCheck, check_kernel_text, kernel_normalized_text, kernel_text, kernel_text_meaning,
};
pub use crate::origin::{DeclarationId, ExpansionStep, Interval, Origin, SourceSpan};
pub use crate::performance::{
    IntegratedTempoMap, ParameterId, PerformanceError, PerformanceEvent, PerformanceLane, PerformanceOptions,
    PerformancePlan, PerformedNote, TempoSegment, Tuning, VoiceInstanceId, lower_performance,
};
pub use crate::pitch::{Accidental, Letter, PitchClass, WrittenPitch};
pub use crate::profile::{ArticulationRealization, PerformanceProfile, ProfileSet};
pub use crate::score::{
    AnnotationStore, ArticulationMark, ArticulationMarking, Clef, DynamicMark, DynamicMarking, EventId, FrontMatter,
    HairpinSpan, HarmonyMark, KeyMap, MeterMap, Mode, MotifDeclaration, NotatedDuration, Part, PartId, PartMap,
    PhraseSpan, ScoreEvent, ScoreEventKind, ScoreSnapshot, SectionMark, SlurSpan, TempoChange, TempoMap, TupletSpan,
    Voice, VoiceId,
};
pub use crate::studio::{
    Assignment, Modulation, NodeIndex, ParamSpec, Patch, Processor, Route, Send, StudioNode, StudioSpec, Unit, Value,
};
pub use crate::time::{MusicalDuration, MusicalTime};
/// The kernel's semantic digest, re-exported so a consumer can hold a
/// compilation's identity without depending on the kernel directly.
pub use musa_kernel::SemanticHash;
