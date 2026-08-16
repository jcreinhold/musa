//! Semantic core: from parsed source to immutable musical snapshots.
//!
//! Pipeline (docs/plan/code-map/stage-pipeline.md): CST → expansion-aware
//! elaboration → temporal kernel (`musa-kernel`) → `ScoreSnapshot` adapter.
//! One semantic path: the kernel — not the
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
//! Facade (roadmap §15.3): [`compile`] and `lower_performance`.
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
mod data;
mod derivation;
mod diagnose;
mod docs;
mod elaborate;
mod expand;
mod factext;
mod groove;
mod harmony;
mod imports;
mod infer;
mod kernel_text;
mod lint;
mod machine;
mod marks;
mod module;
mod origin;
mod package;
mod pc12;
mod performance;
mod pitch;
/// The compiler's own `data` declarations, reachable only from [`registry`].
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "prompt 141e builds the core's registry; prompt 142 is the one cutover that calls it"
    )
)]
mod prelude;
mod profile;
mod project;
mod realize;
mod reference;
/// The compiler's own operations as `musa-core` registrations.
///
/// Nothing reaches it yet, and prompt 141e's Design says why: the registry is
/// built and proved one prompt before the cutover that uses it, so that a wrong
/// signature is distinguishable from a wrong migration. The expectation rather
/// than an `allow` is the point — prompt 142 wiring the elaborator makes it
/// unfulfilled, and the compiler says so.
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "prompt 141e builds the core's registry; prompt 142 is the one cutover that calls it"
    )
)]
mod registry;
mod resolve;
mod roman;
mod scale;
mod scope;
mod score;
mod studio;
mod syntax;
mod template;
mod time;

pub use crate::analysis::{
    AnalysisError, AnalysisFinding, AnalysisKind, AnalysisProfile, AnalysisReport, AnalysisRequest, AnalysisScope,
    Approach, Cadence, ChordName, Evidence, Fit, Ground, NoteRef, Observation, RuleName, Segmentation, Standing,
    Strength, analyze, rule_names,
};
pub use crate::assert::{ClaimDoc, assertion_claims, realization_policies};
pub use crate::bars::{BarBeat, BarLines, Measure};
pub use crate::chord::chord_types;
pub use crate::compile::{Compilation, CompileOptions, DocumentKind, SourceDocument, compile, format_document};
pub use crate::context::ContextTrack;
pub use crate::derivation::Derivation;
pub use crate::diagnose::{Cause, Code, Diagnostic, Fix, FixEdit, Label, Severity};
pub use crate::docs::{ItemDoc, ItemSource, ParameterDoc, TypeNote};
#[doc(hidden)]
pub use crate::elaborate::kernel_normal_form;
pub use crate::expand::{AdapterEdit, AdapterEditError, AdapterPrintError, adapter_edits, adapter_print};
pub use crate::groove::Groove;
pub use crate::harmony::{ChordQuality, ChordSymbol, Seventh};
pub use crate::imports::{
    ImportSources, STANDARD_LIBRARY_LANGUAGE_VERSION, resolve_import, standard_library_module,
    standard_library_modules, standard_library_source,
};
#[doc(hidden)]
pub use crate::kernel_text::{
    KernelCheck, check_kernel_text, kernel_normalized_text, kernel_text, kernel_text_meaning,
};
/// Kernel text as an editor sees it, re-exported so a language server can
/// colour and outline a kernel document without a second copy of the grammar
/// and without depending on `musa-kernel` itself. Renamed on the way through
/// because a shell holds this beside `musa-language`'s classification of
/// surface text, and two things called `TokenClass` in one file is one too
/// many.
pub use musa_kernel::{
    TokenClass as KernelTokenClass, bindings as kernel_bindings, classify as kernel_classify,
    keyword_doc as kernel_keyword_doc,
};

pub use crate::machine::{MACHINE_SPEC_VERSION, MachineSpec, SpecForm, SpecNode};
pub use crate::marks::{Anchor, Argument, Mark, MarkArgument, MarkDef, Slot, VOCABULARY, lookup_mark};
pub use crate::origin::{ChoicePath, ChoiceStep, DeclarationId, ExpansionStep, Interval, Origin, SourceSpan};
pub use crate::performance::{
    IntegratedTempoMap, KeyChange, MeterChange, ParameterId, PerformanceError, PerformanceEvent, PerformanceLane,
    PerformanceOptions, PerformancePlan, PerformedNote, TempoSegment, Tuning, VoiceInstanceId, lower_performance,
};
pub use crate::pitch::{Accidental, Letter, PitchClass, WrittenPitch};
pub use crate::profile::{ArticulationRealization, GracePolicy, PerformanceProfile, ProfileSet, StealFrom};
pub use crate::realize::{Decision, DecisionRecord, Realization};
pub use crate::reference::standard_library_reference;
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
