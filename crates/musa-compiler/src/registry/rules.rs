//! What each compiler-owned δ operation computes.
//!
//! One rule per row of [`crate::core::BUILTIN_OWNERSHIP`]'s δ half and of
//! [`crate::core::SYNTAX_OWNERSHIP`]'s builder half, translated from the old
//! evaluator's `eval_builtin` and `eval_syntax` rather than rewritten. The
//! arithmetic, the domain calls, and the wording of every refusal are the ones
//! that were there; what changed is only how an argument is read and how an
//! answer is written.
//!
//! # A rule is a `fn`, which is D3 rather than a style
//!
//! [`musa_core::Rule`] is a function pointer, so a rule cannot capture the
//! compiler — no meter, no span, no diagnostic sink, no interning table. §5.8's
//! D3 asks that "the result is a function of the argument values alone", and
//! that is a property of the type here rather than a promise a reader has to
//! audit. Two consequences are visible in what follows:
//!
//! - **The meter is gone from the rule.** `text_join`, `range`, and every
//!   builder used to preflight their own construction against `WorkMeter`. §4's
//!   budget now lives in the core and charges realization there, so a rule
//!   computes and the core charges. Nothing about which programs are accepted
//!   moves: the same size is charged, at the same point, by the one meter that
//!   sees every reduction rather than by each rule for itself.
//! - **`None` is louder than it was.** The old evaluator left an unanswered
//!   application as a stuck term. [`musa_core::Malformed::BuiltinStuck`] now
//!   reports it, which is D2 enforced rather than assumed. Every `None` below
//!   is one the old evaluator also answered — `interval_add` at an interval pair
//!   with no representable sum is the honest example — and each is a signature
//!   that promises more than its rule delivers. They are left as they are and
//!   named here rather than quietly widened to `Option`, because widening one is
//!   a language change and prompt 143 is where the whole family is reconsidered.
//!
//! # Reading is untyped and writing is not
//!
//! A [`Datum`] read needs no type: 141d made `Datum::Case` carry the qualified
//! constructor name, so `Option.Some` is `Option.Some` wherever it came from.
//! Writing one does need a type, and the core supplies it — a `Datum::Case` is
//! realized against the builtin's own declared result type, so a rule names a
//! constructor and never a [`musa_core::Term`]. The exception is `Datum::Lit`,
//! which is used with the type the rule supplied, which is why every writer here
//! goes through [`super::literal`] and the same [`super::plain_type`] the
//! registration used.
//!
//! # `Nat` is unary, and that is a cost worth naming
//!
//! `Nat` is *declared* (`Zero`, `Succ`) rather than registered, because source
//! code pattern-matches on it and a base type has no cases. So `nat_add` writes
//! its answer one `Succ` at a time, and a large sum is a large term. That is the
//! representation the prelude chose and not a defect of this module; prompt 143,
//! which collapses `nat_add` and `ratio_add` behind `Add`, is where a binary
//! representation would be argued for.

use std::sync::Arc;

use musa_core::{Datum, Rule};
use num_rational::Ratio;

use super::{domain, literal, plain_type, syntax_type, tagged_type};
use crate::core::{
    Builtin, Coordinate, Exact, SyntaxOp, exact_arithmetic, exact_ratio, written_interval, written_rational,
};
use crate::origin::Interval;
use crate::pitch::{PitchClass, WrittenPitch};
use crate::syntax::{Cat, Syntax, token_kind_spelling};

/// How many rows of the two ownership tables reach the core's registry.
///
/// 92 of `BUILTIN_OWNERSHIP`'s 117 and 16 of `SYNTAX_OWNERSHIP`'s 17. The two
/// phase rows past this module's fourteen are the traversals in
/// [`super::traversal`]: they are §5.8's second family rather than its first, so
/// they carry a rewrite instead of a [`Rule`] and are registered there.
pub(super) const REGISTERED: usize = 108;

/// The rows that do not, by family and by count.
///
/// Twenty-six, and each group is left for a reason that is about the *core*
/// rather than about effort:
///
/// - a **structural eliminator** traverses `Nat`, `List`, or `Option`, which are
///   declared families with generated recursors, and
///   [`musa_core::Registry::new`] refuses a structural target that is not a base
///   type. Registering one would be a second ι-rule for a type that has one;
///   prompt 142 makes them library code.
/// - a **track** or **machine** builtin needs `EventTrack` or `Machine`, and
///   prompt 142 reshapes both when it deletes contextual `Music`. Registering
///   them against the shape that is about to go would be work thrown away.
/// - a **phase projection** is `run_syntax_step`, which hides nothing: it is the
///   `run` field of a `SyntaxStep` applied to a context, and a projection is not
///   a compiler-owned operation. [`musa_core::Registry::new`] would have refused
///   it anyway, since its target is a declared family and a rewrite over one is
///   the second ι-rule that check exists to catch. It is *defined* instead —
///   [`super::run_syntax_step`].
///
/// The suite counts each group again off the tables themselves, so this array
/// cannot drift from what is actually registered.
pub(super) const UNREGISTERED: [(&str, usize); 4] = [
    ("structural eliminators", 8),
    ("track builtins", 8),
    ("machine builtins", 9),
    ("phase projections", 1),
];

// ---- reading an argument ----

