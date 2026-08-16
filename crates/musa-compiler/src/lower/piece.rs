//! A written piece, as the track it denotes.
//!
//! [`super::notation`] is `00-semantics.md` §3's first composition equation —
//! `follow` over a block's statements. This module is the second one, over a
//! score's voices:
//!
//! ```text
//! together(v₁, together(v₂, … together(vₙ, context)))
//! ```
//!
//! and that is the whole of what a piece is. There is no third combinator and
//! no pass: a part is not a thing in the term, it is the *scope* its voices'
//! facts are constructed at.
//!
//! # What the structure is for
//!
//! Scope, and nothing else. [`super::notation`] refuses seven statements: four
//! because "from here onward" has no unique meaning in a value usable at
//! several places, and three because bar structure needs a voice to belong to.
//! Both refusals are about one missing thing, and this module supplies it by
//! widening the reading rather than by widening the statement table — reading a
//! voice's body is `notated(node, Reading::at(Scope::Voice { … }))` and nothing
//! else. Everything the fold already knew it still knows.
//!
//! # The numbering is positional and counts every item
//!
//! A part's id is a running count over the score; a voice's is its position
//! among the part's items, *including* the instance sites this module does not
//! expand. `PartDecl::items` says why the grammar keeps them interleaved — "a
//! `make` between two voices makes a voice *there*" — and counting them here
//! means the number a written voice gets does not move when prompt 142 teaches
//! the reading to expand its neighbour.
//!
//! # A region is the longest voice
//!
//! [`super::notation::extent`] sums, because a block is a fold and a fold's
//! extent is a sum. What a header fact covers is `03-denotational-semantics.md`
//! D3's `max(d, e)` over the parts instead, so it is computed from the voices
//! rather than by calling the same function one level up.
//!
//! # Where a context statement lands
//!
//! A header's facts cover the piece; a context statement written among a
//! voice's items is a **point** where the music reached it. Both are the
//! replaced path's rules unchanged, including the two that look like
//! exceptions: a `meter`, a `key`, or a `tempo` written in one voice is the
//! *piece's* from there — per-voice meter is polymeter, which is a part's own
//! header — and a `clef` is the *part's*, because the reader whose hands change
//! staff is one player.

#[cfg(test)]
mod laws;

use musa_core::{Origin, Raw};
use musa_language::SyntaxNode;
use musa_language::ast::AstNode as _;
use num_rational::Ratio;

use super::notation::{Context, Reading, extent};
use super::{Lowering, applied};
use crate::diagnose::{Code, Diagnostic};

/// A piece, read.
///
/// The track is the whole answer; the parts are what a consumer needs to bucket
/// a projection of it back into the score it came from. Nothing else, because
/// nothing else has a caller: names and ids are what `Scope` is written out of,
/// and every other question about a piece is answered by the term.
pub(crate) struct Piece {
    /// The piece as one written-time track.
    pub(crate) track: Raw,
    /// Its parts, in written order.
    pub(crate) parts: Vec<Part>,
}

/// One part, and the voices it numbered.
pub(crate) struct Part {
    /// The id every fact of this part is constructed at.
    pub(crate) id: u32,
    /// What the score calls it.
    pub(crate) name: String,
    /// Its voices, in written order.
    pub(crate) voices: Vec<Voice>,
}

/// One voice of one part.
pub(crate) struct Voice {
    /// The id within its part, which is its position among the part's items.
    pub(crate) id: u32,
    /// What the part calls it.
    pub(crate) name: String,
    /// What this voice alone sounds, as a term.
    ///
    /// Beside [`Piece::track`] rather than instead of it, because the two answer
    /// different questions and a consumer needs both: a projection is *lanes*,
    /// one per voice, and the piece's own identity is the whole simultaneity.
    /// Deriving either from the other would mean taking a normal form apart by
    /// scope, which is reading provenance back out of a value that was built to
    /// carry it forward.
    pub(crate) track: Raw,
    /// The claims written over passages of this voice, in source order.
    ///
    /// Per voice rather than per piece because a claim is proved against the
    /// barlines *its own part* counts by, and a polymetric piece has more than
    /// one set of them. Each one carries its own two terms, so a claim is placed
    /// by reading, not by a position this walk would have had to keep.
    pub(crate) claims: Vec<super::notation::Claimed>,
}

