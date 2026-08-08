//! Hand-written recursive-descent parser producing a lossless Rowan tree.
//!
//! Architecture (roadmap §10.2–§10.4): the parser emits a flat event stream
//! (`StartNode` / `Token` / `FinishNode`); a second pass replays the events
//! into a `rowan::GreenNodeBuilder`. Parser code never touches Rowan. The
//! grammar is small enough that statements dispatch on their first token, so
//! no forward-parent machinery is needed.
//!
//! Recovery: an unexpected token records an error, then tokens are wrapped in
//! an `ERROR` node until a recovery point (`;`, `}`, or the next declaration
//! keyword), and parsing continues. The parser always returns a tree.

use rowan::GreenNodeBuilder;
use text_size::{TextRange, TextSize};

use crate::language::SyntaxNode;
use crate::{Lexed, SyntaxError, SyntaxKind, Token, lex};

/// The result of parsing a source string.
///
/// Always contains a tree, even for invalid input (roadmap §10.3). Check
/// [`ParsedDocument::errors`] for diagnostics.
pub struct ParsedDocument {
    node: SyntaxNode,
    errors: Vec<SyntaxError>,
}

impl ParsedDocument {
    /// The root of the lossless concrete syntax tree. Cloning the node is
    /// cheap (reference-counted).
    ///
    /// Invariant: the tree's text equals the parsed source exactly.
    pub fn syntax(&self) -> SyntaxNode {
        self.node.clone()
    }

    /// Lexical and parse errors with source spans, in source order.
    pub fn errors(&self) -> &[SyntaxError] {
        &self.errors
    }
}

/// Parse `source` into a lossless tree plus diagnostics.
pub fn parse(source: &str) -> ParsedDocument {
    let lexed = lex(source);
    let mut errors: Vec<SyntaxError> = lexed
        .errors()
        .iter()
        .map(|error| {
            let (message, label, help) = match error.kind() {
                crate::LexErrorKind::InvalidToken => (
                    "musa does not read this",
                    "not part of the language",
                    "check for a stray character, or a word from another notation",
                ),
                crate::LexErrorKind::UnterminatedString => (
                    "this text has no closing quote",
                    "the line ends here",
                    "a title, a name, or a line of front matter is one quoted line",
                ),
                crate::LexErrorKind::UnterminatedBlockComment => (
                    "this comment is never closed",
                    "opened here",
                    "close it with `*/`, or use `//` for one line",
                ),
            };
            SyntaxError::new(error.range(), message, label).with_help(help)
        })
        .collect();
    let node = Parser::new(source, &lexed).run();
    errors.extend(node.1);
    ParsedDocument { node: node.0, errors }
}

/// One step of tree construction.
enum Event<'a> {
    StartNode(SyntaxKind),
    Token(SyntaxKind, &'a str),
    FinishNode,
}

/// Declaration keywords that bound error recovery at each level.
const PIECE_RECOVERY: &[SyntaxKind] = &[
    SyntaxKind::Semicolon,
    SyntaxKind::RBrace,
    SyntaxKind::UseKw,
    SyntaxKind::TempoKw,
    SyntaxKind::MeterKw,
    SyntaxKind::KeyKw,
    SyntaxKind::SubtitleKw,
    SyntaxKind::ComposerKw,
    SyntaxKind::ArrangerKw,
    SyntaxKind::CopyrightKw,
    SyntaxKind::MotifKw,
    SyntaxKind::ScoreKw,
    SyntaxKind::PerformanceKw,
    SyntaxKind::StudioKw,
];
/// The four front-matter keywords, which open statements of one shape.
const FRONT_MATTER: &[SyntaxKind] = &[
    SyntaxKind::SubtitleKw,
    SyntaxKind::ComposerKw,
    SyntaxKind::ArrangerKw,
    SyntaxKind::CopyrightKw,
];
const SCORE_RECOVERY: &[SyntaxKind] = &[
    SyntaxKind::RBrace,
    SyntaxKind::PartKw,
    SyntaxKind::SectionKw,
    SyntaxKind::HarmonyKw,
];
const PART_RECOVERY: &[SyntaxKind] = &[
    SyntaxKind::Semicolon,
    SyntaxKind::RBrace,
    SyntaxKind::ClefKw,
    SyntaxKind::ProfileKw,
    SyntaxKind::VoiceKw,
];
const PERFORMANCE_RECOVERY: &[SyntaxKind] = &[SyntaxKind::RBrace, SyntaxKind::ProfileKw];
const PROFILE_RECOVERY: &[SyntaxKind] = &[
    SyntaxKind::Semicolon,
    SyntaxKind::RBrace,
    SyntaxKind::ArticulationKw,
    SyntaxKind::DynamicKw,
];
const STUDIO_RECOVERY: &[SyntaxKind] = &[
    SyntaxKind::Semicolon,
    SyntaxKind::RBrace,
    SyntaxKind::PatchKw,
    SyntaxKind::BusKw,
    SyntaxKind::ModulateKw,
    SyntaxKind::AssignKw,
    SyntaxKind::RouteKw,
    SyntaxKind::SendKw,
];
const VOICE_RECOVERY: &[SyntaxKind] = &[
    SyntaxKind::Semicolon,
    SyntaxKind::RBrace,
    SyntaxKind::PitchLiteral,
    SyntaxKind::RestKw,
    SyntaxKind::ChordKw,
    SyntaxKind::UseKw,
    SyntaxKind::TransposeKw,
    SyntaxKind::RepeatKw,
    SyntaxKind::BarKw,
    SyntaxKind::SlurKw,
    SyntaxKind::PhraseKw,
    SyntaxKind::CrescendoKw,
    SyntaxKind::DiminuendoKw,
    SyntaxKind::DynamicKw,
    SyntaxKind::TupletKw,
    SyntaxKind::StretchKw,
    SyntaxKind::RetrogradeKw,
    SyntaxKind::InvertKw,
];

struct Parser<'a> {
    source: &'a str,
    tokens: &'a [Token],
    pos: usize,
    events: Vec<Event<'a>>,
    errors: Vec<SyntaxError>,
    /// Whether the end of the file has already been blamed for something.
    ///
    /// One missing `}` closes every block above it, so a file that ends early
    /// produces one complaint per open block — all at the same character, all
    /// saying the same thing. The innermost is reported first, because that is
    /// the order the parser unwinds in, and it is also the one closest to what
    /// was actually being written.
    blamed_the_end: bool,
    /// How many `bar` bodies enclose the position being parsed.
    ///
    /// Bars do not nest, and the parser is where that is said: a bar claims to
    /// be one measure, and a measure inside a measure is not a thing the
    /// notation has a mark for.
    bar_depth: u32,
}

