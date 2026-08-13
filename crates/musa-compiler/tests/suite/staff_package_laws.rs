//! The staff package, read as data (prompt 127dcf).
//!
//! `std::notation::staff` is ordinary unprivileged Musa: `data` declarations,
//! their generated folds, and functions written with the compiler-owned
//! operations any package may use. Nothing in it is built in, which is the
//! claim these laws are here to keep true — if one of them starts needing a
//! new compiler-owned operation, that is evidence against prompt 127dcea's
//! operation list rather than a reason to grant one.
//!
//! **How a value is observed.** The package answers with its own data, and a
//! test cannot read a `Result<Realization, Text>` directly. Every law below
//! therefore bridges the answer into music in the *piece*, never in the
//! package: a realized span at position `p` for duration `d` becomes a note
//! shifted by `p` and stretched to `d`, so the score's onsets and spellings
//! are the realization, exactly. A refusal becomes silence, so a refused
//! document is one that sounds no notes at all.

// Test helpers use expect() on statically-valid inputs: a failure is a bug in
// the test itself, and panicking is the correct behavior there.
#![allow(clippy::expect_used)]

use musa_compiler::{CompileOptions, ScoreEventKind, SourceDocument, compile};

/// The bridge from the package's own data into music, and nothing else.
///
/// It lives here rather than in the package because the package must not
/// depend on contextual `Music`: prompt 127e deletes that type, and a package
/// that reached for it would have to be rewritten rather than recompiled.
const PRELUDE: &str = r#"
import std::notation::staff;

fn one(start: Position<WrittenTime>, held: Duration<WrittenTime>) -> Music {
    match position_between(position_of(0), start) {
        Ok(offset) -> shift(offset, stretch(duration_ratio(held), music { c5/1 })),
        Err(why) -> music { rest/1 },
    }
}

fn heard(spans: WrittenSpans) -> Music {
    written_spans_fold(
        music { rest/1 },
        fn (
            anchor: Nat,
            start: Position<WrittenTime>,
            held: Duration<WrittenTime>,
            tied: Tie,
            after: Music,
        ) -> Music { together(one(start, held), after) },
        spans,
    )
}

fn shown(answer: Result<Realization, Text>) -> Music {
    match answer {
        Ok(reached) -> heard(reached.spans),
        Err(why) -> music { rest/1 },
    }
}

fn base_span(value: WrittenDuration) -> Result<Duration<WrittenTime>, Text> {
    match value {
        NoteValue(division, dots) -> match division_span(division) {
            Ok(base) -> duration_of(base),
            Err(why) -> Err(why),
        },
        ExactSpan(span) -> duration_of(span),
    }
}

fn spelled_one(start: Position<WrittenTime>, value: WrittenDuration) -> Music {
    match base_span(value) {
        Ok(base) -> one(start, base),
        Err(why) -> music { rest/1 },
    }
}

fn engraved(answer: Result<Spelled, Text>) -> Music {
    match answer {
        Ok(chosen) -> spelled_fold(
            music { rest/1 },
            fn (anchor: Nat, start: Position<WrittenTime>, value: WrittenDuration, after: Music) -> Music {
                together(spelled_one(start, value), after)
            },
            chosen,
        ),
        Err(why) -> music { rest/1 },
    }
}

fn staffed(spelling: Spelling, items: StaffItem) -> StaffDocument {
    Document("flute", interval_inverse(M2), Treble, key c major, Beats(4, 4), spelling, items)
}
"#;

/// Compile a piece whose one voice sounds `voice`, with `declarations` after
/// the shared prelude.
fn compile_staff(declarations: &str, voice: &str) -> musa_compiler::Compilation {
    compile(
        &SourceDocument::new(
            format!(
                "piece \"Staff laws\" {{\n{PRELUDE}\n{declarations}\n\
                 meter 4/4;\nkey c major;\n\
                 score {{ part p {{ voice v {{ use {voice}; }} }} }}\n}}\n"
            ),
            "staff-laws.musa",
        ),
        &CompileOptions::default(),
    )
}

