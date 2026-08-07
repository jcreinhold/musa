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
    /// A deterministic offline audio render, 32-bit float stereo WAV.
    Wav,
    /// The performance lowering, as a debug dump.
    PerformanceDump,
    /// The notation plan, as a debug dump.
    NotationPlanDump,
}

impl ExportRequest {
    /// The conventional file extension, so callers need no table of their own.
    pub fn extension(self) -> &'static str {
        match self {
            Self::Mei => "mei",
            Self::LilyPond => "ly",
            Self::Wav => "wav",
            Self::PerformanceDump | Self::NotationPlanDump => "txt",
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
#[non_exhaustive]
pub enum ExportArtifact {
    /// A textual artifact.
    Text(String),
    /// A binary artifact.
    Bytes(Vec<u8>),
}

impl ExportArtifact {
    /// The artifact as raw bytes, whichever kind it is.
    pub fn as_bytes(&self) -> &[u8] {
        match self {
            Self::Text(text) => text.as_bytes(),
            Self::Bytes(bytes) => bytes,
        }
    }

    /// The artifact as text, or `None` if it is binary.
    pub fn as_text(&self) -> Option<&str> {
        match self {
            Self::Text(text) => Some(text),
            Self::Bytes(_) => None,
        }
    }
}
