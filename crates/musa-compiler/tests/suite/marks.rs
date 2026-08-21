//! The notation vocabulary.
//!
//! One claim, in three parts: the table is the only place a mark is defined,
//! so every row is reachable by the name it declares, no two rows answer to
//! the same name, and a name that is not in the table is refused at the one
//! place a name enters — with a suggestion, so the refusal is useful rather
//! than merely correct.

// Test helpers use expect() on statically-valid inputs: a failure is a bug in
// the test itself, and panicking is the correct behavior there.
#![allow(clippy::expect_used)]

use musa_compiler::{CompileOptions, SourceDocument, compile};

use musa_score::{Anchor, Mark, VOCABULARY, lookup_mark};

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
    // A real word of notation that musa has no row for.
    assert!(lookup_mark("sforzando").is_none());
    assert!(lookup_mark("").is_none());
    assert!(Mark::parse("Staccato").is_none(), "names are not case-folded");
}

#[test]
fn an_unknown_mark_is_refused_with_a_suggestion() {
    let reported = diagnostics_of("c4 1 stacatto");
    assert!(
        reported
            .iter()
            .any(|message| message.contains("`stacatto` is not a mark") && message.contains("staccato")),
        "expected a refusal naming the nearest mark, got {reported:?}"
    );
}

/// Every row that a note may carry is accepted on a note.
///
/// Some rows a note may *not* carry — a pedal covers music and a rehearsal
/// letter stands between notes — so the anchor decides, and the full
/// statement of that is
/// `notation_marks::the_anchor_decides_where_a_mark_is_written`. What is left
/// here is the half of the table this file was written about.
#[test]
fn every_mark_in_the_table_is_accepted_where_marks_are_written() {
    for def in VOCABULARY.iter().filter(|def| matches!(def.anchor, Anchor::Note(_))) {
        let reported = diagnostics_of(&format!("c4 1 {}", def.name));
        assert!(reported.is_empty(), "`{}` was refused: {reported:?}", def.name);
    }
}

/// A shorthand is one token, and it names the row it is written in.
///
/// The mark a composer writes on a note (`a5/4>`) and the word for it
/// (`a5/4 accent`) are two spellings of one thing, and they are declared in
/// two crates: the table below says an accent is written `>`, and the parser
/// one layer down is what turns `>` into the name. Nothing makes those agree
/// except this law, which is why it checks the round trip rather than the
/// table alone.
///
/// One token is not a detail either. A mark sits on the note with no space in
/// front of it, so a two-token shorthand would either need one or would change
/// what the note beside it means.
#[test]
fn every_shorthand_is_one_token_and_names_its_own_row() {
    let mut checked = 0usize;
    for def in VOCABULARY {
        let Some(shorthand) = def.shorthand else {
            continue;
        };
        let lexed = musa_language::lex(shorthand);
        assert_eq!(
            lexed.tokens().len(),
            1,
            "`{shorthand}` ({}) is {} tokens",
            def.name,
            lexed.tokens().len()
        );
        let marks = articulations_of(&format!("a5/4{shorthand}"));
        assert_eq!(
            marks,
            vec![def.name.to_owned()],
            "`{shorthand}` should be `{}`",
            def.name
        );
        assert_eq!(marks, articulations_of(&format!("a5/4 {}", def.name)), "two spellings");
        checked = checked.saturating_add(1);
    }
    assert!(checked >= 2, "the table declares shorthands; found {checked}");
}

/// The articulation names a voice's one note carries, in written order.
fn articulations_of(voice: &str) -> Vec<String> {
    let source = format!("piece \"p\" {{ meter 4/4; score {{ part a {{ voice b {{ {voice} }} }} }} }}");
    compile(&SourceDocument::new(&source, "marks.musa"), &CompileOptions::default())
        .into_snapshot()
        .expect("compiles")
        .annotations()
        .articulations()
        .iter()
        .map(|articulation| articulation.mark.name().to_owned())
        .collect()
}
