//! The musical values a compilation produces, and the vocabulary they are
//! written in.
//!
//! Everything here is a *result* or a *word for one*: written pitch, chords,
//! scales, exact musical time, marks, the score snapshot, the performance
//! plan, what an analysis saw, and what a diagnostic says. Nothing here
//! computes one. Resolution, expansion, elaboration, and lowering are
//! `musa-compiler`, one layer up, and no module in this crate names them.
//!
//! # Why this is a crate and not a directory
//!
//! The layering was already true and only convention held it. A closure over
//! every `crate::` reference in `musa-compiler` found these 22 modules
//! referring to no pass module at all — once `SourceMap` moved to [`origin`],
//! beside the [`origin::SourceSpan`] it translates. The crate boundary is what
//! makes that a fact the compiler checks rather than a property someone has to
//! keep re-establishing.
//!
//! It also matches what consumers ask for. `musa-notation` imports 34 names from
//! the compiler and every one of them is here; it now rebuilds against this
//! crate rather than against the whole pipeline.
//!
//! # The surface is wide on purpose
//!
//! This is a vocabulary, not an algorithm behind a facade. A domain has as
//! many nouns as it has, and hiding `Clef` behind an accessor would buy
//! nothing: `docs/plan/roadmap.md` §2's separations — written pitch is not a
//! MIDI number, notated duration is not performed duration — are enforced by
//! *which type* a value has, which only works if the types are namable.
//!
//! What stays narrow is the other direction: nothing here is a way in. There
//! is no constructor that builds a score from text, because building one is
//! the pipeline's job and this crate cannot see it.

pub mod analysis;
pub mod assert;
pub mod bars;
pub mod chord;
pub mod context;
pub mod derivation;
pub mod diagnose;
pub mod groove;
pub mod harmony;
pub mod machine;
pub mod marks;
pub mod origin;
pub mod pc12;
pub mod performance;
pub mod pitch;
pub mod profile;
pub mod realize;
pub mod roman;
pub mod scale;
pub mod scope;
pub mod score;
pub mod time;

pub use crate::analysis::{
    AnalysisError, AnalysisFinding, AnalysisKind, AnalysisProfile, AnalysisReport, AnalysisRequest, AnalysisScope,
    Approach, Cadence, ChordName, Evidence, Fit, Ground, NoteRef, Observation, RuleName, Segmentation, Standing,
    Strength, analyze, rule_names,
};
pub use crate::assert::{ClaimDoc, assertion_claims, realization_policies};
pub use crate::bars::{BarBeat, BarLines, Measure};
pub use crate::chord::chord_types;
pub use crate::context::ContextTrack;
pub use crate::derivation::Derivation;
pub use crate::diagnose::{Cause, Code, Diagnostic, Fix, FixEdit, Label, Severity};
pub use crate::groove::Groove;
pub use crate::harmony::{ChordQuality, ChordSymbol, Seventh};
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
pub use crate::scale::scale_collections;
pub use crate::scope::{ContextKind, Scope};
pub use crate::score::{
    AnnotationStore, ArticulationMarking, Clef, DynamicMark, DynamicMarking, EventId, FreeDuration, FrontMatter,
    HairpinSpan, HarmonyMark, Key, MarkSpan, Meter, Metronome, Mode, MotifDeclaration, NotatedDuration, OpenKind,
    OpenRegion, Part, PartId, PartMap, PhraseSpan, PointMark, Ramp, ScoreEvent, ScoreEventKind, ScoreSnapshot,
    SectionMark, SlurSpan, TempoMarking, TupletSpan, Voice, VoiceId,
};
pub use crate::time::{MusicalDuration, MusicalTime};

/// How a bar divides into the groups a player hears.
///
/// A fact about a [`score::Meter`], so it is named here, where meters are.
/// It is *implemented* one crate down, beside the formatter that spaces a bar
/// by the same boundaries the engraver beams by — two readers of one table,
/// and a second copy of it would be a second convention. Which crate holds the
/// table is not something a caller should have to know.
pub use musa_language::beat_groups;
