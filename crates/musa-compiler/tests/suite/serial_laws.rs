//! What a twelve-tone row promises
//! (`docs/rules/language/03-musical-domains.md` §5).
//!
//! A row is a bijection from twelve order positions onto twelve pitch
//! classes, and `row` is the only way in. Because that invariant is checked
//! once, every operation on a row is total; because it is checked, a sequence
//! that is not a row is refused — and the refusal carries both exact reasons,
//! the repeated pitch class and the missing one, so a caller holding it never
//! has to ask the question again to learn why.
//!
//! The four labels P, I, R, and RI are the 24 affine pitch-class operations
//! times one reversal of order positions, so there are 48 labelled forms and
//! not one 48-element group. How many *distinct* rows those labels produce is
//! a fact about the row, which is why it is counted here rather than assumed.
//!
//! Values become visible the way they do in `pc12_laws`: a `nat` sounds as
//! that many overlaid notes and a `list` as one note per member. Row equality
//! and the group laws behind these counts are checked where they are
//! computed, in `musa_score::pc12`'s own unit tests, because nothing outside the
//! crate can hold a `Row12`.

// Test helpers use expect() on statically-valid inputs: a failure is a bug in
// the test itself, and panicking is the correct behavior there.
#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]

use musa_compiler::{CompileOptions, SourceDocument, compile};

use musa_score::{Code, ScoreEventKind, ScoreSnapshot, Severity};

const SERIAL_FORMS: &str = include_str!("../../../../examples/serial-forms.musa");

/// The counting apparatus and the rows every fixture below is written
/// against: one generic row, one the chromatic ascent, one sequence that is
/// not a row at all.
const PRELUDE: &str = r"
    import std::list;
    import std::post_tonal::pcset;
    import std::post_tonal::serial;

    meter 4/4;

    fn tick(one: EventTrack<WrittenTime>, carried: EventTrack<WrittenTime>) -> EventTrack<WrittenTime> { together(one, carried) }
    fn beat() -> EventTrack<WrittenTime> { music { c4/1 } }
    fn tally(count: Nat) -> EventTrack<WrittenTime> { repeated(beat(), count).fold_from_start(music { rest/1 }, fn (carried, one) { tick(one, carried) }) }
    fn beat_for_pc(member: Pc12) -> EventTrack<WrittenTime> { beat() }
    fn beat_for_nat(count: Nat) -> EventTrack<WrittenTime> { beat() }
    fn beat_for_row(series: Row12) -> EventTrack<WrittenTime> { beat() }
    fn beat_for_spelling(spelled: NoteName) -> EventTrack<WrittenTime> { beat() }
    fn chorus(voices: List<EventTrack<WrittenTime>>) -> EventTrack<WrittenTime> { voices.fold_from_start(music { rest/1 }, fn (carried, one) { tick(one, carried) }) }
    fn sounded(cell: Option<NoteName>) -> EventTrack<WrittenTime> { cell.fold_from_end(music { rest/1 }, fn (found, otherwise) { beat_for_spelling(found) }) }

    let generic_pcs: List<Pc12> = pcs([0, 1, 4, 9, 5, 8, 3, 10, 2, 11, 6, 7]);
    let chromatic_pcs: List<Pc12> = pcs([0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11]);
    let flawed_pcs: List<Pc12> = pcs([0, 1, 2, 0, 4, 5, 6, 7, 8, 9, 10, 3]);

    let generic: Result<Row12, RowFault> = row(generic_pcs);
    let chromatic: Result<Row12, RowFault> = row(chromatic_pcs);
";

/// A piece whose one voice sounds `expression`.
fn probe(bindings: &str, expression: &str) -> String {
    format!(
        "piece \"Law\" {{\n{PRELUDE}\n{bindings}\n    score {{ part p {{ voice v {{ use {expression}; }} }} }}\n}}\n"
    )
}

fn compile_named(source: &str) -> musa_compiler::Compilation {
    compile(&SourceDocument::new(source, "test.musa"), &CompileOptions::default())
}

fn errors_of(source: &str) -> Vec<(Code, String)> {
    compile_named(source)
        .diagnostics()
        .iter()
        .filter(|diagnostic| diagnostic.severity == Severity::Error)
        .map(|diagnostic| (diagnostic.code, diagnostic.message.clone()))
        .collect()
}

fn snapshot_of(source: &str) -> ScoreSnapshot {
    let errors = errors_of(source);
    assert!(errors.is_empty(), "expected a clean compile, got {errors:?}");
    compile_named(source).into_snapshot().expect("compiles")
}

