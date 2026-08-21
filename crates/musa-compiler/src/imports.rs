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

use musa_syntax::ast::{AstNode as _, LibraryDecl};

use crate::package::Package;
use crate::resolve::Resolver;
use musa_score::diagnose::{Code, Diagnostic};
use musa_score::origin::SourceSpan;

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
        return standard_library_uri(module);
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

/// Every `.musa` file under `stdlib/src`, embedded by the build script.
///
/// A flat listing and nothing more: which of these files are *modules* is
/// [`crate::package`]'s answer, read from the `mod` declarations in the source
/// itself. A file that appears here and in no declaration is a fault, not a
/// module, which is the point of embedding the listing rather than a table.
mod embedded {
    include!(concat!(env!("OUT_DIR"), "/stdlib_files.rs"));
}

#[cfg(test)]
const MANIFEST: &str = include_str!("../../../stdlib/musa.toml");

/// The bundled library's module tree, read once from its own declarations.
fn standard_library() -> &'static Package<'static> {
    static PACKAGE: std::sync::LazyLock<Package<'static>> =
        std::sync::LazyLock::new(|| Package::read(embedded::STANDARD_LIBRARY_FILES));
    &PACKAGE
}

/// The virtual URI a bundled module path is readable at.
///
/// `std::tonal::harmony` lives at `musa-stdlib:/std/tonal/harmony.musa` —
/// the module path with its separators changed, so a reader who saw the import
/// can find the file and a reader who saw the URI can write the import.
fn standard_library_uri(path: &str) -> String {
    format!("musa-stdlib:/std/{}.musa", path.replace("::", "/"))
}

/// The module path a bundled URI names, if it is one.
fn standard_library_path(uri: &str) -> Option<String> {
    Some(
        uri.strip_prefix("musa-stdlib:/std/")?
            .strip_suffix(".musa")?
            .replace('/', "::"),
    )
}

/// Source behind one readable virtual standard-library URI.
#[must_use]
pub fn standard_library_source(uri: &str) -> Option<&'static str> {
    standard_library().module(&standard_library_path(uri)?)
}

/// The module path a bundled URI names, as the language spells it:
/// `musa-stdlib:/std/pitch.musa` is `pitch`.
///
/// What an interface titles the document with. The URI is a locator and reads
/// like one; a reader following `triad` to its declaration is looking at a
/// module, and the module has a name. Answered here rather than parsed by the
/// caller so that the URI scheme is spelled in exactly one place.
#[must_use]
pub fn standard_library_module(uri: &str) -> Option<String> {
    let path = standard_library_path(uri)?;
    standard_library().module(&path).map(|_| path)
}

/// Every bundled module, in stable documentation and packaging order.
pub fn standard_library_modules() -> impl Iterator<Item = (String, &'static str)> {
    standard_library()
        .modules()
        .map(|(path, source)| (standard_library_uri(path), source))
}

/// Everything wrong with the bundled library's own declarations.
///
/// Empty in any shipped build — its law test says so — but produced rather
/// than asserted, because the same reading is what a fetched package will be
/// held to and an assertion cannot be shown to a user.
pub(crate) fn standard_library_faults() -> Vec<Diagnostic> {
    standard_library()
        .faults()
        .iter()
        .map(|fault| {
            Diagnostic::error(Code::Import, fault.message())
                .help(fault.help())
                .note("a package's modules are its `mod` declarations; nothing is found by looking")
        })
        .collect()
}

/// Every library a piece imports, transitively, in the order a reader would
/// reach them: a library's own imports before the library itself, so that a
/// declaration is always registered after the ones it may refer to.
pub(crate) struct Libraries {
    /// Kept alive so the AST nodes below stay valid.
    documents: Vec<musa_syntax::ParsedDocument>,
    /// One entry per imported file, in registration order.
    order: Vec<Entry>,
}

/// One imported file, and how the statement that reached it was written.
struct Entry {
    path: String,
    document: usize,
    span: SourceSpan,
    alias: Option<String>,
}

/// One library as the importing document sees it.
///
/// The path is where the text came from; the qualifier is what the importer
/// wrote after `as`, and it is `None` on nearly every import because binding
/// is flat. Both travel together because a name and where it came from are
/// the same question asked twice.
///
/// `at` is the `import` statement itself, and it travels with them for the
/// reason [`crate::document::Source::from`] gives: every diagnostic about an
/// imported declaration is stated *here*, so the span it needs is the one span
/// in this document that is about that library. Carrying it beside the path
/// rather than beside the iterator is what lets a consumer that took the path
/// report against it without the loader handing it a second value to keep in
/// step.
#[derive(Clone, Copy)]
pub(crate) struct Imported<'a> {
    pub(crate) path: &'a str,
    pub(crate) qualifier: Option<&'a str>,
    pub(crate) at: SourceSpan,
}

impl Libraries {
    /// The imported libraries, with the path each was read from.
    pub(crate) fn each(&self) -> impl Iterator<Item = (Imported<'_>, LibraryDecl)> {
        self.order.iter().filter_map(|entry| {
            let document = self.documents.get(entry.document)?;
            Some((entry.imported(), LibraryDecl::from_root(&document.syntax())?))
        })
    }
}

