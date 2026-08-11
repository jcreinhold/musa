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

/// Options controlling compilation. Currently empty: defaults live inside.
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

/// Which of the two things a musa file may be (roadmap §16).
///
/// The grammar has always had both — a `piece` sounds, a `library` declares —
/// but the compiler used to accept only the first at a document's root, which
/// made "material has no score" indistinguishable from "this failed to
/// compile". Callers ask this when they need to tell those apart; callers that
/// only want a score still ask for one and still get `None`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DocumentKind {
    /// `piece "…" { … }` — the thing that has a score.
    #[default]
    Piece,
    /// `library { … }` — declarations for other files to import.
    Material,
    /// `% musa-kernel-1` — a kernel interchange file, read as itself
    /// (`docs/language/01-surface.md` §7).
    ///
    /// A third kind rather than a second flavour of `Piece`, because the two
    /// differ in what a caller may *do*: a kernel document has no surface
    /// syntax tree, so nothing that edits structure, completes a name, or
    /// renames a motif applies to it. It has a score, which is why it is not
    /// `Material` either.
    Kernel,
}

/// The result of compiling a document: diagnostics always, a snapshot and a
/// studio when no error-severity diagnostic was produced.
pub struct Compilation {
    kind: DocumentKind,
    snapshot: Option<ScoreSnapshot>,
    studio: crate::studio::StudioSpec,
    diagnostics: Vec<Diagnostic>,
    identity: musa_kernel::SemanticHash,
    decisions: Vec<crate::DecisionRecord>,
    references: crate::resolve::ReferenceIndex,
}

impl Compilation {
    pub(crate) fn new(snapshot: Option<ScoreSnapshot>, diagnostics: Vec<Diagnostic>) -> Self {
        Self {
            kind: DocumentKind::Piece,
            snapshot,
            studio: crate::studio::StudioSpec::default(),
            diagnostics,
            identity: musa_kernel::SemanticHash::default(),
            decisions: Vec::new(),
            references: crate::resolve::ReferenceIndex::new(),
        }
    }

    pub(crate) fn into_material(mut self) -> Self {
        self.kind = DocumentKind::Material;
        self
    }

    pub(crate) fn into_kernel(mut self) -> Self {
        self.kind = DocumentKind::Kernel;
        self
    }

    /// Which of the two things this document is (roadmap §16).
    ///
    /// Material compiles to no score and that is not a failure: a caller
    /// deciding what to *show* needs the distinction, because "no score yet"
    /// and "no score ever" are different screens.
    pub fn kind(&self) -> DocumentKind {
        self.kind
    }

    pub(crate) fn with_studio(mut self, studio: crate::studio::StudioSpec) -> Self {
        self.studio = studio;
        self
    }

    pub(crate) fn with_identity(mut self, identity: musa_kernel::SemanticHash) -> Self {
        self.identity = identity;
        self
    }

    pub(crate) fn with_decisions(mut self, decisions: Vec<crate::DecisionRecord>) -> Self {
        self.decisions = decisions;
        self
    }

    pub(crate) fn with_references(mut self, references: crate::resolve::ReferenceIndex) -> Self {
        self.references = references;
        self
    }

    /// Every name the resolver resolved, and everywhere it is spoken.
    ///
    /// This is the resolver's own knowledge kept, not a re-scan of the text:
    /// a name that did not resolve has no entry, and a name declared in an
    /// imported library has no declaration span here — its uses are honest
    /// and its home is `None`.
    pub fn references(&self) -> &[crate::resolve::NameReference] {
        self.references.entries()
    }

    /// What each checked declaration says about itself, for an editor to
    /// restate (`crate::docs`).
    ///
    /// Declarations, not names: a name used but never declared has a
    /// reference and no record, and an imported declaration has a record
    /// whether or not this document happens to speak its name. Unique by name
    /// and kind, so a consumer may look one up by the name a reference
    /// carries.
    pub fn items(&self) -> &[crate::docs::ItemDoc] {
        self.references.items()
    }

    /// Every decision this compilation took, in the order the sites were
    /// reached (`docs/kernel/11-realization.md`).
    ///
    /// The realization holds only what a composer *pinned*; this is what the
    /// piece actually asked and what it was answered, which is what a header
    /// line records and what the Origin view shows. A determinate piece
    /// returns nothing, under every seed.
    pub fn decisions(&self) -> &[crate::DecisionRecord] {
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
/// → temporal kernel → `ScoreSnapshot` adapter. There is one semantic path.
///
/// A document written in the *kernel* alternative
/// (`docs/language/01-surface.md` §7) joins that path later rather than
/// running beside it: it has no surface syntax to elaborate, so reading and
/// checking the term replaces everything up to the kernel, and the projection
/// and every backend after it are shared. Which alternative a text is, is a
/// question about its first line and is asked by `musa-language`.
pub fn compile(source: &SourceDocument, options: &CompileOptions) -> Compilation {
    match musa_language::alternative(source.text()) {
        musa_language::DocumentAlternative::Kernel => crate::kernel_text::compile_kernel(source),
        musa_language::DocumentAlternative::Surface => crate::elaborate::elaborate(source, options),
    }
}

/// Lay `text` out canonically, whichever alternative it is written in.
///
/// One entry because there is one question — "how should this document be
/// written down" — and two answers only because there are two languages. A
/// caller that dispatched itself would be a caller that could get the
/// dispatch wrong, and the whole toolchain formats through here.
///
/// Kernel text is laid out by the kernel's own printer, which is what makes
/// `musa format` idempotent on a file `musa kernel` produced. `None` means
/// the text cannot be read at all, and an unreadable document is left exactly
/// as its author has it.
pub fn format_document(text: &str, spacing: musa_language::BarSpacing) -> Option<String> {
    match musa_language::alternative(text) {
        musa_language::DocumentAlternative::Kernel => crate::kernel_text::format_kernel(text),
        musa_language::DocumentAlternative::Surface => {
            let document = musa_language::parse(text);
            Some(musa_language::format(&document, spacing).text().to_owned())
        }
    }
}
