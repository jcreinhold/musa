//! Token and syntax-node kinds for the `.musa` language.
//!
//! `SyntaxKind` serves double duty (rust-analyzer style): it classifies the
//! tokens produced by the lexer now, and it will classify the nodes of the
//! lossless concrete syntax tree built by the parser (prompt 03). The numeric
//! representation is stable because the Rowan tree stores kinds as `u16`.

/// The kind of a lexical token or syntax node.
#[repr(u16)]
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, num_enum::IntoPrimitive, num_enum::FromPrimitive,
)]
pub enum SyntaxKind {
    // --- Trivia: preserved for losslessness, never significant to the parser.
    /// Runs of spaces, tabs, and newlines (newlines are trivia, not syntax).
    Whitespace,
    /// `// ...` to end of line.
    LineComment,
    /// `/* ... */`, not nested.
    BlockComment,

    // --- Literals.
    /// ASCII letters and underscores; keywords and unit suffixes are lexed as
    /// their own kinds, so this is everything else (`sigh`, `glass_pad`).
    Identifier,
    /// `[0-9]+` — whole-note counts, tempos, repeat counts.
    Integer,
    /// `[0-9]+.[0-9]+` — unitless controls such as `q: 0.7`.
    Float,
    /// `[0-9]+/[0-9]+` — durations (`1/4`, `3/8`) and meters (`4/4`).
    Rational,
    /// `"..."` with `\`-escapes; newlines terminate (and are an error).
    String,
    /// Written pitch: letter `a`–`g`, an optional run of `#` or `b` (or
    /// explicit `n`), and octave digits with optional `-` sign: `c5`,
    /// `g#4`, `bbb2`, `a-1`.
    PitchLiteral,
    /// Named interval: `P`/`M`/`m`, repeated `A`/`d`, or `dim`, then a
    /// size — `P5`, `M10`, `AA4`, `dim5`, `ddd7`. `dim` disambiguates a
    /// singly diminished interval from the written pitch `d4`.
    IntervalLiteral,

    // --- Unit suffixes (roadmap §7.2: units are part of the syntax).
    /// `Hz`
    UnitHz,
    /// `ms`
    UnitMs,
    /// `s`
    UnitS,
    /// `dB`
    UnitDb,
    /// `bpm`
    UnitBpm,

    // --- Punctuation.
    /// `{`
    LBrace,
    /// `}`
    RBrace,
    /// `[`
    LBracket,
    /// `]`
    RBracket,
    /// `(`
    LParen,
    /// `)`
    RParen,
    /// `;`
    Semicolon,
    /// `,`
    Comma,
    /// `:`
    Colon,
    /// `->`
    Arrow,
    /// `|>`
    PipeForward,
    /// `=`
    Equals,
    /// `-` (signs are assembled by the parser, not the lexer)
    Minus,
    /// `~` — the tie mark, postfix on a note or chord statement.
    Tilde,
    /// `.` — the path separator in a modulation target, and the augmentation
    /// dot on a short-form duration (`c4/4.`).
    Dot,
    /// `/` — the duration separator (`c4/4`), and the fraction bar inside a
    /// [`SyntaxKind::Rational`], which the lexer keeps whole.
    Slash,
    /// `|` — the barline.
    Pipe,
    /// `>` — the accent mark.
    Greater,
    /// `^` — the marcato mark.
    Caret,
    /// `#` — the sharp.
    Hash,

