//! The `mark` statement and the vocabulary behind it.
//!
//! A closed enum of articulations became a table, with a measured claim:
//! adding a new mark should cost one row. These tests keep that claim
//! holding — every check here is written against the table rather than
//! against a list of mark names, so a row added later is covered by them
//! without any of them being edited.
//!
//! The one that matters most is `the_anchor_decides_where_a_mark_is_written`.
//! A vocabulary that accepted `mark staccato;` or `g4 1/4 pedal;` would be a
//! vocabulary with no shape, and the diagnostics would be about grammar rather
//! than about music.

// A failure of these is a bug in the fixture, not in a caller's input.
#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]

use musa_compiler::{CompileOptions, SourceDocument, compile};

use musa_score::{Anchor, Argument, VOCABULARY};

/// A piece whose only variable is what is written inside the voice.
fn piece(body: &str) -> String {
    format!(
        "piece \"Marks\" {{ tempo 1/4 = 60; meter 4/4; key c major; score {{ part p {{ voice v {{ {body} }} }} }} }}"
    )
}

fn errors(body: &str) -> Vec<String> {
    compile(
        &SourceDocument::new(piece(body), "marks.musa"),
        &CompileOptions::default(),
    )
    .diagnostics()
    .iter()
    .map(|diagnostic| diagnostic.message.clone())
    .collect()
}

fn compiles(body: &str) {
    let compilation = compile(
        &SourceDocument::new(piece(body), "marks.musa"),
        &CompileOptions::default(),
    );
    assert!(
        compilation.into_snapshot().is_some(),
        "`{body}` was supposed to compile"
    );
}

/// The whole design in one test: a name may be written in exactly the one
/// place its row says, and the other place names the row rather than the
/// grammar.
#[test]
fn the_anchor_decides_where_a_mark_is_written() {
    for def in VOCABULARY {
        let statement = match (def.anchor, def.takes) {
            (Anchor::Note(_), _) => {
                let said = errors(&format!("mark {};", def.name));
                assert!(
                    said.iter().any(|message| message.contains("is written on a note")),
                    "`{}` was accepted as a statement: {said:?}",
                    def.name
                );
                continue;
            }
            (Anchor::Point, Argument::None) => format!("mark {};", def.name),
            (Anchor::Point, Argument::Text) => format!("mark {} \"x\";", def.name),
            (Anchor::Point, Argument::Number) => format!("mark {} 1;", def.name),
            (Anchor::Span, Argument::None) => format!("mark {} {{ c5/4 }}", def.name),
            (Anchor::Span, Argument::Text) => format!("mark {} \"x\" {{ c5/4 }}", def.name),
            (Anchor::Span, Argument::Number) => format!("mark {} 1 {{ c5/4 }}", def.name),
        };
        compiles(&format!("{statement} c5/4 d5/4 e5/4"));
        let said = errors(&format!("c5 1/4 {} d5/4 e5/4 f5/4", def.name));
        assert!(
            said.iter().any(|message| message.contains("is not written on a note")),
            "`{}` was accepted on a note: {said:?}",
            def.name
        );
    }
}

/// Every row is reachable by its own name, and no two rows share one.
#[test]
fn the_table_is_a_vocabulary() {
    let mut names: Vec<&str> = VOCABULARY.iter().map(|def| def.name).collect();
    let count = names.len();
    names.sort_unstable();
    names.dedup();
    assert_eq!(names.len(), count, "two rows share a name");
    for def in VOCABULARY {
        assert!(
            musa_score::lookup_mark(def.name).is_some(),
            "`{}` is not found by its own name",
            def.name
        );
    }
}

/// A span needs music under it and a point needs none, and both are told so
/// in the words of the mark rather than in the words of the parser.
#[test]
fn a_shape_that_does_not_fit_the_mark_is_refused() {
    let said = errors("mark pedal; c5/4 d5/4 e5/4 f5/4");
    assert!(said.iter().any(|message| message.contains("covers music")), "{said:?}");
    let said = errors("mark breath { c5/4 } d5/4 e5/4 f5/4");
    assert!(
        said.iter().any(|message| message.contains("stands at one place")),
        "{said:?}"
    );
}

/// An argument is the row's business too: a text direction with no words is
/// not a direction, and a breath with words is a mark someone confused.
#[test]
fn an_argument_that_does_not_fit_the_mark_is_refused() {
    let said = errors("mark text; c5/4 d5/4 e5/4 f5/4");
    assert!(
        said.iter().any(|message| message.contains("is written with")),
        "{said:?}"
    );
    let said = errors("mark breath \"now\"; c5/4 d5/4 e5/4 f5/4");
    assert!(
        said.iter().any(|message| message.contains("is written on its own")),
        "{said:?}"
    );
}

/// A name musa does not know is a diagnostic that names the half of the table
/// the writer was reaching into.
#[test]
fn an_unknown_mark_is_named() {
    let said = errors("mark pedale { c5/4 } d5/4 e5/4 f5/4");
    assert!(
        said.iter().any(|message| message.contains("`pedale` is not a mark")),
        "{said:?}"
    );
}

/// A number may be negative: an ottava down is written `-1` and is not two
/// tokens as far as the compiler is concerned.
#[test]
fn an_ottava_may_go_down() {
    compiles("mark ottava -1 { c5/4 d5/4 } e5/4 f5/4");
}

/// The claim, re-checked: a mark reaches the exporters through the table,
/// so the whole cost of a new one is the row.
///
/// The check is that no production code *selects* a mark by name — no
/// `Mark::parse("staccato")`, no `lookup_mark("pedal")`. Code that did would
/// have to be edited for the next row, and the claim would be false.
///
/// Searching for the bare names instead would be the obvious check and the
/// wrong one: `text` and `rehearsal` are also a `MusicXML` attribute, a
/// `MusicXML` element and a highlight class, so it flags files that would not
/// change if a row were added, and says nothing about the ones that would.
/// Tests are exempt for the same reason — a fixture that plays a staccato note
/// is not a dispatch on the word.
#[test]
fn no_mark_is_named_outside_the_table() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(std::path::Path::parent)
        .expect("the workspace root");
    let mut offenders: Vec<String> = Vec::new();
    for crate_name in ["musa-compiler", "musa-notation", "musa-syntax"] {
        let source = root.join("crates").join(crate_name).join("src");
        let mut stack = vec![source];
        while let Some(directory) = stack.pop() {
            let Ok(entries) = std::fs::read_dir(&directory) else {
                continue;
            };
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    stack.push(path);
                    continue;
                }
                if path.file_name().is_some_and(|name| name == "marks.rs") {
                    continue;
                }
                if path.extension().is_none_or(|extension| extension != "rs") {
                    continue;
                }
                let Ok(text) = std::fs::read_to_string(&path) else {
                    continue;
                };
                // Comments are prose and may name a mark; and this workspace
                // keeps unit tests in one `#[cfg(test)]` module at the foot of
                // the file, so everything from there down is a fixture.
                let code: String = text
                    .lines()
                    .take_while(|line| !line.trim_start().starts_with("#[cfg(test)]"))
                    .filter(|line| !line.trim_start().starts_with("//"))
                    .collect::<Vec<_>>()
                    .join("\n");
                for def in VOCABULARY {
                    for selector in [
                        format!("Mark::parse(\"{}\"", def.name),
                        format!("lookup_mark(\"{}\"", def.name),
                    ] {
                        if code.contains(&selector) {
                            offenders.push(format!("{} selects `{}`", path.display(), def.name));
                        }
                    }
                }
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "a mark is named outside the table, so adding a row costs more than a row: {offenders:?}"
    );
}