impl<'a> Parser<'a> {
    fn new(source: &'a str, lexed: &'a Lexed) -> Self {
        Self {
            source,
            tokens: lexed.tokens(),
            pos: 0,
            events: Vec::new(),
            errors: Vec::new(),
            blamed_the_end: false,
            bar_depth: 0,
        }
    }

    /// Parse the whole document and build the tree.
    fn run(mut self) -> (SyntaxNode, Vec<SyntaxError>) {
        self.start(SyntaxKind::Root);
        // A file is a piece or a library. Which one it is is written at the
        // top of it rather than inferred from what it happens to contain: a
        // library with a `score` in it is then a parse error rather than a
        // rule someone has to remember.
        if self.at(SyntaxKind::LibraryKw) {
            self.library_decl();
        } else {
            self.piece_decl();
        }
        self.eat_trivia();
        self.finish();
        let node = SyntaxNode::new_root(self.build_tree());
        (node, self.errors)
    }

    // --- Event helpers -------------------------------------------------

    fn start(&mut self, kind: SyntaxKind) {
        self.events.push(Event::StartNode(kind));
    }

    fn finish(&mut self) {
        self.events.push(Event::FinishNode);
    }

    fn eat_trivia(&mut self) {
        while let Some(token) = self.tokens.get(self.pos) {
            if token.kind.is_trivia() {
                self.emit_token(token);
                self.advance();
            } else {
                break;
            }
        }
    }

    fn emit_token(&mut self, token: &Token) {
        let start = usize::from(token.range.start());
        let end = usize::from(token.range.end());
        if let Some(text) = self.source.get(start..end) {
            self.events.push(Event::Token(token.kind, text));
        }
    }

    fn advance(&mut self) {
        self.pos = self.pos.saturating_add(1);
    }

    /// Consume the next significant token (emitting any trivia before it).
    fn bump(&mut self) {
        self.eat_trivia();
        if let Some(token) = self.tokens.get(self.pos) {
            let token = *token;
            self.emit_token(&token);
            self.advance();
        }
    }

    /// Kind of the next significant token, or `None` at end of input.
    fn current(&self) -> Option<SyntaxKind> {
        let mut idx = self.pos;
        while let Some(token) = self.tokens.get(idx) {
            if token.kind.is_trivia() {
                idx = idx.saturating_add(1);
            } else {
                return Some(token.kind);
            }
        }
        None
    }

    fn at(&self, kind: SyntaxKind) -> bool {
        self.current() == Some(kind)
    }

    fn at_any(&self, kinds: &[SyntaxKind]) -> bool {
        self.current().is_some_and(|kind| kinds.contains(&kind))
    }

    /// The kind `n` significant tokens ahead (`0` is [`Self::current`]).
    ///
    /// The studio grammar is the only place that needs lookahead: `name =`,
    /// `name(`, and a bare `name` are three different productions that share
    /// a first token, and distinguishing them by backtracking would cost the
    /// tree its losslessness.
    fn nth_significant(&self, n: usize) -> Option<SyntaxKind> {
        let mut seen = 0usize;
        let mut idx = self.pos;
        while let Some(token) = self.tokens.get(idx) {
            idx = idx.saturating_add(1);
            if token.kind.is_trivia() {
                continue;
            }
            if seen == n {
                return Some(token.kind);
            }
            seen = seen.saturating_add(1);
        }
        None
    }

    /// Consume `kind` if present; otherwise record an error and continue.
    ///
    /// Punctuation the grammar spells out gets the better report: the message
    /// names the character rather than the production, the caret sits at the
    /// end of the token before — where the character goes, not where reading
    /// broke — and the error carries the edit that puts it there. Anything
    /// else falls back to naming what was wanted and what was found, which is
    /// all that can honestly be said about a missing name.
    fn expect(&mut self, kind: SyntaxKind, what: &str) {
        if self.at(kind) {
            self.bump();
            return;
        }
        if self.cascading() || !self.may_blame_the_end() {
            return;
        }
        let Some(text) = punctuation_of(kind) else {
            let error = self.expected_error(what);
            self.errors.push(error);
            return;
        };
        let at = TextRange::empty(self.previous_end());
        self.errors.push(
            SyntaxError::new(at, format!("missing `{text}`"), "it goes here").with_fix(format!("add `{text}`"), text),
        );
    }

    /// The end of the last token that is not whitespace or a comment.
    ///
    /// Where a missing `;` belongs: at the end of what was written, not in
    /// front of the word that made the parser notice, and not adrift in the
    /// blank line between them.
    fn previous_end(&self) -> TextSize {
        self.tokens
            .get(..self.pos)
            .unwrap_or_default()
            .iter()
            .rev()
            .find(|token| !token.kind.is_trivia())
            .map_or_else(|| TextSize::from(0), |token| token.range.end())
    }

    /// Record "expected X, found Y" at the current token without consuming.
    fn expected(&mut self, what: &str) {
        if self.cascading() {
            return;
        }
        let error = self.expected_error(what);
        self.errors.push(error);
    }

    /// The same, plus a line saying what to do.
    fn expected_with_help(&mut self, what: &str, help: &str) {
        if self.cascading() {
            return;
        }
        let error = self.expected_error(what).with_help(help);
        self.errors.push(error);
    }

    /// Whether the next token is one the lexer already rejected.
    ///
    /// A stray `*` produces one honest complaint and then three consequences —
    /// no duration, no `;`, no statement — each pointing at the same character
    /// and none of them the reason. The lexer's message is the one worth
    /// reading, so the parser says nothing more until it is past the token.
    fn cascading(&self) -> bool {
        self.significant().is_some_and(|token| token.kind == SyntaxKind::Error)
    }

    /// The shared body of the three above.
    ///
    /// Naming what was actually found is most of what makes a parse error
    /// legible: `expected ';', found '}'` locates the mistake a line earlier
    /// than `expected ';'` does, because the reader can see which construct
    /// ran off its end.
    fn expected_error(&self, what: &str) -> SyntaxError {
        match self.significant() {
            Some(token) => SyntaxError::new(
                token.range,
                format!("expected {what}, found `{}`", self.source[token.range].trim()),
                format!("expected {what}"),
            ),
            None => SyntaxError::new(
                TextRange::empty(self.end_size()),
                format!("expected {what}, found the end of the file"),
                format!("expected {what}"),
            ),
        }
    }

    /// A block that ran to the end of the file without its `}`.
    ///
    /// Reported at the end rather than at the `{`, because that is where the
    /// parser found out — but the message names the block, so the reader knows
    /// which `{` to go back to.
    fn unclosed(&mut self, what: &str) -> Option<SyntaxError> {
        if !self.may_blame_the_end() {
            return None;
        }
        Some(
            SyntaxError::new(
                TextRange::empty(self.end_size()),
                format!("this {what} is never closed"),
                "the file ends here",
            )
            .with_fix("add `}`", "}"),
        )
    }

