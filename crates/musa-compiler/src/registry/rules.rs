//! What each compiler-owned δ operation computes.
//!
//! One rule per row of [`crate::phase::BUILTIN_OWNERSHIP`]'s δ half and of
//! [`crate::phase::SYNTAX_OWNERSHIP`]'s builder half, translated from the old
//! evaluator's `eval_builtin` and `eval_syntax` rather than rewritten. The
//! arithmetic, the domain calls, and the wording of every refusal are the ones
//! that were there; what changed is only how an argument is read and how an
//! answer is written.
//!
//! # A rule is a `fn`, which is D3 rather than a style
//!
//! [`musa_calculus::Rule`] is a function pointer, so a rule cannot capture the
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
//!   application as a stuck term. [`musa_calculus::Malformed::BuiltinStuck`] now
//!   reports it, which is D2 enforced rather than assumed. Every `None` below
//!   is one the old evaluator also answered — `interval_add` at an interval pair
//!   with no representable sum is the honest example — and each is a signature
//!   that promises more than its rule delivers. They are left as they are and
//!   named here rather than quietly widened to `Option`, because widening one is
//!   a language change and prompt 163 is where the whole family is reconsidered.
//!
//! # Reading is untyped and writing is not
//!
//! A [`Datum`] read needs no type: 141d made `Datum::Case` carry the qualified
//! constructor name, so `Option.Some` is `Option.Some` wherever it came from.
//! Writing one does need a type, and the core supplies it — a `Datum::Case` is
//! realized against the builtin's own declared result type, so a rule names a
//! constructor and never a [`musa_calculus::Term`]. The exception is `Datum::Lit`,
//! which is used with the type the rule supplied, which is why every writer here
//! goes through [`super::literal`] and the same [`super::plain_type`] the
//! registration used.
//!
//! # `Nat` is unary, and that is a cost worth naming
//!
//! `Nat` is *declared* (`Zero`, `Succ`) rather than registered, because source
//! code pattern-matches on it and a base type has no cases. So `nat_add` writes
//! its answer one `Succ` at a time, and a large sum is a large term. That is the
//! representation the prelude chose and not a defect of this module; prompt 163,
//! which collapses `nat_add` and `ratio_add` behind `Add`, is where a binary
//! representation would be argued for.

use std::sync::Arc;

use musa_calculus::{Answer, Datum, Literal, Rule};
use num_rational::Ratio;

use super::{domain, literal, plain_type, syntax_type, tagged_type};
use crate::phase::{Builtin, Coordinate, SyntaxOp};
use crate::quote::{Cat, Syntax, token_kind_spelling};
use musa_score::origin::Interval;
use musa_score::pitch::{PitchClass, WrittenPitch};
use musa_score::time::{Exact, exact_arithmetic, exact_ratio, written_rational};

/// How many rows of the two ownership tables reach the core's registry.
///
/// 112 of `BUILTIN_OWNERSHIP`'s 121 and 16 of `SYNTAX_OWNERSHIP`'s 17. The rows
/// past this module's 96 and fourteen are registered where their reduction is:
/// the two traversals in [`super::traversal`], the eight track builtins in
/// [`super::track`], and the eight machine forms in [`super::machine`], which
/// are §5.8's second, third, and fourth families rather than its first. A
/// δ-builtin's rule is a [`Rule`] and lives here; the others carry a rewrite, a
/// family, or no reduction at all, and live beside the argument that admits
/// them.
pub(super) const REGISTERED: usize = 129;

/// The operations the core has that neither ownership table names.
///
/// The third count, and it runs the other way from the two above: those are rows
/// with no registration, and these are registrations with no row. Quotation is
/// the reason there are any — the old checker built a quote inside itself, as an
/// `ExprKind` with a template beside it, so there was never a *name* for
/// instantiating one. There is now, and it is deliberately not a name an adapter
/// can write: [`super::quotation`] says why.
///
/// [`FORGOTTEN`] is the fifth, and the strongest case of the same argument: it
/// is what `11-quotation.md` §1's acceptance rule *elaborates to*, so a row
/// naming it would hand an author the `forget` the section says they do not
/// write.
///
/// Named rather than counted for [`super::traversal::SPELLINGS`]'s reason —
/// "which" is the claim, and the accounting law reads each of them back out of
/// the built registry and checks it against both tables.
pub(super) const BEYOND: [&str; 7] = [
    "instantiate_quote",
    "match_quote",
    "quote_hole",
    "quote_holes",
    "quote_hole_expr",
    "quote_holes_expr",
    FORGOTTEN,
];

/// The name of the operation that carries a parsed tree into a position that
/// has forgotten how it parses.
///
/// One constant because two places have to agree on it and neither can read the
/// other: [`super::syntax_carrier`] registers the builtin under it, and
/// [`forgets`] answers it to `musa-calculus`'s elaborator.
pub(super) const FORGOTTEN: &str = "forget_category";

