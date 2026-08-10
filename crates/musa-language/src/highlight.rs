//! What each token *is*, for an editor that wants to set it.
//!
//! An editor cannot highlight a token stream it has to guess at, and a
//! hand-written pattern list in the interface would drift from the lexer the
//! first time a keyword was added. So the classification lives here, beside
//! the tokens, as one exhaustive match: adding a [`SyntaxKind`] without
//! deciding what it looks like does not compile.
//!
//! The classes are editor vocabulary, not visual instruction — what colour a
//! class takes is `docs/interface/01-visual-language.md`'s business, and it
//! answers with ink weight and one accent rather than a rainbow.

use crate::syntax_kind::SyntaxKind;

/// Every token the language spells literally: keywords, unit suffixes, and
/// punctuation, with the kind each one lexes as.
///
/// The lexer spells these in `logos` attributes, which no other program can
/// read. This is the same list as data, and `spellings_lex_as_their_kind`
/// asserts the two agree — so an editor built from it cannot know a keyword
/// the lexer does not, or miss one it does.
pub const SPELLINGS: &[(&str, SyntaxKind)] = &[
    ("piece", SyntaxKind::PieceKw),
    ("tempo", SyntaxKind::TempoKw),
    ("meter", SyntaxKind::MeterKw),
    ("key", SyntaxKind::KeyKw),
    ("subtitle", SyntaxKind::SubtitleKw),
    ("composer", SyntaxKind::ComposerKw),
    ("arranger", SyntaxKind::ArrangerKw),
    ("copyright", SyntaxKind::CopyrightKw),
    ("motif", SyntaxKind::MotifKw),
    ("score", SyntaxKind::ScoreKw),
    ("part", SyntaxKind::PartKw),
    ("voice", SyntaxKind::VoiceKw),
    ("clef", SyntaxKind::ClefKw),
    ("use", SyntaxKind::UseKw),
    ("transpose", SyntaxKind::TransposeKw),
    ("down", SyntaxKind::DownKw),
    ("up", SyntaxKind::UpKw),
    ("rest", SyntaxKind::RestKw),
    ("repeat", SyntaxKind::RepeatKw),
    ("bar", SyntaxKind::BarKw),
    ("senza", SyntaxKind::SenzaKw),
    ("ending", SyntaxKind::EndingKw),
    ("fragment", SyntaxKind::FragmentKw),
    ("mobile", SyntaxKind::MobileKw),
    ("improvise", SyntaxKind::ImproviseKw),
    ("over", SyntaxKind::OverKw),
    ("slur", SyntaxKind::SlurKw),
    ("dynamic", SyntaxKind::DynamicKw),
    ("tuplet", SyntaxKind::TupletKw),
    ("performance", SyntaxKind::PerformanceKw),
    ("profile", SyntaxKind::ProfileKw),
    ("mark", SyntaxKind::MarkKw),
    ("groove", SyntaxKind::GrooveKw),
    ("grace", SyntaxKind::GraceKw),
    ("studio", SyntaxKind::StudioKw),
    ("patch", SyntaxKind::PatchKw),
    ("modulate", SyntaxKind::ModulateKw),
    ("bus", SyntaxKind::BusKw),
    ("assign", SyntaxKind::AssignKw),
    ("route", SyntaxKind::RouteKw),
    ("send", SyntaxKind::SendKw),
    ("master", SyntaxKind::MasterKw),
    ("at", SyntaxKind::AtKw),
    ("output", SyntaxKind::OutputKw),
    ("pitch", SyntaxKind::PitchKw),
    ("stretch", SyntaxKind::StretchKw),
    ("retrograde", SyntaxKind::RetrogradeKw),
    ("invert", SyntaxKind::InvertKw),
    ("around", SyntaxKind::AroundKw),
    ("with", SyntaxKind::WithKw),
    ("note", SyntaxKind::NoteKw),
    ("phrase", SyntaxKind::PhraseKw),
    ("section", SyntaxKind::SectionKw),
    ("harmony", SyntaxKind::HarmonyKw),
    ("library", SyntaxKind::LibraryKw),
    ("crescendo", SyntaxKind::CrescendoKw),
    ("diminuendo", SyntaxKind::DiminuendoKw),
    ("to", SyntaxKind::ToKw),
    ("let", SyntaxKind::LetKw),
    ("fn", SyntaxKind::FnKw),
    ("music", SyntaxKind::MusicKw),
    ("option", SyntaxKind::OptionKw),
    ("list", SyntaxKind::ListKw),
    ("match", SyntaxKind::MatchKw),
    ("some", SyntaxKind::SomeKw),
    ("none", SyntaxKind::NoneKw),
    ("true", SyntaxKind::TrueKw),
    ("false", SyntaxKind::FalseKw),
    ("scale", SyntaxKind::ScaleKw),
    ("degree", SyntaxKind::DegreeKw),
    ("frame", SyntaxKind::FrameKw),
    ("in", SyntaxKind::InKw),
    ("step", SyntaxKind::StepKw),
    ("chord", SyntaxKind::ChordKw),
    ("stack", SyntaxKind::StackKw),
    ("template", SyntaxKind::TemplateKw),
    ("make", SyntaxKind::MakeKw),
    ("as", SyntaxKind::AsKw),
    ("Hz", SyntaxKind::UnitHz),
    ("ms", SyntaxKind::UnitMs),
    ("s", SyntaxKind::UnitS),
    ("dB", SyntaxKind::UnitDb),
    ("bpm", SyntaxKind::UnitBpm),
    ("{", SyntaxKind::LBrace),
    ("}", SyntaxKind::RBrace),
    ("[", SyntaxKind::LBracket),
    ("]", SyntaxKind::RBracket),
    ("(", SyntaxKind::LParen),
    (")", SyntaxKind::RParen),
    (";", SyntaxKind::Semicolon),
    (",", SyntaxKind::Comma),
    (":", SyntaxKind::Colon),
    ("->", SyntaxKind::Arrow),
    ("|>", SyntaxKind::PipeForward),
    ("=", SyntaxKind::Equals),
    ("-", SyntaxKind::Minus),
    ("~", SyntaxKind::Tilde),
    (".", SyntaxKind::Dot),
    ("/", SyntaxKind::Slash),
    ("|", SyntaxKind::Pipe),
    (">", SyntaxKind::Greater),
    ("^", SyntaxKind::Caret),
    ("#", SyntaxKind::Hash),
];

