//! A staff region, read into the staff package (prompt 127dcfa).
//!
//! The adapter reads and the package decides, so a law about the expansion is
//! a law about the *value* the region produced, never about the text it was
//! rewritten into. Every test below therefore hands the region's document to
//! the package's own `realize` and reads the score that comes out: an onset
//! and a written value per note, which is what the notation said and nothing
//! else.
//!
//! This is `staff_package_laws.rs`'s observation bridge, pointed at a region
//! instead of at hand-written data. That the two agree is the point — the
//! expansion produces the package's data and adds nothing to it.

// Test helpers use expect() on statically-valid inputs: a failure is a bug in
// the test itself, and panicking is the correct behavior there.
#![allow(clippy::expect_used)]
// A fixture the adapter reads differently from how the law says it does is the
// law failing; panicking with what it actually said is the report.
#![allow(clippy::panic)]

use musa_compiler::{CompileOptions, ScoreEventKind, SourceDocument, compile};

/// The bridge from the package's own data into music, and nothing else.
///
/// A realized span at position `p` for duration `d` becomes a note shifted by
/// `p` and stretched to `d`, so the score's onsets are the realization exactly.
/// A refusal becomes silence, so a refused document sounds nothing.
const PRELUDE: &str = r"
import std::notation::staff;
import syntax std::adapters::staff as staff;

fn one(start: Position<WrittenTime>, held: Duration<WrittenTime>) -> EventTrack<WrittenTime> {
    shift(position_between(position_of(0/1), start), stretch(duration_ratio(held), music { c5/1 }))
}

fn heard(spans: WrittenSpans) -> EventTrack<WrittenTime> {
    written_spans_fold(
        music { rest/1 },
        fn (
            anchor: Nat,
            start: Position<WrittenTime>,
            held: Duration<WrittenTime>,
            tied: Tie,
            after: EventTrack<WrittenTime>,
        ) -> EventTrack<WrittenTime> { together(one(start, held), after) },
        spans,
    )
}

fn shown(answer: Result<Realization, Text>) -> EventTrack<WrittenTime> {
    match answer {
        Ok(reached) -> heard(reached.spans),
        Err(why) -> music { rest/1 },
    }
}
";

/// A piece binding `page` to `region` and sounding `sounded`.
fn source(region: &str, sounded: &str) -> String {
    format!(
        "piece \"Staff expansion laws\" {{\n{PRELUDE}\n\
         let page: StaffDocument = syntax staff {{\n{region}\n}};\n\
         let sounded: EventTrack<WrittenTime> = {sounded};\n\
         meter 4/4;\nkey c major;\n\
         score {{ part p {{ voice v {{ use sounded; }} }} }}\n}}\n"
    )
}

/// The score a region and a bridge compile to, or a panic naming the refusal.
fn events(region: &str, sounded: &str) -> Vec<musa_compiler::ScoreEvent> {
    let compilation = compile(
        &SourceDocument::new(source(region, sounded), "staff-expansion-laws.musa"),
        &CompileOptions::default(),
    );
    assert!(!compilation.has_errors(), "{:?}", compilation.diagnostics());
    let score = compilation.snapshot().expect("a score");
    score
        .parts()
        .iter()
        .flat_map(|(_, part)| part.voices())
        .flat_map(|(_, voice)| voice.events().to_vec())
        .collect()
}

/// Every note the region realizes to, as `(onset, written duration)` in
/// whole-note fractions, earliest first.
///
/// The whole rest the span fold ends in is dropped: it is the fold's base and
/// says nothing about what was read.
fn notes(region: &str) -> Vec<(String, String)> {
    let mut found: Vec<(num_rational::Ratio<i64>, String)> = events(region, "shown(realize(page))")
        .into_iter()
        .filter(|event| !matches!(event.kind, ScoreEventKind::Rest))
        .map(|event| (event.onset.as_ratio(), event.notated_duration.spelling))
        .collect();
    found.sort();
    found
        .into_iter()
        .map(|(onset, spelling)| (format!("{}/{}", onset.numer(), onset.denom()), spelling))
        .collect()
}

/// What the laws below compare against: `(onset, written duration)`.
fn spans(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
    pairs
        .iter()
        .map(|(onset, spelling)| ((*onset).to_owned(), (*spelling).to_owned()))
        .collect()
}

