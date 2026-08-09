//! Where a reading of an open work is kept between sessions (prompt 76).
//!
//! **In the project, never in the source.** A `.musa` file is the work; a
//! realization is one reading of it. Putting a seed in the source would make
//! two composers holding the same file unable to disagree about a
//! performance, which is the opposite of what open form is for
//! (`docs/kernel/11-realization.md`).
//!
//! So it lives beside the piece, in the place prompt 19 already keeps a
//! session's state on disk: a sibling file, in a directory the composer can
//! find, written the way the recovery copy is written. `sonata.musa` gets
//! `sonata.musa.performance`.
//!
//! The file is small on purpose — a seed and the decisions the composer kept
//! — and it is written in the same spellings the `.musa.kernel` header uses, so
//! the thing a composer reads in one place is the thing they read in the
//! other. A file that cannot be read is **no realization** rather than an
//! error: a corrupt sidecar should cost a performance, not a session.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use musa_compiler::{ChoicePath, Decision, Realization};
use serde::{Deserialize, Serialize};

/// The suffix a realization takes: `sonata.musa` → `sonata.musa.performance`.
const SUFFIX: &str = ".performance";

/// Where the realization for a piece lives.
pub(crate) fn path_for(path: &Path) -> PathBuf {
    let mut name = path.as_os_str().to_os_string();
    name.push(SUFFIX);
    PathBuf::from(name)
}

/// The file, as it is written.
///
/// `kept` rather than `pins`: what it holds is the decisions the composer
/// decided to keep, and the interface says "kept" for exactly this set.
#[derive(Debug, Default, Deserialize, Serialize)]
struct PerformanceFile {
    performance: u64,
    #[serde(default)]
    kept: BTreeMap<String, String>,
}

/// Read the realization beside `path`, or the deterministic one.
///
/// Absence is the common case and is not a failure: a piece that has never
/// been given a performance is compiled under seed zero with nothing pinned,
/// which is what every determinate piece gets and never consults.
pub(crate) fn read(path: &Path) -> Realization {
    let Ok(text) = std::fs::read_to_string(path_for(path)) else {
        return Realization::deterministic();
    };
    let Ok(file) = toml::from_str::<PerformanceFile>(&text) else {
        let target = path_for(path);
        tracing::warn!(path = %target.display(), "the realization beside this piece could not be read");
        return Realization::deterministic();
    };
    let mut realization = Realization::seeded(file.performance);
    for (site, decision) in &file.kept {
        // A line that names a site or an answer this build does not
        // understand is dropped, not refused: the rest of the reading is
        // still the composer's, and the site falls back to the seed.
        if let (Some(path), Some(decision)) = (ChoicePath::parse(site), Decision::parse(decision)) {
            realization.pin(path, decision);
        }
    }
    realization
}

/// Write the realization beside `path`, or remove it when there is nothing to
/// say.
///
/// "Nothing to say" is the deterministic realization: seed zero and no pins is
/// what a piece gets when it has never been performed, so leaving a file that
/// states it would be leaving a file that means the same as its absence.
///
/// Failure is logged rather than propagated, for the reason autosave's is: a
/// read-only directory should cost the composer a saved performance, not turn
/// a click into an error dialog.
pub(crate) fn write(path: &Path, realization: &Realization) {
    let target = path_for(path);
    if *realization == Realization::deterministic() {
        if let Err(error) = std::fs::remove_file(&target)
            && error.kind() != std::io::ErrorKind::NotFound
        {
            tracing::warn!(%error, path = %target.display(), "could not remove the realization");
        }
        return;
    }
    let file = PerformanceFile {
        performance: realization.seed(),
        kept: realization
            .taken()
            .map(|(site, decision)| (site.canonical(), decision.to_string()))
            .collect(),
    };
    let Ok(text) = toml::to_string(&file) else {
        return;
    };
    if let Err(error) = std::fs::write(&target, text) {
        tracing::warn!(%error, path = %target.display(), "could not write the realization");
    }
}