impl Lowering<'_> {
    /// `piece "…" { … }` — its score, its context, and the track they make.
    ///
    /// [`None`] when anything in it was refused, for [`Lowering::music`]'s
    /// reason one level up: a piece missing a voice is a different piece, not a
    /// shorter one. Every part and every voice is read even after one is
    /// refused, so a score with three bad voices reports three diagnostics.
    pub(crate) fn piece(&mut self, node: &SyntaxNode) -> Option<Piece> {
        let declaration = musa_language::ast::PieceDecl::cast(node.clone())?;
        let origin = self.origin(node);
        let mut whole = true;
        let mut context = self.header(&declaration, &mut whole);
        // The meter every voice starts in. Read off the header rather than
        // carried out of [`Lowering::header`], because a header states three
        // different facts and only one of them is a *reading context*: a `senza`
        // asks what meter to put back, and no voice asks about the key or the
        // tempo the piece opened with.
        let opening = declaration
            .meter()
            .and_then(|statement| crate::resolve::parse_meter(&statement))
            .unwrap_or_default();
        let mut parts: Vec<Part> = Vec::new();
        let mut tracks = Vec::new();
        for written in declaration.score().map(|score| score.parts()).unwrap_or_default() {
            let name = written.name().unwrap_or_default();
            let span = crate::resolve::trimmed_span(written.syntax());
            if parts.iter().any(|part| part.name == name) {
                self.repeated(
                    &mut whole,
                    format!("this score already has a part called `{name}`"),
                    span,
                );
                continue;
            }
            // Numbered only once the name is accepted, so a refused part costs
            // no id and the surviving ones keep the numbers they would have had.
            let id = u32::try_from(parts.len()).unwrap_or(u32::MAX);
            context.extend(self.part_context(&written, id, &mut whole));
            let mut voices: Vec<Voice> = Vec::new();
            for (index, item) in written.items().into_iter().enumerate() {
                let voice = u32::try_from(index).unwrap_or(u32::MAX);
                let held = match item {
                    musa_language::ast::PartItem::Voice(held) => held,
                    musa_language::ast::PartItem::Make(site) => {
                        self.made(&mut whole, site.syntax());
                        continue;
                    }
                };
                let voice_name = held.name().unwrap_or_default();
                if voices.iter().any(|earlier| earlier.name == voice_name) {
                    self.repeated(
                        &mut whole,
                        format!("part `{name}` already has a voice called `{voice_name}`"),
                        crate::resolve::trimmed_span(held.syntax()),
                    );
                    continue;
                }
                let held_at = Reading::at(crate::Scope::Voice { part: id, voice }).metered(opening);
                let Some(track) = self.notated(held.syntax(), held_at) else {
                    whole = false;
                    continue;
                };
                tracks.push(track.clone());
                voices.push(Voice {
                    id: voice,
                    name: voice_name,
                    track,
                    // Taken here, where the voice they were written in is still
                    // the thing being read: the walk is shared across the score,
                    // so leaving them would give the next voice this one's bars.
                    claims: self.claimed(),
                });
            }
            parts.push(Part { id, name, voices });
        }
        let over = reach(&declaration);
        let laid: Vec<Raw> = context
            .into_iter()
            .map(|(scope, said, fact)| self.sounded_at(said, scope, fact, over))
            .collect();
        tracks.push(simultaneous(origin, laid));
        whole.then(|| Piece {
            track: simultaneous(origin, tracks),
            parts,
        })
    }

    /// A name the score already used, at the declaration that used it again.
    ///
    /// Two declarations of one name is always an error here for the reason
    /// `crate::resolve` gives for motifs: musa has no shadowing. What is
    /// specific to a part and a voice is that the name is also how a projection
    /// is read back, so a repeat would make two lanes indistinguishable.
    fn repeated(&mut self, whole: &mut bool, said: String, span: crate::origin::SourceSpan) {
        *whole = false;
        self.refuse::<()>(
            Diagnostic::error(Code::DuplicateName, said)
                .at(span, "declared again here")
                .help("rename one of them, or delete this declaration"),
        );
    }

    /// The facts a piece's own header states, each with the scope it is about
    /// and the origin of the statement that said it.
    ///
    /// The origin travels beside the fact rather than being the piece's,
    /// because `sounded` writes it into the constructed fact and a composer
    /// asking where the 7/8 came from wants the line that says `meter 7/8`.
    ///
    /// The meter is there whether or not anybody wrote one, because an
    /// unwritten meter is still a meter and 4/4 governs the piece either way.
    /// The key and the tempo are not: a page with no marking on it is a page
    /// with no marking on it, and the 120 a performance falls back to is the
    /// performance layer's default rather than something the piece said.
    fn header(
        &mut self,
        declaration: &musa_language::ast::PieceDecl,
        whole: &mut bool,
    ) -> Vec<(crate::Scope, Origin, Raw)> {
        let mut said = Vec::new();
        match declaration.meter() {
            Some(statement) => match self.stated(statement.syntax(), Context::Meter) {
                Some((origin, fact)) => said.push((crate::Scope::Piece, origin, fact)),
                None => *whole = false,
            },
            // Unwritten, so [`Origin::UNKNOWN`]: the table hands out no number
            // for it and `Sites::span` answers [`None`], which is what
            // `provenance_at` already reads as "nowhere of its own to point".
            None => said.push((crate::Scope::Piece, Origin::UNKNOWN, unmeasured(Origin::UNKNOWN))),
        }
        for (statement, which) in [
            declaration.tempo().map(|held| (held.syntax().clone(), Context::Tempo)),
            declaration.key().map(|held| (held.syntax().clone(), Context::Key)),
        ]
        .into_iter()
        .flatten()
        {
            match self.stated(&statement, which) {
                Some((origin, fact)) => said.push((crate::Scope::Piece, origin, fact)),
                None => *whole = false,
            }
        }
        said
    }

    /// The facts a part's own header states, at [`crate::Scope::Part`].
    ///
    /// Read through [`crate::resolve::part_facts`] rather than off the nodes,
    /// because that reading already exists and already carries the two refusals
    /// this one would otherwise have to invent — a second `clef` is two answers
    /// to one question, and an unreadable meter is not a meter. What is left
    /// here is writing the three values it answers as the raw terms
    /// `registry::notation` reads back, which is `AGENTS.md`'s "hand a consumer
    /// what we already computed" at the one place it applies in this module.
    ///
    /// A part that complained answers nothing: the piece is refused either way,
    /// so a half-read context is a term nobody will look at.
    fn part_context(
        &mut self,
        part: &musa_language::ast::PartDecl,
        id: u32,
        whole: &mut bool,
    ) -> Vec<(crate::Scope, Origin, Raw)> {
        let Some(facts) = self.heard(|resolver| crate::resolve::part_facts(resolver, part)) else {
            *whole = false;
            return Vec::new();
        };
        let scope = crate::Scope::Part { part: id };
        let mut said = Vec::new();
        if let Some((clef, span)) = facts.clef {
            let origin = self.sites.at(span);
            said.push((scope, origin, super::notation::clefed(origin, clef)));
        }
        if let Some((meter, span)) = facts.meter {
            let origin = self.sites.at(span);
            said.push((scope, origin, super::notation::metered(origin, meter)));
        }
        if let Some((marking, span)) = facts.tempo {
            let origin = self.sites.at(span);
            said.push((scope, origin, super::notation::tempo(origin, &marking)));
        }
        said
    }

    /// One header statement, as the fact it states and the origin that states it.
    fn stated(&mut self, node: &SyntaxNode, which: Context) -> Option<(Origin, Raw)> {
        let origin = self.origin(node);
        let fact = self.fact(node, origin, crate::resolve::trimmed_span(node), which)?;
        Some((origin, fact))
    }

    /// A `make` standing where a voice would.
    ///
    /// Prompt 142's, and stated rather than skipped: an instance site mints an
    /// expansion path, and `Sites` numbers written nodes and has no way to say
    /// that a term came from a template applied here. Reading the site as an
    /// ordinary application would answer a piece whose provenance pointed at
    /// the template instead of at the score.
    fn made(&mut self, whole: &mut bool, site: &SyntaxNode) {
        *whole = false;
        self.refuse::<()>(
            Diagnostic::error(
                Code::UnsupportedLanguageStage,
                "a voice made from a template has no core spelling yet",
            )
            .at(crate::resolve::trimmed_span(site), "an instance site")
            .note("prompt 142 gives an expansion a provenance to be numbered by"),
        );
    }
}

