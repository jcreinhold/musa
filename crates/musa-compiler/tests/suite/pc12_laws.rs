//! What the chromatic quotient promises
//! (`docs/rules/language/03-musical-domains.md` §1 and §4).
//!
//! `Pc(12)` is `Z/12Z` and a spelled pitch class is not one. Only `χ` is total,
//! and it is not injective; the way back is a policy the author names, which
//! is why it takes a collection and may answer nothing. Every law here holds
//! that boundary at the language surface, where an author would meet it.
//!
//! # How a value becomes visible, and why that changed
//!
//! These laws used to count notes. A `nat` was sounded as that many overlaid
//! notes and a `list` as one note per member, because `Pc12` was a compiler
//! base type with a crate-private representation and nothing in source could
//! hold one, let alone compare two.
//!
//! Prompt 164 moved the domain into `stdlib/src/post_tonal/pcset.musa`, over
//! `std::cyclic`'s `Cycle(n)` rather than hardcoded to twelve, and the counting
//! apparatus went with it: a set's members are an ordinary `List<Nat>` now, and
//! `Equal` states what they are. So each law below is a `.musa` binding whose
//! *type* is the claim and whose value is the proof, and the test asserts that
//! the compiler accepted it. A wrong number is a type error, which is a
//! stronger statement than a note count and a shorter one to read.
//!
//! # Two laws asked at six
//!
//! The interval-class vector costs more reduction steps at a division of twelve
//! than `02-core-calculus.md` §4's budget allows today — prompt 164's Design
//! measures the charge and prompt 165 owns it — so it is asked at six, where
//! there are three interval classes rather than six and the law is the same
//! law. That the division is writable at all is the collapse this prompt made:
//! a modulus is an argument now.
//!
//! The exhaustive `T`/`I` reference and the brute-force prime-form model over
//! all 4096 sets stay in `musa_score::pc12`'s own unit tests, which is where
//! the two surviving builtins' Rust side lives.

// Test helpers use expect() on statically-valid inputs: a failure is a bug in
// the test itself, and panicking is the correct behavior there.
#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]

use musa_compiler::{CompileOptions, SourceDocument, compile};

use musa_score::{Code, Severity};

/// What every fixture below is written against: the division, and the readouts
/// that turn a pitch class or a set into the numbers the chapter writes.
const PRELUDE: &str = r"
    import std::cyclic;
    import std::indexed;
    import std::list;
    import std::post_tonal::pcset;

    meter 4/4;

    fn numbered(member: Pc(12)) -> Nat { class_number(12, member) }
    fn numbers(held: List(Pc(12))) -> List(Nat) { map(numbered, held) }
    fn chromatic_set(written: List(Nat)) -> PcSet(12) { pcset(12, chromatic, pcs(12, chromatic, written)) }
    fn found(cell: Option(NoteName)) -> Bool { cell.fold_from_end(false, fn (one, otherwise) { true }) }
";

/// A piece carrying `bindings` and sounding nothing in particular.
fn probe(bindings: &str) -> String {
    format!("piece \"Law\" {{\n{PRELUDE}\n{bindings}\n    score {{ part p {{ voice v {{ c4/1 }} }} }}\n}}\n")
}

fn errors_of(source: &str) -> Vec<(Code, String)> {
    compile(&SourceDocument::new(source, "test.musa"), &CompileOptions::default())
        .diagnostics()
        .iter()
        .filter(|diagnostic| diagnostic.severity == Severity::Error)
        .map(|diagnostic| (diagnostic.code, diagnostic.message.clone()))
        .collect()
}

/// The bindings elaborate, which is what makes each `Equal` a proof.
fn holds(bindings: &str) {
    let errors = errors_of(&probe(bindings));
    assert!(errors.is_empty(), "expected a clean compile, got {errors:?}");
}

/// The bindings are refused, with the code that says why.
fn refused(bindings: &str, expected: Code, what: &str) {
    let errors = errors_of(&probe(bindings));
    assert!(errors.iter().any(|(code, _)| *code == expected), "{what}: {errors:?}");
}

#[test]
fn a_spelled_pitch_class_is_not_an_unspelled_one() {
    refused(
        "    let wrong: Pc(12) = pitchclass_of(c4);",
        Code::ConversionMismatch,
        "a `pitchclass` must not stand where a `Pc(12)` belongs",
    );
    refused(
        "    let wrong: NoteName = pc(12, chromatic, 0);",
        Code::ConversionMismatch,
        "and a `Pc(12)` must not stand where a `pitchclass` belongs",
    );
}

#[test]
fn a_pitch_class_is_not_the_number_that_names_it() {
    refused(
        "    let wrong: Nat = pc(12, chromatic, 3);",
        Code::ConversionMismatch,
        "reading a residue as a number must be asked for",
    );
}

/// Declared equality is an ordinary namespace definition: the operator, the
/// method, and the qualified path are one term, with the division inferred
/// from the operands.
#[test]
fn a_pitch_class_set_uses_its_written_namespace_equality() {
    holds(
        "
    let left: PcSet(12) = chromatic_set([0, 3, 7]);
    let same: PcSet(12) = chromatic_set([7, 3, 0, 3]);
    let other: PcSet(12) = chromatic_set([0, 4, 7]);
    let operator: Equal(Bool, left == same, true) = Refl(true);
    let method: Equal(Bool, left.equal(same), true) = Refl(true);
    let qualified: Equal(Bool, PcSet::equal(left, same), true) = Refl(true);
    let distinct: Equal(Bool, left == other, false) = Refl(false);
",
    );
}

