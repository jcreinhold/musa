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

mod autosave;
mod command;
mod contents;
mod diagnostic;
mod edit;
mod error;
mod export;
mod facts;
mod imports;
mod midi;
mod playback;
mod position;
mod project;
mod realization;
mod session;
mod snapshot;
mod studio;
mod template;
mod utf16;

pub use crate::command::{DocumentId, ProjectCommand, ProjectUpdate, Revision, TextEdit, TransportRequest, Validity};
pub use crate::contents::{ContentsFacts, EntryFacts};
pub use crate::diagnostic::{Diagnostic, Fix, FixEdit, Label, Severity, Span, codes, explain};
pub use crate::edit::{CandidateEdit, EditCommand, EditImpact, GeneratedEditMode, InsertAt, NoteSpec};
pub use crate::error::ProjectError;
pub use crate::export::{ExportArtifact, ExportRequest, KernelReport, check_kernel};
pub use crate::facts::{
    DecisionFact, EventFacts, EventKind, Fraction, HeaderFact, NameFact, NameKind, OccurrenceFacts, OriginFacts,
    OutlineFacts, OutlineKind, PartFacts, ScoreFacts, VoiceFacts,
};
pub use crate::midi::MidiEntry;
pub use crate::position::Position;
pub use crate::project::{Project, ProjectMeta};
pub use crate::session::ProjectSession;
pub use crate::snapshot::{PlaybackState, ProjectSnapshot};
pub use crate::studio::{
    AssignmentFacts, ContainerFacts, ContainerKind, ParamFacts, RouteFacts, SendFacts, StageFacts, StudioEdit,
    StudioFacts,
};
pub use crate::template::Template;
pub use crate::utf16::Utf16Offsets;
pub use musa_compiler::{ChoicePath, ChoiceStep, Decision, DecisionRecord, DocumentKind, Realization};
pub use musa_language::{HeaderField, KeywordDoc, keyword_doc};
pub use musa_render::MidiMode;
