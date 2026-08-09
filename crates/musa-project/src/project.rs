//! Directory projects (roadmap §16).
//!
//! A project is one file until it needs to share something. When it does, it
//! grows a directory with a `musa.toml` beside `pieces/` and `library/`, and
//! that file carries what the pieces have in common — the album's title, who
//! wrote it, the order they go in. It carries nothing a piece needs in order
//! to compile: a piece opened on its own, with no project around it, is still
//! a whole piece, and every test in `project_files_laws.rs` exists to keep
//! that true.
//!
//! Two nouns, and it is worth being clear which is which. A
//! [`ProjectSession`](crate::ProjectSession) is **one piece, open** — its
//! text, its history, its score, its sound. A [`Project`] is **the volume**:
//! which pieces there are, which one is in hand, and the order they are read
//! in. `Project` forwards nothing; a caller reaches the document with
//! [`Project::current_mut`] and issues the same commands it always did.

use std::path::{Path, PathBuf};

use musa_language::BarSpacing;
use serde::Deserialize;

use crate::contents::{ContentsFacts, Entry, EntryFacts, Layout};
use crate::error::ProjectError;
use crate::session::ProjectSession;
use crate::snapshot::ProjectSnapshot;
use crate::template::Template;

/// What a `musa.toml` says about the project a piece belongs to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProjectMeta {
    /// The directory the `musa.toml` sits in.
    pub root: PathBuf,
    /// The project's title, if it names one.
    pub name: Option<String>,
    /// Who wrote it, if it says.
    pub composer: Option<String>,
    /// The running order, if it sets one: project-relative paths, in the
    /// order the pieces are meant to be read (§16 — "an album manifest may
    /// specify ordering"). Files it does not name still appear, after these.
    pub pieces: Vec<String>,
    /// How this project wants its bars laid out. Not an `Option`: a project
    /// that says nothing has answered `Compact`, which is what every file
    /// written before the setting existed already is.
    pub bar_spacing: BarSpacing,
}

/// The `musa.toml` above `path`, if there is one.
///
/// The search walks up from the piece, which is what makes `pieces/01.musa`
/// find the project it is filed under. A malformed or unreadable file is no
/// project rather than an error: metadata that cannot be read costs the user
/// a title, not their session.
pub(crate) fn find(path: &Path) -> Option<ProjectMeta> {
    for directory in path.ancestors().skip(1) {
        if let Some(meta) = read(directory) {
            return Some(meta);
        }
    }
    None
}

/// The `musa.toml` in `directory`, if there is a readable one.
pub(crate) fn read(directory: &Path) -> Option<ProjectMeta> {
    let text = std::fs::read_to_string(directory.join("musa.toml")).ok()?;
    let file: ProjectFile = toml::from_str(&text).ok()?;
    Some(ProjectMeta {
        root: directory.to_path_buf(),
        name: file.project.name,
        composer: file.project.composer,
        pieces: file.project.pieces,
        bar_spacing: bar_spacing(file.format.bars.as_deref()),
    })
}

/// The layout a manifest asks for, or `Compact` if it asks for nothing this
/// version knows.
///
/// **A typo in `[format]` costs you the setting. Only a manifest that is not
/// TOML costs you the project.** The `.ok()?` above is a stated policy for
/// TOML that is not TOML, and extending it to a typed value here would mean a
/// misspelling in the newest and least important key destroys the oldest and
/// most important ones — the project's name, its composer and its running
/// order — and then `musa format` rewrites every file back. A string always
/// parses, so `[project]` survives and the interpretation happens here.
fn bar_spacing(written: Option<&str>) -> BarSpacing {
    match written {
        None | Some("compact") => BarSpacing::Compact,
        Some("proportional") => BarSpacing::Proportional,
        Some(other) => {
            tracing::warn!("musa.toml: unknown `[format] bars` value `{other}`; using `compact`");
            BarSpacing::Compact
        }
    }
}

#[derive(Deserialize)]
struct ProjectFile {
    #[serde(default)]
    project: ProjectSection,
    #[serde(default)]
    format: FormatSection,
}

#[derive(Default, Deserialize)]
struct ProjectSection {
    name: Option<String>,
    composer: Option<String>,
    #[serde(default)]
    pieces: Vec<String>,
}

