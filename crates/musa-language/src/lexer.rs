//! The lexer: `logos`-based tokenization with full trivia preservation.
//!
//! Lexing never aborts: unrecognized text, unterminated strings, and
//! unterminated block comments produce an [`SyntaxKind::Error`] token (so the
//! token stream always covers the whole source) plus a [`LexError`], and
//! scanning continues. This is the front end of the always-returns-a-tree
//! policy of roadmap §10.3.

use std::ops::Range;

use logos::Logos;
use text_size::{TextRange, TextSize};

use crate::SyntaxKind;

/// Lex `source` into tokens. Trivia tokens (whitespace, comments) are
/// emitted, never discarded.
///
/// Invariant: concatenating the source slices of the returned tokens
/// reproduces `source` exactly, even when [`Lexed::errors`] is non-empty.
pub fn lex(source: &str) -> Lexed {
    let mut tokens = Vec::new();
    let mut errors = Vec::new();
    let mut lexer = RawToken::lexer(source);
    while let Some(result) = lexer.next() {
        let span = lexer.span();
        match result {
            Ok(raw) => {
                let range = text_range(span);
                tokens.push(Token {
                    kind: raw.kind(),
                    range,
                });
                if let Some(error_kind) = raw.error_kind() {
                    errors.push(LexError {
                        range,
                        kind: error_kind,
                    });
                }
            }
            Err(()) => {
                let range = text_range(span);
                tokens.push(Token {
                    kind: SyntaxKind::Error,
                    range,
                });
                errors.push(LexError {
                    range,
                    kind: LexErrorKind::InvalidToken,
                });
            }
        }
    }
    Lexed { tokens, errors }
}

/// The result of lexing a source string.
#[derive(Debug)]
pub struct Lexed {
    tokens: Vec<Token>,
    errors: Vec<LexError>,
}

impl Lexed {
    /// All tokens, in source order, including trivia and error tokens.
    pub fn tokens(&self) -> &[Token] {
        &self.tokens
    }

    /// Lexical errors with source spans, in source order. Empty when the
    /// source lexed cleanly.
    pub fn errors(&self) -> &[LexError] {
        &self.errors
    }
}

/// A single token: a kind plus its byte range in the source.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Token {
    /// What kind of token this is.
    pub kind: SyntaxKind,
    /// Byte range of the token's text in the source.
    pub range: TextRange,
}

/// A lexical error: what went wrong and where.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
#[error("{kind} at byte range {}..{}", usize::from(.range.start()), usize::from(.range.end()))]
pub struct LexError {
    range: TextRange,
    kind: LexErrorKind,
}

impl LexError {
    /// Byte range of the offending text.
    pub fn range(self) -> TextRange {
        self.range
    }

    /// What kind of lexical error this is.
    pub fn kind(self) -> LexErrorKind {
        self.kind
    }
}

/// The category of a lexical error.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum LexErrorKind {
    /// Bytes that match no token class.
    #[error("unrecognized token")]
    InvalidToken,
    /// A `"` with no closing quote before end of line or input.
    #[error("unterminated string literal")]
    UnterminatedString,
    /// A `/*` with no closing `*/`.
    #[error("unterminated block comment")]
    UnterminatedBlockComment,
}

/// Convert a byte span into a `TextRange`, clamping at `u32::MAX` (sources
/// past 4 GiB are far outside this language's design envelope).
fn text_range(span: Range<usize>) -> TextRange {
    let size = |offset: usize| TextSize::from(u32::try_from(offset).unwrap_or(u32::MAX));
    TextRange::new(size(span.start), size(span.end))
}

/// Raw logos tokens. Each variant maps one-to-one onto [`SyntaxKind`]; the
/// split exists so the derive machinery stays out of the public API.
#[derive(Clone, Copy, Debug, Logos)]
enum RawToken {
    #[regex(r"[ \t\r\n]+")]
    Whitespace,
    #[regex(r"//[^\n]*")]
    LineComment,
    #[regex(r"(?s)/\*([^*]|\*[^/])*\*/")]
    BlockComment,