/// The domain value an argument holds.
///
/// One reader for eighteen domains, and the signature is what discriminates: a
/// `Ratio`, a `Duration ⟨written⟩`, and a `Position ⟨written⟩` all carry a
/// `Ratio<i64>`, and a rule declared to take one of them cannot be handed
/// another. The old evaluator needed three functions here because a `Value` was
/// self-describing and the checker's promise was not machine-checkable at this
/// point; the core's is.
fn read<T>(datum: &Datum) -> Option<T>
where
    T: Clone + PartialEq + std::fmt::Debug + std::fmt::Display + Send + Sync + 'static,
{
    domain::<T>(datum).cloned()
}

/// An exact rational argument, whichever of the three types carries it.
fn ratio(datum: &Datum) -> Option<Ratio<i64>> {
    read::<Ratio<i64>>(datum)
}

/// Text.
fn text(datum: &Datum) -> Option<String> {
    read::<String>(datum)
}

/// A whole number, by counting the `Succ`s off a unary `Nat`.
///
/// Iterative rather than recursive: a `Nat` is as deep as it is large, and a
/// recursive count would put the host's stack where the core's budget belongs.
fn nat(datum: &Datum) -> Option<u64> {
    let mut counted: u64 = 0;
    let mut rest = datum;
    loop {
        let Datum::Case {
            ref constructor,
            ref fields,
        } = *rest
        else {
            return None;
        };
        match &**constructor {
            "Nat.Zero" => return Some(counted),
            "Nat.Succ" => {
                counted = counted.checked_add(1)?;
                rest = fields.first()?;
            }
            _ => return None,
        }
    }
}

/// The members of a `List`, in order.
fn items(datum: &Datum) -> Option<Vec<&Datum>> {
    let mut members = Vec::new();
    let mut rest = datum;
    loop {
        let Datum::Case {
            ref constructor,
            ref fields,
        } = *rest
        else {
            return None;
        };
        match &**constructor {
            "List.Empty" => return Some(members),
            "List.Cons" => {
                members.push(fields.first()?);
                rest = fields.get(1)?;
            }
            _ => return None,
        }
    }
}

/// The pitch classes of a `List Pc12`.
fn pc12s(datum: &Datum) -> Option<Vec<crate::pc12::Pc12>> {
    items(datum)?.into_iter().map(read::<crate::pc12::Pc12>).collect()
}

// ---- writing an answer ----

