//! Semantic tokens: the language's own classification, delta-encoded.
//!
//! Highlighting must answer on half-typed source, which the session's facts
//! cannot describe — so this handler reads `musa-syntax`'s [`classify`],
//! which is total (the lexer is total and the parse recovers, so an
//! unrecognized span is still a token), never the facts. `classify` also
//! knows *where* a token stands: `harmony` in `import std::harmony;` is a
//! module name, and only a parse-informed pass can say so. The classes are
//! `musa-syntax`'s [`TokenClass`]: adding a token kind without classifying
//! it does not compile there, so this legend cannot learn a word the lexer
//! does not know.

use lsp_types::{SemanticToken, SemanticTokenType, SemanticTokens, SemanticTokensResult};
use musa_project::{EventsTokenClass, Span, events_classify};
use musa_syntax::{DocumentAlternative, SyntaxKind, TokenClass, classify};

use crate::workspace::Document;

/// The token types the server can emit, in legend order. Indices into this
/// list are what the token data carries.
///
/// Standard types where one fits; one custom type — `unit` — because the
/// language's units are part of its syntax (roadmap §7: "units are part of
/// the syntax") and no standard type says so. A client that does not know
/// `unit` falls back to its own highlighting for it.
pub(crate) fn legend() -> Vec<SemanticTokenType> {
    vec![
        SemanticTokenType::COMMENT,
        SemanticTokenType::KEYWORD,
        SemanticTokenType::MACRO,
        SemanticTokenType::ENUM_MEMBER,
        SemanticTokenType::NUMBER,
        SemanticTokenType::STRING,
        SemanticTokenType::VARIABLE,
        SemanticTokenType::OPERATOR,
        SemanticTokenType::new("unit"),
        SemanticTokenType::TYPE,
    ]
}

/// The legend index of a token class, or `None` for a class the protocol is
/// better off without: whitespace is no one's token, and an invalid span is
/// the client's own squiggle's business, not a color's.
fn type_index(class: TokenClass) -> Option<u32> {
    let index = match class {
        TokenClass::Comment => 0,
        TokenClass::Keyword => 1,
        TokenClass::Use => 2,
        TokenClass::Pitch => 3,
        TokenClass::Duration | TokenClass::Number => 4,
        TokenClass::Text => 5,
        TokenClass::Name => 6,
        TokenClass::Punctuation => 7,
        TokenClass::Unit => 8,
        TokenClass::Invalid => return None,
    };
    Some(index)
}

/// The legend index of an event track token class.
///
/// A separate map rather than a conversion into [`TokenClass`], because the
/// two languages classify different things and a lossy translation between
/// them would colour events text as an approximation of musa text. They meet
/// at the legend, which is where they should: `Comment` is a comment in both.
fn events_type_index(class: EventsTokenClass) -> u32 {
    match class {
        EventsTokenClass::Comment => 0,
        EventsTokenClass::Keyword => 1,
        EventsTokenClass::Number => 4,
        EventsTokenClass::Text => 5,
        EventsTokenClass::Name => 6,
        EventsTokenClass::Punctuation => 7,
        EventsTokenClass::Type => 9,
    }
}

/// Every classified token of the document, delta-encoded as the protocol
/// prescribes. Valid source or not — both lexers are total, so this is total.
pub(crate) fn full(document: &Document) -> SemanticTokensResult {
    let snapshot = document.snapshot();
    let source = snapshot.source();
    let lines = document.lines();
    let mut data = Vec::new();
    let mut previous_line = 0_u32;
    let mut previous_start = 0_u32;
    if document.alternative() == DocumentAlternative::Events {
        for (range, class) in events_classify(source) {
            let span = Span {
                start: u32::try_from(range.start).unwrap_or(u32::MAX),
                end: u32::try_from(range.end).unwrap_or(u32::MAX),
            };
            for (line, start, length) in lines.lines_of(span) {
                push(
                    &mut data,
                    line,
                    start,
                    length,
                    events_type_index(class),
                    &mut previous_line,
                    &mut previous_start,
                );
            }
        }
        return SemanticTokensResult::Tokens(SemanticTokens { result_id: None, data });
    }
    for (token, class) in classify(source) {
        if token.kind == SyntaxKind::Whitespace {
            continue;
        }
        let Some(token_type) = class.and_then(type_index) else {
            continue;
        };
        let span = Span {
            start: u32::from(token.range.start()),
            end: u32::from(token.range.end()),
        };
        // One LSP token occupies one line, so the block comment — the
        // language's only multi-line token — is told line by line.
        for (line, start, length) in lines.lines_of(span) {
            push(
                &mut data,
                line,
                start,
                length,
                token_type,
                &mut previous_line,
                &mut previous_start,
            );
        }
    }
    SemanticTokensResult::Tokens(SemanticTokens { result_id: None, data })
}

/// One token, as a delta from the one before it.
fn push(
    data: &mut Vec<SemanticToken>,
    line: u32,
    start: u32,
    length: u32,
    token_type: u32,
    previous_line: &mut u32,
    previous_start: &mut u32,
) {
    let delta_line = line.saturating_sub(*previous_line);
    let delta_start = if delta_line == 0 {
        start.saturating_sub(*previous_start)
    } else {
        start
    };
    data.push(SemanticToken {
        delta_line,
        delta_start,
        length,
        token_type,
        token_modifiers_bitset: 0,
    });
    *previous_line = line;
    *previous_start = start;
}
