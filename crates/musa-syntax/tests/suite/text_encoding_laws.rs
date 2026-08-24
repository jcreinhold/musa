//! `Text`'s exact encoding
//! (`docs/rules/language/02-core-calculus.md` §1.1).
//!
//! A base type is only as exact as the way it is written down and read back.
//! `quote` writes a text as a source literal and `unquote` reads one, and the
//! pair has to be an encoding rather than a rendering: whatever a composer
//! typed, writing it out and reading it back is the same text, and two texts
//! that differ are written differently. Without that, a text value in a
//! document and the same text value round-tripped through the source would be
//! two values the compiler believed were one.

use musa_syntax::ast::{quote, unquote};
use musa_syntax::{SyntaxKind, lex, parse};
use proptest::prelude::*;

/// Texts drawn from the characters that make encoding hard: the three the
/// escape rule is about, plus prose, whitespace, and one character outside
/// the Latin alphabet — a title is prose, and prose is not ASCII.
///
/// The line feed is here because it is the character the lexer's string body
/// refuses raw, so it is the one that separates an encoding from a rendering:
/// a `quote` that wrote it literally satisfies the round trip and injectivity
/// below and produces a literal the reader cannot read, which is what the
/// third law is for.
fn any_text() -> impl Strategy<Value = String> {
    prop::collection::vec(
        prop_oneof![
            Just('"'),
            Just('\\'),
            Just('\n'),
            Just(' '),
            Just('\t'),
            Just('a'),
            Just('Z'),
            Just('7'),
            Just('é'),
            Just('中'),
            Just('—'),
        ],
        0..24,
    )
    .prop_map(|characters| characters.into_iter().collect())
}

proptest! {
    /// The round trip: reading back what was written is what was written.
    #[test]
    fn writing_a_text_out_and_reading_it_back_is_the_same_text(text in any_text()) {
        prop_assert_eq!(unquote(&quote(&text)), text);
    }

    /// Injectivity, which is the half a round trip alone does not give: two
    /// texts that differ are written differently, so equality on the written
    /// form is equality on the text.
    #[test]
    fn two_texts_are_written_alike_only_when_they_are_one_text(left in any_text(), right in any_text()) {
        prop_assert_eq!(quote(&left) == quote(&right), left == right);
    }

    /// The encoding is what the *lexer* reads, not a private convention: a
    /// quoted text is one string token, and a piece carrying it parses
    /// cleanly. A rendering that the reader could not read back would satisfy
    /// the two properties above and still be useless.
    #[test]
    fn a_written_text_is_one_token_the_lexer_reads(text in any_text()) {
        let written = quote(&text);
        let lexed = lex(&written);
        prop_assert!(lexed.errors().is_empty(), "{:?}", lexed.errors());
        let tokens: Vec<SyntaxKind> = lexed.tokens().iter().map(|token| token.kind).collect();
        prop_assert_eq!(&tokens, &[SyntaxKind::String]);

        let source = format!("piece \"law\" {{ let said: Text = {written}; }}");
        let document = parse(&source);
        prop_assert!(document.errors().is_empty(), "{:?}", document.errors());
        prop_assert_eq!(document.syntax().to_string(), source);
    }
}
