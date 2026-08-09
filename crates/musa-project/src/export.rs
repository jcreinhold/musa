//! What the session can produce from the last score that compiled.

/// An export target.
///
/// Every variant renders from the last **successful** compilation, so an
/// export taken while the source is mid-edit produces the last coherent
/// score rather than nothing (roadmap §14.7).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ExportRequest {
    /// MEI — the interchange and engraving format.
    Mei,
    /// `LilyPond` source.
    LilyPond,
    /// `MusicXML` — the interchange format, for handing a piece to another
    /// notation program. Export only (§12.4).
    MusicXml,
    /// A deterministic offline audio render, 32-bit float stereo WAV.
    Wav,
    /// A Standard MIDI File. Two documents, not one setting: `Score` is the
    /// neutral reading, `Performance` the profiled one (roadmap §12.5).
    Midi(musa_render::MidiMode),
    /// The performance lowering, as a debug dump.
    PerformanceDump,
    /// The notation plan, as a debug dump.
    NotationPlanDump,
    /// The piece as kernel interchange text (docs/kernel/01).
    ///
    /// Unlike every other target this is a projection of the *document*, not
    /// of the score snapshot: a term carries the provenance the snapshot has
    /// already spent. It is still an export — one direction, never read back
    /// into a piece.
    Kernel {
        /// Print the term evaluated to a value rather than as written.
        /// Two decisions, taken in sequence: the printer never normalizes.
        normalized: bool,
    },
}

impl ExportRequest {
    /// The conventional file extension, so callers need no table of their own.
    pub fn extension(self) -> &'static str {
        match self {
            Self::Mei => "mei",
            Self::LilyPond => "ly",
            Self::MusicXml => "musicxml",
            Self::Wav => "wav",
            Self::Midi(_) => "mid",
            Self::PerformanceDump | Self::NotationPlanDump => "txt",
            // `sonata.musa.kernel`, the way `autosave` writes
            // `sonata.musa.recovery` and `realization` writes
            // `sonata.musa.performance`: a suffix musa adds beside a piece
            // says whose file it is. `.kernel` alone says nothing, and every
            // operating system and compiler already owns the word.
            Self::Kernel { .. } => "musa.kernel",
        }
    }
}

/// The result of an export, held in memory.
///
/// The session does not write files for exports: where the bytes go is the
/// caller's policy (stdout, a chosen path, a clipboard), and keeping it out
/// of the session is what lets the CLI and the desktop app share this code
/// without sharing a file-naming convention.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExportArtifact {
    body: Body,
    warnings: Vec<String>,
}

/// Text or bytes: which one an artifact is, and nothing else.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Body {
    Text(String),
    Bytes(Vec<u8>),
}

impl ExportArtifact {
    /// A textual artifact that lost nothing.
    pub(crate) fn text(text: impl Into<String>) -> Self {
        Self {
            body: Body::Text(text.into()),
            warnings: Vec::new(),
        }
    }

    /// A binary artifact that lost nothing.
    pub(crate) fn bytes(bytes: Vec<u8>) -> Self {
        Self {
            body: Body::Bytes(bytes),
            warnings: Vec::new(),
        }
    }

    pub(crate) fn warn(mut self, warnings: Vec<String>) -> Self {
        self.warnings = warnings;
        self
    }

    /// The artifact as raw bytes, whichever kind it is.
    pub fn as_bytes(&self) -> &[u8] {
        match &self.body {
            Body::Text(text) => text.as_bytes(),
            Body::Bytes(bytes) => bytes,
        }
    }

    /// The artifact as text, or `None` if it is binary.
    pub fn as_text(&self) -> Option<&str> {
        match &self.body {
            Body::Text(text) => Some(text),
            Body::Bytes(_) => None,
        }
    }

    /// What the target format could not say about this piece, once per kind.
    ///
    /// Empty for every export that lost nothing, which is nearly all of them.
    /// A format with no element for a written freedom carries the realized
    /// music and a text direction instead — a reading of the work rather than
    /// the work — and whoever asked for the file is told so here rather than
    /// finding out from a reader (`docs/kernel/07-backend-contract.md`).
    pub fn warnings(&self) -> &[String] {
        &self.warnings
    }
}

/// What reading a kernel file established: the piece it names, and what its
/// term denotes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KernelReport {
    /// The piece name the file declares.
    pub name: String,
    /// How many occurrences the term evaluates to.
    pub occurrences: usize,
    /// The evaluated timeline's extent, as an exact rational.
    pub extent: String,
    /// Which reading of the work the file projects, verbatim from its header
    /// (`docs/kernel/11-realization.md`). `None` when the file does not say —
    /// which a reader reports rather than guesses at, because a realization it
    /// cannot reproduce is the one thing that design exists to make visible.
    pub realization: Option<String>,
}

/// Read kernel interchange text: parse, check well-formedness (K7), evaluate.
///
/// The other direction from [`ExportRequest::Kernel`], and deliberately not a
/// session method — checking a file is not an operation on a project, and a
/// `.musa.kernel` file never becomes a document (AGENTS.md: the source is
/// canonical).
///
/// # Errors
///
/// The parse error positioned in the input, or the well-formedness violation.
pub fn check_kernel(text: &str) -> Result<KernelReport, crate::error::ProjectError> {
    musa_compiler::check_kernel_text(text)
        .map(|check| KernelReport {
            name: check.name,
            occurrences: check.occurrences,
            extent: check.extent,
            realization: check.realization,
        })
        .map_err(crate::error::ProjectError::Kernel)
}
