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

mod analysis;
mod assert;
mod bars;
/// Measurement seams for the benchmark suite. Not an interface: see the
/// module docs (roadmap §17.7).
#[doc(hidden)]
pub mod bench;
mod chord;
mod compile;
mod context;
mod core;
mod core_budget;
mod diagnose;
mod elaborate;
mod factext;
mod groove;
mod harmony;
mod imports;
mod kernel_text;
mod lint;
mod marks;
mod module;
mod origin;
mod package;
mod pc12;
mod performance;
mod pitch;
mod profile;
mod project;
mod realize;
mod resolve;
mod roman;
mod scale;
mod scope;
mod score;
mod studio;
mod template;
mod time;

pub use crate::analysis::{
    AnalysisError, AnalysisFinding, AnalysisKind, AnalysisProfile, AnalysisReport, AnalysisRequest, AnalysisScope,
    Approach, Cadence, ChordName, Evidence, Fit, Ground, NoteRef, Observation, RuleName, Segmentation, Standing,
    Strength, analyze, rule_names,
};
pub use crate::bars::{BarBeat, BarLines, Measure};
pub use crate::chord::chord_types;
pub use crate::compile::{Compilation, CompileOptions, DocumentKind, SourceDocument, compile};
pub use crate::context::ContextTrack;
pub use crate::diagnose::{Code, Diagnostic, Fix, FixEdit, Label, Severity};
#[doc(hidden)]
pub use crate::elaborate::kernel_normal_form;
pub use crate::groove::Groove;
pub use crate::harmony::{ChordQuality, ChordSymbol, Seventh};
pub use crate::imports::{
    ImportSources, STANDARD_LIBRARY_LANGUAGE_VERSION, resolve_import, standard_library_modules,
    standard_library_reference, standard_library_source,
};
#[doc(hidden)]
pub use crate::kernel_text::{
    KernelCheck, check_kernel_text, kernel_normalized_text, kernel_text, kernel_text_meaning,
};
pub use crate::marks::{Anchor, Argument, Mark, MarkArgument, MarkDef, Slot, VOCABULARY, lookup_mark};
pub use crate::origin::{ChoicePath, ChoiceStep, DeclarationId, ExpansionStep, Interval, Origin, SourceSpan};
pub use crate::performance::{
    IntegratedTempoMap, KeyChange, MeterChange, ParameterId, PerformanceError, PerformanceEvent, PerformanceLane,
    PerformanceOptions, PerformancePlan, PerformedNote, TempoSegment, Tuning, VoiceInstanceId, lower_performance,
};
pub use crate::pitch::{Accidental, Letter, PitchClass, WrittenPitch};
pub use crate::profile::{ArticulationRealization, GracePolicy, PerformanceProfile, ProfileSet, StealFrom};
pub use crate::realize::{Decision, DecisionRecord, Realization};
pub use crate::resolve::{NameKind, NameReference, SourceLocation};
pub use crate::scale::scale_collections;
pub use crate::scope::{ContextKind, Scope};
pub use crate::score::{
    AnnotationStore, ArticulationMarking, Clef, DynamicMark, DynamicMarking, EventId, FreeDuration, FrontMatter,
    HairpinSpan, HarmonyMark, Key, MarkSpan, Meter, Metronome, Mode, MotifDeclaration, NotatedDuration, OpenKind,
    OpenRegion, Part, PartId, PartMap, PhraseSpan, PointMark, Ramp, ScoreEvent, ScoreEventKind, ScoreSnapshot,
    SectionMark, SlurSpan, TempoMarking, TupletSpan, Voice, VoiceId,
};
pub use crate::studio::{
    Assignment, Modulation, NodeIndex, ParamSpec, Patch, Processor, Route, Send, StudioNode, StudioSpec, Unit, Value,
};
pub use crate::time::{MusicalDuration, MusicalTime};
/// The kernel's semantic digest, re-exported so a consumer can hold a
/// compilation's identity without depending on the kernel directly.
pub use musa_kernel::SemanticHash;
// How a bar divides into the groups a player hears. It lives at the bottom of
// the graph because both readers of that fact are above it: the engraver beams
// a group together and the formatter spaces one. Re-exported here so
// `musa-render` reaches it without an edge of its own to the language.
pub use musa_language::beat_groups;
