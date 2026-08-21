//! What a project holds, as its contents page prints it.
//!
//! Roadmap §16 fixes the shape of a directory project — a `musa.toml` beside
//! `pieces/` and `library/` — and that shape is why this module lists rather
//! than walks. Two named kinds, two flat lists, read when something happens.
//! A tree would model a freedom the format does not have.
//!
//! The two lists are not a presentation choice either: a `library` cannot
//! contain a score, so material and repertoire are different kinds in the
//! grammar before they are different rows on a page.
//!
//! Every string here is decided in Rust, because `docs/rules/desktop/03-interaction.md`
//! §7 says the frontend spells no facts. A piece's `title` is what the piece
//! calls itself; the file name is what the filesystem calls it; and the page
//! sets the two in different faces precisely because they are two different
//! kinds of knowledge.

use std::path::{Path, PathBuf};

/// A project's running order and its shared material.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentsFacts {
    /// What to call the project: the manifest's name, else the folder's, else
    /// the piece's own file name.
    pub name: String,
    /// Who wrote it, if the manifest says.
    pub composer: Option<String>,
    /// The pieces, in the order they are meant to be read.
    pub pieces: Vec<EntryFacts>,
    /// The libraries the pieces draw on.
    pub material: Vec<EntryFacts>,
}

impl ContentsFacts {
    /// Whether there is more than one file to choose between.
    ///
    /// A project of one is a loose `.musa` file, and the interface shows it
    /// nothing: no contents, no running order, no menu item leading to a page
    /// with one line on it.
    #[must_use]
    pub fn is_a_volume(&self) -> bool {
        self.pieces.len().saturating_add(self.material.len()) > 1
    }
}

/// One file in a project.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EntryFacts {
    /// What the composer called it: a piece's own title, or — for material,
    /// and for a piece too broken to state one — the file name.
    pub title: String,
    /// The path relative to the project root, in `/` segments. This is the
    /// name a caller passes to turn to it.
    pub file: String,
    /// Whether this is the file in hand.
    pub current: bool,
    /// Whether it has edits that are not on disk.
    pub unsaved: bool,
    /// Material the current piece imports. Always false for a piece: what a
    /// piece reads is its own business, and its `import` statements already say.
    pub used: bool,
}

/// One file as the directory has it: where it is, and what it calls itself.
pub(crate) struct Entry {
    /// Project-relative, in `/` segments.
    pub(crate) file: String,
    /// The piece's own title, when it states one.
    pub(crate) title: Option<String>,
}

/// Where a project's files are, what order its pieces go in, and what each of
/// them is called.
///
/// Read from the disk when the set of files could have changed — opening,
/// turning to another piece, saving — and not on every keystroke: a title is
/// a parse, and nothing in this changes while someone is typing except which
/// pieces are unsaved, which the sessions answer without a filesystem.
pub(crate) struct Layout {
    /// The directory everything is relative to.
    pub(crate) root: PathBuf,
    /// The pieces, in the running order.
    pub(crate) pieces: Vec<Entry>,
    /// The libraries, in filename order.
    pub(crate) material: Vec<Entry>,
    /// The project's title, composer and running order, if a `musa.toml` said.
    pub(crate) meta: Option<crate::project::ProjectMeta>,
}

impl Layout {
    /// Read the layout of the project rooted at `root`.
    ///
    /// Pieces come from `pieces/` when there is one and from the root
    /// otherwise, so that both shapes §16 sketches — the bare folder and the
    /// filed album — read the same way here.
    pub(crate) fn read(root: &Path, meta: Option<crate::project::ProjectMeta>) -> Self {
        let piece_dir = if root.join("pieces").is_dir() {
            root.join("pieces")
        } else {
            root.to_path_buf()
        };
        let found = musa_files(&piece_dir, root);
        let ordered = order(found, meta.as_ref().map_or(&[], |meta| meta.pieces.as_slice()));
        Self {
            pieces: ordered.into_iter().map(|file| Entry::read(root, file)).collect(),
            material: musa_files(&root.join("library"), root)
                .into_iter()
                .map(|file| Entry::read(root, file))
                .collect(),
            root: root.to_path_buf(),
            meta,
        }
    }

    /// A project of exactly one file on disk.
    ///
    /// The simplest project is one file (§16), and it must behave exactly as
    /// it did before projects existed: one entry, so the interface shows
    /// nothing. A directory is a project when it says so with a `musa.toml`,
    /// or when you open the directory itself — opening one loose piece must
    /// not sweep in every unrelated file beside it.
    pub(crate) fn loose(file: &Path) -> Self {
        let root = file.parent().unwrap_or_else(|| Path::new(".")).to_path_buf();
        let name = file
            .file_name()
            .map_or_else(String::new, |name| name.to_string_lossy().into_owned());
        Self {
            pieces: vec![Entry::read(&root, name)],
            material: Vec::new(),
            root,
            meta: None,
        }
    }