/// What a region the adapter refuses says, and the composer's own text it
/// points at.
///
/// A refusal is `Err((node, text))`: the sentence is the adapter's, and the
/// node is one the adapter was handed, which lives in the region the composer
/// wrote. Reading the primary label's span back out of the piece's own source
/// is what proves the second half — a span that lands on generated text would
/// come back as something the composer never typed.
fn refusal(region: &str) -> (String, String) {
    let text = source(region, "shown(realize(page))");
    let compilation = compile(
        &SourceDocument::new(text.clone(), "staff-expansion-laws.musa"),
        &CompileOptions::default(),
    );
    let complaint = compilation
        .diagnostics()
        .iter()
        .find(|diagnostic| diagnostic.message.starts_with("`std::adapters::staff`:"))
        .unwrap_or_else(|| panic!("the adapter accepted this region: {:?}", compilation.diagnostics()));
    let label = complaint.labels.first().expect("a refusal points somewhere");
    let span = label.span.start as usize..label.span.end as usize;
    let pointed = text.get(span).unwrap_or("<outside the composer's source>").to_owned();
    (complaint.message.clone(), pointed)
}

/// The five words every region below opens with, so that each law's own text
/// is the item it is about.
const HEAD: &str = "\
    instrument \"flute\"
    transposing P1
    clef treble
    key c major
    time (4, 4)
    spelling exact_values
";

/// A region with the ordinary head and `body` for its music.
fn page(body: &str) -> String {
    format!("{HEAD}\n{body}")
}

// --- The fourteen items ----------------------------------------------------

#[test]
fn a_note_states_its_own_pitch_and_its_own_written_value() {
    assert_eq!(
        notes(&page("bar (4, 4) { c5/4 d5/4 e5/4 f5/4 }")),
        spans(&[("0/1", "1/4"), ("1/4", "1/4"), ("1/2", "1/4"), ("3/4", "1/4")])
    );
}

/// A rest is written material with a written value, so the package places it
/// like anything else and the notes around it move by its length. What a rest
/// does not do is sound, and that is a performance question this stage has no
/// opinion about.
#[test]
fn a_rest_takes_its_written_value_and_moves_what_follows_it() {
    assert_eq!(
        notes(&page("bar (4, 4) { c5/4 rest/2 d5/4 }")),
        spans(&[("0/1", "1/4"), ("1/4", "1/2"), ("3/4", "1/4")])
    );
}

#[test]
fn a_chord_is_one_written_value_over_several_pitches() {
    assert_eq!(
        notes(&page("bar (4, 4) { [c5 e5 g5]/2 [d5 f5]/2 }")),
        spans(&[("0/1", "1/2"), ("1/2", "1/2")])
    );
}

#[test]
fn a_dot_lengthens_the_value_it_follows_by_half() {
    assert_eq!(
        notes(&page("bar (4, 4) { c5/2. d5/4 }")),
        spans(&[("0/1", "3/4"), ("3/4", "1/4")])
    );
}

#[test]
fn an_exact_duration_is_written_as_the_rational_it_is() {
    assert_eq!(
        notes(&page("bar (4, 4) { c5(3/8) d5(1/8) e5/2 }")),
        spans(&[("0/1", "3/8"), ("3/8", "1/8"), ("1/2", "1/2")])
    );
}

#[test]
fn a_tie_joins_a_note_to_what_sounds_next_across_a_bar_line() {
    assert_eq!(
        notes(&page(
            "bar (4, 4) { c5/2 d5/4 e5/4 ~ }\n    bar (4, 4) { e5/4 f5/4 g5/2 }"
        )),
        spans(&[
            ("0/1", "1/2"),
            ("1/2", "1/4"),
            ("3/4", "1/2"),
            ("5/4", "1/4"),
            ("3/2", "1/2"),
        ])
    );
}

#[test]
fn a_slur_holds_what_it_marks_and_changes_no_span() {
    assert_eq!(
        notes(&page("bar (4, 4) { slur { c5/4 d5/4 } e5/2 }")),
        spans(&[("0/1", "1/4"), ("1/4", "1/4"), ("1/2", "1/2")])
    );
}

#[test]
fn a_tuplet_plays_its_inside_in_the_time_it_states() {
    assert_eq!(
        notes(&page("bar (4, 4) { tuplet (3, 2) { c5/4 d5/4 e5/4 } f5/2 }")),
        spans(&[("0/1", "1/6"), ("1/6", "1/6"), ("1/3", "1/6"), ("1/2", "1/2")])
    );
}

#[test]
fn a_pickup_is_a_bar_of_the_length_it_states() {
    assert_eq!(
        notes(&page("pickup (1, 4) { g4/4 }\n    bar (4, 4) { c5/1 }")),
        spans(&[("0/1", "1/4"), ("1/4", "1")])
    );
}