    // --- Structural keywords. Processor names (`oscillator`, `lowpass`, …)
    /// are deliberately *not* keywords: they lex as identifiers so the
    /// studio vocabulary can grow without lexer changes (prompt 19).
    /// `piece`
    PieceKw,
    /// `tempo`
    TempoKw,
    /// `meter`
    MeterKw,
    /// `key`
    KeyKw,
    /// `subtitle`
    SubtitleKw,
    /// `composer`
    ComposerKw,
    /// `arranger`
    ArrangerKw,
    /// `copyright`
    CopyrightKw,
    /// `motif`
    MotifKw,
    /// `score`
    ScoreKw,
    /// `part`
    PartKw,
    /// `voice`
    VoiceKw,
    /// `clef`
    ClefKw,
    /// `use`
    UseKw,
    /// `transpose`
    TransposeKw,
    /// `down`
    DownKw,
    /// `up`
    UpKw,
    /// `rest`
    RestKw,
    /// `repeat`
    RepeatKw,
    /// `slur`
    SlurKw,
    /// `dynamic`
    DynamicKw,
    /// `tuplet`
    TupletKw,
    /// `performance`
    PerformanceKw,
    /// `profile`
    ProfileKw,
    /// `mark`
    MarkKw,
    /// `groove`
    GrooveKw,
    /// `grace`
    GraceKw,
    /// `studio`
    StudioKw,
    /// `patch`
    PatchKw,
    /// `modulate`
    ModulateKw,
    /// `bus`
    BusKw,
    /// `assign`
    AssignKw,
    /// `route`
    RouteKw,
    /// `send`
    SendKw,
    /// `master`
    MasterKw,
    /// `at`
    AtKw,
    /// `output`
    OutputKw,
    /// `pitch` (motif parameter type)
    PitchKw,
    /// `stretch`
    StretchKw,
    /// `retrograde`
    RetrogradeKw,
    /// `invert`
    InvertKw,
    /// `around`
    AroundKw,
    /// `with`
    WithKw,
    /// `note`
    NoteKw,
    /// `phrase`
    PhraseKw,
    /// `section`
    SectionKw,
    /// `harmony`
    HarmonyKw,
    /// `library`
    LibraryKw,
    /// `crescendo`
    CrescendoKw,
    /// `diminuendo`
    DiminuendoKw,
    /// `to`
    ToKw,
    /// `bar`
    BarKw,
    /// `senza`
    SenzaKw,
    /// `ending`
    EndingKw,
    /// `fragment`
    FragmentKw,
    /// `mobile`
    MobileKw,
    /// `improvise`
    ImproviseKw,
    /// `over`
    OverKw,
    /// `let`
    LetKw,
    /// `fn`
    FnKw,
    /// `music`
    MusicKw,
    /// `option`
    OptionKw,
    /// `list`
    ListKw,
    /// `match`
    MatchKw,
    /// `some`
    SomeKw,
    /// `none`
    NoneKw,
    /// `true`
    TrueKw,
    /// `false`
    FalseKw,
    /// `scale`
    ScaleKw,
    /// `degree`
    DegreeKw,
    /// `frame`
    FrameKw,
    /// `in`
    InKw,
    /// `step`
    StepKw,
    /// `chord`
    ChordKw,
    /// `stack`
    StackKw,

    /// A span the lexer could not recognize; emitted so the token stream
    /// stays lossless even for invalid input. Also used for parser error
    /// nodes and as the fallback for unknown raw kinds.
    #[num_enum(default)]
    Error,