/// The one thing a project may say about layout.
///
/// Typed as a string rather than as `BarSpacing` for the reason
/// [`bar_spacing`] gives. Nothing else joins it: the line width and the indent
/// are decided in `formatter.rs` at length, and exporting a solved problem
/// upward is the failure this section is shaped to avoid.
#[derive(Default, Deserialize)]
struct FormatSection {
    bars: Option<String>,
}

/// A project: the volume, and the pieces of it that are open.
///
/// Three ways in, and they collapse to one shape. Open a directory and you
/// get its running order with the first piece in hand. Open a `.musa` file
/// with a `musa.toml` above it and you get the project it is filed under,
/// with that piece in hand. Open a loose file and you get a project of one —
/// which is roadmap §16's own first sentence, and which is why there is no
/// second, single-piece mode anywhere above this type.
///
/// **There is always a piece in hand.** A folder with no piece in it is
/// refused when it is opened, not opened into an empty screen. You do not
/// open a volume in order to look at its cover.
///
/// **A piece you turn away from is left as it was.** Its text, its unsaved
/// edits and its undo history stay in the session that holds them, so turning
/// back is turning back rather than reopening. The one thing it gives up is
/// the audio device: only the piece in hand may sound.
pub struct Project {
    layout: Layout,
    /// The piece in hand, and the name the contents calls it by.
    ///
    /// Held apart from the rest rather than as an index into them, so that
    /// "there is always a piece in hand" is a fact about the type instead of
    /// an invariant somebody has to keep.
    hand: (String, ProjectSession),
    /// Pieces opened and turned away from, keeping their text, their unsaved
    /// edits, and their undo history.
    resting: Vec<(String, ProjectSession)>,
    /// The listing, rebuilt whenever something in it could have changed.
    contents: ContentsFacts,
}

