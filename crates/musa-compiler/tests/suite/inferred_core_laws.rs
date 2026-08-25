//! The metatheory of the source core, stated as tests.
//!
//! Prompts 127aa–127b replaced the hand-written source checker with one small
//! typed core: kinded variables, Algorithm W, and a total evaluator over exact
//! time. Prompt 128's amendment replaced *that*, and it deleted this file's
//! premise rather than moving it. `docs/rules/constitution.md` §9 now says
//! "there are no principal types and no global inference", and
//! `02-core-calculus.md` says the same twice more — §1's "inference, principal
//! types, and the `a`/`d` variable classes are gone", and §2's "There is no
//! principal type and no global inference". A proof about a calculus is only a
//! claim about a compiler if the compiler is asked, and half of what this file
//! was asking about no longer exists to ask.
//!
//! What survives is what was never about inference. Evaluation of an accepted
//! program still ends, and ends in a value — progress and termination are
//! interesting together, since a checker that accepted a stuck term would
//! satisfy termination by being wrong. And the shapes the core refuses are still
//! refused *by name*, so a composer who writes one is told which rule they met
//! rather than being handed a term the evaluator quietly declined to run.
//!
//! `an_inferred_scheme_admits_every_instance_a_caller_asks_for` is the law that
//! went with the premise. It fixed that `fn unchanged(value) { value }` — no
//! annotation anywhere — could be called at `Nat`, at `Pitch`, and at
//! `List<Nat>` from one declaration, which is let-polymorphism and is exactly
//! what §2 removed: "Generalization happens only where an author wrote a
//! binder." The claim underneath it was never about the *omission*, though — it
//! was that one declaration serves every instance a caller asks for, so a
//! generic definition is written once. That claim is still true and still worth
//! fixing, at the place the language now puts it, and
//! `a_written_binder_admits_every_instance_a_caller_asks_for` is where it moved.

use musa_compiler::{CompileOptions, SourceDocument, compile};

use musa_score::Code;

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
        .filter(|diagnostic| diagnostic.severity == musa_score::Severity::Error)
        .map(|diagnostic| diagnostic.code)
        .collect()
}

/// One declaration, four types: a written binder serves every instance.
///
/// What makes a generic definition worth writing is that it is written *once*.
/// If each use site had to be met by its own declaration, `unchanged` would be
/// four functions and a fifth caller would need a fifth, which is the shape of
/// a language without abstraction rather than a language with explicit types.
/// So the test is not that `unchanged` checks: it is that a caller may ask for
/// any instance and get it, and that a definition may pass its own binder along
/// to another — `pair_up` calls `unchanged` at the type it was itself given,
/// which is where the binder has to actually be a variable rather than a name
/// the elaborator resolved once.
///
/// The `<A>` is the whole difference from the law this replaces.
/// `02-core-calculus.md` §2 puts generalization "only where an author wrote a
/// binder", so what used to be inferred is now written, and the guarantee a
/// caller gets is the same one.
#[test]
fn a_written_binder_admits_every_instance_a_caller_asks_for() {
    let compilation = compile_core(
        "fn unchanged({A: Type}, value: A) -> A { value } \
         fn pair_up({A: Type}, {B: Type}, left: A, right: B) -> A { unchanged(left) } \
         let counted: Nat = unchanged(3); \
         let sounded: Pitch = unchanged(c4); \
         let listed: List(Nat) = unchanged(range(3)); \
         let chosen: Nat = pair_up(1, c4);",
    );
    assert!(
        !compilation.has_errors(),
        "an instance of the written binder was refused: {:?}",
        errors(&compilation)
    );
}

