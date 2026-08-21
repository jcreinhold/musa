//! What has to be true of the compiler's registrations.
//!
//! Unit tests rather than a file in `tests/suite/`, and that is forced rather
//! than chosen: the laws read `crate::phase`'s registration tables directly, and
//! those are private to it. A test outside this crate links against
//! `parse`/`compile`/`render_notation` and can reach none of them, and
//! widening the facade so it could would leave the crate's internal types
//! visible from everywhere.

#![expect(
    clippy::expect_used,
    clippy::panic,
    reason = "a law that cannot fail loudly is not a law"
)]

use musa_calculus::{Answer, Datum, Refusal, Term};

use super::{bases, builtins, owned, rules};
use crate::phase::{BUILTIN_OWNERSHIP, Family, PhaseFamily, SYNTAX_OWNERSHIP};

/// The whole context builds: declarations, base types, and every registration
/// check [`musa_calculus::Registry::new`] makes.
///
/// This is the law the four-stage order in [`super`] exists for. A base type
/// whose kind named an undeclared family, a musical family holding an
/// unregistered base, or a δ signature that is not finite data all fail here
/// rather than at the first program that touches one — including the finite-data
/// check itself, which is what makes "every δ signature is finite data" a law
/// this file states by building the registry at all.
#[test]
fn the_compilers_own_context_builds() {
    owned().expect("the compiler's own declarations and registrations are well formed");
}

/// Every registered base type is nameable, by the spelling source uses.
#[test]
fn each_base_type_is_nameable() {
    let cx = owned().expect("the compiler's own context builds");
    for base in bases() {
        named(&cx, base.name());
    }
}

/// A family whose field names a base type is declared, which is what stage three
/// of the assembly order exists for.
#[test]
fn a_family_over_a_base_type_is_declared() {
    let cx = owned().expect("the compiler's own context builds");
    crate::prelude::constant(&cx, "RowFault.Fault").expect("`RowFault` holds a `List Pc12` and is declared");
}

/// Every δ spelling of both tables is registered, exactly once, under the name
/// the table gives it.
///
/// `Registry::new` already refuses a duplicate, so building the registry is half
/// of "exactly once". What this adds is the other half: that the *spelling* a
/// table row carries is the one a program would write, and that the count of
/// registrations is the count of δ rows rather than a subset that happened to
/// translate.
#[test]
fn every_delta_spelling_is_registered_exactly_once() {
    let cx = owned().expect("the compiler's own context builds");
    let registered = builtins(&cx).expect("both tables translate");
    let mut spellings: Vec<&str> = registered.iter().map(|builtin| &**builtin.name()).collect();
    spellings.sort_unstable();
    let unique = {
        let mut copy = spellings.clone();
        copy.dedup();
        copy
    };
    assert_eq!(spellings, unique, "a spelling was registered twice");
    assert_eq!(
        spellings.len(),
        rules::REGISTERED
            + rules::BEYOND.len()
            + super::track::TRACK_BEYOND.len()
            + super::notation::BEYOND.len()
            // Counted from the build's own primitive registry rather than
            // stated, because that is where the number comes from: one closed
            // signature per unit, and a build that registers another unit
            // registers another signature with it.
            + super::machine::primitives(&cx).expect("the port shapes name declared families").len(),
        "the registered count and the counts this module states have drifted"
    );
    for builtin in &registered {
        resolves(&cx, builtin.name());
    }
}