    /// A project of one piece that has not been written anywhere yet.
    pub(crate) fn unwritten(name: &str) -> Self {
        Self {
            pieces: vec![Entry {
                file: name.to_owned(),
                title: None,
            }],
            material: Vec::new(),
            root: PathBuf::new(),
            meta: None,
        }
    }

    /// Read the directory again, keeping which project this is.
    pub(crate) fn reread(&self) -> Self {
        if self.root.as_os_str().is_empty() {
            return Self::unwritten(self.pieces.first().map_or("", |entry| entry.file.as_str()));
        }
        let meta = crate::project::read(&self.root).or_else(|| self.meta.clone());
        Self::read(&self.root, meta)
    }

    /// Make sure `file` is listed, wherever in the project it lives.
    ///
    /// A piece opened from a corner of the project that neither `pieces/` nor
    /// `library/` covers is still the piece in hand, and a contents page that
    /// omitted the file you are looking at would be wrong about the one row
    /// you can check.
    pub(crate) fn include(&mut self, file: &str) {
        if self.holds(file) {
            return;
        }
        self.pieces.push(Entry::read(&self.root, file.to_owned()));
    }

    fn holds(&self, file: &str) -> bool {
        self.pieces.iter().chain(&self.material).any(|entry| entry.file == file)
    }

    /// The path a project-relative name resolves to.
    pub(crate) fn locate(&self, file: &str) -> PathBuf {
        self.root.join(file)
    }

    /// What this project is called: the manifest's name, else the folder's,
    /// else the one piece's file name.
    pub(crate) fn title(&self) -> String {
        if let Some(name) = self.meta.as_ref().and_then(|meta| meta.name.clone()) {
            return name;
        }
        self.root.file_name().map_or_else(
            || self.pieces.first().map_or_else(String::new, |entry| entry.name()),
            |name| name.to_string_lossy().into_owned(),
        )
    }

    /// The project-relative name for a path, whether or not it is listed.
    pub(crate) fn name_for(&self, path: &Path) -> String {
        relative(path, &self.root).unwrap_or_else(|| {
            path.file_name()
                .map_or_else(String::new, |name| name.to_string_lossy().into_owned())
        })
    }

    /// The project-relative name for a path that the project actually holds.
    pub(crate) fn listed(&self, path: &Path) -> Option<String> {
        let name = relative(path, &self.root)?;
        self.holds(&name).then_some(name)
    }
}

impl Entry {
    fn read(root: &Path, file: String) -> Self {
        let title = title_of(&root.join(&file));
        Self { file, title }
    }

    /// What to print: the piece's own title, or the file's base name.
    pub(crate) fn name(&self) -> String {
        self.title
            .clone()
            .unwrap_or_else(|| self.file.rsplit('/').next().unwrap_or(&self.file).to_owned())
    }
}

/// Every `*.musa` file directly in `directory`, named relative to `root`, in
/// filename order.
///
/// Not recursive, and deliberately: §16's layout is two directories deep by
/// design, and a scan that descended would turn a project into a filesystem
/// browser with a musical vocabulary.
fn musa_files(directory: &Path, root: &Path) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(directory) else {
        return Vec::new();
    };
    let mut found: Vec<String> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_file() && path.extension().is_some_and(|extension| extension == "musa"))
        .filter_map(|path| relative(&path, root))
        .collect();
    found.sort();
    found
}

/// `path` as project-relative `/` segments, or `None` if it is not under
/// `root`.
fn relative(path: &Path, root: &Path) -> Option<String> {
    let relative = path.strip_prefix(root).ok()?;
    let segments: Vec<String> = relative
        .components()
        .map(|component| component.as_os_str().to_string_lossy().into_owned())
        .collect();
    Some(segments.join("/"))
}

/// The running order: what the manifest names, then everything else.
///
/// A file the manifest names and the directory does not have simply is not
/// there — the manifest sets an order, not a membership. A file the directory
/// has and the manifest does not name still appears, after the named ones,
/// because a contents page that hides a file is worse than one that admits a
/// gap.
fn order(found: Vec<String>, manifest: &[String]) -> Vec<String> {
    let mut ordered: Vec<String> = manifest
        .iter()
        .filter(|named| found.iter().any(|file| file == *named))
        .cloned()
        .collect();
    let rest: Vec<String> = found.into_iter().filter(|file| !ordered.contains(file)).collect();
    ordered.extend(rest);
    ordered
}

/// What a piece calls itself, read from its header.
///
/// The file is parsed rather than scanned: musa has one lexer and one parser,
/// and a second, looser reading of the same syntax is exactly the drift the
/// tree-sitter law exists to prevent. A file that cannot be read, or states no
/// title, is listed under its own name — which is what a composer would call
/// it anyway.
fn title_of(path: &Path) -> Option<String> {
    let source = std::fs::read_to_string(path).ok()?;
    let document = musa_syntax::parse(&source);
    let title = musa_syntax::ast::PieceDecl::of_document(&document.syntax())?.name()?;
    (!title.trim().is_empty()).then_some(title)
}
