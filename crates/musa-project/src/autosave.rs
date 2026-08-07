//! Autosave and crash recovery (roadmap §15.7).
//!
//! The policy is one sentence: **while the source differs from what is on
//! disk, a recovery copy of it sits beside the file; saving writes the file
//! and removes the copy.** So the only state that can survive a crash is a
//! piece of text the composer typed, in a file they can open, next to the
//! file it belongs to — and a session that closed cleanly leaves nothing
//! behind at all.
//!
//! **It is not debounced here, deliberately.** A recovery write costs one
//! `write` and one `rename` of a few kilobytes, and it happens once per
//! command that changes the source — which is already a settled edit, because
//! the editor debounces keystrokes before they reach the session exactly as
//! roadmap §10.7 says it must. Adding a second timer inside the session would
//! mean owning a clock and a thread to answer a question the caller has
//! already answered.
//!
//! **The write is atomic.** A crash during the write must not replace a good
//! recovery copy with half a file, so the copy is written to a temporary
//! sibling and renamed over the old one — `rename` within a directory is
//! atomic on every platform musa targets.
//!
//! The undo history is not saved. It is a session's working memory, not the
//! document, and a recovered file that claimed a history it could no longer
//! reach would be lying about what undo would do.

use std::path::{Path, PathBuf};

/// The suffix a recovery copy takes: `sonata.musa` → `sonata.musa.recovery`.
///
/// Beside the file rather than in a state directory, because a composer who
/// loses power should find their work where their work is, without being
/// told about an application data folder.
const SUFFIX: &str = ".recovery";

/// Where the recovery copy for a piece lives.
pub(crate) fn path_for(path: &Path) -> PathBuf {
    let mut name = path.as_os_str().to_os_string();
    name.push(SUFFIX);
    PathBuf::from(name)
}

/// Write the recovery copy, atomically.
///
/// Failure is not propagated to the composer: autosave is a safety net, and a
/// read-only directory should not turn every keystroke into an error dialog.
/// It is logged, because a net that is not catching is worth knowing about.
pub(crate) fn write(path: &Path, source: &str) {
    let target = path_for(path);
    let mut temporary = target.clone().into_os_string();
    temporary.push(".tmp");
    let temporary = PathBuf::from(temporary);
    let written = std::fs::write(&temporary, source).and_then(|()| std::fs::rename(&temporary, &target));
    if let Err(error) = written {
        std::fs::remove_file(&temporary).ok();
        tracing::warn!(%error, path = %target.display(), "could not write the recovery copy");
    }
}

/// Remove the recovery copy. Absence is success: there is nothing to recover.
pub(crate) fn clear(path: &Path) {
    let target = path_for(path);
    match std::fs::remove_file(&target) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => tracing::warn!(%error, path = %target.display(), "could not remove the recovery copy"),
    }
}

/// Work left behind by a session that did not close cleanly, if it differs
/// from what is on disk.
///
/// A copy identical to the file is not a recovery — it is a save that landed
/// and a copy that did not get cleared — so it is removed rather than offered.
pub(crate) fn take(path: &Path, on_disk: &str) -> Option<String> {
    let recovered = std::fs::read_to_string(path_for(path)).ok()?;
    if recovered == on_disk {
        clear(path);
        return None;
    }
    Some(recovered)
}
