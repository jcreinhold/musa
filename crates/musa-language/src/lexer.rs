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
    #[regex(r"//[^\n]*", allow_greedy = true)]
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
    // A pitch is a letter, an optional accidental, and an octave. `b` is both
    // a letter and a flat, and the two never collide because the letter is
    // always first: `b2` is B, `bb2` is B flat, `bbb2` is B double flat.
    #[regex(r"[a-g](#+|b+|n)?-?[0-9]+")]
    PitchLiteral,
    #[regex(r"(P|M|m|A+|d{2,}|dim)[0-9]+")]
    IntervalLiteral,
    #[regex(r"[0-9]+\.[0-9]+")]
    Float,
    #[regex(r"[0-9]+/[0-9]+")]
    Rational,
    #[regex(r"[0-9]+")]
    Integer,
    // A name may carry digits after its first letter, because musicians write
    // words that do: `major7`, `sus4`, `drop2`. The literals above keep their
    // spellings — `c4` is a pitch and `M3` an interval — because each matches
    // the same text at a higher priority, so a name only wins where no
    // literal reads the word at all.
    #[regex(r"[a-zA-Z_][a-zA-Z_0-9]*")]
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
    // Maximal munch keeps every longer spelling that starts with these
    // characters: `//` and `/*` still open comments, `1/4` is still one
    // `Rational`, `|>` is still `PipeForward`, and `->` is still `Arrow`.
    #[token("/")]
    Slash,
    #[token("|")]
    Pipe,
    #[token(">")]
    Greater,
    #[token("^")]
    Caret,
    #[token("#")]
    Hash,

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
    #[token("repeat", priority = 3)]
    RepeatKw,
    #[token("slur", priority = 3)]
    SlurKw,
    #[token("dynamic", priority = 3)]
    DynamicKw,
    #[token("groove", priority = 3)]
    GrooveKw,
    #[token("grace", priority = 3)]
    GraceKw,
    #[token("tuplet", priority = 3)]
    TupletKw,
    #[token("performance", priority = 3)]
    PerformanceKw,
    #[token("profile", priority = 3)]
    ProfileKw,
    #[token("mark", priority = 3)]
    MarkKw,
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
    #[token("ending", priority = 3)]
    EndingKw,
    #[token("fragment", priority = 3)]
    FragmentKw,
    #[token("mobile", priority = 3)]
    MobileKw,
    #[token("improvise", priority = 3)]
    ImproviseKw,
    #[token("over", priority = 3)]
    OverKw,
    #[token("senza", priority = 3)]
    SenzaKw,
    #[token("let", priority = 3)]
    LetKw,
    #[token("fn", priority = 3)]
    FnKw,
    #[token("music", priority = 3)]
    MusicKw,
    #[token("option", priority = 3)]
    OptionKw,
    #[token("list", priority = 3)]
    ListKw,
    #[token("match", priority = 3)]
    MatchKw,
    #[token("some", priority = 3)]
    SomeKw,
    #[token("none", priority = 3)]
    NoneKw,
    #[token("true", priority = 3)]
    TrueKw,
    #[token("false", priority = 3)]
    FalseKw,
    #[token("scale", priority = 3)]
    ScaleKw,
    #[token("degree", priority = 3)]
    DegreeKw,
    #[token("frame", priority = 3)]
    FrameKw,
    #[token("in", priority = 3)]
    InKw,
    #[token("step", priority = 3)]
    StepKw,
    #[token("chord", priority = 3)]
    ChordKw,
    #[token("stack", priority = 3)]
    StackKw,
    #[token("template", priority = 3)]
    TemplateKw,
    #[token("make", priority = 3)]
    MakeKw,
    #[token("as", priority = 3)]
    AsKw,
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
            | Self::Slash
            | Self::Pipe
            | Self::Greater
            | Self::Caret
            | Self::Hash
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
            | Self::RepeatKw
            | Self::SlurKw
            | Self::DynamicKw
            | Self::GrooveKw
            | Self::GraceKw
            | Self::TupletKw
            | Self::PerformanceKw
            | Self::ProfileKw
            | Self::MarkKw
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
            | Self::FragmentKw
            | Self::MobileKw
            | Self::ImproviseKw
            | Self::OverKw
            | Self::SenzaKw
            | Self::BarKw
            | Self::EndingKw
            | Self::LetKw
            | Self::FnKw
            | Self::MusicKw
            | Self::OptionKw
            | Self::ListKw
            | Self::MatchKw
            | Self::SomeKw
            | Self::NoneKw
            | Self::TrueKw
            | Self::FalseKw
            | Self::ScaleKw
            | Self::DegreeKw
            | Self::FrameKw
            | Self::InKw
            | Self::StepKw
            | Self::ChordKw
            | Self::StackKw
            | Self::TemplateKw
            | Self::MakeKw
            | Self::AsKw => None,
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
            Self::Slash => SyntaxKind::Slash,
            Self::Pipe => SyntaxKind::Pipe,
            Self::Greater => SyntaxKind::Greater,
            Self::Caret => SyntaxKind::Caret,
            Self::Hash => SyntaxKind::Hash,
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
            Self::RepeatKw => SyntaxKind::RepeatKw,
            Self::SlurKw => SyntaxKind::SlurKw,
            Self::DynamicKw => SyntaxKind::DynamicKw,
            Self::GrooveKw => SyntaxKind::GrooveKw,
            Self::GraceKw => SyntaxKind::GraceKw,
            Self::TupletKw => SyntaxKind::TupletKw,
            Self::PerformanceKw => SyntaxKind::PerformanceKw,
            Self::ProfileKw => SyntaxKind::ProfileKw,
            Self::MarkKw => SyntaxKind::MarkKw,
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
            Self::FragmentKw => SyntaxKind::FragmentKw,
            Self::MobileKw => SyntaxKind::MobileKw,
            Self::ImproviseKw => SyntaxKind::ImproviseKw,
            Self::OverKw => SyntaxKind::OverKw,
            Self::SenzaKw => SyntaxKind::SenzaKw,
            Self::BarKw => SyntaxKind::BarKw,
            Self::EndingKw => SyntaxKind::EndingKw,
            Self::LetKw => SyntaxKind::LetKw,
            Self::FnKw => SyntaxKind::FnKw,
            Self::MusicKw => SyntaxKind::MusicKw,
            Self::OptionKw => SyntaxKind::OptionKw,
            Self::ListKw => SyntaxKind::ListKw,
            Self::MatchKw => SyntaxKind::MatchKw,
            Self::SomeKw => SyntaxKind::SomeKw,
            Self::NoneKw => SyntaxKind::NoneKw,
            Self::TrueKw => SyntaxKind::TrueKw,
            Self::FalseKw => SyntaxKind::FalseKw,
            Self::ScaleKw => SyntaxKind::ScaleKw,
            Self::DegreeKw => SyntaxKind::DegreeKw,
            Self::FrameKw => SyntaxKind::FrameKw,
            Self::InKw => SyntaxKind::InKw,
            Self::StepKw => SyntaxKind::StepKw,
            Self::ChordKw => SyntaxKind::ChordKw,
            Self::StackKw => SyntaxKind::StackKw,
            Self::TemplateKw => SyntaxKind::TemplateKw,
            Self::MakeKw => SyntaxKind::MakeKw,
            Self::AsKw => SyntaxKind::AsKw,
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

    const SAMPLE: &str = "piece \"Glass Mountain\" {\n    tempo quarter = 72;\n    meter 4/4;\n    key a minor;\n\n    score {\n        part strings {\n            voice upper {\n                c5/1\n                c5/2 // held\n                a4/1\n                g#4/1\n            }\n        }\n    }\n}\n";

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
        assert!(significant.contains(&SyntaxKind::PitchLiteral)); // c5, a4, g#4
    }

    /// The five tokens added for the compact note syntax take nothing away
    /// from the spellings that already existed.
    ///
    /// Every one of these is a longer match that starts with a character the
    /// lexer now also accepts alone, which is precisely the class maximal
    /// munch is supposed to settle. Settled by test, not by comment.
    #[test]
    fn a_longer_spelling_still_wins_over_the_new_single_characters() {
        for (source, expected) in [
            ("//x", SyntaxKind::LineComment),
            ("/* x */", SyntaxKind::BlockComment),
            ("/* x", SyntaxKind::Error),
            ("1/4", SyntaxKind::Rational),
            ("->", SyntaxKind::Arrow),
            ("|>", SyntaxKind::PipeForward),
            ("a-1", SyntaxKind::PitchLiteral),
            ("0.55", SyntaxKind::Float),
        ] {
            assert_eq!(kinds(source), [expected], "source: {source}");
            assert_round_trip(source);
        }
    }

    #[test]
    fn the_compact_note_syntax_lexes_one_character_at_a_time() {
        assert_eq!(
            kinds("c4/4."),
            [
                SyntaxKind::PitchLiteral,
                SyntaxKind::Slash,
                SyntaxKind::Integer,
                SyntaxKind::Dot,
            ]
        );
        assert_eq!(
            kinds("|>>^#"),
            [
                SyntaxKind::PipeForward,
                SyntaxKind::Greater,
                SyntaxKind::Caret,
                SyntaxKind::Hash,
            ],
            "`|>` is still one token; the `>` after it is its own"
        );
        assert_round_trip("| c4/4 > ^ #");
    }

    #[test]
    fn pitch_literals() {
        for source in ["c5", "g#4", "c####3", "bb4", "bbbbb2", "en5", "a-1", "b2"] {
            assert_eq!(kinds(source), [SyntaxKind::PitchLiteral], "source: {source}");
            assert_round_trip(source);
        }
    }

    #[test]
    fn simple_compound_and_multiply_altered_intervals_are_literals() {
        for source in ["P1", "m2", "M10", "AAA4", "dim5", "ddd17"] {
            assert_eq!(kinds(source), [SyntaxKind::IntervalLiteral], "source: {source}");
        }
    }

    /// A bare letter is not a pitch. `key a minor;`, `mobile { a; b; c; }`
    /// and a motif parameter named `a` all depend on it, so the octave is
    /// required and this is the test that says so.
    #[test]
    fn a_letter_without_an_octave_is_a_name() {
        for source in ["a", "b", "bb", "g", "f"] {
            assert_eq!(kinds(source), [SyntaxKind::Identifier], "source: {source}");
        }
        // Which is why a pitch class with a sharp is two tokens, and why the
        // parser builds a node out of them.
        assert_eq!(kinds("g#"), [SyntaxKind::Identifier, SyntaxKind::Hash]);
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
        let source = "c5/1 /* abc d5";
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