impl Entry {
    fn imported(&self) -> Imported<'_> {
        Imported {
            path: &self.path,
            qualifier: self.alias.as_deref(),
            at: self.span,
        }
    }
}

/// Load the import closure of `imports`, reporting every way it can go wrong
/// against the `import` statement that caused it.
///
/// Diagnostics from inside a library name the file, because a span from
/// another document would point at the wrong bytes of this one.
///
/// Takes the statements rather than the piece because a library opened on its
/// own imports too, and checking one means reading what it builds on.
pub(crate) fn load(
    resolver: &mut Resolver,
    importer: &str,
    imports: &[musa_syntax::ast::ImportStmt],
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
    let mut reported_package_faults = false;
    // A syntax import is not an ordinary import and does not bring a module's
    // declarations into this file. It named a reader, the expansion phase has
    // already used it (`crate::expand`), and loading its module here would ask
    // the ordinary checker to read a phase module.
    for import in imports.iter().filter(|import| !import.changes_syntax()) {
        let span = crate::resolve::trimmed_span(import.syntax());
        let Some(written) = import.path() else { continue };
        // A piece that reads the bundled library is told what is wrong with
        // it, once. A package whose declarations and files disagree cannot
        // answer honestly for any of its modules, so saying so at the first
        // import beats letting each one fail separately with a smaller reason.
        if written.starts_with("std::") && !reported_package_faults {
            reported_package_faults = true;
            for fault in standard_library_faults() {
                resolver.report(fault.at(span, "the bundled library is imported here"));
            }
        }
        loader.load_one(resolver, importer, &written, span, import.alias());
    }
    // "Cannot find `x`" is nearly always a question about which files were
    // read, and the closure is transitive, so the written paths a reader can
    // see are not the whole answer. `order` is already built.
    tracing::debug!(
        phase = "imports",
        written = imports.len(),
        loaded = loader.libraries.order.len(),
        "loaded the import closure"
    );
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
    /// `alias` is the importing statement's `as` clause, which belongs to that
    /// statement and not to the file: a library reached transitively is bound
    /// as it was written, however the piece at the top qualified its own.
    fn load_one(
        &mut self,
        resolver: &mut Resolver,
        importer: &str,
        written: &str,
        span: SourceSpan,
        alias: Option<String>,
    ) {
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
        let document = musa_syntax::parse(text);
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
        // Import resolution turns a written path into a real one against the
        // importing file, and that mapping is the whole of what a reader
        // cannot see from the source.
        tracing::trace!(written, importer, resolved = %path, "resolved an import");
        self.loaded.insert(path.clone());
        // Depth first: a library's own imports are registered before it, so
        // whatever it builds on already exists by the time it is read.
        self.stack.push(path.clone());
        let nested: Vec<String> = LibraryDecl::from_root(&root)
            .map(|library| library.imports())
            .unwrap_or_default()
            .iter()
            .filter(|import| !import.changes_syntax())
            .filter_map(musa_syntax::ast::ImportStmt::path)
            .collect();
        for import in nested {
            self.load_one(resolver, &path, &import, span, None);
        }
        self.stack.pop();
        self.libraries.documents.push(document);
        let document = self.libraries.documents.len().saturating_sub(1);
        self.libraries.order.push(Entry {
            path,
            document,
            span,
            alias,
        });
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use musa_syntax::ast::LibraryDecl;

    use super::*;

    #[test]
    fn bundled_manifest_and_sources_are_one_versioned_set() {
        assert!(
            MANIFEST.contains(&format!("language_version = {STANDARD_LIBRARY_LANGUAGE_VERSION}")),
            "the embedded source language and manifest must advance together"
        );
        for (uri, source) in standard_library_modules() {
            let parsed = musa_syntax::parse(source);
            assert!(parsed.errors().is_empty(), "{uri}: {:?}", parsed.errors());
            assert!(
                LibraryDecl::from_root(&parsed.syntax()).is_some(),
                "{uri} is not a library"
            );
        }
    }

    /// The law the four parallel lists could not state: every file in the
    /// package is declared, and every declaration reaches a file.
    #[test]
    fn the_bundled_library_declares_exactly_the_files_it_has() {
        let faults: Vec<String> = standard_library()
            .faults()
            .iter()
            .map(super::super::package::Fault::message)
            .collect();
        assert!(faults.is_empty(), "{faults:?}");
    }

    #[test]
    fn a_nested_module_path_reads_as_a_nested_file() {
        assert_eq!(
            resolve_import("p.musa", "std::tonal::harmony"),
            "musa-stdlib:/std/tonal/harmony.musa"
        );
        assert!(
            standard_library_source("musa-stdlib:/std/tonal/harmony.musa").is_some(),
            "and the file is there to read"
        );
    }

    #[test]
    fn standard_imports_are_installation_independent() {
        let list = "musa-stdlib:/std/list.musa";
        assert_eq!(resolve_import("/a/piece.musa", "std::list"), list);
        assert_eq!(resolve_import("elsewhere/piece.musa", "std::list"), list);
        assert_eq!(resolve_import("album/piece.musa", "../shared.musa"), "shared.musa");
        assert_eq!(
            resolve_import("piece.musa", "vendor::list"),
            "musa-import:/vendor/list.musa"
        );
    }
}
