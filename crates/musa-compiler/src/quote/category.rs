//! The two closed vocabularies a transformer writes in: the token kinds
//! the lexer produces, and the delimiters a group can wear.
//!
//! One concern of the `syntax` module; see its docs for the calculus.

/// The lexer's own token kinds, under the names an adapter writes them by.
///
/// The phase's `TokenKind` *is* [`musa_syntax::SyntaxKind`] rather than a
/// parallel enum, so there is no second set of cases to fall out of step with
/// the lexer's. What this adds is the spelling, and the spelling cannot
/// disagree with the kind because `stringify!` writes it from the same
/// identifier. The one thing the macro cannot say is that the list is
/// *complete*, so a drift test says it, and the partition it checks has three
/// parts rather than two: every kind the lexer can produce appears here, every
/// kind the parser mints from a composite literal's spelling
/// ([`musa_syntax::SyntaxKind::is_literal_part`]) appears here, and no parser
/// node kind does. A part is here because an adapter that can reach a
/// numerator and cannot say `TokenKind.RationalNumerator` about it has been
/// handed half an operation.
macro_rules! token_kinds {
    ($($case:ident),* $(,)?) => {
        pub(crate) const TOKEN_KINDS: &[(&str, musa_syntax::SyntaxKind)] =
            &[$((stringify!($case), musa_syntax::SyntaxKind::$case)),*];
    };
}

token_kinds!(
    Whitespace,
    LineComment,
    BlockComment,
    Identifier,
    Integer,
    Float,
    Rational,
    String,
    PitchLiteral,
    IntervalLiteral,
    PitchLetter,
    PitchAccidental,
    PitchOctave,
    IntervalQuality,
    IntervalSize,
    RationalNumerator,
    RationalDenominator,
    UnitHz,
    UnitMs,
    UnitS,
    UnitDb,
    UnitBpm,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    LParen,
    RParen,
    Semicolon,
    Comma,
    Colon,
    Arrow,
    PipeForward,
    Equals,
    EqualsEquals,
    Minus,
    Plus,
    Star,
    Tilde,
    Dot,
    Slash,
    Pipe,
    Greater,
    Less,
    Caret,
    Hash,
    Dollar,
    Question,
    PieceKw,
    TempoKw,
    MeterKw,
    KeyKw,
    SubtitleKw,
    ComposerKw,
    ArrangerKw,
    CopyrightKw,
    MotifKw,
    ScoreKw,
    PartKw,
    VoiceKw,
    ClefKw,
    UseKw,
    ImportKw,
    SyntaxKw,
    ModKw,
    TransposeKw,
    DownKw,
    UpKw,
    RestKw,
    RepeatKw,
    SlurKw,
    DynamicKw,
    TupletKw,
    PerformanceKw,
    ProfileKw,
    MarkKw,
    GrooveKw,
    GraceKw,
    StudioKw,
    PatchKw,
    ModulateKw,
    BusKw,
    AssignKw,
    RouteKw,
    SendKw,
    MasterKw,
    AtKw,
    OutputKw,
    PitchKw,
    StretchKw,
    RetrogradeKw,
    InvertKw,
    AroundKw,
    WithKw,
    NoteKw,
    PhraseKw,
    SectionKw,
    HarmonyKw,
    LibraryKw,
    CrescendoKw,
    DiminuendoKw,
    ToKw,
    BarKw,
    AssertKw,
    SenzaKw,
    EndingKw,
    FragmentKw,
    MobileKw,
    ImproviseKw,
    OverKw,
    LetKw,
    FnKw,
    MusicKw,
    EventsKw,
    OptionKw,
    ListKw,
    ResultKw,
    MatchKw,
    IfKw,
    ElseKw,
    SomeKw,
    NoneKw,
    OkKw,
    ErrKw,
    TrueKw,
    FalseKw,
    ScaleKw,
    DegreeKw,
    FrameKw,
    InKw,
    StepKw,
    ChordKw,
    StackKw,
    AsKw,
    DataKw,
    RecordKw,
    EnumKw,
    PrivateKw,
    ImplKw,
    QuoteKw,
    Error,
);

/// How a syntax value parses — `../rules/language/11-quotation.md` §1's index.
///
/// **Two cases, not four.** Prompt 131 wrote `Item` and `Pattern` beside them
/// and prompt 132's trial found that no program constructs either. `Item` comes
/// back when an adapter expands a region into declarations rather than into an
/// expression, which no planned adapter does; re-adding it is a case here and
/// its round-trip test.
///
/// The index is a claim about how the tree parses, and the representation is
/// the same either way. A refined claim is introduced only by an operation that
/// establishes it — today, [`as_expression`]'s checked parse — and it is
/// forgotten wherever it is not needed, which is one acceptance rule in the
/// checker rather than a `forget` an author writes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum Cat {
    /// The real parser read this tree as an expression.
    Expr,
    /// Nothing is claimed about how this tree parses.
    TokenTree,
}

