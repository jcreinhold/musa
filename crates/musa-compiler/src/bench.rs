//! Measurement seams for `benches/pipeline.rs` (roadmap §17.7).
//!
//! **This module exists for measurement only.** It is `#[doc(hidden)]`, it is
//! not part of the compiler's interface, and nothing in the workspace calls
//! it outside a benchmark. Its whole purpose is to let the phases `compile`
//! runs together be timed apart — parse, elaborate, project, canonicalize —
//! without any of them becoming a public stage that callers could sequence
//! themselves. An interface that grew because a benchmark wanted a seam
//! would be a benchmark leaking into a design.

use musa_events::{EventTrack, Occurrence, WrittenTime, together};

use crate::compile::{Compilation, CompileOptions, SourceDocument};
use crate::elaborate::VoiceTrack;
use crate::resolve::Resolver;

/// How a sharing workload writes the same music (prompt 127).
///
/// The three shapes denote different pieces on purpose — comparing them is
/// how the two sharing gaps are priced. [`Sharing::Identical`] is the
/// call-site gap: every call denotes the same body and the compiler must
/// decide whether it elaborates it once. [`Sharing::Distinct`] and
/// [`Sharing::Hoisted`] are the full-laziness gap: the same music, written
/// with the argument-independent tail inside the parameterized body and
/// written beside it.
/// How many distinct arguments [`sharing_source`] has to spend: seven letters
/// over five octaves.
pub const DISTINCT_ROOTS: usize = 35;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sharing {
    /// `calls` calls of one parameterless motif, at `calls` distinct sites.
    Identical,
    /// `calls` calls of one motif whose first note is its argument and whose
    /// remaining `body` notes are not.
    Distinct,
    /// The same music as [`Sharing::Distinct`], with the argument-independent
    /// notes hoisted by hand into a motif of their own.
    Hoisted,
}

/// A generated piece whose sharing behaviour is the whole point.
///
/// `calls` is the number of call sites and `body` the number of
/// argument-independent notes in the body, so the duplicated work a compiler
/// could avoid is `calls * body` notes and the music itself is fixed by the
/// pair. Generated rather than committed because the workload is a *curve*:
/// one row of it would say nothing about how the cost scales.
///
/// The arguments walk seven letters over five octaves, so a workload of more
/// than [`DISTINCT_ROOTS`] calls repeats arguments. That is not a shortcut:
/// the written range is finite and a real piece calling one motif five
/// hundred times is calling it on the same notes more than once.
pub fn sharing_source(shape: Sharing, calls: usize, body: usize) -> String {
    use std::fmt::Write as _;

    const LETTERS: [&str; 7] = ["c", "d", "e", "f", "g", "a", "b"];
    const OCTAVES: [&str; 5] = ["2", "3", "4", "5", "6"];
    let letter = |index: usize| LETTERS.get(index % 7).copied().unwrap_or("c");
    let octave = |index: usize| OCTAVES.get((index / 7) % 5).copied().unwrap_or("4");
    let tail = |source: &mut String| {
        for note in 0..body {
            let _ = write!(source, " {}5/16", letter(note));
        }
    };

    let mut source = String::from("piece \"Sharing\" {\n    meter 4/4;\n    key c major;\n\n");
    match shape {
        Sharing::Identical => {
            source.push_str("    motif cell() {");
            tail(&mut source);
            source.push_str(" }\n");
        }
        Sharing::Distinct => {
            source.push_str("    motif cell(root: Pitch) { root/16");
            tail(&mut source);
            source.push_str(" }\n");
        }
        Sharing::Hoisted => {
            source.push_str("    motif head(root: Pitch) { root/16 }\n    motif tail() {");
            tail(&mut source);
            source.push_str(" }\n");
        }
    }
    source.push_str("\n    score {\n        part p {\n            voice v {\n");
    for call in 0..calls {
        let root = format!("{}{}", letter(call), octave(call));
        match shape {
            Sharing::Identical => source.push_str("                use cell();\n"),
            Sharing::Distinct => {
                let _ = writeln!(source, "                use cell({root});");
            }
            Sharing::Hoisted => {
                let _ = writeln!(source, "                use head({root});");
                source.push_str("                use tail();\n");
            }
        }
    }
    source.push_str("            }\n        }\n    }\n}\n");
    source
}

