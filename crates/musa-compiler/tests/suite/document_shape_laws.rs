//! One document form, and that the compiler reads every file through it.
//!
//! `document := declaration* piece?` (`docs/rules/language/01-surface.md` §1),
//! so a file is not one of several shapes chosen by lookahead — it is a list of
//! declarations that may end with a piece. What a file *turned out to be* is an
//! observation made afterwards: it has a piece, or it has none, or the
//! declarations it holds are all `mod`. [`DocumentKind`] reports that
//! observation; it does not select a grammar.
//!
//! These laws hold the two counts equal. Before prompt 164a the elaborator
//! dispatched on two shapes where the parser modelled three, so
//! `stdlib/src/lib.musa` — a file the standard library cannot be built
//! without — was refused with *this file declares no piece*.

use musa_compiler::{CompileOptions, DocumentKind, SourceDocument, compile};

/// The standard library's own root file, so the law is about the real tree
/// rather than a miniature of it.
const STDLIB_ROOT: &str = include_str!("../../../../stdlib/src/lib.musa");

fn compiled(name: &str, text: &str) -> musa_compiler::Compilation {
    compile(&SourceDocument::new(text, name), &CompileOptions::default())
}

fn messages(compilation: &musa_compiler::Compilation) -> Vec<String> {
    compilation
        .diagnostics()
        .iter()
        .map(|diagnostic| diagnostic.message.clone())
        .collect()
}

/// A module file compiles as itself: no piece owed, no score produced, and
/// nothing to complain about.
#[test]
fn a_module_file_is_a_document_of_its_own() {
    let compilation = compiled("lib.musa", "mod core;\nmod tonal;\n");
    assert_eq!(compilation.kind(), DocumentKind::Modules);
    assert!(!compilation.has_errors(), "{:?}", messages(&compilation));
    assert!(
        compilation.snapshot().is_none(),
        "a module tree is not a score, and having none is not a failure"
    );
}

/// The file this whole law exists for.
#[test]
fn the_standard_librarys_root_file_checks() {
    let compilation = compiled("stdlib/src/lib.musa", STDLIB_ROOT);
    assert_eq!(compilation.kind(), DocumentKind::Modules);
    assert!(!compilation.has_errors(), "{:?}", messages(&compilation));
}

/// A comment above the declarations is trivia, not a fourth shape: the
/// standard library's root file opens with six lines of them.
#[test]
fn comments_do_not_make_a_module_file_something_else() {
    let compilation = compiled("mod.musa", "// The tree.\nmod harmony;\n");
    assert_eq!(compilation.kind(), DocumentKind::Modules);
    assert!(!compilation.has_errors(), "{:?}", messages(&compilation));
}

/// A module file is a path segment and not a module of its own, so a binding
/// beside the `mod`s is unreachable — and refused by a diagnostic that says
/// what a module file is rather than asking for a piece.
#[test]
fn a_module_file_declares_modules_and_nothing_else() {
    let compilation = compiled("lib.musa", "mod core;\nlet stray: Nat = 1;\n");
    assert!(compilation.has_errors(), "an unreachable binding was accepted");
    let said = messages(&compilation).join("\n");
    assert!(
        said.contains("module file"),
        "the refusal should name the shape: {said}"
    );
    assert!(
        !said.contains("declares no piece"),
        "and should not ask a module file for a piece: {said}"
    );
}

/// A file that declares and never sounds is a document, not a failure.
///
/// The law prompt 164a inverted. `let alone: Nat = 1;` used to be a file of no
/// shape, refused by the parser for the `piece` it owed; there is no such debt
/// now, because a piece is the *optional* tail of the one production and a file
/// that omits it has omitted nothing.
#[test]
fn a_file_that_declares_and_never_sounds_is_a_document() {
    let compilation = compiled("stray.musa", "let alone: Nat = 1;\n");
    assert!(!compilation.has_errors(), "{:?}", messages(&compilation));
    assert_eq!(compilation.kind(), DocumentKind::Material);
    assert!(
        compilation.snapshot().is_none(),
        "nothing sounded, and having sounded nothing is not a failure"
    );
}

/// The same file with a piece under the same declarations, read by the same
/// path — which is the whole claim of one document form.
#[test]
fn declarations_and_a_piece_are_the_one_production() {
    let both = concat!(
        "let alone: Nat = 1;\n",
        "\n",
        "piece \"Both\" {\n",
        "    tempo 1/4 = 60;\n",
        "    meter 4/4;\n",
        "    key c major;\n",
        "    score { part p { voice v { c5/1 } } }\n",
        "}\n",
    );
    let compilation = compiled("both.musa", both);
    assert!(!compilation.has_errors(), "{:?}", messages(&compilation));
    assert_eq!(compilation.kind(), DocumentKind::Piece);
    assert!(compilation.snapshot().is_some(), "the piece at the tail sounds");
}