/// A literal of a plain base type.
fn plain<T>(name: &'static str, value: T) -> Datum
where
    T: PartialEq + std::fmt::Debug + std::fmt::Display + Send + Sync + 'static,
{
    Datum::Lit(literal(plain_type(name), value))
}

/// A literal of a base type indexed by a coordinate.
fn tagged<T>(name: &'static str, which: Coordinate, value: T) -> Datum
where
    T: PartialEq + std::fmt::Debug + std::fmt::Display + Send + Sync + 'static,
{
    Datum::Lit(literal(tagged_type(name, which), value))
}

/// A constructor applied to its fields.
fn case(constructor: &'static str, fields: Vec<Datum>) -> Datum {
    Datum::Case {
        constructor: Arc::from(constructor),
        fields,
    }
}

/// `Bool`.
fn boolean(value: bool) -> Datum {
    case(if value { "Bool.True" } else { "Bool.False" }, Vec::new())
}

/// `Nat`, one `Succ` at a time.
fn whole(value: u64) -> Datum {
    let mut built = case("Nat.Zero", Vec::new());
    for _ in 0..value {
        built = case("Nat.Succ", vec![built]);
    }
    built
}

/// `Text`.
fn written(value: String) -> Datum {
    plain("Text", value)
}

/// `Ratio`.
fn exact(value: Ratio<i64>) -> Datum {
    plain("Ratio", value)
}

/// `Duration ⟨written⟩`.
fn duration(value: Ratio<i64>) -> Datum {
    tagged("Duration", Coordinate::WrittenTime, value)
}

/// `Position ⟨written⟩`.
fn position(value: Ratio<i64>) -> Datum {
    tagged("Position", Coordinate::WrittenTime, value)
}

/// `List`, from its members in order.
///
/// A cons list is built from its tail, and the reversal that needs is this
/// function's business rather than its callers'. Requiring a double-ended
/// iterator instead would put the representation in the signature, and one
/// caller — a voicing's pitches — cannot supply one.
fn listing(members: impl IntoIterator<Item = Datum>) -> Datum {
    let mut members: Vec<Datum> = members.into_iter().collect();
    let mut built = case("List.Empty", Vec::new());
    while let Some(last) = members.pop() {
        built = case("List.Cons", vec![last, built]);
    }
    built
}

/// `Option`.
fn optional(value: Option<Datum>) -> Datum {
    value.map_or_else(
        || case("Option.None", Vec::new()),
        |held| case("Option.Some", vec![held]),
    )
}

/// The answering half of a `Result`.
fn answered(value: Datum) -> Datum {
    case("Result.Ok", vec![value])
}

/// The refusing half of a `Result`, whose error type is `Text` in every source
/// operation but `row12_of`.
fn refused(because: &str) -> Datum {
    case("Result.Err", vec![written(because.to_owned())])
}

/// A duration that has to be nonnegative to exist.
///
/// The one place the law is stated, exactly as the old evaluator stated it once.
fn written_duration(value: Ratio<i64>) -> Datum {
    if value < Ratio::ZERO {
        return refused("a duration is nonnegative, and this exact rational is below zero");
    }
    answered(duration(value))
}

/// The four exact-rational operations, which differ only in the operation and in
/// division's own refusal.
fn ratio_arithmetic(operation: Exact, arguments: &[Datum]) -> Option<Datum> {
    let (left, right) = (ratio(arguments.first()?)?, ratio(arguments.get(1)?)?);
    if matches!(operation, Exact::Div) && right == Ratio::ZERO {
        return Some(refused("an exact rational is not divided by zero"));
    }
    Some(match exact_arithmetic(left, right, operation) {
        Some(value) => answered(exact(value)),
        None => refused("these exact rationals have no result this language can represent"),
    })
}

/// The rule for one source operation, or `None` where the core owns the family.
///
/// Written as one arm per row so that a reader comparing this against
/// `eval_builtin` can do it line by line, and so that the 25 rows with no rule
/// are named rather than swept up by a wildcard — the workspace's
/// `wildcard_enum_match_arm` is on for this reason, and here it is doing exactly
/// the job it is on for.
#[expect(
    clippy::too_many_lines,
    reason = "one arm per registered operation; splitting it would hide the correspondence with the table"
)]
pub(super) fn source(operation: Builtin) -> Option<Rule> {
    Some(match operation {
        // ---- exact rationals and text ----
        Builtin::RatioAdd => |arguments| ratio_arithmetic(Exact::Add, arguments),
        Builtin::RatioSub => |arguments| ratio_arithmetic(Exact::Sub, arguments),
        Builtin::RatioMul => |arguments| ratio_arithmetic(Exact::Mul, arguments),
        Builtin::RatioDiv => |arguments| ratio_arithmetic(Exact::Div, arguments),
        Builtin::RatioLess => |arguments| Some(boolean(ratio(arguments.first()?)? < ratio(arguments.get(1)?)?)),
        Builtin::RatioEqual => |arguments| Some(boolean(ratio(arguments.first()?)? == ratio(arguments.get(1)?)?)),
        Builtin::TextEqual => |arguments| Some(boolean(text(arguments.first()?)? == text(arguments.get(1)?)?)),
        Builtin::TextJoin => |arguments| {
            let mut joined = String::new();
            for piece in items(arguments.first()?)? {
                joined.push_str(&text(piece)?);
            }
            Some(written(joined))
        },

        // ---- the literals a printer needs ----
        Builtin::NatLiteral => |arguments| Some(written(nat(arguments.first()?)?.to_string())),
        Builtin::RatioLiteral => |arguments| Some(optional(written_rational(ratio(arguments.first()?)?).map(written))),
        Builtin::PitchLiteral => |arguments| Some(written(read::<WrittenPitch>(arguments.first()?)?.to_string())),
        Builtin::KeyLiteral => |arguments| {
            let key = read::<crate::score::Key>(arguments.first()?)?;
            Some(written(format!(
                "key {} {}",
                key.tonic(),
                match key.mode() {
                    crate::score::Mode::Major => "major",
                    crate::score::Mode::Minor => "minor",
                }
            )))
        },
        Builtin::IntervalLiteral => |arguments| {
            Some(optional(
                written_interval(read::<Interval>(arguments.first()?)?).map(written),
            ))
        },

        // ---- whole numbers ----
        Builtin::NatAdd => |arguments| {
            let (left, right) = (nat(arguments.first()?)?, nat(arguments.get(1)?)?);
            Some(match left.checked_add(right) {
                Some(value) => answered(whole(value)),
                None => refused("these whole numbers have no result this language can represent"),
            })
        },
        Builtin::NatMul => |arguments| {
            let (left, right) = (nat(arguments.first()?)?, nat(arguments.get(1)?)?);
            Some(match left.checked_mul(right) {
                Some(value) => answered(whole(value)),
                None => refused("these whole numbers have no result this language can represent"),
            })
        },
        // Below zero is the *only* way this fails, so `Option` says everything a
        // `Result` would: there is no second reason to distinguish it from.
        Builtin::NatSub => |arguments| {
            let (left, right) = (nat(arguments.first()?)?, nat(arguments.get(1)?)?);
            Some(optional(left.checked_sub(right).map(whole)))
        },

        // ---- durations and positions ----
        Builtin::DurationOf => |arguments| Some(written_duration(ratio(arguments.first()?)?)),
        Builtin::DurationRatio => |arguments| Some(exact(ratio(arguments.first()?)?)),
        // Two nonnegative durations sum to a nonnegative one, so the only thing
        // left to fail is representability. The constructor is still asked,
        // because the law that durations are nonnegative is stated in one place.
        Builtin::DurationAdd => |arguments| {
            let (left, right) = (ratio(arguments.first()?)?, ratio(arguments.get(1)?)?);
            Some(match exact_arithmetic(left, right, Exact::Add) {
                Some(value) => written_duration(value),
                None => refused("these durations have no sum this language can represent"),
            })
        },
        Builtin::DurationScale => |arguments| {
            let (held, factor) = (ratio(arguments.first()?)?, ratio(arguments.get(1)?)?);
            Some(match exact_arithmetic(held, factor, Exact::Mul) {
                Some(value) => written_duration(value),
                None => refused("this duration and factor have no product this language can represent"),
            })
        },
        Builtin::DurationLess | Builtin::PositionLess => {
            |arguments| Some(boolean(ratio(arguments.first()?)? < ratio(arguments.get(1)?)?))
        }
        Builtin::DurationEqual | Builtin::PositionEqual => {
            |arguments| Some(boolean(ratio(arguments.first()?)? == ratio(arguments.get(1)?)?))
        }
        // Total, and that is the difference between a position and a duration:
        // an instant before the origin is an ordinary position, so there is no
        // refinement here to check.
        Builtin::PositionOf => |arguments| Some(position(ratio(arguments.first()?)?)),
        Builtin::PositionRatio => |arguments| Some(exact(ratio(arguments.first()?)?)),
        Builtin::PositionShift => |arguments| {
            let (from, by) = (ratio(arguments.first()?)?, ratio(arguments.get(1)?)?);
            Some(match exact_arithmetic(from, by, Exact::Add) {
                Some(value) => answered(position(value)),
                None => refused("this position and duration have no result this language can represent"),
            })
        },
        // The one operation the whole tagging exists for. Two positions do not
        // add — there is no name for that — and their difference is a duration
        // only when the second is not before the first.
        Builtin::PositionBetween => |arguments| {
            let (from, to) = (ratio(arguments.first()?)?, ratio(arguments.get(1)?)?);
            if to < from {
                return Some(refused(
                    "the second position is before the first, and a duration is nonnegative",
                ));
            }
            Some(match exact_arithmetic(to, from, Exact::Sub) {
                Some(value) => written_duration(value),
                None => refused("these positions have no difference this language can represent"),
            })
        },

        // ---- intervals and pitch classes ----
        Builtin::IntervalAdd => |arguments| {
            read::<Interval>(arguments.first()?)?
                .compose(read::<Interval>(arguments.get(1)?)?)
                .map(|composed| plain("Interval", composed))
        },
        Builtin::IntervalInverse => |arguments| {
            read::<Interval>(arguments.first()?)?
                .inverse()
                .map(|inverse| plain("Interval", inverse))
        },
        Builtin::PitchClassOf => |arguments| {
            Some(plain(
                "PitchClass",
                read::<WrittenPitch>(arguments.first()?)?.pitch_class(),
            ))
        },

        // ---- scales, degrees, and frames ----
        Builtin::SignatureScale => |arguments| {
            Some(plain(
                "Scale",
                crate::scale::signature_scale(read::<crate::score::Key>(arguments.first()?)?),
            ))
        },
        Builtin::ScaleOn => |arguments| {
            let scale = read::<crate::scale::Scale>(arguments.first()?)?;
            Some(plain("Scale", scale.rooted_at(read::<PitchClass>(arguments.get(1)?)?)))
        },
        Builtin::ScaleTonic => |arguments| {
            Some(plain(
                "PitchClass",
                read::<crate::scale::Scale>(arguments.first()?)?.tonic(),
            ))
        },
        Builtin::ScaleSize => |arguments| {
            let size = read::<crate::scale::Scale>(arguments.first()?)?.size();
            Some(whole(u64::try_from(size).ok()?))
        },
        Builtin::ScalePitch => |arguments| {
            let (scale, pitch) = (
                read::<crate::scale::Scale>(arguments.first()?)?,
                read::<WrittenPitch>(arguments.get(1)?)?,
            );
            let located = crate::scale::Frame::around(scale, pitch).and_then(|frame| frame.locate(pitch));
            Some(optional(located.map(|degree| plain("Degree", degree))))
        },
        Builtin::ScaleClass => |arguments| {
            let (scale, degree) = (
                read::<crate::scale::Scale>(arguments.first()?)?,
                read::<crate::scale::Degree>(arguments.get(1)?)?,
            );
            Some(optional(scale.class(degree).map(|class| plain("PitchClass", class))))
        },
        Builtin::ScaleChord => |arguments| {
            let (scale, degree) = (
                read::<crate::scale::Scale>(arguments.first()?)?,
                read::<crate::scale::Degree>(arguments.get(1)?)?,
            );
            let members = usize::try_from(nat(arguments.get(2)?)?).ok()?;
            Some(optional(
                scale.stacked(degree, members).map(|class| plain("ChordClass", class)),
            ))
        },
        Builtin::PitchFrame => |arguments| {
            let (scale, tonic) = (
                read::<crate::scale::Scale>(arguments.first()?)?,
                read::<WrittenPitch>(arguments.get(1)?)?,
            );
            Some(optional(
                crate::scale::Frame::new(scale, tonic).map(|frame| plain("Frame", frame)),
            ))
        },
        Builtin::FrameScale => {
            |arguments| Some(plain("Scale", read::<crate::scale::Frame>(arguments.first()?)?.scale()))
        }
        Builtin::FrameTonic => {
            |arguments| Some(plain("Pitch", read::<crate::scale::Frame>(arguments.first()?)?.tonic()))
        }
        Builtin::FramePitch => |arguments| {
            let (frame, degree) = (
                read::<crate::scale::Frame>(arguments.first()?)?,
                read::<crate::scale::Degree>(arguments.get(1)?)?,
            );
            frame.pitch(degree).map(|pitch| plain("Pitch", pitch))
        },
        // Degrees are written from one, as musicians write them, and `Degree`
        // counts from one as well: no adjustment belongs here.
        Builtin::DegreeOf => |arguments| {
            let ordinal = i64::try_from(nat(arguments.first()?)?).ok()?;
            Some(plain("Degree", crate::scale::Degree::new(ordinal)))
        },
        Builtin::DegreeStepUp => |arguments| {
            let degree = read::<crate::scale::Degree>(arguments.first()?)?;
            let steps = i64::try_from(nat(arguments.get(1)?)?).ok()?;
            degree.step(steps).map(|stepped| plain("Degree", stepped))
        },
        Builtin::DegreeStepDown => |arguments| {
            let degree = read::<crate::scale::Degree>(arguments.first()?)?;
            let steps = i64::try_from(nat(arguments.get(1)?)?).ok()?.checked_neg()?;
            degree.step(steps).map(|stepped| plain("Degree", stepped))
        },
        Builtin::DegreeRaised => |arguments| {
            read::<crate::scale::Degree>(arguments.first()?)?
                .raised()
                .map(|raised| plain("Degree", raised))
        },
        Builtin::DegreeLowered => |arguments| {
            read::<crate::scale::Degree>(arguments.first()?)?
                .lowered()
                .map(|lowered| plain("Degree", lowered))
        },

        // ---- chords, triads, and Roman numerals ----
        Builtin::ChordOn => |arguments| {
            let class = read::<crate::chord::ChordClass>(arguments.first()?)?;
            Some(plain(
                "ChordClass",
                class.rooted_at(read::<PitchClass>(arguments.get(1)?)?),
            ))
        },
        Builtin::ChordRoot => |arguments| {
            Some(plain(
                "PitchClass",
                read::<crate::chord::ChordClass>(arguments.first()?)?.root(),
            ))
        },
        Builtin::ChordBass => |arguments| {
            let bass = read::<crate::chord::ChordClass>(arguments.first()?)?.bass();
            Some(optional(bass.map(|class| plain("PitchClass", class))))
        },
        Builtin::ChordMembers => |arguments| {
            let class = read::<crate::chord::ChordClass>(arguments.first()?)?;
            Some(listing(
                class
                    .members()
                    .iter()
                    .copied()
                    .map(|interval| plain("Interval", interval)),
            ))
        },
        Builtin::ChordInversion => |arguments| {
            let class = read::<crate::chord::ChordClass>(arguments.first()?)?;
            let place = usize::try_from(nat(arguments.get(1)?)?).ok()?;
            Some(optional(
                class.inverted(place).ok().map(|inverted| plain("ChordClass", inverted)),
            ))
        },
        Builtin::ChordOver => |arguments| {
            let class = read::<crate::chord::ChordClass>(arguments.first()?)?;
            Some(plain("ChordClass", class.over(read::<PitchClass>(arguments.get(1)?)?)))
        },
        Builtin::ChordTriad => |arguments| {
            let class = read::<crate::chord::ChordClass>(arguments.first()?)?;
            Some(optional(
                crate::chord::Triad::of(class).map(|triad| plain("Triad", triad)),
            ))
        },
        Builtin::TriadChord => |arguments| {
            Some(plain(
                "ChordClass",
                read::<crate::chord::Triad>(arguments.first()?)?.class(),
            ))
        },
        Builtin::TriadMajor => |arguments| Some(boolean(read::<crate::chord::Triad>(arguments.first()?)?.is_major())),
        Builtin::RomanOf => |arguments| {
            let (ordinal, members, inversion) = (
                nat(arguments.first()?)?,
                nat(arguments.get(1)?)?,
                nat(arguments.get(2)?)?,
            );
            Some(optional(
                crate::roman::Roman::new(ordinal, members, inversion).map(|numeral| plain("Roman", numeral)),
            ))
        },
        Builtin::RomanOrdinal => |arguments| Some(whole(read::<crate::roman::Roman>(arguments.first()?)?.ordinal())),
        Builtin::RomanSize => |arguments| Some(whole(read::<crate::roman::Roman>(arguments.first()?)?.members())),
        Builtin::RomanInversion => {
            |arguments| Some(whole(read::<crate::roman::Roman>(arguments.first()?)?.inversion()))
        }

        // ---- voicings ----
        Builtin::VoicingOf => |arguments| {
            let class = read::<crate::chord::ChordClass>(arguments.first()?)?;
            let pitches = items(arguments.get(1)?)?
                .into_iter()
                .map(read::<WrittenPitch>)
                .collect::<Option<Vec<_>>>()?;
            Some(optional(
                crate::chord::Voicing::new(class, pitches)
                    .ok()
                    .map(|voicing| plain("Voicing", voicing)),
            ))
        },
        Builtin::VoicingPitches => |arguments| {
            let voicing = read::<crate::chord::Voicing>(arguments.first()?)?;
            Some(listing(voicing.pitches().map(|pitch| plain("Pitch", pitch))))
        },
        Builtin::VoicingBass => |arguments| {
            Some(plain(
                "Pitch",
                read::<crate::chord::Voicing>(arguments.first()?)?.bass(),
            ))
        },
        Builtin::VoicingChord => |arguments| {
            Some(plain(
                "ChordClass",
                read::<crate::chord::Voicing>(arguments.first()?)?.class(),
            ))
        },
        Builtin::VoicingPosition => |arguments| {
            let place = read::<crate::chord::Voicing>(arguments.first()?)?
                .inversion()
                .and_then(|place| u64::try_from(place).ok());
            Some(optional(place.map(whole)))
        },
        Builtin::CloseVoicing => |arguments| {
            let (class, bass) = (
                read::<crate::chord::ChordClass>(arguments.first()?)?,
                read::<WrittenPitch>(arguments.get(1)?)?,
            );
            Some(optional(
                crate::chord::Voicing::close_position(class, bass)
                    .ok()
                    .map(|voicing| plain("Voicing", voicing)),
            ))
        },
        Builtin::DropVoicing => |arguments| {
            let (class, bass) = (
                read::<crate::chord::ChordClass>(arguments.first()?)?,
                read::<WrittenPitch>(arguments.get(1)?)?,
            );
            let voice = usize::try_from(nat(arguments.get(2)?)?).ok()?;
            Some(optional(
                crate::chord::Voicing::dropped(class, bass, voice)
                    .ok()
                    .map(|voicing| plain("Voicing", voicing)),
            ))
        },
        Builtin::OmitVoicing => |arguments| {
            let voicing = read::<crate::chord::Voicing>(arguments.first()?)?;
            let place = usize::try_from(nat(arguments.get(1)?)?).ok()?;
            Some(optional(
                voicing.omitting(place).ok().map(|omitted| plain("Voicing", omitted)),
            ))
        },

        // ---- the twelve-tone domains ----
        Builtin::Pc12Of => |arguments| Some(plain("Pc12", crate::pc12::Pc12::from_number(nat(arguments.first()?)?))),
        Builtin::Pc12Number => |arguments| {
            Some(whole(u64::from(
                read::<crate::pc12::Pc12>(arguments.first()?)?.number(),
            )))
        },
        Builtin::Pc12Forget => |arguments| {
            Some(plain(
                "Pc12",
                crate::pc12::Pc12::forgetting(read::<PitchClass>(arguments.first()?)?),
            ))
        },
        Builtin::Pc12Transposed => |arguments| {
            let member = read::<crate::pc12::Pc12>(arguments.first()?)?;
            Some(plain("Pc12", member.transposed(nat(arguments.get(1)?)?)))
        },
        Builtin::Pc12Inverted => |arguments| {
            let member = read::<crate::pc12::Pc12>(arguments.first()?)?;
            Some(plain("Pc12", member.inverted(nat(arguments.get(1)?)?)))
        },
        Builtin::Pc12Spelled => |arguments| {
            let (member, collection) = (
                read::<crate::pc12::Pc12>(arguments.first()?)?,
                read::<crate::scale::Scale>(arguments.get(1)?)?,
            );
            Some(optional(
                member.spelled(collection).map(|class| plain("PitchClass", class)),
            ))
        },
        Builtin::PcSet12Of => |arguments| Some(plain("PcSet12", crate::pc12::PcSet12::of(pc12s(arguments.first()?)?))),
        Builtin::PcSet12Members => |arguments| {
            let set = read::<crate::pc12::PcSet12>(arguments.first()?)?;
            Some(pc12_listing(set.members().collect()))
        },
        Builtin::PcSet12Normal => |arguments| {
            let set = read::<crate::pc12::PcSet12>(arguments.first()?)?;
            Some(pc12_listing(set.normal_order()))
        },
        Builtin::PcSet12Transposed => |arguments| {
            let set = read::<crate::pc12::PcSet12>(arguments.first()?)?;
            Some(plain("PcSet12", set.transposed(nat(arguments.get(1)?)?)))
        },
        Builtin::PcSet12Inverted => |arguments| {
            let set = read::<crate::pc12::PcSet12>(arguments.first()?)?;
            Some(plain("PcSet12", set.inverted(nat(arguments.get(1)?)?)))
        },
        Builtin::PcSet12Prime => |arguments| {
            let set = read::<crate::pc12::PcSet12>(arguments.first()?)?;
            Some(plain("PcSet12", set.prime_form()))
        },
        Builtin::PcSet12Vector => |arguments| {
            let set = read::<crate::pc12::PcSet12>(arguments.first()?)?;
            Some(listing(
                set.interval_class_vector()
                    .into_iter()
                    .map(|count| whole(u64::from(count))),
            ))
        },
        // The one builtin that says *which* way it failed. A caller used to
        // learn that from `row12_repeats` and `row12_missing`, run again on the
        // same input; the reason now comes back with the refusal.
        Builtin::Row12Of => |arguments| {
            let pcs = pc12s(arguments.first()?)?;
            Some(match crate::pc12::Row12::checked(&pcs) {
                Some(row) => answered(plain("Row12", row)),
                None => case(
                    "Result.Err",
                    vec![case(
                        "RowFault.Fault",
                        vec![
                            listing(crate::pc12::repeated_positions(&pcs).into_iter().map(whole)),
                            pc12_listing(crate::pc12::missing_classes(&pcs)),
                        ],
                    )],
                ),
            })
        },
        Builtin::Row12Pcs => |arguments| {
            let row = read::<crate::pc12::Row12>(arguments.first()?)?;
            Some(pc12_listing(row.pcs().collect()))
        },
        Builtin::Row12Head => |arguments| Some(plain("Pc12", read::<crate::pc12::Row12>(arguments.first()?)?.head())),
        Builtin::Row12Transposed => |arguments| {
            let row = read::<crate::pc12::Row12>(arguments.first()?)?;
            Some(plain("Row12", row.transposed(nat(arguments.get(1)?)?)))
        },
        Builtin::Row12Inverted => |arguments| {
            let row = read::<crate::pc12::Row12>(arguments.first()?)?;
            Some(plain("Row12", row.inverted(nat(arguments.get(1)?)?)))
        },
        Builtin::Row12Retrograde => |arguments| {
            Some(plain(
                "Row12",
                read::<crate::pc12::Row12>(arguments.first()?)?.retrograde(),
            ))
        },
        Builtin::Row12Matrix => |arguments| {
            let row = read::<crate::pc12::Row12>(arguments.first()?)?;
            Some(listing(row.matrix().into_iter().map(|form| plain("Row12", form))))
        },
        Builtin::Row12Forms => |arguments| {
            Some(whole(u64::from(
                read::<crate::pc12::Row12>(arguments.first()?)?.forms(),
            )))
        },
        Builtin::Row12Symmetries => |arguments| {
            Some(whole(u64::from(
                read::<crate::pc12::Row12>(arguments.first()?)?.symmetries(),
            )))
        },
        Builtin::Row12Repeats => |arguments| {
            let pcs = pc12s(arguments.first()?)?;
            Some(listing(crate::pc12::repeated_positions(&pcs).into_iter().map(whole)))
        },
        Builtin::Row12Missing => {
            |arguments| Some(pc12_listing(crate::pc12::missing_classes(&pc12s(arguments.first()?)?)))
        }

        // ---- the 25 rows this module does not own ----
        //
        // Eight structural eliminators over declared families, whose recursors
        // `musa_core::declare` already generated; eight track and nine machine
        // builtins waiting on the reshaping prompt 142 does; and the phase
        // table, which has a walk of its own below.
        Builtin::NatFold
        | Builtin::ListFoldFromStart
        | Builtin::ListFoldFromEnd
        | Builtin::OptionFold
        | Builtin::Map
        | Builtin::Filter
        | Builtin::Range
        | Builtin::Repeat
        | Builtin::Transpose
        | Builtin::Stretch
        | Builtin::Retrograde
        | Builtin::Invert
        | Builtin::Shift
        | Builtin::Together
        | Builtin::MapNotePitches
        | Builtin::Play
        | Builtin::Primitive
        | Builtin::Machine
        | Builtin::Identity
        | Builtin::Connect
        | Builtin::Beside
        | Builtin::Feedback
        | Builtin::Copy
        | Builtin::Drop
        | Builtin::Swap
        | Builtin::Syntax(_) => return None,
    })
}

