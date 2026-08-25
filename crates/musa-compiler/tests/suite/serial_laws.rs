//! What a twelve-tone row promises
//! (`docs/rules/language/03-musical-domains.md` §5).
//!
//! A row is a bijection from twelve order positions onto twelve pitch
//! classes, and `row` is the only way in. Because that invariant is checked
//! once, every operation on a row is total; because it is checked, a sequence
//! that is not a row is refused — and the refusal carries both exact reasons,
//! the repeated position and the missing class, so a caller holding it never
//! has to ask the question again to learn why.
//!
//! The four labels P, I, R, and RI are the 24 affine pitch-class operations
//! times one reversal of order positions, so there are 48 labelled forms and
//! not one 48-element group.
//!
//! # What moved, and what moved out of this file
//!
//! Prompt 164 put the domain in `stdlib/src/post_tonal/serial.musa`, over
//! `std::cyclic`'s `Cycle(n)`: `ToneRow(n)` rather than `Row12`, and the
//! division reaching every operation as the number it is. A row's numbers are
//! an ordinary `List<Nat>` now, so each law below is a `.musa` binding whose
//! *type* is the claim and whose value is the proof — a wrong number is a type
//! error rather than a note count that came out different.
//!
//! **Two laws left with the fixture.** How many distinct rows the 48 labels
//! produce, and that every row stands in a twelve-by-twelve matrix, are the
//! `4n` group computations: `row_forms`, `distinct_forms`, `row_symmetries`,
//! and `matrix`. Written in Musa they cost between three and fifteen times
//! `02-core-calculus.md` §4's reduction-step budget — prompt 164's Design
//! measures the charge, which is the checker's and not the library's — and
//! prompt 165's Target names those four facts as its own. The functions are in
//! the standard library and are exercised there when the budget reaches them.

// Test helpers use expect() on statically-valid inputs: a failure is a bug in
// the test itself, and panicking is the correct behavior there.
#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]

use musa_compiler::{CompileOptions, SourceDocument, compile};

use musa_score::{Code, Severity};

const SERIAL_FORMS: &str = include_str!("../../../../examples/serial-forms.musa");

/// The rows every fixture below is written against — one generic, one the
/// chromatic ascent, one sequence that is not a row at all — and the readouts
/// that make a row's answer an ordinary value.
///
/// `twelve` rather than `chromatic` for the local name of the row that ascends
/// by semitone: `chromatic` is `std::cyclic`'s `Cycle(12)`, the division every
/// call below hands over, and shadowing it here would make the fixtures read as
/// if the division were a row.
const PRELUDE: &str = r"
    import std::cyclic;
    import std::indexed;
    import std::list;
    import std::post_tonal::serial;

    meter 4/4;

    let generic_numbers: List<Nat> = [0, 1, 4, 9, 5, 8, 3, 10, 2, 11, 6, 7];
    let twelve_numbers: List<Nat> = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11];
    let flawed_numbers: List<Nat> = [0, 1, 2, 0, 4, 5, 6, 7, 8, 9, 10, 3];

    let generic: Result<ToneRow(12), RowFault> = row(12, chromatic, generic_numbers);
    let twelve: Result<ToneRow(12), RowFault> = row(12, chromatic, twelve_numbers);

    fn is_row(built: Result<ToneRow(12), RowFault>) -> Bool {
        match built { Ok(series) -> true, Err(reason) -> false }
    }
    fn numbers_of(built: Result<ToneRow(12), RowFault>, operation: ToneRow(12) -> ToneRow(12)) -> List<Nat> {
        match built { Ok(series) -> row_numbers(12, operation(series)), Err(reason) -> [] }
    }
    fn asked(built: Result<ToneRow(12), RowFault>, question: ToneRow(12) -> Nat) -> Nat {
        match built { Ok(series) -> question(series), Err(reason) -> 0 }
    }
    fn itself(series: ToneRow(12)) -> ToneRow(12) { series }
    fn spelled_count(cells: List<Option<NoteName>>) -> Nat {
        length(filter(fn (cell: Option<NoteName>) -> Bool {
            cell.fold_from_end(false, fn (one, otherwise) { true })
        }, cells))
    }
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

#[test]
fn only_a_bijection_is_a_row() {
    holds(
        "
    let generic_admitted: Equal<Bool>(is_row(generic), true) = Refl(true);
    let ascent_admitted: Equal<Bool>(is_row(twelve), true) = Refl(true);
    let repeated_refused: Equal<Bool>(is_row(row(12, chromatic, flawed_numbers)), false) =
        Refl(false);
    let short_refused: Equal<Bool>(is_row(row(12, chromatic, [0, 1, 2])), false) = Refl(false);
    let long_refused: Equal<Bool>(
        is_row(row(12, chromatic, [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 0])),
        false,
    ) = Refl(false);
    let residue_refused: Equal<Bool>(
        is_row(row(12, chromatic, [0, 12, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11])),
        false,
    ) = Refl(false);
",
    );
}

