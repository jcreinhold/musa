//! The eight ways a program can build or transform an event track, and the
//! three it cannot name, as core builtins.
//!
//! §5.8's *third* family. A track builtin is neither a δ-builtin nor a
//! structural eliminator: `02-core-calculus.md` §5.8 gives it its own
//! admissibility argument — §5.7's, about construction safety — and
//! [`musa_calculus::Family::Track`] is where that is recorded. Seven of the eight
//! still *reduce* the way a δ-builtin does, because data in and data out is what
//! they do; `map_note_pitches` takes a mapper and reduces the way a traversal
//! does. That is why [`musa_calculus::Builtin::structural_with`] takes the family
//! rather than fixing it: the family says which of §5.8's admissibility
//! arguments covers the operation, the reduction says how it computes, and this
//! module is the case that proves they are two questions.
//!
//! # What was not carried over
//!
//! [`crate::core`]'s evaluator has an arm for each of these eight, and not one
//! of them is the arithmetic below. Each builds a `MusicOperation` — a *deferred
//! contextual* description that [`crate::elaborate`] closes later against a key,
//! a placement, a scope, and an origin — so there was nothing to translate. What
//! is carried over is one layer down and already pure:
//! [`ScoreFact::transposed`], [`ScoreFact::stretched`], [`ScoreFact::inverted`],
//! [`musa_events::EventTrack::scale`], and [`musa_events::together`]. The
//! contextual path is untouched, and prompt 142 owns its deletion.
//!
//! Two things that path does are therefore *absent* here, and both are absences
//! by construction rather than omissions. A transposition does not rebase an
//! enclosing key, because a fact in a track already has a written pitch and
//! there is no key left to rebase; and `play` does not apply an enclosing
//! scale's intervals, because a rule has no enclosing anything. What a track
//! builtin sees is the track.
//!
//! # Partiality is in the result type, one builtin at a time
//!
//! D2 asks that a rule answer `None` only where the host's table is wrong, so an
//! operation that can fail on data its signature admits says so in its own
//! result instead. Four can: `transpose` and `invert` because a fixed-width
//! storage coordinate can overflow, `stretch` because a factor at or below zero
//! is not a stretch, `shift` because a negative offset is not a placement, and
//! `play` because a chord sounding for no time is not a sounding. `retrograde`,
//! `together`, and `map_note_pitches` cannot fail and answer a track directly.
//! Uniformity is not a reason to give a total operation an error case.
//!
//! # Provenance
//!
//! §5.7 requires every fact to carry a complete `Origin`. The transforms are
//! handed facts that have one and record their own step as having *enclosed* it
//! — [`ExpansionStep::Transposition`], `Stretch`, `Retrograde`, `Inversion`, and
//! `MapNotePitches` each carry no span, so a `fn` pointer can build one out of
//! its own arguments, and [`Origin::enclosed_by`] puts it where a path that
//! reads outside-in needs it. `play` *constructs*, and a source span is not something a
//! `fn` pointer can invent, so its origin and its scope are arguments: the one
//! thing a rule cannot compute is the thing the caller supplies, exactly as
//! `instantiate_quote` takes the anchor it builds under. `instanced` takes one
//! for the same reason and reads only its path: an expansion's identity is
//! minted where the reading resolved it, and nothing here could derive it.
//!
//! `spliced` needs none, because an event track quote's raw payloads were stamped
//! where they were read (see [`crate::lower::events`]): a quote is a written
//! form, so its span, its scope, and its splice step are all fixed before any
//! material arrives.

#[cfg(test)]
mod laws;

use musa_calculus::{Builtin, Cx, Datum, ElabError, Family, Index, Literal, Rule, Term};
use musa_events::{Duration, Occurrence, Position, Span};
use num_rational::Ratio;

use super::rules::{items, nat, read, reduced, refused};
use super::{HERE, held, literal, plain_type, tagged_type};
use crate::core::Coordinate;
use crate::elaborate::{FactKind, ScoreFact, VoiceTrack};
use musa_score::Interval;
use musa_score::origin::{ExpansionStep, Origin};
use musa_score::pitch::WrittenPitch;
use musa_score::scope::Scope;
use musa_score::score::NotatedDuration;