/// A `List Pc12`.
fn pc12_listing(members: Vec<crate::pc12::Pc12>) -> Datum {
    listing(members.into_iter().map(|member| plain("Pc12", member)))
}

// ---- the expansion phase ----

/// A token kind, wrapped so that it can print.
///
/// `musa_language::SyntaxKind` is another crate's, so the orphan rule puts
/// [`std::fmt::Display`] out of reach for it here. The wrapper is the smallest
/// thing that fixes that, and the spelling it prints is the one the phase
/// registry offers the kind under — the same table `token_kind_equal` compares
/// against, so what a diagnostic shows is what an adapter would have written.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Kind(pub(super) musa_language::SyntaxKind);

impl std::fmt::Display for Kind {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        out.write_str(token_kind_spelling(self.0))
    }
}

/// A `Syntax ⟨cat⟩` literal.
///
/// The category is a parameter because two of the fourteen change it: the whole
/// content of `as_expression` and the gate is that a tree the parser has read is
/// a `Syntax ⟨Expr⟩` and the same tree unread is not.
fn tree(cat: Cat, node: Syntax) -> Datum {
    Datum::Lit(literal(syntax_type(cat), node))
}

/// A `Syntax ⟨token-tree⟩` literal, which is what a builder answers with.
fn built(node: Syntax) -> Datum {
    tree(Cat::TokenTree, node)
}

