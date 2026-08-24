//! The three shapes a musa file may be, and that the compiler reads all three.
//!
//! A `piece` sounds, a `library { … }` declares for others to import, and a
//! **module file** — `mod …;` and nothing else — is a node of a package's
//! module tree (`docs/rules/language/01-surface.md` §1, §6;
//! `docs/rules/language/04-templates-and-modules.md`). The parser has modelled
//! all three since the package tree landed; the elaborator dispatched on two,
//! so `stdlib/src/lib.musa` — a file the standard library cannot be built
//! without — was refused with *this file declares no piece*. These laws are
//! what keep the two counts equal.

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

/// A file that is none of the three is still refused — by the parser, which
/// was already reading the piece such a file owes. The elaborator's own
/// *declares no piece* sits behind that as a backstop, and `stdlib/src/lib.musa`
/// used to be its one live case.
#[test]
fn a_file_of_no_shape_is_still_refused() {
    let compilation = compiled("stray.musa", "let alone: Nat = 1;\n");
    assert!(compilation.has_errors());
    let said = messages(&compilation).join("\n");
    assert!(said.contains("`piece`"), "the refusal should ask for one: {said}");
}
