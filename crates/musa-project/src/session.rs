//! The session itself: one canonical source, one history, one orchestration.

use std::path::{Path, PathBuf};

use musa_compiler::{CompileOptions, SourceDocument};
use musa_engine::{AudioEngine, EngineConfig, MidiInput, TransportCommand};

use crate::command::{ProjectCommand, ProjectUpdate, Revision, TextEdit, TransportRequest, Validity};
use crate::diagnostic::Diagnostic;
use crate::error::ProjectError;
use crate::export::{ExportArtifact, ExportRequest};
use crate::midi::{EntryBuffer, MidiEntry};
use crate::playback;
use crate::project::ProjectMeta;
use crate::snapshot::{PlaybackState, ProjectSnapshot, ValidArtifacts};
use crate::template::Template;

/// One open `.musa` project.
///
/// The single owner of everything between the source text and the outside
/// world: revisions, undo/redo, compilation, engraving, exports, and the
/// audio engine (roadmap §15.7). A caller issues a [`ProjectCommand`] and
/// reads a [`ProjectSnapshot`]; that one user action may have parsed,
/// compiled, engraved, rebuilt a DSP plan, and reinstalled it on the audio
/// thread is not visible from outside, which is the point.
///
/// Compilation is synchronous per command. Debouncing keystrokes is the
/// caller's concern (roadmap §10.7) — the editor knows when the user has
/// paused and the session does not. Moving compilation to a worker thread
/// is a later change to be made if profiling asks for it, and it would not
/// change this facade.
///
/// The session is single-threaded by construction: `apply` takes `&mut
/// self`, so there is no interleaving to reason about.
pub struct ProjectSession {
    /// Where the project lives, if it has been given a home.
    path: Option<PathBuf>,
    /// Display name for diagnostics and window titles.
    name: String,
    /// The canonical document (roadmap §11).
    source: String,
    /// What is currently on disk, to answer "are there unsaved changes".
    on_disk: Option<String>,
    /// The revision the current source is at.
    revision: Revision,
    /// The next revision number to mint.
    next_revision: u64,
    /// The project this piece is filed under, if a `musa.toml` is above it.
    project: Option<crate::project::ProjectMeta>,
    /// The files the current source imports, transitively, and their text.
    /// Refreshed on every recompile: a library edited on disk is picked up
    /// the next time the piece compiles, which is what "recompile-on-save of
    /// the library file" means with one document open.
    imports: musa_compiler::ImportSources,
    /// The paths behind those imports, for callers that watch them.
    import_paths: Vec<PathBuf>,
    /// Diagnostics for the *current* source.
    diagnostics: Vec<Diagnostic>,
    /// Whether the current source compiles.
    compiles: bool,
    /// The last successful compile's artifacts (roadmap §14.7).
    valid: Option<ValidArtifacts>,
    /// Snapshot-based undo: every state the source has been in, in order.
    ///
    /// Snapshot-based rather than command-inverse because the document *is*
    /// text: a state is a `String`, inverting an edit means storing the
    /// replaced text anyway, and there is no separate model that could drift
    /// out of step with the history. The cost is proportional to edit count
    /// times document size, which for a piece of music is nothing.
    history: Vec<HistoryEntry>,
    /// Index into `history` of the current state.
    cursor: usize,
    /// The audio device, opened on the first [`TransportRequest::Play`] so
    /// that compiling, rendering, and CI need no sound card at all.
    audio: Option<AudioEngine>,
    /// The loop region as requested; mirrored here because the engine's copy
    /// lives on the audio thread.
    loop_region: Option<(u64, u64)>,
    /// Length of the plan currently installed in the engine.
    total_frames: u64,
    /// What the installed playback plan was built from, so an edit that does
    /// not change the music does not interrupt it.
    ///
    /// Two documents, because a plan is built from two: the piece's semantic
    /// identity (docs/kernel/05 N6) and the studio that voices it. Keying on
    /// the score alone would let a changed instrument go unheard until the
    /// next note edit.
    installed: Option<InstalledPlan>,
    /// Work a previous session left behind, until this one keeps or discards
    /// it (roadmap §15.7).
    recovery: Option<String>,
    /// The MIDI keyboard, opened on request so a session that never enters
    /// notes never touches the MIDI host.
    midi: Option<MidiInput>,
    /// Presses waiting to be grouped into chords.
    entry: EntryBuffer,
}