/// The operations past both tables are exactly the eleven that are said to be
/// past them, counted off the registry rather than off a table.
///
/// Two claims, and the second is the one that needs a test: that each is
/// registered, and that each is in *neither* ownership table. The second is what
/// keeps `instantiate_quote`, `set_note_pitches`, `instanced`, and `spliced` out
/// of an adapter's reach — the tables are the old checker's name lookup, and a
/// row added to one of them would be a word a transformer could write, silently.
#[test]
fn the_operations_past_both_tables_are_named_and_in_neither() {
    let cx = owned().expect("the compiler's own context builds");
    let registered = builtins(&cx).expect("both tables translate");
    for spelling in rules::BEYOND
        .into_iter()
        .chain(super::track::TRACK_BEYOND)
        .chain(super::notation::BEYOND)
    {
        assert!(
            registered.iter().any(|builtin| &**builtin.name() == spelling),
            "`{spelling}` is registered"
        );
        assert!(
            !BUILTIN_OWNERSHIP.iter().any(|entry| entry.spelling == spelling),
            "`{spelling}` is not a source word"
        );
        assert!(
            !SYNTAX_OWNERSHIP.iter().any(|entry| entry.spelling == spelling),
            "`{spelling}` is not a phase word either"
        );
        named(&cx, &std::sync::Arc::from(spelling));
    }
}

/// Every row of either ownership table names the information it hides.
///
/// The rationale is the row's own claim to being compiler-owned: an operation
/// whose hidden information is not stated is an operation whose ownership is
/// not argued. Prompt 141e made the column load-bearing; this keeps it from
/// going decorative.
#[test]
fn every_owned_operation_names_its_hidden_information() {
    for entry in &BUILTIN_OWNERSHIP {
        assert!(
            !entry.hidden_information.is_empty(),
            "`{}` does not say what it hides",
            entry.spelling
        );
    }
    for entry in &SYNTAX_OWNERSHIP {
        assert!(
            !entry.hidden_information.is_empty(),
            "`{}` does not say what it hides",
            entry.spelling
        );
    }
}

/// The rows with no registration are exactly the three families that are said to
/// have none, counted from the tables rather than from a comment.
///
/// The point of counting both ways is that neither number is trusted: the tables
/// say how many rows each family has, [`rules::UNREGISTERED`] says how many are
/// expected to be left, and 141e's Design says which families those are. A row
/// that quietly changed family would move one of the three and not the others.
///
/// The track family left this array in prompt 141h and is checked the other way
/// round in [`super::track::laws`] — that all eight of its rows *are*
/// registered. Prompt 141ha did the same to eight of the machine family's nine,
/// which is why that count is a subtraction: the row left is `primitive`, and
/// [`super::machine::laws`] checks the eight from the other side.
#[test]
fn the_unregistered_rows_are_the_families_they_are_said_to_be() {
    let eliminators = BUILTIN_OWNERSHIP
        .iter()
        .filter(|entry| matches!(entry.family, Family::Eliminator(_)))
        .count();
    // The machine family's rows minus the eight registered as constructors,
    // which leaves `primitive` and says so by subtraction rather than by naming
    // it twice.
    let machines = BUILTIN_OWNERSHIP
        .iter()
        .filter(|entry| matches!(entry.family, Family::Machine(_)))
        .count()
        - super::machine::SPELLINGS.len();
    // The descent family's rows minus the two registered as structural
    // eliminators, which is `run_syntax_step` and says so by subtraction rather
    // than by being named twice.
    let projections = SYNTAX_OWNERSHIP
        .iter()
        .filter(|entry| matches!(entry.family, PhaseFamily::Fold))
        .count()
        - super::traversal::SPELLINGS.len();
    let counted = [
        ("structural eliminators", eliminators),
        ("machine builtins", machines),
        ("phase projections", projections),
    ];
    assert_eq!(counted, rules::UNREGISTERED, "a family's row count moved");

    let left = BUILTIN_OWNERSHIP.len() + SYNTAX_OWNERSHIP.len() - rules::REGISTERED;
    assert_eq!(
        left,
        counted.iter().map(|&(_, count)| count).sum::<usize>(),
        "the rows left over are not the three families they are said to be"
    );

    // And nothing outside those four families is missing a rule.
    for entry in &BUILTIN_OWNERSHIP {
        let owned = matches!(entry.family, Family::Delta { .. });
        assert_eq!(
            rules::source(entry.operation).is_some(),
            owned,
            "`{}` has a rule exactly when it is δ, and does not",
            entry.spelling
        );
    }
    for entry in &SYNTAX_OWNERSHIP {
        let owned = matches!(entry.family, PhaseFamily::Builder);
        assert_eq!(
            rules::phase(entry.operation).is_some(),
            owned,
            "`{}` has a rule exactly when it is a builder, and does not",
            entry.spelling
        );
    }
}