/// How many notes the probe's voice sounds — the value, read as music.
///
/// Simultaneous notes in one voice are one chord event, so what is counted is
/// pitches and not events.
fn counted(bindings: &str, expression: &str) -> usize {
    let snapshot = snapshot_of(&probe(bindings, expression));
    snapshot
        .parts()
        .iter()
        .flat_map(|(_, part)| part.voices())
        .flat_map(|(_, voice)| voice.events())
        .map(|event| match event.kind {
            ScoreEventKind::Note { .. } => 1,
            ScoreEventKind::Chord { ref pitches } => pitches.len(),
            ScoreEventKind::Rest => 0,
        })
        .sum()
}

/// One note when the result holds a row, none when it holds a reason.
fn admitted(sequence: &str) -> usize {
    counted(
        &format!(
            "    fn one(series: Row12) -> EventTrack<WrittenTime> {{ beat() }}
    let admitted: EventTrack<WrittenTime> = match row({sequence}) {{
        Ok(series) -> one(series),
        Err(reason) -> music {{ rest/1 }},
    }};"
        ),
        "admitted",
    )
}

#[test]
fn only_a_bijection_is_a_row() {
    assert_eq!(admitted("generic_pcs"), 1, "twelve pitch classes, each once");
    assert_eq!(admitted("chromatic_pcs"), 1, "and the chromatic ascent is one too");
    assert_eq!(admitted("flawed_pcs"), 0, "a repeated pitch class is not a row");
    assert_eq!(admitted("pcs([0, 1, 2])"), 0, "nor is a sequence too short to be one");
    assert_eq!(
        admitted("pcs([0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 0])"),
        0,
        "nor one too long, however many pitch classes it names"
    );
    assert_eq!(
        admitted("pcs([0, 12, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11])"),
        0,
        "and twelve is zero, so this sequence repeats a pitch class it looks like it does not"
    );
}

#[test]
fn a_refusal_says_which_position_repeated_and_which_class_never_came() {
    let bindings = "
    let repeats: List<Nat> = repeated_positions(flawed_pcs);
    let missing: List<Pc12> = missing_classes(flawed_pcs);
";
    assert_eq!(
        counted(bindings, "chorus(map(beat_for_nat, repeats))"),
        1,
        "one order position repeated a pitch class heard earlier"
    );
    assert_eq!(
        counted(bindings, "chorus(map(tally, repeats))"),
        3,
        "and it is position three, not position zero: the first occurrence is where the class belongs"
    );
    assert_eq!(
        counted(bindings, "chorus(map(beat_for_pc, missing))"),
        1,
        "one pitch class never arrived"
    );
    assert_eq!(
        counted(bindings, "chorus(map(tally, map(number_of, missing)))"),
        11,
        "and it is eleven"
    );
    assert_eq!(
        counted("", "chorus(map(beat_for_nat, repeated_positions(generic_pcs)))"),
        0,
        "a row repeats nothing"
    );
    assert_eq!(
        counted("", "chorus(map(beat_for_pc, missing_classes(generic_pcs)))"),
        0,
        "and misses nothing, which for a sequence of twelve is the same fact twice"
    );
}

#[test]
fn the_refusal_itself_carries_both_reasons() {
    // The same two facts as the test above, but read out of the value `row`
    // returned rather than asked of the sequence a second time. That is what
    // the sum buys: the reason travels with the failure.
    let bindings = "
    let flawed: Result<Row12, RowFault> = row(flawed_pcs);
    fn repeats_of(reason: RowFault) -> List<Nat> {
        match reason { Fault(repeats, missing) -> repeats }
    }
    fn missing_of(reason: RowFault) -> List<Pc12> {
        match reason { Fault(repeats, missing) -> missing }
    }
    let carried_repeats: List<Nat> = match flawed { Ok(series) -> [], Err(reason) -> repeats_of(reason) };
    let carried_missing: List<Pc12> = match flawed { Ok(series) -> [], Err(reason) -> missing_of(reason) };
";
    assert_eq!(
        counted(bindings, "chorus(map(tally, carried_repeats))"),
        3,
        "position three, carried by the failure"
    );
    assert_eq!(
        counted(bindings, "chorus(map(tally, map(number_of, carried_missing)))"),
        11,
        "and pitch class eleven, carried by the same failure"
    );
    assert_eq!(
        counted(
            "
    let intact: Result<Row12, RowFault> = row(generic_pcs);
    let is_row: Nat = match intact { Ok(series) -> 1, Err(reason) -> 0 };
",
            "tally(is_row)"
        ),
        1,
        "and a row arrives on the other side of the sum, with no reason to carry"
    );
}

#[test]
fn a_row_has_twelve_order_positions() {
    let bindings = "
    fn spread(series: Row12) -> EventTrack<WrittenTime> { chorus(map(beat_for_pc, pcs_of(series))) }
    let positions: EventTrack<WrittenTime> = match generic { Ok(series) -> spread(series), Err(reason) -> music { rest/1 } };
";
    assert_eq!(
        counted(bindings, "positions"),
        12,
        "twelve positions, each holding a pitch class"
    );
}

