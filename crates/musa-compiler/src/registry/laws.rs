//! What has to be true of the compiler's registrations.
//!
//! Unit tests rather than a file in `tests/suite/`, and that is forced rather
//! than chosen: the agreement law runs a rule and the corresponding arm of the
//! old evaluator on the same input, and [`crate::core`]'s `eval_builtin`,
//! `Expr`, and `Value` are private to it. A test outside this crate links
//! against `parse`/`compile`/`render_notation` and can reach none of them, and
//! widening the facade so it could would leave the crate's most doomed types
//! visible from everywhere. Prompt 142 moves whatever survives the cutover into
//! the suite, once there is a public path to it.

#![expect(
    clippy::expect_used,
    clippy::panic,
    reason = "a law that cannot fail loudly is not a law"
)]

use musa_core::{Answer, Datum, Raw, Refusal, Term};

use super::{bases, builtins, owned, rules};
use crate::core::{BUILTIN_OWNERSHIP, Family, PhaseFamily, SYNTAX_OWNERSHIP, Shape};

/// The whole context builds: declarations, base types, and every registration
/// check [`musa_core::Registry::new`] makes.
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
/// Fourteen builders and two traversals reach [`musa_core::Registry`];
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
            Term::lam(super::HERE, "context", Term::var(super::HERE, musa_core::Index(0))),
        ],
    );
    let asked = super::literal(text.clone(), "the context".to_owned()).term(super::HERE);
    let run = applied(
        Term::var(super::HERE, musa_core::Index(0)),
        [text.clone(), text.clone(), asked.clone(), sealed],
    );
    assert_eq!(
        musa_core::well_typed(&cx, &text, &run),
        Ok(()),
        "the application checks"
    );
    assert_eq!(
        musa_core::normalize(&cx, &text, &run).expect("it reduces"),
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

/// Each rule answers what the corresponding arm of the old evaluator answers.
///
/// This is the law that makes prompt 141e a translation rather than a rewrite,
/// and everything else in this file is scaffolding around it. The samples come
/// from the signatures themselves ([`crate::core::oracle`]), so a rule cannot be
/// checked at inputs chosen to suit it, and every δ row is required to be
/// *reached*: an operation whose arguments could not be sampled is reported by
/// name rather than skipped, because a law with a silent hole is the shape of a
/// translation nobody checked.
#[test]
fn each_rule_agrees_with_the_old_evaluator() {
    let mut unreached = Vec::new();
    for entry in &BUILTIN_OWNERSHIP {
        let Some(rule) = rules::source(entry.operation) else {
            continue;
        };
        let cases = crate::core::oracle::cases(entry.operation);
        if cases.is_empty() {
            unreached.push(entry.spelling);
            continue;
        }
        for case in cases {
            let answered = rule(&case.arguments);
            assert!(
                agrees(answered.as_ref(), case.answer.as_ref()),
                "`{}` disagrees with the old evaluator at {:?}: {answered:?} rather than {:?}",
                entry.spelling,
                case.arguments,
                case.answer
            );
        }
    }
    assert!(unreached.is_empty(), "no sample reached {unreached:?}");
}

/// Whether a rule and the old evaluator gave the same answer.
///
/// Equality everywhere except one outcome, and the exception is prompt 141m's
/// whole subject: where the old evaluator declined to answer, a rule now refuses
/// the program and says why. Those are one judgment written in two vocabularies,
/// so reading them as agreement is what keeps this law about the *translation*
/// rather than about the move.
///
/// The old vocabulary had two spellings for declining and the new one has one.
/// A `Result.Err` carrying a sentence is the declined answer an operation
/// *declared*; a bare absence is the one it did not — `ratio_div` at a zero
/// divisor is the standing example, total in its signature and partial in its
/// arm, which is the shape D2 exists to forbid. Both become
/// [`Answer::Refused`], and the second is why: a refusal carries the sentence
/// the absence never had a place to put.
fn agrees(answered: Option<&Answer>, old: Option<&Datum>) -> bool {
    match (answered, old) {
        (Some(Answer::Reduced(datum)), Some(other)) => datum == other,
        (Some(Answer::Refused(_)), Some(Datum::Case { constructor, .. })) => &**constructor == "Result.Err",
        (Some(Answer::Refused(_)) | None, None) => true,
        _ => false,
    }
}

/// Every sampled application is well typed at its signature and either reduces
/// and re-checks or refuses the program in words.
///
/// The agreement law compares two implementations to each other; this one
/// compares one to its *type*, and they fail on different mistakes. Two
/// implementations can agree perfectly on an answer neither of them may give —
/// an `Option.Some` where the signature says `Result`, a `List.Cons` whose
/// members are of the wrong domain, a `Nat` where a `Ratio` was declared — and
/// only the core can say so.
///
/// It asks in three steps, each of which can fail on its own: the application is
/// re-checked at the result type [`super::shape_type`] read out of the same
/// signature the builtin was registered with, so an argument of the wrong domain
/// is refused before the rule ever runs; [`musa_core::normalize`] fires the rule
/// and realizes what it answered against that declared result, which is where a
/// rule that answered the wrong shape becomes `MisfitAnswer`; and
/// [`musa_core::well_typed`] re-checks the normal form independently of the
/// evaluator that produced it.
///
/// Elaboration is deliberately not in that path. Nothing elaborates *to* these
/// builtins until prompt 142 wires the surface language to them, and going
/// through [`musa_core::infer`] here would be testing that unwritten path's
/// implicit insertion rather than this prompt's registrations.
#[test]
fn every_sampled_application_reduces_and_re_checks_or_states_its_refusal() {
    let cx = owned().expect("the compiler's own context builds");
    let mut checked = 0_usize;
    for entry in &BUILTIN_OWNERSHIP {
        let Family::Delta { arguments, result } = entry.family else {
            continue;
        };
        let head = named(&cx, entry.spelling);
        let ty = super::shape_type(&cx, result)
            .unwrap_or_else(|why| fail(entry.spelling, "has a result type the core cannot name", &why));
        for case in crate::core::oracle::cases(entry.operation) {
            // Where the old evaluator declined, the rule owes a *sentence*. This
            // is the second half of §5.8's D2, and it is stated here because the
            // old evaluator cannot state it: `eval_builtin` has two answers and
            // the rule has three, so absence there is the old vocabulary's only
            // spelling for both "the program is wrong" and "this table is". D2
            // forbids the second, and what tells them apart is whether the
            // registered rule refuses — `ratio_div` at a zero divisor and
            // `duration_of` below zero are the two the sample reaches.
            if case.answer.is_none() {
                let rule = rules::source(entry.operation)
                    .unwrap_or_else(|| fail(entry.spelling, "declined with no rule to say why", &"the core owns it"));
                let because = match rule(&case.arguments) {
                    Some(Answer::Refused(because)) => because,
                    Some(Answer::Reduced(_)) | None => fail(
                        entry.spelling,
                        "declined without declaring that it cannot answer",
                        &"D2",
                    ),
                };
                assert!(
                    !because.is_empty(),
                    "`{}` refused with nothing to say about the program",
                    entry.spelling
                );
                continue;
            }
            let applied = arguments
                .iter()
                .zip(&case.arguments)
                .fold(head.clone(), |function, (shape, argument)| {
                    Term::app(super::HERE, function, written(&cx, *shape, argument))
                });
            musa_core::well_typed(&cx, &ty, &applied)
                .unwrap_or_else(|why| fail(entry.spelling, "is not well typed at its own signature", &why));
            let normal = musa_core::normalize(&cx, &ty, &applied)
                .unwrap_or_else(|why| fail(entry.spelling, "does not reduce", &why));
            musa_core::well_typed(&cx, &ty, &normal)
                .unwrap_or_else(|why| fail(entry.spelling, "reduces to something its own type refuses", &why));
            checked = checked.checked_add(1).expect("the count fits");
        }
    }
    assert!(checked > 0, "no application was checked against its signature");
}

/// Why one sampled application did not survive the path its own signature
/// promises.
///
/// Generic in its answer so that it can stand where any of the steps would have
/// produced a value; every call diverges.
fn fail<T>(spelling: &str, what: &str, why: &dyn std::fmt::Display) -> T {
    panic!("`{spelling}` {what}: {why}")
}

/// One argument as the term a call would have built for it.
///
/// Type-directed, because the core is. A literal carries its own type and goes
/// in as one, but a constructor does not carry its family's parameters: `Cons`
/// cannot be applied without saying what it is a list of, and `Empty` says
/// nothing at all about what it is empty of. The signature already knows, which
/// is the same reason the core realizes a rule's answer against the declared
/// result rather than guessing it from the datum.
fn written(cx: &musa_core::Cx, shape: Shape, datum: &Datum) -> Term {
    match *datum {
        Datum::Lit(ref held) => held.term(super::HERE),
        // A count is closed and says which family it stands at, so it needs no
        // parameters read off the shape — the one datum arm that is as
        // self-describing as a literal.
        Datum::Count { ref family, count } => musa_core::infer(cx, &Raw::numeral(super::HERE, &**family, count))
            .map(|(term, _)| term)
            .unwrap_or_else(|why| fail(family, "is not a type a number can be written at", &why)),
        Datum::Case {
            ref constructor,
            ref fields,
        } => {
            let (parameters, shapes) = surrounding(cx, shape, constructor);
            let head = crate::prelude::constant(cx, constructor)
                .unwrap_or_else(|why| fail(constructor, "is not a declared constructor", &why));
            let applied = parameters
                .into_iter()
                .fold(head, |function, parameter| Term::app(super::HERE, function, parameter));
            shapes
                .into_iter()
                .zip(fields)
                .fold(applied, |function, (shape, field)| {
                    Term::app(super::HERE, function, written(cx, shape, field))
                })
        }
    }
}

/// A family's parameters, and the shapes of one constructor's own fields.
///
/// Both are read off the shape rather than off the datum, because the datum does
/// not carry them: constructor fields are positional and a family's parameters
/// are not fields at all. Every recursion here is into a shape the argument's
/// own signature already named.
fn surrounding(cx: &musa_core::Cx, shape: Shape, constructor: &str) -> (Vec<Term>, Vec<Shape>) {
    let ty = |member: Shape| {
        super::shape_type(cx, member).unwrap_or_else(|why| fail(constructor, "holds a type the core cannot name", &why))
    };
    match shape {
        // `Bool` and `Nat` are the two base shapes whose values are
        // constructors. Neither family takes a parameter, and a `Nat`'s
        // predecessor is a `Nat`, so the shape recurses into itself.
        Shape::Base(_) => (
            Vec::new(),
            if constructor == "Nat.Succ" {
                vec![shape]
            } else {
                Vec::new()
            },
        ),
        Shape::Option(member) => (
            vec![ty(*member)],
            if constructor == "Option.Some" {
                vec![*member]
            } else {
                Vec::new()
            },
        ),
        // The tail of a list is a list of the same thing, which is why one shape
        // describes both fields of `Cons`.
        Shape::List(member) => (
            vec![ty(*member)],
            if constructor == "List.Cons" {
                vec![*member, shape]
            } else {
                Vec::new()
            },
        ),
        Shape::Result(value, error) => (
            vec![ty(*value), ty(*error)],
            vec![if constructor == "Result.Err" { *error } else { *value }],
        ),
        Shape::Fault => (Vec::new(), vec![crate::core::NATS, crate::core::PC12S]),
    }
}

/// `name` resolves in `cx`, which is what "registered" means to a program, and
/// the term it resolves to.
fn named(cx: &musa_core::Cx, name: &str) -> Term {
    let (term, _) = musa_core::infer(cx, &musa_core::Raw::var(super::HERE, name)).unwrap_or_else(|refusal| {
        let unknown = matches!(refusal, musa_core::ElabError::Refused(Refusal::UnknownName { .. }));
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
pub(super) fn resolves(cx: &musa_core::Cx, name: &str) {
    if let Err(refusal) = musa_core::infer(cx, &musa_core::Raw::var(super::HERE, name)) {
        assert!(
            matches!(refusal, musa_core::ElabError::Refused(Refusal::Unsolved { .. })),
            "`{name}` is not registered: {refusal}"
        );
    }
}

/// A `Datum` is compared by value, which is what the agreement law relies on.
///
/// Stated because it is the one assumption that law makes and the one that would
/// fail silently: if [`musa_core::Payload::same`] answered `false` for equal
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
            crate::core::Builtin::RatioDiv,
            vec![exact(num_rational::Ratio::new(3, 4)), exact(num_rational::Ratio::ZERO)],
            "an exact rational is not divided by zero",
        ),
        (
            "duration_of",
            crate::core::Builtin::DurationOf,
            vec![exact(below)],
            "a duration is nonnegative, and this exact rational is below zero",
        ),
        (
            "duration_scale",
            crate::core::Builtin::DurationScale,
            vec![beat(num_rational::Ratio::new(1, 4)), exact(below)],
            "a duration is nonnegative, and this exact rational is below zero",
        ),
        (
            "position_between",
            crate::core::Builtin::PositionBetween,
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
        super::tagged_type("Duration", crate::core::Coordinate::WrittenTime),
        value,
    ))
}

/// A `Position ⟨written⟩` argument.
fn instant(value: num_rational::Ratio<i64>) -> Datum {
    Datum::Lit(super::literal(
        super::tagged_type("Position", crate::core::Coordinate::WrittenTime),
        value,
    ))
}
