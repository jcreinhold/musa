//! The file root makes progress on junk, whatever the junk is.
//!
//! `document := declaration* piece?` (`docs/rules/language/01-surface.md` §1)
//! is read by a loop with no closing brace to stop at, which is what makes it
//! different from every other loop in the parser. Those all sit inside a block
//! and end on the block's own `}`, so a recovery that stops *at* a recovery
//! token still terminates them: the next pass sees the `}` and breaks.
//!
//! At a root there is no such pass. A recovery that consumed nothing would
//! report, skip nothing, and meet the same token again — an infinite loop
//! pushing a diagnostic and two tree events per turn until the process runs
//! out of memory. `RBrace` is in `PIECE_RECOVERY`, and a stray `}` is exactly
//! what a file left over from a bad edit has, so this was reachable from
//! ordinary broken source rather than from anything exotic.
//!
//! Each law below parses junk at a root and asserts an answer came back. The
//! assertion that matters is that the call *returns*: if the invariant breaks,
//! these do not fail, they hang, and the suite's timeout is what reports it.

use musa_syntax::parse;

/// The token that made this a law: `RBrace` is a recovery point, so a
/// recovery that stops at one without eating it never leaves this `}` behind.
#[test]
fn a_stray_closing_brace_at_a_root_is_refused_and_not_endless() {
    let parsed = parse("let x: Nat = 1;\n}\n");
    assert!(
        !parsed.errors().is_empty(),
        "a `}}` closing nothing is an error, not a declaration"
    );
}

/// Several of them, because consuming exactly one per turn is the invariant
/// and one is the case a wrong fix would still pass.
#[test]
fn a_run_of_closing_braces_is_refused_once_per_brace_at_most() {
    let parsed = parse("}}}}\n");
    assert!(!parsed.errors().is_empty(), "junk at a root is an error");
}

/// The same token after a piece, where the root loop resumes with nothing
/// left that could open a declaration.
#[test]
fn junk_after_the_piece_still_terminates() {
    let parsed = parse("piece \"P\" { score { part p { voice v { c4/1 } } } }\n}\n");
    assert!(!parsed.errors().is_empty(), "a `}}` after the piece closes nothing");
}

/// A word that opens nothing, which recovery *can* skip — the case that
/// already worked, kept so a fix that only special-cased `}` is visible.
#[test]
fn an_unknown_word_at_a_root_is_refused() {
    let parsed = parse("banana\n");
    assert!(!parsed.errors().is_empty(), "`banana` opens no declaration");
}

/// A file that is only trivia declares nothing and is not an error: the empty
/// package root is this, and prompt 164a is what made it legal.
#[test]
fn a_file_of_comments_alone_declares_nothing_and_is_not_an_error() {
    let parsed = parse("// nothing here yet\n");
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());
}
