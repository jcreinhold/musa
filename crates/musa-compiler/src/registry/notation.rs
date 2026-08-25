//! The words a notated block is built out of, as core builtins.
//!
//! `00-semantics.md` §3 deletes the contextual `music` value and says what
//! replaces it: "a fragment is a value of type `EventTrack[WrittenTime,
//! ScoreFact]`", "placement is applied by the enclosing voice's left fold", and
//! composition "is therefore the core's own operations, with no separate
//! equation set to maintain". The core's own operations were eight, and exactly
//! one of them — [`super::track`]'s `play` — makes a fact at all. It makes
//! `FactKind::Note` and nothing else, out of the nineteen kinds a score says.
//!
//! Five words are missing and this module supplies them — four as
//! registrations, one as a literal:
//!
//! - `sounded` puts **one** fact of any kind over `[0, held]`, which is what
//!   every notation statement but a chord contributes.
//! - `follow` places one track after another. It had no word because in the
//!   deleted design placement was the evaluator's *cursor* rather than an
//!   operation: a fragment was instantiated at a context, so nothing ever had to
//!   name the act of placing.
//! - `tied` marks what a `~` was written on, and `joined` is where a tie stops
//!   existing. Two words rather than one because a tie is a property of *two*
//!   facts (roadmap §6.3): the notehead can only say "I continue", and what it
//!   continues into is not known until the block that holds both has been
//!   composed. `sounded` therefore writes no tie and never could — it builds one
//!   fact and a tie is a relation between two.
//! - `nothing` is `follow`'s identity — the empty block, the voice with no
//!   statements, and the seed of the fold — and it is a *literal* rather than a
//!   registration for the reason given at [`nothing`].
//!
//! # Why none of the five is a source word
//!
//! For `sounded`, it is `set_note_pitches`'s argument one domain over
//! ([`super::track`]'s `TRACK_BEYOND`): a registered word that would be the
//! control removed. `00-semantics.md` §3 says a block may not contain a key,
//! meter, tempo, or clef change, "because 'from here onward' has no unique
//! meaning in a value usable at several places" — and a source `sounded` would
//! hand every program the ability to put a `Fact.Key` in the middle of a voice.
//! Keeping the word out of the ownership tables makes that a property of the
//! vocabulary instead of a check somewhere. `follow` and `nothing` are out for a
//! weaker reason worth stating: `use` is the source spelling of sequencing
//! (`01-surface.md` §2), §3's worked programs write `together`, `shift`, and
//! `map_note_pitches` and never write `follow`, and a second source spelling for
//! sequencing is a surface change with its own evidence. `tied` and `joined` are
//! out because `~` is already the source spelling of the whole idea, and a
//! *called* pair would let a program mark material it did not write as
//! continuing, or join two noteheads a composer wrote as two.
//!
//! # Why `sounded` admits a span of no length and `play` does not
//!
//! A chord that sounds for no time is not a sounding, and `play` refuses it. A
//! *fact* of no length is ordinary: `FactKind::Mark` is "a point at the instant
//! it is written, or a span over the music its block covers", a grace note is a
//! point occurrence at its principal's onset, and a dynamic applies from an
//! onset. So `sounded` refuses a negative length and admits zero, and the two
//! rules differ because the two questions do.

#[cfg(test)]
mod laws;

use musa_calculus::{Builtin, Cx, Datum, ElabError, Family, Literal, Rule};
use musa_events::{Occurrence, Position, Span};
use num_rational::Ratio;

use super::plain_type;
use super::rules::{halves, items, nat, read, reduced, refused};
use super::track::{Provenance, built, scope_of, track_of, track_type};
use crate::elaborate::{FactKind, ScoreFact, VoiceTrack};
use musa_score::harmony::ChordSymbol;
use musa_score::marks::MarkArgument;
use musa_score::score::{Clef, DynamicMark, FreeDuration, Metronome, NotatedDuration, Ramp};