/// The eight source words of `BUILTIN_OWNERSHIP`'s track family, in the order
/// the table writes them.
///
/// Named rather than counted, for [`super::traversal::SPELLINGS`]'s reason:
/// "which eight" is the claim the accounting law checks, and a count agrees with
/// a wrong set as readily as with the right one.
pub(super) const SPELLINGS: [&str; 8] = [
    "transpose",
    "stretch",
    "retrograde",
    "invert",
    "shift",
    "together",
    "map_note_pitches",
    "play",
];

/// The four operations this module registers that neither ownership table
/// names.
///
/// [`super::rules::BEYOND`]'s reason, in a second place, and four times over
/// for one idea: a program may not write the word that would let it say
/// something false about where a fact came from.
///
/// `map_note_pitches` is §5.7's *controlled* transform: it applies a mapper to
/// each written pitch and puts the answers back where they came from, and the
/// control is exactly that a program cannot choose the places.
/// `set_note_pitches` is the putting-back half, so a source word for it would be
/// the control removed — it would let any list of pitches into any track's
/// noteheads.
///
/// `instanced` is the same argument about provenance rather than about pitch.
/// The expansion path it stamps is minted by the reading that resolved the site
/// — a `make`'s structural address in [`crate::template`], or a `${…}`'s locus
/// in [`crate::lower::events`] — and Origin view reads that path to tell a
/// composer's notes from generated ones. A source word for it would let a
/// program claim its notes were made by an expansion that never made them.
///
/// `spliced` is the third because the source already spells the whole operation,
/// as `events EventTrack[WrittenTime, ScoreFact] { … }`. A row in either table
/// would invent a second spelling for it — a *called* word, taking a term no
/// expression can build and a list of material in an order only the reading
/// knows. `instantiate_quote` is out of both tables for exactly this reason one
/// stage up.
///
/// `scoped` is the fourth, and the one that makes reusable material *reusable*.
/// A `fragment`, a `motif`, and a free `music { … }` are read at
/// [`Scope::Piece`] because they have no voice of their own
/// (`00-semantics.md` §3), so folding one into a voice has to say which voice it
/// was folded into — and that cannot be done while reading, for `instanced`'s
/// reason exactly: the facts do not exist until the term is evaluated, and the
/// ones a function the body calls produced were read in another declaration
/// entirely. A source word for it would let a program put its notes in another
/// player's staff, which is the same false claim `set_note_pitches` and
/// `instanced` exist to prevent, about place rather than about pitch or
/// provenance.
///
/// `respelled` is the fifth and is `set_note_pitches` with a position: a `with`
/// clause names one note of one occurrence and writes a pitch onto it. It is out
/// of both tables for the first one's reason exactly — a source word for it
/// would let a program put any pitch on any note of any track it can name, and
/// the whole control is that the *reading* chose the occurrence. The position is
/// counted rather than chosen for the same reason: `01-surface.md`'s `with`
/// clause is the only thing that writes one.
pub(super) const TRACK_BEYOND: [&str; 6] = [
    "set_note_pitches",
    "instanced",
    "spliced",
    "scoped",
    "respelled",
    "track_duration",
];

/// The term naming an event track in written time.
///
/// The only instantiation this compiler has, and the one every signature below
/// is written at. `EventTrack` is registered at `Coordinate → Type 0`, so this
/// is the same application `Duration ⟨written⟩` is.
pub(super) fn track_type() -> Term {
    tagged_type("EventTrack", Coordinate::WrittenTime)
}

/// An event track quote's term, at the one instantiation this compiler has.
pub(super) type Quoted = musa_events::Term<musa_events::WrittenTime, ScoreFact>;

/// An event track quote's body as a literal's payload: the term, and the name each of
/// its holes was given.
///
/// The two are one value because they are one fact. The names are minted by
/// [`crate::lower::events`] while it substitutes the holes out of the text, and
/// the *i*th of them is what the *i*th member of `spliced`'s list is bound to;
/// a term carrying the names and a caller carrying them separately would be two
/// copies of an order that has to agree.
#[derive(Debug, PartialEq)]
pub(super) struct Assembly {
    /// The parsed term, with its raw payloads already stamped.
    pub(super) term: Quoted,
    /// The fresh name standing at each hole, in written order.
    pub(super) holes: Vec<String>,
}

impl std::fmt::Display for Assembly {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(out, "event-track term with {} holes", self.holes.len())
    }
}

/// An `Origin` as a literal's payload.
///
/// A wrapper only because [`super::Domain`] asks for `Display` and an origin has
/// none — there is nothing sensible to print for a value whose whole content is
/// spans and an expansion path, and putting a `Display` on the type itself would
/// be a rendering nobody asked for in front of every formatter in the compiler.
/// The precedent is [`super::rules::Kind`], one domain over.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct Provenance(pub(super) Origin);

