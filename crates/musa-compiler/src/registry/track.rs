//! The eight ways to build or transform an event track, as core builtins.
//!
//! §5.8's *third* family. A track builtin is neither a δ-builtin nor a
//! structural eliminator: `02-core-calculus.md` §5.8 gives it its own
//! admissibility argument — §5.7's, about construction safety — and
//! [`musa_core::Family::Track`] is where that is recorded. Seven of the eight
//! still *reduce* the way a δ-builtin does, because data in and data out is what
//! they do; `map_note_pitches` takes a mapper and reduces the way a traversal
//! does. That is why [`musa_core::Builtin::structural_with`] takes the family
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
//! [`musa_kernel::EventTrack::scale`], and [`musa_kernel::together`]. The
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
//! handed facts that have one and append their own step —
//! [`ExpansionStep::Transposition`], `Stretch`, `Retrograde`, `Inversion`, and
//! `MapNotePitches` each carry no span, so a `fn` pointer can build one out of
//! its own arguments. `play` *constructs*, and a source span is not something a
//! `fn` pointer can invent, so its origin and its scope are arguments: the one
//! thing a rule cannot compute is the thing the caller supplies, exactly as
//! `instantiate_quote` takes the anchor it builds under.

#[cfg(test)]
mod laws;

use musa_core::{Builtin, Cx, Datum, ElabError, Family, Index, Literal, Rule, Term};
use musa_kernel::{Duration, Occurrence, Position, Span};
use num_rational::Ratio;

use super::rules::{answered as ok, items, nat, read, refused as err};
use super::{HERE, held, literal, plain_type, tagged_type};
use crate::Interval;
use crate::core::Coordinate;
use crate::elaborate::{FactKind, ScoreFact, VoiceTrack};
use crate::origin::{ExpansionStep, Origin};
use crate::pitch::WrittenPitch;
use crate::scope::Scope;
use crate::score::NotatedDuration;

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

/// The one operation this module registers that neither ownership table names.
///
/// [`super::rules::BEYOND`]'s reason, in a second place. `map_note_pitches` is
/// §5.7's *controlled* transform: it applies a mapper to each written pitch and
/// puts the answers back where they came from, and the control is exactly that a
/// program cannot choose the places. `set_note_pitches` is the putting-back
/// half, so a source word for it would be the control removed — it would let any
/// list of pitches into any track's noteheads.
pub(super) const TRACK_BEYOND: [&str; 1] = ["set_note_pitches"];

/// The term naming an event track in written time.
///
/// The only instantiation this compiler has, and the one every signature below
/// is written at. `EventTrack` is registered at `Coordinate → Type 0`, so this
/// is the same application `Duration ⟨written⟩` is.
fn track_type() -> Term {
    tagged_type("EventTrack", Coordinate::WrittenTime)
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
/// [`ElabError`] when `Result`, `List`, `Scope`, or a `List` constructor is not
/// declared in `cx`, which is a defect in this compiler rather than in any
/// program.
pub(super) fn builtins(cx: &Cx) -> Result<Vec<Builtin>, ElabError> {
    let track = track_type;
    let beat = || tagged_type("Duration", Coordinate::WrittenTime);
    let fallible = super::applied(cx, "Result", [track(), plain_type("Text")])?;
    let delta = |name: &'static str, arguments: Vec<Term>, result: Term, rule: Rule| {
        Builtin::new(name, super::arrow(arguments, result), Family::Track, rule)
    };
    Ok(vec![
        delta(
            SPELLINGS[0],
            vec![plain_type("Interval"), track()],
            fallible.clone(),
            TRANSPOSE,
        ),
        delta(
            SPELLINGS[1],
            vec![plain_type("Ratio"), track()],
            fallible.clone(),
            STRETCH,
        ),
        delta(SPELLINGS[2], vec![track()], track(), RETROGRADE),
        delta(
            SPELLINGS[3],
            vec![plain_type("Pitch"), track()],
            fallible.clone(),
            INVERT,
        ),
        delta(SPELLINGS[4], vec![beat(), track()], fallible.clone(), SHIFT),
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
            fallible,
            PLAY,
        ),
        set_note_pitches(cx)?,
    ])
}

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
fn track_of(datum: &Datum) -> Option<VoiceTrack> {
    let Datum::Lit(ref value) = *datum else {
        return None;
    };
    held::<VoiceTrack>(value).cloned()
}

/// A track, as a literal at `EventTrack ⟨written⟩`.
fn built(track: VoiceTrack) -> Datum {
    Datum::Lit(literal(track_type(), track))
}