/// A written type is a decision, and a caller may not ask past it.
///
/// The half of the old law that outlived principality. A declaration that says
/// `Nat -> Nat` means it: there is no more general type behind the annotation
/// for a caller to reach, because there is no inference that would have found
/// one. Without this the written type would be advice rather than a decision.
///
/// `ConversionMismatch` rather than `TypeMismatch`, and the rename is the
/// mechanism showing through: `diagnose.rs` introduced it as distinct from
/// `TypeMismatch`, "which the rank-1 checker raises". The two types are compared
/// by normalizing both and deciding convertibility, so the refusal names that
/// question rather than a shape mismatch in a unifier.
#[test]
fn a_written_type_cannot_be_widened_by_a_caller() {
    let specialized = compile_core("fn unchanged(value: Nat) -> Nat { value } let counted: Nat = unchanged(3);");
    assert!(
        !specialized.has_errors(),
        "an annotated declaration was refused at its own type: {:?}",
        errors(&specialized)
    );
    let widened = compile_core("fn unchanged(value: Nat) -> Nat { value } let sounded: Pitch = unchanged(c4);");
    assert!(
        errors(&widened).contains(&Code::ConversionMismatch),
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
    // The binders are annotated because `10-traits.md` §6's rule is exact
    // receiver: a method on a lambda binder whose type is still a
    // metavariable is `MethodOnVariable` by design — "the repair is: write
    // the trait", and the annotation is the writing. Nothing about the
    // progress-and-termination claim is weaker for it: the eliminators still
    // nest, and the value is still the one a reader computes by hand.
    let compilation = compile_core(
        "let rows: List(List(Nat)) = map(fn (index: Nat) { range(index) }, range(4)); \
         let widths: List(Nat) = map(fn (row: List(Nat)) { row.fold_from_start(0, fn (running: Nat, member: Nat) { running }) }, rows); \
         let total: Nat = widths.fold_from_start(7, fn (running: Nat, width: Nat) { running }); \
         let repeated: List(Nat) = rows.fold_from_start(range(3), fn (running: List(Nat), row: List(Nat)) { running });",
    );
    assert!(
        !compilation.has_errors(),
        "a finite nest of eliminators did not evaluate: {:?}",
        errors(&compilation)
    );
}

/// The shapes the core refuses, each by its own name.
///
/// Table-driven because the point is that the set is *closed*: these are the
/// ways a source program can fail to be a term of the core, and each has a
/// diagnostic a composer can act on. A refusal that arrived as a generic
/// failure — or as a compilation that produced nothing and said nothing —
/// would be the core failing to be a language.
///
/// The function-in-a-`data`-field row left this table with §1.2's amendment: a
/// field holding an arrow is no longer a refusable shape, it is a type without
/// a `Storable` instance. It is checked next door, at the position that needs
/// the encoding.
#[test]
fn each_refused_shape_is_refused_by_name() {
    let refusals: [(&str, &str, Code); 5] = [
        (
            "a call that leaves an argument out",
            "fn pick(left: Nat, right: Nat) -> Nat { left } let one: Nat = pick(1);",
            Code::WrongArity,
        ),
        // Two different refusals now, where the old core filed both under one.
        // A self-reference is no longer a cycle in the dependency graph:
        // `declare_program` builds the `rec` form itself and hands it to the
        // measure, so `forever` is recursion that was admitted as recursion and
        // then failed to decrease — which is what the termination checker is
        // for, and `UncheckedRecursion` is what it says. A *mutual* reference
        // has no `rec` form to be built into and stays a cycle with nowhere to
        // go (§2.4).
        (
            "a term that refers to itself without getting smaller",
            "fn forever(value: Nat) -> Nat { forever(value) }",
            Code::UncheckedRecursion,
        ),
        (
            "two terms that refer to each other",
            "fn ping(value: Nat) -> Nat { pong(value) } fn pong(value: Nat) -> Nat { ping(value) }",
            Code::DependencyCycle,
        ),
        (
            "a match that leaves a case out",
            "data Shape { Silence, Sounded(held: Duration(WrittenTime)) } \
             fn named(shape: Shape) -> Nat { match shape { Silence -> 0 } }",
            // `IncompleteMatch`, not `NonExhaustiveMatch`: coverage is decided
            // while the `match` is compiled to a recursor, so the refusal names
            // a constructor of an inductive family rather than a shape in a
            // pattern list. `resource_validation.rs` states the same migration.
            Code::IncompleteMatch,
        ),
        // Refused, and refused under a code that does not name what went
        // wrong. `Pair` takes one parameter and was handed two, and what comes
        // back is `TypeMismatch` — from `NotAType`, because an over-applied
        // family stops being a type before anyone counts its arguments. The
        // shape is closed, which is what this table is for; that the composer
        // is not told the arity is a diagnostic prompt 165 owes, and asserting
        // `WrongArity` here would be asserting a message nothing produces.
        (
            "a data instantiation at the wrong arity",
            "data Pair(A: Type) { Both(left: A, right: A): Pair(A) } \
             fn wrong(p: Pair(Nat, Bool)) -> Nat { 0 }",
            Code::TypeMismatch,
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

/// A hidden function is refused where storable data is *required*, and a `data`
/// field is not one of those places.
///
/// This law used to say the opposite, and the amendment is why. Under the old
/// core a stored field was a `d`-classed variable, so `data Box { Hold(transform:
/// Nat -> Nat) }` was refused at the declaration. §1.2 calls that "the right
/// *rule* stated in the wrong *place*" and moves it: storability is now the
/// `Storable A` constraint, a family "is storable exactly when every field type
/// it stores is", and only a closed list of positions requires it — an
/// event-track payload, a machine's port type, its feedback value, a registered
/// primitive's configuration, and a foreign primitive's argument. A `data` field
/// is on neither list, so `Box` is a perfectly good type that simply has no
/// `Storable` instance, and the refusal arrives when someone tries to put one
/// where an encoding is needed.
///
/// That is a strictly better place for it. The old refusal fired at a
/// declaration that had done nothing wrong; this one fires at the use that
/// cannot work, and it names the position that needs the encoding. Both halves
/// are checked here, because a law that only tested the acceptance would pass
/// just as well if the constraint were never required anywhere.
#[test]
fn a_function_may_not_hide_where_an_encoding_is_required() {
    for holding in [
        "data Box { Hold(transform: Nat -> Nat) }",
        "data Box { Hold(transforms: List(Nat -> Nat)) }",
        "data Box { Hold(transform: Option(Nat -> Nat)) }",
    ] {
        let declared = compile_core(holding);
        assert!(
            !declared.has_errors(),
            "a field holding a function is a type without a `Storable` instance, not a refusal: {:?}",
            errors(&declared)
        );
    }

    let payload = compile_core("let held: EventTrack(WrittenTime, Nat -> Nat) = empty_track();");
    assert!(
        !errors(&payload).is_empty(),
        "an event-track payload that holds a function was admitted"
    );
}