impl std::fmt::Display for Provenance {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(out, "origin@{}..{}", self.0.source_span.start, self.0.source_span.end)
    }
}

/// The eight track builtins, and the one operation `map_note_pitches` reduces
/// through.
///
/// # Errors
///
/// [`ElabError`] when `List`, `Scope`, or a `List` constructor is not declared
/// in `cx`, which is a defect in this compiler rather than in any program.
pub(super) fn builtins(cx: &Cx) -> Result<Vec<Builtin>, ElabError> {
    let track = track_type;
    let beat = || tagged_type("Duration", Coordinate::WrittenTime);
    let delta = |name: &'static str, arguments: Vec<Term>, result: Term, rule: Rule| {
        Builtin::new(name, super::arrow(arguments, result), Family::Track, rule)
    };
    Ok(vec![
        delta(SPELLINGS[0], vec![plain_type("Interval"), track()], track(), TRANSPOSE),
        delta(SPELLINGS[1], vec![plain_type("Ratio"), track()], track(), STRETCH),
        delta(SPELLINGS[2], vec![track()], track(), RETROGRADE),
        delta(SPELLINGS[3], vec![plain_type("Pitch"), track()], track(), INVERT),
        delta(SPELLINGS[4], vec![beat(), track()], track(), SHIFT),
        delta(SPELLINGS[5], vec![track(), track()], track(), TOGETHER),
        Builtin::structural_with(
            SPELLINGS[6],
            super::arrow(
                vec![
                    Term::pi(HERE, "pitch", plain_type("Pitch"), plain_type("Pitch")),
                    track(),
                ],
                track(),
            ),
            Family::Track,
            MAPPED_TRACK,
            vec![
                crate::prelude::constant(cx, "List.Empty")?,
                crate::prelude::constant(cx, "List.Cons")?,
                set_note_pitches(cx)?.term(HERE),
            ],
            rewrite_map_note_pitches,
        ),
        delta(
            SPELLINGS[7],
            vec![
                plain_type("Origin"),
                crate::prelude::constant(cx, "Scope")?,
                plain_type("Voicing"),
                beat(),
            ],
            track(),
            PLAY,
        ),
        set_note_pitches(cx)?,
        delta(TRACK_BEYOND[1], vec![plain_type("Origin"), track()], track(), INSTANCED),
        delta(
            TRACK_BEYOND[2],
            vec![plain_type("EventsTerm"), super::applied(cx, "List", [track()])?],
            track(),
            SPLICED,
        ),
        delta(
            TRACK_BEYOND[3],
            vec![crate::prelude::constant(cx, "Scope")?, track()],
            track(),
            SCOPED,
        ),
        delta(
            TRACK_BEYOND[4],
            vec![
                plain_type("Origin"),
                crate::prelude::constant(cx, "Nat")?,
                plain_type("Pitch"),
                track(),
            ],
            track(),
            RESPELLED,
        ),
        delta(TRACK_BEYOND[5], vec![track()], beat(), TRACK_DURATION),
    ])
}

/// `track_duration(t)` — how long `t` sounds, read off the value.
///
/// The piece's header facts are sounded for exactly this long: "a header's
/// facts cover the piece" is a statement about the piece's *evaluated* extent
/// — a voice whose length arrives through a `use` is as long as the material
/// it names, which the written tree does not say — so the lowering hands the
/// sounding the music as a term and this rule answers the number. A source
/// word for it would be a second way to ask what `01-surface.md` gives no
/// composer-facing reason to ask; it is out of both tables for
/// [`super::rules::BEYOND`]'s reason.
const TRACK_DURATION: Rule = |arguments| {
    let track = track_of(arguments.first()?)?;
    reduced(super::rules::duration(track.duration().as_ratio()))
};

/// `set_note_pitches : EventTrack ⟨written⟩ → List Pitch → EventTrack ⟨written⟩`.
///
/// Total by construction: it replaces the *i*th note's pitch with the *i*th
/// member of the list and leaves every note the list does not reach, and every
/// fact that is not a note, exactly as it was. That is what lets it be a rule
/// rather than a partial one — a length mismatch is not a failure, it is fewer
/// replacements.
///
/// # Errors
///
/// [`ElabError`] when `List` is not declared in `cx`.
fn set_note_pitches(cx: &Cx) -> Result<Builtin, ElabError> {
    Ok(Builtin::new(
        TRACK_BEYOND[0],
        super::arrow(
            vec![track_type(), super::applied(cx, "List", [plain_type("Pitch")])?],
            track_type(),
        ),
        Family::Track,
        SET_NOTE_PITCHES,
    ))
}