#[test]
fn a_repeat_holds_what_it_repeats_and_says_how_many_times() {
    assert_eq!(
        notes(&page("repeat (2) {\n        bar (4, 4) { c5/1 }\n    }")),
        spans(&[("0/1", "1")])
    );
}

#[test]
fn an_alternate_ending_says_which_pass_it_is_for() {
    assert_eq!(
        notes(&page(
            "repeat (2) {\n        bar (4, 4) { c5/1 }\n        ending (1) {\n            bar (4, 4) { d5/1 }\n        }\n    }"
        )),
        spans(&[("0/1", "1"), ("1/1", "1")])
    );
}

#[test]
fn a_meter_change_is_written_where_it_happens_and_takes_no_time() {
    assert_eq!(
        notes(&page("bar (4, 4) { c5/1 }\n    meter (3, 4)\n    bar (3, 4) { d5/2. }")),
        spans(&[("0/1", "1"), ("1/1", "3/4")])
    );
}

/// A grace note is written and has no written span, so a document holding one
/// is refused by the package rather than placed by it. The adapter reads it
/// and records the pitches; when it takes its time from is a performance
/// profile's answer, which is §2.4's deliberate silence.
#[test]
fn a_grace_note_is_read_and_left_for_a_performance_to_time() {
    let with_grace = notes(&page("bar (4, 4) { c5/2 grace [e5] d5/2 }"));
    assert!(
        with_grace.is_empty(),
        "the package placed a grace note instead of refusing to: {with_grace:?}"
    );
    assert_eq!(
        notes(&page("bar (4, 4) { c5/2 d5/2 }")),
        spans(&[("0/1", "1/2"), ("1/2", "1/2")]),
        "the same bar without the grace is placed, so the grace is what the package refused"
    );
}

/// A transposing instrument is recorded and not applied: the written spans are
/// the same whatever the shift is, and the shift is still there to be read.
/// Roadmap §2's row — written pitch is not sounding pitch — held by a test.
#[test]
fn a_transposing_instrument_is_recorded_and_not_applied() {
    let concert = notes(&page("bar (4, 4) { c5/1 }"));
    let clarinet = notes(&format!(
        "{}\n{}",
        HEAD.replace("transposing P1", "transposing M2"),
        "bar (4, 4) { c5/1 }"
    ));
    assert_eq!(concert, clarinet, "the shift moved a written span");

    let sounded = "match page {
        Document(instrument, shift, written_clef, tonality, beats, spelled, items) ->
            transpose(shift, music { c5/1 }),
    }";
    let heard: Vec<String> = events(
        &format!(
            "{}\n{}",
            HEAD.replace("transposing P1", "transposing M2"),
            "bar (4, 4) { c5/1 }"
        ),
        sounded,
    )
    .into_iter()
    .filter_map(|event| match event.kind {
        ScoreEventKind::Note { pitch } => Some(pitch.to_string()),
        ScoreEventKind::Rest | ScoreEventKind::Chord { .. } => None,
    })
    .collect();
    assert_eq!(
        heard,
        vec!["d5".to_owned()],
        "the region did not record `transposing M2`"
    );
}

// --- The refusals ----------------------------------------------------------
//
// Every law below asserts two things at once: the sentence the adapter says,
// and that the caret lands on text the composer typed. The second half is the
// one that is easy to lose — an expansion writes new text, and a diagnostic
// against that text names a place nobody can open.

/// §2.5's first: the adapter computes each bar's length from what the tokens
/// spell and compares it against what the bar says it measures. Only the
/// adapter can say *where*, because only the adapter holds nodes.
#[test]
fn a_bar_that_does_not_fill_its_meter_is_refused_at_the_bar() {
    let (said, pointed) = refusal(&page("bar (4, 4) { c5/2 d5/4 }"));
    assert!(
        said.contains("this bar does not hold what it says it measures"),
        "the adapter said something else: {said}"
    );
    assert_eq!(pointed, "bar", "the caret landed on `{pointed}`");
}

/// §2.5's second: a tie joins two notes, so the note it continues into has to
/// be spelled the same way. Two spellings of one sounding pitch are two notes.
#[test]
fn a_tie_into_a_differently_spelled_note_is_refused_at_the_tie() {
    let (said, pointed) = refusal(&page(
        "bar (4, 4) { c5/2 d5/4 e5/4 ~ }\n    bar (4, 4) { f5/4 g5/4 a5/2 }",
    ));
    assert!(
        said.contains("ties a note to a differently spelled one"),
        "the adapter said something else: {said}"
    );
    assert!(!pointed.is_empty(), "the refusal pointed at nothing");
}