/// The same track with each fact rewritten by `each` and `step` appended to
/// every fact's expansion path.
///
/// One helper for four of the eight, because "transform each payload and record
/// that it happened" is what a controlled transform *is*: a version that let a
/// caller do one without the other would be the provenance hole §7 exists to
/// close. `each` answers `None` for a fact it cannot transform, and the whole
/// track is then refused rather than the untransformed fact silently kept.
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
            fact.origin.expansion_path.push(step.clone());
            Some(Occurrence::new(occurrence.span(), fact))
        })
        .collect::<Option<Vec<_>>>()?;
    musa_kernel::track(track.duration(), occurrences).ok()
}

// ---- the seven rules ----

/// `transpose(interval, t)` — every written pitch raised, every other fact left
/// where it is.
const TRANSPOSE: Rule = |arguments| {
    let interval = read::<Interval>(arguments.first()?)?;
    let track = track_of(arguments.get(1)?)?;
    let step = ExpansionStep::Transposition(interval);
    Some(rewritten(&track, &step, |fact| fact.transposed(interval)).map_or_else(
        || err("a transposed pitch does not fit the written-pitch coordinate"),
        |raised| ok(built(raised)),
    ))
};

/// `stretch(factor, t)` — the same music notated `factor` times as long, in a
/// track that much longer.
const STRETCH: Rule = |arguments| {
    let factor = read::<Ratio<i64>>(arguments.first()?)?;
    let track = track_of(arguments.get(1)?)?;
    let Ok(scaled) = track.scale(factor) else {
        return Some(err("a stretch factor is greater than zero"));
    };
    let step = ExpansionStep::Stretch(factor);
    Some(
        rewritten(&scaled, &step, |fact| Some(fact.stretched(factor))).map_or_else(
            || err("the stretched music leaves the track it is in"),
            |stretched| ok(built(stretched)),
        ),
    )
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
            fact.origin.expansion_path.push(ExpansionStep::Retrograde);
            Some(Occurrence::new(mirrored, fact))
        })
        .collect::<Option<Vec<_>>>()?;
    Some(built(musa_kernel::track(track.duration(), occurrences).ok()?))
};

/// `invert(axis, t)` — every written pitch mirrored about `axis`.
const INVERT: Rule = |arguments| {
    let axis = read::<WrittenPitch>(arguments.first()?)?;
    let track = track_of(arguments.get(1)?)?;
    let step = ExpansionStep::Inversion { axis: axis.to_string() };
    Some(rewritten(&track, &step, |fact| fact.inverted(axis)).map_or_else(
        || err("an inverted pitch does not fit the written-pitch coordinate"),
        |mirrored| ok(built(mirrored)),
    ))
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
        return Some(err("music cannot be shifted to before the start"));
    };
    let occurrences = track
        .occurrences()
        .iter()
        .map(|occurrence| Occurrence::new(occurrence.span().translate(offset), occurrence.payload().clone()))
        .collect();
    let Ok(shifted) = musa_kernel::track(track.duration().plus(offset), occurrences) else {
        return Some(err("the shifted music leaves the track it is in"));
    };
    Some(ok(built(shifted)))
};

/// `together(a, b)` — both at once, in a track as long as the longer.
///
/// The kernel's own stacking, which is D3's `max(d, e)` and `E ⊎ F`. §5.7's
/// composition clause is discharged by calling the operation the clause is about
/// rather than by re-deriving it here.
const TOGETHER: Rule = |arguments| {
    let left = track_of(arguments.first()?)?;
    let right = track_of(arguments.get(1)?)?;
    Some(built(musa_kernel::together(vec![left, right])))
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
            if let FactKind::Note { ref mut pitch, .. } = fact.kind
                && let Some(replacement) = replacements.next()
            {
                *pitch = replacement;
                fact.origin.expansion_path.push(ExpansionStep::MapNotePitches);
            }
            Occurrence::new(occurrence.span(), fact)
        })
        .collect();
    Some(built(musa_kernel::track(track.duration(), occurrences).ok()?))
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
    let voicing = read::<crate::chord::Voicing>(arguments.get(2)?)?;
    let sounding = read::<Ratio<i64>>(arguments.get(3)?)?;
    if sounding <= Ratio::ZERO {
        return Some(err("a chord sounds for longer than no time at all"));
    }
    let Ok(span) = Span::new(Position::ZERO, Position::new(sounding)) else {
        return Some(err("a chord sounds for longer than no time at all"));
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
    let Ok(sounded) = musa_kernel::track(span.duration(), occurrences) else {
        return Some(err("the chord does not fit the length it was given"));
    };
    Some(ok(built(sounded)))
};

/// The scope a `Scope` datum stands for.
///
/// The inverse of [`crate::prelude`]'s declaration, and the one place the two
/// spellings of a scope meet. A part or voice number past `u32` is not a scope
/// this compiler can name, so it reads as absent rather than as a truncation.
fn scope_of(datum: &Datum) -> Option<Scope> {
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
        .filter_map(|occurrence| occurrence.payload().pitch_of())
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