/// What the plan currently in the engine was built from.
///
/// Compared, never read: the session installs a new plan when this differs
/// and leaves the running one alone when it does not. The studio is held by
/// value because it is a small declaration set with no timeline in it, and
/// comparing two of them is cheaper than digesting either.
#[derive(Debug, PartialEq)]
struct InstalledPlan {
    music: musa_compiler::SemanticHash,
    studio: musa_compiler::StudioSpec,
}

/// One state of the document.
struct HistoryEntry {
    source: String,
    revision: Revision,
}

impl ProjectSession {
    /// Open an existing `.musa` file.
    ///
    /// A file that does not compile still opens: the session reports its
    /// diagnostics and has no score yet.
    ///
    /// # Errors
    /// [`ProjectError::Io`] if the file cannot be read as UTF-8.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, ProjectError> {
        let path = path.as_ref();
        let source = std::fs::read_to_string(path).map_err(|error| ProjectError::io(path.display(), error))?;
        let mut session = Self::from_source(source.clone(), path.to_string_lossy().into_owned());
        session.path = Some(path.to_path_buf());
        session.project = crate::project::find(path);
        session.recovery = crate::autosave::take(path, &source);
        session.on_disk = Some(source);
        session.recompile();
        Ok(session)
    }

    /// Create a new project at `path` from `template`, writing it to disk.
    ///
    /// # Errors
    /// [`ProjectError::Io`] if the file cannot be written.
    pub fn create(path: impl AsRef<Path>, template: Template) -> Result<Self, ProjectError> {
        let path = path.as_ref();
        let title = path
            .file_stem()
            .map_or_else(|| "Untitled".to_owned(), |stem| stem.to_string_lossy().into_owned());
        let source = template.source(&title);
        std::fs::write(path, &source).map_err(|error| ProjectError::io(path.display(), error))?;
        let mut session = Self::from_source(source.clone(), path.to_string_lossy().into_owned());
        session.path = Some(path.to_path_buf());
        session.on_disk = Some(source);
        session.recompile();
        Ok(session)
    }

    /// A new piece that has not been given a home yet.
    ///
    /// A new file is playable before it is saved: the "New piece" state of
    /// `docs/interface/05-states.md` §2 shows a real engraved system and puts
    /// the caret in it, and asking for a path first would make the empty
    /// state a file dialog.
    pub fn new_piece(template: Template, title: &str) -> Self {
        Self::from_text(template.source(title), format!("{title}.musa"))
    }

    /// A session over text with no file behind it, for callers that hold the
    /// document themselves (tests, and the desktop app's scratch buffer).
    pub fn from_text(source: impl Into<String>, name: impl Into<String>) -> Self {
        let mut session = Self::from_source(source.into(), name.into());
        session.recompile();
        session
    }

    /// Everything observable about the session right now.
    ///
    /// Cheap: nothing is compiled, rendered, or copied here. The transport
    /// figures are read from the audio thread's atomics.
    pub fn snapshot(&self) -> ProjectSnapshot<'_> {
        ProjectSnapshot {
            source: &self.source,
            name: &self.name,
            revision: self.revision,
            diagnostics: &self.diagnostics,
            valid: &self.valid,
            compiles: self.compiles,
            unsaved: self.on_disk.as_ref() != Some(&self.source),
            autosaved: self.autosaved(),
            recovery: self.recovery.as_deref(),
            midi_port: self.midi.as_ref().and_then(MidiInput::port),
            playback: self.playback_state(),
        }
    }

    /// Run a command.
    ///
    /// # Errors
    /// [`ProjectError::Io`] from [`ProjectCommand::Save`], or
    /// [`ProjectError::Engine`] / [`ProjectError::Performance`] when a
    /// transport request cannot be honoured.
    pub fn apply(&mut self, command: ProjectCommand) -> Result<ProjectUpdate, ProjectError> {
        match command {
            ProjectCommand::SetSource(text) => Ok(self.set_source(text)),
            ProjectCommand::EditScore(edit) => self.edit_score(&edit),
            ProjectCommand::EditStudio(edit) => self.edit_studio(&edit),
            ProjectCommand::ApplyEdits(edits) => {
                let edits: Vec<_> = edits.iter().map(TextEdit::to_language).collect();
                let text = musa_language::apply_edits(&self.source, &edits);
                Ok(self.set_source(text))
            }
            ProjectCommand::Format => {
                let document = musa_language::parse(&self.source);
                let text = musa_language::format(&document).text().to_owned();
                Ok(self.set_source(text))
            }
            ProjectCommand::Save => {
                self.save()?;
                Ok(ProjectUpdate::unchanged(self.revision, self.validity()))
            }
            ProjectCommand::RestoreRecovery => {
                let recovered = self.recovery.take().ok_or(ProjectError::NothingTo("recover"))?;
                Ok(self.set_source(recovered))
            }
            ProjectCommand::DiscardRecovery => {
                self.recovery = None;
                if let Some(path) = self.path.as_ref() {
                    crate::autosave::clear(path);
                }
                Ok(ProjectUpdate::unchanged(self.revision, self.validity()))
            }
            ProjectCommand::Transport(request) => {
                self.transport(request)?;
                Ok(ProjectUpdate::unchanged(self.revision, self.validity()))
            }
        }
    }

    /// What a structured edit would change, before it is made.
    ///
    /// This is the source of the counts in `04-provenance.md` §4's inline
    /// choice and of the events it haloes. It is a query: nothing is applied,
    /// and asking twice is free.
    ///
    /// # Errors
    /// [`ProjectError::NoSuchEvent`] if the command names an event this
    /// revision does not have, or [`ProjectError::NoValidScore`] if the piece
    /// has never compiled and so has no events at all.
    pub fn edit_impact(&self, command: &crate::edit::EditCommand) -> Result<crate::EditImpact, ProjectError> {
        let facts = &self.valid.as_ref().ok_or(ProjectError::NoValidScore)?.facts;
        crate::edit::impact_of(facts, command)
    }

    /// Move to the previous state.
    ///
    /// # Errors
    /// [`ProjectError::NothingTo`] at the start of the history.
    pub fn undo(&mut self) -> Result<ProjectUpdate, ProjectError> {
        let target = self.cursor.checked_sub(1).ok_or(ProjectError::NothingTo("undo"))?;
        Ok(self.travel_to(target))
    }

    /// Move to the next state.
    ///
    /// # Errors
    /// [`ProjectError::NothingTo`] at the end of the history.
    pub fn redo(&mut self) -> Result<ProjectUpdate, ProjectError> {
        let target = self.cursor.saturating_add(1);
        if target >= self.history.len() {
            return Err(ProjectError::NothingTo("redo"));
        }
        Ok(self.travel_to(target))
    }

    /// Produce an export from the last score that compiled.
    ///
    /// # Errors
    /// [`ProjectError::NoValidScore`] if the piece has never compiled, or
    /// [`ProjectError::Notation`] / [`ProjectError::Performance`] if the
    /// backend fails.
    pub fn export(&self, request: ExportRequest) -> Result<ExportArtifact, ProjectError> {
        let valid = self.valid.as_ref().ok_or(ProjectError::NoValidScore)?;
        let score = &valid.score;
        match request {
            // Already rendered when the score last compiled.
            ExportRequest::Mei => Ok(ExportArtifact::Text(valid.mei.clone())),
            ExportRequest::LilyPond => Ok(ExportArtifact::Text(render_notation(
                score,
                musa_render::NotationTarget::LilyPond,
            )?)),
            ExportRequest::MusicXml => Ok(ExportArtifact::Text(render_notation(
                score,
                musa_render::NotationTarget::MusicXml,
            )?)),
            ExportRequest::Wav => Ok(ExportArtifact::Bytes(playback::to_wav(score, &valid.studio)?)),
            ExportRequest::Midi(mode) => Ok(ExportArtifact::Bytes(playback::to_midi(score, mode)?)),
            ExportRequest::PerformanceDump => Ok(ExportArtifact::Text(playback::performance_dump(score)?)),
            ExportRequest::Kernel { normalized } => {
                let document = musa_compiler::SourceDocument::new(&valid.source, &self.name);
                let printer = if normalized {
                    musa_compiler::kernel_normalized_text
                } else {
                    musa_compiler::kernel_text
                };
                printer(&document)
                    .map(ExportArtifact::Text)
                    // The source compiled, so it elaborates; this arm exists
                    // because the printer is total in its signature, not
                    // because it is reachable.
                    .ok_or(ProjectError::NoValidScore)
            }
            ExportRequest::NotationPlanDump => {
                let plan = musa_render::plan_notation(score, &musa_render::NotationOptions::default())
                    .map_err(|error| ProjectError::Notation(error.to_string()))?;
                Ok(ExportArtifact::Text(format!("{plan:#?}")))
            }
        }
    }

    /// Start listening to a MIDI keyboard, and report which one.
    ///
    /// Idempotent, and never an error: a machine with no keyboard answers
    /// `None` and keeps working, which is the normal case on CI and on a
    /// laptop with nothing plugged in (roadmap §14.8).
    pub fn listen_to_midi(&mut self) -> Option<&str> {
        if self.midi.is_none() {
            self.midi = Some(MidiInput::open(None));
        }
        self.midi.as_ref().and_then(MidiInput::port)
    }

    /// The notes played since the last call, spelled in the piece's key and
    /// grouped into chords.
    ///
    /// A poll, not a subscription: the caller already has a loop, and a
    /// callback from the MIDI thread into the session would need a lock the
    /// MIDI thread is not allowed to take (§13.2).
    ///
    /// Presses still inside the chord window are held back, so a triad is
    /// never split across two calls. Releases are read and dropped: entry
    /// writes a notated duration the composer chose, not one the key was held
    /// for (`03-interaction.md` §3).
    pub fn midi_entry(&mut self) -> Vec<MidiEntry> {
        let Some(midi) = self.midi.as_mut() else {
            return Vec::new();
        };
        let now = std::time::Instant::now();
        while let Some(event) = midi.poll() {
            if event.pressed {
                self.entry.press(event.note, now);
            }
        }
        let key = self.valid.as_ref().and_then(|valid| valid.score.key());
        self.entry
            .ready(now)
            .into_iter()
            .map(|notes| MidiEntry {
                pitches: notes.into_iter().map(|note| crate::midi::spell(note, key)).collect(),
            })
            .collect()
    }

    /// Block until playback finishes, for callers with nothing else to do
    /// (the CLI's `play`). Returns immediately if nothing is playing.
    pub fn wait_for_playback(&self) {
        let Some(audio) = self.audio.as_ref() else { return };
        while audio.is_playing() {
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
    }

    // --- internals ---------------------------------------------------------

    fn validity(&self) -> Validity {
        Validity::from_compiles(self.compiles)
    }

    fn from_source(source: String, name: String) -> Self {
        let revision = Revision(0);
        Self {
            path: None,
            project: None,
            imports: musa_compiler::ImportSources::default(),
            import_paths: Vec::new(),
            name,
            history: vec![HistoryEntry {
                source: source.clone(),
                revision,
            }],
            source,
            on_disk: None,
            revision,
            next_revision: 1,
            diagnostics: Vec::new(),
            compiles: false,
            valid: None,
            cursor: 0,
            audio: None,
            loop_region: None,
            total_frames: 0,
            installed: None,
            recovery: None,
            midi: None,
            entry: EntryBuffer::default(),
        }
    }

    /// Resolve a structured edit through provenance and apply it
    /// transactionally (roadmap §14.6).
    ///
    /// Transactional means what it says: the candidate source is compiled
    /// before it is committed, and a candidate that does not compile leaves
    /// the session — source, revision, history, playback — exactly as it was.
    /// That costs one extra compile per successful edit, which at the rate a
    /// human edits music is not worth complicating the history to avoid.
    fn edit_score(&mut self, command: &crate::edit::EditCommand) -> Result<ProjectUpdate, ProjectError> {
        let facts = &self.valid.as_ref().ok_or(ProjectError::NoValidScore)?.facts;
        let intent = crate::edit::intent_of(facts, command)?;
        let edits = musa_language::compute_edits(&self.source, &intent)
            .map_err(|error| ProjectError::Uneditable(error.to_string()))?;
        let candidate = musa_language::apply_edits(&self.source, &edits);
        if let Some(reason) = self.first_error(&candidate) {
            return Err(ProjectError::RejectedEdit {
                intent: crate::edit::describe(command),
                reason,
            });
        }
        Ok(self.set_source(candidate))
    }

    /// Resolve a studio edit and apply it under the same transaction a score
    /// edit gets: a knob that produced an uncompilable patch changes nothing.
    fn edit_studio(&mut self, edit: &crate::studio::StudioEdit) -> Result<ProjectUpdate, ProjectError> {
        let studio = &self.valid.as_ref().ok_or(ProjectError::NoValidScore)?.studio;
        let edits = crate::studio::edits_for(studio, &self.source, edit)?;
        let edits: Vec<_> = edits.iter().map(TextEdit::to_language).collect();
        let candidate = musa_language::apply_edits(&self.source, &edits);
        if let Some(reason) = self.first_error(&candidate) {
            return Err(ProjectError::RejectedEdit {
                intent: crate::studio::describe(edit),
                reason,
            });
        }
        Ok(self.set_source(candidate))
    }

    /// The compile options this session compiles under: the import closure
    /// it last read, and nothing else.
    fn options(&self) -> CompileOptions {
        CompileOptions {
            imports: self.imports.clone(),
        }
    }

    /// The project this piece belongs to, if it was opened from inside one.
    pub fn project(&self) -> Option<&ProjectMeta> {
        self.project.as_ref()
    }

    /// The files this piece imports, transitively, in path order.
    pub fn imports(&self) -> &[PathBuf] {
        &self.import_paths
    }

    /// The first error a candidate source would produce, if any.
    fn first_error(&self, candidate: &str) -> Option<String> {
        let document = SourceDocument::new(candidate.to_owned(), self.name.clone());
        let compilation = musa_compiler::compile(&document, &self.options());
        let lines = crate::position::Lines::new(candidate);
        compilation
            .diagnostics()
            .iter()
            .map(|diagnostic| Diagnostic::from_compiler(diagnostic, &lines))
            .find(|diagnostic| diagnostic.severity == crate::diagnostic::Severity::Error)
            .map(|diagnostic| diagnostic.message)
    }

    /// Replace the source, recording a new state in the history.
    fn set_source(&mut self, text: String) -> ProjectUpdate {
        if text == self.source {
            return ProjectUpdate::unchanged(self.revision, self.validity());
        }
        // A new edit after an undo abandons the redo branch, as everywhere else.
        self.history.truncate(self.cursor.saturating_add(1));
        self.revision = Revision(self.next_revision);
        self.next_revision = self.next_revision.saturating_add(1);
        self.source = text;
        self.history.push(HistoryEntry {
            source: self.source.clone(),
            revision: self.revision,
        });
        self.cursor = self.history.len().saturating_sub(1);
        self.autosave();
        let changed = self.recompile();
        ProjectUpdate {
            revision: self.revision,
            source_changed: true,
            ..changed
        }
    }

    /// Move the history cursor and recompile the state we land on.
    fn travel_to(&mut self, index: usize) -> ProjectUpdate {
        let Some(entry) = self.history.get(index) else {
            return ProjectUpdate::unchanged(self.revision, self.validity());
        };
        self.cursor = index;
        self.source = entry.source.clone();
        self.revision = entry.revision;
        self.autosave();
        let changed = self.recompile();
        ProjectUpdate {
            revision: self.revision,
            source_changed: true,
            ..changed
        }
    }

    /// Compile the current source, updating diagnostics and — only on
    /// success — the engraved score, the export score, and the audible plan.
    ///
    /// A failed compile deliberately leaves `self.valid` alone. That single
    /// decision is what makes a half-typed bar non-destructive: the score
    /// stays on screen and playback keeps running from the last revision
    /// that made sense (roadmap §14.7).
    fn recompile(&mut self) -> ProjectUpdate {
        let (imports, import_paths) = crate::imports::closure(&self.name, &self.source);
        self.imports = imports;
        self.import_paths = import_paths;
        let document = SourceDocument::new(self.source.clone(), self.name.clone());
        let compilation = musa_compiler::compile(&document, &self.options());
        let lines = crate::position::Lines::new(&self.source);
        let diagnostics: Vec<Diagnostic> = compilation
            .diagnostics()
            .iter()
            .map(|diagnostic| Diagnostic::from_compiler(diagnostic, &lines))
            .collect();
        let diagnostics_changed = diagnostics != self.diagnostics;
        self.diagnostics = diagnostics;

        let revision = self.revision;
        let identity = compilation.identity();
        // A snapshot alongside error diagnostics is a partial recovery, not a
        // score: taking it would show the user something they did not write.
        let (score, studio) = if compilation.has_errors() {
            (None, musa_compiler::StudioSpec::default())
        } else {
            compilation.into_parts()
        };
        self.compiles = score.is_some();
        let mut score_changed = false;
        if let Some(mut score) = score {
            // The project's composer, for a piece that named none. Done here
            // rather than in the compiler because a piece opened on its own
            // is still a whole piece: the `musa.toml` above it is context,
            // not part of the document.
            if let Some(composer) = self.project.as_ref().and_then(|meta| meta.composer.as_deref()) {
                score.inherit_composer(composer);
            }
            match render_notation(&score, musa_render::NotationTarget::Mei) {
                Ok(mei) => {
                    score_changed = self.valid.as_ref().is_none_or(|valid| valid.mei != mei);
                    let facts = crate::facts::ScoreFacts::derive(&score, &self.source);
                    let parts: Vec<String> = facts.parts.iter().map(|part| part.name.clone()).collect();
                    let studio_facts = crate::studio::StudioFacts::derive(&studio, &parts);
                    self.valid = Some(ValidArtifacts {
                        mei,
                        score,
                        source: self.source.clone(),
                        studio,
                        facts,
                        studio_facts,
                        revision,
                        identity,
                    });
                    // `score_changed` is about the *engraving*: it decides
                    // whether the interface redraws. Whether playback is
                    // stale is a different question, and `install_current_plan`
                    // is the one place that answers it.
                    self.reinstall_plan();
                }
                Err(error) => {
                    // Engraving failed on a score that compiled: report it
                    // like any other problem rather than losing the edit.
                    self.compiles = false;
                    self.diagnostics.push(Diagnostic {
                        severity: crate::diagnostic::Severity::Error,
                        code: "engrave".to_owned(),
                        message: error.to_string(),
                        labels: Vec::new(),
                        help: Some("this is a bug in musa, not in the piece".to_owned()),
                        note: None,
                        fixes: Vec::new(),
                        span: None,
                    });
                }
            }
        }
        ProjectUpdate {
            revision,
            source_changed: false,
            score_changed,
            diagnostics_changed,
            validity: Validity::from_compiles(self.compiles),
        }
    }

    /// Rebuild the playback plan and hand it to the engine — but only if the
    /// engine is open. Preparing a plan nobody can hear is pure waste, and
    /// the first `Play` builds one anyway.
    fn reinstall_plan(&mut self) {
        if self.audio.is_none() {
            return;
        }
        if let Err(error) = self.install_current_plan() {
            tracing::warn!(%error, "could not install the updated playback plan");
        }
    }

    /// What a plan built right now would be built from: the music and the
    /// studio. `None` before anything has compiled.
    fn plan_identity(&self) -> Option<InstalledPlan> {
        self.valid.as_ref().map(|valid| InstalledPlan {
            music: valid.identity,
            studio: valid.studio.clone(),
        })
    }

    /// Build and install the plan, unless the one in the engine already says
    /// the same thing.
    ///
    /// The guard is here rather than at each caller because "is the installed
    /// plan stale" is one question with one answer, and the two callers —
    /// recompiling and starting playback — were asking it two different ways.
    fn install_current_plan(&mut self) -> Result<(), ProjectError> {
        let installed = self.plan_identity().ok_or(ProjectError::NoValidScore)?;
        if self.installed.as_ref() == Some(&installed) {
            return Ok(());
        }
        let valid = self.valid.as_ref().ok_or(ProjectError::NoValidScore)?;
        let plan = playback::prepare(&valid.score, &valid.studio)?;
        self.total_frames = plan.total_frames();
        let audio = self
            .audio
            .as_ref()
            .ok_or_else(|| ProjectError::Engine("not open".into()))?;
        audio
            .install(plan)
            .map_err(|error| ProjectError::Engine(error.to_string()))?;
        self.installed = Some(installed);
        Ok(())
    }

    fn transport(&mut self, request: TransportRequest) -> Result<(), ProjectError> {
        let command = match request {
            TransportRequest::Play => {
                self.ensure_playable()?;
                TransportCommand::Play
            }
            TransportRequest::Stop => TransportCommand::Stop,
            TransportRequest::Seek { frame } => TransportCommand::Seek { frame },
            TransportRequest::SetLoop { start, end } => {
                if start < end {
                    self.loop_region = Some((start, end));
                }
                TransportCommand::SetLoop { start, end }
            }
            TransportRequest::ClearLoop => {
                self.loop_region = None;
                TransportCommand::ClearLoop
            }
        };
        // Transport requests other than Play are no-ops while silent: there
        // is no device to command and nothing to stop or seek.
        let Some(audio) = self.audio.as_ref() else {
            return Ok(());
        };
        audio
            .command(command)
            .map_err(|error| ProjectError::Engine(error.to_string()))
    }

    /// Open the device on first use and make sure the current score is what
    /// is installed.
    fn ensure_playable(&mut self) -> Result<(), ProjectError> {
        if self.valid.is_none() {
            return Err(ProjectError::NoValidScore);
        }
        if self.audio.is_none() {
            let config = EngineConfig {
                sample_rate: playback::sample_rate(),
            };
            self.audio = Some(AudioEngine::open(config).map_err(|error| ProjectError::Engine(error.to_string()))?);
        }
        self.install_current_plan()
    }

    fn playback_state(&self) -> PlaybackState {
        let (playing, position_frames) = self
            .audio
            .as_ref()
            .map_or((false, 0), |audio| (audio.is_playing(), audio.position()));
        PlaybackState {
            playing,
            position_frames,
            total_frames: self.total_frames,
            sample_rate: playback::sample_rate(),
            loop_region: self.loop_region,
        }
    }

    /// Keep the current text where a crash cannot lose it.
    ///
    /// Called after every state change rather than on a timer of its own:
    /// the caller has already debounced (§10.7), so one command is one
    /// settled edit. A piece with no file yet has nowhere to put a copy,
    /// which the snapshot reports as `autosaved: false`.
    fn autosave(&self) {
        let Some(path) = self.path.as_ref() else { return };
        if self.on_disk.as_ref() == Some(&self.source) {
            crate::autosave::clear(path);
        } else {
            crate::autosave::write(path, &self.source);
        }
    }

    /// Whether the unsaved text is currently held in a recovery copy.
    fn autosaved(&self) -> bool {
        self.path.is_some() && self.on_disk.as_ref() != Some(&self.source)
    }

    fn save(&mut self) -> Result<(), ProjectError> {
        let path = self.path.as_ref().ok_or(ProjectError::NothingTo("save to"))?;
        std::fs::write(path, &self.source).map_err(|error| ProjectError::io(path.display(), error))?;
        crate::autosave::clear(path);
        self.on_disk = Some(self.source.clone());
        Ok(())
    }
}