impl Cat {
    /// The name this category is written by, inside `Syntax<…>`.
    pub(crate) const fn name(self) -> &'static str {
        match self {
            Self::Expr => "Expr",
            Self::TokenTree => "TokenTree",
        }
    }

    /// The category a written name denotes.
    pub(crate) fn named(text: &str) -> Option<Self> {
        [Self::Expr, Self::TokenTree]
            .into_iter()
            .find(|candidate| candidate.name() == text)
    }
}

/// The token kind `TokenKind.<case>` names.
///
/// A linear scan, because it runs once per name an adapter writes and the
/// alternative is a second ordering to keep in step with the first.
pub(crate) fn token_kind_named(case: &str) -> Option<musa_syntax::SyntaxKind> {
    TOKEN_KINDS
        .iter()
        .find(|(name, _)| *name == case)
        .map(|(_, kind)| *kind)
}

/// The name `TokenKind.<case>` gives a kind, which is [`token_kind_named`]
/// read the other way round.
///
/// Answers a name for every kind in the table and `"Error"` for one outside it,
/// because the drift test already holds the table to the lexer and a diagnostic
/// is not the place to discover that it slipped.
pub(crate) fn token_kind_spelling(kind: musa_syntax::SyntaxKind) -> &'static str {
    TOKEN_KINDS
        .iter()
        .find(|(_, known)| *known == kind)
        .map_or("Error", |(name, _)| *name)
}

/// The five delimiters the fixed grouper knows.
///
/// A type rather than the spellings it used to be. A transformer named one as
/// text and the gate checked afterwards that the text named something real;
/// now there is nothing to check, because the only values are these five and
/// the phase offers them by name (`../rules/language/11-quotation.md` §4).
/// `syntax_group`'s ownership entry claimed to hide "the fixed grouper's
/// delimiter set", and this is what hides it instead.
///
/// Two of them open and close nothing, and they differ in exactly one thing —
/// whether the children are separate words. [`Self::Layout`] holds siblings
/// together by their layout and writes a space between them; [`Self::Fused`]
/// holds the parts of *one lexeme* together and writes nothing, because `c # 5`
/// is three things to the reader that reads the text back and `c#5` is one.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum Delimiter {
    Parentheses,
    Brackets,
    Braces,
    /// No delimiter at all — siblings held together by their layout.
    Layout,
    /// No delimiter and no separator either — the parts of one lexeme.
    ///
    /// Admitted only for the three composite literals, and the gate is where
    /// that is decided rather than here: a delimiter says how the children are
    /// written, and [`crate::quote::check_expression`] says whether the reader
    /// would have read the result as one token of one of the three kinds.
    Fused,
}

impl Delimiter {
    /// Every delimiter, in the order the grouper tries them.
    ///
    /// The two whose pair is empty are never *matched*: [`Self::Fused`] is
    /// chosen by the node's own kind before the loop runs, and [`Self::Layout`]
    /// is the fallback the loop falls out to. Either one tried against a node's
    /// outermost tokens would match every node and swallow the other three.
    pub(crate) const ALL: [Self; 5] = [
        Self::Parentheses,
        Self::Brackets,
        Self::Braces,
        Self::Layout,
        Self::Fused,
    ];

    /// The name the phase spells this delimiter by, after `Delimiter.`.
    pub(crate) const fn name(self) -> &'static str {
        match self {
            Self::Parentheses => "Parentheses",
            Self::Brackets => "Brackets",
            Self::Braces => "Braces",
            Self::Layout => "Layout",
            Self::Fused => "Fused",
        }
    }

    /// What stands between two children of a group of this delimiter.
    ///
    /// One space everywhere but [`Self::Fused`], where the children are the
    /// parts of one lexeme and anything between them would make it several.
    pub(crate) const fn separator(self) -> &'static str {
        match self {
            Self::Parentheses | Self::Brackets | Self::Braces | Self::Layout => " ",
            Self::Fused => "",
        }
    }

    /// The text that opens and closes a group of this delimiter, both empty
    /// for [`Self::Layout`] and [`Self::Fused`].
    pub(crate) const fn pair(self) -> (&'static str, &'static str) {
        match self {
            Self::Parentheses => ("(", ")"),
            Self::Brackets => ("[", "]"),
            Self::Braces => ("{", "}"),
            Self::Layout | Self::Fused => ("", ""),
        }
    }

    /// The delimiter `Delimiter.<case>` names.
    pub(crate) fn named(case: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|candidate| candidate.name() == case)
    }
}

/// A category, under the name it is written by inside `Syntax<…>`.
///
/// It is the index of `Syntax` in [`crate::registry`], so this is what a
/// diagnostic shows when a syntax type is printed.
impl std::fmt::Display for Cat {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        out.write_str(self.name())
    }
}

/// A delimiter, under the name the phase offers it by.
impl std::fmt::Display for Delimiter {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        out.write_str(self.name())
    }
}
