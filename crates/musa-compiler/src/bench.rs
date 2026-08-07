//! Measurement seams for `benches/pipeline.rs` (roadmap §17.7).
//!
//! **This module exists for measurement only.** It is `#[doc(hidden)]`, it is
//! not part of the compiler's interface, and nothing in the workspace calls
//! it outside a benchmark. Its whole purpose is to let the phases `compile`
//! runs together be timed apart — parse, elaborate, project, canonicalize —
//! without any of them becoming a public stage that callers could sequence
//! themselves. An interface that grew because a benchmark wanted a seam
//! would be a benchmark leaking into a design.

use musa_kernel::{Occurrence, Timeline, overlay};

use crate::compile::{Compilation, CompileOptions, SourceDocument};
use crate::elaborate::VoiceTimeline;
use crate::lower::Lowering;

/// A parsed document, held so a benchmark can exclude parsing from its
/// measurement.
pub struct Parsed {
    document: musa_language::ParsedDocument,
    name: String,
}

/// Parse only — the stage P2 excludes.
pub fn parse(source: &SourceDocument) -> Parsed {
    Parsed {
        document: musa_language::parse(source.text()),
        name: source.name().to_string(),
    }
}

/// Everything after parsing: elaboration through the kernel and the snapshot
/// adapter (P2).
pub fn elaborate(parsed: &Parsed, options: &CompileOptions) -> Compilation {
    let mut lowering = Lowering::new();
    crate::elaborate::elaborate_parsed(&parsed.document, &parsed.name, options, &mut lowering)
}

/// The piece's voice timelines, kept for the projection and canonical-form
/// benchmarks. Producing them is elaboration; consuming them is P3 and P4.
pub struct Timelines {
    voices: Vec<VoiceTimeline>,
}

/// Elaborate, and keep the kernel timelines the adapter would have consumed.
pub fn timelines(parsed: &Parsed, options: &CompileOptions) -> Timelines {
    let mut lowering = Lowering::new();
    lowering.timeline_sink = Some(Vec::new());
    drop(crate::elaborate::elaborate_parsed(
        &parsed.document,
        &parsed.name,
        options,
        &mut lowering,
    ));
    Timelines {
        voices: lowering.timeline_sink.unwrap_or_default(),
    }
}

impl Timelines {
    /// How many occurrences the piece holds — the size of what P3 and P4
    /// walk, reported so a benchmark can state its workload.
    pub fn occurrences(&self) -> usize {
        self.voices.iter().map(|timeline| timeline.occurrences().len()).sum()
    }

    /// The snapshot projection (P3): the piece's timeline read back as score
    /// events. Returns the event count so the work cannot be optimized away.
    pub fn project(&self) -> usize {
        let mut lowering = Lowering::new();
        let piece = musa_kernel::overlay(self.voices.clone());
        crate::project::project(&mut lowering, &piece)
            .values()
            .map(|voice| voice.events.len())
            .sum()
    }

    /// Canonical form of the whole piece (P4): every voice overlaid into one
    /// timeline and normalized — the sort and the `Canonical` keys that
    /// prompt 43's semantic identity will pay on every edit.
    pub fn canonical(&self) -> usize {
        let piece: Timeline<_> = overlay(
            self.voices
                .iter()
                .map(|timeline| {
                    let occurrences: Vec<Occurrence<_>> = timeline
                        .occurrences()
                        .iter()
                        .map(|occurrence| Occurrence::new(occurrence.span(), occurrence.payload().clone()))
                        .collect();
                    musa_kernel::timeline(timeline.extent(), occurrences).unwrap_or_else(|_| {
                        // The occurrences came from a valid timeline, so this
                        // is unreachable; an empty timeline is the harmless
                        // answer rather than a panic in a benchmark.
                        musa_kernel::zero()
                    })
                })
                .collect(),
        );
        piece.normalize().occurrences().len()
    }
}