fn render_notation(
    score: &musa_compiler::ScoreSnapshot,
    target: musa_render::NotationTarget,
) -> Result<String, ProjectError> {
    musa_render::render_notation(score, target, &musa_render::NotationOptions::default())
        .map(|rendered| rendered.text().to_owned())
        .map_err(|error| ProjectError::Notation(error.to_string()))
}

#[cfg(test)]
mod playback_identity_laws {
    use super::{ProjectCommand, ProjectSession};

    const PIECE: &str = concat!(
        "piece \"Test\" {\n",
        "    tempo quarter = 120;\n",
        "    meter 4/4;\n",
        "    score {\n",
        "        part piano {\n",
        "            voice upper {\n",
        "                c4 1/4;\n",
        "                e4 1/4;\n",
        "            }\n",
        "        }\n",
        "    }\n",
        "}\n",
    );

    /// The claim playback rests on: an edit that does not change the music
    /// does not change what the engine was given, so the plan is not
    /// reinstalled and a note being held is not cut off.
    ///
    /// Asserted on the key `install_current_plan` compares rather than on the
    /// engine, because installing needs a sound card and a test does not have
    /// one — and because the key is the decision. The old counter failed this
    /// test by construction: every keystroke minted a revision.
    #[test]
    fn an_edit_that_does_not_change_the_music_does_not_change_the_plan() {
        let mut session = ProjectSession::from_text(PIECE, "test.musa");
        let before = session.plan_identity();
        assert!(before.is_some(), "the fixture compiles");

        // Appended after the piece, so that no note's source span moves: the
        // identity covers provenance on purpose (`Compilation::identity`), so
        // "the text changed and the music did not" means the notes' own text
        // did not move either.
        let commented = format!("{PIECE}\n// a note to self\n");
        assert!(
            session.apply(ProjectCommand::SetSource(commented)).is_ok(),
            "a comment is valid"
        );
        assert!(session.snapshot().compiles());
        assert_eq!(
            session.plan_identity(),
            before,
            "a comment is not music, so the plan it would build is the same plan"
        );
    }

