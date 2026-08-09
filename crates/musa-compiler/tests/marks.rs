//! The notation vocabulary (docs/prompts/62).
//!
//! One claim, in three parts: the table is the only place a mark is defined,
//! so every row is reachable by the name it declares, no two rows answer to
//! the same name, and a name that is not in the table is refused at the one
//! place a name enters — with prompt 56's suggestion, so the refusal is
//! useful rather than merely correct.

// Test helpers use expect() on statically-valid inputs: a failure is a bug in
// the test itself, and panicking is the correct behavior there.
#![allow(clippy::expect_used)]

use musa_compiler::{Anchor, CompileOptions, Mark, SourceDocument, VOCABULARY, compile, lookup_mark};

fn diagnostics_of(voice: &str) -> Vec<String> {
    let source = format!("piece \"p\" {{ meter 4/4; score {{ part a {{ voice b {{ {voice} }} }} }} }}");
    compile(&SourceDocument::new(&source, "marks.musa"), &CompileOptions::default())
        .diagnostics()
        .iter()
        .map(|diagnostic| format!("{}{}", diagnostic.message, diagnostic.help.clone().unwrap_or_default()))
        .collect()
}

#[test]
fn every_row_is_reachable_by_its_own_name() {
    for def in VOCABULARY {
        let mark = Mark::parse(def.name).expect("a row is found by its own name");
        assert_eq!(mark.name(), def.name);
        assert_eq!(mark.def().mei, def.mei);
        assert_eq!(mark.def().musicxml, def.musicxml);
        assert_eq!(mark.def().lilypond, def.lilypond);
    }
}

#[test]
fn no_two_rows_answer_to_one_name() {
    let mut names: Vec<&str> = VOCABULARY.iter().map(|def| def.name).collect();
    let count = names.len();
    names.sort_unstable();
    names.dedup();
    assert_eq!(names.len(), count, "two rows share a name");
}

#[test]
fn a_name_outside_the_table_is_not_a_mark() {
    // A real word of notation that musa has no row for. `fermata` used to be
    // the example here and became a row in prompt 70, which is the table
    // working rather than the test aging.
    assert!(lookup_mark("sforzando").is_none());
    assert!(lookup_mark("").is_none());
    assert!(Mark::parse("Staccato").is_none(), "names are not case-folded");
}

#[test]
fn an_unknown_mark_is_refused_with_a_suggestion() {
    let reported = diagnostics_of("c4 1 stacatto;");
    assert!(
        reported
            .iter()
            .any(|message| message.contains("`stacatto` is not a mark") && message.contains("staccato")),
        "expected a refusal naming the nearest mark, got {reported:?}"
    );
}

/// Every row that a note may carry is accepted on a note.
///
/// Prompt 70 added rows that a note may *not* carry — a pedal covers music and
/// a rehearsal letter stands between notes — so the anchor decides, and the
/// full statement of that is
/// `notation_marks::the_anchor_decides_where_a_mark_is_written`. What is left
/// here is the half of the table this file was written about.
#[test]
fn every_mark_in_the_table_is_accepted_where_marks_are_written() {
    for def in VOCABULARY.iter().filter(|def| matches!(def.anchor, Anchor::Note(_))) {
        let reported = diagnostics_of(&format!("c4 1 {};", def.name));
        assert!(reported.is_empty(), "`{}` was refused: {reported:?}", def.name);
    }
}
