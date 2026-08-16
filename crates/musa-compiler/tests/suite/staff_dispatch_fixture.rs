//! Prompt 132's staff-dispatch program, compiled (prompt 140).
//!
//! The trial in
//! `docs/notes/research/language-design-closure/43-dependent-language-trial.md`
//! §2 rewrote `stdlib/src/adapters/staff.musa`'s dispatch table before any code
//! implemented a quote pattern, and reached a conclusion about quote patterns
//! that is negative: **no quote pattern appears anywhere in the staff
//! adapter**, because a pattern is written in Musa and the notation the adapter
//! reads is not. Prompt 140's Target makes that program a fixture, so the
//! conclusion is checked rather than remembered.
//!
//! The adapter is `tests/fixtures/staff-dispatch.musa`, handed to the compiler
//! the way any package adapter is. Three things are checked and they are
//! different in kind: that the program compiles and expands, that a sixteenth
//! staff word does not compile until it is read, and that the file asks each of
//! its three questions the way the trial says it does.
//!
//! [`syntax_pattern_laws`](super::syntax_pattern_laws) has the other half of
//! the same argument — the two small programs that say which form is for which
//! job. This is what a real adapter does with the answer.

// A failure is more useful reported with what actually happened than with an
// assertion message alone.
#![allow(clippy::panic)]

use musa_compiler::{CompileOptions, ImportSources, Severity, SourceDocument, compile, resolve_import};

/// The trial's program.
const DISPATCH: &str = include_str!("../../../../tests/fixtures/staff-dispatch.musa");

/// Where the piece imports it from.
const STAFF: &str = "trial::staff";

/// A piece that declares what the dispatch answers with and reads one region.
///
/// The vocabulary is the count: fifteen staff words reach nine readers, and the
/// eleven nullary constructors below are what those readers and the token half
/// name between them. Annotating `read` is the observation — a dispatch that
/// fell through to the wrong arm is an expression this piece cannot have
/// annotated this way.
fn piece(region: &str) -> String {
    format!(
        r#"piece "staff dispatch" {{
    import syntax {STAFF} as staff;

    data StaffRead {{
        NoItems,
        Unknown,
        Rested,
        Barred,
        Slurred,
        Tupleted,
        Graced,
        Repeated,
        Timed,
        Metered,
        Stated,
        Named,
        Keyed,
        Tied,
        Dotted,
        Numbered,
        Sung,
        Body(items: List<StaffRead>),
        Stating(items: List<StaffRead>),
        Voiced(items: List<StaffRead>),
    }}

    let read: StaffRead = syntax staff {{ {region} }};

    score {{ part p {{ voice v {{ c4/1 }} }} }}
}}
"#
    )
}

/// Every error compiling that piece against `module`, as the whole small
/// document each one is, and every **cause** with it.
///
/// A fault the checker finds *inside* the fixture module is a diagnostic about
/// another document, and arrives as a cause of the diagnostic about the import
/// rather than being summarized into it — which is what lets a law here name
/// the coverage complaint the fixture is for.
fn errors(region: &str, module: &str) -> Vec<String> {
    let source = SourceDocument::new(piece(region), "staff-dispatch.musa");
    let mut imports = ImportSources::default();
    imports.insert(resolve_import("staff-dispatch.musa", STAFF), module.to_owned());
    compile(
        &source,
        &CompileOptions {
            imports,
            ..CompileOptions::default()
        },
    )
    .diagnostics()
    .iter()
    .filter(|diagnostic| diagnostic.severity == Severity::Error)
    .flat_map(|diagnostic| {
        let mut found = vec![whole(
            &diagnostic.message,
            diagnostic.note.as_deref(),
            diagnostic.help.as_deref(),
        )];
        found.extend(
            diagnostic
                .causes
                .iter()
                .map(|cause| whole(&cause.message, cause.note.as_deref(), cause.help.as_deref())),
        );
        found
    })
    .collect()
}

/// One diagnostic, or one cause, as the small document it is.
fn whole(message: &str, note: Option<&str>, help: Option<&str>) -> String {
    let mut lines = vec![message.to_owned()];
    if let Some(note) = note {
        lines.push(format!("note: {note}"));
    }
    if let Some(help) = help {
        lines.push(format!("help: {help}"));
    }
    lines.join("\n")
}

/// Lines of the fixture that are not comments.
fn code() -> impl Iterator<Item = &'static str> {
    DISPATCH.lines().filter(|line| !line.trim_start().starts_with("//"))
}

#[test]
fn the_trials_staff_dispatch_program_compiles() {
    // All three halves at once: a region whose leaves are a notation keyword,
    // a pitch, a numeral, and a tie, inside two of the three delimiters.
    let found = errors("bar { c5 ~ 4 } (3, 2)", DISPATCH);
    assert!(found.is_empty(), "the trial's program did not compile: {found:?}");
}

#[test]
fn a_word_the_notation_does_not_have_reaches_one_arm() {
    // The `None` of the lookup, which is the only arm an unknown word can
    // reach. `bra` is the trial's own example of the failure a fifteen-way
    // chain of literals has and a lookup does not hide: it is a perfectly good
    // identifier, so nothing but this arm can catch it.
    let found = errors("bra", DISPATCH);
    assert!(
        found.is_empty(),
        "an unknown word did not reach the last arm: {found:?}"
    );
}

#[test]
fn a_sixteenth_staff_word_does_not_compile_until_it_is_read() {
    // The trial's claim about what the lookup buys, checked the only way a
    // claim about coverage can be: by adding the word and watching the arm
    // count go wrong. `word_read` is exhaustive over a declared type, so the
    // sixteenth constructor is a hole in it — where the string version's `_ ->`
    // would have swallowed it.
    let extended = DISPATCH.replace("        Key,\n    }", "        Key,\n        Caesura,\n    }");
    assert!(
        extended != DISPATCH,
        "the sixteenth-word edit did not apply; `StaffWord` was respelled"
    );
    let found = errors("bar", &extended);
    assert!(
        found.iter().any(|error| error.contains("uncovered")),
        "a sixteenth staff word compiled without being read: {found:?}"
    );
}

#[test]
fn the_adapter_matches_no_shape_and_spells_no_keyword_twice() {
    // The trial's conclusion, read off the file. Two claims:
    //
    // - **No quote pattern.** The form prompt 140 adds does not appear here,
    //   and the header says why: it describes Musa's shapes and this adapter
    //   reads staff notation's.
    // - **Every `text_equal` is inside `staff_word`.** That is the containment
    //   the lookup was written for — twenty-one sites over thirteen literals
    //   became fifteen sites in one function, where a misspelling is visible.
    let matching: Vec<&str> = code().filter(|line| line.contains("quote {")).collect();
    assert!(
        matching.is_empty(),
        "the staff dispatch matched a Musa shape after all: {matching:?}"
    );

    let mut inside = false;
    let stray: Vec<&str> = code()
        .filter(|line| {
            if line.starts_with("    fn staff_word(") {
                inside = true;
            } else if line.starts_with("    fn ") {
                inside = false;
            }
            !inside && line.contains("text_equal")
        })
        .collect();
    assert!(
        stray.is_empty(),
        "a staff keyword is spelled outside `staff_word`: {stray:?}"
    );
}
