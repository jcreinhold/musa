//! The application's deepest module: one session, one canonical source.
//!
//! Owns project loading and saving, source documents, revisions, commands,
//! undo/redo, compiler orchestration, engraved-score snapshots, playback-plan
//! preparation and installation, and exports — design roadmap §15.7.
//!
//! Never exposes parser, compiler, renderer, DSP, or CPAL types: the frontend
//! and CLI see only session-level types ([`ProjectSnapshot`],
//! [`Diagnostic`], [`ExportArtifact`]). Never contains a second editable AST
//! or a mutable expanded cache — the `.musa` source is the single persistent
//! truth (roadmap §11), and score and studio interfaces are structured
//! editors of that source.
//!
//! ```no_run
//! # fn main() -> Result<(), musa_project::ProjectError> {
//! use musa_project::{ExportRequest, ProjectCommand, ProjectSession};
//!
//! let mut session = ProjectSession::open("examples/glass-mountain.musa")?;
//! session.apply(ProjectCommand::SetSource("piece \"x\" {}".into()))?;
//! // The source no longer compiles, so the previous score is still there.
//! assert!(!session.snapshot().compiles());
//! assert!(session.snapshot().mei().is_some());
//! session.undo()?;
//! let wav = session.export(ExportRequest::Wav)?;
//! # let _ = wav;
//! # Ok(())
//! # }
//! ```
//!
//! One user command may parse, compile, engrave, rebuild a DSP plan, and
//! reinstall it on the audio thread; callers cannot tell. While the source is
//! temporarily invalid, the last valid score and playback plan stay live and
//! are flagged as such (roadmap §14.7).

mod analysis;
mod assets;
mod autosave;
mod barlines;
mod command;
mod contents;
mod diagnostic;
mod edit;
mod error;
mod export;
mod facts;
mod format_support;
mod group_edit;
mod imports;
mod library;
mod lock;
mod logging;
mod midi;
mod packages;
mod playback;
mod position;
mod project;
mod realization;
mod review;
mod review_placement;
mod rhythm;
mod session;
mod sf2;
mod sfz;
mod snapshot;
mod studio;
mod template;
mod transcription_pairing;
mod transcription_policy;
mod transcription_proposal;
mod transcription_search;
mod transcription_spell;
#[doc(hidden)]
pub mod transcription_trial;
mod transcription_voice;
mod transcription_written;
mod utf16;
mod vocabulary;

pub use crate::analysis::{AnalysisFacts, EvidenceFacts, FindingFacts, GroundFacts, NoteFacts};
pub use crate::assets::{AssetFact, AssetKind, AssetStatus, asset_inventory, lock_assets};
pub use crate::barlines::{BarlineBlocker, BarlineBlockerReason, BarlineRewrite};
pub use crate::command::{DocumentId, ProjectCommand, ProjectUpdate, Revision, TextEdit, TransportRequest, Validity};
pub use crate::contents::{ContentsFacts, EntryFacts};
pub use crate::diagnostic::{Cause, CauseLabel, Diagnostic, Fix, FixEdit, Label, Severity, Span, codes, explain};
pub use crate::edit::{CandidateEdit, EditCommand, EditImpact, GeneratedEditMode, InsertAt, NoteSpec};
pub use crate::error::ProjectError;
pub use crate::export::{EventsReport, ExportArtifact, ExportRequest, check_events};
pub use crate::facts::{
    DecisionFact, EventFacts, EventKind, Fraction, HeaderFact, ItemFact, MediaFacts, NameFact, NameKind,
    OccurrenceFacts, OriginFacts, OutlineFacts, OutlineKind, ParameterFact, PartFacts, ScoreFacts, SourceLocation,
    StepFact, StepKind, TypeFact, VoiceFacts,
};
pub use crate::format_support::{FormatSupportFacts, format_support, format_supports};
pub use crate::group_edit::{GroupBarEffect, GroupDefinition, GroupEdit, GroupEditPlan, GroupIntent};
pub use crate::library::{LibraryDocument, library_document};
pub use crate::logging::{FILTER_VARIABLE, Logging};
pub use crate::midi::{
    CapturedMidiEvent, MidiCaptureFacts, MidiCaptureState, MidiDeviceFacts, MidiEntry, MidiLossFacts, MidiPairingFact,
    MidiTake, MidiTakeContext,
};
pub use crate::packages::{fetch_packages, verify_packages};
pub use crate::position::Position;
pub use crate::project::{Project, ProjectMeta};
pub use crate::review::{
    AmbiguityKind, ReviewAction, ReviewAmbiguity, ReviewAudition, ReviewChoice, ReviewDecision, ReviewDestination,
    ReviewError, ReviewFacts, ReviewNote, ReviewRequest,
};
pub use crate::review_placement::{PlacedVoice, PlacementError, PlacementPlan, PlacementReport};
pub use crate::rhythm::RhythmTranscriptionReport;
pub use crate::session::ProjectSession;
pub use crate::sf2::{Sf2InstrumentFacts, Sf2Limits};
pub use crate::sfz::{SfzInstrumentFacts, SfzLimits};
pub use crate::snapshot::{PlaybackState, ProjectSnapshot};
pub use crate::studio::{
    AssignmentFacts, ContainerFacts, ContainerKind, MediaSourceFacts, ParamFacts, RouteFacts, SendFacts, StageFacts,
    StudioEdit, StudioFacts,
};
pub use crate::template::Template;
pub use crate::transcription_pairing::GroupShape;
pub use crate::transcription_proposal::{
    NotationProposal, PROPOSAL_VERSION, ProposalError, ProposalGroup, ProposalLoss, ProposalNote, ProposalSource,
    ProposalVoice,
};
pub use crate::transcription_search::{
    Candidate, CostRecord, REPORT_VERSION, Refusal, ReviewRegion, RhythmEvent, SearchOutcome, Take, TakeClock,
};
pub use crate::transcription_voice::{VoiceConstraint, VoiceRefusal};
pub use crate::utf16::Utf16Offsets;
pub use crate::vocabulary::{format_studio_ratio, standard_instrument_contracts, standard_studio_vocabulary};
pub use musa_compiler::DocumentKind;
pub use musa_compiler::standard_library_source;
pub use musa_compiler::{EventsTokenClass, events_bindings, events_classify, events_keyword_doc};
pub use musa_notation::MidiMode;
pub use musa_score::{
    AnalysisKind, AnalysisProfile, AnalysisRequest, AnalysisScope, ChoicePath, ChoiceStep, ClaimDoc, Decision,
    DecisionRecord, Key, Mode, MusicalTime, Realization, Segmentation, assertion_claims, chord_types,
    realization_policies, rule_names, scale_collections,
};
pub use musa_syntax::{BarSpacing, HeaderField, KeywordDoc, keyword_doc};