#[test]
fn a_tie_with_nothing_sounding_after_it_is_refused() {
    let (said, _) = refusal(&page("bar (4, 4) { c5/2 d5/4 e5/4 ~ }"));
    assert!(
        said.contains("ties a note to nothing"),
        "the adapter said something else: {said}"
    );
}

#[test]
fn a_word_the_staff_does_not_know_is_refused_at_the_word() {
    let (said, pointed) = refusal(&page("crescendo\n    bar (4, 4) { c5/1 }"));
    assert!(
        said.contains("this staff does not know this word"),
        "the adapter said something else: {said}"
    );
    assert_eq!(pointed, "crescendo", "the caret landed on `{pointed}`");
}

#[test]
fn a_note_that_states_no_written_value_is_refused_at_the_note() {
    let (said, pointed) = refusal(&page("bar (1, 4) { c5 d5/4 }"));
    assert!(
        said.contains("this note states no written value"),
        "the adapter said something else: {said}"
    );
    assert_eq!(pointed, "c5", "the caret landed on `{pointed}`");
}

#[test]
fn an_empty_chord_is_refused_at_the_chord() {
    let (said, pointed) = refusal(&page("bar (4, 4) { []/1 }"));
    assert!(
        said.contains("this chord holds no pitches"),
        "the adapter said something else: {said}"
    );
    assert_eq!(pointed, "[]", "the caret landed on `{pointed}`");
}

#[test]
fn a_tuplet_that_plays_nothing_in_the_time_of_some_is_refused_at_the_tuplet() {
    let (said, pointed) = refusal(&page("bar (4, 4) { tuplet (0, 2) { c5/4 } f5/2 }"));
    assert!(
        said.contains("this tuplet plays no notes in the time of some"),
        "the adapter said something else: {said}"
    );
    assert_eq!(pointed, "tuplet", "the caret landed on `{pointed}`");
}

#[test]
fn a_header_stated_twice_is_refused_at_the_second_statement() {
    let (said, _) = refusal(&format!("{HEAD}    clef bass\n\n    bar (4, 4) {{ c5/1 }}"));
    assert!(
        said.contains("this staff says this twice"),
        "the adapter said something else: {said}"
    );
}

// --- Local readability -----------------------------------------------------

/// Every event says its own register and its own written value, so reading one
/// note never requires reading the note before it.
///
/// The law is stated twice because it has two halves. Read positively: each
/// event of a bar, put alone in a bar of its own length, keeps exactly the
/// value it had among its neighbours. Read negatively: an event that leaves
/// either half out is refused rather than completed from what came before —
/// which is the half that would quietly stop being true if the reader ever
/// grew a memory.
#[test]
fn no_note_takes_its_register_or_its_value_from_the_note_before_it() {
    let together: Vec<String> = notes(&page("bar (4, 4) { c5/4 d5/8 e5/8 f5/2 }"))
        .into_iter()
        .map(|(_, held)| held)
        .collect();
    let alone: Vec<String> = [("1, 4", "c5/4"), ("1, 8", "d5/8"), ("1, 8", "e5/8"), ("1, 2", "f5/2")]
        .iter()
        .flat_map(|(measures, event)| notes(&page(&format!("bar ({measures}) {{ {event} }}"))))
        .map(|(_, held)| held)
        .collect();
    assert_eq!(together, alone, "an event read alone is not the event read in company");

    let (no_value, _) = refusal(&page("bar (1, 4) { c5 }"));
    assert!(
        no_value.contains("this note states no written value"),
        "a note with no written value was completed from somewhere: {no_value}"
    );

    let (no_register, _) = refusal(&page("bar (1, 4) { c/4 }"));
    assert!(
        no_register.contains("this staff does not know this word"),
        "a pitch with no octave was completed from somewhere: {no_register}"
    );
}

/// A written value is spent on the note it belongs to, so a second one before
/// any note has been read belongs to nothing — and picking either of the two
/// would be the reader deciding what the composer meant.
///
/// `c5(3/8)/4` is the way to write two: it states an exact span *and* a note
/// value. `c5/4/8` cannot be the fixture, because the lexer reads `4/8` whole
/// as one rational token and the adapter never sees two numerals at all.
#[test]
fn a_note_that_states_two_written_values_is_refused() {
    let (said, _) = refusal(&page("bar (1, 4) { c5(3/8)/4 }"));
    assert!(
        said.contains("this note states two written values"),
        "the adapter said something else: {said}"
    );
}
