//! Source imports (roadmap §16): declarative, acyclic, deterministic, and
//! side-effect-free.
//!
//! A piece writes `import "../library/patches.musa";` and gets that file's
//! declarations — motifs, profiles, patches — as if it had written them
//! itself. Three rules keep this from growing into a package manager:
//!
//! 1. **The compiler does no I/O.** Import text arrives in
//!    [`ImportSources`], keyed by resolved path; whoever owns the filesystem
//!    (`musa-project`, the CLI) fills it. Compilation stays a pure function
//!    of its inputs, which is what makes it testable and cacheable.
//! 2. **An imported file is a `library`, not a piece.** The grammar says so,
//!    so "a score in an imported file" is a parse error rather than a rule
//!    somebody has to remember.
//! 3. **Names are flat.** An imported motif is called what it is called.
//!    Collisions are diagnostics, not shadowing — silently preferring one of
//!    two identically named motifs is the kind of thing that makes a piece
//!    sound different on someone else's machine.
//!
//! Relative paths are joined lexically, never canonicalized: `resolve` is a string
//! function, so the same import graph resolves the same way on every machine
//! and inside tests that have no files at all. The reserved `std::` namespace
//! instead maps to embedded, version-matched source under stable virtual URIs.

use std::collections::HashMap;
use std::fmt::Write as _;

use musa_language::ast::{AstNode as _, LibraryDecl};

use crate::diagnose::{Code, Diagnostic};
use crate::origin::SourceSpan;
use crate::resolve::Resolver;

/// The text of every file a compilation may import, by resolved path.
///
/// Empty by default: a piece that imports no local file needs no filesystem,
/// and one that does is handed exactly the files it named. Bundled standard
/// modules are available without being inserted here.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ImportSources {
    files: HashMap<String, String>,
}

impl ImportSources {
    /// Record one file's text under the path importers will name it by.
    pub fn insert(&mut self, path: impl Into<String>, text: impl Into<String>) {
        self.files.insert(path.into(), text.into());
    }

    /// The text of a resolved path, if it was provided.
    pub fn get(&self, path: &str) -> Option<&str> {
        self.files
            .get(path)
            .map(String::as_str)
            .or_else(|| standard_library_source(path))
    }

    /// Whether any file was provided at all.
    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }
}

/// Join `written` onto the directory holding `importer`, lexically.
///
/// `.` segments vanish and `..` pops the segment before it — with no
/// filesystem involved, so a path that climbs above the root simply keeps
/// its leading `..` and fails to resolve later, as a missing file.
#[must_use]
pub fn resolve_import(importer: &str, written: &str) -> String {
    if let Some(module) = written.strip_prefix("std::") {
        return format!("musa-stdlib:/std/{module}.musa");
    }
    if let Some((namespace, module)) = written.split_once("::") {
        return format!("musa-import:/{namespace}/{module}.musa");
    }
    // An absolute importer stays absolute: the root is not a segment that
    // `..` can climb past, it is where the path starts.
    let root = if importer.starts_with('/') { "/" } else { "" };
    let mut segments: Vec<&str> = Vec::new();
    let directory = importer.rsplit_once('/').map_or("", |(head, _)| head);
    for segment in directory.split('/').chain(written.split('/')) {
        match segment {
            "" | "." => {}
            ".." if matches!(segments.last(), Some(last) if *last != "..") => {
                segments.pop();
            }
            other => segments.push(other),
        }
    }
    format!("{root}{}", segments.join("/"))
}

/// Version of the source language expected by the embedded standard library.
pub const STANDARD_LIBRARY_LANGUAGE_VERSION: u32 = 1;

