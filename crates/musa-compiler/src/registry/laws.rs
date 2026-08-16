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

use musa_core::{Datum, Refusal, Term};

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
        rules::REGISTERED,
        "the registered count and the count this module states have drifted"
    );
    for builtin in &registered {
        named(&cx, builtin.name());
    }
}

/// The rows with no rule are exactly the four families that are said to have
/// none, counted from the tables rather than from a comment.
///
/// The point of counting both ways is that neither number is trusted: the tables
/// say how many rows each family has, [`rules::UNREGISTERED`] says how many are
/// expected to be left, and 141e's Design says which families those are. A row
/// that quietly changed family would move one of the three and not the others.
#[test]
fn the_unregistered_rows_are_the_families_they_are_said_to_be() {
    let eliminators = BUILTIN_OWNERSHIP
        .iter()
        .filter(|entry| matches!(entry.family, Family::Eliminator(_)))
        .count();
    let tracks = BUILTIN_OWNERSHIP
        .iter()
        .filter(|entry| matches!(entry.family, Family::Track))
        .count();
    let machines = BUILTIN_OWNERSHIP
        .iter()
        .filter(|entry| matches!(entry.family, Family::Machine(_)))
        .count();
    let traversals = SYNTAX_OWNERSHIP
        .iter()
        .filter(|entry| matches!(entry.family, PhaseFamily::Fold))
        .count();
    let counted = [
        ("structural eliminators", eliminators),
        ("track builtins", tracks),
        ("machine builtins", machines),
        ("phase traversals", traversals),
    ];
    assert_eq!(counted, rules::UNREGISTERED, "a family's row count moved");

    let left = BUILTIN_OWNERSHIP.len() + SYNTAX_OWNERSHIP.len() - rules::REGISTERED;
    assert_eq!(
        left,
        counted.iter().map(|&(_, count)| count).sum::<usize>(),
        "the rows left over are not the four families they are said to be"
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
            assert_eq!(
                rule(&case.arguments),
                case.answer,
                "`{}` disagrees with the old evaluator at {:?}",
                entry.spelling,
                case.arguments
            );
        }
    }
    assert!(unreached.is_empty(), "no sample reached {unreached:?}");
}

/// Every sampled application is well typed at its signature, reduces, and
/// re-checks.
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
fn every_sampled_application_reduces_and_re_checks() {
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
            // A rule that declines is making no claim about its result type, and
            // the agreement law has already checked that it declines exactly
            // where the old evaluator did. 141e's Design names the rows this
            // reaches — an interval sum with no name, a degree stepped off its
            // scale — and defers them to prompt 143.
            if case.answer.is_none() {
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
        panic!("`{name}` is not registered (unknown name: {unknown})");
    });
    term
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
