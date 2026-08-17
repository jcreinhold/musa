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

/// The class of a module-name word is decided where the word stands.
///
/// `MODULE_NAME` (parser.rs) lets a module be named after a type or a domain
/// keyword, so `harmony` in `import std::harmony;` is a name and `harmony`
/// opening a harmony lane is a keyword — the same spelling, two classes,
/// distinguished by position. These laws pin the distinction in both
/// directions for every keyword the module namespace may borrow.
#[test]
fn module_names_are_names_not_keywords() {
    let cases: &[(&str, &str)] = &[
        ("import std::harmony;", "harmony"),
        ("import std::list;", "list"),
        ("import std::pitch;", "pitch"),
        ("import std::pitch;", "pitch"),
        ("import std::scale;", "scale"),
        ("import std::tonal::harmony;", "harmony"),
        ("mod harmony;", "harmony"),
        ("mod list;", "list"),
        ("mod option;", "option"),
        ("mod pitch;", "pitch"),
        ("mod scale;", "scale"),
    ];
    for &(source, word) in cases {
        let classified = musa_language::classify(source);
        let hits: Vec<_> = classified
            .iter()
            .filter(|(token, _)| &source[usize::from(token.range.start())..usize::from(token.range.end())] == word)
            .collect();
        let Some(first) = hits.first() else {
            panic!("`{source}`: `{word}` is not one token");
        };
        assert_eq!(hits.len(), 1, "`{source}`: `{word}` is not one token");
        assert_eq!(
            first.1,
            Some(TokenClass::Name),
            "`{source}`: `{word}` is a module name here, not a keyword"
        );
    }
}

/// The same words in keyword positions keep the keyword class — the
/// reclassification may not leak out of module-name positions.
///
/// A type name is here too, travelling the other way: `Pitch` is an
/// identifier to the lexer, and the `TypeName` position is what makes it
/// vocabulary rather than something the composer named.
#[test]
fn the_borrowed_words_stay_keywords_in_keyword_positions() {
    let cases: &[(&str, &str)] = &[
        ("piece \"P\" { harmony { at 1:1 C; } }", "harmony"),
        ("piece \"P\" { let xs: List<Pitch> = []; }", "List"),
        ("piece \"P\" { let x: Option<Pitch> = None; }", "Option"),
        ("piece \"P\" { let p: Pitch = c4; }", "Pitch"),
        ("piece \"P\" { let s: Scale = scale c dorian; }", "scale"),
    ];
    for &(source, word) in cases {
        let classified = musa_language::classify(source);
        let hits: Vec<_> = classified
            .iter()
            .filter(|(token, _)| &source[usize::from(token.range.start())..usize::from(token.range.end())] == word)
            .collect();
        assert!(!hits.is_empty(), "`{source}`: `{word}` never lexed");
        for (_, class) in &hits {
            assert_eq!(
                *class,
                Some(TokenClass::Keyword),
                "`{source}`: `{word}` is a keyword here"
            );
        }
    }
}

/// A half-typed import still classifies: the LSP answers on broken source,
/// so `classify` must be total where the parse recovers.
#[test]
fn classify_is_total_on_recovering_source() {
    let source = "import std::harmony\nmod";
    let classified = musa_language::classify(source);
    // Every produced token has a verdict — totality means no gaps, and the
    // recovered import path still reads as a name.
    assert!(classified.iter().all(|(_, class)| class.is_some()));
    let harmony = classified
        .iter()
        .find(|(token, _)| &source[usize::from(token.range.start())..usize::from(token.range.end())] == "harmony");
    assert_eq!(
        harmony.map(|(_, class)| *class),
        Some(Some(TokenClass::Name)),
        "a recovered import path still classifies its module names"
    );
}
