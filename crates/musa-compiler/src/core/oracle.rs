//! The old evaluator, asked the same questions the new rules are asked.
//!
//! Prompt 141e's whole claim is that [`crate::registry::rules`] is a
//! *translation* of `eval_builtin` rather than a rewrite of it. A claim like that
//! is worth exactly as much as the thing that checks it, so this module runs the
//! old arm and the new rule on the same inputs and hands both answers back in one
//! shape for comparison.
//!
//! # Why it lives here and not beside the law
//!
//! `Expr`, `Value`, and `eval_builtin` are private to [`crate::core`], and they
//! stay that way: they are the largest thing prompt 142 deletes, and widening
//! them so a test in another module can see them would leave the crate's most
//! doomed type visible from everywhere at exactly the moment it should be getting
//! harder to reach. So the sampling, the evaluation, and the translation of both
//! sides into [`Datum`] all happen inside `core`, and what leaves is data.
//!
//! # Sampled from the signature, not from a list
//!
//! There is no table of hand-written arguments here. Every δ row already declares
//! its argument [`Shape`]s, so the samples are generated from those: one value per
//! shape per seed, three seeds per row. A hand-written table would have 92 rows to
//! keep in step with the signatures, and the first signature to change would leave
//! it silently testing the wrong thing.
//!
//! # The translation here is deliberately not the one under test
//!
//! [`as_datum`] writes `Nat`, `List`, `Option`, and `Result` by hand rather than
//! calling [`crate::registry::rules`]'s writers. That duplication is the point: an
//! oracle that shared the encoder with the code it checks would agree with a
//! broken encoder just as happily as with a correct one.

use std::sync::Arc;

use musa_core::Datum;
use num_rational::Ratio;

use super::{BUILTIN_OWNERSHIP, Builtin, Expr, ExprKind, Family, Shape, Type, Value, eval_builtin};
use crate::core::Base as Leaf;
use crate::core_budget::{Budget, WorkMeter};
use crate::origin::{Interval, SourceSpan};
use crate::registry::{literal, plain_type, tagged_type};

/// One sampled application, with the answer the old evaluator gave.
pub(crate) struct Case {
    /// What the operation was applied to.
    pub(crate) arguments: Vec<Datum>,
    /// What `eval_builtin` answered, or `None` where it declined to.
    pub(crate) answer: Option<Datum>,
}

/// How many argument tuples each operation is sampled at.
///
/// Three, because that is enough for the two things a translation gets wrong:
/// an argument read from the wrong position, which one tuple with distinct
/// values already catches, and a branch taken on the value, which needs more
/// than one tuple to reach. Exhaustiveness is not the goal and could not be —
/// the goal is that the two implementations agree wherever they are both asked.
const SEEDS: u64 = 3;

/// Every sampled application of one δ operation, answered by the old evaluator.
///
/// Empty for an operation with no δ signature, and for one whose argument shapes
/// have no sampler — both of which the law reads as "this row was not checked"
/// and reports rather than passing over.
pub(crate) fn cases(operation: Builtin) -> Vec<Case> {
    let Some(entry) = BUILTIN_OWNERSHIP.iter().find(|entry| entry.operation == operation) else {
        return Vec::new();
    };
    let Family::Delta { arguments, .. } = entry.family else {
        return Vec::new();
    };
    (0..SEEDS)
        .filter_map(|seed| {
            let values = arguments
                .iter()
                .enumerate()
                .map(|(place, shape)| sample(*shape, seed.wrapping_add(place as u64)))
                .collect::<Option<Vec<_>>>()?;
            let answer = evaluated(operation, &values);
            Some(Case {
                arguments: values.iter().map(as_datum).collect::<Option<Vec<_>>>()?,
                answer: match answer {
                    Some(ref value) => Some(as_datum(value)?),
                    None => None,
                },
            })
        })
        .collect()
}

