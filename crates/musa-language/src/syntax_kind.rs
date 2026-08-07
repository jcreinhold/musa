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
    /// Written pitch: letter `a`–`g`, optional accidental (`s`/`ss` sharp,
    /// `f`/`ff` flat, `n` natural — `LilyPond` English convention), octave
    /// digits with optional `-` sign: `c5`, `gs4`, `bff2`, `a-1`.
    PitchLiteral,
    /// Interval: quality `P`/`M`/`m` plus size — `P5`, `M3`, `m3`.
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
    /// `.` — the path separator in a modulation target.
    Dot,

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
    /// `chord`
    ChordKw,
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
    /// `articulation`
    ArticulationKw,
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
    /// `chord [<pitch>, ...] <duration>;`
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
    /// The articulation names trailing a note or chord's duration. Their own
    /// node so a pitch reference and an articulation name — both bare
    /// identifiers — never have to be told apart by position.
    ArticulationList,
    /// `performance { ... }`
    PerformanceDecl,
    /// `profile name { ... }` inside a `performance` block.
    ProfileDecl,
    /// `articulation <name> { ... }` inside a profile.
    ArticulationRule,
    /// `dynamic <mark> { ... }` inside a profile.
    DynamicRule,
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
}

impl SyntaxKind {
    /// Whether this kind is whitespace or a comment. Trivia tokens are kept
    /// in the tree (losslessness) but skipped by the parser.
    pub fn is_trivia(self) -> bool {
        matches!(self, Self::Whitespace | Self::LineComment | Self::BlockComment)
    }
}
