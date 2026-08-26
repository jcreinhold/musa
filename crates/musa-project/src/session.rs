//! The session itself: one canonical source, one history, one orchestration.

use std::path::{Path, PathBuf};

use musa_compiler::{CompileOptions, SourceDocument};

use musa_playback::{AudioEngine, EngineConfig, MidiInput, TransportCommand};
use musa_score::{MusicalTime, Scope};
use musa_syntax::BarSpacing;
use musa_syntax::ast::{Document, PieceDecl};

use crate::command::{DocumentId, ProjectCommand, ProjectUpdate, Revision, TextEdit, TransportRequest, Validity};
use crate::diagnostic::Diagnostic;
use crate::error::ProjectError;
use crate::export::{ExportArtifact, ExportRequest};
use crate::midi::{EntryBuffer, MidiEntry};
use crate::playback;
use crate::project::ProjectMeta;
use crate::snapshot::{PlaybackState, ProjectSnapshot, ValidArtifacts};
use crate::template::Template;

fn package_diagnostic(message: &str, detail: &str) -> Diagnostic {
    Diagnostic {
        severity: crate::diagnostic::Severity::Error,
        code: "package-lock".to_owned(),
        message: message.to_owned(),
        labels: Vec::new(),
        help: Some(detail.to_owned()),
        note: Some("package checking is offline; change exact pins and run `musa fetch` explicitly".to_owned()),
        fixes: Vec::new(),
        causes: Vec::new(),
        span: None,
    }
}

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
    /// Which document this is, so a caller holding snapshots from two
    /// sessions can tell them apart (see [`DocumentId`]).
    document: DocumentId,
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
    /// Current verified-asset facts and the closure identity used by audio.
    assets: crate::assets::AssetInventory,
    /// Which reading of the work this session compiles
    /// (`docs/rules/events/11-realization.md`). A piece that leaves nothing open
    /// never consults it, which is why it is a plain field with a default
    /// rather than something an opener has to supply.
    realization: musa_score::Realization,
    /// Diagnostics for the *current* source.
    diagnostics: Vec<Diagnostic>,
    /// Which of the two things this document is. Material has no
    /// score and never will, which is a different fact from "no score yet".
    kind: musa_compiler::DocumentKind,
    /// Whether the current source is well-formed as what it is.
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
    /// identity (docs/rules/events/05 N6) and the studio that voices it. Keying on
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
    studio: musa_dsp::StudioExecution,
    assets: [u8; 32],
}

/// One state of the document.
///
/// The realization is part of the state, not a setting beside it. A composer
/// who draws a performance and dislikes it reaches for undo, and undo has to
/// have somewhere to go back to — so a pin lands in the history exactly as an
/// edit does, and there is one undo stack rather than two.
struct HistoryEntry {
    source: String,
    realization: musa_score::Realization,
    revision: Revision,
}

impl ProjectSession {
    /// Prepare one source-declared native sample map against this project's
    /// verified local asset closure. Raw bytes remain inside the project-to-DSP
    /// boundary and are rechecked at the exact read consumed by decoding.
    ///
    /// # Errors
    /// Returns source diagnostics, an unverified or changed asset error, or a
    /// bounded native sampler preparation failure.
    pub fn prepare_sample_map(
        &self,
        binding: &str,
        limits: musa_dsp::SamplerLimits,
    ) -> Result<musa_dsp::PreparedSampleMap, ProjectError> {
        if !self.assets.is_verified() {
            return Err(ProjectError::Assets(
                "the project asset closure is not verified".to_owned(),
            ));
        }
        let document = SourceDocument::new(self.source.clone(), self.name.clone());
        let checked =
            musa_compiler::checked_source_value(&document, &self.options(), binding, &musa_dsp::sample_map_schema())
                .map_err(|diagnostics| ProjectError::Performance(format!("{diagnostics:#?}")))?;
        let map =
            musa_dsp::decode_sample_map(&checked).map_err(|error| ProjectError::Performance(error.to_string()))?;
        musa_dsp::prepare_sample_map(
            map,
            |logical| self.assets.read_verified(logical).map_err(|error| error.to_string()),
            limits,
        )
        .map_err(|error| ProjectError::Performance(error.to_string()))
    }

    /// Adapt one source-declared `sfz@1` instrument through an ordinary
    /// checked `SampleMapArtifact`, then prepare its verified samples.
    ///
    /// # Errors
    /// Refuses an absent/non-SFZ declaration, an unverified asset, any parser
    /// bound or support-matrix violation, a checked-map disagreement, or a
    /// native sampler preparation failure.
    pub fn prepare_sfz_instrument(
        &self,
        instrument: &str,
        sfz_limits: crate::sfz::SfzLimits,
        sampler_limits: musa_dsp::SamplerLimits,
    ) -> Result<(musa_dsp::PreparedSampleMap, crate::sfz::SfzInstrumentFacts), ProjectError> {
        if !self.assets.is_verified() {
            return Err(ProjectError::Assets(
                "the project asset closure is not verified".to_owned(),
            ));
        }
        let parsed = musa_syntax::parse(&self.source);
        let root = parsed.syntax();
        let declarations = Document::of_root(&root)
            .into_iter()
            .flat_map(|document| document.instruments())
            .chain(
                PieceDecl::from_root(&root)
                    .into_iter()
                    .flat_map(|piece| piece.instruments()),
            );
        let asset = declarations
            .filter(|declaration| declaration.name().as_deref() == Some(instrument))
            .find_map(|declaration| declaration.asset())
            .ok_or_else(|| ProjectError::Assets(format!("no asset instrument named `{instrument}`")))?;
        let fact = self
            .assets
            .facts()
            .iter()
            .find(|fact| fact.path == asset)
            .ok_or_else(|| {
                ProjectError::Assets(format!("instrument `{instrument}` asset is absent from the closure"))
            })?;
        if fact.kind != Some(crate::assets::AssetKind::Sfz) || fact.adapter.as_deref() != Some("sfz@1") {
            return Err(ProjectError::Assets(format!(
                "instrument `{instrument}` requires a verified `kind = \"sfz\"`, `adapter = \"sfz@1\"` asset"
            )));
        }
        let bytes = self.assets.read_verified(&asset)?;
        let adapted = crate::sfz::adapt(instrument, &asset, &bytes, sfz_limits)?;
        let document = SourceDocument::new(adapted.source, format!("sfz:{asset}"));
        let checked = musa_compiler::checked_source_value(
            &document,
            &CompileOptions::default(),
            "imported_sfz_map",
            &musa_dsp::sample_map_schema(),
        )
        .map_err(|diagnostics| {
            ProjectError::Performance(format!("SFZ adapter result did not check: {diagnostics:#?}"))
        })?;
        let map = musa_dsp::decode_sample_map(&checked).map_err(|error| {
            ProjectError::Performance(format!("SFZ adapter result disagrees with SampleMap: {error}"))
        })?;
        let prepared = musa_dsp::prepare_sample_map(
            map,
            |logical| self.assets.read_verified(logical).map_err(|error| error.to_string()),
            sampler_limits,
        )
        .map_err(|error| ProjectError::Performance(error.to_string()))?;
        Ok((prepared, adapted.facts))
    }