/// Every note the piece sounds, as `(onset, written duration)` in whole-note
/// fractions, earliest first.
///
/// Rests are dropped: the fold over the package's own data ends in one, so
/// every span sequence carries a whole rest that says nothing about the
/// realization.
fn notes(declarations: &str, voice: &str) -> Vec<(String, String)> {
    let compilation = compile_staff(declarations, voice);
    assert!(!compilation.has_errors(), "{:?}", compilation.diagnostics());
    let score = compilation.snapshot().expect("a score");
    let mut found: Vec<(num_rational::Ratio<i64>, String)> = score
        .parts()
        .iter()
        .flat_map(|(_, part)| part.voices())
        .flat_map(|(_, voice)| voice.events().to_vec())
        .filter(|event| !matches!(event.kind, ScoreEventKind::Rest))
        .map(|event| (event.onset.as_ratio(), event.notated_duration.spelling))
        .collect();
    found.sort();
    found
        .into_iter()
        .map(|(onset, spelling)| (format!("{}/{}", onset.numer(), onset.denom()), spelling))
        .collect()
}

/// One written item after another, as the package's own right-nested
/// constructors. Each part is written up to but not including the field that
/// holds the rest of the sequence.
fn sequence(items: &[&str]) -> String {
    items
        .iter()
        .rev()
        .fold("NoItems".to_owned(), |after, item| format!("{item}, {after})"))
}

/// What the laws below compare against: `(onset, written duration)`.
fn spans(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
    pairs
        .iter()
        .map(|(onset, spelling)| ((*onset).to_owned(), (*spelling).to_owned()))
        .collect()
}

// --- The data covers what a staff writes -----------------------------------

/// All fourteen items the trial has to reach — notes, rests, chords, dots,
/// exact durations, ties, slurs, tuplets, grace notes, a pickup, a repeat, an
/// alternate ending, a meter change, and a transposing instrument — written
/// directly as data.
///
/// The anchors are the syntax positions an expansion would have carried; here
/// they are written by hand, because this prompt has no expansion yet.
const FOURTEEN: &str = "\
let every_item: StaffItem =
    Bar(0, Beats(1, 4),
        Sounded(1, Note(c5, NoteValue(4, 0), Untied), NoItems),
        Bar(2, Beats(4, 4),
            Sounded(3, Note(d5, NoteValue(4, 1), Untied),
            Sounded(4, Rest(NoteValue(8, 0)),
            Sounded(5, Chord([e5, g5], NoteValue(8, 0), Untied),
            Sounded(6, Note(f5, ExactSpan(1/4), TiedOn),
            Sounded(7, Note(f5, NoteValue(4, 0), Untied), NoItems))))),
            Slur(8,
                Sounded(9, Note(g5, NoteValue(4, 0), Untied), NoItems),
                Tuplet(10, 3, 2,
                    Sounded(11, Note(a5, NoteValue(4, 0), Untied), NoItems),
                    Sounded(12, Grace([b5]),
                    Sounded(13, MeterChange(Beats(3, 4)),
                    Repeat(14, 2,
                        Sounded(15, Note(c6, NoteValue(4, 0), Untied), NoItems),
                        Ending(16, 1,
                            Sounded(17, Note(d6, NoteValue(4, 0), Untied), NoItems),
                            NoItems))))))));

