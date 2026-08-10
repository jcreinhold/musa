//! `.musaignore` — the files a bulk format passes over.
//!
//! A formatter that walks a folder needs a way to be told "not this one",
//! because some files are shaped the way they are on purpose: fixtures a
//! generator writes and a test compares byte for byte, examples that are
//! deliberately malformed. The list lives beside the source rather than in the
//! command, so a bare `musa format` and the one CI runs agree about what they
//! are allowed to rewrite.
//!
//! The syntax is a small, deliberate subset of `.gitignore`:
//!
//! ```text
//! # comments and blank lines are skipped
//! tests/fixtures/         a path, and everything under it
//! *.generated.musa        `*` matches inside one path segment
//! examples/**/draft.musa  `**` matches across separators
//! build                   with no `/`, a name matched at any depth
//! ```
//!
//! Two departures from `.gitignore`, both stated rather than discovered: a
//! trailing `/` reads as documentation and is not a folders-only assertion,
//! and there is no negation. What matches a folder matches everything in it.
//!
//! A file named directly on the command line is always formatted. The list
//! governs what a walk *finds*, which is where a file gets rewritten by
//! accident; naming one is not an accident.

use std::path::{Path, PathBuf};

/// The `.musaignore` governing one walk.
pub(crate) struct Ignore {
    /// The walked folder as the caller spelled it, which is how every path the
    /// walk hands back begins.
    spelled: PathBuf,
    /// The walked folder's own path below the folder holding the list. It is
    /// the head of every path a pattern is matched against, so a list at the
    /// top of a tree means the same paths from anywhere inside it.
    within: PathBuf,
    patterns: Vec<Pattern>,
}

impl Ignore {
    /// The nearest `.musaignore` at or above `folder`, if there is one.
    ///
    /// The first one found wins outright, the way the nearest `musa.toml`
    /// does. A list that merges with the lists above it is a list you cannot
    /// read in one place, and the question here — is this file excluded? —
    /// should be answerable by opening one file.
    ///
    /// The search resolves `folder` to say where it sits; the *matching* then
    /// stays lexical, because a resolved path and a typed one need not share a
    /// single character — `/tmp` is `/private/tmp` — and only the typed one is
    /// what the walk hands back.
    pub(crate) fn found_at(folder: &Path) -> Self {
        let Ok(start) = std::fs::canonicalize(folder) else {
            return Self::nothing();
        };
        for directory in start.ancestors() {
            if let Ok(text) = std::fs::read_to_string(directory.join(".musaignore")) {
                return Self {
                    spelled: folder.to_path_buf(),
                    within: start
                        .strip_prefix(directory)
                        .unwrap_or_else(|_| Path::new(""))
                        .to_path_buf(),
                    patterns: text.lines().filter_map(Pattern::parse).collect(),
                };
            }
        }
        Self::nothing()
    }

    /// An empty list, which excludes nothing.
    fn nothing() -> Self {
        Self {
            spelled: PathBuf::new(),
            within: PathBuf::new(),
            patterns: Vec::new(),
        }
    }

    /// Whether the walk should pass this path over.
    pub(crate) fn excludes(&self, path: &Path) -> bool {
        if self.patterns.is_empty() {
            return false;
        }
        let Ok(under) = path.strip_prefix(&self.spelled) else {
            return false;
        };
        // Rebuilt from components rather than joined as text: the walk's paths
        // carry the `.` the caller typed, and `./broken` is `broken`.
        let relative = self
            .within
            .components()
            .chain(under.components())
            .map(|component| component.as_os_str().to_string_lossy())
            .collect::<Vec<_>>()
            .join("/");
        self.patterns.iter().any(|pattern| pattern.matches(&relative))
    }
}

/// One line of the file.
struct Pattern {
    glob: String,
    /// No `/` in it, so it names a file or folder at any depth.
    bare: bool,
}

impl Pattern {
    /// One line, or nothing if the line is a comment or blank.
    fn parse(line: &str) -> Option<Self> {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            return None;
        }
        let line = line.trim_end_matches('/');
        let line = line.strip_prefix("./").unwrap_or(line);
        if line.is_empty() {
            return None;
        }
        Some(Self {
            glob: line.to_owned(),
            bare: !line.contains('/'),
        })
    }

    fn matches(&self, relative: &str) -> bool {
        if self.bare {
            return relative.split('/').any(|component| glob(&self.glob, component));
        }
        // What matches a folder matches everything under it, so the path's own
        // prefixes are candidates alongside the path.
        prefixes(relative).any(|prefix| glob(&self.glob, prefix))
    }
}

/// `a/b/c` → `a`, `a/b`, `a/b/c`.
fn prefixes(relative: &str) -> impl Iterator<Item = &str> {
    relative
        .char_indices()
        .filter_map(|(at, character)| (character == '/').then(|| relative.get(..at)).flatten())
        .chain(std::iter::once(relative))
}

/// Whether `text` matches `pattern`, with `*` confined to one path segment and
/// `**` free to cross them.
fn glob(pattern: &str, text: &str) -> bool {
    let Some(head) = pattern.chars().next() else {
        return text.is_empty();
    };
    let rest = pattern.get(head.len_utf8()..).unwrap_or_default();
    if head != '*' {
        let Some(first) = text.chars().next() else {
            return false;
        };
        return head == first && glob(rest, text.get(first.len_utf8()..).unwrap_or_default());
    }
    let crossing = rest.starts_with('*');
    let rest = if crossing {
        rest.get(1..).unwrap_or_default()
    } else {
        rest
    };
    // `a/**/b` names `a/b` as well as `a/x/b`: the separator written after
    // `**` is one the text need not have.
    if crossing
        && let Some(after) = rest.strip_prefix('/')
        && glob(after, text)
    {
        return true;
    }
    let mut tail = text;
    loop {
        if glob(rest, tail) {
            return true;
        }
        let Some(next) = tail.chars().next() else {
            return false;
        };
        if next == '/' && !crossing {
            return false;
        }
        tail = tail.get(next.len_utf8()..).unwrap_or_default();
    }
}

#[cfg(test)]
mod tests {
    use super::glob;

    #[test]
    fn a_star_stays_inside_one_segment() {
        assert!(glob("*.musa", "canon.musa"));
        assert!(!glob("*.musa", "examples/canon.musa"));
        assert!(glob("lib-*", "lib-0"));
    }

    #[test]
    fn a_double_star_crosses_them() {
        assert!(glob("examples/**/draft.musa", "examples/a/b/draft.musa"));
        // The separator the pattern writes after `**` the text need not.
        assert!(glob("examples/**/draft.musa", "examples/draft.musa"));
        assert!(!glob("examples/**/draft.musa", "other/draft.musa"));
    }

    #[test]
    fn a_pattern_without_a_star_is_itself() {
        assert!(glob("tests/fixtures", "tests/fixtures"));
        assert!(!glob("tests/fixtures", "tests/fixtures/large-score.musa"));
    }
}
