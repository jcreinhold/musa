//! What an import is, and what it is not (docs/prompts/36; roadmap §16).
//!
//! Imports are the one place a musa compilation reads something the author of
//! the piece did not write, so the rules are worth pinning: paths join
//! lexically, a library is not a piece, names are flat and collisions are
//! errors, a cycle is reported rather than followed, and — the property the
//! rest depend on — an imported motif produces exactly the notes it would
//! have produced written in place.

// Test helpers use expect() on statically-valid inputs: a failure is a bug in
// the test itself, and panicking is the correct behavior there.
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]

use musa_compiler::{
    Compilation, CompileOptions, ImportSources, ScoreSnapshot, SourceDocument, compile, resolve_import,
};

/// Compile `source` as `name`, with `files` available to import.
fn compile_with(name: &str, source: &str, files: &[(&str, &str)]) -> Compilation {
    let mut imports = ImportSources::default();
    for (path, text) in files {
        imports.insert(*path, *text);
    }
    compile(
        &SourceDocument::new(source, name),
        &CompileOptions {
            imports,
            ..CompileOptions::default()
        },
    )
}

/// Each error as the whole small document it is: the claim, then the rule and
/// the advice under it. What a reader is told is the sum of the three, so a
/// test that reads only the first line tests less than it looks like it does.
fn errors(compilation: &Compilation) -> Vec<String> {
    compilation
        .diagnostics()
        .iter()
        .filter(|diagnostic| diagnostic.severity == musa_compiler::Severity::Error)
        .map(|diagnostic| {
            let mut lines = vec![diagnostic.message.clone()];
            if let Some(note) = diagnostic.note.as_deref() {
                lines.push(format!("note: {note}"));
            }
            if let Some(help) = diagnostic.help.as_deref() {
                lines.push(format!("help: {help}"));
            }
            lines.join("\n")
        })
        .collect()
}

fn snapshot(compilation: Compilation) -> ScoreSnapshot {
    let messages = errors(&compilation);
    compilation
        .into_snapshot()
        .unwrap_or_else(|| panic!("expected a snapshot; errors: {messages:?}"))
}

const MOTIFS: &str = "library { motif rise() { c5/4 d5/4 e5/4 g5/4 } }";

fn piece(body: &str) -> String {
    format!(
        "piece \"P\" {{ {body} tempo 1/4 = 60; meter 4/4; key c major; score {{ part p {{ voice v {{ use rise(); }} }} }} }}"
    )
}

/// A path is joined onto the importer's directory, with `.` and `..` doing
/// what they say — and no filesystem consulted, so the same graph resolves
/// the same way everywhere.
#[test]
fn a_written_path_joins_onto_the_file_that_wrote_it() {
    let cases = [
        ("pieces/01.musa", "../library/patches.musa", "library/patches.musa"),
        ("pieces/01.musa", "./shared.musa", "pieces/shared.musa"),
        ("a/b/c.musa", "../../top.musa", "top.musa"),
        ("piece.musa", "library.musa", "library.musa"),
        ("/abs/pieces/01.musa", "../library/m.musa", "/abs/library/m.musa"),
    ];
    for (importer, written, expected) in cases {
        assert_eq!(
            resolve_import(importer, written),
            expected,
            "`{written}` from `{importer}`"
        );
    }
}

/// The property everything else rests on: importing a motif is writing it in
/// place. The two pieces below must compile to the same notes.
#[test]
fn an_imported_motif_sounds_exactly_as_it_would_written_in_place() {
    let imported = snapshot(compile_with(
        "p.musa",
        &piece("import \"lib.musa\";"),
        &[("lib.musa", MOTIFS)],
    ));
    let local = snapshot(compile_with(
        "p.musa",
        &piece("motif rise() { c5/4 d5/4 e5/4 g5/4 }"),
        &[],
    ));
    let events = |score: &ScoreSnapshot| {
        score
            .parts()
            .iter()
            .flat_map(|(_, part)| part.voices().map(|(_, voice)| voice))
            .flat_map(|voice| voice.events().iter())
            .map(|event| (event.onset, event.kind.clone(), event.notated_duration.value))
            .collect::<Vec<_>>()
    };
    assert_eq!(events(&imported), events(&local));
}