// ---- reading and writing a track ----

/// The track an argument holds.
pub(super) fn track_of(datum: &Datum) -> Option<VoiceTrack> {
    let Datum::Lit(ref value) = *datum else {
        return None;
    };
    held::<VoiceTrack>(value).cloned()
}

/// A track, as a literal at `EventTrack ⟨written⟩`.
pub(super) fn built(track: VoiceTrack) -> Datum {
    Datum::Lit(literal(track_type(), track))
}

/// The same track with each fact rewritten by `each` and `step` recorded as
/// having happened around every fact's expansion path.
///
/// One helper for four of the eight, because "transform each payload and record
/// that it happened" is what a controlled transform *is*: a version that let a
/// caller do one without the other would be the provenance hole §7 exists to
/// close. `each` answers `None` for a fact it cannot transform, and the whole
/// track is then refused rather than the untransformed fact silently kept.
///
/// [`Origin::enclosed_by`] rather than a push, because a transform *is* the
/// enclosing thing: the track it was handed was built first and the path reads
/// outside-in.
fn rewritten(
    track: &VoiceTrack,
    step: &ExpansionStep,
    each: impl Fn(&ScoreFact) -> Option<ScoreFact>,
) -> Option<VoiceTrack> {
    let occurrences = track
        .occurrences()
        .iter()
        .map(|occurrence| {
            let mut fact = each(occurrence.payload())?;
            fact.origin.enclosed_by(step.clone());
            Some(Occurrence::new(occurrence.span(), fact))
        })
        .collect::<Option<Vec<_>>>()?;
    musa_events::track(track.duration(), occurrences).ok()
}

// ---- the seven rules that reduce, and the three past both tables ----

/// `transpose(interval, t)` — every written pitch raised, every other fact left
/// where it is.
const TRANSPOSE: Rule = |arguments| {
    let interval = read::<Interval>(arguments.first()?)?;
    let track = track_of(arguments.get(1)?)?;
    let step = ExpansionStep::Transposition(interval);
    let Some(raised) = rewritten(&track, &step, |fact| fact.transposed(interval)) else {
        return Some(refused("a transposed pitch does not fit the written-pitch coordinate"));
    };
    reduced(built(raised))
};

/// `stretch(factor, t)` — the same music notated `factor` times as long, in a
/// track that much longer.
const STRETCH: Rule = |arguments| {
    let factor = read::<Ratio<i64>>(arguments.first()?)?;
    let track = track_of(arguments.get(1)?)?;
    let Ok(scaled) = track.scale(factor) else {
        return Some(refused("a stretch factor is greater than zero"));
    };
    let step = ExpansionStep::Stretch(factor);
    let Some(stretched) = rewritten(&scaled, &step, |fact| Some(fact.stretched(factor))) else {
        return Some(refused("the stretched music leaves the track it is in"));
    };
    reduced(built(stretched))
};

/// `retrograde(t)` — every occurrence mirrored about the track's midpoint.
///
/// The duration is unchanged and the payloads are untouched: a retrograde is a
/// statement about *when* things happen, and a note that was written as a
/// quarter is still written as a quarter.
const RETROGRADE: Rule = |arguments| {
    let track = track_of(arguments.first()?)?;
    let extent = track.duration().as_ratio();
    let occurrences = track
        .occurrences()
        .iter()
        .map(|occurrence| {
            let span = occurrence.span();
            let mirrored = Span::new(
                Position::new(extent - span.end().as_ratio()),
                Position::new(extent - span.start().as_ratio()),
            )
            .ok()?;
            let mut fact = occurrence.payload().clone();
            fact.origin.enclosed_by(ExpansionStep::Retrograde);
            Some(Occurrence::new(mirrored, fact))
        })
        .collect::<Option<Vec<_>>>()?;
    reduced(built(musa_events::track(track.duration(), occurrences).ok()?))
};

