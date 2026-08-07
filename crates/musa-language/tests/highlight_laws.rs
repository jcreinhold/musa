//! The token table an editor is built from says what the lexer says.
//!
//! `SPELLINGS` and `TokenClass` exist so the source editor can set musa text
//! without a second, drifting copy of the language. These laws are what make
//! that claim true: every spelling lexes as the kind it claims, every kind
//! the lexer can produce has a class, and every keyword-ish kind is in the
//! table.

#![allow(clippy::panic)]

use musa_language::{SPELLINGS, SyntaxKind, TokenClass, lex};

/// The table is the lexer's own vocabulary, not a parallel guess at it.
#[test]
fn spellings_lex_as_their_kind() {
    for &(text, kind) in SPELLINGS {
        let lexed = lex(text);
        let tokens = lexed.tokens();
        assert_eq!(tokens.len(), 1, "`{text}` is not one token");
        let Some(token) = tokens.first() else {
            panic!("`{text}` lexed to nothing");
        };
        assert_eq!(token.kind, kind, "`{text}` lexes as {:?}", token.kind);
        assert!(lexed.errors().is_empty(), "`{text}` lexes with errors");
    }
}

/// Every kind the composer types as a fixed word or mark is in the table —
/// otherwise an editor built from it would treat a real keyword as a name.
#[test]
fn every_literal_kind_is_spelled() {
    for raw in 0_u16..=u16::MAX {
        let kind = SyntaxKind::from(raw);
        let literal = matches!(
            TokenClass::of(kind),
            Some(TokenClass::Keyword | TokenClass::Use | TokenClass::Unit | TokenClass::Punctuation)
        );
        if !literal {
            continue;
        }
        assert!(
            SPELLINGS.iter().any(|&(_, spelled)| spelled == kind),
            "{kind:?} has no spelling in SPELLINGS"
        );
    }
}

/// Two spellings of one kind, or one spelling of two kinds, would make the
/// table ambiguous to read in either direction.
#[test]
fn spellings_are_unique() {
    let mut texts: Vec<&str> = SPELLINGS.iter().map(|&(text, _)| text).collect();
    texts.sort_unstable();
    let count = texts.len();
    texts.dedup();
    assert_eq!(texts.len(), count, "two entries share a spelling");
}

/// Trivia and literals have classes too: an editor must be able to ask about
/// any token it is handed, not only the ones it expected.
#[test]
fn every_token_kind_has_a_class() {
    for kind in [
        SyntaxKind::Whitespace,
        SyntaxKind::LineComment,
        SyntaxKind::BlockComment,
        SyntaxKind::Identifier,
        SyntaxKind::Integer,
        SyntaxKind::Float,
        SyntaxKind::Rational,
        SyntaxKind::String,
        SyntaxKind::PitchLiteral,
        SyntaxKind::IntervalLiteral,
        SyntaxKind::Error,
    ] {
        assert!(TokenClass::of(kind).is_some(), "{kind:?} has no class");
    }
    // Node kinds are not tokens and must say so.
    assert_eq!(TokenClass::of(SyntaxKind::VoiceDecl), None);
}
