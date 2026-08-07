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
use crate::resolve::Resolver;

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
    let mut resolver = Resolver::new();
    crate::elaborate::elaborate_parsed(&parsed.document, &parsed.name, options, &mut resolver)
}

/// The piece's voice timelines, kept for the projection and canonical-form
/// benchmarks. Producing them is elaboration; consuming them is P3 and P4.
pub struct Timelines {
    voices: Vec<VoiceTimeline>,
}

/// Elaborate, and keep the kernel timelines the adapter would have consumed.
pub fn timelines(parsed: &Parsed, options: &CompileOptions) -> Timelines {
    let mut resolver = Resolver::new();
    resolver.timeline_sink = Some(Vec::new());
    drop(crate::elaborate::elaborate_parsed(
        &parsed.document,
        &parsed.name,
        options,
        &mut resolver,
    ));
    Timelines {
        voices: resolver.timeline_sink.unwrap_or_default(),
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
        let mut resolver = Resolver::new();
        let piece = musa_kernel::overlay(self.voices.clone());
        crate::project::project(&mut resolver, &piece)
            .voices
            .values()
            .map(|voice| voice.events().len())
            .sum()
    }

    /// The semantic hash of the whole piece (P5): the digest prompt 43 makes
    /// the session ask for on every recompile. It canonicalizes exactly as
    /// P4 does and then absorbs the bytes, so P5 − P4 is the price of the
    /// identity itself.
    pub fn hash(&self) -> u128 {
        self.piece().semantic_hash().to_u128()
    }

    /// Every voice overlaid into one timeline — what P4 and P5 both start
    /// from.
    fn piece(&self) -> Timeline<crate::elaborate::ScoreFact> {
        overlay(
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
        )
    }

    /// Canonical form of the whole piece (P4): every voice overlaid into one
    /// timeline and normalized — the sort and the `Canonical` keys that
    /// prompt 43's semantic identity will pay on every edit.
    pub fn canonical(&self) -> usize {
        self.piece().normalize().occurrences().len()
    }
}