/// `together` over `tracks`, right-nested, seeded with the empty track.
///
/// Right-nested rather than folded from the left because `together` is
/// associative and commutative (L4, L5), so the shape is free and the one that
/// reads as "this voice, beside all the rest" is the one written.
fn simultaneous(origin: Origin, tracks: Vec<Raw>) -> Raw {
    tracks
        .into_iter()
        .rev()
        .fold(Raw::lit(origin, crate::registry::empty_track()), |rest, track| {
            applied(origin, Raw::var(origin, "together"), [track, rest])
        })
}

/// How far the longest voice in `declaration` reaches.
///
/// D3's `max(d, e)` over the parts, and the reason it is not
/// [`super::notation::extent`] of the piece: that function sums, because a
/// block is a fold.
fn reach(declaration: &musa_language::ast::PieceDecl) -> Ratio<i64> {
    declaration
        .score()
        .map(|score| score.parts())
        .unwrap_or_default()
        .iter()
        .flat_map(|part| part.voices())
        .map(|voice| extent(voice.syntax()))
        .max()
        .unwrap_or(Ratio::ZERO)
}

/// `Fact.Meter 4 4`, for a piece that wrote none.
fn unmeasured(origin: Origin) -> Raw {
    let meter = crate::score::Meter::default();
    applied(
        origin,
        Raw::var(origin, "Fact.Meter"),
        [
            super::whole(origin, u64::from(meter.numerator())),
            super::whole(origin, u64::from(meter.denominator())),
        ],
    )
}