#[test]
fn the_forty_eight_labels_are_not_forty_eight_rows() {
    let bindings = "
    fn form_count(series: Row12) -> Nat { distinct_forms(series) }
    fn symmetry_count(series: Row12) -> Nat { symmetries(series) }
    let generic_forms: Nat = match generic { Ok(series) -> form_count(series), Err(reason) -> 0 };
    let generic_symmetries: Nat = match generic { Ok(series) -> symmetry_count(series), Err(reason) -> 0 };
    let chromatic_forms: Nat = match chromatic { Ok(series) -> form_count(series), Err(reason) -> 0 };
    let chromatic_symmetries: Nat = match chromatic { Ok(series) -> symmetry_count(series), Err(reason) -> 0 };
";
    assert_eq!(
        counted(bindings, "tally(generic_forms)"),
        48,
        "a generic row has all 48"
    );
    assert_eq!(
        counted(bindings, "tally(generic_symmetries)"),
        1,
        "fixed only by doing nothing"
    );
    assert_eq!(
        counted(bindings, "tally(chromatic_forms)"),
        24,
        "the chromatic ascent has half as many, because a retrograde inversion returns it"
    );
    assert_eq!(
        counted(bindings, "tally(chromatic_symmetries)"),
        2,
        "which is exactly the operation that fixes it, alongside doing nothing"
    );
}

#[test]
fn every_row_stands_in_a_twelve_by_twelve_matrix() {
    let bindings = "
    fn matrix_of(series: Row12) -> List<Row12> { matrix(series) }
    let rows: List<Row12> = match generic { Ok(series) -> matrix_of(series), Err(reason) -> [] };
";
    assert_eq!(counted(bindings, "chorus(map(beat_for_row, rows))"), 12, "twelve rows");
    assert_eq!(
        counted(
            "
    fn matrix_of(series: Row12) -> List<Row12> { matrix(series) }
    fn spread(series: Row12) -> EventTrack<WrittenTime> { chorus(map(beat_for_pc, pcs_of(series))) }
    let rows: List<Row12> = match generic { Ok(series) -> matrix_of(series), Err(reason) -> [] };
",
            "chorus(map(spread, rows))"
        ),
        144,
        "and twelve pitch classes in each"
    );
}

#[test]
fn a_form_is_numbered_only_once_a_convention_is_named() {
    let bindings = "
    fn up_three(series: Row12) -> Row12 { transposed(series, 3) }
    fn fixed(series: Row12) -> Nat { fixed_zero_index(series) }
    fn fixed_of_moved(series: Row12) -> Nat { fixed_zero_index(up_three(series)) }
    fn moveable_from(series: Row12) -> Nat { moveable_zero_index(series, series) }
    fn moveable_of_moved(series: Row12) -> Nat { moveable_zero_index(series, up_three(series)) }
";
    assert_eq!(
        counted(
            bindings,
            "tally(match generic { Ok(series) -> fixed(series), Err(reason) -> 0 })"
        ),
        0,
        "this row begins on pitch class zero, so fixed-zero calls it P0"
    );
    assert_eq!(
        counted(
            bindings,
            "tally(match generic { Ok(series) -> fixed_of_moved(series), Err(reason) -> 0 })"
        ),
        3,
        "and calls its transposition by three P3"
    );
    assert_eq!(
        counted(
            bindings,
            "tally(match generic { Ok(series) -> moveable_from(series), Err(reason) -> 0 })"
        ),
        0,
        "moveable-zero calls the row as written P0, whatever it begins on"
    );
    assert_eq!(
        counted(
            bindings,
            "tally(match generic { Ok(series) -> moveable_of_moved(series), Err(reason) -> 0 })"
        ),
        3,
        "and measures every other form from there"
    );
}

#[test]
fn spelling_a_row_loses_the_notes_the_collection_cannot_write() {
    let bindings = "
    fn in_c_major(series: Row12) -> List<Option<NoteName>> { row_spelled_in(series, scale c major) }
    fn in_octatonic(series: Row12) -> List<Option<NoteName>> { row_spelled_in(series, scale c octatonic_half_whole) }
    let spelled_in_c: List<Option<NoteName>> = match generic { Ok(series) -> in_c_major(series), Err(reason) -> [] };
    let spelled_octatonically: List<Option<NoteName>> = match generic {
        Ok(series) -> in_octatonic(series),
        Err(reason) -> [],
    };
";
    assert_eq!(
        counted(bindings, "chorus(map(sounded, spelled_in_c))"),
        7,
        "C major writes seven of the twelve pitch classes and no others"
    );
    assert_eq!(
        counted(bindings, "chorus(map(beat_for_nat, map(number_of, generic_pcs)))"),
        12,
        "though the row itself still has twelve positions: the loss is in the spelling, not the row"
    );
    assert_eq!(
        counted(bindings, "chorus(map(sounded, spelled_octatonically))"),
        8,
        "and a collection of eight writes eight of them: what survives is the collection's own size"
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