    fn unclosed_list(&mut self) -> Option<SyntaxError> {
        if !self.may_blame_the_end() {
            return None;
        }
        Some(
            SyntaxError::new(
                TextRange::empty(self.end_size()),
                "this argument list is never closed",
                "the file ends here",
            )
            .with_fix("add `)`", ")"),
        )
    }

    /// Whether the end of the file is still available to blame.
    ///
    /// Returns `true` once, when the parser is out of tokens, and `false`
    /// every time after — and `true` unconditionally while tokens remain,
    /// because then the complaint is about a place the reader can see.
    fn may_blame_the_end(&mut self) -> bool {
        if self.significant().is_some() {
            return true;
        }
        let first = !self.blamed_the_end;
        self.blamed_the_end = true;
        first
    }

    /// The next token that is not whitespace or a comment.
    ///
    /// Errors point at what the reader would call the next token; the trivia
    /// in front of it is not what is wrong.
    fn significant(&self) -> Option<&Token> {
        self.tokens
            .get(self.pos..)?
            .iter()
            .find(|token| !token.kind.is_trivia())
    }

    fn end_size(&self) -> TextSize {
        TextSize::from(u32::try_from(self.source.len()).unwrap_or(u32::MAX))
    }

    /// Wrap tokens in an `ERROR` node until a recovery point. A recovery
    /// semicolon is consumed (it terminated the broken construct); anything
    /// else is left for the enclosing parse.
    fn recover(&mut self, recovery: &[SyntaxKind]) {
        self.start(SyntaxKind::Error);
        while let Some(kind) = self.current() {
            if recovery.contains(&kind) {
                break;
            }
            self.bump();
        }
        if recovery.contains(&SyntaxKind::Semicolon) && self.at(SyntaxKind::Semicolon) {
            self.bump();
        }
        self.finish();
    }

    fn build_tree(&self) -> rowan::GreenNode {
        use crate::language::MusaLanguage;
        use rowan::Language as _;
        let mut builder = GreenNodeBuilder::new();
        for event in &self.events {
            match event {
                Event::StartNode(kind) => {
                    builder.start_node(MusaLanguage::kind_to_raw(*kind));
                }
                Event::Token(kind, text) => {
                    builder.token(MusaLanguage::kind_to_raw(*kind), text);
                }
                Event::FinishNode => builder.finish_node(),
            }
        }
        builder.finish()
    }

    // --- Grammar --------------------------------------------------------

    /// `piece "name" { ... }`
    fn piece_decl(&mut self) {
        self.start(SyntaxKind::PieceDecl);
        self.expect(SyntaxKind::PieceKw, "`piece`");
        self.expect(SyntaxKind::String, "a piece name");
        self.expect(SyntaxKind::LBrace, "`{`");
        loop {
            if self.at(SyntaxKind::RBrace) {
                self.bump();
                break;
            }
            if self.current().is_none() {
                if let Some(error) = self.unclosed("`piece` block") {
                    self.errors.push(error);
                }
                break;
            }
            if self.at(SyntaxKind::UseKw) {
                self.import_stmt();
            } else if self.at(SyntaxKind::TempoKw) {
                self.tempo_stmt();
            } else if self.at(SyntaxKind::MeterKw) {
                self.meter_stmt();
            } else if self.at(SyntaxKind::KeyKw) {
                self.key_stmt();
            } else if self.at_any(FRONT_MATTER) {
                self.front_matter_stmt();
            } else if self.at(SyntaxKind::MotifKw) {
                self.motif_decl();
            } else if self.at(SyntaxKind::ScoreKw) {
                self.score_decl();
            } else if self.at(SyntaxKind::PerformanceKw) {
                self.performance_decl();
            } else if self.at(SyntaxKind::StudioKw) {
                self.studio_decl();
            } else {
                self.expected_with_help(
                    "a declaration",
                    "a piece holds `use`, `tempo`, `meter`, `key`, front matter, `motif`, `score`, `performance`, and `studio`",
                );
                self.recover(PIECE_RECOVERY);
            }
        }
        self.finish();
    }

    /// `composer "Name";` — one of the four front-matter statements.
    ///
    /// All four have the same shape, and which one this is lives in the
    /// keyword token the node opens with. What each means on the page is the
    /// engraver's business, not the parser's.
    fn front_matter_stmt(&mut self) {
        self.start(SyntaxKind::FrontMatterStmt);
        self.bump(); // the keyword
        self.expect(SyntaxKind::String, "a quoted line of front matter");
        self.expect(SyntaxKind::Semicolon, "`;`");
        self.finish();
    }

    /// `tempo <beat> = <bpm>;` — beat is a name (`quarter`) or fraction.
    fn tempo_stmt(&mut self) {
        self.start(SyntaxKind::TempoStmt);
        self.bump(); // tempo
        if self.at_any(&[SyntaxKind::Identifier, SyntaxKind::Rational]) {
            self.bump();
        } else {
            self.expected("a beat unit (`quarter` or `1/4`)");
        }
        self.expect(SyntaxKind::Equals, "`=`");
        self.expect(SyntaxKind::Integer, "a tempo in bpm");
        // `tempo 1/4 = 96 at 9:1;` — a tempo change, written the way a form
        // marker is. Without the `at`, it is the tempo the piece starts in.
        if self.at(SyntaxKind::AtKw) {
            self.bump();
            self.position();
        }
        self.expect(SyntaxKind::Semicolon, "`;`");
        self.finish();
    }

    /// `library { ... }` — shared declarations, importable by a piece.
    fn library_decl(&mut self) {
        self.start(SyntaxKind::LibraryDecl);
        self.bump(); // library
        self.expect(SyntaxKind::LBrace, "`{`");
        loop {
            if self.at(SyntaxKind::RBrace) {
                self.bump();
                break;
            }
            if self.current().is_none() {
                if let Some(error) = self.unclosed("`library` block") {
                    self.errors.push(error);
                }
                break;
            }
            if self.at(SyntaxKind::UseKw) {
                self.import_stmt();
            } else if self.at(SyntaxKind::MotifKw) {
                self.motif_decl();
            } else if self.at(SyntaxKind::PerformanceKw) {
                self.performance_decl();
            } else if self.at(SyntaxKind::StudioKw) {
                self.studio_decl();
            } else {
                // A library holds what can be shared. Music belongs to a
                // piece, which is why `score` is not in this list.
                self.expected_with_help(
                    "a declaration",
                    "a library holds `use`, `motif`, `performance`, and `studio` — music belongs to a piece",
                );
                self.recover(PIECE_RECOVERY);
            }
        }
        self.finish();
    }