    // String bodies exclude newlines in *both* patterns: an unterminated
    // string dies at end of line instead of swallowing the rest of the file
    // (logos falls back to the last accepted match — the unterminated
    // pattern — only when no pattern's path is still alive).
    #[regex(r#""([^"\\\n]|\\[^\n])*""#)]
    String,
    /// A `"` that never closes before end of line or input. Longest-match
    /// rules mean a properly terminated string always wins over this.
    #[regex(r#""([^"\\\n]|\\[^\n])*"#)]
    UnterminatedString,
    /// A `/*` that never closes. The body pattern stops before any `*/`, so
    /// a properly terminated comment always wins by longest match.
    #[regex(r"(?s)/\*([^*]|\*[^/])*")]
    UnterminatedBlockComment,
    #[regex(r"[a-g](ss|ff|[sfn])?-?[0-9]+")]
    PitchLiteral,
    #[regex(r"[PMm][0-9]+")]
    IntervalLiteral,
    #[regex(r"[0-9]+\.[0-9]+")]
    Float,
    #[regex(r"[0-9]+/[0-9]+")]
    Rational,
    #[regex(r"[0-9]+")]
    Integer,
    #[regex(r"[a-zA-Z_]+")]
    Identifier,

    #[token("Hz", priority = 3)]
    UnitHz,
    #[token("ms", priority = 3)]
    UnitMs,
    #[token("s", priority = 3)]
    UnitS,
    #[token("dB", priority = 3)]
    UnitDb,
    #[token("bpm", priority = 3)]
    UnitBpm,

    #[token("{")]
    LBrace,
    #[token("}")]
    RBrace,
    #[token("[")]
    LBracket,
    #[token("]")]
    RBracket,
    #[token("(")]
    LParen,
    #[token(")")]
    RParen,
    #[token(";")]
    Semicolon,
    #[token(",")]
    Comma,
    #[token(":")]
    Colon,
    #[token("->")]
    Arrow,
    #[token("|>")]
    PipeForward,
    #[token("=")]
    Equals,
    #[token("-")]
    Minus,
    #[token("~")]
    Tilde,
    #[token(".")]
    Dot,

    #[token("piece", priority = 3)]
    PieceKw,
    #[token("tempo", priority = 3)]
    TempoKw,
    #[token("meter", priority = 3)]
    MeterKw,
    #[token("key", priority = 3)]
    KeyKw,
    #[token("subtitle", priority = 3)]
    SubtitleKw,
    #[token("composer", priority = 3)]
    ComposerKw,
    #[token("arranger", priority = 3)]
    ArrangerKw,
    #[token("copyright", priority = 3)]
    CopyrightKw,
    #[token("motif", priority = 3)]
    MotifKw,
    #[token("score", priority = 3)]
    ScoreKw,
    #[token("part", priority = 3)]
    PartKw,
    #[token("voice", priority = 3)]
    VoiceKw,
    #[token("clef", priority = 3)]
    ClefKw,
    #[token("use", priority = 3)]
    UseKw,
    #[token("transpose", priority = 3)]
    TransposeKw,
    #[token("down", priority = 3)]
    DownKw,
    #[token("up", priority = 3)]
    UpKw,
    #[token("rest", priority = 3)]
    RestKw,
    #[token("chord", priority = 3)]
    ChordKw,
    #[token("repeat", priority = 3)]
    RepeatKw,
    #[token("slur", priority = 3)]
    SlurKw,
    #[token("dynamic", priority = 3)]
    DynamicKw,
    #[token("tuplet", priority = 3)]
    TupletKw,
    #[token("performance", priority = 3)]
    PerformanceKw,
    #[token("profile", priority = 3)]
    ProfileKw,
    #[token("articulation", priority = 3)]
    ArticulationKw,
    #[token("studio", priority = 3)]
    StudioKw,
    #[token("patch", priority = 3)]
    PatchKw,
    #[token("modulate", priority = 3)]
    ModulateKw,
    #[token("bus", priority = 3)]
    BusKw,
    #[token("assign", priority = 3)]
    AssignKw,
    #[token("route", priority = 3)]
    RouteKw,
    #[token("send", priority = 3)]
    SendKw,
    #[token("master", priority = 3)]
    MasterKw,
    #[token("at", priority = 3)]
    AtKw,
    #[token("output", priority = 3)]
    OutputKw,
    #[token("pitch", priority = 3)]
    PitchKw,
    #[token("stretch", priority = 3)]
    StretchKw,
    #[token("retrograde", priority = 3)]
    RetrogradeKw,
    #[token("invert", priority = 3)]
    InvertKw,
    #[token("around", priority = 3)]
    AroundKw,
    #[token("with", priority = 3)]
    WithKw,
    #[token("note", priority = 3)]
    NoteKw,
    #[token("phrase", priority = 3)]
    PhraseKw,
    #[token("section", priority = 3)]
    SectionKw,
    #[token("harmony", priority = 3)]
    HarmonyKw,
    #[token("library", priority = 3)]
    LibraryKw,
    #[token("crescendo", priority = 3)]
    CrescendoKw,
    #[token("diminuendo", priority = 3)]
    DiminuendoKw,
    #[token("to", priority = 3)]
    ToKw,
    #[token("bar", priority = 3)]
    BarKw,
}