impl Project {
    /// Open a project from a piece or a folder.
    ///
    /// # Errors
    /// [`ProjectError::Io`] if the piece cannot be read, and
    /// [`ProjectError::NoPieces`] if a folder holds none.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, ProjectError> {
        let path = path.as_ref();
        if path.is_dir() {
            let layout = Layout::read(path, read(path));
            let Some(first) = layout.pieces.first().map(|entry| entry.file.clone()) else {
                return Err(ProjectError::NoPieces {
                    root: path.display().to_string(),
                });
            };
            let session = ProjectSession::open(layout.locate(&first))?;
            return Ok(Self::around(layout, first, session));
        }
        Ok(Self::around_file(path, ProjectSession::open(path)?))
    }

    /// Create a new piece at `path` from `template`, and the project around
    /// it.
    ///
    /// # Errors
    /// [`ProjectError::Io`] if the file cannot be written.
    pub fn create(path: impl AsRef<Path>, template: Template) -> Result<Self, ProjectError> {
        let path = path.as_ref();
        Ok(Self::around_file(path, ProjectSession::create(path, template)?))
    }

    /// A new piece that has not been given a home yet, and the project of one
    /// around it.
    pub fn new_piece(template: Template, title: &str) -> Self {
        Self::from_text(template.source(title), format!("{title}.musa"))
    }

    /// A project of one piece with no file behind it — the scratch buffer a
    /// new, unsaved piece lives in, and what tests hold.
    pub fn from_text(source: impl Into<String>, name: impl Into<String>) -> Self {
        let name = name.into();
        let session = ProjectSession::from_text(source, name.clone());
        Self::around(Layout::unwritten(&name), name, session)
    }

    /// The piece in hand.
    pub fn current(&self) -> &ProjectSession {
        &self.hand.1
    }

    /// The piece in hand, to issue a command against.
    pub fn current_mut(&mut self) -> &mut ProjectSession {
        &mut self.hand.1
    }

    /// Which file is in hand, as the contents names it.
    pub fn showing(&self) -> &str {
        &self.hand.0
    }

    /// Turn to another file, opening it the first time.
    ///
    /// The piece left behind keeps everything except the audio device: only
    /// the piece in hand may sound, and a session that never played never
    /// opened one.
    ///
    /// # Errors
    /// [`ProjectError::Io`] if the file cannot be read.
    pub fn show(&mut self, file: &str) -> Result<(), ProjectError> {
        if self.hand.0 == file {
            return Ok(());
        }
        let taken = match self.resting.iter().position(|(name, _)| name == file) {
            Some(index) => self.resting.swap_remove(index),
            None => (file.to_owned(), ProjectSession::open(self.layout.locate(file))?),
        };
        self.hand.1.release_audio();
        self.resting.push(std::mem::replace(&mut self.hand, taken));
        self.rescan();
        Ok(())
    }

    /// The piece in hand's snapshot, with the project's contents in it.
    ///
    /// Restating the contents is part of taking the snapshot rather than
    /// something a caller has to remember, because `edited` has to be true
    /// while someone is typing and nobody should have to sequence that. It
    /// costs no filesystem and no parsing: the sessions already know what is
    /// unsaved, and the rest comes from the layout read at [`Self::rescan`].
    pub fn snapshot(&mut self) -> ProjectSnapshot<'_> {
        self.contents = self.describe();
        let mut snapshot = self.hand.1.snapshot();
        snapshot.contents = Some(&self.contents);
        snapshot
    }

    /// Write every piece that has unsaved edits.
    ///
    /// # Errors
    /// [`ProjectError::Io`] from the first piece that cannot be written. The
    /// ones before it stay saved, which is what "save all" should do when a
    /// disk goes read-only halfway through.
    pub fn save_all(&mut self) -> Result<(), ProjectError> {
        for (_, session) in std::iter::once(&mut self.hand).chain(&mut self.resting) {
            if session.snapshot().unsaved() {
                session.apply(crate::command::ProjectCommand::Save)?;
            }
        }
        self.rescan();
        Ok(())
    }

    /// Read the directory again.
    ///
    /// Call when the set of files, or what one of them is called, could have
    /// changed — after saving, or after opening. Not on every keystroke: this
    /// reads a directory and parses each piece's header, while the only thing
    /// that changes as someone types is what is unsaved, which
    /// [`Self::snapshot`] restates on its own.
    pub fn rescan(&mut self) {
        self.layout = self.layout.reread();
        let showing = self.hand.0.clone();
        self.layout.include(&showing);
        self.contents = self.describe();
    }

    // --- internals ---------------------------------------------------------

    /// The project around a session that already knows its file.
    fn around_file(path: &Path, session: ProjectSession) -> Self {
        // A directory is a project when it says so with a `musa.toml`, or
        // when you open the directory itself. Opening one loose piece must
        // not sweep in every unrelated file that happens to sit beside it.
        let layout = find(path).map_or_else(
            || Layout::loose(path),
            |meta| Layout::read(&meta.root.clone(), Some(meta)),
        );
        let file = layout.name_for(path);
        Self::around(layout, file, session)
    }

    fn around(mut layout: Layout, file: String, session: ProjectSession) -> Self {
        layout.include(&file);
        let mut project = Self {
            layout,
            hand: (file, session),
            resting: Vec::new(),
            contents: ContentsFacts::default(),
        };
        project.contents = project.describe();
        project
    }

    /// The listing as it stands: the layout's files, with what the open
    /// sessions know about them written in.
    fn describe(&self) -> ContentsFacts {
        let used: Vec<String> = self
            .hand
            .1
            .imports()
            .iter()
            .filter_map(|path| self.layout.listed(path))
            .collect();
        ContentsFacts {
            name: self.layout.title(),
            composer: self.layout.meta.as_ref().and_then(|meta| meta.composer.clone()),
            pieces: self
                .layout
                .pieces
                .iter()
                .map(|entry| self.state(entry, &used))
                .collect(),
            material: self
                .layout
                .material
                .iter()
                .map(|entry| self.state(entry, &used))
                .collect(),
        }
    }

    /// One listed file, with what the open sessions know about it written in.
    fn state(&self, entry: &Entry, used: &[String]) -> EntryFacts {
        let session = std::iter::once(&self.hand)
            .chain(&self.resting)
            .find(|(name, _)| *name == entry.file)
            .map(|(_, session)| session);
        EntryFacts {
            title: entry.name(),
            current: self.hand.0 == entry.file,
            unsaved: session.is_some_and(|session| session.snapshot().unsaved()),
            used: used.contains(&entry.file),
            file: entry.file.clone(),
        }
    }
}