/// What a token is, for setting purposes.
///
/// Coarser than [`SyntaxKind`] on purpose: an editor that distinguished
/// `tempo` from `meter` would be spending the reader's attention on the
/// difference between two words they can already read.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum TokenClass {
    /// Whitespace and comments.
    Comment,
    /// Structural keywords: `piece`, `voice`, `motif`, `transpose`.
    Keyword,
    /// `use` alone. It is where generated material comes from, and the
    /// interface marks it as such wherever it appears.
    Use,
    /// Written pitches and intervals — the music itself, in the text.
    Pitch,
    /// Rationals: durations and meters.
    Duration,
    /// Integers and floats.
    Number,
    /// String literals.
    Text,
    /// Unit suffixes: `Hz`, `dB`, `bpm`.
    Unit,
    /// Everything the composer named: motifs, parts, voices, patches.
    Name,
    /// Braces, brackets, separators, operators.
    Punctuation,
    /// A span the lexer could not read.
    Invalid,
}

impl TokenClass {
    /// The class's name, as the interface spells it.
    ///
    /// One spelling, generated into the editor's own table, so a class cannot
    /// be styled under a name the lexer never produces.
    pub fn name(self) -> &'static str {
        match self {
            Self::Comment => "comment",
            Self::Keyword => "keyword",
            Self::Use => "use",
            Self::Pitch => "pitch",
            Self::Duration => "duration",
            Self::Number => "number",
            Self::Text => "text",
            Self::Unit => "unit",
            Self::Name => "name",
            Self::Punctuation => "punctuation",
            Self::Invalid => "invalid",
        }
    }

    /// The class of a token kind, or `None` for a parser node kind.
    ///
    /// Exhaustive by construction: the workspace forbids wildcard match arms,
    /// so a new [`SyntaxKind`] arrives here as a compile error rather than as
    /// unstyled text in the editor.
    pub fn of(kind: SyntaxKind) -> Option<Self> {
        let class = match kind {
            SyntaxKind::Whitespace | SyntaxKind::LineComment | SyntaxKind::BlockComment => Self::Comment,

            SyntaxKind::Identifier => Self::Name,
            SyntaxKind::Integer | SyntaxKind::Float => Self::Number,
            SyntaxKind::Rational => Self::Duration,
            SyntaxKind::String => Self::Text,
            SyntaxKind::PitchLiteral | SyntaxKind::IntervalLiteral => Self::Pitch,

            SyntaxKind::UnitHz | SyntaxKind::UnitMs | SyntaxKind::UnitS | SyntaxKind::UnitDb | SyntaxKind::UnitBpm => {
                Self::Unit
            }

            SyntaxKind::LBrace
            | SyntaxKind::RBrace
            | SyntaxKind::LBracket
            | SyntaxKind::RBracket
            | SyntaxKind::LParen
            | SyntaxKind::RParen
            | SyntaxKind::Semicolon
            | SyntaxKind::Comma
            | SyntaxKind::Colon
            | SyntaxKind::Arrow
            | SyntaxKind::PipeForward
            | SyntaxKind::Equals
            | SyntaxKind::Minus
            | SyntaxKind::Tilde
            | SyntaxKind::Dot
            | SyntaxKind::Slash
            | SyntaxKind::Pipe
            | SyntaxKind::Greater
            | SyntaxKind::Caret
            | SyntaxKind::Hash => Self::Punctuation,

            SyntaxKind::UseKw => Self::Use,

            SyntaxKind::PieceKw
            | SyntaxKind::TempoKw
            | SyntaxKind::MeterKw
            | SyntaxKind::KeyKw
            | SyntaxKind::SubtitleKw
            | SyntaxKind::ComposerKw
            | SyntaxKind::ArrangerKw
            | SyntaxKind::CopyrightKw
            | SyntaxKind::MotifKw
            | SyntaxKind::ScoreKw
            | SyntaxKind::PartKw
            | SyntaxKind::VoiceKw
            | SyntaxKind::ClefKw
            | SyntaxKind::TransposeKw
            | SyntaxKind::DownKw
            | SyntaxKind::UpKw
            | SyntaxKind::RestKw
            | SyntaxKind::RepeatKw
            | SyntaxKind::SlurKw
            | SyntaxKind::DynamicKw
            | SyntaxKind::TupletKw
            | SyntaxKind::PerformanceKw
            | SyntaxKind::ProfileKw
            | SyntaxKind::MarkKw
            | SyntaxKind::GrooveKw
            | SyntaxKind::GraceKw
            | SyntaxKind::StudioKw
            | SyntaxKind::PatchKw
            | SyntaxKind::ModulateKw
            | SyntaxKind::BusKw
            | SyntaxKind::AssignKw
            | SyntaxKind::RouteKw
            | SyntaxKind::SendKw
            | SyntaxKind::MasterKw
            | SyntaxKind::AtKw
            | SyntaxKind::OutputKw
            | SyntaxKind::PitchKw
            | SyntaxKind::StretchKw
            | SyntaxKind::RetrogradeKw
            | SyntaxKind::InvertKw
            | SyntaxKind::AroundKw
            | SyntaxKind::WithKw
            | SyntaxKind::NoteKw
            | SyntaxKind::PhraseKw
            | SyntaxKind::SectionKw
            | SyntaxKind::HarmonyKw
            | SyntaxKind::LibraryKw
            | SyntaxKind::TemplateKw
            | SyntaxKind::MakeKw
            | SyntaxKind::AsKw
            | SyntaxKind::CrescendoKw
            | SyntaxKind::DiminuendoKw
            | SyntaxKind::ToKw
            | SyntaxKind::BarKw
            | SyntaxKind::SenzaKw
            | SyntaxKind::EndingKw
            | SyntaxKind::FragmentKw
            | SyntaxKind::MobileKw
            | SyntaxKind::ImproviseKw
            | SyntaxKind::OverKw
            | SyntaxKind::LetKw
            | SyntaxKind::FnKw
            | SyntaxKind::MusicKw
            | SyntaxKind::OptionKw
            | SyntaxKind::ListKw
            | SyntaxKind::MatchKw
            | SyntaxKind::SomeKw
            | SyntaxKind::NoneKw
            | SyntaxKind::TrueKw
            | SyntaxKind::FalseKw
            | SyntaxKind::ScaleKw
            | SyntaxKind::DegreeKw
            | SyntaxKind::FrameKw
            | SyntaxKind::InKw
            | SyntaxKind::StepKw
            | SyntaxKind::ChordKw
            | SyntaxKind::StackKw => Self::Keyword,

            SyntaxKind::Error => Self::Invalid,

            SyntaxKind::Root
            | SyntaxKind::PieceDecl
            | SyntaxKind::TempoStmt
            | SyntaxKind::MeterStmt
            | SyntaxKind::KeyStmt
            | SyntaxKind::FrontMatterStmt
            | SyntaxKind::MotifDecl
            | SyntaxKind::ScoreDecl
            | SyntaxKind::PartDecl
            | SyntaxKind::ClefStmt
            | SyntaxKind::VoiceDecl
            | SyntaxKind::NoteStmt
            | SyntaxKind::RestStmt
            | SyntaxKind::ChordStmt
            | SyntaxKind::UseStmt
            | SyntaxKind::TransposeStmt
            | SyntaxKind::RepeatStmt
            | SyntaxKind::SlurStmt
            | SyntaxKind::DynamicStmt
            | SyntaxKind::TupletStmt
            | SyntaxKind::StretchStmt
            | SyntaxKind::RetrogradeStmt
            | SyntaxKind::InvertStmt
            | SyntaxKind::WithClause
            | SyntaxKind::OverrideStmt
            | SyntaxKind::PhraseStmt
            | SyntaxKind::MarkStmt
            | SyntaxKind::GraceStmt
            | SyntaxKind::GraceNote
            | SyntaxKind::SectionStmt
            | SyntaxKind::HarmonyDecl
            | SyntaxKind::HarmonyStmt
            | SyntaxKind::Position
            | SyntaxKind::PitchClass
            | SyntaxKind::ChordSymbol
            | SyntaxKind::LibraryDecl
            | SyntaxKind::ImportStmt
            | SyntaxKind::HairpinStmt
            | SyntaxKind::Duration
            | SyntaxKind::ArticulationList
            | SyntaxKind::PerformanceDecl
            | SyntaxKind::ProfileDecl
            | SyntaxKind::MarkRule
            | SyntaxKind::DynamicRule
            | SyntaxKind::GrooveRule
            | SyntaxKind::GraceRule
            | SyntaxKind::SettingStmt
            | SyntaxKind::ProfileStmt
            | SyntaxKind::StudioDecl
            | SyntaxKind::PatchDecl
            | SyntaxKind::BusDecl
            | SyntaxKind::SignalBinding
            | SyntaxKind::ChainStmt
            | SyntaxKind::SignalChain
            | SyntaxKind::CallExpr
            | SyntaxKind::ArgList
            | SyntaxKind::Arg
            | SyntaxKind::ValueLiteral
            | SyntaxKind::NameRef
            | SyntaxKind::ModulateStmt
            | SyntaxKind::ParamPath
            | SyntaxKind::AssignStmt
            | SyntaxKind::RouteStmt
            | SyntaxKind::SendStmt
            | SyntaxKind::Block
            | SyntaxKind::BarStmt
            | SyntaxKind::SenzaStmt
            | SyntaxKind::EndingStmt
            | SyntaxKind::FragmentDecl
            | SyntaxKind::MobileStmt
            | SyntaxKind::ImproviseStmt
            | SyntaxKind::LetDecl
            | SyntaxKind::FnDecl
            | SyntaxKind::Param
            | SyntaxKind::ParamList
            | SyntaxKind::TypeExpr
            | SyntaxKind::TypeName
            | SyntaxKind::FunctionType
            | SyntaxKind::ProductType
            | SyntaxKind::OptionType
            | SyntaxKind::ListType
            | SyntaxKind::NameExpr
            | SyntaxKind::LiteralExpr
            | SyntaxKind::ParenExpr
            | SyntaxKind::ProductExpr
            | SyntaxKind::ListExpr
            | SyntaxKind::OptionExpr
            | SyntaxKind::ApplyExpr
            | SyntaxKind::PitchExpr
            | SyntaxKind::ExprArgList
            | SyntaxKind::ExprArg
            | SyntaxKind::MatchExpr
            | SyntaxKind::MatchArm
            | SyntaxKind::Pattern
            | SyntaxKind::MusicExpr
            | SyntaxKind::ScaleExpr
            | SyntaxKind::KeyExpr
            | SyntaxKind::StepExpr
            | SyntaxKind::InScaleStmt
            | SyntaxKind::ChordExpr
            | SyntaxKind::StackStmt
            | SyntaxKind::TemplateDecl
            | SyntaxKind::MakeStmt => return None,
        };
        Some(class)
    }
}