/// The same file reached twice is read once and shared — importing a library
/// that your library already imported is not a duplicate declaration.
#[test]
fn a_file_reached_twice_is_read_once() {
    let compilation = compile_with(
        "p.musa",
        &piece("import \"lib.musa\"; import \"also.musa\";"),
        &[("lib.musa", MOTIFS), ("also.musa", "library { import \"lib.musa\"; }")],
    );
    assert_eq!(errors(&compilation), Vec::<String>::new());
    assert_eq!(snapshot(compilation).motifs().len(), 1, "one declaration, not two");
}

/// A cycle is reported with the files in it, so the fix is visible from the
/// message rather than from a stack trace.
#[test]
fn an_import_cycle_is_reported_with_its_files() {
    let compilation = compile_with(
        "p.musa",
        &piece("import \"a.musa\";"),
        &[
            ("a.musa", "library { import \"b.musa\"; }"),
            ("b.musa", "library { import \"a.musa\"; }"),
        ],
    );
    let messages = errors(&compilation);
    assert!(
        messages.iter().any(|message| message.contains("import each other")
            && message.contains("a.musa")
            && message.contains("b.musa")),
        "expected a cycle naming both files, got {messages:?}"
    );
}

/// A file that is not there is named, at the `import` that asked for it.
#[test]
fn a_missing_import_names_the_path_it_looked_for() {
    let compilation = compile_with("pieces/01.musa", &piece("import \"../library/gone.musa\";"), &[]);
    assert!(
        errors(&compilation)
            .first()
            .is_some_and(|first| first.starts_with("cannot find `library/gone.musa`")),
        "the missing file is the first thing said; what it would have declared follows"
    );
}

/// An imported file is a `library`. A piece in one is rejected — the point of
/// the rule is that a score cannot be imported and silently ignored.
#[test]
fn a_piece_cannot_be_imported() {
    let compilation = compile_with(
        "p.musa",
        &piece("import \"other.musa\";"),
        &[(
            "other.musa",
            "piece \"Other\" { tempo 1/4 = 60; meter 4/4; key c major; score { part p { voice v { c5/1 } } } }",
        )],
    );
    let messages = errors(&compilation);
    assert!(
        messages
            .iter()
            .any(|message| message.contains("is a piece, not a library")),
        "expected a library rejection, got {messages:?}"
    );
}

/// Names are flat, and a collision is an error rather than a silent winner.
#[test]
fn two_declarations_of_one_name_is_an_error_not_a_shadow() {
    let compilation = compile_with(
        "p.musa",
        &piece("import \"lib.musa\"; motif rise() { c5/1 }"),
        &[("lib.musa", MOTIFS)],
    );
    let messages = errors(&compilation);
    assert!(
        messages
            .iter()
            .any(|message| message.contains("`rise` is declared twice")),
        "expected a collision naming the motif, got {messages:?}"
    );
}

/// A library ships building blocks. Wiring belongs to the piece, which is the
/// only thing that knows what parts exist.
#[test]
fn a_library_studio_may_not_wire_a_score_it_cannot_see() {
    let compilation = compile_with(
        "p.musa",
        &piece("import \"lib.musa\";"),
        &[(
            "lib.musa",
            "library { motif rise() { c5/1 } studio { assign p -> reed; } }",
        )],
    );
    let messages = errors(&compilation);
    assert!(
        messages
            .iter()
            .any(|message| message.contains("`assign` belongs to the piece")),
        "expected the library's `assign` to be refused, got {messages:?}"
    );
}

/// A syntax error inside a library is reported against the file it is in,
/// because a byte offset from another document would point at the wrong
/// bytes of this one.
#[test]
fn a_broken_library_is_reported_by_name() {
    let compilation = compile_with(
        "p.musa",
        &piece("import \"lib.musa\";"),
        &[("lib.musa", "library { motif rise( { c5/1 } }")],
    );
    let messages = errors(&compilation);
    assert!(
        messages
            .iter()
            .any(|message| message.starts_with("`lib.musa` does not compile")),
        "expected the library's name in the message, got {messages:?}"
    );
}

#[test]
fn an_unknown_standard_module_reports_its_stable_virtual_uri() {
    let compilation = compile_with("p.musa", &piece("import std::unknown; motif rise() { c5/1 }"), &[]);
    let messages = errors(&compilation);
    assert!(
        messages
            .iter()
            .any(|message| message.contains("musa-stdlib:/std/unknown.musa")),
        "expected the virtual URI in the diagnostic, got {messages:?}"
    );
}