/// One δ-builtin, run by the old evaluator on already-evaluated arguments.
///
/// Wrapping each value in a literal expression is the identity as far as
/// evaluation is concerned — `eval_builtin` evaluates its arguments first — so
/// what comes back is exactly the arm's own answer.
fn evaluated(operation: Builtin, arguments: &[Value]) -> Option<Value> {
    let span = SourceSpan::new(0, 0);
    let arguments = arguments
        .iter()
        .map(|value| Expr {
            kind: ExprKind::Literal(value.clone()),
            ty: value.ty(),
            span,
        })
        .collect::<Vec<_>>();
    // Read by `Builtin::Map` alone, which is a structural eliminator and not one
    // of the 92 this oracle is asked about.
    let expression = Expr {
        kind: ExprKind::Literal(Value::Bool(false)),
        ty: Type::Unit,
        span,
    };
    eval_builtin(
        operation,
        &arguments,
        &indexmap::IndexMap::new(),
        &mut WorkMeter::new(Budget::LANGUAGE),
        &expression,
    )
}

/// A value of the shape a signature declares.
fn sample(shape: Shape, seed: u64) -> Option<Value> {
    Some(match shape {
        Shape::Base(leaf) => base(leaf, seed)?,
        Shape::Option(member) => Value::Option {
            member: member.ty(),
            value: match seed % 2 {
                0 => Some(Box::new(sample(*member, seed)?)),
                _ => None,
            },
        },
        // Two members rather than one, so that a rule which reads only the head
        // of a list disagrees with one that reads all of it.
        Shape::List(member) => Value::List {
            member: member.ty(),
            values: vec![sample(*member, seed)?, sample(*member, seed.wrapping_add(1))?],
        },
        Shape::Result(value, error) => Value::Sum {
            value_type: value.ty(),
            error_type: error.ty(),
            error: false,
            held: Box::new(sample(*value, seed)?),
        },
        Shape::Fault => Value::Product(vec![sample(super::NATS, seed)?, sample(super::PC12S, seed)?]),
    })
}