/// A node path.
fn where_at(path: crate::syntax::NodePath) -> Datum {
    plain("NodePath", path)
}

/// The path argument of a builder.
fn path(datum: &Datum) -> Option<crate::syntax::NodePath> {
    read::<crate::syntax::NodePath>(datum)
}

/// The syntax argument of a builder.
fn node(datum: &Datum) -> Option<Syntax> {
    read::<Syntax>(datum)
}

/// A whole-number argument narrowed to the width a path role is written in.
fn role(datum: &Datum) -> Option<u32> {
    u32::try_from(nat(datum)?).ok()
}

/// The rule for one phase builder, or `None` for the three traversals.
///
/// The phase's own table, walked separately for §5.9's reason: nothing here is
/// looked up when ordinary source resolves a name. What it shares with
/// [`source`] is the shape of a rule and nothing else.
pub(super) fn phase(operation: SyntaxOp) -> Option<Rule> {
    Some(match operation {
        SyntaxOp::At => |arguments| {
            let subject = node(arguments.first()?)?;
            let wanted = path(arguments.get(1)?)?;
            Some(optional(subject.at(&wanted).cloned().map(built)))
        },
        // The number is built into a token rather than handed over as a `Nat`: a
        // transformer's answer is an expression, and the only way a number
        // reaches one is as a literal the ordinary parser reads.
        SyntaxOp::Anchor => |arguments| {
            let subject = node(arguments.first()?)?;
            let (wanted, here) = (path(arguments.get(1)?)?, path(arguments.get(2)?)?);
            let found = subject
                .anchor(&wanted)
                .map(|anchor| crate::syntax::token(here, musa_language::SyntaxKind::Integer, anchor.to_string()));
            Some(optional(found.map(built)))
        },
        // The reader's own reading, handed back rather than re-derived. Both
        // numeric kinds the lexer distinguishes answer here, and everything else
        // is not a number and says so in the value — asked as two questions
        // rather than as a match on the kind, because the lexer has some three
        // hundred kinds and naming the other two hundred and ninety-eight would
        // be a list nobody could read.
        SyntaxOp::Number => |arguments| {
            let found = match node(arguments.first()?)? {
                Syntax::Token { kind, ref text, .. } => {
                    if kind == musa_language::SyntaxKind::Integer {
                        text.parse::<i64>().ok().map(Ratio::from_integer)
                    } else if kind == musa_language::SyntaxKind::Rational {
                        text.split_once('/')
                            .and_then(|(numerator, denominator)| {
                                Some((numerator.parse::<i128>().ok()?, denominator.parse::<i128>().ok()?))
                            })
                            .and_then(|(numerator, denominator)| exact_ratio(numerator, denominator))
                    } else {
                        None
                    }
                }
                Syntax::Missing(_) | Syntax::Identifier { .. } | Syntax::Group { .. } => None,
            };
            Some(optional(found.map(exact)))
        },
        // Written through `Derived` rather than through `NodePath::built`
        // directly, because the triple is what a derived path *is*: the origin
        // the builder was pointed at, the construction site that read it, and
        // the position within what that site built.
        SyntaxOp::Built => |arguments| {
            Some(where_at(
                crate::syntax::Derived {
                    origin: path(arguments.first()?)?,
                    quotation: role(arguments.get(1)?)?,
                    path: vec![role(arguments.get(2)?)?],
                }
                .path(),
            ))
        },
        SyntaxOp::Binding => |arguments| {
            Some(plain(
                "BindingPath",
                path(arguments.first()?)?.binding(role(arguments.get(1)?)?),
            ))
        },
        SyntaxOp::Token => |arguments| {
            Some(built(crate::syntax::token(
                path(arguments.first()?)?,
                read::<Kind>(arguments.get(1)?)?.0,
                text(arguments.get(2)?)?,
            )))
        },
        SyntaxOp::Identifier => |arguments| {
            Some(built(crate::syntax::identifier(
                path(arguments.first()?)?,
                text(arguments.get(1)?)?,
            )))
        },
        SyntaxOp::Group => |arguments| {
            let children = items(arguments.get(2)?)?
                .into_iter()
                .map(node)
                .collect::<Option<Vec<_>>>()?;
            Some(built(crate::syntax::group(
                path(arguments.first()?)?,
                read::<crate::syntax::Delimiter>(arguments.get(1)?)?,
                children,
            )))
        },
        SyntaxOp::Binder => |arguments| {
            Some(built(crate::syntax::binder(
                &read::<crate::syntax::BindingPath>(arguments.first()?)?,
                text(arguments.get(1)?)?,
            )))
        },
        SyntaxOp::Reference => |arguments| {
            Some(built(crate::syntax::reference(
                path(arguments.first()?)?,
                &read::<crate::syntax::BindingPath>(arguments.get(1)?)?,
                text(arguments.get(2)?)?,
            )))
        },
        SyntaxOp::KindEqual => |arguments| {
            Some(boolean(
                read::<Kind>(arguments.first()?)? == read::<Kind>(arguments.get(1)?)?,
            ))
        },
        SyntaxOp::DelimiterEqual => |arguments| {
            let (left, right) = (
                read::<crate::syntax::Delimiter>(arguments.first()?)?,
                read::<crate::syntax::Delimiter>(arguments.get(1)?)?,
            );
            Some(boolean(left == right))
        },
        // The claim is established by running the real parser, which is the only
        // thing that can establish it. Nothing about the value changes — the
        // index is a claim about how the tree parses, and this is the question
        // being asked.
        SyntaxOp::AsExpression => |arguments| {
            let subject = node(arguments.first()?)?;
            let parses = crate::syntax::parses_as_expression(&subject);
            Some(optional(parses.then(|| tree(Cat::Expr, subject))))
        },
        // The gate answers with a value either way, which is what keeps it
        // total: a transformer that builds badly gets a `Result` back and
        // decides what to say about it.
        SyntaxOp::Checked => |arguments| {
            let subject = node(arguments.first()?)?;
            Some(match crate::syntax::check_expression(&subject) {
                Ok(()) => answered(tree(Cat::Expr, subject)),
                Err(refusal) => refused(&refusal.to_string()),
            })
        },
        // Prompt 141f's three. A traversal takes a function argument, so it is
        // §5.8's second family and not this one.
        SyntaxOp::Recurse | SyntaxOp::Run | SyntaxOp::Fold => return None,
    })
}