#[test]
fn a_refusal_says_which_position_repeated_and_which_class_never_came() {
    holds(
        "
    let repeats: Equal<List<Nat>>(repeated_positions(flawed_numbers), [3]) = Refl([3]);
    let missing: Equal<List<Nat>>(missing_classes(12, chromatic, flawed_numbers), [11]) =
        Refl([11]);
    let row_repeats_nothing: Equal<List<Nat>>(repeated_positions(generic_numbers), []) = Refl([]);
    let row_misses_nothing: Equal<List<Nat>>(missing_classes(12, chromatic, generic_numbers), []) =
        Refl([]);
",
    );
}

#[test]
fn the_refusal_itself_carries_both_reasons() {
    // The same two facts as the test above, but read out of the value `row`
    // returned rather than asked of the sequence a second time. That is what
    // the sum buys: the reason travels with the failure.
    holds(
        "
    let flawed: Result<ToneRow(12), RowFault> = row(12, chromatic, flawed_numbers);
    fn carried(built: Result<ToneRow(12), RowFault>, read: RowFault -> List<Nat>) -> List<Nat> {
        match built { Ok(series) -> [], Err(reason) -> read(reason) }
    }
    fn repeats_of(reason: RowFault) -> List<Nat> {
        match reason { Fault(repeats, missing) -> repeats }
    }
    fn missing_of(reason: RowFault) -> List<Nat> {
        match reason { Fault(repeats, missing) -> missing }
    }
    let carried_repeats: Equal<List<Nat>>(carried(flawed, repeats_of), [3]) = Refl([3]);
    let carried_missing: Equal<List<Nat>>(carried(flawed, missing_of), [11]) = Refl([11]);
    let intact: Equal<Bool>(is_row(generic), true) = Refl(true);
",
    );
}

#[test]
fn a_row_has_twelve_order_positions() {
    holds(
        "
    let positions: Equal<Nat>(length(numbers_of(generic, itself)), 12) = Refl(12);
    let held: Equal<List<Nat>>(
        numbers_of(generic, itself),
        [0, 1, 4, 9, 5, 8, 3, 10, 2, 11, 6, 7],
    ) = Refl([0, 1, 4, 9, 5, 8, 3, 10, 2, 11, 6, 7]);
",
    );
}

#[test]
fn a_tone_row_uses_its_written_namespace_equality() {
    holds(
        "
    fn rows_equal(
        left: Result<ToneRow(12), RowFault>,
        right: Result<ToneRow(12), RowFault>,
    ) -> Bool {
        match left {
            Ok(one) -> match right { Ok(other) -> one == other, Err(reason) -> false },
            Err(reason) -> false,
        }
    }
    let same: Equal<Bool>(rows_equal(generic, generic), true) = Refl(true);
    let different: Equal<Bool>(rows_equal(generic, twelve), false) = Refl(false);
",
    );
}

#[test]
fn each_label_is_one_operation_on_the_row() {
    holds(
        "
    let prime_three: Equal<List<Nat>>(
        numbers_of(generic, fn (series: ToneRow(12)) -> ToneRow(12) {
            row_transposed(12, chromatic, series, 3)
        }),
        [3, 4, 7, 0, 8, 11, 6, 1, 5, 2, 9, 10],
    ) = Refl([3, 4, 7, 0, 8, 11, 6, 1, 5, 2, 9, 10]);
",
    );
    holds(
        "
    let inversion_zero: Equal<List<Nat>>(
        numbers_of(generic, fn (series: ToneRow(12)) -> ToneRow(12) {
            row_inverted(12, chromatic, series, 0)
        }),
        [0, 11, 8, 3, 7, 4, 9, 2, 10, 1, 6, 5],
    ) = Refl([0, 11, 8, 3, 7, 4, 9, 2, 10, 1, 6, 5]);
",
    );
    holds(
        "
    let retrograde_form: Equal<List<Nat>>(
        numbers_of(generic, fn (series: ToneRow(12)) -> ToneRow(12) {
            row_retrograde(12, chromatic, series)
        }),
        [7, 6, 11, 2, 10, 3, 8, 5, 9, 4, 1, 0],
    ) = Refl([7, 6, 11, 2, 10, 3, 8, 5, 9, 4, 1, 0]);
",
    );
    holds(
        "
    let retrograde_inversion: Equal<List<Nat>>(
        numbers_of(generic, fn (series: ToneRow(12)) -> ToneRow(12) {
            row_retrograde_inversion(12, chromatic, series, 0)
        }),
        [5, 6, 1, 10, 2, 9, 4, 7, 3, 8, 11, 0],
    ) = Refl([5, 6, 1, 10, 2, 9, 4, 7, 3, 8, 11, 0]);
",
    );
}