/// The rows that do not, by family and by count.
///
/// Ten, and each group is left for a reason that is about the *core* rather
/// than about effort:
///
/// - a **structural eliminator** traverses `Nat`, `List`, or `Option`, which are
///   declared families with generated recursors, and
///   [`musa_calculus::Registry::new`] refuses a structural target that is not a base
///   type. Registering one would be a second ι-rule for a type that has one, so
///   all eight are library code. Four of them no longer exist as *names*: the
///   prelude declares `Iterable<C, A>`, so a fold over a list, an option, or a
///   count is `xs.fold_from_start(seed, combine)` and its mirror — one spelling
///   per direction for every container that can be walked — and `nat_fold`,
///   `option_fold`, and the two list folds have nothing left to be. The other
///   four are in `stdlib/src/list.musa`. Three keep their old spelling;
///   `repeat` could not,
///   because a `repeat { … }` in a score means the word is a statement keyword
///   and `fn repeat` does not parse. It is `repeated` there.
/// - the one **machine** builtin left is `primitive`, which has no type of its
///   own: its ports and its configuration type are read out of the build-local
///   registry rather than written in a signature, so what is registered is one
///   closed signature per unit the build knows
///   ([`super::machine::primitives`]) and the *word* is registered nowhere.
///   [`super::machine::UNREGISTERED`] argues it. Its eight siblings are
///   registered under their own spellings.
/// - a **phase projection** is `run_syntax_step`, which hides nothing: it is the
///   `run` field of a `SyntaxStep` applied to a context, and a projection is not
///   a compiler-owned operation. [`musa_calculus::Registry::new`] would have refused
///   it anyway, since its target is a declared family and a rewrite over one is
///   the second ι-rule that check exists to catch. It is *defined* instead —
///   [`super::run_syntax_step`].
///
/// The suite counts each group again off the tables themselves, so this array
/// cannot drift from what is actually registered.
#[cfg(test)]
pub(super) const UNREGISTERED: [(&str, usize); 3] = [
    ("structural eliminators", 8),
    ("machine builtins", 1),
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
pub(super) fn read<T>(datum: &Datum) -> Option<T>
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

/// A whole number.
///
/// One arm and no walk: `Nat` is a counting family, so the core reads a closed
/// `Nat` back as [`Datum::Count`] and the number is already the number. This
/// used to count `Succ`s off a tower iteratively, because a tower is as deep as
/// it is large and a recursive count would have put the host's stack where the
/// core's budget belongs; now no tower is built on either side.
pub(super) fn nat(datum: &Datum) -> Option<u64> {
    match *datum {
        Datum::Count { ref family, count } if &**family == "Nat" => Some(count),
        Datum::Count { .. } | Datum::Case { .. } | Datum::Lit(_) => None,
    }
}

/// The members of a `List`, in order.
pub(super) fn items(datum: &Datum) -> Option<Vec<&Datum>> {
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

/// The two halves of a `Pair`, whatever they hold.
///
/// Generic where [`super::notation`]'s reading was specialized, because the
/// halves of a pair are the same two fields at every instantiation: a written
/// `(a, b)` is `Pair.Both a b` (`lower::values`), and what the halves *are* is
/// the caller's question. `super::argument` reads a pair of pitches through
/// this and `super::notation` reads a pair of counts, and neither has to know
/// where a constructor's parameters stop.
pub(super) fn halves(datum: &Datum) -> Option<(&Datum, &Datum)> {
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
    Some((fields.first()?, fields.get(1)?))
}

/// The pitch classes of a `List Pc12`.
fn pc12s(datum: &Datum) -> Option<Vec<musa_score::pc12::Pc12>> {
    items(datum)?.into_iter().map(read::<musa_score::pc12::Pc12>).collect()
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

/// `Nat`, as the count it is.
fn whole(value: u64) -> Datum {
    Datum::Count {
        family: Arc::from("Nat"),
        count: value,
    }
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
pub(super) fn duration(value: Ratio<i64>) -> Datum {
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

/// What a rule computed.
///
/// `Some(Answer::Reduced(…))` written at every tail would say one thing three
/// times; this says it once. The `None` a rule may still answer is a *different*
/// statement — "this rule does not apply to these arguments", which at closed
/// data is a defect in this table — and keeps its own spelling so the two never
/// blur.
pub(super) fn reduced(value: Datum) -> Option<Answer> {
    Some(Answer::Reduced(value))
}

/// The program is wrong, and this is what to say about it.
///
/// Until prompt 141m these operations answered `Result τ Text` and handed the
/// sentence back as a *value*, because a rule's only other "no" was silence and
/// the evaluator reads silence as this table being broken. Every caller then
/// carried a failure it could do nothing with: no program can recover from "a
/// chord sounds for longer than no time at all", and the only repair is in the
/// source.
pub(super) fn refused(because: &str) -> Answer {
    Answer::Refused(because.to_owned())
}

/// The accepting half of the one `Result` this table still answers with.
///
/// Prompt 141m moved the notation and track refusals; prompt 142 moved the
/// eleven arithmetic, duration, and position rules that were waiting on their
/// callers. What is left is [`SyntaxOp::Checked`], and 141m's survey says why it
/// stays: the gate exists so a transformer can *decide* what to say about a tree
/// it built badly, and refusing would take that decision away.
fn answered(value: Datum) -> Answer {
    Answer::Reduced(case("Result.Ok", vec![value]))
}

/// The refusing half of that same `Result`.
fn errored(because: &str) -> Answer {
    Answer::Reduced(case("Result.Err", vec![written(because.to_owned())]))
}

/// A duration that has to be nonnegative to exist.
///
/// The one place the law is stated, exactly as the old evaluator stated it once.
fn written_duration(value: Ratio<i64>) -> Answer {
    if value < Ratio::ZERO {
        return refused("a duration is nonnegative, and this exact rational is below zero");
    }
    Answer::Reduced(duration(value))
}

/// The four exact-rational operations, which differ only in the operation and in
/// division's own refusal.
fn ratio_arithmetic(operation: Exact, arguments: &[Datum]) -> Option<Answer> {
    let (left, right) = (ratio(arguments.first()?)?, ratio(arguments.get(1)?)?);
    if matches!(operation, Exact::Div) && right == Ratio::ZERO {
        return Some(refused("an exact rational is not divided by zero"));
    }
    Some(match exact_arithmetic(left, right, operation) {
        Some(value) => Answer::Reduced(exact(value)),
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
        Builtin::RatioLess => |arguments| reduced(boolean(ratio(arguments.first()?)? < ratio(arguments.get(1)?)?)),
        Builtin::RatioEqual => |arguments| reduced(boolean(ratio(arguments.first()?)? == ratio(arguments.get(1)?)?)),
        Builtin::TextEqual => |arguments| reduced(boolean(text(arguments.first()?)? == text(arguments.get(1)?)?)),
        Builtin::TextJoin => |arguments| {
            let mut joined = String::new();
            for piece in items(arguments.first()?)? {
                joined.push_str(&text(piece)?);
            }
            reduced(written(joined))
        },

        // ---- the literals a printer needs ----
        Builtin::NatLiteral => |arguments| reduced(written(nat(arguments.first()?)?.to_string())),
        Builtin::RatioLiteral => {
            |arguments| reduced(optional(written_rational(ratio(arguments.first()?)?).map(written)))
        }
        Builtin::PitchLiteral => |arguments| reduced(written(read::<WrittenPitch>(arguments.first()?)?.to_string())),
        Builtin::KeyLiteral => |arguments| {
            let key = read::<musa_score::score::Key>(arguments.first()?)?;
            reduced(written(format!(
                "key {} {}",
                key.tonic(),
                match key.mode() {
                    musa_score::score::Mode::Major => "major",
                    musa_score::score::Mode::Minor => "minor",
                }
            )))
        },
        Builtin::IntervalLiteral => {
            |arguments| reduced(optional(read::<Interval>(arguments.first()?)?.literal().map(written)))
        }

        // ---- whole numbers ----
        Builtin::NatAdd => |arguments| {
            let (left, right) = (nat(arguments.first()?)?, nat(arguments.get(1)?)?);
            Some(match left.checked_add(right) {
                Some(value) => Answer::Reduced(whole(value)),
                None => refused("these whole numbers have no result this language can represent"),
            })
        },
        Builtin::NatMul => |arguments| {
            let (left, right) = (nat(arguments.first()?)?, nat(arguments.get(1)?)?);
            Some(match left.checked_mul(right) {
                Some(value) => Answer::Reduced(whole(value)),
                None => refused("these whole numbers have no result this language can represent"),
            })
        },
        // Below zero is the *only* way this fails, so `Option` says everything a
        // `Result` would: there is no second reason to distinguish it from.
        Builtin::NatSub => |arguments| {
            let (left, right) = (nat(arguments.first()?)?, nat(arguments.get(1)?)?);
            reduced(optional(left.checked_sub(right).map(whole)))
        },

        // ---- durations and positions ----
        Builtin::DurationOf => |arguments| Some(written_duration(ratio(arguments.first()?)?)),
        Builtin::DurationRatio => |arguments| reduced(exact(ratio(arguments.first()?)?)),
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
            |arguments| reduced(boolean(ratio(arguments.first()?)? < ratio(arguments.get(1)?)?))
        }
        Builtin::DurationEqual | Builtin::PositionEqual => {
            |arguments| reduced(boolean(ratio(arguments.first()?)? == ratio(arguments.get(1)?)?))
        }
        // Total, and that is the difference between a position and a duration:
        // an instant before the origin is an ordinary position, so there is no
        // refinement here to check.
        Builtin::PositionOf => |arguments| reduced(position(ratio(arguments.first()?)?)),
        Builtin::PositionRatio => |arguments| reduced(exact(ratio(arguments.first()?)?)),
        Builtin::PositionShift => |arguments| {
            let (from, by) = (ratio(arguments.first()?)?, ratio(arguments.get(1)?)?);
            Some(match exact_arithmetic(from, by, Exact::Add) {
                Some(value) => Answer::Reduced(position(value)),
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
                .map(|composed| plain("Interval", composed).into())
        },
        Builtin::IntervalInverse => |arguments| {
            read::<Interval>(arguments.first()?)?
                .inverse()
                .map(|inverse| plain("Interval", inverse).into())
        },
        // Written equality, and deliberately not enharmonic: `M3` and `d4` span
        // the same number of semitones and are different intervals, exactly as
        // §2's layer table separates a written pitch from a MIDI number. A
        // literal pattern is the caller — `match by { M3 -> …, }` is the only
        // way a source program asks this question today — and a pattern that
        // matched a spelling the author did not write would be the same defect
        // this suite's text arm was.
        Builtin::IntervalEqual => |arguments| {
            reduced(boolean(
                read::<Interval>(arguments.first()?)? == read::<Interval>(arguments.get(1)?)?,
            ))
        },
        // `c4 up M3` and `c up M3` were arms of the replaced checker rather
        // than rows of this table, so `141e` had nothing to translate: an
        // interval moves two coordinates at once and the operation that does
        // it had no source word. It has one here because the notation fold
        // needs it for a pitch it was *handed* — `(root up M2)/4` in a motif —
        // where there is no literal to move at read time.
        Builtin::PitchTransposed => |arguments| {
            let (pitch, interval) = (
                read::<WrittenPitch>(arguments.first()?)?,
                read::<Interval>(arguments.get(1)?)?,
            );
            Some(pitch.transpose(interval).map_or_else(
                || refused("this transposition leaves the range of written pitches"),
                |moved| Answer::Reduced(plain("Pitch", moved)),
            ))
        },
        // The other direction of [`Builtin::PitchTransposed`], and the operation
        // that makes `Pitch` and `Interval` a torsor rather than a carrier with
        // an action: `03-musical-domains.md` §1 proves the interval between two
        // written pitches exists and is unique — "if `p+i=p+j`, integer
        // cancellation gives `i=j`" — and until this row the registry had the
        // action and not the difference.
        //
        // Written and not sounding, for [`Builtin::IntervalEqual`]'s reason: the
        // answer from `c4` to `fb4` is `d4` and never `M3`, because the two
        // count to different letters.
        Builtin::PitchBetween => |arguments| {
            let (from, to) = (
                read::<WrittenPitch>(arguments.first()?)?,
                read::<WrittenPitch>(arguments.get(1)?)?,
            );
            Some(from.between(to).map_or_else(
                || refused("the interval between these pitches leaves the range of written intervals"),
                |span| Answer::Reduced(plain("Interval", span)),
            ))
        },
        // [`Builtin::IntervalEqual`]'s argument at the other written coordinate:
        // `e#5` and `f5` sound alike and are two pitches, so this is equality of
        // letter, alteration, and octave and never of semitone number.
        Builtin::PitchEqual => |arguments| {
            reduced(boolean(
                read::<WrittenPitch>(arguments.first()?)? == read::<WrittenPitch>(arguments.get(1)?)?,
            ))
        },
        Builtin::PitchClassTransposed => |arguments| {
            let (class, interval) = (
                read::<PitchClass>(arguments.first()?)?,
                read::<Interval>(arguments.get(1)?)?,
            );
            Some(class.transpose(interval).map_or_else(
                || refused("this transposition leaves the range of spelled pitch classes"),
                |moved| Answer::Reduced(plain("PitchClass", moved)),
            ))
        },
        Builtin::PitchClassOf => |arguments| {
            reduced(plain(
                "PitchClass",
                read::<WrittenPitch>(arguments.first()?)?.pitch_class(),
            ))
        },

        // ---- scales, degrees, and frames ----
        Builtin::SignatureScale => |arguments| {
            reduced(plain(
                "Scale",
                musa_score::scale::signature_scale(read::<musa_score::score::Key>(arguments.first()?)?),
            ))
        },
        Builtin::ScaleOn => |arguments| {
            let scale = read::<musa_score::scale::Scale>(arguments.first()?)?;
            reduced(plain("Scale", scale.rooted_at(read::<PitchClass>(arguments.get(1)?)?)))
        },
        Builtin::ScaleTonic => |arguments| {
            reduced(plain(
                "PitchClass",
                read::<musa_score::scale::Scale>(arguments.first()?)?.tonic(),
            ))
        },
        Builtin::ScaleSize => |arguments| {
            let size = read::<musa_score::scale::Scale>(arguments.first()?)?.size();
            reduced(whole(u64::try_from(size).ok()?))
        },
        Builtin::ScalePitch => |arguments| {
            let (scale, pitch) = (
                read::<musa_score::scale::Scale>(arguments.first()?)?,
                read::<WrittenPitch>(arguments.get(1)?)?,
            );
            let located = musa_score::scale::Frame::around(scale, pitch).and_then(|frame| frame.locate(pitch));
            reduced(optional(located.map(|degree| plain("Degree", degree))))
        },
        Builtin::ScaleClass => |arguments| {
            let (scale, degree) = (
                read::<musa_score::scale::Scale>(arguments.first()?)?,
                read::<musa_score::scale::Degree>(arguments.get(1)?)?,
            );
            reduced(optional(scale.class(degree).map(|class| plain("PitchClass", class))))
        },
        Builtin::ScaleChord => |arguments| {
            let (scale, degree) = (
                read::<musa_score::scale::Scale>(arguments.first()?)?,
                read::<musa_score::scale::Degree>(arguments.get(1)?)?,
            );
            let members = usize::try_from(nat(arguments.get(2)?)?).ok()?;
            reduced(optional(
                scale.stacked(degree, members).map(|class| plain("ChordClass", class)),
            ))
        },
        Builtin::PitchFrame => |arguments| {
            let (scale, tonic) = (
                read::<musa_score::scale::Scale>(arguments.first()?)?,
                read::<WrittenPitch>(arguments.get(1)?)?,
            );
            reduced(optional(
                musa_score::scale::Frame::new(scale, tonic).map(|frame| plain("Frame", frame)),
            ))
        },
        Builtin::FrameScale => |arguments| {
            reduced(plain(
                "Scale",
                read::<musa_score::scale::Frame>(arguments.first()?)?.scale(),
            ))
        },
        Builtin::FrameTonic => |arguments| {
            reduced(plain(
                "Pitch",
                read::<musa_score::scale::Frame>(arguments.first()?)?.tonic(),
            ))
        },
        Builtin::FramePitch => |arguments| {
            let (frame, degree) = (
                read::<musa_score::scale::Frame>(arguments.first()?)?,
                read::<musa_score::scale::Degree>(arguments.get(1)?)?,
            );
            frame.pitch(degree).map(|pitch| plain("Pitch", pitch).into())
        },
        // Degrees are written from one, as musicians write them, and `Degree`
        // counts from one as well: no adjustment belongs here.
        Builtin::DegreeOf => |arguments| {
            let ordinal = i64::try_from(nat(arguments.first()?)?).ok()?;
            reduced(plain("Degree", musa_score::scale::Degree::new(ordinal)))
        },
        Builtin::DegreeStepUp => |arguments| {
            let degree = read::<musa_score::scale::Degree>(arguments.first()?)?;
            let steps = i64::try_from(nat(arguments.get(1)?)?).ok()?;
            degree.step(steps).map(|stepped| plain("Degree", stepped).into())
        },
        Builtin::DegreeStepDown => |arguments| {
            let degree = read::<musa_score::scale::Degree>(arguments.first()?)?;
            let steps = i64::try_from(nat(arguments.get(1)?)?).ok()?.checked_neg()?;
            degree.step(steps).map(|stepped| plain("Degree", stepped).into())
        },
        Builtin::DegreeRaised => |arguments| {
            read::<musa_score::scale::Degree>(arguments.first()?)?
                .raised()
                .map(|raised| plain("Degree", raised).into())
        },
        Builtin::DegreeLowered => |arguments| {
            read::<musa_score::scale::Degree>(arguments.first()?)?
                .lowered()
                .map(|lowered| plain("Degree", lowered).into())
        },

        // ---- chords, triads, and Roman numerals ----
        Builtin::ChordOn => |arguments| {
            let class = read::<musa_score::chord::ChordClass>(arguments.first()?)?;
            reduced(plain(
                "ChordClass",
                class.rooted_at(read::<PitchClass>(arguments.get(1)?)?),
            ))
        },
        Builtin::ChordRoot => |arguments| {
            reduced(plain(
                "PitchClass",
                read::<musa_score::chord::ChordClass>(arguments.first()?)?.root(),
            ))
        },
        Builtin::ChordBass => |arguments| {
            let bass = read::<musa_score::chord::ChordClass>(arguments.first()?)?.bass();
            reduced(optional(bass.map(|class| plain("PitchClass", class))))
        },
        Builtin::ChordMembers => |arguments| {
            let class = read::<musa_score::chord::ChordClass>(arguments.first()?)?;
            reduced(listing(
                class
                    .members()
                    .iter()
                    .copied()
                    .map(|interval| plain("Interval", interval)),
            ))
        },
        Builtin::ChordInversion => |arguments| {
            let class = read::<musa_score::chord::ChordClass>(arguments.first()?)?;
            let place = usize::try_from(nat(arguments.get(1)?)?).ok()?;
            reduced(optional(
                class.inverted(place).ok().map(|inverted| plain("ChordClass", inverted)),
            ))
        },
        Builtin::ChordOver => |arguments| {
            let class = read::<musa_score::chord::ChordClass>(arguments.first()?)?;
            reduced(plain("ChordClass", class.over(read::<PitchClass>(arguments.get(1)?)?)))
        },
        Builtin::ChordTriad => |arguments| {
            let class = read::<musa_score::chord::ChordClass>(arguments.first()?)?;
            reduced(optional(
                musa_score::chord::Triad::of(class).map(|triad| plain("Triad", triad)),
            ))
        },
        Builtin::TriadChord => |arguments| {
            reduced(plain(
                "ChordClass",
                read::<musa_score::chord::Triad>(arguments.first()?)?.class(),
            ))
        },
        Builtin::TriadMajor => |arguments| {
            reduced(boolean(
                read::<musa_score::chord::Triad>(arguments.first()?)?.is_major(),
            ))
        },
        Builtin::RomanOf => |arguments| {
            let (ordinal, members, inversion) = (
                nat(arguments.first()?)?,
                nat(arguments.get(1)?)?,
                nat(arguments.get(2)?)?,
            );
            reduced(optional(
                musa_score::roman::Roman::new(ordinal, members, inversion).map(|numeral| plain("Roman", numeral)),
            ))
        },
        Builtin::RomanOrdinal => {
            |arguments| reduced(whole(read::<musa_score::roman::Roman>(arguments.first()?)?.ordinal()))
        }
        Builtin::RomanSize => {
            |arguments| reduced(whole(read::<musa_score::roman::Roman>(arguments.first()?)?.members()))
        }
        Builtin::RomanInversion => {
            |arguments| reduced(whole(read::<musa_score::roman::Roman>(arguments.first()?)?.inversion()))
        }

        // ---- voicings ----
        Builtin::VoicingOf => |arguments| {
            let class = read::<musa_score::chord::ChordClass>(arguments.first()?)?;
            let pitches = items(arguments.get(1)?)?
                .into_iter()
                .map(read::<WrittenPitch>)
                .collect::<Option<Vec<_>>>()?;
            reduced(optional(
                musa_score::chord::Voicing::new(class, pitches)
                    .ok()
                    .map(|voicing| plain("Voicing", voicing)),
            ))
        },
        Builtin::VoicingPitches => |arguments| {
            let voicing = read::<musa_score::chord::Voicing>(arguments.first()?)?;
            reduced(listing(voicing.pitches().map(|pitch| plain("Pitch", pitch))))
        },
        Builtin::VoicingBass => |arguments| {
            reduced(plain(
                "Pitch",
                read::<musa_score::chord::Voicing>(arguments.first()?)?.bass(),
            ))
        },
        Builtin::VoicingChord => |arguments| {
            reduced(plain(
                "ChordClass",
                read::<musa_score::chord::Voicing>(arguments.first()?)?.class(),
            ))
        },
        Builtin::VoicingPosition => |arguments| {
            let place = read::<musa_score::chord::Voicing>(arguments.first()?)?
                .inversion()
                .and_then(|place| u64::try_from(place).ok());
            reduced(optional(place.map(whole)))
        },
        Builtin::CloseVoicing => |arguments| {
            let (class, bass) = (
                read::<musa_score::chord::ChordClass>(arguments.first()?)?,
                read::<WrittenPitch>(arguments.get(1)?)?,
            );
            reduced(optional(
                musa_score::chord::Voicing::close_position(class, bass)
                    .ok()
                    .map(|voicing| plain("Voicing", voicing)),
            ))
        },
        Builtin::DropVoicing => |arguments| {
            let (class, bass) = (
                read::<musa_score::chord::ChordClass>(arguments.first()?)?,
                read::<WrittenPitch>(arguments.get(1)?)?,
            );
            let voice = usize::try_from(nat(arguments.get(2)?)?).ok()?;
            reduced(optional(
                musa_score::chord::Voicing::dropped(class, bass, voice)
                    .ok()
                    .map(|voicing| plain("Voicing", voicing)),
            ))
        },
        Builtin::OmitVoicing => |arguments| {
            let voicing = read::<musa_score::chord::Voicing>(arguments.first()?)?;
            let place = usize::try_from(nat(arguments.get(1)?)?).ok()?;
            reduced(optional(
                voicing.omitting(place).ok().map(|omitted| plain("Voicing", omitted)),
            ))
        },

        // ---- the twelve-tone domains ----
        Builtin::Pc12Of => |arguments| {
            reduced(plain(
                "Pc12",
                musa_score::pc12::Pc12::from_number(nat(arguments.first()?)?),
            ))
        },
        Builtin::Pc12Number => |arguments| {
            reduced(whole(u64::from(
                read::<musa_score::pc12::Pc12>(arguments.first()?)?.number(),
            )))
        },
        Builtin::Pc12Forget => |arguments| {
            reduced(plain(
                "Pc12",
                musa_score::pc12::Pc12::forgetting(read::<PitchClass>(arguments.first()?)?),
            ))
        },
        Builtin::Pc12Transposed => |arguments| {
            let member = read::<musa_score::pc12::Pc12>(arguments.first()?)?;
            reduced(plain("Pc12", member.transposed(nat(arguments.get(1)?)?)))
        },
        Builtin::Pc12Inverted => |arguments| {
            let member = read::<musa_score::pc12::Pc12>(arguments.first()?)?;
            reduced(plain("Pc12", member.inverted(nat(arguments.get(1)?)?)))
        },
        Builtin::Pc12Spelled => |arguments| {
            let (member, collection) = (
                read::<musa_score::pc12::Pc12>(arguments.first()?)?,
                read::<musa_score::scale::Scale>(arguments.get(1)?)?,
            );
            reduced(optional(
                member.spelled(collection).map(|class| plain("PitchClass", class)),
            ))
        },
        Builtin::PcSet12Of => |arguments| {
            reduced(plain(
                "PcSet12",
                musa_score::pc12::PcSet12::of(pc12s(arguments.first()?)?),
            ))
        },
        Builtin::PcSet12Members => |arguments| {
            let set = read::<musa_score::pc12::PcSet12>(arguments.first()?)?;
            reduced(pc12_listing(set.members().collect()))
        },
        Builtin::PcSet12Normal => |arguments| {
            let set = read::<musa_score::pc12::PcSet12>(arguments.first()?)?;
            reduced(pc12_listing(set.normal_order()))
        },
        Builtin::PcSet12Transposed => |arguments| {
            let set = read::<musa_score::pc12::PcSet12>(arguments.first()?)?;
            reduced(plain("PcSet12", set.transposed(nat(arguments.get(1)?)?)))
        },
        Builtin::PcSet12Inverted => |arguments| {
            let set = read::<musa_score::pc12::PcSet12>(arguments.first()?)?;
            reduced(plain("PcSet12", set.inverted(nat(arguments.get(1)?)?)))
        },
        Builtin::PcSet12Prime => |arguments| {
            let set = read::<musa_score::pc12::PcSet12>(arguments.first()?)?;
            reduced(plain("PcSet12", set.prime_form()))
        },
        Builtin::PcSet12Vector => |arguments| {
            let set = read::<musa_score::pc12::PcSet12>(arguments.first()?)?;
            reduced(listing(
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
            Some(match musa_score::pc12::Row12::checked(&pcs) {
                // The one operation the criterion leaves as a value, and the
                // error type is what says so: a `RowFault` names *which*
                // positions repeat and *which* classes are missing, which is an
                // analysis a program reads rather than a sentence a composer is
                // told. `stdlib/src/post_tonal/serial.musa`'s `row` writes the
                // `Result<Row12, (List<Nat>, List<Pc12>)>` out and hands it on.
                Some(row) => Answer::Reduced(case("Result.Ok", vec![plain("Row12", row)])),
                None => Answer::Reduced(case(
                    "Result.Err",
                    vec![case(
                        "RowFault.Fault",
                        vec![
                            listing(musa_score::pc12::repeated_positions(&pcs).into_iter().map(whole)),
                            pc12_listing(musa_score::pc12::missing_classes(&pcs)),
                        ],
                    )],
                )),
            })
        },
        Builtin::Row12Pcs => |arguments| {
            let row = read::<musa_score::pc12::Row12>(arguments.first()?)?;
            reduced(pc12_listing(row.pcs().collect()))
        },
        Builtin::Row12Head => |arguments| {
            reduced(plain(
                "Pc12",
                read::<musa_score::pc12::Row12>(arguments.first()?)?.head(),
            ))
        },
        Builtin::Row12Transposed => |arguments| {
            let row = read::<musa_score::pc12::Row12>(arguments.first()?)?;
            reduced(plain("Row12", row.transposed(nat(arguments.get(1)?)?)))
        },
        Builtin::Row12Inverted => |arguments| {
            let row = read::<musa_score::pc12::Row12>(arguments.first()?)?;
            reduced(plain("Row12", row.inverted(nat(arguments.get(1)?)?)))
        },
        Builtin::Row12Retrograde => |arguments| {
            reduced(plain(
                "Row12",
                read::<musa_score::pc12::Row12>(arguments.first()?)?.retrograde(),
            ))
        },
        Builtin::Row12Matrix => |arguments| {
            let row = read::<musa_score::pc12::Row12>(arguments.first()?)?;
            reduced(listing(row.matrix().into_iter().map(|form| plain("Row12", form))))
        },
        Builtin::Row12Forms => |arguments| {
            reduced(whole(u64::from(
                read::<musa_score::pc12::Row12>(arguments.first()?)?.forms(),
            )))
        },
        Builtin::Row12Symmetries => |arguments| {
            reduced(whole(u64::from(
                read::<musa_score::pc12::Row12>(arguments.first()?)?.symmetries(),
            )))
        },
        Builtin::Row12Repeats => |arguments| {
            let pcs = pc12s(arguments.first()?)?;
            reduced(listing(
                musa_score::pc12::repeated_positions(&pcs).into_iter().map(whole),
            ))
        },
        Builtin::Row12Missing => |arguments| {
            reduced(pc12_listing(musa_score::pc12::missing_classes(&pc12s(
                arguments.first()?,
            )?)))
        },

        // ---- the 25 rows this module does not own ----
        //
        // Eight structural eliminators over declared families, whose recursors
        // `musa_calculus::declare` already generated; eight track and nine machine
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
        | Builtin::Swap => return None,
    })
}

/// A `List Pc12`.
fn pc12_listing(members: Vec<musa_score::pc12::Pc12>) -> Datum {
    listing(members.into_iter().map(|member| plain("Pc12", member)))
}

// ---- the expansion phase ----

/// A token kind, wrapped so that it can print.
///
/// `musa_syntax::SyntaxKind` is another crate's, so the orphan rule puts
/// [`std::fmt::Display`] out of reach for it here. The wrapper is the smallest
/// thing that fixes that, and the spelling it prints is the one the phase
/// registry offers the kind under — the same table `token_kind_equal` compares
/// against, so what a diagnostic shows is what an adapter would have written.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Kind(pub(super) musa_syntax::SyntaxKind);

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
fn where_at(path: crate::quote::NodePath) -> Datum {
    plain("NodePath", path)
}

/// The path argument of a builder.
fn path(datum: &Datum) -> Option<crate::quote::NodePath> {
    read::<crate::quote::NodePath>(datum)
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
            reduced(optional(subject.at(&wanted).cloned().map(built)))
        },
        // The number is built into a token rather than handed over as a `Nat`: a
        // transformer's answer is an expression, and the only way a number
        // reaches one is as a literal the ordinary parser reads.
        SyntaxOp::Anchor => |arguments| {
            let subject = node(arguments.first()?)?;
            let wanted = path(arguments.get(1)?)?;
            let found = subject.anchor(&wanted).map(|anchor| {
                crate::quote::token(
                    crate::quote::anchor_place(&wanted),
                    musa_syntax::SyntaxKind::Integer,
                    anchor.to_string(),
                )
            });
            reduced(optional(found.map(built)))
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
                    if kind == musa_syntax::SyntaxKind::Integer {
                        text.parse::<i64>().ok().map(Ratio::from_integer)
                    } else if kind == musa_syntax::SyntaxKind::Rational {
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
            reduced(optional(found.map(exact)))
        },
        // Written through `Derived` rather than through `NodePath::built`
        // directly, because the triple is what a derived path *is*: the origin
        // the builder was pointed at, the construction site that read it, and
        // the position within what that site built.
        SyntaxOp::Built => |arguments| {
            reduced(where_at(
                crate::quote::Derived {
                    origin: path(arguments.first()?)?,
                    quotation: role(arguments.get(1)?)?,
                    path: vec![role(arguments.get(2)?)?],
                }
                .path(),
            ))
        },
        SyntaxOp::Binding => |arguments| {
            reduced(plain(
                "BindingPath",
                path(arguments.first()?)?.binding(role(arguments.get(1)?)?),
            ))
        },
        SyntaxOp::Token => |arguments| {
            reduced(built(crate::quote::token(
                path(arguments.first()?)?,
                read::<Kind>(arguments.get(1)?)?.0,
                text(arguments.get(2)?)?,
            )))
        },
        SyntaxOp::Identifier => |arguments| {
            reduced(built(crate::quote::identifier(
                path(arguments.first()?)?,
                text(arguments.get(1)?)?,
            )))
        },
        SyntaxOp::Group => |arguments| {
            let children = items(arguments.get(2)?)?
                .into_iter()
                .map(node)
                .collect::<Option<Vec<_>>>()?;
            reduced(built(crate::quote::group(
                path(arguments.first()?)?,
                read::<crate::quote::Delimiter>(arguments.get(1)?)?,
                children,
            )))
        },
        SyntaxOp::Binder => |arguments| {
            reduced(built(crate::quote::binder(
                &read::<crate::quote::BindingPath>(arguments.first()?)?,
                text(arguments.get(1)?)?,
            )))
        },
        SyntaxOp::Reference => |arguments| {
            reduced(built(crate::quote::reference(
                path(arguments.first()?)?,
                &read::<crate::quote::BindingPath>(arguments.get(1)?)?,
                text(arguments.get(2)?)?,
            )))
        },
        SyntaxOp::KindEqual => |arguments| {
            reduced(boolean(
                read::<Kind>(arguments.first()?)? == read::<Kind>(arguments.get(1)?)?,
            ))
        },
        SyntaxOp::DelimiterEqual => |arguments| {
            let (left, right) = (
                read::<crate::quote::Delimiter>(arguments.first()?)?,
                read::<crate::quote::Delimiter>(arguments.get(1)?)?,
            );
            reduced(boolean(left == right))
        },
        // The claim is established by running the real parser, which is the only
        // thing that can establish it. Nothing about the value changes — the
        // index is a claim about how the tree parses, and this is the question
        // being asked.
        SyntaxOp::AsExpression => |arguments| {
            let subject = node(arguments.first()?)?;
            let parses = crate::quote::parses_as_expression(&subject);
            reduced(optional(parses.then(|| tree(Cat::Expr, subject))))
        },
        // The gate answers with a value either way, which is what keeps it
        // total: a transformer that builds badly gets a `Result` back and
        // decides what to say about it.
        SyntaxOp::Checked => |arguments| {
            let subject = node(arguments.first()?)?;
            Some(match crate::quote::check_expression(&subject) {
                Ok(()) => answered(tree(Cat::Expr, subject)),
                Err(refusal) => errored(&refusal.to_string()),
            })
        },
        // Prompt 141f's three. A traversal takes a function argument, so it is
        // §5.8's second family and not this one.
        SyntaxOp::Recurse | SyntaxOp::Run | SyntaxOp::Fold => return None,
    })
}

// ---- the forgetting rule ----

/// Whether a `Syntax ⟨wanted⟩` position accepts a `Syntax ⟨held⟩`, and what
/// carries it there.
///
/// `11-quotation.md` §1's forgetting rule, as `musa-calculus` asks it: a token-tree
/// position accepts a tree of any category, because a token-tree position is
/// precisely one that has not been parsed as anything more specific, and every
/// other position requires its own category exactly. Directional on purpose —
/// the reverse, a `Syntax ⟨expr⟩` position taking a tree nobody parsed, is the
/// uncertified splice the index exists to refuse, and the core asks this only
/// where a direction exists.
///
/// `None` at two equal categories, which is [`musa_calculus::Accepts`]'s contract:
/// there is nothing to carry, and ordinary conversion says so more cheaply than
/// a coercion nobody would read.
pub(super) fn forgets(wanted: &Literal, held: &Literal) -> Option<&'static str> {
    let (wanted, held) = (super::held::<Cat>(wanted)?, super::held::<Cat>(held)?);
    (*wanted == Cat::TokenTree && *held != Cat::TokenTree).then_some(FORGOTTEN)
}

/// `forget_category(subject)` — the same tree, no longer claiming how it parses.
///
/// The value does not change and the *type* does, which is the content of a
/// forgetting rule: [`tree`] rebuilds the literal at `Syntax ⟨token-tree⟩` so
/// that what the position holds afterwards is a value of the type the position
/// asked for. Answering the argument unchanged would leave a `Syntax ⟨expr⟩`
/// standing where the elaborated term says otherwise, which the re-checker would
/// then be right to refuse.
pub(super) const FORGET: Rule = |arguments| reduced(built(node(arguments.first()?)?));

// ---- quotation ----

/// A quote's body and the construction site that read it.
///
/// Two fields and one literal, because [`crate::quote::instantiate`] needs both
/// and only one of them is an argument. `11-quotation.md` §3 mints a node's
/// identity from the anchor, the quotation, and the position in the template; the
/// anchor is written at the use site and the other two belong to the quote
/// itself, so they travel with it.
///
/// The wrapper exists for [`Kind`]'s reason as well: [`crate::quote::Template`]
/// is this crate's, but a payload must print, and what a template should print as
/// is its site rather than its shape — a diagnostic naming a hundred-node body
/// would say nothing a reader could use.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Quotation {
    pub(super) template: crate::quote::Template,
    pub(super) quotation: u32,
}

impl std::fmt::Display for Quotation {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(out, "quote #{}", self.quotation)
    }
}

/// Which holes a template has, in the order it numbers them, and whether each
/// stands for a run.
///
/// Read off the template rather than carried beside it: the walk that built it
/// numbered the holes densely from zero, and a count stored next to the body
/// would be a second answer to a question the body already answers.
/// [`crate::quote::matched`] takes the count as an argument for the same reason
/// the checker could supply it — here the template is all there is.
fn hole_kinds(template: &crate::quote::Template) -> Vec<bool> {
    fn walk(template: &crate::quote::Template, found: &mut Vec<(usize, bool)>) {
        match *template {
            crate::quote::Template::Splice(hole) => found.push((hole, false)),
            crate::quote::Template::Sequence(hole) => found.push((hole, true)),
            crate::quote::Template::Group { ref children, .. } => {
                for child in children {
                    walk(child, found);
                }
            }
            crate::quote::Template::Missing
            | crate::quote::Template::Token { .. }
            | crate::quote::Template::Identifier { .. } => {}
        }
    }
    let mut found = Vec::new();
    walk(template, &mut found);
    let mut kinds = vec![false; found.iter().map(|&(hole, _)| hole.saturating_add(1)).max().unwrap_or(0)];
    for (hole, sequence) in found {
        if let Some(slot) = kinds.get_mut(hole) {
            *slot = sequence;
        }
    }
    kinds
}

/// What matching a subject against one quote pattern found.
///
/// Two cases and not [`Option`], because the [`Option`] the reading already
/// answers means something else: `None` there is "these arguments are not the
/// shapes this rule reads", which leaves the term neutral, and `No` here is a
/// perfectly good answer that the pattern does not match.
enum Matched {
    /// The subject has another shape.
    No,
    /// The subject is this template's, with what each hole bound, in order.
    Holes(Vec<crate::quote::Spliced>),
}

impl Matched {
    /// What hole `which` bound, where the subject matched and has one.
    fn hole(self, which: usize) -> Option<crate::quote::Spliced> {
        match self {
            Self::No => None,
            Self::Holes(holes) => holes.into_iter().nth(which),
        }
    }
}

/// What one quote pattern bound.
///
/// The one reading the three pattern rules share, so that the answer to "does
/// this match" and the answer to "what did hole two bind" cannot disagree.
fn bound(arguments: &[Datum]) -> Option<Matched> {
    let subject = node(arguments.first()?)?;
    let quoted = read::<Quotation>(arguments.get(1)?)?;
    let holes = hole_kinds(&quoted.template).len();
    Some(crate::quote::matched(&quoted.template, &subject, holes).map_or(Matched::No, Matched::Holes))
}

/// The hole a pattern rule was asked about.
fn which(datum: &Datum) -> Option<usize> {
    usize::try_from(nat(datum)?).ok()
}

/// `instantiate_quote(anchor, template, splices)`.
///
/// The whole of building a quote, and it is [`crate::quote::instantiate`] —
/// which mints every derived path, carries every spliced node in with the
/// identity it arrived with, and is the *only* implementation of either. Reading
/// the arguments and shaping the answer is all that happens here.
///
/// The splices arrive as a list of lists because a hole is a run or a node and
/// the core has no sum of the two that a lowering could write; which of the two a
/// hole is, the template says. A single hole handed anything but one node
/// answers [`None`] — the arity mismatch [`crate::quote::Spliced`] refuses,
/// refused where it was, and unreachable from a lowered quote because the same
/// template decided both.
pub(super) const INSTANTIATE: Rule = |arguments| {
    let anchor = path(arguments.first()?)?;
    let quoted = read::<Quotation>(arguments.get(1)?)?;
    let kinds = hole_kinds(&quoted.template);
    let written = items(arguments.get(2)?)?;
    if written.len() != kinds.len() {
        return None;
    }
    let mut spliced = Vec::with_capacity(kinds.len());
    for (&sequence, hole) in kinds.iter().zip(written) {
        let nodes = items(hole)?.into_iter().map(node).collect::<Option<Vec<_>>>()?;
        spliced.push(if sequence {
            crate::quote::Spliced::Many(nodes)
        } else {
            let [one] = <[Syntax; 1]>::try_from(nodes).ok()?;
            crate::quote::Spliced::One(one)
        });
    }
    let built = crate::quote::instantiate(&quoted.template, &anchor, quoted.quotation, &spliced)?;
    // The certificate the `⟨expr⟩` index claims, checked rather than believed:
    // a splice can carry any tree into the body, so what comes out is an
    // expression only when the parser says so. None is a stuck term, not a
    // diagnostic — the adapter that spliced the tree is the defect, and
    // 147's round-trip obligation owns the reporting of it.
    if !crate::quote::parses_as_expression(&built) {
        return None;
    }
    reduced(tree(Cat::Expr, built))
};

/// `match_quote(subject, template)` — whether the pattern's shape is this
/// value's.
pub(super) const MATCHES: Rule = |arguments| reduced(boolean(matches!(bound(arguments)?, Matched::Holes(_))));

/// `quote_hole(subject, template, i)` — the one node hole `i` binds.
///
/// A node either way, because what binds it is a `let` and a `let` binds one
/// type. Reached only under [`MATCHES`], which is what makes the answer for a
/// subject of another shape a question of totality rather than of meaning: a
/// [`crate::quote::Syntax::Missing`] at the subject's own place is the node a
/// reader expected and did not find, which is exactly the situation.
///
/// One rule body at two categories — [`HOLE`] answers at `⟨token-tree⟩` and
/// [`HOLE_EXPR`] at `⟨expr⟩` — because 11-quotation §4 reads a pattern at the
/// scrutinee's category and a δ signature cannot take that category as a
/// variable (the D1 test in [`super::quotation`]'s doc). The lowering picks
/// the reader from the scrutinee's *written* category; the rule answers what
/// it is asked.
pub(super) const HOLE: Rule = |arguments| hole(arguments, Cat::TokenTree);

/// [`HOLE`] answering at `⟨expr⟩`, for a scrutinee written `Syntax<Expr>`.
pub(super) const HOLE_EXPR: Rule = |arguments| hole(arguments, Cat::Expr);

/// The shared body of [`HOLE`] and [`HOLE_EXPR`]: the hole's node, claimed at
/// `cat` — the scrutinee's category, which is the only claim the match that
/// gates this read supports.
fn hole(arguments: &[Datum], cat: Cat) -> Option<Answer> {
    let subject = node(arguments.first()?)?;
    let held = bound(arguments)?
        .hole(which(arguments.get(2)?)?)
        .and_then(|held| match held {
            crate::quote::Spliced::One(one) => Some(one),
            crate::quote::Spliced::Many(_) => None,
        });
    reduced(tree(
        cat,
        held.unwrap_or_else(|| Syntax::Missing(crate::quote::SourceInfo::Generated(subject.info().path().clone()))),
    ))
}

/// `quote_holes(subject, template, i)` — the run hole `i` binds.
///
/// The empty list where [`HOLE`] answers `Missing`, and for the same reason: a
/// run of no nodes is the answer a spread of nothing already gives, so the
/// unreachable case needs no case of its own.
pub(super) const HOLES: Rule = |arguments| holes(arguments, Cat::TokenTree);

/// [`HOLES`] answering at `⟨expr⟩`, for a scrutinee written `Syntax<Expr>`.
pub(super) const HOLES_EXPR: Rule = |arguments| holes(arguments, Cat::Expr);

/// The shared body of [`HOLES`] and [`HOLES_EXPR`] — one list, claimed at the
/// scrutinee's category for [`hole`]'s reason.
fn holes(arguments: &[Datum], cat: Cat) -> Option<Answer> {
    let run = bound(arguments)?
        .hole(which(arguments.get(2)?)?)
        .map_or_else(Vec::new, |held| match held {
            crate::quote::Spliced::One(one) => vec![one],
            crate::quote::Spliced::Many(many) => many,
        });
    reduced(listing(run.into_iter().map(|node| tree(cat, node))))
}