impl RawToken {
    /// Unterminated constructs are tokens (so the stream stays lossless)
    /// *and* errors; this returns the error kind for those variants.
    fn error_kind(self) -> Option<LexErrorKind> {
        match self {
            Self::UnterminatedString => Some(LexErrorKind::UnterminatedString),
            Self::UnterminatedBlockComment => Some(LexErrorKind::UnterminatedBlockComment),
            Self::Whitespace
            | Self::LineComment
            | Self::BlockComment
            | Self::String
            | Self::PitchLiteral
            | Self::IntervalLiteral
            | Self::Float
            | Self::Rational
            | Self::Integer
            | Self::Identifier
            | Self::UnitHz
            | Self::UnitMs
            | Self::UnitS
            | Self::UnitDb
            | Self::UnitBpm
            | Self::LBrace
            | Self::RBrace
            | Self::LBracket
            | Self::RBracket
            | Self::LParen
            | Self::RParen
            | Self::Semicolon
            | Self::Comma
            | Self::Colon
            | Self::Arrow
            | Self::PipeForward
            | Self::Equals
            | Self::Minus
            | Self::Tilde
            | Self::Dot
            | Self::PieceKw
            | Self::TempoKw
            | Self::MeterKw
            | Self::KeyKw
            | Self::SubtitleKw
            | Self::ComposerKw
            | Self::ArrangerKw
            | Self::CopyrightKw
            | Self::MotifKw
            | Self::ScoreKw
            | Self::PartKw
            | Self::VoiceKw
            | Self::ClefKw
            | Self::UseKw
            | Self::TransposeKw
            | Self::DownKw
            | Self::UpKw
            | Self::RestKw
            | Self::ChordKw
            | Self::RepeatKw
            | Self::SlurKw
            | Self::DynamicKw
            | Self::TupletKw
            | Self::PerformanceKw
            | Self::ProfileKw
            | Self::ArticulationKw
            | Self::StudioKw
            | Self::PatchKw
            | Self::ModulateKw
            | Self::BusKw
            | Self::AssignKw
            | Self::RouteKw
            | Self::SendKw
            | Self::MasterKw
            | Self::AtKw
            | Self::OutputKw
            | Self::PitchKw
            | Self::StretchKw
            | Self::RetrogradeKw
            | Self::InvertKw
            | Self::AroundKw
            | Self::WithKw
            | Self::NoteKw
            | Self::PhraseKw
            | Self::SectionKw
            | Self::HarmonyKw
            | Self::LibraryKw
            | Self::CrescendoKw
            | Self::DiminuendoKw
            | Self::ToKw
            | Self::BarKw => None,
        }
    }

