//! Relative imports (roadmap §16): declarative, acyclic, local, and
//! side-effect-free.
//!
//! A piece writes `use "../library/patches.musa";` and gets that file's
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
//! Paths are joined lexically, never canonicalized: `resolve` is a string
//! function, so the same import graph resolves the same way on every machine
//! and inside tests that have no files at all.

use std::collections::HashMap;

use musa_language::ast::{AstNode as _, LibraryDecl, PieceDecl};

use crate::diagnose::{Code, Diagnostic};
use crate::origin::SourceSpan;
use crate::resolve::Resolver;

/// The text of every file a compilation may import, by resolved path.
///
/// Empty by default: a piece that imports nothing needs no filesystem, and a
/// piece that imports something is handed exactly the files it named.
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
        self.files.get(path).map(String::as_str)
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

/// Every library a piece imports, transitively, in the order a reader would
/// reach them: a library's own imports before the library itself, so that a
/// declaration is always registered after the ones it may refer to.
pub(crate) struct Libraries {
    /// Kept alive so the AST nodes below stay valid.
    documents: Vec<musa_language::ParsedDocument>,
    /// One entry per imported file, in registration order.
    order: Vec<(String, usize)>,
}

impl Libraries {
    /// The imported libraries, with the path each was read from.
    pub(crate) fn each(&self) -> impl Iterator<Item = (&str, LibraryDecl)> {
        self.order.iter().filter_map(|(path, index)| {
            let document = self.documents.get(*index)?;
            Some((path.as_str(), LibraryDecl::from_root(&document.syntax())?))
        })
    }
}

/// Load the import closure of `piece`, reporting every way it can go wrong
/// against the `use` statement that caused it.
///
/// Diagnostics from inside a library name the file, because a span from
/// another document would point at the wrong bytes of this one.
pub(crate) fn load(resolver: &mut Resolver, importer: &str, piece: &PieceDecl, sources: &ImportSources) -> Libraries {
    let mut loader = Loader {
        sources,
        libraries: Libraries {
            documents: Vec::new(),
            order: Vec::new(),
        },
        loaded: std::collections::HashSet::new(),
        stack: Vec::new(),
    };
    for import in piece.imports() {
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
        self.libraries.order.push((path, index));
    }
}
