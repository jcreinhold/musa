//! The compiler facade (roadmap §10.6, §15.3): one deep operation. Passes
//! (resolution, units, lowering) are private; callers see `Compilation`.

use crate::origin::SourceSpan;
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
    /// Which semantic path `compile` takes (course correction §30 Steps
    /// 5–6). The direct lowerer is the regression oracle until prompt 12
    /// makes the kernel path canonical.
    #[doc(hidden)]
    pub elaboration: Elaboration,
}

/// The semantic path used by [`compile`] (prompt 11/12 transition).
#[doc(hidden)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Elaboration {
    /// The direct CST→snapshot lowerer (prompts 05–06; the oracle).
    #[default]
    Direct,
    /// Elaboration through the temporal kernel (prompt 11).
    Kernel,
}

/// Diagnostic severity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Severity {
    /// Compilation failed for this construct; the snapshot may be absent.
    Error,
    /// Something was skipped or is suspicious; compilation continues.
    Warning,
}

/// A semantic diagnostic with an optional source span.
#[derive(Clone, Debug)]
pub struct Diagnostic {
    /// How bad it is.
    pub severity: Severity,
    /// What is wrong.
    pub message: String,
    /// Where, if tied to source.
    pub span: Option<SourceSpan>,
}

impl Diagnostic {
    pub(crate) fn error(message: impl Into<String>, span: Option<SourceSpan>) -> Self {
        Self {
            severity: Severity::Error,
            message: message.into(),
            span,
        }
    }

    pub(crate) fn warning(message: impl Into<String>, span: Option<SourceSpan>) -> Self {
        Self {
            severity: Severity::Warning,
            message: message.into(),
            span,
        }
    }
}

/// The result of compiling a document: diagnostics always, a snapshot when
/// no error-severity diagnostic was produced.
pub struct Compilation {
    snapshot: Option<ScoreSnapshot>,
    diagnostics: Vec<Diagnostic>,
}

impl Compilation {
    pub(crate) fn new(snapshot: Option<ScoreSnapshot>, diagnostics: Vec<Diagnostic>) -> Self {
        Self { snapshot, diagnostics }
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

/// Compile `source` into a [`Compilation`] (roadmap §10.6, §15.3).
///
/// Pipeline: parse → name resolution and unit checks → high-level model →
/// (motif expansion, prompt 06) → normalized `ScoreSnapshot`. Motif
/// applications, `transpose`, and `repeat` currently produce warnings and
/// are skipped (prompt 06 implements them).
pub fn compile(source: &SourceDocument, options: &CompileOptions) -> Compilation {
    match options.elaboration {
        Elaboration::Direct => crate::lower::lower(source),
        Elaboration::Kernel => crate::elaborate::elaborate(source),
    }
}