let clarinet: StaffDocument =
    Document(\"bb_clarinet\", interval_inverse(M2), Treble, key d major, Beats(4, 4), ShortestReadable, every_item);

fn plus_one(after: Result<Nat, Text>) -> Result<Nat, Text> {
    match after {
        Ok(sofar) -> nat_add(1, sofar),
        Err(why) -> Err(why),
    }
}

fn both_counts(body: Result<Nat, Text>, after: Result<Nat, Text>) -> Result<Nat, Text> {
    match body {
        Ok(inner) -> match after {
            Ok(later) -> nat_add(inner, later),
            Err(why) -> Err(why),
        },
        Err(why) -> Err(why),
    }
}

fn touched(items: StaffItem) -> Result<Nat, Text> {
    staff_item_fold(
        Ok(0),
        fn (anchor: Nat, event: StaffEvent, after: Result<Nat, Text>) -> Result<Nat, Text> { plus_one(after) },
        fn (anchor: Nat, beats: Meter, body: Result<Nat, Text>, after: Result<Nat, Text>) -> Result<Nat, Text> {
            plus_one(both_counts(body, after))
        },
        fn (anchor: Nat, body: Result<Nat, Text>, after: Result<Nat, Text>) -> Result<Nat, Text> {
            plus_one(both_counts(body, after))
        },
        fn (
            anchor: Nat,
            played: Nat,
            against: Nat,
            body: Result<Nat, Text>,
            after: Result<Nat, Text>,
        ) -> Result<Nat, Text> { plus_one(both_counts(body, after)) },
        fn (anchor: Nat, times: Nat, body: Result<Nat, Text>, after: Result<Nat, Text>) -> Result<Nat, Text> {
            plus_one(both_counts(body, after))
        },
        fn (anchor: Nat, pass: Nat, body: Result<Nat, Text>, after: Result<Nat, Text>) -> Result<Nat, Text> {
            plus_one(both_counts(body, after))
        },
        items,
    )
}

fn counted(answer: Result<Nat, Text>) -> Music {
    match answer {
        Ok(total) -> match ratio_of(total) {
            Ok(many) -> match ratio_div(many, 64) {
                Ok(factor) -> stretch(factor, music { c5/1 }),
                Err(why) -> music { rest/1 },
            },
            Err(why) -> music { rest/1 },
        },
        Err(why) -> music { rest/1 },
    }
}
";

/// The generated fold is total over the whole declaration: every constructor
/// has a case, and a document holding all fourteen written items is traversed
/// without one of them being skipped or refused.
///
/// Counted rather than realized, because realization refuses a grace note and
/// this law is about the traversal rather than about time. The count arrives
/// as a duration — eighteen sixty-fourths of a whole note, one per
/// constructor the fold visited — because a `Nat` is not otherwise something
/// a score can show.
#[test]
fn the_fold_reaches_every_one_of_the_fourteen_written_items() {
    assert_eq!(
        notes(FOURTEEN, "counted(touched(clarinet.items))"),
        spans(&[("0/1", "9/32")]),
        "the traversal did not visit all eighteen constructors exactly once"
    );
}

/// A transposing instrument is *recorded*, not applied: the same items under
/// two different transpositions realize identically, because the written side
/// is what this package owns and what a part sounds is a performance
/// profile's question.
#[test]
fn a_document_records_its_transposition_without_applying_it() {
    let declarations = "\
        let items: StaffItem = Sounded(0, Note(c5, NoteValue(4, 0), Untied), NoItems); \
        let concert: StaffDocument = \
            Document(\"flute\", P1, Treble, key c major, Beats(4, 4), ShortestReadable, items); \
        let clarinet: StaffDocument = \
            Document(\"bb_clarinet\", interval_inverse(M2), Treble, key d major, Beats(4, 4), ShortestReadable, items);";
    assert_eq!(
        notes(declarations, "shown(realize(concert))"),
        notes(declarations, "shown(realize(clarinet))"),
        "the transposition changed the written realization"
    );
}

// --- Realization ------------------------------------------------------------

/// Each written value means one exact span, and `realize` says which: an
/// undotted quarter is `1/4`, one dot adds half of it, a second dot adds half
/// again, and an exact span is itself.
#[test]
fn realize_gives_the_exact_span_each_written_value_means() {
    let items = sequence(&[
        "Sounded(0, Note(c5, NoteValue(4, 0), Untied)",
        "Sounded(1, Note(d5, NoteValue(4, 1), Untied)",
        "Sounded(2, Note(e5, NoteValue(4, 2), Untied)",
        "Sounded(3, Note(f5, ExactSpan(1/5), Untied)",
    ]);
    let declarations = format!("let written: StaffDocument = staffed(ExactValues, {items});");
    assert_eq!(
        notes(&declarations, "shown(realize(written))"),
        spans(&[("0/1", "1/4"), ("1/4", "3/8"), ("5/8", "7/16"), ("17/16", "1/5"),]),
    );
}

/// A tie joins what is written to what sounds next, and a barline between
/// them changes nothing about that: two quarters tied across a bar realize as
/// one half, starting where the first was written.
#[test]
fn a_tie_across_a_barline_realizes_as_one_joined_span() {
    let declarations = "\
        let across: StaffDocument = staffed(ShortestReadable, \
            Bar(0, Beats(1, 4), Sounded(1, Note(c5, NoteValue(4, 0), TiedOn), NoItems), \
            Bar(2, Beats(4, 4), Sounded(3, Note(c5, NoteValue(4, 0), Untied), NoItems), \
            NoItems)));";
    assert_eq!(
        notes(declarations, "shown(realize(across))"),
        spans(&[("0/1", "1/2")]),
        "the tie realized as two spans rather than one"
    );
}

/// Three in the time of two: the written values stay quarters and the time
/// they take does not, so three quarters cover a half and whatever follows
/// the tuplet starts there.
#[test]
fn a_tuplet_realizes_its_body_in_the_time_it_borrows() {
    let inside = sequence(&[
        "Sounded(1, Note(c5, NoteValue(4, 0), Untied)",
        "Sounded(2, Note(d5, NoteValue(4, 0), Untied)",
        "Sounded(3, Note(e5, NoteValue(4, 0), Untied)",
    ]);
    let declarations = format!(
        "let triplet: StaffDocument = staffed(ExactValues, \
            Tuplet(0, 3, 2, {inside}, Sounded(4, Note(f5, NoteValue(4, 0), Untied), NoItems)));"
    );
    assert_eq!(
        notes(&declarations, "shown(realize(triplet))"),
        spans(&[("0/1", "1/6"), ("1/6", "1/6"), ("1/3", "1/6"), ("1/2", "1/4")]),
    );
}

/// A pickup is a bar shorter than the meter, and it needs no rule of its own:
/// its content starts where the bar does, and the full bar after it starts a
/// quarter in.
#[test]
fn a_pickup_bar_realizes_where_it_is_written() {
    let declarations = "\
        let upbeat: StaffDocument = staffed(ShortestReadable, \
            Bar(0, Beats(1, 4), Sounded(1, Note(g4, NoteValue(4, 0), Untied), NoItems), \
            Bar(2, Beats(4, 4), Sounded(3, Note(c5, NoteValue(1, 0), Untied), NoItems), \
            NoItems)));";
    assert_eq!(
        notes(declarations, "shown(realize(upbeat))"),
        spans(&[("0/1", "1/4"), ("1/4", "1")]),
    );
}

/// A repeat and its ending are read once, because written time counts the
/// page and not the performance: how many passes a player makes is a reading
/// of the score, and `realize` answers what the score says.
#[test]
fn a_repeat_and_its_ending_are_read_once() {
    let declarations = "\
        let repeated: StaffDocument = staffed(ShortestReadable, \
            Repeat(0, 2, Sounded(1, Note(c5, NoteValue(4, 0), Untied), NoItems), \
            Ending(2, 1, Sounded(3, Note(d5, NoteValue(4, 0), Untied), NoItems), \
            NoItems)));";
    assert_eq!(
        notes(declarations, "shown(realize(repeated))"),
        spans(&[("0/1", "1/4"), ("1/4", "1/4")]),
    );
}

/// A meter change occupies no written time: it says how the bars after it are
/// counted, and what follows it starts where what preceded it ended.
#[test]
fn a_meter_change_takes_no_written_time() {
    let items = sequence(&[
        "Sounded(0, Note(c5, NoteValue(4, 0), Untied)",
        "Sounded(1, MeterChange(Beats(3, 4))",
        "Sounded(2, Note(d5, NoteValue(4, 0), Untied)",
    ]);
    let declarations = format!("let changed: StaffDocument = staffed(ShortestReadable, {items});");
    assert_eq!(
        notes(&declarations, "shown(realize(changed))"),
        spans(&[("0/1", "1/4"), ("1/4", "1/4")]),
    );
}

/// Each item is read in isolation. The same two items in the other order
/// realize to the same two spans in the other order, because no item takes
/// its register or its duration from the one before it.
#[test]
fn no_item_inherits_register_or_duration_from_the_item_before_it() {
    let forward = sequence(&[
        "Sounded(0, Note(c5, NoteValue(4, 0), Untied)",
        "Sounded(1, Note(d5, NoteValue(2, 0), Untied)",
    ]);
    let backward = sequence(&[
        "Sounded(0, Note(d5, NoteValue(2, 0), Untied)",
        "Sounded(1, Note(c5, NoteValue(4, 0), Untied)",
    ]);
    let declarations = format!(
        "let forward: StaffDocument = staffed(ShortestReadable, {forward}); \
         let backward: StaffDocument = staffed(ShortestReadable, {backward});"
    );
    assert_eq!(
        notes(&declarations, "shown(realize(forward))"),
        spans(&[("0/1", "1/4"), ("1/4", "1/2")]),
    );
    assert_eq!(
        notes(&declarations, "shown(realize(backward))"),
        spans(&[("0/1", "1/2"), ("1/2", "1/4")]),
        "an item's span changed with what preceded it"
    );
}

// --- What realization declines to guess -------------------------------------

/// Every document below realizes to nothing, and each is one written item
/// away from realizing to something: the refusal is the only difference.
///
/// The sentence each refusal carries is `Text`, and a piece has no way to
/// observe text, so what is checked here is that the answer is the refusing
/// half. The sentences themselves are in the package, beside the choice each
/// one declines.
#[test]
fn realize_refuses_the_choices_it_will_not_guess() {
    for (name, items) in [
        (
            "a grace note's timing",
            "Sounded(0, Grace([c5]), Sounded(1, Note(d5, NoteValue(4, 0), Untied), NoItems))",
        ),
        (
            "a note value that divides nothing",
            "Sounded(0, Note(c5, NoteValue(0, 0), Untied), NoItems)",
        ),
        (
            "a tie with nothing after it",
            "Sounded(0, Note(c5, NoteValue(4, 0), TiedOn), NoItems)",
        ),
        (
            "a tuplet that plays nothing",
            "Tuplet(0, 0, 2, Sounded(1, Note(c5, NoteValue(4, 0), Untied), NoItems), NoItems)",
        ),
    ] {
        let declarations = format!("let refused: StaffDocument = staffed(ShortestReadable, {items});");
        let sounded = notes(&declarations, "shown(realize(refused))");
        assert!(
            sounded.is_empty(),
            "{name} was guessed at rather than refused: {sounded:?}"
        );
    }
}

// --- Engraving --------------------------------------------------------------

/// `engrave` makes the choice expansion left open: which written value prints
/// a realized span. The observation is each chosen value's *undotted* base,
/// so a dotted quarter chosen for `3/8` shows a base of `1/4`, while the
/// exact policy shows the span itself.
#[test]
fn engrave_spells_a_span_with_the_shortest_readable_value() {
    let items = sequence(&[
        "Sounded(0, Note(c5, NoteValue(4, 1), Untied)",
        "Sounded(1, Note(d5, NoteValue(2, 0), Untied)",
    ]);
    let declarations = format!(
        "let by_reading: StaffDocument = staffed(ShortestReadable, {items}); \
         let exact: StaffDocument = staffed(ExactValues, {items});"
    );
    assert_eq!(
        notes(&declarations, "engraved(engrave(by_reading))"),
        spans(&[("0/1", "1/4"), ("3/8", "1/2")]),
        "the dotted quarter was not spelled as a dotted quarter"
    );
    assert_eq!(
        notes(&declarations, "engraved(engrave(exact))"),
        spans(&[("0/1", "3/8"), ("3/8", "1/2")]),
        "the exact policy did not answer with the span itself"
    );
}

/// A span no readable value covers is a complaint about the document's
/// spelling policy, not a note quietly rounded — and the same span under the
/// exact policy is spelled without difficulty.
#[test]
fn engrave_refuses_a_span_no_readable_value_spells() {
    let items = "Sounded(0, Note(c5, ExactSpan(1/5), Untied), NoItems)";
    let declarations = format!(
        "let by_reading: StaffDocument = staffed(ShortestReadable, {items}); \
         let exact: StaffDocument = staffed(ExactValues, {items});"
    );
    assert!(
        notes(&declarations, "engraved(engrave(by_reading))").is_empty(),
        "a fifth of a whole note was spelled as something readable"
    );
    assert_eq!(
        notes(&declarations, "engraved(engrave(exact))"),
        spans(&[("0/1", "1/5")]),
    );
}