    /// Adapt one source-declared `sf2@1` preset through an ordinary checked
    /// `SampleMapArtifact`, then prepare its verified embedded samples.
    ///
    /// # Errors
    /// Refuses an absent/non-SoundFont declaration, an unverified bank, an
    /// invalid preset fragment, any bounded RIFF/Hydra or support-matrix
    /// violation, a checked-map disagreement, or native preparation failure.
    pub fn prepare_sf2_instrument(
        &self,
        instrument: &str,
        sf2_limits: crate::sf2::Sf2Limits,
        sampler_limits: musa_dsp::SamplerLimits,
    ) -> Result<(musa_dsp::PreparedSampleMap, crate::sf2::Sf2InstrumentFacts), ProjectError> {
        if !self.assets.is_verified() {
            return Err(ProjectError::Assets(
                "the project asset closure is not verified".to_owned(),
            ));
        }
        let parsed = musa_syntax::parse(&self.source);
        let root = parsed.syntax();
        let declarations = Document::of_root(&root)
            .into_iter()
            .flat_map(|document| document.instruments())
            .chain(
                PieceDecl::from_root(&root)
                    .into_iter()
                    .flat_map(|piece| piece.instruments()),
            );
        let address = declarations
            .filter(|declaration| declaration.name().as_deref() == Some(instrument))
            .find_map(|declaration| declaration.asset())
            .ok_or_else(|| ProjectError::Assets(format!("no asset instrument named `{instrument}`")))?;
        let asset = crate::assets::soundfont_asset_base(&address).ok_or_else(|| {
            ProjectError::Assets(format!(
                "instrument `{instrument}` requires a canonical SoundFont preset fragment"
            ))
        })?;
        let fact = self
            .assets
            .facts()
            .iter()
            .find(|fact| fact.path == asset)
            .ok_or_else(|| {
                ProjectError::Assets(format!("instrument `{instrument}` bank is absent from the closure"))
            })?;
        if fact.kind != Some(crate::assets::AssetKind::SoundFont) || fact.adapter.as_deref() != Some("sf2@1") {
            return Err(ProjectError::Assets(format!(
                "instrument `{instrument}` requires a verified `kind = \"sound-font\"`, `adapter = \"sf2@1\"` asset"
            )));
        }
        if !asset.to_ascii_lowercase().ends_with(".sf2") {
            return Err(ProjectError::Assets("sf2@1 does not accept SF3 banks".to_owned()));
        }
        let bytes = self.assets.read_verified(&asset)?;
        let adapted = crate::sf2::adapt(instrument, &address, &bytes, sf2_limits)?;
        let document = SourceDocument::new(adapted.source, format!("sf2:{address}"));
        let checked = musa_compiler::checked_source_value(
            &document,
            &CompileOptions::default(),
            "imported_sf2_map",
            &musa_dsp::sample_map_schema(),
        )
        .map_err(|diagnostics| {
            ProjectError::Performance(format!("SoundFont adapter result did not check: {diagnostics:#?}"))
        })?;
        let map = musa_dsp::decode_sample_map(&checked).map_err(|error| {
            ProjectError::Performance(format!("SoundFont adapter result disagrees with SampleMap: {error}"))
        })?;
        let prepared = musa_dsp::prepare_sample_map(
            map,
            |logical| {
                adapted
                    .samples
                    .get(logical)
                    .cloned()
                    .ok_or_else(|| format!("embedded SoundFont sample `{logical}` is absent"))
            },
            sampler_limits,
        )
        .map_err(|error| ProjectError::Performance(error.to_string()))?;
        Ok((prepared, adapted.facts))
    }