/// A parsed document, held so a benchmark can exclude parsing from its
/// measurement.
pub struct Parsed {
    document: musa_syntax::ParsedDocument,
    name: String,
}

/// Parse only — the stage P2 excludes.
pub fn parse(source: &SourceDocument) -> Parsed {
    Parsed {
        document: musa_syntax::parse(source.text()),
        name: source.name().to_string(),
    }
}

/// Everything after parsing: elaboration through the event track and the snapshot
/// adapter (P2).
pub fn elaborate(parsed: &Parsed, options: &CompileOptions) -> Compilation {
    let mut resolver = Resolver::new();
    crate::elaborate::elaborate_parsed(&parsed.document, &parsed.name, options, &mut resolver)
}

/// The piece's voice tracks, kept for the projection and canonical-form
/// benchmarks. Producing them is elaboration; consuming them is P3 and P4.
pub struct Tracks {
    voices: Vec<VoiceTrack>,
}

/// Elaborate, and keep the event track tracks the adapter would have consumed.
pub fn tracks(parsed: &Parsed, options: &CompileOptions) -> Tracks {
    let mut resolver = Resolver::new();
    resolver.track_sink = Some(Vec::new());
    drop(crate::elaborate::elaborate_parsed(
        &parsed.document,
        &parsed.name,
        options,
        &mut resolver,
    ));
    Tracks {
        voices: resolver.track_sink.unwrap_or_default(),
    }
}

impl Tracks {
    /// How many occurrences the piece holds — the size of what P3 and P4
    /// walk, reported so a benchmark can state its workload.
    pub fn occurrences(&self) -> usize {
        self.voices.iter().map(|track| track.occurrences().len()).sum()
    }

    /// The snapshot projection (P3): the piece's track read back as score
    /// events. Returns the event count so the work cannot be optimized away.
    pub fn project(&self) -> usize {
        let mut resolver = Resolver::new();
        let piece = together(self.voices.clone());
        crate::project::project(&mut resolver, &piece)
            .voices
            .values()
            .map(|voice| voice.events().len())
            .sum()
    }

    /// The semantic hash of the whole piece (P5): the digest the session
    /// asks for on every recompile. It canonicalizes exactly as
    /// P4 does and then absorbs the bytes, so P5 − P4 is the price of the
    /// identity itself.
    pub fn hash(&self) -> u128 {
        self.piece().semantic_hash().to_u128()
    }

    /// Every voice stacked into one track — what P4 and P5 both start
    /// from.
    fn piece(&self) -> EventTrack<WrittenTime, crate::elaborate::ScoreFact> {
        together(
            self.voices
                .iter()
                .map(|track| {
                    let occurrences: Vec<Occurrence<_, _>> = track
                        .occurrences()
                        .iter()
                        .map(|occurrence| Occurrence::new(occurrence.span(), occurrence.payload().clone()))
                        .collect();
                    musa_events::track(track.duration(), occurrences).unwrap_or_else(|_| {
                        // The occurrences came from a valid track, so this
                        // is unreachable; an empty track is the harmless
                        // answer rather than a panic in a benchmark.
                        musa_events::empty(musa_events::Duration::ZERO)
                    })
                })
                .collect(),
        )
    }

    /// Canonical form of the whole piece (P4): every voice overlaid into one
    /// track and normalized — the sort and the `Canonical` keys that
    /// semantic identity pays on every edit.
    pub fn canonical(&self) -> usize {
        self.piece().normalize().occurrences().len()
    }
}