/// `invert(axis, t)` — every written pitch mirrored about `axis`.
const INVERT: Rule = |arguments| {
    let axis = read::<WrittenPitch>(arguments.first()?)?;
    let track = track_of(arguments.get(1)?)?;
    let step = ExpansionStep::Inversion { axis: axis.to_string() };
    let Some(mirrored) = rewritten(&track, &step, |fact| fact.inverted(axis)) else {
        return Some(refused("an inverted pitch does not fit the written-pitch coordinate"));
    };
    reduced(built(mirrored))
};

/// `shift(by, t)` — the same music starting `by` later, in a track that much
/// longer.
///
/// No expansion step, and the asymmetry is worth naming: `ExpansionStep` has no
/// `Shift`, because moving in time is the occurrence's *span*, which is where
/// the change is already visible and exact. A step recording it would be a
/// second copy of a number the fact already carries.
const SHIFT: Rule = |arguments| {
    let by = read::<Ratio<i64>>(arguments.first()?)?;
    let track = track_of(arguments.get(1)?)?;
    let Ok(offset) = Duration::new(by) else {
        return Some(refused("music cannot be shifted to before the start"));
    };
    let occurrences = track
        .occurrences()
        .iter()
        .map(|occurrence| Occurrence::new(occurrence.span().translate(offset), occurrence.payload().clone()))
        .collect();
    let Ok(shifted) = musa_events::track(track.duration().plus(offset), occurrences) else {
        return Some(refused("the shifted music leaves the track it is in"));
    };
    reduced(built(shifted))
};

/// `together(a, b)` — both at once, in a track as long as the longer.
///
/// The event track's own stacking, which is D3's `max(d, e)` and `E ⊎ F`. §5.7's
/// composition clause is discharged by calling the operation the clause is about
/// rather than by re-deriving it here.
const TOGETHER: Rule = |arguments| {
    let left = track_of(arguments.first()?)?;
    let right = track_of(arguments.get(1)?)?;
    reduced(built(musa_events::together(vec![left, right])))
};

/// `set_note_pitches(t, ps)` — the *i*th note's pitch replaced by the *i*th
/// member.
const SET_NOTE_PITCHES: Rule = |arguments| {
    let track = track_of(arguments.first()?)?;
    let mut replacements = items(arguments.get(1)?)?
        .into_iter()
        .map(read::<WrittenPitch>)
        .collect::<Option<Vec<_>>>()?
        .into_iter();
    let occurrences = track
        .occurrences()
        .iter()
        .map(|occurrence| {
            let mut fact = occurrence.payload().clone();
            if let Some(pitch) = fact.sounding_pitch_mut()
                && let Some(replacement) = replacements.next()
            {
                *pitch = replacement;
                fact.origin.enclosed_by(ExpansionStep::MapNotePitches);
            }
            Occurrence::new(occurrence.span(), fact)
        })
        .collect();
    reduced(built(musa_events::track(track.duration(), occurrences).ok()?))
};

/// The occurrences of `track` that a `with` clause counts, grouped into
/// positions.
///
/// One position per group of *event* occurrences sharing a span, which is how a
/// chord's pitches become one position and how a slur laid over the body becomes
/// none: `04-provenance.md`'s inspector numbers what a reader can point at, and
/// a reader points at noteheads and rests. Skipping a rest instead would
/// renumber every note after it, so a rest holds its position and refuses to be
/// written on.
fn positions_of(track: &VoiceTrack) -> Vec<Vec<usize>> {
    let mut positions: Vec<Vec<usize>> = Vec::new();
    let mut previous = None;
    for (index, occurrence) in track.occurrences().iter().enumerate() {
        if !occurrence.payload().kind.is_event() {
            continue;
        }
        match positions.last_mut() {
            Some(group) if previous == Some(occurrence.span()) => group.push(index),
            _ => positions.push(vec![index]),
        }
        previous = Some(occurrence.span());
    }
    positions
}

