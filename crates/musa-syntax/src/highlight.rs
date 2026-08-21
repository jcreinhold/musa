//! What each token *is*, for an editor that wants to set it.
//!
//! An editor cannot highlight a token stream it has to guess at, and a
//! hand-written pattern list in the interface would drift from the lexer the
//! first time a keyword was added. So the classification lives here, beside
//! the tokens, as one exhaustive match: adding a [`SyntaxKind`] without
//! deciding what it looks like does not compile.
//!
//! The classes are editor vocabulary, not visual instruction — what colour a
//! class takes is `docs/rules/desktop/01-visual-language.md`'s business, and it
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
    ("import", SyntaxKind::ImportKw),
    ("syntax", SyntaxKind::SyntaxKw),
    ("transpose", SyntaxKind::TransposeKw),
    ("down", SyntaxKind::DownKw),
    ("up", SyntaxKind::UpKw),
    ("rest", SyntaxKind::RestKw),
    ("repeat", SyntaxKind::RepeatKw),
    ("bar", SyntaxKind::BarKw),
    ("assert", SyntaxKind::AssertKw),
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
    ("events", SyntaxKind::EventsKw),
    ("quote", SyntaxKind::QuoteKw),
    ("Option", SyntaxKind::OptionKw),
    ("List", SyntaxKind::ListKw),
    ("Result", SyntaxKind::ResultKw),
    ("match", SyntaxKind::MatchKw),
    ("if", SyntaxKind::IfKw),
    ("else", SyntaxKind::ElseKw),
    ("Some", SyntaxKind::SomeKw),
    ("None", SyntaxKind::NoneKw),
    ("Ok", SyntaxKind::OkKw),
    ("Err", SyntaxKind::ErrKw),
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
    ("signature", SyntaxKind::SignatureKw),
    ("structure", SyntaxKind::StructureKw),
    ("data", SyntaxKind::DataKw),
    ("record", SyntaxKind::RecordKw),
    ("enum", SyntaxKind::EnumKw),
    ("module", SyntaxKind::ModuleKw),
    ("mod", SyntaxKind::ModKw),
    ("private", SyntaxKind::PrivateKw),
    ("impl", SyntaxKind::ImplKw),
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
    ("==", SyntaxKind::EqualsEquals),
    ("-", SyntaxKind::Minus),
    ("+", SyntaxKind::Plus),
    ("*", SyntaxKind::Star),
    ("~", SyntaxKind::Tilde),
    (".", SyntaxKind::Dot),
    ("/", SyntaxKind::Slash),
    ("|", SyntaxKind::Pipe),
    (">", SyntaxKind::Greater),
    ("<", SyntaxKind::Less),
    ("^", SyntaxKind::Caret),
    ("#", SyntaxKind::Hash),
    ("$", SyntaxKind::Dollar),
    ("?", SyntaxKind::Question),
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
    /// interface marks it as such wherever it appears. `import` is a
    /// structural keyword and not this: it brings in names, not material.
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
            | SyntaxKind::EqualsEquals
            | SyntaxKind::Minus
            | SyntaxKind::Plus
            | SyntaxKind::Star
            | SyntaxKind::Tilde
            | SyntaxKind::Dot
            | SyntaxKind::Slash
            | SyntaxKind::Pipe
            | SyntaxKind::Greater
            | SyntaxKind::Less
            | SyntaxKind::Caret
            | SyntaxKind::Hash
            | SyntaxKind::Dollar
            | SyntaxKind::Question => Self::Punctuation,

            SyntaxKind::UseKw => Self::Use,

            SyntaxKind::ImportKw
            | SyntaxKind::SyntaxKw
            | SyntaxKind::PieceKw
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
            | SyntaxKind::SignatureKw
            | SyntaxKind::StructureKw
            | SyntaxKind::DataKw
            | SyntaxKind::RecordKw
            | SyntaxKind::EnumKw
            | SyntaxKind::ModuleKw
            | SyntaxKind::ModKw
            | SyntaxKind::PrivateKw
            | SyntaxKind::ImplKw
            | SyntaxKind::CrescendoKw
            | SyntaxKind::DiminuendoKw
            | SyntaxKind::ToKw
            | SyntaxKind::BarKw
            | SyntaxKind::AssertKw
            | SyntaxKind::SenzaKw
            | SyntaxKind::EndingKw
            | SyntaxKind::FragmentKw
            | SyntaxKind::MobileKw
            | SyntaxKind::ImproviseKw
            | SyntaxKind::OverKw
            | SyntaxKind::LetKw
            | SyntaxKind::FnKw
            | SyntaxKind::MusicKw
            | SyntaxKind::EventsKw
            | SyntaxKind::QuoteKw
            | SyntaxKind::OptionKw
            | SyntaxKind::ListKw
            | SyntaxKind::ResultKw
            | SyntaxKind::MatchKw
            | SyntaxKind::IfKw
            | SyntaxKind::ElseKw
            | SyntaxKind::SomeKw
            | SyntaxKind::NoneKw
            | SyntaxKind::OkKw
            | SyntaxKind::ErrKw
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
            | SyntaxKind::SyntaxRegion
            | SyntaxKind::SyntaxGroup
            | SyntaxKind::ImplDecl
            | SyntaxKind::BinaryExpr
            | SyntaxKind::MethodCallExpr
            | SyntaxKind::IndexExpr
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
            | SyntaxKind::AssertStmt
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
            | SyntaxKind::ResultType
            | SyntaxKind::NameExpr
            | SyntaxKind::LiteralExpr
            | SyntaxKind::ParenExpr
            | SyntaxKind::BlockExpr
            | SyntaxKind::ProductExpr
            | SyntaxKind::ListExpr
            | SyntaxKind::OptionExpr
            | SyntaxKind::ResultExpr
            | SyntaxKind::ApplyExpr
            | SyntaxKind::LambdaExpr
            | SyntaxKind::PitchExpr
            | SyntaxKind::ExprArgList
            | SyntaxKind::ExprArg
            | SyntaxKind::MatchExpr
            | SyntaxKind::MatchArm
            | SyntaxKind::IfExpr
            | SyntaxKind::RecordUpdateExpr
            | SyntaxKind::FieldUpdate
            | SyntaxKind::QuestionExpr
            | SyntaxKind::Pattern
            | SyntaxKind::MusicExpr
            | SyntaxKind::EventsQuote
            | SyntaxKind::EventsHole
            | SyntaxKind::QuoteExpr
            | SyntaxKind::QuotePattern
            | SyntaxKind::Splice
            | SyntaxKind::SequenceSplice
            | SyntaxKind::ScaleExpr
            | SyntaxKind::KeyExpr
            | SyntaxKind::StepExpr
            | SyntaxKind::InScaleStmt
            | SyntaxKind::ChordExpr
            | SyntaxKind::StackStmt
            | SyntaxKind::TemplateDecl
            | SyntaxKind::MakeStmt
            | SyntaxKind::SignatureDecl
            | SyntaxKind::SignatureMember
            | SyntaxKind::StructureDecl
            | SyntaxKind::ModDecl
            | SyntaxKind::DataDecl
            | SyntaxKind::TypeParams
            | SyntaxKind::TypeParam
            | SyntaxKind::IndexParam
            | SyntaxKind::DataVariant
            | SyntaxKind::DataField
            | SyntaxKind::RecordDecl
            | SyntaxKind::FieldDecl
            | SyntaxKind::EnumDecl
            | SyntaxKind::EnumCase
            | SyntaxKind::RecordLiteralExpr
            | SyntaxKind::FieldInit
            | SyntaxKind::PathExpr
            | SyntaxKind::RecordPattern
            | SyntaxKind::FieldPattern
            | SyntaxKind::FieldPath
            | SyntaxKind::AppliedType
            | SyntaxKind::IndexedType
            | SyntaxKind::DataMember => return None,
        };
        Some(class)
    }
}

