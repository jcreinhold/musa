//! Directory projects (roadmap §16), in their minimal form.
//!
//! A project is one file until it needs to share something. When it does, it
//! grows a directory with a `musa.toml` beside `pieces/` and `library/`, and
//! that file carries what the pieces have in common — the album's title, who
//! wrote it. It carries nothing a piece needs in order to compile: a piece
//! opened on its own, with no project around it, is still a whole piece.
//! That is the property this module exists to preserve, so it reads metadata
//! and stops.

use std::path::{Path, PathBuf};

use serde::Deserialize;

/// What a `musa.toml` says about the project a piece belongs to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProjectMeta {
    /// The directory the `musa.toml` sits in.
    pub root: PathBuf,
    /// The project's title, if it names one.
    pub name: Option<String>,
    /// Who wrote it, if it says.
    pub composer: Option<String>,
}

/// The `musa.toml` above `path`, if there is one.
///
/// The search walks up from the piece, which is what makes `pieces/01.musa`
/// find the project it is filed under. A malformed or unreadable file is no
/// project rather than an error: metadata that cannot be read costs the user
/// a title, not their session.
pub(crate) fn find(path: &Path) -> Option<ProjectMeta> {
    for directory in path.ancestors().skip(1) {
        let candidate = directory.join("musa.toml");
        let Ok(text) = std::fs::read_to_string(&candidate) else {
            continue;
        };
        let file: ProjectFile = toml::from_str(&text).ok()?;
        return Some(ProjectMeta {
            root: directory.to_path_buf(),
            name: file.project.name,
            composer: file.project.composer,
        });
    }
    None
}

#[derive(Deserialize)]
struct ProjectFile {
    #[serde(default)]
    project: ProjectSection,
}

#[derive(Default, Deserialize)]
struct ProjectSection {
    name: Option<String>,
    composer: Option<String>,
}