/// `respelled(at, position, pitch, t)` — the one note `position` names, written
/// as `pitch` instead.
///
/// The three ways of naming no such note are three refusals rather than a silent
/// no-op, because a `with` clause that quietly did nothing would be a composer's
/// edit that never happened. None of the three is knowable while reading: the
/// material is a term until it is evaluated, so how many positions it has, and
/// what stands at each, are answers only this rule has. What *is* knowable
/// there — a position of zero, a clause with no pitch, one note named twice —
/// [`crate::lower::notation`] refuses where it is written, and this rule never
/// sees it.
///
/// `at` carries the override's own [`ExpansionStep::Specialization`], and the
/// path is spliced in front for [`INSTANCED`]'s reason: everything the
/// respelled note already carried happened inside the occurrence being
/// specialized, and only this note is touched — its siblings were not
/// specialized and their paths must not say they were.
const RESPELLED: Rule = |arguments| {
    let Datum::Lit(ref written) = *arguments.first()? else {
        return None;
    };
    let path = held::<Provenance>(written)?.0.expansion_path.clone();
    let position = usize::try_from(nat(arguments.get(1)?)?).ok()?;
    let pitch = read::<WrittenPitch>(arguments.get(2)?)?;
    let track = track_of(arguments.get(3)?)?;

    let positions = positions_of(&track);
    let Some(group) = position.checked_sub(1).and_then(|at| positions.get(at)) else {
        let count = positions.len();
        let plural = if count == 1 { "" } else { "s" };
        return Some(refused(&format!(
            "this occurrence has {count} note{plural}, so there is no note {position}"
        )));
    };
    if group.len() > 1 {
        return Some(refused(&format!(
            "note {position} is a chord, and an override respells one note"
        )));
    }
    let &[only] = group.as_slice() else {
        return None;
    };
    if track
        .occurrences()
        .get(only)
        .and_then(|occurrence| occurrence.payload().pitch_of())
        .is_none()
    {
        return Some(refused(&format!(
            "note {position} is a rest, and a rest has no pitch to respell"
        )));
    }

    let occurrences = track
        .occurrences()
        .iter()
        .enumerate()
        .map(|(index, occurrence)| {
            let mut fact = occurrence.payload().clone();
            if index == only
                && let FactKind::Note {
                    pitch: ref mut written, ..
                } = fact.kind
            {
                *written = pitch;
                fact.origin.expansion_path.splice(0..0, path.iter().cloned());
            }
            Occurrence::new(occurrence.span(), fact)
        })
        .collect();
    reduced(built(musa_events::track(track.duration(), occurrences).ok()?))
};

/// `play(origin, scope, voicing, held)` — a chord sounding for a length.
///
/// §5.7's construction clause is discharged here and nowhere else, because this
/// is the only builtin that *makes* a fact. Every fact it makes carries the
/// scope and the origin it was handed, sounds over `[0, held]` inside a track of
/// exactly `held`, and is a note and nothing else: `play` writes no
/// articulation, no tie, and no freedom, because none of those is a property of
/// a chord sounding for a length.
const PLAY: Rule = |arguments| {
    let Datum::Lit(ref written) = *arguments.first()? else {
        return None;
    };
    let origin = &held::<Provenance>(written)?.0;
    let scope = scope_of(arguments.get(1)?)?;
    let voicing = read::<musa_score::chord::Voicing>(arguments.get(2)?)?;
    let sounding = read::<Ratio<i64>>(arguments.get(3)?)?;
    if sounding <= Ratio::ZERO {
        return Some(refused("a chord sounds for longer than no time at all"));
    }
    let Ok(span) = Span::new(Position::ZERO, Position::new(sounding)) else {
        return Some(refused("a chord sounds for longer than no time at all"));
    };
    let duration = NotatedDuration::spelled(sounding);
    let occurrences = voicing
        .pitches()
        .map(|pitch| {
            Occurrence::new(
                span,
                ScoreFact {
                    scope,
                    kind: FactKind::Note {
                        pitch,
                        duration: duration.clone(),
                        articulations: Vec::new(),
                        free: None,
                    },
                    origin: origin.clone(),
                    tied: false,
                },
            )
        })
        .collect();
    let Ok(sounded) = musa_events::track(span.duration(), occurrences) else {
        return Some(refused("the chord does not fit the length it was given"));
    };
    reduced(built(sounded))
};