    /// `use "../library/motifs.musa";` — a relative import.
    ///
    /// Told apart from a motif call by what follows `use`: a string is a
    /// file, a name is a motif.
    fn import_stmt(&mut self) {
        self.start(SyntaxKind::ImportStmt);
        self.bump(); // use
        self.expect(SyntaxKind::String, "a relative path in quotes");
        self.expect(SyntaxKind::Semicolon, "`;`");
        self.finish();
    }

    /// `meter <n>/<d>;`
    fn meter_stmt(&mut self) {
        self.start(SyntaxKind::MeterStmt);
        self.bump(); // meter
        self.expect(SyntaxKind::Rational, "a meter such as `4/4`");
        self.expect(SyntaxKind::Semicolon, "`;`");
        self.finish();
    }

    /// `key <pitch-class> <mode>;`
    fn key_stmt(&mut self) {
        self.start(SyntaxKind::KeyStmt);
        self.bump(); // key
        self.expect(SyntaxKind::Identifier, "a pitch class such as `a` or `gs`");
        self.expect(SyntaxKind::Identifier, "a mode (`major` or `minor`)");
        self.expect(SyntaxKind::Semicolon, "`;`");
        self.finish();
    }

    /// `motif name(param: type = default, ...) { ... }`
    fn motif_decl(&mut self) {
        self.start(SyntaxKind::MotifDecl);
        self.bump(); // motif
        self.expect(SyntaxKind::Identifier, "a motif name");
        self.expect(SyntaxKind::LParen, "`(`");
        while self.at(SyntaxKind::Identifier) {
            self.bump(); // parameter name
            self.expect(SyntaxKind::Colon, "`:`");
            if self.at_any(&[SyntaxKind::PitchKw, SyntaxKind::Identifier]) {
                self.bump(); // parameter type (`pitch`)
            } else {
                self.expected("a parameter type (`pitch`)");
            }
            if self.at(SyntaxKind::Equals) {
                self.bump();
                if self.at_any(&[SyntaxKind::PitchLiteral, SyntaxKind::Rational, SyntaxKind::Integer]) {
                    self.bump(); // default value (pitch or duration)
                } else {
                    self.expected("a default value");
                }
            }
            if self.at(SyntaxKind::Comma) {
                self.bump();
            } else {
                break;
            }
        }
        self.expect(SyntaxKind::RParen, "`)`");
        self.block();
        self.finish();
    }

    /// `score { part ... }`
    fn score_decl(&mut self) {
        self.start(SyntaxKind::ScoreDecl);
        self.bump(); // score
        self.expect(SyntaxKind::LBrace, "`{`");
        loop {
            if self.at(SyntaxKind::RBrace) {
                self.bump();
                break;
            }
            if self.current().is_none() {
                if let Some(error) = self.unclosed("`score` block") {
                    self.errors.push(error);
                }
                break;
            }
            if self.at(SyntaxKind::PartKw) {
                self.part_decl();
            } else if self.at(SyntaxKind::SectionKw) {
                self.section_stmt();
            } else if self.at(SyntaxKind::HarmonyKw) {
                self.harmony_decl();
            } else {
                self.expected("a `part`, `section`, or `harmony` declaration");
                self.recover(SCORE_RECOVERY);
            }
        }
        self.finish();
    }

    /// `part name { clef ...; voice ... }`
    fn part_decl(&mut self) {
        self.start(SyntaxKind::PartDecl);
        self.bump(); // part
        self.expect(SyntaxKind::Identifier, "a part name");
        self.expect(SyntaxKind::LBrace, "`{`");
        loop {
            if self.at(SyntaxKind::RBrace) {
                self.bump();
                break;
            }
            if self.current().is_none() {
                if let Some(error) = self.unclosed("`part` block") {
                    self.errors.push(error);
                }
                break;
            }
            if self.at(SyntaxKind::ClefKw) {
                self.clef_stmt();
            } else if self.at(SyntaxKind::ProfileKw) {
                self.profile_stmt();
            } else if self.at(SyntaxKind::VoiceKw) {
                self.voice_decl();
            } else {
                self.expected("`clef`, `profile`, or `voice`");
                self.recover(PART_RECOVERY);
            }
        }
        self.finish();
    }

    /// `clef <name>;`
    fn clef_stmt(&mut self) {
        self.start(SyntaxKind::ClefStmt);
        self.bump(); // clef
        self.expect(SyntaxKind::Identifier, "a clef name");
        self.expect(SyntaxKind::Semicolon, "`;`");
        self.finish();
    }

    /// `profile <name>;` — which performance profile realizes this part.
    fn profile_stmt(&mut self) {
        self.start(SyntaxKind::ProfileStmt);
        self.bump(); // profile
        self.expect(SyntaxKind::Identifier, "a profile name");
        self.expect(SyntaxKind::Semicolon, "`;`");
        self.finish();
    }

    /// `performance { profile ... }`
    fn performance_decl(&mut self) {
        self.start(SyntaxKind::PerformanceDecl);
        self.bump(); // performance
        self.expect(SyntaxKind::LBrace, "`{`");
        loop {
            if self.at(SyntaxKind::RBrace) {
                self.bump();
                break;
            }
            if self.current().is_none() {
                if let Some(error) = self.unclosed("`performance` block") {
                    self.errors.push(error);
                }
                break;
            }
            if self.at(SyntaxKind::ProfileKw) {
                self.profile_decl();
            } else {
                self.expected("a `profile` declaration");
                self.recover(PERFORMANCE_RECOVERY);
            }
        }
        self.finish();
    }

    /// `profile name { articulation ... dynamic ... }`
    fn profile_decl(&mut self) {
        self.start(SyntaxKind::ProfileDecl);
        self.bump(); // profile
        self.expect(SyntaxKind::Identifier, "a profile name");
        self.expect(SyntaxKind::LBrace, "`{`");
        loop {
            if self.at(SyntaxKind::RBrace) {
                self.bump();
                break;
            }
            if self.current().is_none() {
                if let Some(error) = self.unclosed("`profile` block") {
                    self.errors.push(error);
                }
                break;
            }
            if self.at(SyntaxKind::ArticulationKw) {
                self.rule(SyntaxKind::ArticulationRule, "an articulation name");
            } else if self.at(SyntaxKind::DynamicKw) {
                self.rule(SyntaxKind::DynamicRule, "a dynamic marking");
            } else {
                self.expected("an `articulation` or `dynamic` rule");
                self.recover(PROFILE_RECOVERY);
            }
        }
        self.finish();
    }