/// A declaration does not acquire equality merely by being finite. `Pc` has
/// no `equal` definition, so the ordinary method refusal is the whole answer.
#[test]
fn a_declared_type_without_written_equality_has_no_implicit_fallback() {
    refused(
        "    let invented: Bool = pc(12, chromatic, 0) == pc(12, chromatic, 0);",
        Code::NoMethodForType,
        "a declared type without `equal` must not gain one implicitly",
    );
}

#[test]
fn the_quotient_reduces_modulo_twelve() {
    holds(
        "
    let eleven: Equal(Nat, numbered(pc(12, chromatic, 11)), 11) = Refl(11);
    let thirteen: Equal(Nat, numbered(pc(12, chromatic, 13)), 1) = Refl(1);
    let twenty_five: Equal(Nat, numbered(pc(12, chromatic, 25)), 1) = Refl(1);
    let twelve: Equal(Nat, numbered(pc(12, chromatic, 12)), 0) = Refl(0);
",
    );
}

#[test]
fn forgetting_a_spelling_is_total_and_not_injective() {
    holds(
        "
    let sharp: Equal(Nat, numbered(forget_spelling(pitchclass_of(c#4))), 1) = Refl(1);
    let flat: Equal(Nat, numbered(forget_spelling(pitchclass_of(db4))), 1) = Refl(1);
    let sharpened: Equal(Nat, numbered(forget_spelling(pitchclass_of(b#3))), 0) = Refl(0);
    let doubly: Equal(Nat, numbered(forget_spelling(pitchclass_of(cbb4))), 10) = Refl(10);
",
    );
}

#[test]
fn a_spelling_needs_a_collection_and_may_not_exist_in_it() {
    holds(
        "
    let in_c: Equal(Bool, found(spelled_in(pc(12, chromatic, 1), scale c major)), false) =
        Refl(false);
    let in_d: Equal(Bool, found(spelled_in(pc(12, chromatic, 1), scale d major)), true) =
        Refl(true);
",
    );
}

#[test]
fn a_set_holds_a_repeated_member_once() {
    holds(
        "
    let triad: PcSet(12) = chromatic_set([0, 0, 4, 7, 7]);
    let held: Equal(List(Nat), numbers(set_members(12, chromatic, triad)), [0, 4, 7]) =
        Refl([0, 4, 7]);
",
    );
}

#[test]
fn a_set_reads_out_ascending_and_normal_order_need_not() {
    holds(
        "
    let set: PcSet(12) = chromatic_set([0, 5, 8]);
    let ascending: Equal(List(Nat), numbers(set_members(12, chromatic, set)), [0, 5, 8]) =
        Refl([0, 5, 8]);
    let normal: Equal(List(Nat), numbers(normal_order(12, chromatic, set)), [5, 8, 0]) =
        Refl([5, 8, 0]);
",
    );
}

#[test]
fn the_interval_class_vector_has_six_entries_and_counts_every_pair() {
    holds(
        "
    let sixfold: Cycle(6) = Positions(5);
    let triad: PcSet(6) = pcset(6, sixfold, pcs(6, sixfold, [0, 1, 3]));
    let width: Equal(Nat, length(interval_class_vector(6, sixfold, triad)), 3) = Refl(3);
    let counts: Equal(List(Nat), interval_class_vector(6, sixfold, triad), [1, 1, 1]) =
        Refl([1, 1, 1]);
",
    );
}

#[test]
fn a_set_class_survives_transposition_and_inversion() {
    holds(
        "
    let triad: PcSet(12) = chromatic_set([0, 4, 7]);
    let upright: Equal(Bool,
        prime_form(12, chromatic, triad) == chromatic_set([0, 3, 7]),
        true,
    ) = Refl(true);
",
    );
    holds(
        "
    let triad: PcSet(12) = chromatic_set([0, 4, 7]);
    let moved: PcSet(12) = set_transposed(12, chromatic, triad, 3);
    let same: Equal(Bool,
        prime_form(12, chromatic, moved) == prime_form(12, chromatic, triad),
        true,
    ) = Refl(true);
",
    );
    holds(
        "
    let triad: PcSet(12) = chromatic_set([0, 4, 7]);
    let mirrored: PcSet(12) = set_inverted(12, chromatic, triad, 0);
    let same: Equal(Bool,
        prime_form(12, chromatic, mirrored) == prime_form(12, chromatic, triad),
        true,
    ) = Refl(true);
",
    );
}

/// The post-tonal workload prompt 165 reserved until the replacement checker's
/// cost table was re-derived: the full T/I action at twelve, not the division-
/// six proxy the example used while 200,000 steps was the language wall.
#[test]
fn the_twelve_class_orbits_and_limited_transpositions_fit_the_language_budget() {
    holds(
        "
    let whole_tone: PcSet(12) = chromatic_set([0, 2, 4, 6, 8, 10]);
    let whole_tone_fixers: Equal(Nat, length(
        transposition_symmetries(12, chromatic, whole_tone),
    ), 6) = Refl(6);
",
    );
    holds(
        "
    let octatonic: PcSet(12) = chromatic_set([0, 1, 3, 4, 6, 7, 9, 10]);
    let octatonic_fixers: Equal(Nat, length(
        transposition_symmetries(12, chromatic, octatonic),
    ), 4) = Refl(4);
",
    );
    holds(
        "
    let generic_hexachord: PcSet(12) = chromatic_set([0, 1, 2, 4, 7, 8]);
    let full_orbit: Equal(Nat, length(
        set_class(12, chromatic, generic_hexachord),
    ), 24) = Refl(24);
",
    );
}

#[test]
fn the_bundled_libraries_import_like_any_other() {
    let errors = errors_of(&probe(""));
    assert!(
        errors.is_empty(),
        "`std::cyclic` and `std::post_tonal::pcset` must compile as bundled sources: {errors:?}"
    );
}