#[test]
fn a_form_is_numbered_only_once_a_convention_is_named() {
    holds(
        "
    fn up_three(series: ToneRow(12)) -> ToneRow(12) { row_transposed(12, chromatic, series, 3) }
    let fixed_as_written: Equal<Nat>(
        asked(generic, fn (series: ToneRow(12)) -> Nat { fixed_zero_index(12, series) }),
        0,
    ) = Refl(0);
    let fixed_when_moved: Equal<Nat>(
        asked(generic, fn (series: ToneRow(12)) -> Nat { fixed_zero_index(12, up_three(series)) }),
        3,
    ) = Refl(3);
",
    );
    holds(
        "
    fn up_three(series: ToneRow(12)) -> ToneRow(12) { row_transposed(12, chromatic, series, 3) }
    let moveable_as_written: Equal<Nat>(
        asked(generic, fn (series: ToneRow(12)) -> Nat {
            moveable_zero_index(12, chromatic, series, series)
        }),
        0,
    ) = Refl(0);
",
    );
    holds(
        "
    fn up_three(series: ToneRow(12)) -> ToneRow(12) { row_transposed(12, chromatic, series, 3) }
    let moveable_when_moved: Equal<Nat>(
        asked(generic, fn (series: ToneRow(12)) -> Nat {
            moveable_zero_index(12, chromatic, series, up_three(series))
        }),
        3,
    ) = Refl(3);
",
    );
}

#[test]
fn spelling_a_row_loses_the_notes_the_collection_cannot_write() {
    holds(
        "
    let in_c_major: Equal<Nat>(
        asked(generic, fn (series: ToneRow(12)) -> Nat {
            spelled_count(row_spelled_in(series, scale c major))
        }),
        7,
    ) = Refl(7);
",
    );
    holds(
        "
    let octatonically: Equal<Nat>(
        asked(generic, fn (series: ToneRow(12)) -> Nat {
            spelled_count(row_spelled_in(series, scale c octatonic_half_whole))
        }),
        8,
    ) = Refl(8);
",
    );
    holds(
        "
    let the_row_is_intact: Equal<Nat>(length(numbers_of(generic, itself)), 12) = Refl(12);
",
    );
}

/// The `4n` row computations at twelve, deferred from prompt 164 until prompt
/// 165 re-derived the replacement checker's cost table.
#[test]
fn a_row_has_four_n_labelled_operations_at_twelve() {
    holds(
        "
    let labelled: Equal<Nat>(length(row_operations(12, chromatic)), 48) = Refl(48);
",
    );
}

#[test]
fn a_generic_row_has_four_n_distinct_forms_at_twelve() {
    holds(
        "
    fn orbit_size(built: Result<ToneRow(12), RowFault>) -> Nat {
        match built { Ok(series) -> length(row_forms(12, chromatic, series)), Err(reason) -> 0 }
    }
    let forms: Equal<Nat>(orbit_size(generic), 48) = Refl(48);
",
    );
}

#[test]
fn a_generic_rows_stabilizer_is_the_identity_at_twelve() {
    holds(
        "
    fn symmetry_count(built: Result<ToneRow(12), RowFault>) -> Nat {
        match built { Ok(series) -> length(row_symmetries(12, chromatic, series)), Err(reason) -> 0 }
    }
    let symmetries: Equal<Nat>(symmetry_count(generic), 1) = Refl(1);
",
    );
}

#[test]
fn a_twelve_tone_rows_matrix_has_twelve_rows() {
    holds(
        "
    fn matrix_size(built: Result<ToneRow(12), RowFault>) -> Nat {
        match built { Ok(series) -> length(matrix(12, chromatic, series)), Err(reason) -> 0 }
    }
    let rows: Equal<Nat>(matrix_size(generic), 12) = Refl(12);
",
    );
}

#[test]
fn the_bundled_example_compiles() {
    let errors = errors_of(SERIAL_FORMS);
    assert!(
        errors.is_empty(),
        "`examples/serial-forms.musa` must compile: {errors:?}"
    );
}
