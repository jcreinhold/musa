//! The piece term and its normal form, for golden snapshots.
//!
//! One concern of the `elaborate` module; see its docs for the semantic path.

use super::bars::resolve_meters;
use super::declaring;
use super::fact::{ScoreFact, VoiceTrack};
use super::place::{instantiate, placed, track_or_empty};
use super::score::{meter_of, stated};
use crate::compile::SourceDocument;
use crate::resolve::{self, Resolver};
use musa_events::WrittenTime;
use musa_score::score::ScoreSnapshot;
use musa_syntax::ast::AstNode as _;
use musa_syntax::ast::PieceDecl;

/// The normalized human-display text of a source's piece timeline, for golden
/// snapshots (docs/rules/events/05 N5). Semantic hashing uses separate framed N6
/// bytes. `None` when the
/// source does not elaborate cleanly.
///
/// One timeline, not one per part: a compilation has exactly
/// one temporal object, and the normal form is the text of that object —
/// key, meter, form markers and chord symbols included.
#[doc(hidden)]
pub fn events_normal_form(
    source: &SourceDocument,
    realization: &musa_score::Realization,
    imports: &crate::imports::ImportSources,
) -> Option<String> {
    let (_, term, _) = piece_term(source, realization, imports)?;
    Some(musa_events::evaluate_marked(term, instantiate).to_string())
}

/// The piece as a **term** (docs/rules/events/10): its name, and an `over` of one
/// literal per voice plus one for the piece-wide context.
///
/// One literal per voice and not one for the whole, because that shape is what
/// the interchange spelling is *for*: a reader of the text can see which lane a
/// fact belongs to without taking a scope apart, and the normalized spelling
/// ([`crate::events_normalized_text`]) is the one that collapses it. The sharing
/// the replaced elaborator wrapped around the `over` is gone with it: a motif
/// called from two voices is one definition of the *document* now, and its body
/// is shared where documents share things rather than in the events text.
pub(crate) fn piece_term(
    source: &SourceDocument,
    realization: &musa_score::Realization,
    imports: &crate::imports::ImportSources,
) -> Option<(
    String,
    musa_events::Term<WrittenTime, ScoreFact>,
    Vec<musa_score::DecisionRecord>,
)> {
    let document = musa_syntax::parse(source.text());
    if !document.errors().is_empty() {
        return None;
    }
    let root = document.syntax();
    let mut resolver = Resolver::new();
    resolver.realization = realization.clone();
    // A made piece is this document's piece, so the term of a document whose
    // piece is an instance is the term of what the instance makes.
    let mut templates = crate::template::Templates::collect(&mut resolver, &root);
    let made = musa_syntax::ast::MakeStmt::from_root(&root).and_then(|site| {
        templates.instance(
            &mut resolver,
            &site,
            "piece",
            crate::template::Kind::Piece,
            None,
            source.name(),
        )
    });
    let piece = PieceDecl::from_root(&root).or_else(|| made.as_ref().and_then(crate::template::Instance::piece))?;
    let mut snapshot = ScoreSnapshot::default();
    // The same closure full compilation reads: a `use` of imported material
    // is the piece's own music, and an export that could not name it would be
    // an export some pieces cannot make.
    let mut wanted = musa_syntax::ast::ImportStmt::all_at_root(&root);
    wanted.extend(piece.imports());
    let libraries = crate::imports::load(&mut resolver, source.name(), &wanted, imports);
    let sources = declaring(&root, &libraries, piece.syntax());
    let mut elaborated = crate::document::elaborate(&mut resolver, &sources, made.as_ref())?;
    resolve::lower_header(&mut resolver, &piece, &mut snapshot);
    let score = piece.score()?;
    resolve::register_bars(&mut resolver, &mut snapshot, &score);
    let read = elaborated.piece(&mut resolver, piece.syntax(), source.name())?;
    let sounding = elaborated.track(&read.track).ok()?;
    // The barlines, and only because the markers below are placed in them: this
    // helper answers a term rather than a diagnosis, so the meters are folded for
    // the coordinates they decide and the refusals go nowhere.
    let bars = resolve_meters(&mut resolver, stated(&sounding, meter_of));
    let mut parts = Vec::new();
    for part in &read.parts {
        for voice in &part.voices {
            let lane = elaborated.track(&voice.track).ok()?;
            // A voice's fold sounds the *piece's* and the *part's* facts too
            // — a `key` written mid-voice is the piece's from there — and the
            // piece layer below carries exactly those (`spoken_of` splits by
            // the same test). Printed in both, one fact would hash as two on
            // reparse: `a_piece_and_its_events_printing_have_one_meaning`.
            let own = lane
                .occurrences()
                .iter()
                .filter(|occurrence| occurrence.payload().scope.voice() == Some((part.id, voice.id)))
                .cloned()
                .collect();
            parts.push(musa_events::Term::literal(track_or_empty(lane.duration(), own)));
        }
    }
    let marked = placed(&mut resolver, &score, &bars, sounding.duration());
    parts.push(musa_events::Term::literal(spoken_of(&sounding, &marked)));
    let term = musa_events::Term::together(parts).ok()?;
    term.check().ok()?;
    Some((piece.name().unwrap_or_default(), term, resolver.decisions))
}

/// Everything the piece states about itself rather than about a voice, as one
/// track: the header's context, each part's, and the markers a coordinate
/// placed.
///
/// Split off the evaluated piece by the same test the projection buckets with —
/// a fact's scope names a voice or it does not — because [`piece_term`]'s shape
/// is one literal per voice and one for the piece, and each voice is a term of
/// its own already.
fn spoken_of(sounding: &VoiceTrack, marked: &VoiceTrack) -> VoiceTrack {
    let occurrences = sounding
        .occurrences()
        .iter()
        .filter(|occurrence| occurrence.payload().scope.voice().is_none())
        .chain(marked.occurrences().iter())
        .cloned()
        .collect();
    track_or_empty(sounding.duration(), occurrences)
}
