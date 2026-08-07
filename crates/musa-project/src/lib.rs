//! The application's deepest module: one session, one canonical source.
//!
//! Owns: project loading and saving, source documents, revisions, commands,
//! undo/redo, compiler orchestration, rendered-score snapshots, playback-plan
//! preparation and installation, autosave, and asset references — design
//! roadmap §15.7.
//!
//! Must never expose: parser, compiler, renderer, DSP, or CPAL types; the
//! frontend and CLI see only session-level DTOs. Must never contain: a
//! second editable AST or a mutable expanded cache — the `.musa` source is
//! the single persistent truth (roadmap §11), and score/studio interfaces
//! are structured editors of that source.
//!
//! Intended facade (roadmap §15.7), to be implemented by prompts 19 and 25:
//!
//! ```text
//! pub struct ProjectSession { /* hidden */ }
//! impl ProjectSession {
//!     pub fn open(path: impl AsRef<Path>) -> Result<Self, ProjectError>;
//!     pub fn snapshot(&self) -> ProjectSnapshot;
//!     pub fn apply(&mut self, command: ProjectCommand)
//!         -> Result<ProjectUpdate, ProjectError>;
//!     pub fn export(&self, request: ExportRequest)
//!         -> Result<ExportArtifact, ProjectError>;
//! }
//! ```
//!
//! Invariants: one user command may trigger parsing, compilation, score
//! rendering, and playback-plan rebuilding, and callers cannot tell; while
//! the source is temporarily invalid, the last valid score and playback plan
//! stay live and are clearly flagged as such (roadmap §14.7).