const COLLECTIONS_URI: &str = "musa-stdlib:/std/collections.musa";
const CORE_URI: &str = "musa-stdlib:/std/core.musa";
const HARMONY_URI: &str = "musa-stdlib:/std/harmony.musa";
const LIST_URI: &str = "musa-stdlib:/std/list.musa";
const OPTION_URI: &str = "musa-stdlib:/std/option.musa";
const PCSET_URI: &str = "musa-stdlib:/std/pcset.musa";
const PITCH_URI: &str = "musa-stdlib:/std/pitch.musa";
const SCALE_URI: &str = "musa-stdlib:/std/scale.musa";
const SERIAL_URI: &str = "musa-stdlib:/std/serial.musa";
const TONAL_HARMONY_URI: &str = "musa-stdlib:/std/tonal_harmony.musa";
const TRANSFORMATIONAL_URI: &str = "musa-stdlib:/std/transformational.musa";
const VOICING_URI: &str = "musa-stdlib:/std/voicing.musa";
const CONTEXT_URI: &str = "musa-stdlib:/std/context.musa";
const COLLECTIONS_SOURCE: &str = include_str!("../../../stdlib/collections.musa");
const CONTEXT_SOURCE: &str = include_str!("../../../stdlib/context.musa");
const CORE_SOURCE: &str = include_str!("../../../stdlib/core.musa");
const HARMONY_SOURCE: &str = include_str!("../../../stdlib/harmony.musa");
const LIST_SOURCE: &str = include_str!("../../../stdlib/list.musa");
const OPTION_SOURCE: &str = include_str!("../../../stdlib/option.musa");
const PCSET_SOURCE: &str = include_str!("../../../stdlib/pcset.musa");
const PITCH_SOURCE: &str = include_str!("../../../stdlib/pitch.musa");
const SCALE_SOURCE: &str = include_str!("../../../stdlib/scale.musa");
const SERIAL_SOURCE: &str = include_str!("../../../stdlib/serial.musa");
const TONAL_HARMONY_SOURCE: &str = include_str!("../../../stdlib/tonal_harmony.musa");
const TRANSFORMATIONAL_SOURCE: &str = include_str!("../../../stdlib/transformational.musa");
const VOICING_SOURCE: &str = include_str!("../../../stdlib/voicing.musa");
#[cfg(test)]
const MANIFEST: &str = include_str!("../../../stdlib/manifest.toml");

/// Source behind one readable virtual standard-library URI.
#[must_use]
pub fn standard_library_source(uri: &str) -> Option<&'static str> {
    match uri {
        COLLECTIONS_URI => Some(COLLECTIONS_SOURCE),
        CONTEXT_URI => Some(CONTEXT_SOURCE),
        CORE_URI => Some(CORE_SOURCE),
        HARMONY_URI => Some(HARMONY_SOURCE),
        LIST_URI => Some(LIST_SOURCE),
        OPTION_URI => Some(OPTION_SOURCE),
        PCSET_URI => Some(PCSET_SOURCE),
        PITCH_URI => Some(PITCH_SOURCE),
        SCALE_URI => Some(SCALE_SOURCE),
        SERIAL_URI => Some(SERIAL_SOURCE),
        TONAL_HARMONY_URI => Some(TONAL_HARMONY_SOURCE),
        TRANSFORMATIONAL_URI => Some(TRANSFORMATIONAL_SOURCE),
        VOICING_URI => Some(VOICING_SOURCE),
        _ => None,
    }
}

/// Every bundled module, in stable documentation and packaging order.
pub fn standard_library_modules() -> impl Iterator<Item = (&'static str, &'static str)> {
    [
        (COLLECTIONS_URI, COLLECTIONS_SOURCE),
        (CONTEXT_URI, CONTEXT_SOURCE),
        (CORE_URI, CORE_SOURCE),
        (HARMONY_URI, HARMONY_SOURCE),
        (LIST_URI, LIST_SOURCE),
        (OPTION_URI, OPTION_SOURCE),
        (PCSET_URI, PCSET_SOURCE),
        (PITCH_URI, PITCH_SOURCE),
        (SCALE_URI, SCALE_SOURCE),
        (SERIAL_URI, SERIAL_SOURCE),
        (TONAL_HARMONY_URI, TONAL_HARMONY_SOURCE),
        (TRANSFORMATIONAL_URI, TRANSFORMATIONAL_SOURCE),
        (VOICING_URI, VOICING_SOURCE),
    ]
    .into_iter()
}