/// Sixteen of the phase's seventeen rows are registered, and the seventeenth is
/// defined.
///
/// The claim prompt 141f closes, stated where both halves can be checked at once.
/// Fourteen builders and two traversals reach [`musa_calculus::Registry`];
/// `run_syntax_step` does not, and is a term that checks at its own type instead
/// — which is what makes leaving it out a design decision rather than a gap.
#[test]
fn sixteen_phase_rows_are_registered_and_one_is_defined() {
    let cx = owned().expect("the compiler's own context builds");
    let all = builtins(&cx).expect("both tables translate");
    let registered: Vec<&str> = all
        .iter()
        .map(|builtin| &**builtin.name())
        .filter(|name| SYNTAX_OWNERSHIP.iter().any(|entry| entry.spelling == *name))
        .collect();
    assert_eq!(registered.len(), 16, "fourteen builders and two traversals");

    let left: Vec<&str> = SYNTAX_OWNERSHIP
        .iter()
        .map(|entry| entry.spelling)
        .filter(|spelling| !registered.contains(spelling))
        .collect();
    assert_eq!(left, ["run_syntax_step"], "the one row left over is the projection");
    for spelling in super::traversal::SPELLINGS {
        assert!(registered.contains(&spelling), "`{spelling}` is registered");
    }

    // And the definition is a projection in the only sense that matters: it runs
    // the function the step sealed, at the context it is given.
    let defined = super::run_syntax_step(&cx).expect("`run_syntax_step` is definable");
    let cx = cx
        .define(&defined.ty, &defined.value)
        .expect("a checked definition is definable");
    let text = super::plain_type("Text");
    let step = crate::prelude::constant(&cx, "SyntaxStep.Step").expect("the constructor is declared");
    let sealed = applied(
        step,
        [
            text.clone(),
            text.clone(),
            Term::lam(super::HERE, "context", Term::var(super::HERE, musa_calculus::Index(0))),
        ],
    );
    let asked = super::literal(text.clone(), "the context".to_owned()).term(super::HERE);
    let run = applied(
        Term::var(super::HERE, musa_calculus::Index(0)),
        [text.clone(), text.clone(), asked.clone(), sealed],
    );
    assert_eq!(
        musa_calculus::normalize(&cx, &text, &run).expect("it reduces"),
        asked,
        "running a step is the sealed function applied to the context and nothing else"
    );
}

/// `head a b …`.
fn applied(head: Term, arguments: impl IntoIterator<Item = Term>) -> Term {
    arguments
        .into_iter()
        .fold(head, |function, argument| Term::app(super::HERE, function, argument))
}

/// `name` resolves in `cx`, which is what "registered" means to a program, and
/// the term it resolves to.
fn named(cx: &musa_calculus::Cx, name: &str) -> Term {
    let (term, _) = musa_calculus::infer(cx, &musa_calculus::Raw::var(super::HERE, name)).unwrap_or_else(|refusal| {
        let unknown = matches!(refusal, musa_calculus::ElabError::Refused(Refusal::UnknownName { .. }));
        panic!("`{name}` is not registered (unknown name: {unknown}): {refusal}");
    });
    term
}

/// `name` resolves in `cx`, without asking what it elaborates to.
///
/// Weaker than [`named`] on purpose, and the weakening is
/// `03-machine-calculus.md` §2's. Eight registered spellings are its machine
/// forms, whose type arguments are every one of them *implicit*, so inferring
/// one in isolation inserts metavariables with nothing to solve them from and
/// [`Refusal::Unsolved`] is the correct answer rather than a failure. It still
/// settles what this law asks — the name resolved and a signature was found —
/// and where those arguments actually come from is
/// [`super::machine::laws`]'s question, asked at the positions §2's rules put
/// them in.
pub(super) fn resolves(cx: &musa_calculus::Cx, name: &str) {
    if let Err(refusal) = musa_calculus::infer(cx, &musa_calculus::Raw::var(super::HERE, name)) {
        assert!(
            matches!(refusal, musa_calculus::ElabError::Refused(Refusal::Unsolved { .. })),
            "`{name}` is not registered: {refusal}"
        );
    }
}