    /// `articulation|dynamic <name> { <setting>* }` — one shape, two heads.
    fn rule(&mut self, kind: SyntaxKind, what: &str) {
        self.start(kind);
        self.bump(); // articulation | dynamic
        self.expect(SyntaxKind::Identifier, what);
        self.expect(SyntaxKind::LBrace, "`{`");
        loop {
            if self.at(SyntaxKind::RBrace) {
                self.bump();
                break;
            }
            if self.current().is_none() {
                if let Some(error) = self.unclosed("rule block") {
                    self.errors.push(error);
                }
                break;
            }
            if self.at(SyntaxKind::Identifier) {
                self.setting_stmt();
            } else {
                self.expected("a setting such as `gate = 0.55;`");
                self.recover(&[SyntaxKind::Semicolon, SyntaxKind::RBrace]);
            }
        }
        self.finish();
    }

    /// `<name> = <number> [unit];` — the unit is the value's, not the
    /// setting's, so `attack = 8 ms;` and `attack = 0.008 s;` both parse.
    fn setting_stmt(&mut self) {
        self.start(SyntaxKind::SettingStmt);
        self.bump(); // setting name
        self.expect(SyntaxKind::Equals, "`=`");
        if self.at_any(&[SyntaxKind::Float, SyntaxKind::Integer]) {
            self.bump();
        } else {
            self.expected("a number");
        }
        if self.at_any(&[SyntaxKind::UnitMs, SyntaxKind::UnitS]) {
            self.bump();
        }
        self.expect(SyntaxKind::Semicolon, "`;`");
        self.finish();
    }

    /// `studio { patch ... bus ... assign ... }` (roadmap §7.1).
    ///
    /// The studio never mentions notes: everything inside it names signals,
    /// patches, buses, and the bindings between them (§6.5).
    fn studio_decl(&mut self) {
        self.start(SyntaxKind::StudioDecl);
        self.bump(); // studio
        self.expect(SyntaxKind::LBrace, "`{`");
        loop {
            if self.at(SyntaxKind::RBrace) {
                self.bump();
                break;
            }
            if self.current().is_none() {
                if let Some(error) = self.unclosed("`studio` block") {
                    self.errors.push(error);
                }
                break;
            }
            if self.at(SyntaxKind::PatchKw) {
                self.patch_decl();
            } else if self.at(SyntaxKind::BusKw) {
                self.bus_decl();
            } else if self.at(SyntaxKind::ModulateKw) {
                self.modulate_stmt();
            } else if self.at(SyntaxKind::AssignKw) {
                self.binding_stmt(SyntaxKind::AssignStmt, "a part name", "a patch name");
            } else if self.at(SyntaxKind::RouteKw) {
                self.binding_stmt(SyntaxKind::RouteStmt, "a source name", "a destination");
            } else if self.at(SyntaxKind::SendKw) {
                self.send_stmt();
            } else if self.at(SyntaxKind::Identifier) {
                self.signal_binding();
            } else {
                self.expected("`patch`, `bus`, `modulate`, `assign`, `route`, `send`, or a signal binding");
                self.recover(STUDIO_RECOVERY);
            }
        }
        self.finish();
    }

    /// `patch <name> { <signal bindings and chains> }`
    fn patch_decl(&mut self) {
        self.start(SyntaxKind::PatchDecl);
        self.bump(); // patch
        self.expect(SyntaxKind::Identifier, "a patch name");
        self.expect(SyntaxKind::LBrace, "`{`");
        self.chain_body("patch");
        self.finish();
    }

    /// `bus <name> { <chains> }`
    fn bus_decl(&mut self) {
        self.start(SyntaxKind::BusDecl);
        self.bump(); // bus
        self.expect(SyntaxKind::Identifier, "a bus name");
        self.expect(SyntaxKind::LBrace, "`{`");
        self.chain_body("bus");
        self.finish();
    }

    /// The shared body of a patch or bus: named signals and bare chains, in
    /// any order, until the closing brace.
    fn chain_body(&mut self, what: &str) {
        loop {
            if self.at(SyntaxKind::RBrace) {
                self.bump();
                break;
            }
            if self.current().is_none() {
                if let Some(error) = self.unclosed(&format!("`{what}` block")) {
                    self.errors.push(error);
                }
                break;
            }
            if self.at(SyntaxKind::Identifier) && self.nth_significant(1) == Some(SyntaxKind::Equals) {
                self.signal_binding();
            } else if self.at_any(&[SyntaxKind::Identifier, SyntaxKind::OutputKw]) {
                self.chain_stmt();
            } else {
                self.expected("a signal binding or a signal chain");
                self.recover(STUDIO_RECOVERY);
            }
        }
    }

    /// `<name> = <chain>;`
    fn signal_binding(&mut self) {
        self.start(SyntaxKind::SignalBinding);
        self.bump(); // name
        self.expect(SyntaxKind::Equals, "`=`");
        self.signal_chain();
        self.expect(SyntaxKind::Semicolon, "`;`");
        self.finish();
    }

    /// `<chain>;` — unnamed, so its value is the enclosing patch or bus's.
    fn chain_stmt(&mut self) {
        self.start(SyntaxKind::ChainStmt);
        self.signal_chain();
        self.expect(SyntaxKind::Semicolon, "`;`");
        self.finish();
    }

    /// `<stage> |> <stage> |> ...`
    ///
    /// `|>` is left-associative and the only operator in the language, so the
    /// "expression parser" §10.2 anticipates is this loop: a precedence table
    /// would be machinery with one entry.
    fn signal_chain(&mut self) {
        self.start(SyntaxKind::SignalChain);
        self.stage();
        while self.at(SyntaxKind::PipeForward) {
            self.bump();
            self.stage();
        }
        self.finish();
    }

    /// One stage: a construction `name(args)`, or a bare name (another
    /// signal, or the `output` terminal).
    fn stage(&mut self) {
        if self.at(SyntaxKind::OutputKw) || self.at(SyntaxKind::MasterKw) {
            self.start(SyntaxKind::NameRef);
            self.bump();
            self.finish();
        } else if self.at(SyntaxKind::Identifier) {
            if self.nth_significant(1) == Some(SyntaxKind::LParen) {
                self.call_expr();
            } else {
                self.start(SyntaxKind::NameRef);
                self.bump();
                self.finish();
            }
        } else {
            self.expected("a processor, a signal name, or `output`");
            self.recover(STUDIO_RECOVERY);
        }
    }

    /// `<name>(<args>)`
    fn call_expr(&mut self) {
        self.start(SyntaxKind::CallExpr);
        self.bump(); // processor name
        self.start(SyntaxKind::ArgList);
        self.expect(SyntaxKind::LParen, "`(`");
        loop {
            if self.at(SyntaxKind::RParen) {
                self.bump();
                break;
            }
            if self.current().is_none() {
                if let Some(error) = self.unclosed_list() {
                    self.errors.push(error);
                }
                break;
            }
            self.arg();
            if self.at(SyntaxKind::Comma) {
                self.bump();
            }
        }
        self.finish(); // ArgList
        self.finish(); // CallExpr
    }