/// Reference markdown derived from source comments.
///
/// The checked-in copy makes the library readable outside tooling; its law
/// test prevents prose and executable source from drifting.
#[must_use]
pub fn standard_library_reference() -> String {
    let mut out = String::from(
        "# Musa standard library 1\n\nThis reference is generated from the source comments in the bundled `.musa` modules. Standard definitions are ordinary\nMusa definitions; importing a module is explicit and never searches the filesystem.\n",
    );
    for (uri, source) in standard_library_modules() {
        let module = uri
            .strip_prefix("musa-stdlib:/std/")
            .and_then(|name| name.strip_suffix(".musa"))
            .unwrap_or("unknown");
        let _ = write!(out, "\n## `std::{module}`\n\n");
        let mut comments = Vec::new();
        // A signature's members are documented under it; a module's are not.
        // What a reader may write is `M.x` for each `x` the signature lists,
        // and everything else a module defines is private to it — so listing
        // a module's own lines would document what nobody can name.
        let mut requires: Option<&str> = None;
        let mut inside_module = false;
        for line in source.lines().map(str::trim) {
            if let Some(comment) = line.strip_prefix("// ") {
                comments.push(comment);
                continue;
            }
            if let Some(rest) = line.strip_prefix("signature ") {
                let name = rest.split([':', '(', ' ']).next().unwrap_or(rest);
                let _ = writeln!(out, "- `signature {name}` — {}", comments.join(" "));
                requires = Some(name);
                comments.clear();
                continue;
            }
            if let Some(rest) = line.strip_prefix("module ") {
                let head = rest.split_once(" {").map_or(rest, |(head, _)| head);
                let _ = writeln!(out, "- `module {head}` — {}", comments.join(" "));
                inside_module = true;
                comments.clear();
                continue;
            }
            if line == "}" {
                requires = None;
                inside_module = false;
                comments.clear();
                continue;
            }
            if inside_module {
                comments.clear();
                continue;
            }
            if let Some(signature) = line.strip_prefix("fn ").or_else(|| line.strip_prefix("let ")) {
                let signature = signature
                    .split('=')
                    .next()
                    .unwrap_or(signature)
                    .trim()
                    .trim_end_matches(';');
                match requires {
                    Some(named) => {
                        let _ = writeln!(out, "  - `{named}.{signature}` — {}", comments.join(" "));
                    }
                    None => {
                        let _ = writeln!(out, "- `{signature}` — {}", comments.join(" "));
                    }
                }
            }
            comments.clear();
        }
    }
    out
}

/// Every library a piece imports, transitively, in the order a reader would
/// reach them: a library's own imports before the library itself, so that a
/// declaration is always registered after the ones it may refer to.
pub(crate) struct Libraries {
    /// Kept alive so the AST nodes below stay valid.
    documents: Vec<musa_language::ParsedDocument>,
    /// One entry per imported file, in registration order.
    order: Vec<(String, usize, SourceSpan)>,
}

impl Libraries {
    /// The imported libraries, with the path each was read from.
    pub(crate) fn each(&self) -> impl Iterator<Item = (&str, LibraryDecl)> {
        self.order.iter().filter_map(|(path, index, _)| {
            let document = self.documents.get(*index)?;
            Some((path.as_str(), LibraryDecl::from_root(&document.syntax())?))
        })
    }

    /// The imported libraries and the local `use` span through which each
    /// was reached. Semantic consumers use this to remap a foreign failure
    /// instead of displaying another document's byte offsets in this one.
    pub(crate) fn each_with_import_span(&self) -> impl Iterator<Item = (&str, LibraryDecl, SourceSpan)> {
        self.order.iter().filter_map(|(path, index, span)| {
            let document = self.documents.get(*index)?;
            Some((path.as_str(), LibraryDecl::from_root(&document.syntax())?, *span))
        })
    }
}

/// Load the import closure of `imports`, reporting every way it can go wrong
/// against the `import` statement that caused it.
///
/// Diagnostics from inside a library name the file, because a span from
/// another document would point at the wrong bytes of this one.
///
/// Takes the statements rather than the piece because a library opened on its
/// own imports too, and checking one means reading what it builds on
/// (prompt 84).
pub(crate) fn load(
    resolver: &mut Resolver,
    importer: &str,
    imports: &[musa_language::ast::ImportStmt],
    sources: &ImportSources,
) -> Libraries {
    let mut loader = Loader {
        sources,
        libraries: Libraries {
            documents: Vec::new(),
            order: Vec::new(),
        },
        loaded: std::collections::HashSet::new(),
        stack: Vec::new(),
    };
    for import in imports {
        let span = crate::resolve::trimmed_span(import.syntax());
        let Some(written) = import.path() else { continue };
        loader.load_one(resolver, importer, &written, span);
    }
    loader.libraries
}

struct Loader<'a> {
    sources: &'a ImportSources,
    libraries: Libraries,
    /// Resolved paths already read, so a diamond resolves once and is shared.
    loaded: std::collections::HashSet<String>,
    /// The chain currently being read, for cycle reporting.
    stack: Vec<String>,
}