/// A value of one inert domain, or of the two families that are not.
///
/// Every domain that has to be *constructed* rather than counted is built from a
/// spelling the reader would accept, so the sample is a value a composer could
/// have written rather than a bit pattern that happens to fit.
fn base(leaf: Leaf, seed: u64) -> Option<Value> {
    let place = usize::try_from(seed % 3).ok()?;
    let one_of = |spellings: [&'static str; 3]| spellings.get(place).copied();
    let pitch = || crate::pitch::WrittenPitch::parse(one_of(["c4", "e4", "g4"])?);
    let class = || crate::pitch::PitchClass::parse(one_of(["c", "e", "g"])?);
    let key = || Some(crate::score::Key::new(class()?, crate::score::Mode::Major));
    let scale = || Some(crate::scale::signature_scale(key()?));
    let chord = || {
        Some(crate::chord::ChordClass::new(
            class()?,
            crate::chord::ChordType::named(one_of(["major", "minor", "dominant seventh"])?)?,
        ))
    };
    Some(match leaf {
        Leaf::Bool => Value::Bool(seed.is_multiple_of(2)),
        Leaf::Nat => Value::Nat(seed % 5),
        Leaf::Ratio => Value::Ratio(Ratio::new(i64::try_from(seed).ok()?.checked_sub(1)?, 2)),
        Leaf::Text => Value::Text(format!("sample {seed}")),
        // Nonnegative, because a duration that is not would be refused by its own
        // constructor and the sample would be testing the refusal every time.
        Leaf::Duration(coordinate) => Value::Duration(coordinate, Ratio::new(i64::try_from(seed).ok()?, 2)),
        Leaf::Position(coordinate) => {
            Value::Position(coordinate, Ratio::new(i64::try_from(seed).ok()?.checked_sub(1)?, 2))
        }
        Leaf::Pitch => Value::Pitch(pitch()?),
        Leaf::PitchClass => Value::PitchClass(class()?),
        Leaf::Interval => Value::Interval(Interval::parse(one_of(["P5", "M3", "m3"])?, false)?),
        Leaf::Key => Value::Key(key()?),
        Leaf::Scale => Value::Scale(scale()?),
        Leaf::Degree => Value::Degree(crate::scale::Degree::new(i64::try_from(seed % 7).ok()?.checked_add(1)?)),
        Leaf::Frame => Value::Frame(crate::scale::Frame::new(scale()?, pitch()?)?),
        Leaf::ChordClass => Value::ChordClass(chord()?),
        Leaf::Triad => Value::Triad(crate::chord::Triad::of(chord()?)?),
        Leaf::Roman => Value::Roman(crate::roman::Roman::new(
            u64::try_from(place).ok()?.checked_add(1)?,
            3,
            0,
        )?),
        Leaf::Voicing => Value::Voicing(crate::chord::Voicing::close_position(chord()?, pitch()?).ok()?),
        Leaf::Pc12 => Value::Pc12(crate::pc12::Pc12::from_number(seed % 12)),
        Leaf::PcSet12 => Value::PcSet12(crate::pc12::PcSet12::of(
            [0_u64, 4, 7].map(|number| crate::pc12::Pc12::from_number(number.wrapping_add(seed % 5))),
        )),
        Leaf::Row12 => Value::Row12(crate::pc12::Row12::checked(
            &(0_u64..12)
                .map(|number| crate::pc12::Pc12::from_number(number.wrapping_add(seed)))
                .collect::<Vec<_>>(),
        )?),
    })
}

/// One old value as core data.
///
/// `None` for a value no δ signature can mention, which is most of the enum:
/// closures, machines, music, and syntax are not storable data, and a δ-builtin
/// that answered one would have been refused where `BUILTIN_OWNERSHIP` is
/// written.
#[expect(
    clippy::wildcard_enum_match_arm,
    reason = "listing forty non-storable variants would hide the twenty that matter; `Value` leaves with prompt 142"
)]
fn as_datum(value: &Value) -> Option<Datum> {
    // Each domain keeps its own Rust type on the way in. A payload compares by
    // downcasting to itself, so a `Pitch` encoded as its spelling would never
    // equal the `Pitch` a rule wrote, and the law would pass by never matching.
    macro_rules! plain {
        ($name:literal, $held:expr) => {
            Datum::Lit(literal(plain_type($name), $held))
        };
    }
    Some(match *value {
        Value::Bool(held) => case(if held { "Bool.True" } else { "Bool.False" }, Vec::new()),
        Value::Nat(held) => {
            let mut built = case("Nat.Zero", Vec::new());
            for _ in 0..held {
                built = case("Nat.Succ", vec![built]);
            }
            built
        }
        Value::Ratio(held) => Datum::Lit(literal(plain_type("Ratio"), held)),
        Value::Text(ref held) => Datum::Lit(literal(plain_type("Text"), held.clone())),
        Value::Duration(coordinate, held) => Datum::Lit(literal(tagged_type("Duration", coordinate), held)),
        Value::Position(coordinate, held) => Datum::Lit(literal(tagged_type("Position", coordinate), held)),
        Value::Pitch(held) => plain!("Pitch", held),
        Value::PitchClass(held) => plain!("PitchClass", held),
        Value::Interval(held) => plain!("Interval", held),
        Value::Key(held) => plain!("Key", held),
        Value::Scale(held) => plain!("Scale", held),
        Value::Degree(held) => plain!("Degree", held),
        Value::Frame(held) => plain!("Frame", held),
        Value::ChordClass(held) => plain!("ChordClass", held),
        Value::Triad(held) => plain!("Triad", held),
        Value::Roman(held) => plain!("Roman", held),
        Value::Voicing(ref held) => plain!("Voicing", held.clone()),
        Value::Pc12(held) => plain!("Pc12", held),
        Value::PcSet12(held) => plain!("PcSet12", held),
        Value::Row12(held) => plain!("Row12", held),
        Value::Option { ref value, .. } => match *value {
            Some(ref held) => case("Option.Some", vec![as_datum(held)?]),
            None => case("Option.None", Vec::new()),
        },
        Value::List { ref values, .. } => values
            .iter()
            .rev()
            .try_fold(case("List.Empty", Vec::new()), |rest, member| {
                Some(case("List.Cons", vec![as_datum(member)?, rest]))
            })?,
        Value::Sum { error, ref held, .. } => {
            case(if error { "Result.Err" } else { "Result.Ok" }, vec![as_datum(held)?])
        }
        // The one product in the whole table, and prompt 141e's Design is what
        // made it a name: `row12_of` answers the order positions that repeat and
        // the pitch classes the sequence never names.
        Value::Product(ref fields) => case(
            "RowFault.Fault",
            fields.iter().map(as_datum).collect::<Option<Vec<_>>>()?,
        ),
        _ => return None,
    })
}

/// A constructor applied to its fields.
fn case(constructor: &'static str, fields: Vec<Datum>) -> Datum {
    Datum::Case {
        constructor: Arc::from(constructor),
        fields,
    }
}