    /// `<name>: <value>` or a positional `<value>`.
    fn arg(&mut self) {
        self.start(SyntaxKind::Arg);
        if self.at(SyntaxKind::Identifier) && self.nth_significant(1) == Some(SyntaxKind::Colon) {
            self.bump(); // argument name
            self.bump(); // :
        }
        self.value();
        self.finish();
    }

    /// A number with an optional unit, a nested construction, or a name.
    fn value(&mut self) {
        if self.at_any(&[
            SyntaxKind::Minus,
            SyntaxKind::Float,
            SyntaxKind::Integer,
            SyntaxKind::Rational,
        ]) {
            self.start(SyntaxKind::ValueLiteral);
            if self.at(SyntaxKind::Minus) {
                self.bump();
            }
            if self.at_any(&[SyntaxKind::Float, SyntaxKind::Integer, SyntaxKind::Rational]) {
                self.bump();
            } else {
                self.expected("a number");
            }
            if self.at_any(&[
                SyntaxKind::UnitHz,
                SyntaxKind::UnitMs,
                SyntaxKind::UnitS,
                SyntaxKind::UnitDb,
            ]) {
                self.bump();
            }
            self.finish();
        } else if self.at(SyntaxKind::Identifier) && self.nth_significant(1) == Some(SyntaxKind::LParen) {
            self.call_expr();
        } else if self.at_any(&[SyntaxKind::Identifier, SyntaxKind::OutputKw, SyntaxKind::MasterKw]) {
            self.start(SyntaxKind::NameRef);
            self.bump();
            self.finish();
        } else {
            self.expected("a value");
            self.recover(&[
                SyntaxKind::Comma,
                SyntaxKind::RParen,
                SyntaxKind::Semicolon,
                SyntaxKind::RBrace,
            ]);
        }
    }

    /// `modulate <signal> -> <patch>.<stage>.<parameter>;`
    fn modulate_stmt(&mut self) {
        self.start(SyntaxKind::ModulateStmt);
        self.bump(); // modulate
        self.expect(SyntaxKind::Identifier, "a signal name");
        self.expect(SyntaxKind::Arrow, "`->`");
        self.start(SyntaxKind::ParamPath);
        self.expect(SyntaxKind::Identifier, "a patch name");
        while self.at(SyntaxKind::Dot) {
            self.bump();
            self.expect(SyntaxKind::Identifier, "a stage or parameter name");
        }
        self.finish();
        self.expect(SyntaxKind::Semicolon, "`;`");
        self.finish();
    }

    /// `assign <a> -> <b>;` and `route <a> -> <b>;` — the same shape with
    /// different names on each side, so one production serves both.
    fn binding_stmt(&mut self, kind: SyntaxKind, source: &str, destination: &str) {
        self.start(kind);
        self.bump(); // assign / route
        self.expect(SyntaxKind::Identifier, source);
        self.expect(SyntaxKind::Arrow, "`->`");
        if self.at(SyntaxKind::MasterKw) {
            self.bump();
        } else {
            self.expect(SyntaxKind::Identifier, destination);
        }
        self.expect(SyntaxKind::Semicolon, "`;`");
        self.finish();
    }

    /// `send <source> -> <bus> at <gain> dB;`
    fn send_stmt(&mut self) {
        self.start(SyntaxKind::SendStmt);
        self.bump(); // send
        self.expect(SyntaxKind::Identifier, "a source name");
        self.expect(SyntaxKind::Arrow, "`->`");
        self.expect(SyntaxKind::Identifier, "a bus name");
        self.expect(SyntaxKind::AtKw, "`at`");
        self.value();
        self.expect(SyntaxKind::Semicolon, "`;`");
        self.finish();
    }

    /// `voice name { ... }`
    fn voice_decl(&mut self) {
        self.start(SyntaxKind::VoiceDecl);
        self.bump(); // voice
        self.expect(SyntaxKind::Identifier, "a voice name");
        self.expect(SyntaxKind::LBrace, "`{`");
        self.voice_items();
        self.expect(SyntaxKind::RBrace, "`}`");
        self.finish();
    }

    /// Items of a voice or motif body, stopped by `}` or end of input.
    fn voice_items(&mut self) {
        loop {
            if self.at(SyntaxKind::RBrace) || self.current().is_none() {
                break;
            }
            if self.at_any(&[SyntaxKind::PitchLiteral, SyntaxKind::Identifier]) {
                self.note_stmt();
            } else if self.at(SyntaxKind::RestKw) {
                self.rest_stmt();
            } else if self.at(SyntaxKind::ChordKw) {
                self.chord_stmt();
            } else if self.at(SyntaxKind::UseKw) {
                self.use_stmt();
            } else if self.at(SyntaxKind::TransposeKw) {
                self.transpose_stmt();
            } else if self.at(SyntaxKind::RepeatKw) {
                self.repeat_stmt();
            } else if self.at(SyntaxKind::BarKw) {
                self.bar_stmt();
            } else if self.at(SyntaxKind::SlurKw) {
                self.slur_stmt();
            } else if self.at(SyntaxKind::DynamicKw) {
                self.dynamic_stmt();
            } else if self.at(SyntaxKind::TupletKw) {
                self.tuplet_stmt();
            } else if self.at(SyntaxKind::StretchKw) {
                self.stretch_stmt();
            } else if self.at(SyntaxKind::RetrogradeKw) {
                self.retrograde_stmt();
            } else if self.at(SyntaxKind::InvertKw) {
                self.invert_stmt();
            } else if self.at(SyntaxKind::PhraseKw) {
                self.phrase_stmt();
            } else if self.at_any(&[SyntaxKind::CrescendoKw, SyntaxKind::DiminuendoKw]) {
                self.hairpin_stmt();
            } else {
                self.expected_with_help(
                    "something to play",
                    "a voice holds notes (`c5 1/4;`), `rest`, `chord`, `bar`, and `use` — run `musa explain syntax` for the rest",
                );
                self.recover(VOICE_RECOVERY);
            }
        }
    }

    /// `{ ... }` body of a motif, transpose, or repeat.
    fn block(&mut self) {
        self.start(SyntaxKind::Block);
        self.expect(SyntaxKind::LBrace, "`{`");
        self.voice_items();
        self.expect(SyntaxKind::RBrace, "`}`");
        self.finish();
    }

    /// `<pitch-or-ref> <duration> <articulation>* ~? ;`
    fn note_stmt(&mut self) {
        self.start(SyntaxKind::NoteStmt);
        self.bump(); // pitch literal or pitch reference
        self.duration();
        self.articulations();
        self.tie();
        self.expect(SyntaxKind::Semicolon, "`;`");
        self.finish();
    }

