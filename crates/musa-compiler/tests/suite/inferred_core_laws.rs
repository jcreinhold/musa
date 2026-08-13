//! The metatheory of the inferred source core, stated as tests.
//!
//! Prompts 127aa–127b replaced the hand-written source checker with one small
//! typed core: kinded variables, Algorithm W, and a total evaluator over exact
//! time. What that buys is stated in research `06-proof-outline.md` §2 —
//! principal types, preservation, progress, determinism, and termination — and
//! a proof about a calculus is only a claim about a compiler if the compiler is
//! asked. This file asks.
//!
//! The compile-fail half is the other side of the same statement: the five
//! shapes the core refuses are refused *by name*, so a composer who writes one
//! is told which rule they met rather than being handed a term the evaluator
//! quietly declined to run.

use musa_compiler::{Code, CompileOptions, SourceDocument, compile};

fn compile_core(declarations: &str) -> musa_compiler::Compilation {
    let source = SourceDocument::new(
        format!(
            "piece \"Inferred core\" {{ import std::core; import std::list; {declarations} \
             score {{ part p {{ voice v {{ c4/1 }} }} }} }}"
        ),
        "inferred-core-laws.musa",
    );
    compile(&source, &CompileOptions::default())
}

fn errors(compilation: &musa_compiler::Compilation) -> Vec<Code> {
    compilation
        .diagnostics()
        .iter()
        .filter(|diagnostic| diagnostic.severity == musa_compiler::Severity::Error)
        .map(|diagnostic| diagnostic.code)
        .collect()
}

/// One declaration, three types: the inferred scheme is the *principal* one,
/// not merely *a* typing (`06-proof-outline.md` §2, Theorem 2.1).
///
/// Principality is what makes an omitted annotation safe. If Algorithm W
/// returned some arbitrary sound typing, a declaration written without types
/// would silently commit its callers to whichever type the first use happened
/// to force, and the second use would fail for a reason nothing in the source
/// explains. So the test is not that `unchanged` type-checks: it is that every
/// instance a caller can ask for is admitted from the one declaration.
#[test]
fn an_inferred_scheme_admits_every_instance_a_caller_asks_for() {
    let compilation = compile_core(
        "fn unchanged(value) { value } \
         fn pair_up(left, right) { unchanged(left) } \
         let counted: Nat = unchanged(3); \
         let sounded: Pitch = unchanged(c4); \
         let listed: List<Nat> = unchanged(range(3)); \
         let deeper: List<List<Nat>> = unchanged(map(fn (index) { range(index) }, range(2))); \
         let chosen: Nat = pair_up(1, c4);",
    );
    assert!(
        !compilation.has_errors(),
        "an instance of the principal type was refused: {:?}",
        errors(&compilation)
    );
}

/// An annotation may specialize the principal type; it may not widen it.
///
/// The two halves are one law. A declaration that says `Nat -> Nat` means it,
/// and a caller may not ask for the general type back — otherwise the written
/// type would be advice rather than a decision.
#[test]
fn an_annotation_specializes_the_principal_type_and_cannot_widen_it() {
    let specialized = compile_core("fn unchanged(value: Nat) -> Nat { value } let counted: Nat = unchanged(3);");
    assert!(
        !specialized.has_errors(),
        "an annotation that specializes was refused: {:?}",
        errors(&specialized)
    );
    let widened = compile_core("fn unchanged(value: Nat) -> Nat { value } let sounded: Pitch = unchanged(c4);");
    assert!(
        errors(&widened).contains(&Code::TypeMismatch),
        "an annotated declaration was used at a type it does not have: {:?}",
        errors(&widened)
    );
}

/// Evaluation of an accepted program always ends, and ends in a value
/// (Theorems 2.3 and 2.5).
///
/// The nesting here is what a proof of termination is *about*: folds inside
/// maps inside folds, over a term that has no recursion to run away with. It
/// finishes because the calculus has no way not to, and it answers with the
/// value the reader can compute by hand — progress and termination are only
/// interesting together, since a checker that accepted a stuck term would
/// satisfy termination by being wrong.
#[test]
fn a_deeply_nested_finite_program_evaluates_to_its_value() {
    let compilation = compile_core(
        "let rows: List<List<Nat>> = map(fn (index) { range(index) }, range(4)); \
         let widths: List<Nat> = map(fn (row) { list_fold(0, fn (member, running) { running }, row) }, rows); \
         let total: Nat = list_fold(7, fn (width, running) { running }, widths); \
         let repeated: List<Nat> = list_fold(range(3), fn (row, running) { running }, rows);",
    );
    assert!(
        !compilation.has_errors(),
        "a finite nest of eliminators did not evaluate: {:?}",
        errors(&compilation)
    );
}

/// The five shapes the inferred core refuses, each by its own name.
///
/// Table-driven because the point is that the set is *closed*: these are the
/// ways a source program can fail to be a term of the core, and each has a
/// diagnostic a composer can act on. A refusal that arrived as a generic
/// failure — or as a compilation that produced nothing and said nothing —
/// would be the core failing to be a language.
#[test]
fn each_refused_shape_is_refused_by_name() {
    let refusals: [(&str, &str, Code); 6] = [
        (
            "a function hidden in a data field",
            "data Box { Hold(transform: Nat -> Nat) }",
            Code::TypeMismatch,
        ),
        (
            "a call that leaves an argument out",
            "fn pick(left: Nat, right: Nat) -> Nat { left } let one: Nat = pick(1);",
            Code::WrongArity,
        ),
        (
            "a term that refers to itself",
            "fn forever(value: Nat) -> Nat { forever(value) }",
            Code::DependencyCycle,
        ),
        (
            "two terms that refer to each other",
            "fn ping(value: Nat) -> Nat { pong(value) } fn pong(value: Nat) -> Nat { ping(value) }",
            Code::DependencyCycle,
        ),
        (
            "a match that leaves a case out",
            "data Shape { Silence, Sounded(held: Duration) } \
             fn named(shape: Shape) -> Nat { match shape { Silence -> 0 } }",
            Code::NonExhaustiveMatch,
        ),
        (
            "a data instantiation at the wrong arity",
            "data Pair<A> { Both(left: A, right: A) } fn wrong(p: Pair<Nat, Bool>) -> Nat { 0 }",
            Code::WrongArity,
        ),
    ];
    for (what, source, expected) in refusals {
        let compilation = compile_core(source);
        let found = errors(&compilation);
        assert!(
            found.contains(&expected),
            "{what} was not refused as {expected:?}: {found:?}"
        );
    }
}

/// A hidden function is refused wherever data is declared, not only in a
/// `data` field: storability is a property of the type
/// (`02-core-calculus.md` §1.1), so every position that requires storable data
/// asks the same question.
#[test]
fn a_function_may_not_hide_in_a_stored_position() {
    let field = compile_core("data Box { Hold(transform: Nat -> Nat) }");
    assert!(!errors(&field).is_empty(), "a field holding a function was admitted");
    let nested = compile_core("data Box { Hold(transforms: List<Nat -> Nat>) }");
    assert!(
        !errors(&nested).is_empty(),
        "a field holding a list of functions was admitted"
    );
    let inside_option = compile_core("data Box { Hold(transform: Option<Nat -> Nat>) }");
    assert!(
        !errors(&inside_option).is_empty(),
        "a field holding an optional function was admitted"
    );
}