    /// Open an existing `.musa` file.
    ///
    /// A file that does not compile still opens: the session reports its
    /// diagnostics and has no score yet.
    ///
    /// # Errors
    /// [`ProjectError::Io`] if the file cannot be read as UTF-8.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, ProjectError> {
        let path = path.as_ref();
        let span = tracing::info_span!("open", path = %path.display());
        let _entered = span.enter();
        let source = std::fs::read_to_string(path).map_err(|error| ProjectError::io(path.display(), error))?;
        let mut session = Self::from_source(source.clone(), path.to_string_lossy().into_owned());
        session.path = Some(path.to_path_buf());
        session.project = crate::project::find(path);
        session.recovery = crate::autosave::take(path, &source);
        session.on_disk = Some(source);
        // Before the first compile, because a realization is an *input* to
        // one: opening a piece and seeing a different page than the one it
        // was left showing is exactly the unreliability a saved realization
        // removes.
        session.realization = crate::realization::read(path);
        if let Some(first) = session.history.first_mut() {
            first.realization = session.realization.clone();
        }
        session.recompile();
        // A project file changes where imports resolve from and a recovery
        // copy changes what the composer is looking at, and neither is
        // visible from the path that was asked for.
        tracing::debug!(
            bytes = session.source.len(),
            project = session.project.is_some(),
            recovery = session.recovery.is_some(),
            "opened"
        );
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
    /// `docs/rules/desktop/05-states.md` §2 shows a real engraved system and puts
    /// the caret in it, and asking for a path first would make the empty
    /// state a file dialog.
    pub fn new_piece(template: Template, title: &str) -> Self {
        Self::from_text(template.source(title), format!("{title}.musa"))
    }

    /// A session over text with no file behind it, for callers that hold the
    /// document themselves (tests, and the desktop app's scratch buffer).
    /// A piece named by its absolute path finds the project it is filed
    /// under, the same walk [`Self::open`] makes. That is what lets the
    /// language server — which names its buffers by path so that `use`
    /// resolves — answer a format request with the layout the manifest asks
    /// for, instead of silently disagreeing with `musa format`. A name that
    /// is not an absolute path is a scratch buffer and belongs to no project,
    /// which is also why this does not depend on the working directory.
    pub fn from_text(source: impl Into<String>, name: impl Into<String>) -> Self {
        let name = name.into();
        let filed_under = Path::new(&name);
        let mut session = Self::from_source(source.into(), name.clone());
        if filed_under.is_absolute() {
            session.project = crate::project::find(filed_under);
        }
        session.recompile();
        session
    }