    /// Zero or more articulation names after a duration (`accent staccato`).
    ///
    /// They live in their own node: a bare identifier in a note statement is
    /// otherwise a pitch or duration parameter reference, and telling the two
    /// apart by counting tokens is exactly the kind of positional rule that
    /// breaks the next time the statement grows a part.
    fn articulations(&mut self) {
        if !self.at(SyntaxKind::Identifier) {
            return;
        }
        self.start(SyntaxKind::ArticulationList);
        while self.at(SyntaxKind::Identifier) {
            self.bump();
        }
        self.finish();
    }

    /// The postfix tie mark, tying this statement to the next.
    fn tie(&mut self) {
        if self.at(SyntaxKind::Tilde) {
            self.bump();
        }
    }

    /// `rest <duration>;`
    fn rest_stmt(&mut self) {
        self.start(SyntaxKind::RestStmt);
        self.bump(); // rest
        self.duration();
        self.expect(SyntaxKind::Semicolon, "`;`");
        self.finish();
    }

    /// `chord [<pitch>, ...] <duration>;`
    fn chord_stmt(&mut self) {
        self.start(SyntaxKind::ChordStmt);
        self.bump(); // chord
        self.expect(SyntaxKind::LBracket, "`[`");
        self.expect(SyntaxKind::PitchLiteral, "a pitch");
        while self.at(SyntaxKind::Comma) {
            self.bump();
            self.expect(SyntaxKind::PitchLiteral, "a pitch");
        }
        self.expect(SyntaxKind::RBracket, "`]`");
        self.duration();
        self.articulations();
        self.tie();
        self.expect(SyntaxKind::Semicolon, "`;`");
        self.finish();
    }

    /// `use name(args);` — or `use name;`, when the material takes none.
    fn use_stmt(&mut self) {
        self.start(SyntaxKind::UseStmt);
        self.bump(); // use
        self.expect(SyntaxKind::Identifier, "a name");
        // The parentheses *are* the argument list. Material that takes no
        // arguments — a bar, a motif declared without parameters — is played
        // by naming it, and `use head();` would be punctuation standing in for
        // nothing.
        if self.at(SyntaxKind::LParen) {
            self.use_args();
        }
        // `with { ... }` specializes this occurrence and only this one
        // (roadmap §9). A call that ends there is a block, not a statement,
        // so it takes no `;` — the same shape every other block has.
        if self.at(SyntaxKind::WithKw) {
            self.with_clause();
        } else {
            self.expect(SyntaxKind::Semicolon, "`;`");
        }
        self.finish();
    }

    /// `(a, b, c)` — the arguments of a `use`.
    fn use_args(&mut self) {
        self.bump(); // (
        if !self.at(SyntaxKind::RParen) {
            loop {
                if self.at_any(&[
                    SyntaxKind::PitchLiteral,
                    SyntaxKind::Identifier,
                    SyntaxKind::Rational,
                    SyntaxKind::Integer,
                ]) {
                    self.bump();
                } else {
                    self.expected("an argument");
                    break;
                }
                if self.at(SyntaxKind::Comma) {
                    self.bump();
                } else {
                    break;
                }
            }
        }
        self.expect(SyntaxKind::RParen, "`)`");
    }

    /// `with { note <n> = <pitch>; ... }` — overrides on one occurrence.
    fn with_clause(&mut self) {
        self.start(SyntaxKind::WithClause);
        self.bump(); // with
        self.expect(SyntaxKind::LBrace, "`{`");
        loop {
            if self.at(SyntaxKind::RBrace) || self.current().is_none() {
                break;
            }
            if self.at(SyntaxKind::NoteKw) {
                self.override_stmt();
            } else {
                self.expected("an override such as `note 2 = d5;`");
                self.recover(&[SyntaxKind::Semicolon, SyntaxKind::RBrace, SyntaxKind::NoteKw]);
            }
        }
        self.expect(SyntaxKind::RBrace, "`}`");
        self.finish();
    }

    /// `note <n> = <pitch>;` — the nth note of this occurrence, respelled.
    fn override_stmt(&mut self) {
        self.start(SyntaxKind::OverrideStmt);
        self.bump(); // note
        self.expect(SyntaxKind::Integer, "the note's position in the occurrence");
        self.expect(SyntaxKind::Equals, "`=`");
        self.expect(SyntaxKind::PitchLiteral, "a pitch");
        self.expect(SyntaxKind::Semicolon, "`;`");
        self.finish();
    }

    /// `transpose up|down <quality><size> { ... }` — e.g. `transpose down P5`.
    fn transpose_stmt(&mut self) {
        self.start(SyntaxKind::TransposeStmt);
        self.bump(); // transpose
        if self.at_any(&[SyntaxKind::UpKw, SyntaxKind::DownKw]) {
            self.bump();
        } else {
            self.expected("`up` or `down`");
        }
        self.expect(SyntaxKind::IntervalLiteral, "an interval such as `P5` or `m3`");
        self.block();
        self.finish();
    }

    /// `repeat <n> { ... }`
    fn repeat_stmt(&mut self) {
        self.start(SyntaxKind::RepeatStmt);
        self.bump(); // repeat
        self.expect(SyntaxKind::Integer, "a repeat count");
        self.block();
        self.finish();
    }

    /// `bar { ... }` / `bar head { ... }`
    fn bar_stmt(&mut self) {
        if let Some(error) = self.nested_bar() {
            self.errors.push(error);
        }
        self.start(SyntaxKind::BarStmt);
        self.bump(); // bar
        // The name is optional and there is nothing to disambiguate: a bar's
        // contents start with `{`, so an identifier here can only be a name.
        if self.at(SyntaxKind::Identifier) {
            self.bump();
        }
        self.bar_depth = self.bar_depth.saturating_add(1);
        self.block();
        self.bar_depth = self.bar_depth.saturating_sub(1);
        self.finish();
    }

    /// A `bar` inside a `bar`, if that is where the parser is.
    fn nested_bar(&self) -> Option<SyntaxError> {
        if self.bar_depth == 0 || self.cascading() {
            return None;
        }
        let range = self
            .significant()
            .map_or_else(|| TextRange::empty(self.end_size()), |token| token.range);
        Some(
            SyntaxError::new(range, "bars do not nest", "this bar is inside another")
                .with_help("close the bar above this one, or delete this `bar`"),
        )
    }

    /// `slur { ... }`
    fn slur_stmt(&mut self) {
        self.start(SyntaxKind::SlurStmt);
        self.bump(); // slur
        self.block();
        self.finish();
    }

    /// `phrase "A" { ... }` — a named span over the music it wraps.
    fn phrase_stmt(&mut self) {
        self.start(SyntaxKind::PhraseStmt);
        self.bump(); // phrase
        self.expect(SyntaxKind::String, "a phrase name in quotes");
        self.block();
        self.finish();
    }