/// A `Datum` is compared by value, which is what the agreement law relies on.
///
/// Stated because it is the one assumption that law makes and the one that would
/// fail silently: if [`musa_calculus::Payload::same`] answered `false` for equal
/// values, every comparison would fail, and if it answered `true` for unequal
/// ones, every comparison would pass.
#[test]
fn data_holding_the_same_domain_value_are_equal() {
    let (one, other) = (
        Datum::Lit(super::literal(super::plain_type("Text"), "same".to_owned())),
        Datum::Lit(super::literal(super::plain_type("Text"), "same".to_owned())),
    );
    assert_eq!(one, other, "two literals of one domain and one value are equal");
    let different = Datum::Lit(super::literal(super::plain_type("Text"), "other".to_owned()));
    assert_ne!(one, different, "two literals of one domain and two values are not");
    let elsewhere = Datum::Lit(super::literal(super::plain_type("Ratio"), "same".to_owned()));
    assert_ne!(one, elsewhere, "one value at two domains is two data");
}

/// Every partial exact-time operation states its own refusal.
///
/// D2 puts partiality where the composer reads it, and prompt 142 moved *where*
/// that is: these eleven rules used to answer `Result<τ, Text>`, so the law read
/// the error half out of the value. Now they answer bare `τ` and refuse through
/// [`Answer::Refused`], which is the same sentence delivered one layer down —
/// the caller no longer writes a `match` to get at a number that is always
/// there. The law follows the sentence rather than the encoding, so it is stated
/// here, against the rules themselves, instead of against a program the old
/// evaluator ran.
#[test]
fn every_partial_exact_time_operation_states_its_own_refusal() {
    let below = num_rational::Ratio::new(-1, 4);
    for (spelling, operation, arguments, expected) in [
        (
            "ratio_div",
            crate::phase::Builtin::RatioDiv,
            vec![exact(num_rational::Ratio::new(3, 4)), exact(num_rational::Ratio::ZERO)],
            "an exact rational is not divided by zero",
        ),
        (
            "duration_of",
            crate::phase::Builtin::DurationOf,
            vec![exact(below)],
            "a duration is nonnegative, and this exact rational is below zero",
        ),
        (
            "duration_scale",
            crate::phase::Builtin::DurationScale,
            vec![beat(num_rational::Ratio::new(1, 4)), exact(below)],
            "a duration is nonnegative, and this exact rational is below zero",
        ),
        (
            "position_between",
            crate::phase::Builtin::PositionBetween,
            vec![
                instant(num_rational::Ratio::new(2, 1)),
                instant(num_rational::Ratio::new(1, 1)),
            ],
            "the second position is before the first, and a duration is nonnegative",
        ),
    ] {
        let rule = rules::source(operation).expect("a registered arithmetic rule");
        let Some(Answer::Refused(ref because)) = rule(&arguments) else {
            panic!("`{spelling}` answered where the law expects a refusal");
        };
        assert_eq!(&**because, expected, "`{spelling}`");
    }
}

/// A `Ratio` argument.
fn exact(value: num_rational::Ratio<i64>) -> Datum {
    Datum::Lit(super::literal(super::plain_type("Ratio"), value))
}

/// A `Duration ⟨written⟩` argument.
fn beat(value: num_rational::Ratio<i64>) -> Datum {
    Datum::Lit(super::literal(
        super::tagged_type("Duration", crate::phase::Coordinate::WrittenTime),
        value,
    ))
}

/// A `Position ⟨written⟩` argument.
fn instant(value: num_rational::Ratio<i64>) -> Datum {
    Datum::Lit(super::literal(
        super::tagged_type("Position", crate::phase::Coordinate::WrittenTime),
        value,
    ))
}
