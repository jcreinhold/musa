//! Prompt 132's staff-construction program, compiled (prompt 139).
//!
//! The trial in
//! `docs/notes/research/language-design-closure/43-dependent-language-trial.md`
//! §1 rewrote `stdlib/src/adapters/staff.musa`'s emitting section and its call
//! sites on `quote at here { … }` before any code implemented one. Prompt
//! 139's Read makes that program the prompt's own acceptance condition: if it
//! does not compile at the end, the prompt is not done.
//!
//! So this runs it. The adapter is
//! `tests/fixtures/staff-construction.musa`, handed to the compiler the way
//! any package adapter is, and a piece reads one region with it and annotates
//! what came back. Two things are checked and they are different in kind: that
//! the program compiles and expands, and that it contains none of the
//! machinery it was supposed to remove.

// A failure is more useful reported with what actually happened than with an
// assertion message alone.
#![allow(clippy::panic)]

use musa_compiler::{CompileOptions, ImportSources, SourceDocument, compile, resolve_import};

use musa_score::Severity;

/// The trial's program.
const CONSTRUCTION: &str = include_str!("../../../../tests/fixtures/staff-construction.musa");

/// Where the piece imports it from.
const STAFF: &str = "trial::staff";

/// A piece that declares the vocabulary the adapter emits and reads one region
/// with it.
///
/// The declarations are the package's side of the same program: `Sounded`,
/// `Note`, `Rest`, and `Tuplet` are what `staff.musa`'s call sites name, and
/// the annotation on `read` is the observation — an expansion that dropped a
/// splice, or built a call of the wrong arity, is an expression this piece
/// cannot have annotated that way.
fn piece(region: &str) -> String {
    format!(
        r#"piece "staff construction" {{
    import syntax {STAFF} as staff;

    data Tie {{
        Untied,
        TiedOn,
    }}

    data WrittenValue {{
        NoteValue(division: Nat, dots: Nat),
        ExactSpan(division: Nat),
    }}

    data StaffItem {{
        NoItems,
        Note(sung: Pitch, value: WrittenValue, tie: Tie),
        Rest(value: WrittenValue),
        Sounded(place: Nat, event: StaffItem, items: List(StaffItem)),
        Tuplet(
            place: Nat,
            played: Nat,
            against: Nat,
            items: List(StaffItem),
            after: StaffItem,
        ),
    }}

    let read: StaffItem = syntax staff {{ {region} }};

    score {{ part p {{ voice v {{ c4/1 }} }} }}
}}
"#
    )
}

/// Every error compiling that piece against the trial's adapter, as the whole
/// small document each one is.
fn errors(region: &str) -> Vec<String> {
    let source = SourceDocument::new(piece(region), "staff-construction.musa");
    let mut imports = ImportSources::default();
    imports.insert(
        resolve_import("staff-construction.musa", STAFF),
        CONSTRUCTION.to_owned(),
    );
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

#[test]
fn the_trials_staff_construction_program_compiles() {
    // The whole point of the fixture. Every definition the trial's §1.1 wrote
    // is checked, and the four call sites of §1.2 are evaluated over a real
    // region — including the one the trial said does not go through, whose
    // pitch is the composer's own node rather than a token rebuilt from its
    // spelling.
    let found = errors("c5 e5");
    assert!(found.is_empty(), "the trial's program did not compile: {found:?}");
}

#[test]
fn a_group_inside_the_region_reaches_the_other_call_site() {
    // The parenthesised half of the dispatch, so `tuplet` is evaluated too and
    // its five spliced positions arrive with the separators the argument list
    // supplies. A nested group also puts two `anchored` calls in one expansion
    // at two different places, which is what the identity law has to hold for.
    let found = errors("c5 (3 2)");
    assert!(
        found.is_empty(),
        "a region with a group inside it did not compile: {found:?}"
    );
}

#[test]
fn the_program_names_no_builder_and_allocates_no_role() {
    // The trial's first two claims, checked against the file rather than
    // against a reading of it: `named` and `call1`–`call7` are gone and
    // nothing replaces them, and every hand-allocated role integer goes with
    // them because nothing left can take one.
    //
    // Reading the source is the right observation here. These operations are
    // still in the language — prompt 166 is where `staff.musa` stops using
    // them — so what this says is that the *rewritten* program has no use for
    // any of them, which is the measurement prompt 166 is gated on.
    let retired = [
        "syntax_built",
        "syntax_token",
        "syntax_group",
        "syntax_identifier",
        "syntax_binding",
        "syntax_binder",
        "syntax_reference",
        "call1",
        "call2",
        "call3",
        "call4",
        "call5",
        "call6",
        "call7",
    ];
    let written: Vec<&str> = retired
        .into_iter()
        .filter(|operation| {
            CONSTRUCTION
                .lines()
                .filter(|line| !line.trim_start().starts_with("//"))
                .any(|line| line.contains(operation))
        })
        .collect();
    assert!(
        written.is_empty(),
        "the rewritten construction section still assembles syntax by hand: {written:?}"
    );
}