    /// `crescendo to f { ... }` — a hairpin over the notes it wraps.
    ///
    /// The mark it grows to is written; the mark it grows *from* is whatever
    /// dynamic is in force, because that is what a hairpin means on a page.
    fn hairpin_stmt(&mut self) {
        self.start(SyntaxKind::HairpinStmt);
        self.bump(); // crescendo or diminuendo
        self.expect(SyntaxKind::ToKw, "`to`");
        self.expect(SyntaxKind::Identifier, "a dynamic such as `f`");
        self.block();
        self.finish();
    }

    /// `section "Exposition" at 1:1;` — a form marker in the score.
    fn section_stmt(&mut self) {
        self.start(SyntaxKind::SectionStmt);
        self.bump(); // section
        self.expect(SyntaxKind::String, "a section name in quotes");
        self.expect(SyntaxKind::AtKw, "`at`");
        self.position();
        self.expect(SyntaxKind::Semicolon, "`;`");
        self.finish();
    }

    /// `harmony { at 1:1 am; ... }` — the chord-symbol lane.
    fn harmony_decl(&mut self) {
        self.start(SyntaxKind::HarmonyDecl);
        self.bump(); // harmony
        self.expect(SyntaxKind::LBrace, "`{`");
        loop {
            if self.at(SyntaxKind::RBrace) || self.current().is_none() {
                break;
            }
            if self.at(SyntaxKind::AtKw) {
                self.harmony_stmt();
            } else {
                self.expected("a chord such as `at 1:1 am;`");
                self.recover(&[SyntaxKind::Semicolon, SyntaxKind::RBrace, SyntaxKind::AtKw]);
            }
        }
        self.expect(SyntaxKind::RBrace, "`}`");
        self.finish();
    }

    /// `at 1:1 am;` — one chord symbol at a position.
    fn harmony_stmt(&mut self) {
        self.start(SyntaxKind::HarmonyStmt);
        self.bump(); // at
        self.position();
        self.chord_symbol();
        self.expect(SyntaxKind::Semicolon, "`;`");
        self.finish();
    }

    /// `<measure>:<beat>` — a position in the piece, in the coordinates a
    /// composer reads off the page.
    fn position(&mut self) {
        self.start(SyntaxKind::Position);
        self.expect(SyntaxKind::Integer, "a measure number");
        self.expect(SyntaxKind::Colon, "`:`");
        if self.at_any(&[SyntaxKind::Integer, SyntaxKind::Rational]) {
            self.bump();
        } else {
            self.expected("a beat such as `1` or `3/2`");
        }
        self.finish();
    }

    /// A chord symbol as written: `am`, `fmaj7`, `a7`.
    ///
    /// The lexer has no chord-symbol token and does not need one — a symbol
    /// is one word, and the word lexes as a name (`am`), a pitch (`a7`), or a
    /// name and a number (`fmaj` `7`) depending on how it is spelled. The
    /// parser takes that word; the compiler reads what it says, and checks
    /// that it really was one word rather than two.
    fn chord_symbol(&mut self) {
        self.start(SyntaxKind::ChordSymbol);
        if self.at_any(&[SyntaxKind::Identifier, SyntaxKind::PitchLiteral]) {
            self.bump();
            if self.at(SyntaxKind::Integer) {
                self.bump();
            }
        } else {
            self.expected("a chord such as `am` or `fmaj7`");
        }
        self.finish();
    }

    /// `dynamic <mark>;` — the marking applies from the next event on.
    fn dynamic_stmt(&mut self) {
        self.start(SyntaxKind::DynamicStmt);
        self.bump(); // dynamic
        self.expect(SyntaxKind::Identifier, "a dynamic marking such as `p` or `mf`");
        self.expect(SyntaxKind::Semicolon, "`;`");
        self.finish();
    }

    /// `stretch <n>/<d> { ... }` — the block, its durations multiplied.
    fn stretch_stmt(&mut self) {
        self.start(SyntaxKind::StretchStmt);
        self.bump(); // stretch
        if self.at_any(&[SyntaxKind::Rational, SyntaxKind::Integer]) {
            self.bump();
        } else {
            self.expected("a factor such as `3/2` or `2`");
        }
        self.block();
        self.finish();
    }

    /// `retrograde { ... }` — the block, backwards.
    fn retrograde_stmt(&mut self) {
        self.start(SyntaxKind::RetrogradeStmt);
        self.bump(); // retrograde
        self.block();
        self.finish();
    }

    /// `invert around <pitch> { ... }` — the block, mirrored about a pitch.
    fn invert_stmt(&mut self) {
        self.start(SyntaxKind::InvertStmt);
        self.bump(); // invert
        self.expect(SyntaxKind::AroundKw, "`around`");
        self.expect(SyntaxKind::PitchLiteral, "the axis pitch, such as `c5`");
        self.block();
        self.finish();
    }

    /// `tuplet <n>/<d> { ... }` — `n` written values in the time of `d`.
    fn tuplet_stmt(&mut self) {
        self.start(SyntaxKind::TupletStmt);
        self.bump(); // tuplet
        self.expect(SyntaxKind::Rational, "a tuplet ratio such as `3/2`");
        self.block();
        self.finish();
    }

    /// A duration: `1`, `1/2`, `3/8`, `1/12`, …
    fn duration(&mut self) {
        if self.at_any(&[SyntaxKind::Rational, SyntaxKind::Integer, SyntaxKind::Identifier]) {
            self.bump(); // literal or duration-parameter reference
        } else {
            self.expected("a duration");
        }
    }
}

/// The text of a token the grammar spells out, when it has one.
///
/// Only these get a fix: inserting a `;` the language requires is not a guess,
/// while inserting an identifier or a pitch would be inventing music.
/// Keywords are deliberately absent from this table. Inserting `at` where a
/// token ends gives `96at`, and a fix that has to guess at whitespace is not
/// the certain edit a fix is supposed to be.
const PUNCTUATION: &[(SyntaxKind, &str)] = &[
    (SyntaxKind::Semicolon, ";"),
    (SyntaxKind::LBrace, "{"),
    (SyntaxKind::RBrace, "}"),
    (SyntaxKind::LParen, "("),
    (SyntaxKind::RParen, ")"),
    (SyntaxKind::LBracket, "["),
    (SyntaxKind::RBracket, "]"),
    (SyntaxKind::Equals, "="),
    (SyntaxKind::Colon, ":"),
    (SyntaxKind::Arrow, "->"),
];

fn punctuation_of(kind: SyntaxKind) -> Option<&'static str> {
    PUNCTUATION
        .iter()
        .find(|(candidate, _)| *candidate == kind)
        .map(|(_, text)| *text)
}