impl Loader<'_> {
    fn load_one(&mut self, resolver: &mut Resolver, importer: &str, written: &str, span: SourceSpan) {
        let path = resolve_import(importer, written);
        if self.stack.contains(&path) {
            let cycle = self
                .stack
                .iter()
                .map(String::as_str)
                .chain(std::iter::once(path.as_str()))
                .collect::<Vec<_>>()
                .join(" → ");
            resolver.report(
                Diagnostic::error(Code::Import, "these files import each other")
                    .at(span, "the loop closes here")
                    .note(format!("the loop is {cycle}"))
                    .help("move the shared material into a third library both can use"),
            );
            return;
        }
        if self.loaded.contains(&path) {
            return;
        }
        let Some(text) = self.sources.get(&path) else {
            resolver.report(
                Diagnostic::error(Code::Import, format!("cannot find `{path}`"))
                    .at(span, "no file here")
                    .help("paths are relative to the file that writes them, and end in `.musa`"),
            );
            return;
        };
        let document = musa_language::parse(text);
        if let Some(error) = document.errors().first() {
            resolver.report(
                Diagnostic::error(Code::Import, format!("`{path}` does not compile"))
                    .at(span, "imported here")
                    .note(format!("it says: {}", error.message()))
                    .help(format!("run `musa check {path}`")),
            );
            return;
        }
        let root = document.syntax();
        if LibraryDecl::from_root(&root).is_none() {
            resolver.report(
                Diagnostic::error(Code::Import, format!("`{path}` is a piece, not a library"))
                    .at(span, "only a library can be imported")
                    .help("wrap the material you want to share in `library { … }`"),
            );
            return;
        }
        self.loaded.insert(path.clone());
        // Depth first: a library's own imports are registered before it, so
        // whatever it builds on already exists by the time it is read.
        self.stack.push(path.clone());
        let nested: Vec<String> = LibraryDecl::from_root(&root)
            .map(|library| library.imports())
            .unwrap_or_default()
            .iter()
            .filter_map(musa_language::ast::ImportStmt::path)
            .collect();
        for import in nested {
            self.load_one(resolver, &path, &import, span);
        }
        self.stack.pop();
        self.libraries.documents.push(document);
        let index = self.libraries.documents.len().saturating_sub(1);
        self.libraries.order.push((path, index, span));
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use musa_language::ast::LibraryDecl;

    use super::*;

    #[test]
    fn bundled_manifest_and_sources_are_one_versioned_set() {
        assert!(
            MANIFEST.contains(&format!("language_version = {STANDARD_LIBRARY_LANGUAGE_VERSION}")),
            "the embedded source language and manifest must advance together"
        );
        for (uri, source) in standard_library_modules() {
            let module = uri
                .strip_prefix("musa-stdlib:/std/")
                .and_then(|name| name.strip_suffix(".musa"))
                .expect("standard URI shape");
            assert!(MANIFEST.contains(&format!("{module} = \"{module}.musa\"")));
            let parsed = musa_language::parse(source);
            assert!(parsed.errors().is_empty(), "{uri}: {:?}", parsed.errors());
            assert!(
                LibraryDecl::from_root(&parsed.syntax()).is_some(),
                "{uri} is not a library"
            );
        }
    }

    #[test]
    fn standard_imports_are_installation_independent() {
        assert_eq!(resolve_import("/a/piece.musa", "std::list"), LIST_URI);
        assert_eq!(resolve_import("elsewhere/piece.musa", "std::list"), LIST_URI);
        assert_eq!(resolve_import("album/piece.musa", "../shared.musa"), "shared.musa");
        assert_eq!(
            resolve_import("piece.musa", "vendor::list"),
            "musa-import:/vendor/list.musa"
        );
    }

    #[test]
    fn checked_in_reference_is_derived_from_executable_source() {
        if std::env::var_os("UPDATE_FIXTURES").is_some() {
            std::fs::write(
                concat!(env!("CARGO_MANIFEST_DIR"), "/../../stdlib/reference.md"),
                standard_library_reference(),
            )
            .expect("write the reference");
        }
        assert_eq!(
            standard_library_reference(),
            include_str!("../../../stdlib/reference.md"),
            "run the source-derived reference generator logic when comments or signatures change"
        );
    }
}