/// `instanced(origin, t)` — every fact in `t` recorded as having been produced
/// inside the expansion `origin` names.
///
/// The one rule that writes the *front* of an expansion path, and the reason an
/// instance site needs a builtin at all rather than a reading that stamps as it
/// walks. A template's body is a term; the facts it answers are made when that
/// term is evaluated, and some of them come out of definitions elaborated long
/// before any site made anything. Stamping while reading would reach the notes
/// written inside the template body and miss every note a function it calls
/// produced.
///
/// A fact keeps its own `definition_span` and its own declaration, because an
/// instance does not relocate text: what an editor points at inside a template
/// body is the line the author wrote, once, for every instance of it
/// (`04-templates-and-modules.md` §1) — see [`crate::template`], whose expansion
/// is a binding and never a rewrite.
///
/// `source_span` is the one field that *does* move, and only where the reading
/// left it unset. Material read as usable at several places carries
/// [`crate::elaborate::SHARED_ORIGIN`] there, because "where this event came
/// from" is a question about the use and a shared body has no one answer; this
/// is the use, so this is where it is answered. A body read at one place
/// already holds its own span and keeps it, so a template instance relocates
/// nothing and a `use` of a fragment relocates exactly the field that was
/// waiting to be filled. The fill is conditional for the reason [`SCOPED`]'s is:
/// the innermost use answers, and a use of material that already used something
/// else does not overwrite the answer that use gave.
///
/// In front rather than behind, which is why this cannot be [`rewritten`]:
/// [`ExpansionStep::TemplateInstance`] is the *first* step of anything a `make`
/// produced, and everything a transform appends happened inside the instance.
/// Nested sites compose without knowing it — the inner `instanced` has already
/// run by the time the outer one prepends.
///
/// An event track quote's hole is the second caller and the same claim in different
/// words: the material a `${…}` splices was produced inside that splice, so
/// [`crate::lower::events`] hands it an origin whose path is one
/// [`ExpansionStep::EventsSplice`] at the hole's locus. Two readings, one
/// operation, because "these facts were made inside this expansion" is one
/// thing to say.
const INSTANCED: Rule = |arguments| {
    let Datum::Lit(ref written) = *arguments.first()? else {
        return None;
    };
    let at = &held::<Provenance>(written)?.0;
    let (path, site) = (&at.expansion_path, at.source_span);
    let track = track_of(arguments.get(1)?)?;
    let occurrences = track
        .occurrences()
        .iter()
        .map(|occurrence| {
            let mut fact = occurrence.payload().clone();
            fact.origin.expansion_path.splice(0..0, path.iter().cloned());
            if fact.origin.source_span == crate::elaborate::SHARED_ORIGIN {
                fact.origin.source_span = site;
            }
            Occurrence::new(occurrence.span(), fact)
        })
        .collect();
    reduced(built(musa_events::track(track.duration(), occurrences).ok()?))
};

/// `spliced(quote, material)` — the track an event track quote assembles once its
/// holes hold the material the host wrote in them.
///
/// The half of `01-surface.md` §7 that needs values, and the only half.
/// [`crate::lower::events`] has already decided everything a quote can be wrong
/// about — that it parses, that it settles nothing its use settles, that every
/// `${…}` stands where material can, that it is closed — and has stamped the
/// payloads it wrote raw. What is left is a substitution and an evaluation, and
/// both need the material, which is a value.
///
/// A `let` per hole, first hole outermost, which is what the event track's own
/// sharing is: `Term::bind` names an evaluated track, and a name referenced
/// three times in a quote is three placements of one elaboration rather than
/// three re-readings of the expression the hole wrote. The names are the fresh
/// ones the reading minted, so nothing in the quoted text can shadow one and no
/// hole can capture another's material.
///
/// Nothing here can refuse. A term the reading checked is closed, and
/// [`musa_events::evaluate`] is total on a closed term — `Term::duration`'s one
/// error is a free name, which is the very thing `check` already rejected. The
/// `None`s below are D2's: a datum of the wrong shape, or a list whose length
/// disagrees with the term's holes, is this compiler's table being wrong rather
/// than any program's.
const SPLICED: Rule = |arguments| {
    let Datum::Lit(ref quoted) = *arguments.first()? else {
        return None;
    };
    let assembly = held::<Assembly>(quoted)?;
    let material = items(arguments.get(1)?)?
        .into_iter()
        .map(track_of)
        .collect::<Option<Vec<_>>>()?;
    if material.len() != assembly.holes.len() {
        return None;
    }
    let mut assembled = assembly.term.clone();
    for (name, track) in assembly.holes.iter().zip(material).rev() {
        assembled = musa_events::Term::bind(name.clone(), musa_events::Term::literal(track), assembled);
    }
    reduced(built(musa_events::evaluate(assembled)))
};