    /// Everything observable about the session right now.
    ///
    /// Cheap: nothing is compiled, rendered, or copied here. The transport
    /// figures are read from the audio thread's atomics.
    pub fn snapshot(&self) -> ProjectSnapshot<'_> {
        ProjectSnapshot {
            document: self.document,
            source: &self.source,
            name: &self.name,
            revision: self.revision,
            diagnostics: &self.diagnostics,
            imports: &self.imports,
            valid: &self.valid,
            compiles: self.compiles,
            unsaved: self.on_disk.as_ref() != Some(&self.source),
            autosaved: self.autosaved(),
            recovery: self.recovery.as_deref(),
            midi_port: self.midi.as_ref().and_then(MidiInput::port),
            playback: self.playback_state(),
            kind: self.kind,
            // A session on its own knows nothing about the volume it is filed
            // in; `Project::snapshot` is what fills this in.
            contents: None,
            assets: &self.assets,
        }
    }

    /// Run a command.
    ///
    /// # Errors
    /// [`ProjectError::Io`] from [`ProjectCommand::Save`], or
    /// [`ProjectError::Engine`] / [`ProjectError::Performance`] when a
    /// transport request cannot be honoured.
    pub fn apply(&mut self, command: ProjectCommand) -> Result<ProjectUpdate, ProjectError> {
        // One span per command, so every compile, engrave, and plan rebuild
        // the command sets off is filed under the thing the user did.
        let span = tracing::info_span!("apply", command = command.name(), from = self.revision.0);
        let _entered = span.enter();
        let result = self.dispatch(command);
        match &result {
            Ok(update) => tracing::debug!(to = update.revision.0, validity = ?update.validity, "applied"),
            Err(error) => tracing::debug!(%error, "refused"),
        }
        result
    }

    /// [`Self::apply`] without the span, so the span is entered exactly once.
    fn dispatch(&mut self, command: ProjectCommand) -> Result<ProjectUpdate, ProjectError> {
        match command {
            ProjectCommand::SetSource(text) => Ok(self.set_source(text)),
            ProjectCommand::EditScore(edit) => self.edit_score(&edit),
            ProjectCommand::EditStudio(edit) => self.edit_studio(&edit),
            ProjectCommand::ApplyEdits(edits) => {
                let edits: Vec<_> = edits.iter().map(TextEdit::to_language).collect();
                let text = musa_syntax::apply_edits(&self.source, &edits);
                Ok(self.set_source(text))
            }
            ProjectCommand::Format => {
                let text = self.formatted_source();
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
            ProjectCommand::NewPerformance { performance } => {
                let mut next = self.realization.clone();
                next.reseed(performance);
                Ok(self.set_realization(next))
            }
            ProjectCommand::Pin(path) => {
                let path = musa_score::ChoicePath::parse(&path).ok_or(ProjectError::NothingTo("keep"))?;
                // What is pinned is what this compile decided *there*: pinning
                // is "keep this one", and the only thing that could be kept is
                // the answer the composer is looking at.
                let valid = self.valid.as_ref().ok_or(ProjectError::NoValidScore)?;
                let decision = valid
                    .decisions
                    .iter()
                    .find(|record| *record.path() == path)
                    .ok_or(ProjectError::NothingTo("keep"))?
                    .decision()
                    .clone();
                let mut next = self.realization.clone();
                next.pin(path, decision);
                Ok(self.set_realization(next))
            }
            ProjectCommand::Unpin(path) => {
                let path = musa_score::ChoicePath::parse(&path).ok_or(ProjectError::NothingTo("release"))?;
                let mut next = self.realization.clone();
                next.unpin(&path);
                Ok(self.set_realization(next))
            }
            ProjectCommand::Transport(request) => {
                self.transport(request)?;
                Ok(ProjectUpdate::unchanged(self.revision, self.validity()))
            }
        }
    }

    /// What [`ProjectCommand::Format`] would write, without writing it.
    ///
    /// A question, like [`Self::is_formatted`]: `musa format --diff` promises
    /// to show rather than edit, and an edit would be autosaved, so the
    /// preview leaves no `.recovery` copy behind.
    /// A document too broken to read is returned unchanged, which is the same
    /// promise the surface formatter makes: formatting never guesses at what
    /// an author meant to write.
    #[must_use]
    pub fn formatted_source(&self) -> String {
        musa_compiler::format_document(&self.source, self.bar_spacing()).unwrap_or_else(|| self.source.clone())
    }

    /// How this piece's project wants its bars laid out.
    ///
    /// Private, and asked here rather than at each call site, so that
    /// [`ProjectCommand::Format`] and [`Self::formatted_source`] cannot answer
    /// differently — a preview that disagrees with the edit it previews is the
    /// one bug this setting could introduce. A piece with no project above it
    /// gets [`BarSpacing::Compact`], which is what a file with no manifest
    /// already is.
    fn bar_spacing(&self) -> BarSpacing {
        self.project
            .as_ref()
            .map_or(BarSpacing::Compact, |meta| meta.bar_spacing)
    }

    /// Whether the text is already what [`ProjectCommand::Format`] would
    /// write.
    ///
    /// A question, not an edit. `musa format --check` promises to fail rather
    /// than write, and formatting to find out breaks that promise: an unsaved
    /// edit is autosaved, so the check leaves a `.recovery` copy beside every
    /// file it rejects.
    #[must_use]
    pub fn is_formatted(&self) -> bool {
        self.formatted_source() == self.source
    }

    /// What a structured edit would change, before it is made.
    ///
    /// This is the source of the counts in `04-provenance.md` §4's inline
    /// choice and of the events it haloes, and of the token a live pointer
    /// gesture marks in the source before it commits. It is a
    /// query: nothing is applied, and asking twice is free.
    ///
    /// The text it would write is computed by the same code that would write
    /// it, so what the composer is shown mid-gesture is not a second guess at
    /// the edit. A command this source cannot express writes nothing and says
    /// so by returning no writes; the counts are still true, so the caller
    /// gets an answer rather than an error for a question it can ask at
    /// pointer rate.
    ///
    /// # Errors
    /// [`ProjectError::NoSuchEvent`] if the command names an event this
    /// revision does not have, or [`ProjectError::NoValidScore`] if the piece
    /// has never compiled and so has no events at all.
    pub fn edit_impact(&self, command: &crate::edit::EditCommand) -> Result<crate::EditImpact, ProjectError> {
        let facts = &self.valid.as_ref().ok_or(ProjectError::NoValidScore)?.facts;
        let mut impact = crate::edit::impact_of(facts, command)?;
        // An adapter command's text comes from the adapter, by the same code
        // that would apply it — so what a composer is shown mid-gesture is the
        // adapter's own answer and not a second guess at it, exactly as it is
        // for the edits this crate resolves itself.
        if let crate::edit::EditCommand::AdapterCommand { .. } = *command {
            impact.writes = self
                .adapter_edits(command)
                .unwrap_or_default()
                .into_iter()
                .map(|edit| crate::edit::CandidateEdit {
                    start: edit.start,
                    end: edit.end,
                    text: edit.text,
                })
                .collect();
            return Ok(impact);
        }
        impact.writes = crate::edit::intent_of(facts, command)
            .ok()
            .and_then(|intent| musa_syntax::compute_edits(&self.source, &intent).ok())
            .unwrap_or_default()
            .iter()
            .map(|edit| crate::edit::CandidateEdit {
                start: u32::from(edit.range.start()),
                end: u32::from(edit.range.end()),
                text: edit.replacement.clone(),
            })
            .collect();
        Ok(impact)
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
        // The export is of the last score that *compiled*, which may be older
        // than the source on screen. Which revision it came from is the
        // difference between a stale artifact and a mystery.
        let span = tracing::info_span!("export", request = ?request, revision = self.revision.0);
        let _entered = span.enter();
        let valid = self.valid.as_ref().ok_or(ProjectError::NoValidScore)?;
        let score = &valid.score;
        match request {
            // Already rendered when the score last compiled.
            // Already rendered when the score last compiled, so the text is
            // taken from there; the losses are a fact about the target and
            // are the same either way.
            ExportRequest::Mei => Ok(ExportArtifact::text(valid.mei.clone()).warn(valid.mei_warnings.clone())),
            ExportRequest::LilyPond => Ok(render_notation(score, musa_notation::NotationTarget::LilyPond)?),
            ExportRequest::MusicXml => Ok(render_notation(score, musa_notation::NotationTarget::MusicXml)?),
            ExportRequest::Wav => Ok(ExportArtifact::bytes(playback::to_wav(score, &valid.studio_execution)?)),
            ExportRequest::Midi(mode) => {
                let (bytes, warnings) = playback::to_midi(score, mode)?;
                Ok(ExportArtifact::bytes(bytes).warn(warnings))
            }
            ExportRequest::PerformanceDump => Ok(ExportArtifact::text(playback::performance_dump(score)?)),
            ExportRequest::Events { normalized } => {
                let document = musa_compiler::SourceDocument::new(&valid.source, &self.name);
                let printer = if normalized {
                    musa_compiler::events_normalized_text
                } else {
                    musa_compiler::events_text
                };
                printer(&document, &self.realization, &self.imports)
                    .map(ExportArtifact::text)
                    // The source compiled, so it elaborates; this arm exists
                    // because the printer is total in its signature, not
                    // because it is reachable.
                    .ok_or(ProjectError::NoValidScore)
            }
            ExportRequest::NotationPlanDump => {
                let plan = musa_notation::plan_notation(score, &musa_notation::NotationOptions::default())
                    .map_err(|error| ProjectError::Notation(error.to_string()))?;
                Ok(ExportArtifact::text(format!("{plan:#?}")))
            }
        }
    }

    /// Observe the last score that compiled, without changing it.
    ///
    /// The one caller-facing analysis operation. It reads: no
    /// revision is minted, no diagnostic is raised, and the source is
    /// untouched — an analysis that could report *into* the session would be
    /// a lint with extra steps.
    ///
    /// # Errors
    /// [`ProjectError::NoValidScore`] if the piece has never compiled, or
    /// [`ProjectError::Analysis`] when the request names a part or voice this
    /// score does not have, or a window with no music in it.
    pub fn analyze(&self, request: &musa_score::AnalysisRequest) -> Result<crate::AnalysisFacts, ProjectError> {
        let span = tracing::info_span!("analyze");
        let _entered = span.enter();
        let valid = self.valid.as_ref().ok_or(ProjectError::NoValidScore)?;
        let report =
            musa_score::analyze(&valid.score, request).map_err(|error| ProjectError::Analysis(error.to_string()))?;
        // The kind is read off the report rather than the request: the report
        // already answers it, and adding an accessor to `AnalysisRequest` for
        // a log line would be a public item with one caller.
        tracing::debug!(kind = ?report.kind(), findings = report.findings().len(), "analyzed");
        Ok(crate::AnalysisFacts::derive(&report, &valid.score, &valid.source))
    }

    /// [`Self::analyze`], as a frontend receives it: every span restated in
    /// UTF-16 code units, and the revision it read.
    ///
    /// The same contract [`ProjectSnapshot::to_wire`] states, for the same
    /// reason and by the same walk. A report's spans point into the source the
    /// snapshot already carries, so a frontend that received them in bytes
    /// would reveal the wrong characters in exactly the documents where it
    /// matters — the ones with an accented word or an em dash in them.
    ///
    /// The revision is the score's, not the document's: an analysis reads the
    /// last valid compile (`analyze` refuses without one), so a reader typing
    /// into a piece that no longer parses is still looking at a reading of the
    /// score in front of them. It is here rather than derived on the frontend
    /// because only this side knows which compile was read
    /// (`08-elaboration.md` §8).
    ///
    /// # Errors
    /// Whatever [`Self::analyze`] refuses for.
    pub fn analyze_wire(&self, request: &musa_score::AnalysisRequest) -> Result<serde_json::Value, ProjectError> {
        /// The report, plus the compile it is a reading of.
        #[derive(serde::Serialize)]
        struct Reading<'a> {
            revision: u64,
            #[serde(flatten)]
            report: &'a crate::AnalysisFacts,
        }

        let facts = self.analyze(request)?;
        let valid = self.valid.as_ref().ok_or(ProjectError::NoValidScore)?;
        let reading = Reading {
            revision: valid.revision.0,
            report: &facts,
        };
        let mut wire = serde_json::to_value(&reading).unwrap_or(serde_json::Value::Null);
        crate::utf16::translate_spans(&mut wire, &crate::Utf16Offsets::new(&valid.source));
        Ok(wire)
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
    pub fn midi_entry(&mut self, caret: Option<&str>) -> Vec<MidiEntry> {
        let Some(midi) = self.midi.as_mut() else {
            return Vec::new();
        };
        let now = std::time::Instant::now();
        while let Some(event) = midi.poll() {
            if event.pressed {
                self.entry.press(event.note, now);
            }
        }
        // The key **at the caret**, which is the whole point of a positional
        // context: entering a C♯ after a modulation to D major must spell C♯
        // and not D♭, whatever the header says. With nothing selected there
        // is no position and the piece's opening key is the honest answer.
        let key = self.key_at_caret(caret);
        self.entry
            .ready(now)
            .into_iter()
            .map(|notes| MidiEntry {
                pitches: notes.into_iter().map(|note| crate::midi::spell(note, key)).collect(),
            })
            .collect()
    }

    /// The key a note entered at `caret` is spelled in.
    ///
    /// `caret` is an engraved event id (`event-1f`), the same identity the
    /// page and the inspector use; the note being entered goes *before* it,
    /// so its own onset is the position to ask about. Scoped to the caret's
    /// part, so a part in its own key spells in that one.
    fn key_at_caret(&self, caret: Option<&str>) -> Option<musa_score::Key> {
        let valid = self.valid.as_ref()?;
        let Some(caret) = caret else {
            return valid.score.key_at(Scope::Piece, MusicalTime::ZERO);
        };
        let (scope, at) = valid
            .score
            .parts()
            .iter()
            .flat_map(|(_, part)| {
                part.voices()
                    .flat_map(move |(_, voice)| voice.events().iter().map(move |event| (part.id(), event)))
            })
            .find(|(_, event)| format!("event-{:x}", event.id.0) == caret)
            .map_or((Scope::Piece, MusicalTime::ZERO), |(part, event)| {
                (Scope::Part { part: part.0 }, event.onset)
            });
        valid.score.key_at(scope, at)
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
            document: DocumentId::mint(),
            path: None,
            project: None,
            imports: musa_compiler::ImportSources::default(),
            import_paths: Vec::new(),
            assets: crate::assets::AssetInventory::default(),
            realization: musa_score::Realization::deterministic(),
            name,
            history: vec![HistoryEntry {
                source: source.clone(),
                realization: musa_score::Realization::deterministic(),
                revision,
            }],
            source,
            on_disk: None,
            revision,
            next_revision: 1,
            diagnostics: Vec::new(),
            kind: musa_compiler::DocumentKind::Piece,
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
        // A region is not written in Musa, so what its command means is the
        // adapter's to say. Everything after that is the same transaction.
        if let crate::edit::EditCommand::AdapterCommand { .. } = *command {
            return self.edit_adapter(command);
        }
        let facts = &self.valid.as_ref().ok_or(ProjectError::NoValidScore)?.facts;
        let intent = crate::edit::intent_of(facts, command)?;
        let edits = musa_syntax::compute_edits(&self.source, &intent)
            .map_err(|error| ProjectError::Uneditable(error.to_string()))?;
        let candidate = musa_syntax::apply_edits(&self.source, &edits);
        if let Some(reason) = self.first_error(&candidate) {
            return Err(ProjectError::RejectedEdit {
                intent: crate::edit::describe(command),
                reason,
            });
        }
        Ok(self.set_source(candidate))
    }

    /// Ask a region's adapter to serve one command, and apply what it answers
    /// under the same transaction every other structured edit gets.
    ///
    /// The adapter decides what the command means and which node it touches;
    /// this decides nothing about the region and only carries the answer
    /// through the door every edit uses. A command the adapter refuses, a
    /// region whose adapter declares no `edit`, and an adapter that is broken
    /// are three different sentences, and each arrives as the adapter's own
    /// rather than as a compiler complaint.
    fn edit_adapter(&mut self, command: &crate::edit::EditCommand) -> Result<ProjectUpdate, ProjectError> {
        let edits = self.adapter_edits(command)?;
        let edits: Vec<musa_syntax::TextEdit> = edits
            .iter()
            .map(|edit| {
                musa_syntax::TextEdit::new(
                    text_size::TextRange::new(edit.start.into(), edit.end.into()),
                    edit.text.clone(),
                )
            })
            .collect();
        let candidate = musa_syntax::apply_edits(&self.source, &edits);
        if let Some(reason) = self.first_error(&candidate) {
            return Err(ProjectError::RejectedEdit {
                intent: crate::edit::describe(command),
                reason,
            });
        }
        Ok(self.set_source(candidate))
    }

    /// What a region's adapter answers to one command, in this document's
    /// coordinates.
    fn adapter_edits(
        &self,
        command: &crate::edit::EditCommand,
    ) -> Result<Vec<musa_compiler::AdapterEdit>, ProjectError> {
        let crate::edit::EditCommand::AdapterCommand {
            at,
            ref command,
            anchor,
            ref argument,
        } = *command
        else {
            return Err(ProjectError::Uneditable("that is not an adapter command".to_owned()));
        };
        let document = musa_compiler::SourceDocument::new(&self.source, &self.name);
        musa_compiler::adapter_edits(&document, &self.options(), at, command, anchor, argument)
            .map_err(|error| ProjectError::Uneditable(error.to_string()))
    }

    /// What a region's adapter would write for one of its own commands.
    ///
    /// A question, like [`Self::edit_impact`]: nothing is applied, and the
    /// caller decides whether to send the command that applies it. The text is
    /// computed by the same code that would apply it, so an editor showing a
    /// preview is showing the adapter's answer rather than a second guess at
    /// it.
    ///
    /// # Errors
    /// [`ProjectError::Uneditable`], carrying the adapter's own sentence: a
    /// command it refuses, a region whose adapter declares no `edit` and is
    /// therefore read-only, or an offset no region stands at.
    pub fn adapter_command(
        &self,
        at: u32,
        command: &str,
        anchor: u64,
        argument: &str,
    ) -> Result<Vec<crate::CandidateEdit>, ProjectError> {
        let command = crate::edit::EditCommand::AdapterCommand {
            at,
            command: command.to_owned(),
            anchor,
            argument: argument.to_owned(),
        };
        Ok(self
            .adapter_edits(&command)?
            .into_iter()
            .map(|edit| crate::CandidateEdit {
                start: edit.start,
                end: edit.end,
                text: edit.text,
            })
            .collect())
    }

    /// Resolve a studio edit and apply it under the same transaction a score
    /// edit gets: a knob that produced an uncompilable patch changes nothing.
    fn edit_studio(&mut self, edit: &crate::studio::StudioEdit) -> Result<ProjectUpdate, ProjectError> {
        let valid = self.valid.as_ref().ok_or(ProjectError::NoValidScore)?;
        let edits = crate::studio::edits_for(&valid.studio_execution, &valid.studio_spans, &self.source, edit)?;
        let edits: Vec<_> = edits.iter().map(TextEdit::to_language).collect();
        let candidate = musa_syntax::apply_edits(&self.source, &edits);
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
            realization: self.realization.clone(),
        }
    }

    /// Compile under `realization` from here on.
    ///
    /// Not an edit — the source is untouched and nothing about the piece
    /// changed. It *is* a state of the session: the page a composer is
    /// looking at is a function of the source **and** the realization
    /// (`docs/rules/events/11-realization.md`), so a new realization takes a
    /// revision and lands in the history, and undo goes back to the reading
    /// that was on screen before.
    pub fn realize(&mut self, realization: musa_score::Realization) -> ProjectUpdate {
        self.set_realization(realization)
    }

    /// Move to `realization`, recording the move so it can be undone.
    fn set_realization(&mut self, realization: musa_score::Realization) -> ProjectUpdate {
        if realization == self.realization {
            return ProjectUpdate::unchanged(self.revision, self.validity());
        }
        // Which performance is on screen. A piece that leaves something open
        // compiles to different music under different seeds, and every later
        // "why does this sound different" question starts here.
        tracing::debug!(seed = realization.seed(), "realizing");
        // A new performance after an undo abandons the redo branch, as every
        // other state change does.
        self.history.truncate(self.cursor.saturating_add(1));
        self.revision = Revision(self.next_revision);
        self.next_revision = self.next_revision.saturating_add(1);
        self.realization = realization;
        self.history.push(HistoryEntry {
            source: self.source.clone(),
            realization: self.realization.clone(),
            revision: self.revision,
        });
        self.cursor = self.history.len().saturating_sub(1);
        self.save_realization();
        let changed = self.recompile();
        ProjectUpdate {
            revision: self.revision,
            ..changed
        }
    }

    /// Keep the current reading beside the piece, so it survives the session.
    fn save_realization(&self) {
        if let Some(path) = self.path.as_ref() {
            crate::realization::write(path, &self.realization);
        }
    }

    /// Which reading of the work this session is compiling.
    pub fn realization(&self) -> &musa_score::Realization {
        &self.realization
    }

    /// The project this piece belongs to, if it was opened from inside one.
    pub fn project(&self) -> Option<&ProjectMeta> {
        self.project.as_ref()
    }

    /// The files this piece imports, transitively, in path order.
    pub fn imports(&self) -> &[PathBuf] {
        &self.import_paths
    }

    /// Stop, and give the audio device back.
    ///
    /// Only the piece in hand may sound, so turning to another one releases
    /// this one's stream. A session that never played never opened a device,
    /// so for most pieces in a project this does nothing at all — which is
    /// what makes holding several of them affordable.
    pub fn release_audio(&mut self) {
        if let Some(audio) = self.audio.take() {
            drop(audio);
        }
        self.installed = None;
        self.total_frames = 0;
    }

    /// The first error a candidate source would produce, if any.
    fn first_error(&self, candidate: &str) -> Option<String> {
        let document = SourceDocument::new(candidate.to_owned(), self.name.clone());
        let compilation = musa_compiler::compile(&document, &self.options());
        let lines = crate::position::Lines::new(candidate);
        compilation
            .diagnostics()
            .iter()
            .map(|diagnostic| Diagnostic::from_compiler(diagnostic, &lines, &self.imports))
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
            realization: self.realization.clone(),
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
        self.realization = entry.realization.clone();
        self.revision = entry.revision;
        self.save_realization();
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
        let (package_sources, package_assets, package_diagnostic) = match self.project.as_ref() {
            Some(project) if project.package_error.is_some() => (
                musa_compiler::ImportSources::default(),
                std::collections::BTreeMap::new(),
                project
                    .package_error
                    .as_ref()
                    .map(|error| package_diagnostic("project package policy is invalid", error)),
            ),
            Some(project) => match crate::packages::offline(&project.root, &project.packages, &self.name) {
                Ok(closure) => (closure.sources, closure.assets, None),
                Err(error) => (
                    musa_compiler::ImportSources::default(),
                    std::collections::BTreeMap::new(),
                    Some(package_diagnostic(
                        "project package closure is unavailable",
                        &error.to_string(),
                    )),
                ),
            },
            None => (
                musa_compiler::ImportSources::default(),
                std::collections::BTreeMap::new(),
                None,
            ),
        };
        let (imports, import_paths) = crate::imports::closure(&self.name, &self.source, package_sources);
        self.imports = imports;
        self.import_paths = import_paths;
        self.assets = crate::assets::session_inventory(
            self.path.as_deref(),
            self.project.as_ref(),
            &self.name,
            &self.source,
            &self.imports,
            &package_assets,
        );
        let document = SourceDocument::new(self.source.clone(), self.name.clone());
        let compilation = musa_compiler::compile(&document, &self.options());
        let lines = crate::position::Lines::new(&self.source);
        let mut diagnostics: Vec<Diagnostic> = compilation
            .diagnostics()
            .iter()
            .map(|diagnostic| Diagnostic::from_compiler(diagnostic, &lines, &self.imports))
            .collect();
        diagnostics.extend(self.assets.diagnostics().iter().cloned());
        let package_failed = package_diagnostic.is_some();
        diagnostics.extend(package_diagnostic);
        let diagnostics_changed = diagnostics != self.diagnostics;
        self.diagnostics = diagnostics;

        let revision = self.revision;
        let identity = compilation.identity();
        let kind = compilation.kind();
        let had_errors = compilation.has_errors() || !self.assets.is_verified() || package_failed;
        // A snapshot alongside error diagnostics is a partial recovery, not a
        // score: taking it would show the user something they did not write.
        // The reference record travels with the successful compile, like
        // every other fact — borrowed here because `into_parts` consumes.
        let names: Vec<musa_compiler::NameReference> = if compilation.has_errors() {
            Vec::new()
        } else {
            compilation.references().to_vec()
        };
        let items: Vec<musa_compiler::ItemDoc> = if compilation.has_errors() {
            Vec::new()
        } else {
            compilation.items().to_vec()
        };
        let decisions: Vec<musa_score::DecisionRecord> = if compilation.has_errors() {
            Vec::new()
        } else {
            compilation.decisions().to_vec()
        };
        // Where the score came from, borrowed for the same reason the
        // references are: `into_parts` consumes, and the Origin row needs the
        // record after it.
        let derivation: Option<musa_score::Derivation> = if compilation.has_errors() {
            None
        } else {
            compilation.derivation().cloned()
        };
        let studio_execution = if compilation.has_errors() {
            None
        } else {
            compilation
                .studio_source()
                .and_then(|source| musa_dsp::decode_studio_execution(source).ok())
        };
        let studio_spans = compilation.studio_spans().clone();
        let score = if had_errors || studio_execution.is_none() {
            None
        } else {
            compilation.into_snapshot()
        };
        self.kind = kind;
        // `compiles` means well-formed as *what it is*. Material declares and
        // does not sound, so it has no score, and the absence of one is not a
        // failure — which is the whole reason `kind` exists.
        self.compiles = match kind {
            // An events document has a score for the same reason a piece does,
            // and its emptiness is as legitimate: `events "x" { … timeline 0
            // {} }` is a well-formed file that denotes silence.
            musa_compiler::DocumentKind::Piece | musa_compiler::DocumentKind::Events => score.is_some(),
            // A module file names its children and declares nothing, so
            // "well-formed as what it is" is the absence of a complaint.
            musa_compiler::DocumentKind::Material | musa_compiler::DocumentKind::Modules => !had_errors,
        };
        let mut score_changed = false;
        if let (Some(mut score), Some(studio_execution)) = (score, studio_execution) {
            // The project's composer, for a piece that named none. Done here
            // rather than in the compiler because a piece opened on its own
            // is still a whole piece: the `musa.toml` above it is context,
            // not part of the document.
            if let Some(composer) = self.project.as_ref().and_then(|meta| meta.composer.as_deref()) {
                score.inherit_composer(composer);
            }
            match render_notation(&score, musa_notation::NotationTarget::Mei) {
                Ok(rendered) => {
                    let mei = rendered.as_text().unwrap_or_default().to_owned();
                    score_changed = self.valid.as_ref().is_none_or(|valid| valid.mei != mei);
                    let facts = crate::facts::ScoreFacts::derive(&score, &self.source, &decisions, derivation.as_ref());
                    let parts: Vec<String> = facts.parts.iter().map(|part| part.name.clone()).collect();
                    let studio_facts =
                        crate::studio::StudioFacts::derive(&studio_execution, &studio_spans, &score, &parts);
                    let names = names.iter().map(crate::facts::NameFact::from_compiler).collect();
                    let items = items.iter().map(crate::facts::ItemFact::from_compiler).collect();
                    self.valid = Some(ValidArtifacts {
                        decisions,
                        mei,
                        mei_warnings: rendered.warnings().to_vec(),
                        score,
                        source: self.source.clone(),
                        studio_execution,
                        studio_spans,
                        facts,
                        studio_facts,
                        names,
                        items,
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
                        causes: Vec::new(),
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
            studio: valid.studio_execution.clone(),
            assets: self.assets.identity(),
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
        let plan = playback::prepare(&valid.score, &valid.studio_execution)?;
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
    score: &musa_score::ScoreSnapshot,
    target: musa_notation::NotationTarget,
) -> Result<ExportArtifact, ProjectError> {
    musa_notation::render_notation(score, target, &musa_notation::NotationOptions::default())
        .map(|rendered| ExportArtifact::text(rendered.text()).warn(rendered.warnings().to_vec()))
        .map_err(|error| ProjectError::Notation(error.to_string()))
}

#[cfg(test)]
mod playback_identity_laws {
    use std::fs;

    use super::{ProjectCommand, ProjectSession};

    const PIECE: &str = concat!(
        "piece \"Test\" {\n",
        "    tempo quarter = 120;\n",
        "    meter 4/4;\n",
        "    score {\n",
        "        part piano {\n",
        "            voice upper {\n",
        "                c4/4\n",
        "                e4/4\n",
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
                .apply(ProjectCommand::SetSource(PIECE.replace("e4/4", "g4/4")))
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

    /// The tempo map used to be a scalar beside the timeline,
    /// so changing only the tempo left the semantic hash where it was,
    /// `install_current_plan` decided the engine was already up to date, and
    /// playback stayed at the old speed until some *other* edit dislodged it.
    ///
    /// The marking is a fact in the timeline now, so the hash moves and the
    /// plan is reinstalled. Asserted on the key rather than on the engine,
    /// for the reason the test above gives.
    #[test]
    fn changing_only_the_tempo_reinstalls_the_plan() {
        let mut session = ProjectSession::from_text(PIECE, "test.musa");
        let before = session.plan_identity();
        assert!(before.is_some(), "the fixture compiles");

        assert!(
            session
                .apply(ProjectCommand::SetSource(
                    PIECE.replace("tempo quarter = 120;", "tempo quarter = 60;")
                ))
                .is_ok(),
            "still valid"
        );
        assert!(session.snapshot().compiles());
        assert_ne!(
            session.plan_identity(),
            before,
            "the notes are the same and the speed is not"
        );
    }

    /// A raw asset is not score semantics, but it is an input to every audio
    /// artifact prepared from the project. The plan key therefore moves on a
    /// digest change while the event/notation identity stays put.
    #[test]
    fn changing_only_an_asset_changes_only_the_asset_part_of_the_plan_identity()
    -> Result<(), Box<dyn std::error::Error>> {
        let directory = tempfile::tempdir()?;
        fs::create_dir(directory.path().join("assets"))?;
        fs::write(
            directory.path().join("musa.toml"),
            concat!(
                "[assets.\"assets/tone.sfz\"]\n",
                "kind = \"sfz\"\n",
                "adapter = \"sfz@1\"\n",
            ),
        )?;
        fs::write(directory.path().join("piece.musa"), PIECE)?;
        fs::write(directory.path().join("assets/tone.sfz"), b"first")?;
        crate::lock_assets(directory.path())?;

        let mut session = ProjectSession::open(directory.path().join("piece.musa"))?;
        let before = session.plan_identity().ok_or("compiled plan identity")?;
        let before_mei = session.snapshot().mei().ok_or("engraving")?.to_owned();

        fs::write(directory.path().join("assets/tone.sfz"), b"other")?;
        crate::lock_assets(directory.path())?;
        session.apply(ProjectCommand::SetSource(format!("{PIECE}\n// recheck the closure\n")))?;
        let after = session.plan_identity().ok_or("recompiled plan identity")?;

        assert_eq!(after.music, before.music, "raw bytes are not event-track semantics");
        assert_eq!(
            after.studio, before.studio,
            "raw bytes do not rewrite the source machine graph"
        );
        assert_ne!(
            after.assets, before.assets,
            "the prepared-audio closure must be invalidated"
        );
        assert_eq!(session.snapshot().mei(), Some(before_mei.as_str()));
        Ok(())
    }
}