    /// And the other direction, without which the first test would pass for a
    /// hash that never changes at all.
    #[test]
    fn an_edit_that_changes_a_note_changes_the_plan() {
        let mut session = ProjectSession::from_text(PIECE, "test.musa");
        let before = session.plan_identity();

        assert!(
            session
                .apply(ProjectCommand::SetSource(PIECE.replace("e4 1/4;", "g4 1/4;")))
                .is_ok(),
            "still valid"
        );
        assert!(session.snapshot().compiles());
        assert_ne!(session.plan_identity(), before, "a different note is different music");
    }

    /// A studio edit is inaudible in the score and audible in the sound, so it
    /// must reinstall: the plan is built from two documents, and semantic
    /// identity covers only one of them.
    #[test]
    fn an_edit_that_changes_only_the_studio_still_changes_the_plan() {
        // The studio block goes inside the piece, before its closing brace —
        // which is the only `\n}\n` in the fixture.
        let with_studio = PIECE.replace(
            "\n}\n",
            concat!(
                "\n",
                "    studio {\n",
                "        patch lead {\n",
                "            oscillator(sine) |> gain(-6 dB) |> output;\n",
                "        }\n",
                "\n",
                "        assign piano -> lead;\n",
                "        route piano -> master;\n",
                "    }\n",
                "}\n",
            ),
        );
        let mut session = ProjectSession::from_text(&with_studio, "test.musa");
        assert!(session.snapshot().compiles(), "fixture with a studio must compile");
        let before = session.plan_identity();

        assert!(
            session
                .apply(ProjectCommand::SetSource(with_studio.replace("-6 dB", "-12 dB")))
                .is_ok(),
            "still valid"
        );
        assert!(session.snapshot().compiles());
        assert_ne!(
            session.plan_identity(),
            before,
            "the notes are the same and the sound is not"
        );
    }
}