/// `scoped(scope, t)` — every fact in `t` that had no voice of its own placed in
/// `scope`.
///
/// What makes reusable material reusable. A `fragment`, a `motif`, and a free
/// `music { … }` are read at [`Scope::Piece`], because "usable at several
/// places" is exactly the property of having no one place
/// (`00-semantics.md` §3); folding one into a voice is what decides which voice
/// its notes are played by, and the projection buckets by that scope. The
/// replaced elaborator answered this by *expanding* the material at each use and
/// constructing its facts in the reader's context. There is no reader here — the
/// material is one value, evaluated once — so the question moves from
/// construction to a relabelling, and this is it.
///
/// Only [`Scope::Piece`] is rewritten, which is what makes this idempotent and
/// composable: a `use` inside a fragment relabels `Piece` to `Piece` and changes
/// nothing, and the `use` in the voice above it then reaches the whole nesting
/// at once. A fact that already names a part or a voice is left alone, and
/// nothing reusable can carry one — [`crate::lower::notation::Reading`] refuses
/// a `key`, a `meter`, a `tempo`, and a `clef` in material that is not placed,
/// so the only piece-scoped facts a fragment can hold are the ones that belong
/// to whoever plays it.
const SCOPED: Rule = |arguments| {
    let scope = scope_of(arguments.first()?)?;
    let track = track_of(arguments.get(1)?)?;
    let occurrences = track
        .occurrences()
        .iter()
        .map(|occurrence| {
            let mut fact = occurrence.payload().clone();
            if fact.scope == Scope::Piece {
                fact.scope = scope;
            }
            Occurrence::new(occurrence.span(), fact)
        })
        .collect();
    reduced(built(musa_events::track(track.duration(), occurrences).ok()?))
};

/// The scope a `Scope` datum stands for.
///
/// The inverse of [`crate::prelude`]'s declaration, and the one place the two
/// spellings of a scope meet. A part or voice number past `u32` is not a scope
/// this compiler can name, so it reads as absent rather than as a truncation.
pub(super) fn scope_of(datum: &Datum) -> Option<Scope> {
    let Datum::Case {
        ref constructor,
        ref fields,
    } = *datum
    else {
        return None;
    };
    let number = |at: usize| u32::try_from(nat(fields.get(at)?)?).ok();
    match &**constructor {
        "Scope.Piece" => Some(Scope::Piece),
        "Scope.Part" => Some(Scope::Part { part: number(0)? }),
        "Scope.Voice" => Some(Scope::Voice {
            part: number(0)?,
            voice: number(1)?,
        }),
        _ => None,
    }
}

// ---- the one rewrite ----

/// Which argument of `map_note_pitches` the rewrite fires on.
const MAPPED_TRACK: usize = 1;

/// Where each vocabulary entry stands, for the rewrite that reads it back.
mod word {
    pub(super) const EMPTY: usize = 0;
    pub(super) const CONS: usize = 1;
    pub(super) const SET: usize = 2;
}

/// The mapper's de Bruijn index inside the rewrite's answer.
///
/// A rewrite's answer is read in an environment built from the arguments
/// innermost-last, so the track it fired on stands at `0` and the mapper —
/// written first — stands at `1`.
const MAPPER: Index = Index(1);

/// `map_note_pitches(f, t)` ⟶ `set_note_pitches(⟦t⟧, [f ⟨p₀⟩, …, f ⟨pₙ⟩])`.
///
/// The mapper is a function, so no δ-rule can ever fire at it: the core answers
/// "not canonical data" at a λ and the spine blocks forever. A rewrite can name
/// it, by the index it stands at, and that is the whole of why this one
/// operation reduces differently from its seven siblings.
///
/// **Termination needs no argument.** §5.8's structural obligation is about a
/// builtin that applies *itself* inside its own answer, and this one applies
/// itself nowhere: the answer names `set_note_pitches` and `f`, and neither is
/// `map_note_pitches`.
fn rewrite_map_note_pitches(builtin: &Builtin, subject: &Literal) -> Option<Term> {
    let track = held::<VoiceTrack>(subject)?;
    let member = plain_type("Pitch");
    let empty = builtin.vocabulary().get(word::EMPTY)?;
    let cons = builtin.vocabulary().get(word::CONS)?;
    let mut mapped = Term::app(HERE, empty.clone(), member.clone());
    for pitch in track
        .occurrences()
        .iter()
        .filter_map(|occurrence| occurrence.payload().sounding_pitch())
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
    {
        let at = Term::app(HERE, Term::var(HERE, MAPPER), literal(member.clone(), pitch).term(HERE));
        mapped = Term::app(
            HERE,
            Term::app(HERE, Term::app(HERE, cons.clone(), member.clone()), at),
            mapped,
        );
    }
    Some(Term::app(
        HERE,
        Term::app(HERE, builtin.vocabulary().get(word::SET)?.clone(), subject.term(HERE)),
        mapped,
    ))
}
