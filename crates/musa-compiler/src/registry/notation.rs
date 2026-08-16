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
//! Three words are missing and this module supplies them — two as
//! registrations, one as a literal:
//!
//! - `sounded` puts **one** fact of any kind over `[0, held]`, which is what
//!   every notation statement but a chord contributes.
//! - `follow` places one track after another. It had no word because in the
//!   deleted design placement was the evaluator's *cursor* rather than an
//!   operation: a fragment was instantiated at a context, so nothing ever had to
//!   name the act of placing.
//! - `nothing` is `follow`'s identity — the empty block, the voice with no
//!   statements, and the seed of the fold — and it is a *literal* rather than a
//!   registration for the reason given at [`nothing`].
//!
//! # Why none of the three is a source word
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
//! sequencing is a surface change with its own evidence.
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

use musa_core::{Builtin, Cx, Datum, ElabError, Family, Literal, Rule};
use musa_kernel::{Occurrence, Position, Span};
use num_rational::Ratio;

use super::rules::{answered as ok, items, nat, read, refused as err};
use super::track::{Provenance, built, scope_of, track_of, track_type};
use super::{literal, plain_type};
use crate::elaborate::{FactKind, ScoreFact, VoiceTrack};
use crate::harmony::ChordSymbol;
use crate::marks::MarkArgument;
use crate::score::{Clef, DynamicMark, FreeDuration, Metronome, Mode, NotatedDuration, Ramp};

/// The two operations this module registers, in the order [`builtins`] writes
/// them.
///
/// Named rather than counted, for [`super::traversal::SPELLINGS`]'s reason:
/// "which two" is the claim the accounting law checks, and a count agrees with a
/// wrong set as readily as with the right one. `nothing` is not here because it
/// is not registered — see [`nothing`].
pub(super) const BEYOND: [&str; 2] = ["sounded", "follow"];

/// A payload whose domain has no written spelling.
///
/// [`super::Domain`] asks for `Display`, and nine of `Fact`'s payload types have
/// none: a `Clef`, a `Ramp`, a `Progress` are values the language reads and
/// prints through a backend rather than through a `Display`, and
/// `musa_kernel::Progress` could not be given one here in any case. The
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
    Datum::Lit(literal(plain_type(name), Opaque(value)))
}

/// The value inside such a literal.
fn unwrapped<T>(datum: &Datum) -> Option<T>
where
    T: Clone + PartialEq + std::fmt::Debug + Send + Sync + 'static,
{
    read::<Opaque<T>>(datum).map(|held| held.0)
}

/// The two registrations, at the types `../../rules/language/00-semantics.md`
/// §3 gives them.
///
/// # Errors
///
/// [`ElabError`] when `Result`, `Text`, `Scope`, or `Fact` is not declared in
/// `cx`, which is a defect in this compiler rather than in any program.
pub(super) fn builtins(cx: &Cx) -> Result<Vec<Builtin>, ElabError> {
    let track = track_type;
    let fallible = super::applied(cx, "Result", [track(), plain_type("Text")])?;
    Ok(vec![
        Builtin::new(
            BEYOND[0],
            super::arrow(
                vec![
                    plain_type("Origin"),
                    crate::prelude::constant(cx, "Scope")?,
                    crate::prelude::constant(cx, "Fact")?,
                    super::tagged_type("Duration", crate::core::Coordinate::WrittenTime),
                ],
                fallible,
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
    ])
}

/// `nothing` — the track of no duration and no occurrences.
///
/// A literal rather than a registration, and the reduction rules are the reason
/// rather than a preference. A δ-rule fires "at the moment the last argument
/// arrives" (`02-core-calculus.md` §5.8), so a rule of no arguments never fires
/// at all; a [`musa_core::Builtin::constructor`] has no reduction by design, so
/// `follow(nothing, t)` would be handed a rigid neutral where [`FOLLOW`] expects
/// canonical data, and the spine would block forever. Neither shape is a near
/// miss — both say "operation", and `nothing` is not one.
///
/// Nor is it a [`super::Definition`], which is the shape one module up that looks
/// closest. `run_syntax_step` needs one *because a λ has no inferable type*, so
/// handing back the value alone would leave its caller to rebuild the type. A
/// literal at a base type carries its own, so a `Definition` here would be a
/// type the value already states, written twice.
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "prompt 141k writes the fold this seeds; until then the laws are the caller"
    )
)]
pub(crate) fn nothing() -> Literal {
    let empty: VoiceTrack = musa_kernel::empty(musa_kernel::Duration::ZERO);
    literal(track_type(), empty)
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
        return Some(err("a fact lasts for no less than no time at all"));
    }
    let Ok(span) = Span::new(Position::ZERO, Position::new(held)) else {
        return Some(err("a fact lasts for no less than no time at all"));
    };
    let fact = ScoreFact {
        scope,
        kind,
        origin: origin.clone(),
        tied: false,
    };
    let Ok(sounded) = musa_kernel::track(span.duration(), vec![Occurrence::new(span, fact)]) else {
        return Some(err("the fact does not fit the length it was given"));
    };
    Some(ok(built(sounded)))
};

/// `follow(first, next)` — `next` placed after `first`, in a track as long as
/// both.
///
/// Total, which is why it answers a track rather than a `Result`:
/// `musa_kernel::follow` translates every occurrence of `next` by `first`'s
/// duration and adds the two durations, and neither step has a failing case —
/// `../../rules/kernel/04-algebraic-laws.md` proves the associativity and the
/// duration additivity this rule inherits rather than restates.
const FOLLOW: Rule = |arguments| {
    let first = track_of(arguments.first()?)?;
    let next = track_of(arguments.get(1)?)?;
    Some(built(musa_kernel::follow(vec![first, next])))
};

/// The origin a literal holds, as [`super::track`] wraps one.
fn held_origin(value: &musa_core::Literal) -> Option<&Provenance> {
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
            .map(read::<crate::Mark>)
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
            shape: unwrapped::<musa_kernel::Progress>(at(2)?)?,
        },
        "Fact.Key" => FactKind::Key {
            tonic: read(at(0)?)?,
            mode: unwrapped::<Mode>(at(1)?)?,
        },
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
    let Datum::Case {
        ref constructor,
        ref fields,
    } = *datum
    else {
        return None;
    };
    if &**constructor != "Pair.Both" {
        return None;
    }
    Some((
        u32::try_from(nat(fields.first()?)?).ok()?,
        u32::try_from(nat(fields.get(1)?)?).ok()?,
    ))
}