/// The four operations this module registers, in the order [`builtins`] writes
/// them.
///
/// Named rather than counted, for [`super::traversal::SPELLINGS`]'s reason:
/// "which four" is the claim the accounting law checks, and a count agrees with
/// a wrong set as readily as with the right one. `nothing` is not here because
/// it is not registered — see [`nothing`].
pub(super) const BEYOND: [&str; 5] = ["sounded", "follow", "tied", "joined", "notated_duration"];

/// A payload whose domain has no written spelling.
///
/// [`super::Domain`] asks for `Display`, and nine of `Fact`'s payload types have
/// none: a `Clef`, a `Ramp`, a `Progress` are values the language reads and
/// prints through a backend rather than through a `Display`, and
/// `musa_events::Progress` could not be given one here in any case. The
/// precedent is [`Provenance`], one module over, and the choice this makes is
/// the honest one — a payload with no spelling shows as its `Debug`, rather than
/// acquiring a rendering nobody asked for in front of every formatter in the
/// compiler.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Opaque<T>(pub(super) T);

impl<T: std::fmt::Debug> std::fmt::Display for Opaque<T> {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(out, "{:?}", self.0)
    }
}

/// A literal of a domain with no written spelling.
pub(super) fn opaque<T>(name: &'static str, value: T) -> Datum
where
    T: Clone + PartialEq + std::fmt::Debug + Send + Sync + 'static,
{
    Datum::Lit(super::opaque_literal(name, value))
}

/// The value inside such a literal.
fn unwrapped<T>(datum: &Datum) -> Option<T>
where
    T: Clone + PartialEq + std::fmt::Debug + Send + Sync + 'static,
{
    read::<Opaque<T>>(datum).map(|held| held.0)
}

/// The four registrations, at the types `../../rules/language/00-semantics.md`
/// §3 gives them.
///
/// # Errors
///
/// [`ElabError`] when `Scope` or `Fact` is not declared in `cx`, which is a
/// defect in this compiler rather than in any program.
pub(super) fn builtins(cx: &Cx) -> Result<Vec<Builtin>, ElabError> {
    let track = track_type;
    Ok(vec![
        Builtin::new(
            BEYOND[0],
            super::arrow(
                vec![
                    plain_type("Origin"),
                    crate::prelude::constant(cx, "Scope")?,
                    crate::prelude::constant(cx, "Fact")?,
                    super::tagged_type("Duration", crate::phase::Coordinate::WrittenTime),
                ],
                track(),
            ),
            Family::Track,
            SOUNDED,
        ),
        Builtin::new(
            BEYOND[1],
            super::arrow(vec![track(), track()], track()),
            Family::Track,
            FOLLOW,
        ),
        Builtin::new(BEYOND[2], super::arrow(vec![track()], track()), Family::Track, TIED),
        Builtin::new(BEYOND[3], super::arrow(vec![track()], track()), Family::Track, JOINED),
        Builtin::new(
            BEYOND[4],
            super::arrow(
                vec![super::tagged_type("Duration", crate::phase::Coordinate::WrittenTime)],
                super::plain_type("NotatedDuration"),
            ),
            Family::Delta,
            NOTATED,
        ),
    ])
}

/// `notated_duration(d)` — the `NotatedDuration` a *computed* duration stands
/// in for, spelled the way [`NotatedDuration::spelled`] spells one.
///
/// The lowering inserts this word where a duration position holds a term — a
/// motif's `d: Duration<WrittenTime>` played as `c4 d` — because the payload
/// every other fact carries is baked in at lowering time and a parameter has
/// no value until evaluation. Not a table row for the reason
/// [`super::rules::BEYOND`] gives: a composer writes a fraction, never this
/// conversion, and a source-visible name for it would be a second way to
/// spell what `c4/4` already says. The spelling the rule derives is the one
/// honest answer a computed duration has — the CST's record of the written
/// form is exactly what a parameter does not have.
const NOTATED: Rule = |arguments| {
    let value = read::<Ratio<i64>>(arguments.first()?)?;
    reduced(opaque(
        "NotatedDuration",
        musa_score::score::NotatedDuration::spelled(value),
    ))
};

/// `nothing` — the track of no duration and no occurrences.
///
/// A literal rather than a registration, and the reduction rules are the reason
/// rather than a preference. A δ-rule fires "at the moment the last argument
/// arrives" (`02-core-calculus.md` §5.8), so a rule of no arguments never fires
/// at all; a [`musa_calculus::Builtin::constructor`] has no reduction by design, so
/// `follow(nothing, t)` would be handed a rigid neutral where [`FOLLOW`] expects
/// canonical data, and the spine would block forever. Neither shape is a near
/// miss — both say "operation", and `nothing` is not one.
///
/// Nor is it a [`super::Definition`], which is the shape one module up that looks
/// closest. `run_syntax_step` needs one *because a λ has no inferable type*, so
/// handing back the value alone would leave its caller to rebuild the type. A
/// literal at a base type carries its own, so a `Definition` here would be a
/// type the value already states, written twice.
pub(crate) fn nothing() -> Literal {
    let empty: VoiceTrack = musa_events::empty(musa_events::Duration::ZERO);
    super::literal_with_shape(track_type(), empty, (1, 16))
}

/// `sounded(origin, scope, fact, held)` — one fact, over `[0, held]`, in a track
/// of exactly `held`.
///
/// §5.7's construction clause is discharged the way `play` discharges it: the
/// origin and the scope are arguments, because a source span and a place in the
/// score's structure are the two things a `fn` pointer cannot invent.
const SOUNDED: Rule = |arguments| {
    let Datum::Lit(ref written) = *arguments.first()? else {
        return None;
    };
    let origin = &held_origin(written)?.0;
    let scope = scope_of(arguments.get(1)?)?;
    let kind = fact_of(arguments.get(2)?)?;
    let held = read::<Ratio<i64>>(arguments.get(3)?)?;
    if held < Ratio::ZERO {
        return Some(refused("a fact lasts for no less than no time at all"));
    }
    let Ok(span) = Span::new(Position::ZERO, Position::new(held)) else {
        return Some(refused("a fact lasts for no less than no time at all"));
    };
    let fact = ScoreFact {
        scope,
        kind,
        origin: origin.clone(),
        tied: false,
    };
    let Ok(sounded) = musa_events::track(span.duration(), vec![Occurrence::new(span, fact)]) else {
        return Some(refused("the fact does not fit the length it was given"));
    };
    reduced(built(sounded))
};

/// `follow(first, next)` — `next` placed after `first`, in a track as long as
/// both.
///
/// Total, which is why it answers a track rather than a `Result`:
/// `musa_events::follow` translates every occurrence of `next` by `first`'s
/// duration and adds the two durations, and neither step has a failing case —
/// `../../rules/events/04-algebraic-laws.md` proves the associativity and the
/// duration additivity this rule inherits rather than restates.
const FOLLOW: Rule = |arguments| {
    let first = track_of(arguments.first()?)?;
    let next = track_of(arguments.get(1)?)?;
    reduced(built(musa_events::follow(vec![first, next])))
};

/// `tied(t)` — every fact of `t` continues into whatever follows it.
///
/// Marks the whole track and not "its last statement", because the only thing
/// this is ever applied to is one notehead statement: `g4/4 ~` is one `sounded`
/// and `[c4 e4]/2 ~` is a `together` of two, and in both the written `~` is
/// about all of them. Widening it to a general "tie the end of this block" would
/// be a word for a shape no source writes.
///
/// Total: a track with no occurrences is marked to no effect, which is what a
/// `~` on a rest already means one line down.
const TIED: Rule = |arguments| {
    let track = track_of(arguments.first()?)?;
    let occurrences: Vec<Occurrence<_, ScoreFact>> = track
        .occurrences()
        .iter()
        .map(|occurrence| {
            let mut fact = occurrence.payload().clone();
            fact.tied = true;
            Occurrence::new(occurrence.span(), fact)
        })
        .collect();
    let Ok(marked) = musa_events::track(track.duration(), occurrences) else {
        return Some(refused("a tie changes no time and this one did"));
    };
    reduced(built(marked))
};

/// `joined(t)` — tied noteheads read as the single sounds they spell
/// (roadmap §6.3: a tie is duration structure, not an annotation).
///
/// This is where a tie stops existing. The merged occurrence's span is the sum,
/// its written duration is the compound spelling, its articulations are both
/// noteheads', and nothing downstream ever sees a tie flag.
///
/// # Why it is applied once per voice and not by `follow`
///
/// Because both of its refusals are about the *whole* of a voice. "Nothing to
/// tie to" is only true at the end of one: a tie at the end of a repeat body or
/// a slur continues into whatever comes after the block, and a `follow` that
/// judged its own two arguments would report that as a dangling tie in every
/// nested block. Merging at the voice sees the composed music once, which is
/// also the only place the question has an answer.
const JOINED: Rule = |arguments| {
    let track = track_of(arguments.first()?)?;
    if !track.occurrences().iter().any(|occurrence| occurrence.payload().tied) {
        return reduced(built(track));
    }
    let mut merged: Vec<Vec<Occurrence<_, ScoreFact>>> = Vec::new();
    for statement in statements(track.occurrences()) {
        let continues = merged
            .last()
            .and_then(|previous| previous.first())
            .is_some_and(|first| first.payload().tied);
        match merged.last_mut().filter(|_| continues) {
            Some(previous) if same_sound(previous, &statement) => join(previous, &statement),
            Some(_) => return Some(refused("a tie joins two of the same note")),
            _ => merged.push(statement),
        }
    }
    if merged
        .last()
        .and_then(|last| last.first())
        .is_some_and(|first| first.payload().tied)
    {
        return Some(refused("this tie has nothing to tie to"));
    }
    let Ok(joined) = musa_events::track(track.duration(), merged.into_iter().flatten().collect()) else {
        return Some(refused("a tied note reaches past the music it is written in"));
    };
    reduced(built(joined))
};

/// One occurrence of a written-time track, which is all three helpers below
/// take and all any of them answers.
type Written = Occurrence<musa_events::WrittenTime, ScoreFact>;

/// The occurrences grouped into *statements*: one written note or rest, or the
/// pitches of one chord, which share a span and an origin.
///
/// Region and point facts are statements of one and never merge: only a notehead
/// can be tied.
fn statements(occurrences: &[Written]) -> Vec<Vec<Written>> {
    let mut grouped: Vec<Vec<Occurrence<_, ScoreFact>>> = Vec::with_capacity(occurrences.len());
    for occurrence in occurrences {
        let joins = grouped.last().and_then(|group| group.first()).is_some_and(|first| {
            first.span() == occurrence.span()
                && first.payload().origin == occurrence.payload().origin
                && first.payload().pitch_of().is_some()
                && occurrence.payload().pitch_of().is_some()
        });
        match (joins, grouped.last_mut()) {
            (true, Some(group)) => group.push(occurrence.clone()),
            _ => grouped.push(vec![occurrence.clone()]),
        }
    }
    grouped
}

/// Whether two statements are the same sound: the same pitches, in order.
fn same_sound(left: &[Written], right: &[Written]) -> bool {
    let pitches = |statement: &[Written]| {
        statement
            .iter()
            .map(|occurrence| occurrence.payload().pitch_of())
            .collect::<Option<Vec<_>>>()
    };
    match (pitches(left), pitches(right)) {
        (Some(left), Some(right)) => !left.is_empty() && left == right,
        _ => false,
    }
}

/// Extend `previous` through `statement`: one occurrence per pitch, spanning
/// both, spelled as the noteheads the composer wrote.
fn join(previous: &mut [Written], statement: &[Written]) {
    let Some(end) = statement.first().map(|first| first.span().end()) else {
        return;
    };
    // The tie travels: three noteheads tied in a row are one sound, and the
    // second one's `~` is what says the third belongs to it.
    let tied = statement.first().is_some_and(|first| first.payload().tied);
    for (index, occurrence) in previous.iter_mut().enumerate() {
        let mut fact = occurrence.payload().clone();
        fact.tied = tied;
        if let (
            FactKind::Note {
                duration,
                articulations,
                ..
            },
            Some(next),
        ) = (&mut fact.kind, statement.get(index).map(Occurrence::payload))
        {
            if let Some(added) = next.kind.duration_of() {
                *duration = duration.tied_to(added);
            }
            if let FactKind::Note {
                articulations: more, ..
            } = &next.kind
            {
                articulations.extend(more.iter().copied());
            }
        }
        let span = Span::new(occurrence.span().start(), end).unwrap_or_else(|_| occurrence.span());
        *occurrence = Occurrence::new(span, fact);
    }
}

/// The origin a literal holds, as [`super::track`] wraps one.
fn held_origin(value: &musa_calculus::Literal) -> Option<&Provenance> {
    super::held::<Provenance>(value)
}

/// The fact a `Fact` datum stands for.
///
/// The inverse of [`crate::prelude`]'s declaration, and one half of the
/// correspondence its doc claims: this reads a constructor into a
/// [`FactKind`], and `laws::written` writes one back out by a second hand-written
/// path, so a broken reading cannot make the round trip pass.
///
/// A constructor this compiler did not declare, a field missing, or a payload of
/// the wrong domain all answer [`None`], which is D2's contract: the core reports
/// a builtin applied to what its signature does not admit, and a rule does not
/// crash over it.
fn fact_of(datum: &Datum) -> Option<FactKind> {
    let Datum::Case {
        ref constructor,
        ref fields,
    } = *datum
    else {
        return None;
    };
    let at = |index: usize| fields.get(index);
    let marks = |index: usize| {
        items(at(index)?)?
            .into_iter()
            .map(read::<musa_score::Mark>)
            .collect::<Option<Vec<_>>>()
    };
    let free = |index: usize| Some(optional(at(index)?, unwrapped::<FreeDuration>)?.value());
    let count = |index: usize| u32::try_from(nat(at(index)?)?).ok();
    Some(match &**constructor {
        "Fact.Note" => FactKind::Note {
            pitch: read(at(0)?)?,
            duration: unwrapped::<NotatedDuration>(at(1)?)?,
            articulations: marks(2)?,
            free: free(3)?,
        },
        "Fact.Rest" => FactKind::Rest {
            duration: unwrapped::<NotatedDuration>(at(0)?)?,
            articulations: marks(1)?,
            free: free(2)?,
        },
        "Fact.Mark" => FactKind::Mark {
            mark: read(at(0)?)?,
            argument: optional(at(1)?, read::<MarkArgument>)?.value(),
        },
        "Fact.Grace" => FactKind::Grace {
            pitch: read(at(0)?)?,
            articulations: marks(1)?,
            index: u8::try_from(nat(at(2)?)?).ok()?,
        },
        "Fact.Slur" => FactKind::Slur,
        "Fact.Phrase" => FactKind::Phrase { name: read(at(0)?)? },
        "Fact.Tuplet" => FactKind::Tuplet {
            num: count(0)?,
            den: count(1)?,
        },
        "Fact.Dynamic" => FactKind::Dynamic {
            mark: unwrapped::<DynamicMark>(at(0)?)?,
        },
        "Fact.Hairpin" => FactKind::Hairpin {
            grows: boolean(at(0)?)?,
            target: unwrapped::<DynamicMark>(at(1)?)?,
            shape: unwrapped::<musa_events::Progress>(at(2)?)?,
        },
        "Fact.Key" => {
            let key = read::<musa_score::Key>(at(0)?)?;
            FactKind::Key {
                tonic: key.tonic(),
                mode: key.mode(),
            }
        }
        "Fact.Meter" => FactKind::Meter {
            numerator: count(0)?,
            denominator: count(1)?,
        },
        "Fact.Clef" => FactKind::Clef {
            clef: unwrapped::<Clef>(at(0)?)?,
        },
        "Fact.Tempo" => FactKind::Tempo {
            metronome: optional(at(0)?, unwrapped::<Metronome>)?.value(),
            text: optional(at(1)?, read::<String>)?.value(),
            ramp: optional(at(2)?, unwrapped::<Ramp>)?.value(),
        },
        "Fact.Section" => FactKind::Section { name: read(at(0)?)? },
        "Fact.Harmony" => FactKind::Harmony {
            symbol: unwrapped::<ChordSymbol>(at(0)?)?,
        },
        "Fact.Repeat" => FactKind::Repeat {
            times: count(0)?,
            range: optional(at(1)?, pair)?.value(),
        },
        "Fact.Mobile" => FactKind::Mobile {
            fragments: items(at(0)?)?
                .into_iter()
                .map(read::<String>)
                .collect::<Option<Vec<_>>>()?,
            order: items(at(1)?)?
                .into_iter()
                .map(|order| u32::try_from(nat(order)?).ok())
                .collect::<Option<Vec<_>>>()?,
        },
        "Fact.Improvise" => FactKind::Improvise {
            over: optional(at(0)?, read::<String>)?.value(),
        },
        "Fact.Ending" => FactKind::Ending {
            bracket: count(0)?,
            pass: count(1)?,
        },
        _ => return None,
    })
}

/// What an `Option`-typed field of a `Fact` says: a value, or that the score
/// wrote none.
///
/// Named rather than left as an inner [`Option`], because the two nested
/// `Option`s of `Option<Option<T>>` would be answering two different questions
/// at the same call site: whether the *datum* is an `Option` at all, and whether
/// what the composer wrote is there. The first is a defect in this compiler and
/// the second is an ordinary score, so they read better under two names.
enum Given<T> {
    /// `Option.None` — the score wrote nothing here.
    Nothing,
    /// `Option.Some(v)`, read into the domain the field stands at.
    Something(T),
}

impl<T> Given<T> {
    /// The value, as `FactKind` writes it.
    fn value(self) -> Option<T> {
        match self {
            Self::Nothing => None,
            Self::Something(value) => Some(value),
        }
    }
}

/// What an `Option` datum holds, read by `each`.
///
/// [`None`] where a defect in this compiler put something that is not an
/// `Option` here, or where `each` could not read the payload — D2's contract,
/// and the same answer [`read`] gives for a literal of the wrong domain.
fn optional<T>(datum: &Datum, each: impl Fn(&Datum) -> Option<T>) -> Option<Given<T>> {
    let Datum::Case {
        ref constructor,
        ref fields,
    } = *datum
    else {
        return None;
    };
    match &**constructor {
        "Option.None" => Some(Given::Nothing),
        "Option.Some" => each(fields.first()?).map(Given::Something),
        _ => None,
    }
}

/// What a `Bool` datum stands for.
fn boolean(datum: &Datum) -> Option<bool> {
    let Datum::Case { ref constructor, .. } = *datum else {
        return None;
    };
    match &**constructor {
        "Bool.False" => Some(false),
        "Bool.True" => Some(true),
        _ => None,
    }
}

/// The two whole numbers a `Pair Nat Nat` holds.
fn pair(datum: &Datum) -> Option<(u32, u32)> {
    let (first, second) = halves(datum)?;
    Some((u32::try_from(nat(first)?).ok()?, u32::try_from(nat(second)?).ok()?))
}