    // --- Nodes (produced by the parser, never by the lexer).
    /// Root of a parsed document.
    Root,
    /// `piece "name" { ... }`
    PieceDecl,
    /// `tempo <beat> = <bpm>;`
    TempoStmt,
    /// `meter <n>/<d>;`
    MeterStmt,
    /// `key <pitch-class> <mode>;`
    KeyStmt,
    /// `composer "…";` and its three siblings — one node kind for all four,
    /// because they differ only in which keyword opens them.
    FrontMatterStmt,
    /// `motif name(params) { ... }`
    MotifDecl,
    /// `score { ... }`
    ScoreDecl,
    /// `part name { ... }`
    PartDecl,
    /// `clef <name>;`
    ClefStmt,
    /// `voice name { ... }`
    VoiceDecl,
    /// `<pitch-or-ref> <duration>;`
    NoteStmt,
    /// `rest <duration>;`
    RestStmt,
    /// `[<pitch> ...] <duration>;`
    ChordStmt,
    /// `use name(args);`
    UseStmt,
    /// `transpose up|down <interval> { ... }`
    TransposeStmt,
    /// `repeat <n> { ... }`
    RepeatStmt,
    /// `slur { ... }`
    SlurStmt,
    /// `dynamic <mark>;`
    DynamicStmt,
    /// `tuplet <n>/<d> { ... }`
    TupletStmt,
    /// `stretch <n>/<d> { ... }`
    StretchStmt,
    /// `retrograde { ... }`
    RetrogradeStmt,
    /// `invert around <pitch> { ... }`
    InvertStmt,
    /// `with { ... }` — the overrides specializing one motif occurrence.
    WithClause,
    /// `note <n> = <pitch>;` — one override inside a [`SyntaxKind::WithClause`].
    OverrideStmt,
    /// `phrase "A" { ... }` — a named span over a voice's music.
    PhraseStmt,
    /// `mark breath;`, `mark text "dolce";`, `mark pedal { ... }` — a notation
    /// mark that is not written on a note. One node for every such mark,
    /// because the vocabulary decides the shape and the grammar does not.
    MarkStmt,
    /// `grace { c5 d5 }` — the grace notes crushed before the note that
    /// follows. Not a mark: each has a pitch, an accidental and a place in an
    /// order that matters, which is an identity.
    GraceStmt,
    /// One pitch inside a `grace` block: a notehead with no written duration.
    GraceNote,
    /// `section "Exposition" at 1:1;` — a form marker in the score.
    SectionStmt,
    /// `harmony { ... }` — the chord-symbol lane.
    HarmonyDecl,
    /// `at 1:1 am;` — one chord symbol at a position, inside a
    /// [`SyntaxKind::HarmonyDecl`].
    HarmonyStmt,
    /// `1:1` — a measure:beat position.
    Position,
    /// `fmaj7`, `f#m7` — a chord symbol, as written.
    ChordSymbol,
    /// `a`, `g#`, `bb` — a pitch class with no octave, as a key tonic is
    /// written.
    ///
    /// Its own node because a sharp is a token and a flat is not: `bb` lexes
    /// as one identifier while `g#` lexes as two, so the tonic of a key is
    /// one token or three depending on which accidental it carries. The node
    /// makes that a fact about the tree instead of a count the reader has to
    /// get right.
    PitchClass,
    /// `library { ... }` — a file of shared declarations, importable by a
    /// piece. Root of a library file, in place of a [`SyntaxKind::PieceDecl`].
    LibraryDecl,
    /// `use "../library/motifs.musa";` — a relative import.
    ImportStmt,
    /// `crescendo to f { ... }` / `diminuendo to p { ... }` — a hairpin over
    /// the notes it wraps.
    HairpinStmt,
    /// `1/4`, `1`, `/4`, `/4.`, or a duration parameter's name — how long one
    /// note, rest or chord lasts, with the longest it may be held after `to`.
    /// Its own node for the same reason [`SyntaxKind::ArticulationList`] is:
    /// the numeral in `c4/4` is a `4` like any other, and a reader that finds
    /// a duration by taking the first numeral under the statement would call
    /// every quarter a whole note without ever failing.
    Duration,
    /// The articulation names trailing a note or chord's duration. Their own
    /// node so a pitch reference and an articulation name — both bare
    /// identifiers — never have to be told apart by position.
    ArticulationList,
    /// `performance { ... }`
    PerformanceDecl,
    /// `profile name { ... }` inside a `performance` block.
    ProfileDecl,
    /// `mark <name> { ... }` inside a profile.
    MarkRule,
    /// `dynamic <mark> { ... }` inside a profile.
    DynamicRule,
    /// `groove <name> { ... }` inside a profile.
    GrooveRule,
    /// `grace { steal = 1/16; from = principal; }` inside a profile — how
    /// this reading plays the grace notes the score writes.
    GraceRule,
    /// `<name> = <number> [unit];` inside a rule.
    SettingStmt,
    /// `profile <name>;` inside a part: which profile realizes it.
    ProfileStmt,
    /// `studio { ... }`
    StudioDecl,
    /// `patch <name> { ... }`
    PatchDecl,
    /// `bus <name> { ... }`
    BusDecl,
    /// `<name> = <chain>;` — a named signal.
    SignalBinding,
    /// `<chain>;` — an unnamed chain, terminal in its patch or bus.
    ChainStmt,
    /// `<stage> |> <stage> |> ...` — the signal chain itself.
    SignalChain,
    /// `<name>(<args>)` — a processor construction.
    CallExpr,
    /// The parenthesized arguments of a [`SyntaxKind::CallExpr`].
    ArgList,
    /// One argument: `<name>: <value>` or a positional `<value>`.
    Arg,
    /// A number with an optional unit suffix, possibly negated.
    ValueLiteral,
    /// A bare name used as a value: another signal, or `output`.
    NameRef,
    /// `modulate <signal> -> <patch>.<stage>.<parameter>;`
    ModulateStmt,
    /// `<patch>.<stage>.<parameter>` — a modulation target.
    ParamPath,
    /// `assign <part> -> <patch>;`
    AssignStmt,
    /// `route <source> -> <destination>;`
    RouteStmt,
    /// `send <source> -> <bus> at <gain> dB;`
    SendStmt,
    /// `{ ... }` body of a motif, transpose, or repeat.
    Block,
    /// `bar { ... }` / `bar head { ... }` — one measure, written down.
    BarStmt,
    /// `senza { ... }` — a stretch with no barlines, and the meter back after.
    SenzaStmt,
    /// `ending 1 { ... }` — what a repeat plays on one of its passes.
    EndingStmt,
    /// `fragment a { ... }` — material a performance may reorder or repeat.
    FragmentDecl,
    /// `mobile { a; b; c; }` — its fragments in an order the performance
    /// chooses.
    MobileStmt,
    /// `improvise 8/1 over "Dm7 | G7";` — a notated frame with unnotated
    /// contents.
    ImproviseStmt,
    /// `let name: type = expression;`
    LetDecl,
    /// `fn name(parameters) -> type = expression;`
    FnDecl,
    /// One annotated parameter, with an optional default expression.
    Param,
    /// A comma-separated parameter list.
    ParamList,
    /// A primitive, product, option, list, or arrow type.
    TypeExpr,
    /// A reference to a primitive or named type.
    TypeName,
    /// `left -> right`, right associative.
    FunctionType,
    /// `(left, right, ...)` in a type position.
    ProductType,
    /// `option[type]`.
    OptionType,
    /// `list[type]`.
    ListType,
    /// A reference to a value by name.
    NameExpr,
    /// A core scalar literal.
    LiteralExpr,
    /// `(expression)`.
    ParenExpr,
    /// `(left, right, ...)` in a value position.
    ProductExpr,
    /// `[left, right, ...]` in a value position.
    ListExpr,
    /// `some(expression)` or `none`.
    OptionExpr,
    /// Ordinary function application.
    ApplyExpr,
    /// Spelling-preserving written-pitch translation: `pitch up M2`.
    PitchExpr,
    /// The comma-separated arguments of an ordinary application.
    ExprArgList,
    /// One positional or named ordinary-call argument.
    ExprArg,
    /// `match value { pattern -> result, ... }`.
    MatchExpr,
    /// One pattern and result in a match.
    MatchArm,
    /// A literal, binding, option, list, or product pattern.
    Pattern,
    /// `music { ... }`, a notation-first contextual music value.
    MusicExpr,
    /// `scale c dorian` — a collection rooted on a spelled tonic class.
    ScaleExpr,
    /// `key c minor` in a value position, which is the tonal fact and not a
    /// statement that changes the key.
    KeyExpr,
    /// `pitch step <n>` — motion by scale steps, read against the scale in
    /// force rather than against a written interval.
    StepExpr,
    /// `in scale <expr> { ... }` — the lexically scoped pitch context.
    InScaleStmt,
    /// `chord c major7` — rooted spelled content, with no register.
    ChordExpr,
    /// `stack c4 major7/2` — the close-position sugar, which is an event and
    /// not a value.
    StackStmt,
}

impl SyntaxKind {
    /// Whether this kind is whitespace or a comment. Trivia tokens are kept
    /// in the tree (losslessness) but skipped by the parser.
    pub fn is_trivia(self) -> bool {
        matches!(self, Self::Whitespace | Self::LineComment | Self::BlockComment)
    }
}
