//! A sub-position holds another pattern.
//!
//! `docs/rules/language/01-surface.md` §1's **Patterns nest** paragraph is the
//! governing sentence, and `02-core-calculus.md` §6.2's case tree is what makes
//! it cheap: a nested pattern compiles by splitting the sub-position the same
//! way the subject is split. So the laws here are about two things and no
//! third. The nested form must *answer* what the depth-one form answered — a
//! test that only asked whether it parsed would pass for a lowering that
//! matched the wrong arm — and coverage must be exactly what it was, which the
//! inexhaustive law asserts by reading the diagnostic §6.2 already names.
//!
//! Every law reads a pitch. `P1` leaves `c4` alone and `P8` takes it to `c5`,
//! so the note this piece sounds *is* which arm ran.

#![allow(clippy::expect_used, clippy::panic)]

use musa_compiler::{CompileOptions, SourceDocument, compile};

use musa_score::ScoreEventKind;

/// The declarations every law here matches against: a two-constructor family,
/// a wrapper around it, and a record holding one.
const DECLARATIONS: &str = "data Inner { Quiet, Loud(count: Nat) } \
                            data Outer { Wrap(held: Inner) } \
                            record Marking { written: Inner; }";

/// A piece whose one note is transposed by the interval `chosen` names.
fn piece(body: &str) -> musa_compiler::Compilation {
    let source = SourceDocument::new(
        format!(
            "piece \"Nested patterns\" {{ {DECLARATIONS} {body} \
             let tune: EventTrack(WrittenTime) = transpose(chosen, music {{ c4/1 }}); \
             tempo 1/4 = 84; meter 4/4; \
             score {{ part p {{ voice v {{ use tune; }} }} }} }}"
        ),
        "nested-patterns.musa",
    );
    compile(&source, &CompileOptions::default())
}

/// The one note that piece sounds, as it is spelled.
fn sounded(body: &str) -> String {
    let compilation = piece(body);
    assert!(!compilation.has_errors(), "{:?}", compilation.diagnostics());
    let score = compilation.snapshot().expect("a score");
    let event = score
        .parts()
        .iter()
        .flat_map(|(_, part)| part.voices())
        .flat_map(|(_, voice)| voice.events().to_vec())
        .next()
        .expect("the one note");
    match event.kind {
        ScoreEventKind::Note { pitch, .. } => pitch.to_string(),
        other @ (ScoreEventKind::Rest | ScoreEventKind::Chord { .. }) => {
            panic!("a voice holding one note holds a note: {other:?}")
        }
    }
}

/// The nested form and the depth-one form are two spellings of one question,
/// so they answer alike — for the arm that is taken *and* for the arm that is
/// not, which is why both subjects are run. One subject would be satisfied by
/// a lowering that always chose the first arm.
#[test]
fn a_nested_match_answers_what_the_two_level_match_answered() {
    for (subject, expected) in [("Wrap(Loud(1))", "c5"), ("Wrap(Quiet)", "c4")] {
        let nested = sounded(&format!(
            "let subject: Outer = {subject}; \
             let chosen: Interval = match subject {{ Wrap(Loud(count)) -> P8, Wrap(Quiet) -> P1 }};"
        ));
        let two_level = sounded(&format!(
            "let subject: Outer = {subject}; \
             let chosen: Interval = \
                 match subject {{ Wrap(held) -> match held {{ Loud(count) -> P8, Quiet -> P1 }} }};"
        ));
        assert_eq!(nested, expected, "the nested match, on {subject}");
        assert_eq!(two_level, nested, "the two readings of {subject}");
    }
}

/// Coverage did not move. A nested arm that leaves an inner constructor
/// unanswered is refused by the rule that already refused the outer one, with
/// the code and the constructor `02-core-calculus.md` §6.2 names — so a
/// lowering that quietly widened what is accepted fails here.
#[test]
fn an_inexhaustive_nested_match_is_refused_where_it_always_was() {
    let compilation = piece(
        "let subject: Outer = Wrap(Loud(1)); \
         let chosen: Interval = match subject { Wrap(Loud(count)) -> P8 };",
    );
    let refusal = compilation
        .diagnostics()
        .iter()
        .find(|diagnostic| diagnostic.code == musa_score::Code::IncompleteMatch)
        .expect("the incomplete-match refusal");
    assert!(
        refusal.message.contains("Inner.Quiet"),
        "the refusal names the constructor with no arm: {}",
        refusal.message
    );
}

/// The same sub-position question in the list spelling. `[Loud(count), ..
/// others]` splits the head, and the tail binding is still a binding.
#[test]
fn a_nested_list_pattern_parses_and_lowers() {
    for (subject, expected) in [("[Loud(1), Quiet]", "c5"), ("[Quiet, Loud(1)]", "c4")] {
        let answer = sounded(&format!(
            "let subject: List(Inner) = {subject}; \
             let chosen: Interval = match subject {{ \
                 [] -> P1, [Loud(count), .. others] -> P8, [Quiet, .. others] -> P1, \
             }};"
        ));
        assert_eq!(answer, expected, "the list match, on {subject}");
    }
}

/// And in the record spelling, where the sub-position is a field rather than a
/// place in an argument list.
#[test]
fn a_nested_record_pattern_parses_and_lowers() {
    for (subject, expected) in [("Loud(1)", "c5"), ("Quiet", "c4")] {
        let answer = sounded(&format!(
            "let subject: Marking = Marking {{ written = {subject} }}; \
             let chosen: Interval = match subject {{ \
                 Marking {{ written = Loud(count) }} -> P8, Marking {{ written = Quiet }} -> P1, \
             }};"
        ));
        assert_eq!(answer, expected, "the record match, on {subject}");
    }
}