/// The keywords a module name may borrow: `harmony`, `pitch`, `scale` — the
/// parser's `MODULE_NAME` minus `Identifier`.
///
/// The lexer writes the keyword token wherever the word appears, and the
/// path position is what makes the word a name. So the class is decided
/// twice: once by kind (above, in [`TokenClass::of`]) and once by where the
/// token stands (below, in [`classify`]). The list is public so that every
/// highlighter the project ships — the LSP's semantic tokens, the desktop
/// editor's generated tables, the VS Code grammar's generator — works from
/// the one vocabulary instead of carrying its own copy.
///
/// `stdlib/src/list.musa` and `stdlib/src/option.musa` are no longer here
/// because their names are no longer keywords: the types they hold are
/// `List` and `Option`, and a file name is written the way a file name is.
pub const MODULE_NAME_KEYWORDS: &[SyntaxKind] = &[SyntaxKind::HarmonyKw, SyntaxKind::PitchKw, SyntaxKind::ScaleKw];

/// Every token of the source with its class, *where it stands* accounted
/// for.
///
/// [`TokenClass::of`] answers by kind alone, which is right for everything
/// but the module-name positions: in `import std::harmony;` the word
/// `harmony` is a name the composer is reaching for, and in `mod list;` the
/// word `list` names a child of the package — the parser's `MODULE_NAME`
/// says so, and an editor that colors them as keywords is telling a lie the
/// language never told. So this walks the (total, error-tolerant) parse,
/// collects the tokens those statements read as names, and reclassifies
/// them `Keyword → Name`. Everything else is exactly `TokenClass::of`.
///
/// A type name travels the other way. `Pitch` is an identifier to the lexer,
/// because a type is spelled with a capital and the language owns no other
/// mechanism for saying so; but the composer did not name it, and in
/// `let root: Pitch` the word is vocabulary. So a `TypeName` reclassifies
/// `Name → Keyword`, by the same rule and for the same reason.
///
/// The pair with the token keeps the answer honest on half-typed source:
/// the lexer is total and the parse recovers, so this is total too — the
/// LSP's semantic tokens depend on that.
pub fn classify(source: &str) -> Vec<(crate::Token, Option<TokenClass>)> {
    let parsed = crate::parse(source);
    let root = parsed.syntax();
    let mut names = std::collections::HashSet::new();
    let mut vocabulary = std::collections::HashSet::new();
    for node in root.descendants() {
        let type_name = node.kind() == SyntaxKind::TypeName;
        if !type_name && !matches!(node.kind(), SyntaxKind::ImportStmt | SyntaxKind::ModDecl) {
            continue;
        }
        let borrowed = if type_name { &mut vocabulary } else { &mut names };
        for element in node.children_with_tokens() {
            let Some(token) = element.into_token() else {
                continue;
            };
            if node.kind() == SyntaxKind::TypeName || MODULE_NAME_KEYWORDS.contains(&token.kind()) {
                borrowed.insert(token.text_range());
            }
        }
    }
    crate::lex(source)
        .tokens()
        .iter()
        .map(|&token| {
            let class = TokenClass::of(token.kind).map(|class| {
                if class == TokenClass::Keyword && names.contains(&token.range) {
                    TokenClass::Name
                } else if class == TokenClass::Name && vocabulary.contains(&token.range) {
                    TokenClass::Keyword
                } else {
                    class
                }
            });
            (token, class)
        })
        .collect()
}
