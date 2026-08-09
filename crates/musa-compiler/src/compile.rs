//! The compiler facade (roadmap §10.6, §15.3): one deep operation. Passes
//! (resolution, units, resolver) are private; callers see `Compilation`.

use crate::diagnose::{Diagnostic, Severity};
use crate::score::ScoreSnapshot;

/// A source document to compile.
pub struct SourceDocument {
    name: String,
    text: String,
}

impl SourceDocument {
    /// Create a document from text and a display name (used in diagnostics).
    pub fn new(text: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            text: text.into(),
        }
    }

    /// Load a document from a `.musa` file.
    ///
    /// # Errors
    /// Returns the I/O error when the file cannot be read as UTF-8.
    pub fn open(path: impl AsRef<std::path::Path>) -> std::io::Result<Self> {
        let path = path.as_ref();
        let text = std::fs::read_to_string(path)?;
        Ok(Self::new(text, path.to_string_lossy()))
    }

    /// The source text.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// The document name used in diagnostics.
    pub fn name(&self) -> &str {
        &self.name
    }
}

/// Options controlling compilation. Currently empty: defaults live inside
/// (12-TET tuning arrives with prompt 15's performance options).
#[derive(Clone, Debug, Default)]
pub struct CompileOptions {
    /// The text of every file this compilation may `use` (roadmap §16),
    /// keyed by the path an importer resolves to. The compiler reads no
    /// files: whoever owns the filesystem fills this in.
    pub imports: crate::imports::ImportSources,
    /// Which performance to compile.
    ///
    /// A piece that leaves nothing open never consults this, so the default —
    /// `Realization::deterministic()` — is the absence of a question rather
    /// than a choice of answer (`docs/kernel/11-realization.md`).
    pub realization: crate::realize::Realization,
}

/// The result of compiling a document: diagnostics always, a snapshot and a
/// studio when no error-severity diagnostic was produced.
pub struct Compilation {
    snapshot: Option<ScoreSnapshot>,
    studio: crate::studio::StudioSpec,
    diagnostics: Vec<Diagnostic>,
    identity: musa_kernel::SemanticHash,
    decisions: Vec<(crate::ChoicePath, crate::Decision)>,
    references: crate::resolve::ReferenceIndex,
}

impl Compilation {
    pub(crate) fn new(snapshot: Option<ScoreSnapshot>, diagnostics: Vec<Diagnostic>) -> Self {
        Self {
            snapshot,
            studio: crate::studio::StudioSpec::default(),
            diagnostics,
            identity: musa_kernel::SemanticHash::default(),
            decisions: Vec::new(),
            references: crate::resolve::ReferenceIndex::new(),
        }
    }

    pub(crate) fn with_studio(mut self, studio: crate::studio::StudioSpec) -> Self {
        self.studio = studio;
        self
    }

    pub(crate) fn with_identity(mut self, identity: musa_kernel::SemanticHash) -> Self {
        self.identity = identity;
        self
    }

    pub(crate) fn with_decisions(mut self, decisions: Vec<(crate::ChoicePath, crate::Decision)>) -> Self {
        self.decisions = decisions;
        self
    }

    pub(crate) fn with_references(mut self, references: crate::resolve::ReferenceIndex) -> Self {
        self.references = references;
        self
    }

    /// Every name the resolver resolved, and everywhere it is spoken
    /// (prompt 78).
    ///
    /// This is the resolver's own knowledge kept, not a re-scan of the text:
    /// a name that did not resolve has no entry, and a name declared in an
    /// imported library has no declaration span here — its uses are honest
    /// and its home is `None`.
    pub fn references(&self) -> &[crate::resolve::NameReference] {
        self.references.entries()
    }

    /// Every decision this compilation took, in the order the sites were
    /// reached (`docs/kernel/11-realization.md`).
    ///
    /// The realization holds only what a composer *pinned*; this is what the
    /// piece actually asked and what it was answered, which is what a header
    /// line records and what prompt 76 shows on the page. A determinate piece
    /// returns nothing, under every seed.
    pub fn decisions(&self) -> &[(crate::ChoicePath, crate::Decision)] {
        &self.decisions
    }

    /// What this compilation *means*, as a digest of the piece's timeline
    /// (docs/kernel/05 N6).
    ///
    /// Two compilations with the same identity are the same music, whatever
    /// their sources looked like; two with different identities differ in
    /// something a listener or an engraver would see. Provenance is part of
    /// the timeline's payloads, so moving a note's text without changing the
    /// note changes the identity — the question it answers is "is this the
    /// same compiled piece", not "does it sound the same".
    ///
    /// A compilation that produced no score has the identity of the empty
    /// piece, which is what a caller keying on it wants: nothing to install.
    pub fn identity(&self) -> musa_kernel::SemanticHash {
        self.identity
    }

    /// The compiled studio. Empty when the piece declares no `studio` block,
    /// which is the zero-setup case: every part keeps the default instrument
    /// (§14.8).
    pub fn studio(&self) -> &crate::studio::StudioSpec {
        &self.studio
    }

    /// The score and the studio together, consuming the compilation. They are
    /// two documents produced by one pass (§10.6), and the callers that
    /// render sound need both.
    pub fn into_parts(self) -> (Option<ScoreSnapshot>, crate::studio::StudioSpec) {
        (self.snapshot, self.studio)
    }

    /// The expanded score, if compilation succeeded.
    pub fn snapshot(&self) -> Option<&ScoreSnapshot> {
        self.snapshot.as_ref()
    }

    /// The score snapshot, consuming the compilation.
    pub fn into_snapshot(self) -> Option<ScoreSnapshot> {
        self.snapshot
    }

    /// All diagnostics (parse and semantic), in source order.
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    /// Whether any error-severity diagnostic was produced.
    pub fn has_errors(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity == Severity::Error)
    }
}

/// Compile `source` into a [`Compilation`] (roadmap §10.6, §15.3; course
/// correction §26).
///
/// Pipeline: parse → expansion-aware elaboration (motifs, repeat, transpose)
/// → temporal kernel → `ScoreSnapshot` adapter. There is one semantic path;
/// the direct lowerer that shadowed it through the kernel migration was
/// deleted at prompt 41.
pub fn compile(source: &SourceDocument, options: &CompileOptions) -> Compilation {
    crate::elaborate::elaborate(source, options)
}