    fn kind(self) -> SyntaxKind {
        match self {
            Self::Whitespace => SyntaxKind::Whitespace,
            Self::LineComment => SyntaxKind::LineComment,
            Self::BlockComment => SyntaxKind::BlockComment,
            Self::String => SyntaxKind::String,
            Self::UnterminatedString | Self::UnterminatedBlockComment => SyntaxKind::Error,
            Self::PitchLiteral => SyntaxKind::PitchLiteral,
            Self::IntervalLiteral => SyntaxKind::IntervalLiteral,
            Self::Float => SyntaxKind::Float,
            Self::Rational => SyntaxKind::Rational,
            Self::Integer => SyntaxKind::Integer,
            Self::Identifier => SyntaxKind::Identifier,
            Self::UnitHz => SyntaxKind::UnitHz,
            Self::UnitMs => SyntaxKind::UnitMs,
            Self::UnitS => SyntaxKind::UnitS,
            Self::UnitDb => SyntaxKind::UnitDb,
            Self::UnitBpm => SyntaxKind::UnitBpm,
            Self::LBrace => SyntaxKind::LBrace,
            Self::RBrace => SyntaxKind::RBrace,
            Self::LBracket => SyntaxKind::LBracket,
            Self::RBracket => SyntaxKind::RBracket,
            Self::LParen => SyntaxKind::LParen,
            Self::RParen => SyntaxKind::RParen,
            Self::Semicolon => SyntaxKind::Semicolon,
            Self::Comma => SyntaxKind::Comma,
            Self::Colon => SyntaxKind::Colon,
            Self::Arrow => SyntaxKind::Arrow,
            Self::PipeForward => SyntaxKind::PipeForward,
            Self::Equals => SyntaxKind::Equals,
            Self::Minus => SyntaxKind::Minus,
            Self::Tilde => SyntaxKind::Tilde,
            Self::Dot => SyntaxKind::Dot,
            Self::PieceKw => SyntaxKind::PieceKw,
            Self::TempoKw => SyntaxKind::TempoKw,
            Self::MeterKw => SyntaxKind::MeterKw,
            Self::KeyKw => SyntaxKind::KeyKw,
            Self::SubtitleKw => SyntaxKind::SubtitleKw,
            Self::ComposerKw => SyntaxKind::ComposerKw,
            Self::ArrangerKw => SyntaxKind::ArrangerKw,
            Self::CopyrightKw => SyntaxKind::CopyrightKw,
            Self::MotifKw => SyntaxKind::MotifKw,
            Self::ScoreKw => SyntaxKind::ScoreKw,
            Self::PartKw => SyntaxKind::PartKw,
            Self::VoiceKw => SyntaxKind::VoiceKw,
            Self::ClefKw => SyntaxKind::ClefKw,
            Self::UseKw => SyntaxKind::UseKw,
            Self::TransposeKw => SyntaxKind::TransposeKw,
            Self::DownKw => SyntaxKind::DownKw,
            Self::UpKw => SyntaxKind::UpKw,
            Self::RestKw => SyntaxKind::RestKw,
            Self::ChordKw => SyntaxKind::ChordKw,
            Self::RepeatKw => SyntaxKind::RepeatKw,
            Self::SlurKw => SyntaxKind::SlurKw,
            Self::DynamicKw => SyntaxKind::DynamicKw,
            Self::TupletKw => SyntaxKind::TupletKw,
            Self::PerformanceKw => SyntaxKind::PerformanceKw,
            Self::ProfileKw => SyntaxKind::ProfileKw,
            Self::ArticulationKw => SyntaxKind::ArticulationKw,
            Self::StudioKw => SyntaxKind::StudioKw,
            Self::PatchKw => SyntaxKind::PatchKw,
            Self::ModulateKw => SyntaxKind::ModulateKw,
            Self::BusKw => SyntaxKind::BusKw,
            Self::AssignKw => SyntaxKind::AssignKw,
            Self::RouteKw => SyntaxKind::RouteKw,
            Self::SendKw => SyntaxKind::SendKw,
            Self::MasterKw => SyntaxKind::MasterKw,
            Self::AtKw => SyntaxKind::AtKw,
            Self::OutputKw => SyntaxKind::OutputKw,
            Self::PitchKw => SyntaxKind::PitchKw,
            Self::StretchKw => SyntaxKind::StretchKw,
            Self::RetrogradeKw => SyntaxKind::RetrogradeKw,
            Self::InvertKw => SyntaxKind::InvertKw,
            Self::AroundKw => SyntaxKind::AroundKw,
            Self::WithKw => SyntaxKind::WithKw,
            Self::NoteKw => SyntaxKind::NoteKw,
            Self::PhraseKw => SyntaxKind::PhraseKw,
            Self::SectionKw => SyntaxKind::SectionKw,
            Self::HarmonyKw => SyntaxKind::HarmonyKw,
            Self::LibraryKw => SyntaxKind::LibraryKw,
            Self::CrescendoKw => SyntaxKind::CrescendoKw,
            Self::DiminuendoKw => SyntaxKind::DiminuendoKw,
            Self::ToKw => SyntaxKind::ToKw,
            Self::BarKw => SyntaxKind::BarKw,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(source: &str) -> Vec<SyntaxKind> {
        lex(source).tokens().iter().map(|token| token.kind).collect()
    }

    /// The losslessness invariant: tokens tile the whole source, errors or
    /// not.
    fn assert_round_trip(source: &str) {
        let lexed = lex(source);
        let mut rebuilt = String::new();
        for token in lexed.tokens() {
            let start = usize::from(token.range.start());
            let end = usize::from(token.range.end());
            if let Some(slice) = source.get(start..end) {
                rebuilt.push_str(slice);
            }
        }
        assert_eq!(rebuilt, source, "tokens must tile the source");
    }

    const SAMPLE: &str = "piece \"Glass Mountain\" {\n    tempo quarter = 72;\n    meter 4/4;\n    key a minor;\n\n    score {\n        part strings {\n            voice upper {\n                c5 1;\n                c5 1/2; // held\n                a4 1;\n                gs4 1;\n            }\n        }\n    }\n}\n";

    #[test]
    fn structural_source_lexes_and_round_trips() {
        let lexed = lex(SAMPLE);
        assert!(lexed.errors().is_empty(), "errors: {:?}", lexed.errors());
        assert_round_trip(SAMPLE);
        let significant: Vec<SyntaxKind> = lexed
            .tokens()
            .iter()
            .filter(|token| !token.kind.is_trivia())
            .map(|token| token.kind)
            .collect();
        let expected_prefix = [
            SyntaxKind::PieceKw,
            SyntaxKind::String,
            SyntaxKind::LBrace,
            SyntaxKind::TempoKw,
            SyntaxKind::Identifier, // quarter
            SyntaxKind::Equals,
            SyntaxKind::Integer, // 72
            SyntaxKind::Semicolon,
            SyntaxKind::MeterKw,
            SyntaxKind::Rational, // 4/4
            SyntaxKind::Semicolon,
            SyntaxKind::KeyKw,
            SyntaxKind::Identifier, // a (bare letter is not a pitch without an octave)
            SyntaxKind::Identifier, // minor
            SyntaxKind::Semicolon,
        ];
        assert!(significant.starts_with(&expected_prefix), "got: {significant:?}");
        assert!(significant.contains(&SyntaxKind::PitchLiteral)); // c5, a4, gs4
    }

    #[test]
    fn pitch_literals() {
        for source in ["c5", "gs4", "css3", "bf4", "bff2", "en5", "a-1"] {
            assert_eq!(kinds(source), [SyntaxKind::PitchLiteral], "source: {source}");
            assert_round_trip(source);
        }
    }

    #[test]
    fn durations_and_numbers() {
        assert_eq!(
            kinds("1 1/2 3/8 1/12 4/4 0.55"),
            [
                SyntaxKind::Integer,
                SyntaxKind::Whitespace,
                SyntaxKind::Rational,
                SyntaxKind::Whitespace,
                SyntaxKind::Rational,
                SyntaxKind::Whitespace,
                SyntaxKind::Rational,
                SyntaxKind::Whitespace,
                SyntaxKind::Rational,
                SyntaxKind::Whitespace,
                SyntaxKind::Float,
            ]
        );
    }

    #[test]
    fn unit_suffixes() {
        assert_eq!(
            kinds("1400 Hz 250 ms 1.8 s -18 dB 72 bpm"),
            [
                SyntaxKind::Integer,
                SyntaxKind::Whitespace,
                SyntaxKind::UnitHz,
                SyntaxKind::Whitespace,
                SyntaxKind::Integer,
                SyntaxKind::Whitespace,
                SyntaxKind::UnitMs,
                SyntaxKind::Whitespace,
                SyntaxKind::Float,
                SyntaxKind::Whitespace,
                SyntaxKind::UnitS,
                SyntaxKind::Whitespace,
                SyntaxKind::Minus,
                SyntaxKind::Integer,
                SyntaxKind::Whitespace,
                SyntaxKind::UnitDb,
                SyntaxKind::Whitespace,
                SyntaxKind::Integer,
                SyntaxKind::Whitespace,
                SyntaxKind::UnitBpm,
            ]
        );
    }

    #[test]
    fn identifiers_keywords_and_units_are_distinct() {
        assert_eq!(
            kinds("sigh glass_pad use ms bpm s"),
            [
                SyntaxKind::Identifier,
                SyntaxKind::Whitespace,
                SyntaxKind::Identifier,
                SyntaxKind::Whitespace,
                SyntaxKind::UseKw,
                SyntaxKind::Whitespace,
                SyntaxKind::UnitMs,
                SyntaxKind::Whitespace,
                SyntaxKind::UnitBpm,
                SyntaxKind::Whitespace,
                SyntaxKind::UnitS,
            ]
        );
    }

    #[test]
    fn invalid_token_is_recorded_and_lexing_continues() {
        let source = "c5 @ d5";
        let lexed = lex(source);
        assert_eq!(
            kinds(source),
            [
                SyntaxKind::PitchLiteral,
                SyntaxKind::Whitespace,
                SyntaxKind::Error,
                SyntaxKind::Whitespace,
                SyntaxKind::PitchLiteral,
            ]
        );
        assert_eq!(lexed.errors().len(), 1);
        assert_eq!(
            lexed.errors().first().copied().map(LexError::kind),
            Some(LexErrorKind::InvalidToken)
        );
        assert_round_trip(source);
    }

    #[test]
    fn unterminated_string_extends_to_line_end() {
        let source = "piece \"abc\nc5 1;";
        let lexed = lex(source);
        assert_eq!(lexed.errors().len(), 1);
        let error = lexed.errors().first().copied();
        assert_eq!(error.map(LexError::kind), Some(LexErrorKind::UnterminatedString));
        // The error token covers `"abc` and stops before the newline, which
        // then lexes as ordinary whitespace.
        assert_eq!(kinds(source).last(), Some(&SyntaxKind::Semicolon));
        assert_round_trip(source);
    }

    #[test]
    fn unterminated_block_comment_extends_to_eof() {
        let source = "c5 1; /* abc d5";
        let lexed = lex(source);
        assert_eq!(lexed.errors().len(), 1);
        assert_eq!(
            lexed.errors().first().copied().map(LexError::kind),
            Some(LexErrorKind::UnterminatedBlockComment)
        );
        assert_eq!(kinds(source).last(), Some(&SyntaxKind::Error));
        assert_round_trip(source);
    }

    #[test]
    fn punctuation() {
        assert_eq!(
            kinds("{ } [ ] ( ) ; , : -> |> = -"),
            [
                SyntaxKind::LBrace,
                SyntaxKind::Whitespace,
                SyntaxKind::RBrace,
                SyntaxKind::Whitespace,
                SyntaxKind::LBracket,
                SyntaxKind::Whitespace,
                SyntaxKind::RBracket,
                SyntaxKind::Whitespace,
                SyntaxKind::LParen,
                SyntaxKind::Whitespace,
                SyntaxKind::RParen,
                SyntaxKind::Whitespace,
                SyntaxKind::Semicolon,
                SyntaxKind::Whitespace,
                SyntaxKind::Comma,
                SyntaxKind::Whitespace,
                SyntaxKind::Colon,
                SyntaxKind::Whitespace,
                SyntaxKind::Arrow,
                SyntaxKind::Whitespace,
                SyntaxKind::PipeForward,
                SyntaxKind::Whitespace,
                SyntaxKind::Equals,
                SyntaxKind::Whitespace,
                SyntaxKind::Minus,
            ]
        );
    }
}
