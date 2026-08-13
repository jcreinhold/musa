//! Hand-written recursive-descent parser producing a lossless Rowan tree.
//!
//! Architecture (roadmap §10.2–§10.4): the parser emits a flat event stream
//! (`StartNode` / `Token` / `FinishNode`); a second pass replays the events
//! into a `rowan::GreenNodeBuilder`. Parser code never touches Rowan. The
//! grammar is small enough that statements dispatch on their first token. A
//! postfix call inserts its parent start event at the primary's checkpoint;
//! this is the one place expression precedence needs a forward parent.
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

/// What a file's lexical root turned out to be, as far as the parser can tell.
///
/// Both flags answer the same question — whether the file still owes a piece.
/// A `make` may *be* the piece, and a file that declares a module tree is a
/// package's own bookkeeping and is not music at all.
#[derive(Clone, Copy, Default)]
struct RootShape {
    made: bool,
    declares_modules: bool,
}

/// One step of tree construction.
enum Event<'a> {
    StartNode(SyntaxKind),
    Token(SyntaxKind, &'a str),
    FinishNode,
}

/// What may name a module inside an import path.
///
/// A module may be called after a type or a domain — `pitch`, `scale`,
/// `harmony` — and the lexer writes the keyword token wherever the word
/// appears. The path position is what makes the word a name. `list` and
/// `option` left this list when they stopped being keywords: the types are
/// `List` and `Option`, and the files that hold them are ordinary names.
pub(crate) const MODULE_NAME: &[SyntaxKind] = &[
    SyntaxKind::Identifier,
    SyntaxKind::HarmonyKw,
    SyntaxKind::PitchKw,
    SyntaxKind::ScaleKw,
];

/// The music statement keywords a type name used to be allowed to be.
///
/// Each of these words is still a keyword and still means what it meant;
/// `key c major;` is untouched. They are read in type position only so that
/// `let tonic: key` gets the one complaint that carries `Key`.
const MOVED_TYPE_KEYWORDS: &[SyntaxKind] = &[
    SyntaxKind::PitchKw,
    SyntaxKind::MusicKw,
    SyntaxKind::ScaleKw,
    SyntaxKind::KeyKw,
    SyntaxKind::DegreeKw,
    SyntaxKind::FrameKw,
];

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
    SyntaxKind::FragmentKw,
    SyntaxKind::ScoreKw,
    SyntaxKind::PerformanceKw,
    SyntaxKind::StudioKw,
    SyntaxKind::LetKw,
    SyntaxKind::FnKw,
    SyntaxKind::DataKw,
];
/// What ends a broken declaration at the file's lexical root.
const ROOT_RECOVERY: &[SyntaxKind] = &[
    SyntaxKind::Semicolon,
    SyntaxKind::TemplateKw,
    SyntaxKind::MakeKw,
    SyntaxKind::SignatureKw,
    SyntaxKind::StructureKw,
    SyntaxKind::DataKw,
    SyntaxKind::PieceKw,
    SyntaxKind::LibraryKw,
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
    SyntaxKind::MeterKw,
    SyntaxKind::TempoKw,
    SyntaxKind::ProfileKw,
    SyntaxKind::VoiceKw,
    SyntaxKind::MakeKw,
];
const PERFORMANCE_RECOVERY: &[SyntaxKind] = &[SyntaxKind::RBrace, SyntaxKind::ProfileKw];
const PROFILE_RECOVERY: &[SyntaxKind] = &[
    SyntaxKind::Semicolon,
    SyntaxKind::RBrace,
    SyntaxKind::MarkKw,
    SyntaxKind::DynamicKw,
    SyntaxKind::GrooveKw,
    SyntaxKind::GraceKw,
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
    // `|` cannot appear inside an event, so it is the strongest anchor a
    // voice has: one malformed note poisons its own bar and no more.
    SyntaxKind::Pipe,
    SyntaxKind::LBracket,
    SyntaxKind::PitchLiteral,
    SyntaxKind::RestKw,
    SyntaxKind::UseKw,
    SyntaxKind::TransposeKw,
    SyntaxKind::RepeatKw,
    SyntaxKind::BarKw,
    SyntaxKind::AssertKw,
    SyntaxKind::SenzaKw,
    SyntaxKind::TempoKw,
    SyntaxKind::MeterKw,
    SyntaxKind::KeyKw,
    SyntaxKind::ClefKw,
    SyntaxKind::EndingKw,
    SyntaxKind::SlurKw,
    SyntaxKind::PhraseKw,
    SyntaxKind::GraceKw,
    SyntaxKind::CrescendoKw,
    SyntaxKind::DiminuendoKw,
    SyntaxKind::DynamicKw,
    SyntaxKind::TupletKw,
    SyntaxKind::StretchKw,
    SyntaxKind::RetrogradeKw,
    SyntaxKind::InvertKw,
    SyntaxKind::MobileKw,
    SyntaxKind::ImproviseKw,
    SyntaxKind::MarkKw,
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
    /// Whether the items being parsed are inside a `|` bar.
    ///
    /// A `|` both closes the bar it stands after and opens the one it stands
    /// before, so the same token means "stop" one level down and "start" one
    /// level up. This flag is which of the two the parser is looking at.
    in_pipe_bar: bool,
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
            in_pipe_bar: false,
        }
    }

    /// Parse the whole document and build the tree.
    fn run(mut self) -> (SyntaxNode, Vec<SyntaxError>) {
        self.start(SyntaxKind::Root);
        // A file is a piece or a library. Which one it is is written at the
        // top of it rather than inferred from what it happens to contain: a
        // library with a `score` in it is then a parse error rather than a
        // rule someone has to remember.
        //
        // What may precede it is the file's lexical root: imports, values,
        // functions, signatures, modules, and templates, plus every `make`
        // they are instantiated by. A `make` of a piece template stands in
        // the piece's place and is that piece — one file is still one piece,
        // whether it is written out or made.
        let shape = self.root_preamble();
        if self.at(SyntaxKind::LibraryKw) {
            self.library_decl();
        } else if self.at(SyntaxKind::PieceKw) || !(shape.made || shape.declares_modules) {
            // A file that made nothing still owes a piece, and saying so here
            // is how `piece_decl` reports the one it cannot find.
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

    /// Wrap everything emitted since `checkpoint` in a new parent node.
    fn start_at(&mut self, checkpoint: usize, kind: SyntaxKind) {
        self.events.insert(checkpoint, Event::StartNode(kind));
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

    /// The text of the token at the cursor.
    ///
    /// The parser reads a spelling here only where a spelling is the whole
    /// question: which removed word this is, so the reader can be told what
    /// replaced it. Nothing the language still accepts is decided this way.
    fn word(&self) -> Option<&'a str> {
        let token = self.significant()?;
        self.source
            .get(usize::from(token.range.start())..usize::from(token.range.end()))
    }

    fn at_word(&self, word: &str) -> bool {
        self.at(SyntaxKind::Identifier) && self.word() == Some(word)
    }

    /// Whether the cursor is on `some` or `none` — the constructors as they
    /// were spelled before they took their type's capital.
    fn at_constructor(&self) -> bool {
        self.at_word("some") || self.at_word("none")
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

    /// Whatever stands before the file's piece or library: imports, values,
    /// functions, and templates.
    ///
    /// These are the file's lexical root, and the only scope a template body
    /// reads besides its own parameters. Nothing here is the file's
    /// declaration — the piece or library that follows is.
    fn root_preamble(&mut self) -> RootShape {
        let mut shape = RootShape::default();
        loop {
            if self.at_any(&[SyntaxKind::ImportKw, SyntaxKind::UseKw]) {
                self.import_stmt();
            } else if self.at(SyntaxKind::ModKw) {
                self.mod_decl();
                shape.declares_modules = true;
            } else if self.at(SyntaxKind::LetKw) {
                self.let_decl();
            } else if self.at(SyntaxKind::FnKw) {
                self.fn_decl();
            } else if self.at(SyntaxKind::TemplateKw) {
                self.template_decl();
            } else if self.at(SyntaxKind::SignatureKw) {
                self.signature_decl();
            } else if self.at(SyntaxKind::DataKw) {
                self.data_decl();
            } else if self.at_any(&[SyntaxKind::StructureKw, SyntaxKind::ModuleKw]) {
                self.structure_decl();
            } else if self.at(SyntaxKind::MakeKw) {
                // Every `make` a file's root writes is read here, whether it
                // makes a module or the piece itself. Which one is the piece
                // is a question about what the names mean, and the parser
                // does not know what anything means.
                self.make_stmt();
                shape.made = true;
            } else {
                break;
            }
        }
        shape
    }

    /// `mod tonal;` — one child of a package's module tree.
    ///
    /// A name and nothing else: what the name reaches is a question about the
    /// package's files, which the parser has none of. The name is read from
    /// [`MODULE_NAME`] for the same reason an import path is — `mod list;` is
    /// the module namespace, where `list` is a name and not a type.
    fn mod_decl(&mut self) {
        self.start(SyntaxKind::ModDecl);
        self.bump(); // `mod`
        if self.at_any(MODULE_NAME) {
            self.bump();
        } else {
            self.expected("a module name");
        }
        self.expect(SyntaxKind::Semicolon, "`;`");
        self.finish();
    }

    /// `signature TonalContext { let tonic: key; }` — the members a module
    /// must provide, each a `let` with its definition left out.
    fn signature_decl(&mut self) {
        self.start(SyntaxKind::SignatureDecl);
        self.bump(); // signature
        self.expect(SyntaxKind::Identifier, "a signature name");
        self.expect(SyntaxKind::LBrace, "`{`");
        while !self.at(SyntaxKind::RBrace) && self.current().is_some() {
            if self.at(SyntaxKind::LetKw) {
                self.signature_member();
            } else if self.at(SyntaxKind::DataKw) {
                self.data_member();
            } else {
                self.expected("`let`, `data`, or `}`");
                self.recover(&[SyntaxKind::LetKw, SyntaxKind::DataKw, SyntaxKind::RBrace]);
            }
        }
        self.expect(SyntaxKind::RBrace, "`}`");
        self.finish();
    }

    /// `data Motive { Silence, Sounded(pitch: Pitch, held: Duration), }` —
    /// one finite nominal declaration.
    ///
    /// A variant with no fields writes no parentheses, because there are no
    /// fields to name; a variant with fields names every one of them, because
    /// a field a reader cannot name is a field the record case could not
    /// project.
    fn data_decl(&mut self) {
        self.start(SyntaxKind::DataDecl);
        self.bump(); // data
        self.expect(SyntaxKind::Identifier, "a type name");
        if self.at(SyntaxKind::Less) {
            self.type_params();
        }
        self.expect(SyntaxKind::LBrace, "`{`");
        while !self.at(SyntaxKind::RBrace) && self.current().is_some() {
            if self.at(SyntaxKind::Identifier) {
                self.data_variant();
                if self.at(SyntaxKind::Comma) {
                    self.bump();
                }
            } else {
                self.expected("a constructor name, or `}`");
                self.recover(&[SyntaxKind::Identifier, SyntaxKind::RBrace]);
            }
        }
        self.expect(SyntaxKind::RBrace, "`}`");
        self.finish();
    }

    /// `<A, B>` — the type parameters a declaration abstracts over.
    fn type_params(&mut self) {
        self.start(SyntaxKind::TypeParams);
        self.bump(); // `<`
        while !self.at(SyntaxKind::Greater) && self.current().is_some() {
            if self.at(SyntaxKind::Identifier) {
                self.start(SyntaxKind::TypeParam);
                self.bump();
                self.finish();
                if self.at(SyntaxKind::Comma) {
                    self.bump();
                }
            } else {
                self.expected("a type parameter name, or `>`");
                self.recover(&[SyntaxKind::Identifier, SyntaxKind::Greater, SyntaxKind::LBrace]);
                break;
            }
        }
        self.expect(SyntaxKind::Greater, "`>`");
        self.finish();
    }

    /// `Sounded(pitch: Pitch, held: Duration)` — one constructor.
    fn data_variant(&mut self) {
        self.start(SyntaxKind::DataVariant);
        self.bump(); // the constructor's name
        if self.at(SyntaxKind::LParen) {
            self.bump();
            while !self.at(SyntaxKind::RParen) && self.current().is_some() {
                if self.at(SyntaxKind::Identifier) {
                    self.start(SyntaxKind::DataField);
                    self.bump();
                    self.expect(SyntaxKind::Colon, "`:`");
                    self.type_expr();
                    self.finish();
                    if self.at(SyntaxKind::Comma) {
                        self.bump();
                    }
                } else {
                    self.expected("a field name, or `)`");
                    self.recover(&[SyntaxKind::Identifier, SyntaxKind::RParen]);
                    break;
                }
            }
            self.expect(SyntaxKind::RParen, "`)`");
        }
        self.finish();
    }

    /// `data Motive;` — one signature member naming a type and not its
    /// constructors.
    fn data_member(&mut self) {
        self.start(SyntaxKind::DataMember);
        self.bump(); // data
        self.expect(SyntaxKind::Identifier, "a type name");
        if self.at(SyntaxKind::Less) {
            self.type_params();
        }
        self.expect(SyntaxKind::Semicolon, "`;`");
        self.finish();
    }

    /// `let tonic: key;` — one signature member.
    fn signature_member(&mut self) {
        self.start(SyntaxKind::SignatureMember);
        self.bump(); // let
        self.expect(SyntaxKind::Identifier, "a member name");
        self.expect(SyntaxKind::Colon, "`:`");
        self.type_expr();
        self.expect(SyntaxKind::Semicolon, "`;`");
        self.finish();
    }

    /// `structure CMajor : TonalContext { ... }`, or the same with a
    /// parameter list for the structure a `template` parameterizes.
    fn structure_decl(&mut self) {
        self.start(SyntaxKind::StructureDecl);
        if self.at(SyntaxKind::ModuleKw) {
            self.moved_to_structure();
        }
        self.bump(); // `structure`, or the `module` that should have been one
        self.expect(SyntaxKind::Identifier, "a structure name");
        if self.at(SyntaxKind::LParen) {
            self.param_list();
        }
        self.expect(SyntaxKind::Colon, "`:`");
        self.expect(SyntaxKind::Identifier, "the signature this structure provides");
        self.expect(SyntaxKind::LBrace, "`{`");
        while !self.at(SyntaxKind::RBrace) && self.current().is_some() {
            if self.at(SyntaxKind::LetKw) {
                self.let_decl();
            } else if self.at(SyntaxKind::FnKw) {
                self.fn_decl();
            } else if self.at(SyntaxKind::DataKw) {
                self.data_decl();
            } else {
                self.expected("`let`, `fn`, `data`, or `}`");
                self.recover(&[
                    SyntaxKind::LetKw,
                    SyntaxKind::FnKw,
                    SyntaxKind::DataKw,
                    SyntaxKind::RBrace,
                ]);
            }
        }
        self.expect(SyntaxKind::RBrace, "`}`");
        self.finish();
    }

    /// `template piece study(k: key) "Study" { ... }`, or the same for a
    /// voice.
    ///
    /// The node wraps an ordinary [`SyntaxKind::PieceDecl`] or
    /// [`SyntaxKind::VoiceDecl`] carrying a parameter list, so a template
    /// body is read by exactly the accessors a written-out declaration is.
    /// The only thing `template` adds is the word that says the declaration
    /// is a pattern rather than a thing.
    fn template_decl(&mut self) {
        self.start(SyntaxKind::TemplateDecl);
        self.bump(); // template
        if self.at(SyntaxKind::PieceKw) {
            self.piece_decl();
        } else if self.at(SyntaxKind::VoiceKw) {
            self.voice_decl();
        } else if self.at_any(&[SyntaxKind::StructureKw, SyntaxKind::ModuleKw]) {
            self.structure_decl();
        } else {
            self.expected_with_help(
                "`piece`, `voice`, or `structure`",
                "a template parameterizes a declaration, and those are the three kinds it may parameterize",
            );
            self.recover(ROOT_RECOVERY);
        }
        self.finish();
    }

    /// `make study(key g major) as study_in_g;` — one instance site.
    fn make_stmt(&mut self) {
        self.start(SyntaxKind::MakeStmt);
        self.bump(); // make
        self.expect(SyntaxKind::Identifier, "a template name");
        if self.at(SyntaxKind::LParen) {
            self.expr_arg_list();
        } else {
            self.expected("`(`");
        }
        self.expect(SyntaxKind::AsKw, "`as`");
        self.expect(SyntaxKind::Identifier, "a name for what is made");
        self.expect(SyntaxKind::Semicolon, "`;`");
        self.finish();
    }

    /// `piece "name" { ... }`, or `piece study(k: key) "Study" { ... }` for
    /// the piece a `template` parameterizes.
    fn piece_decl(&mut self) {
        self.start(SyntaxKind::PieceDecl);
        self.expect(SyntaxKind::PieceKw, "`piece`");
        // A template's piece is named twice: once as the template, in code,
        // and once as the piece, on the page. The identifier is the first.
        if self.at(SyntaxKind::Identifier) {
            self.bump();
            self.param_list();
        }
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
            if self.at_any(&[SyntaxKind::ImportKw, SyntaxKind::UseKw]) {
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
            } else if self.at(SyntaxKind::FragmentKw) {
                self.fragment_decl();
            } else if self.at(SyntaxKind::LetKw) {
                self.let_decl();
            } else if self.at(SyntaxKind::FnKw) {
                self.fn_decl();
            } else if self.at(SyntaxKind::DataKw) {
                self.data_decl();
            } else if self.at(SyntaxKind::ScoreKw) {
                self.score_decl();
            } else if self.at(SyntaxKind::PerformanceKw) {
                self.performance_decl();
            } else if self.at(SyntaxKind::StudioKw) {
                self.studio_decl();
            } else {
                self.expected_with_help(
                    "a declaration",
                    "a piece holds imports, value/function declarations, score declarations, performance, and studio",
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
    /// `tempo 1/4 = 92;`, `tempo 1/4 = 132 "Allegro vivace";`, `tempo "Andante";`
    ///
    /// The third form is the one that shapes the grammar. A metronome mark and
    /// a tempo word are two different things a composer writes — often one
    /// without the other — so the number is optional, the word is optional,
    /// and a statement with neither is the error.
    fn tempo_stmt(&mut self) {
        self.start(SyntaxKind::TempoStmt);
        self.bump(); // tempo
        // The word may lead (`tempo "Andante";`) or trail
        // (`tempo 1/4 = 132 "Allegro";`) — wherever it can be read aloud in
        // the order it is written. One or the other, never both.
        let leads = self.at(SyntaxKind::String);
        if leads {
            self.bump();
        } else {
            if self.at_any(&[SyntaxKind::Identifier, SyntaxKind::Rational]) {
                self.bump();
            } else {
                self.expected("a beat unit (`quarter` or `1/4`) or a tempo word in quotes");
            }
            self.expect(SyntaxKind::Equals, "`=`");
            self.expect(SyntaxKind::Integer, "a tempo in bpm");
            // `to 60` — where a gradual change arrives, in the same beat
            // unit. `to` rather than a keyword of its own, for the reason
            // `crescendo to f` reads: arriving somewhere is one idea.
            if self.at(SyntaxKind::ToKw) {
                self.bump();
                self.expect(SyntaxKind::Integer, "the tempo the change arrives at");
            }
        }
        // `over 4/1` — how far a gradual change reaches.
        if self.at(SyntaxKind::OverKw) {
            self.bump();
            self.expect(SyntaxKind::Rational, "how far the change reaches, like `4/1`");
        }
        if !leads && self.at(SyntaxKind::String) {
            self.bump();
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
            if self.at_any(&[SyntaxKind::ImportKw, SyntaxKind::UseKw]) {
                self.import_stmt();
            } else if self.at(SyntaxKind::MotifKw) {
                self.motif_decl();
            } else if self.at(SyntaxKind::FragmentKw) {
                self.fragment_decl();
            } else if self.at(SyntaxKind::LetKw) {
                self.let_decl();
            } else if self.at(SyntaxKind::FnKw) {
                self.fn_decl();
            } else if self.at(SyntaxKind::PerformanceKw) {
                self.performance_decl();
            } else if self.at(SyntaxKind::StudioKw) {
                self.studio_decl();
            } else if self.at(SyntaxKind::SignatureKw) {
                self.signature_decl();
            } else if self.at(SyntaxKind::DataKw) {
                self.data_decl();
            } else if self.at_any(&[SyntaxKind::StructureKw, SyntaxKind::ModuleKw]) {
                self.structure_decl();
            } else if self.at(SyntaxKind::TemplateKw)
                && matches!(
                    self.nth_significant(1),
                    Some(SyntaxKind::StructureKw | SyntaxKind::ModuleKw)
                )
            {
                // A piece or a voice belongs to a piece, so the one template
                // a library may hold is the one that makes a module.
                self.template_decl();
            } else if self.at(SyntaxKind::MakeKw) {
                self.make_stmt();
            } else {
                // A library holds what can be shared. Music belongs to a
                // piece, which is why `score` is not in this list.
                self.expected_with_help(
                    "a declaration",
                    "a library holds imports, reusable values/functions, material, signatures, modules, performance, \
                     and studio declarations",
                );
                self.recover(PIECE_RECOVERY);
            }
        }
        self.finish();
    }

    /// `import "../library/motifs.musa";` or `import std::core as core;`
    ///
    /// Nothing is told apart by lookahead any more: `import` is one statement
    /// and `use` is the other, which is the whole point of there being two
    /// words. The old spelling is still read here so that a file written
    /// against it parses into the same shape and gets one located complaint
    /// instead of a cascade.
    fn import_stmt(&mut self) {
        self.start(SyntaxKind::ImportStmt);
        if self.at(SyntaxKind::UseKw) {
            self.moved_to_import();
        }
        self.bump(); // `import`, or the `use` that should have been one
        if self.at(SyntaxKind::String) {
            self.bump();
        } else {
            self.expect(SyntaxKind::Identifier, "a relative path in quotes, or `std::module`");
            // A path nests as far as the package's module tree does, and the
            // parser counts no segments: how deep `std::tonal::harmony` is
            // is a fact about `stdlib/`, not about the grammar.
            loop {
                self.expect(SyntaxKind::Colon, "`::`");
                self.expect(SyntaxKind::Colon, "`::`");
                // A module may be named after a type or a domain keyword:
                // `pitch`, `list`, `option`, `scale`, `harmony`. The path
                // position is what makes the word a module name.
                if self.at_any(MODULE_NAME) {
                    self.bump();
                } else {
                    self.expected("a module name");
                    break;
                }
                if !self.at(SyntaxKind::Colon) {
                    break;
                }
            }
        }
        if self.at(SyntaxKind::AsKw) {
            self.bump();
            self.expect(SyntaxKind::Identifier, "a name for the imported module");
        }
        self.expect(SyntaxKind::Semicolon, "`;`");
        self.finish();
    }

    /// The one complaint a file written against the old spelling gets.
    ///
    /// Located at the word itself and carrying the word that replaces it, so
    /// the migration is an accepted fix rather than a search. Both meanings
    /// are named because the reader's next question is which `use` this was:
    /// the splices in their score are not affected and should not be touched.
    fn moved_to_import(&mut self) {
        if self.cascading() {
            return;
        }
        let Some(token) = self.significant() else {
            return;
        };
        self.errors.push(
            SyntaxError::new(
                token.range,
                "`use` no longer imports",
                "this brings in a file, so it is an `import`",
            )
            .with_help("`use` writes out a motif where it stands; `import` brings another file's names into this one")
            .with_fix("write `import`", "import"),
        );
    }

    /// The one complaint a file written against the old spelling gets.
    ///
    /// Located at the word itself and carrying the word that replaces it, so
    /// the migration is an accepted fix rather than a search. `module` is
    /// still a word in this language — it names a node of a package's tree —
    /// which is exactly why the thing it used to declare needed its own.
    fn moved_to_structure(&mut self) {
        if self.cascading() {
            return;
        }
        let Some(token) = self.significant() else {
            return;
        };
        self.errors.push(
            SyntaxError::new(
                token.range,
                "`module` no longer declares one",
                "this provides a signature, so it is a `structure`",
            )
            .with_help(
                "a `module` is a node of a package's tree, declared with `mod`; a `structure` provides a `signature`",
            )
            .with_fix("write `structure`", "structure"),
        );
    }

    /// `meter <n>/<d>;` or `meter none;`
    fn meter_stmt(&mut self) {
        self.start(SyntaxKind::MeterStmt);
        self.bump(); // meter
        // `none` is a value of the meter, not a mechanism beside it: music
        // with no barlines is music whose meter says there are none. It is
        // spelled with the identifier rather than a keyword because there is
        // nothing else `meter` can be followed by.
        if self.at(SyntaxKind::Identifier) {
            self.bump();
        } else {
            self.expect(SyntaxKind::Rational, "a meter such as `4/4`, or `none`");
        }
        self.expect(SyntaxKind::Semicolon, "`;`");
        self.finish();
    }

    /// `let name = expression;`, with an optional `: type` before the `=`.
    ///
    /// The annotation is written when it says something the expression does
    /// not — a public signature, a narrower type than the value's own — and
    /// omitted when it would only repeat what is already there. Which of the
    /// two a file chose is a fact about the file, so the colon and the type
    /// stay in the tree exactly where they were written.
    fn let_decl(&mut self) {
        self.start(SyntaxKind::LetDecl);
        self.bump();
        self.expect(SyntaxKind::Identifier, "a binding name");
        if self.at(SyntaxKind::Colon) {
            self.bump();
            self.type_expr();
        }
        self.expect(SyntaxKind::Equals, "`=`");
        self.expr();
        self.expect(SyntaxKind::Semicolon, "`;`");
        self.finish();
    }

    /// `fn name(parameters) { expression }`, with an optional `-> result`.
    ///
    /// A parameter's `: type` is optional for the same reason a `let`'s is,
    /// and both are read here in the same way: present if written, absent if
    /// not, never invented. The body is a [`Self::block_expr`], which every
    /// other body in this language is delimited by. The old `= expression;`
    /// is read here too, so that a file written against it gets one complaint
    /// carrying the rewrite rather than a cascade about a missing `{`.
    fn fn_decl(&mut self) {
        self.start(SyntaxKind::FnDecl);
        self.bump();
        self.expect(SyntaxKind::Identifier, "a function name");
        self.param_list();
        if self.at(SyntaxKind::Arrow) {
            self.bump();
            self.type_expr();
        }
        if self.at(SyntaxKind::Equals) {
            self.old_function_body();
        } else {
            self.block_expr();
        }
        self.finish();
    }

    /// The one complaint a file written against `fn f() -> τ = e;` gets.
    ///
    /// The whole `= e;` is read, so the tree is the tree the file describes
    /// and the error spans exactly the text the fix replaces. The fix is the
    /// body written back between braces: mechanical, because the expression
    /// is unchanged and only its delimiters moved.
    fn old_function_body(&mut self) {
        let equals = self.significant().map(|token| token.range.start());
        self.bump(); // `=`
        let body_start = self.significant().map(|token| token.range.start());
        self.expr();
        let body_end = self.previous_end();
        self.expect(SyntaxKind::Semicolon, "`;`");
        let end = self.previous_end();
        if self.cascading() {
            return;
        }
        let (Some(equals), Some(body_start)) = (equals, body_start) else {
            return;
        };
        let Some(body) = self.source.get(usize::from(body_start)..usize::from(body_end)) else {
            return;
        };
        self.errors.push(
            SyntaxError::new(
                TextRange::new(equals, end),
                "a function body is written in braces",
                "this body is `= expression;`",
            )
            .with_help("`fn f(x: Nat) -> Nat { g(x) }` — the braces are the body, and they hold one expression")
            .with_fix("write the body in braces", format!("{{ {body} }}")),
        );
    }

    fn param_list(&mut self) {
        self.start(SyntaxKind::ParamList);
        self.expect(SyntaxKind::LParen, "`(`");
        while !self.at(SyntaxKind::RParen) && self.current().is_some() {
            self.start(SyntaxKind::Param);
            self.expect(SyntaxKind::Identifier, "a parameter name");
            if self.at(SyntaxKind::Colon) {
                self.bump();
                self.type_expr();
            }
            if self.at(SyntaxKind::Equals) {
                self.bump();
                self.expr();
            }
            self.finish();
            if !self.at(SyntaxKind::Comma) {
                break;
            }
            self.bump();
        }
        self.expect(SyntaxKind::RParen, "`)`");
        self.finish();
    }

    /// Right-associative arrow types; product/list/option types are atoms.
    fn type_expr(&mut self) {
        let checkpoint = self.events.len();
        self.type_atom();
        if self.at(SyntaxKind::Arrow) {
            self.start_at(checkpoint, SyntaxKind::FunctionType);
            self.bump();
            self.type_expr();
            self.finish();
        }
    }

    /// A type is an identifier, a parenthesized or product type, or one of the
    /// two the compiler parameterizes.
    ///
    /// No keyword stands here. `key` is a statement and `Key` is a type, and
    /// the capital is what tells them apart — which is why this reads a plain
    /// [`SyntaxKind::Identifier`] rather than a list of the keywords a type
    /// was allowed to also be. The removed spellings are read only to be
    /// reported.
    fn type_atom(&mut self) {
        let parameterized = match self.current() {
            Some(SyntaxKind::OptionKw) => Some(SyntaxKind::OptionType),
            Some(SyntaxKind::ListKw) => Some(SyntaxKind::ListType),
            Some(SyntaxKind::Identifier) if self.at_word("option") => Some(SyntaxKind::OptionType),
            Some(SyntaxKind::Identifier) if self.at_word("list") => Some(SyntaxKind::ListType),
            _ => None,
        };
        if let Some(kind) = parameterized {
            self.start(kind);
            self.respelled_type();
            self.bump();
            if self.at(SyntaxKind::LBracket) {
                self.bracketed_parameter();
            } else {
                self.expect(SyntaxKind::Less, "`<`");
                self.type_expr();
                self.expect(SyntaxKind::Greater, "`>`");
            }
            self.finish();
            return;
        }
        if self.at(SyntaxKind::ResultKw) || (self.at(SyntaxKind::Identifier) && self.at_word("result")) {
            self.start(SyntaxKind::ResultType);
            self.respelled_type();
            self.bump();
            // Two parameters, and no bracketed legacy form: `Result` is new,
            // so there is no `Result[τ]` anybody could have written.
            self.expect(SyntaxKind::Less, "`<`");
            self.type_expr();
            self.expect(SyntaxKind::Comma, "`,`");
            self.type_expr();
            self.expect(SyntaxKind::Greater, "`>`");
            self.finish();
            return;
        }
        if self.at(SyntaxKind::LParen) {
            let checkpoint = self.events.len();
            self.bump();
            self.type_expr();
            if self.at(SyntaxKind::Comma) {
                self.start_at(checkpoint, SyntaxKind::ProductType);
                while self.at(SyntaxKind::Comma) {
                    self.bump();
                    self.type_expr();
                }
            } else {
                self.start_at(checkpoint, SyntaxKind::TypeExpr);
            }
            self.expect(SyntaxKind::RParen, "`)`");
            self.finish();
            return;
        }
        self.eat_trivia();
        // `Tree<Nat>` — a declared type applied to its arguments. Only a
        // parameterized `data` declaration can be written this way, and which
        // names are declarations is not a question the parser can answer, so
        // the shape is what decides the node.
        if self.at(SyntaxKind::Identifier) && self.nth_significant(1) == Some(SyntaxKind::Less) {
            self.start(SyntaxKind::AppliedType);
            self.start(SyntaxKind::TypeName);
            self.bump();
            self.finish();
            self.bump(); // `<`
            self.type_expr();
            while self.at(SyntaxKind::Comma) {
                self.bump();
                self.type_expr();
            }
            self.expect(SyntaxKind::Greater, "`>`");
            self.finish();
            return;
        }
        self.start(SyntaxKind::TypeName);
        if self.at(SyntaxKind::Identifier) || self.at_any(MOVED_TYPE_KEYWORDS) {
            self.respelled_type();
            self.bump();
        } else {
            self.expected("a type");
        }
        self.finish();
    }

    /// The one complaint `Option[τ]` gets, and the tree it still produces.
    ///
    /// Located at the whole bracketed parameter rather than at either bracket,
    /// because the fix rewrites a *pair* and an edit offered on one half alone
    /// would leave the other behind. The inner type is parsed into the node it
    /// belongs to, so a file with one old parameter gets one complaint and the
    /// declaration around it is still checked.
    ///
    /// In doubly-obsolete source — `option[voicing]`, with both an old word
    /// and old brackets — this range contains the inner type's own
    /// respelling fix, and the replacement carries the inner text over
    /// unrespelled. That is deliberate: the bracket complaint fixes brackets and
    /// the word complaint fixes the word, and nothing here applies both at once.
    /// `musa check --fix` rewrites warnings only, and an editor
    /// applies one code action, reparses, and finds the other complaint waiting
    /// at its new place. Respelling inside this fix would make one offer quietly
    /// do two jobs.
    fn bracketed_parameter(&mut self) {
        let open = self.significant().map(|token| token.range);
        self.bump();
        self.type_expr();
        let close = self.significant().filter(|token| token.kind == SyntaxKind::RBracket);
        let Some((open, close)) = open.zip(close.map(|token| token.range)) else {
            // No closing `]`: the ordinary missing-token complaint says more
            // than a migration note about a parameter that was never finished.
            self.expect(SyntaxKind::RBracket, "`]`");
            return;
        };
        self.bump();
        if self.cascading() {
            return;
        }
        let range = TextRange::new(open.start(), close.end());
        let inner = &self.source[usize::from(open.end())..usize::from(close.start())];
        self.errors.push(
            SyntaxError::new(range, "a type parameter is angle-bracketed", "this is `<…>`")
                .with_help(
                    "`[` means a list here — the literal `[c4, d4]` and the pattern `[x, ..xs]` — so the type layer \
                     takes `<` and `>` and the character reads one way",
                )
                .with_fix("write `<…>`", format!("<{inner}>")),
        );
    }

    /// The one complaint a type written in the old vocabulary gets.
    ///
    /// Located at the word and carrying the word that replaces it, so the
    /// migration is an accepted fix rather than a search. The old spelling is
    /// then read as the type it named, so the rest of the declaration is
    /// checked rather than buried under a cascade.
    fn respelled_type(&mut self) {
        let Some(was) = self.word() else {
            return;
        };
        let Some(now) = crate::respelled_type(was) else {
            return;
        };
        if self.cascading() {
            return;
        }
        let Some(token) = self.significant() else {
            return;
        };
        let help = if was == "pitchclass" {
            "`pitchclass` is `NoteName`: a pitch class forgets spelling, and this is the type that keeps it, so C♯ and \
             D♭ are two things here and one `Pc12`"
                .to_owned()
        } else {
            format!("every type the compiler owns is spelled with a capital, so `{was}` is `{now}`")
        };
        self.errors.push(
            SyntaxError::new(
                token.range,
                "a type is spelled with a capital",
                format!("this is `{now}`"),
            )
            .with_help(help)
            .with_fix(format!("write `{now}`"), now),
        );
    }

    /// The one complaint `some` or `none` gets.
    ///
    /// They move with their type: a constructor of `Option` carries `Option`'s
    /// capital, so that a reader holds one rule about spelling rather than
    /// two.
    fn respelled_constructor(&mut self) {
        let now = match self.word() {
            Some("some") => "Some",
            Some("none") => "None",
            _ => return,
        };
        if self.cascading() {
            return;
        }
        let Some(token) = self.significant() else {
            return;
        };
        self.errors.push(
            SyntaxError::new(
                token.range,
                "a constructor is spelled with a capital",
                format!("this is `{now}`"),
            )
            .with_help("`Some` and `None` are the two constructors of `Option`, and they are spelled the way it is")
            .with_fix(format!("write `{now}`"), now),
        );
    }

    /// An ordinary expression. Calls bind tighter than the written-pitch
    /// operators, and `step` binds tighter than `up`/`down`
    /// (`01-surface.md` §1), so `p step 1 up m2` steps first.
    ///
    /// Both pitch operators are non-associative: `p up M2 down m2` needs
    /// parentheses, and so does a second `step`.
    fn expr(&mut self) {
        let checkpoint = self.events.len();
        self.expr_atom();
        while self.at(SyntaxKind::LParen) {
            self.start_at(checkpoint, SyntaxKind::ApplyExpr);
            self.expr_arg_list();
            self.finish();
        }
        if self.at(SyntaxKind::StepKw) {
            self.start_at(checkpoint, SyntaxKind::StepExpr);
            self.bump();
            // The direction is optional and defaults to up, which is what
            // `c5 step 2` reads as on the page.
            if self.at_any(&[SyntaxKind::UpKw, SyntaxKind::DownKw]) {
                self.bump();
            }
            self.expr_atom();
            self.finish();
        }
        if self.at_any(&[SyntaxKind::UpKw, SyntaxKind::DownKw]) {
            self.start_at(checkpoint, SyntaxKind::PitchExpr);
            self.bump();
            self.expr_atom();
            self.finish();
        }
    }

    /// `scale <tonic> <collection>` — a collection rooted on a pitch class.
    fn scale_expr(&mut self) {
        self.start(SyntaxKind::ScaleExpr);
        self.bump(); // scale
        self.pitch_class();
        self.expect(SyntaxKind::Identifier, "a collection such as `major` or `dorian`");
        self.finish();
    }

    /// `key <tonic> <mode>` in a value position.
    ///
    /// The same three words as the `key` *statement*, and deliberately not
    /// the same thing: this one is a value a function can take, and writing
    /// it changes no signature and declares no modulation.
    fn key_expr(&mut self) {
        self.start(SyntaxKind::KeyExpr);
        self.bump(); // key
        self.pitch_class();
        self.expect(SyntaxKind::Identifier, "a mode (`major` or `minor`)");
        self.finish();
    }

    /// `chord <root> <type>` — rooted spelled content.
    ///
    /// The same shape as `scale c dorian`, and for the same reason: the words
    /// that name a chord type are a closed vocabulary the compiler owns, so
    /// they are read here as one literal rather than as a call whose second
    /// argument would have to be a value nobody can write.
    fn chord_expr(&mut self) {
        self.start(SyntaxKind::ChordExpr);
        self.bump(); // chord
        self.pitch_class();
        self.expect(SyntaxKind::Identifier, "a chord type such as `major` or `major7`");
        self.finish();
    }

    fn expr_atom(&mut self) {
        match self.current() {
            Some(SyntaxKind::Identifier) if self.at_constructor() => self.option_expr(),
            Some(
                SyntaxKind::Identifier
                | SyntaxKind::RepeatKw
                | SyntaxKind::TransposeKw
                | SyntaxKind::StretchKw
                | SyntaxKind::RetrogradeKw
                | SyntaxKind::InvertKw,
            ) => {
                self.start(SyntaxKind::NameExpr);
                self.bump();
                // `Module.member` — one name in two words. A dot only ever
                // reads this way here: a dotted duration follows a rational,
                // never a name.
                if self.at(SyntaxKind::Dot) && self.nth_significant(1) == Some(SyntaxKind::Identifier) {
                    self.bump();
                    self.bump();
                }
                self.finish();
            }
            Some(
                SyntaxKind::Integer
                | SyntaxKind::Rational
                | SyntaxKind::PitchLiteral
                | SyntaxKind::IntervalLiteral
                | SyntaxKind::TrueKw
                | SyntaxKind::FalseKw
                | SyntaxKind::String,
            ) => {
                self.start(SyntaxKind::LiteralExpr);
                self.bump();
                self.finish();
            }
            Some(SyntaxKind::NoneKw | SyntaxKind::SomeKw) => self.option_expr(),
            Some(SyntaxKind::OkKw | SyntaxKind::ErrKw) => self.result_expr(),
            Some(SyntaxKind::LBracket) => self.list_expr(),
            Some(SyntaxKind::LBrace) => self.block_expr(),
            Some(SyntaxKind::LParen) => self.paren_or_product_expr(),
            Some(SyntaxKind::MatchKw) => self.match_expr(),
            Some(SyntaxKind::MusicKw) => self.music_expr(),
            Some(SyntaxKind::KernelKw) => self.kernel_quote(),
            Some(SyntaxKind::ScaleKw) => self.scale_expr(),
            Some(SyntaxKind::KeyKw) => self.key_expr(),
            Some(SyntaxKind::ChordKw) => self.chord_expr(),
            _ => self.expected("an expression"),
        }
    }

    /// `{ expression }` — a block, which is a delimiter and not a sequence.
    ///
    /// Exactly one expression, because there is no statement here to be the
    /// second one: `⟦{ e }⟧ = ⟦e⟧`, so a block adds a shape to the surface
    /// and nothing to the calculus. Anything after the first expression is
    /// reported as the rule it breaks rather than read as a sequence nobody
    /// wrote a semantics for.
    fn block_expr(&mut self) {
        self.start(SyntaxKind::BlockExpr);
        self.expect(SyntaxKind::LBrace, "`{`");
        self.expr();
        if !self.at(SyntaxKind::RBrace) && self.current().is_some() {
            self.second_expression_in_a_block();
            self.recover(&[SyntaxKind::RBrace]);
        }
        self.expect(SyntaxKind::RBrace, "`}`");
        self.finish();
    }

    /// A block holding more than one expression, named as the rule it breaks.
    ///
    /// A silent sequence would be a statement language arriving by accident,
    /// so the complaint says what a block is instead of what was expected
    /// where reading stopped.
    fn second_expression_in_a_block(&mut self) {
        if self.cascading() {
            return;
        }
        let Some(token) = self.significant() else {
            return;
        };
        self.errors.push(
            SyntaxError::new(
                token.range,
                "a block holds one expression",
                "a second expression begins here",
            )
            .with_help("braces delimit a body, they do not sequence one: `{ e }` is `e`"),
        );
    }

    fn expr_arg_list(&mut self) {
        self.start(SyntaxKind::ExprArgList);
        self.bump();
        while !self.at(SyntaxKind::RParen) && self.current().is_some() {
            self.start(SyntaxKind::ExprArg);
            if self.at(SyntaxKind::Identifier) && self.nth_significant(1) == Some(SyntaxKind::Colon) {
                self.bump();
                self.bump();
            }
            self.expr();
            self.finish();
            if !self.at(SyntaxKind::Comma) {
                break;
            }
            self.bump();
        }
        self.expect(SyntaxKind::RParen, "`)`");
        self.finish();
    }

    fn paren_or_product_expr(&mut self) {
        let checkpoint = self.events.len();
        self.bump();
        self.expr();
        let product = self.at(SyntaxKind::Comma);
        if product {
            self.start_at(checkpoint, SyntaxKind::ProductExpr);
            while self.at(SyntaxKind::Comma) {
                self.bump();
                self.expr();
            }
        } else {
            self.start_at(checkpoint, SyntaxKind::ParenExpr);
        }
        self.expect(SyntaxKind::RParen, "`)`");
        self.finish();
    }

    fn list_expr(&mut self) {
        self.start(SyntaxKind::ListExpr);
        self.bump();
        while !self.at(SyntaxKind::RBracket) && self.current().is_some() {
            self.expr();
            if !self.at(SyntaxKind::Comma) {
                break;
            }
            self.bump();
        }
        self.expect(SyntaxKind::RBracket, "`]`");
        self.finish();
    }

    fn option_expr(&mut self) {
        self.start(SyntaxKind::OptionExpr);
        let some = self.at(SyntaxKind::SomeKw) || self.at_word("some");
        self.respelled_constructor();
        self.bump();
        if some {
            self.expect(SyntaxKind::LParen, "`(`");
            self.expr();
            self.expect(SyntaxKind::RParen, "`)`");
        }
        self.finish();
    }

    /// `Ok(value)` or `Err(reason)` — one injection into the binary sum.
    ///
    /// Both carry a value, unlike `None`, because a sum has no empty side:
    /// the shape is the same either way, and which side it is is the whole
    /// information the constructor adds.
    fn result_expr(&mut self) {
        self.start(SyntaxKind::ResultExpr);
        self.bump();
        self.expect(SyntaxKind::LParen, "`(`");
        self.expr();
        self.expect(SyntaxKind::RParen, "`)`");
        self.finish();
    }

    fn match_expr(&mut self) {
        self.start(SyntaxKind::MatchExpr);
        self.bump();
        self.expr();
        self.expect(SyntaxKind::LBrace, "`{`");
        if self.at(SyntaxKind::RBrace) {
            self.expected("a match arm");
        }
        while !self.at(SyntaxKind::RBrace) && self.current().is_some() {
            self.start(SyntaxKind::MatchArm);
            self.pattern();
            self.expect(SyntaxKind::Arrow, "`->`");
            self.expr();
            self.finish();
            if !self.at(SyntaxKind::Comma) {
                break;
            }
            self.bump();
        }
        self.expect(SyntaxKind::RBrace, "`}`");
        self.finish();
    }

    fn pattern(&mut self) {
        self.start(SyntaxKind::Pattern);
        match self.current() {
            Some(SyntaxKind::Identifier) if self.at_word("none") => {
                self.respelled_constructor();
                self.bump();
            }
            Some(SyntaxKind::Identifier) if self.at_word("some") => {
                self.respelled_constructor();
                self.bump();
                self.expect(SyntaxKind::LParen, "`(`");
                self.expect(SyntaxKind::Identifier, "a binding name");
                self.expect(SyntaxKind::RParen, "`)`");
            }
            // `Sounded(pitch, held)` — a declared constructor, taking one
            // binding per field. A bare name is still one token here, because
            // whether `Silence` is a constructor or a binding is a question
            // about what is declared, which the checker answers against the
            // type being matched.
            Some(SyntaxKind::Identifier) if self.nth_significant(1) == Some(SyntaxKind::LParen) => {
                self.bump();
                self.bump(); // `(`
                while !self.at(SyntaxKind::RParen) && self.current().is_some() {
                    self.expect(SyntaxKind::Identifier, "a binding name");
                    if self.at(SyntaxKind::Comma) {
                        self.bump();
                    } else {
                        break;
                    }
                }
                self.expect(SyntaxKind::RParen, "`)`");
            }
            Some(
                SyntaxKind::Identifier
                | SyntaxKind::Integer
                | SyntaxKind::Rational
                | SyntaxKind::PitchLiteral
                | SyntaxKind::IntervalLiteral
                | SyntaxKind::TrueKw
                | SyntaxKind::FalseKw
                | SyntaxKind::String
                | SyntaxKind::NoneKw,
            ) => self.bump(),
            Some(SyntaxKind::SomeKw | SyntaxKind::OkKw | SyntaxKind::ErrKw) => {
                self.bump();
                self.expect(SyntaxKind::LParen, "`(`");
                self.expect(SyntaxKind::Identifier, "a binding name");
                self.expect(SyntaxKind::RParen, "`)`");
            }
            Some(SyntaxKind::LBracket) => {
                self.bump();
                if !self.at(SyntaxKind::RBracket) {
                    self.expect(SyntaxKind::Identifier, "a head binding");
                    self.expect(SyntaxKind::Comma, "`,`");
                    self.expect(SyntaxKind::Dot, "`.`");
                    self.expect(SyntaxKind::Dot, "`.`");
                    self.expect(SyntaxKind::Identifier, "a tail binding");
                }
                self.expect(SyntaxKind::RBracket, "`]`");
            }
            Some(SyntaxKind::LParen) => {
                self.bump();
                self.expect(SyntaxKind::Identifier, "a binding name");
                self.expect(SyntaxKind::Comma, "`,`");
                self.expect(SyntaxKind::Identifier, "a binding name");
                while self.at(SyntaxKind::Comma) {
                    self.bump();
                    self.expect(SyntaxKind::Identifier, "a binding name");
                }
                self.expect(SyntaxKind::RParen, "`)`");
            }
            _ => self.expected("a match pattern"),
        }
        self.finish();
    }

    fn music_expr(&mut self) {
        self.start(SyntaxKind::MusicExpr);
        self.bump();
        self.expect(SyntaxKind::LBrace, "`{`");
        self.voice_items();
        self.expect(SyntaxKind::RBrace, "`}`");
        self.finish();
    }

    /// `kernel Timeline[ScoreFact] { … }` — a quoted composition expression
    /// (`docs/rules/language/01-surface.md` §7).
    ///
    /// **Recognized, not read.** The tokens between the braces spell the
    /// kernel's own grammar, and `musa-kernel` owns that grammar: a second
    /// reading of it here would be a second thing to keep in step with the
    /// first. What this crate must find is the shape — where the quote ends,
    /// and where the holes are — because those are the two questions a
    /// lossless tree and an editor ask. The words in between are handed
    /// along as source text.
    ///
    /// The braces are counted rather than matched against a production, so a
    /// `timeline … { … }` inside the quote does not end it, and a quote that
    /// is never closed ends at the end of the file rather than eating the
    /// declaration after it.
    fn kernel_quote(&mut self) {
        self.start(SyntaxKind::KernelQuote);
        self.bump(); // kernel
        self.expect(SyntaxKind::Identifier, "`Timeline`");
        self.expect(SyntaxKind::LBracket, "`[`");
        self.expect(SyntaxKind::Identifier, "a payload type");
        self.expect(SyntaxKind::RBracket, "`]`");
        self.expect(SyntaxKind::LBrace, "`{`");
        let mut depth = 0_usize;
        loop {
            match self.current() {
                None => break,
                Some(SyntaxKind::RBrace) if depth == 0 => break,
                Some(SyntaxKind::RBrace) => {
                    depth = depth.saturating_sub(1);
                    self.bump();
                }
                Some(SyntaxKind::LBrace) => {
                    depth = depth.saturating_add(1);
                    self.bump();
                }
                // Every `$` in a quote is a hole attempted: the kernel's
                // grammar has no other use for the character, so reading it
                // as one and complaining about what follows says more than
                // "unexpected token" would.
                Some(SyntaxKind::Dollar) => self.kernel_hole(),
                Some(_) => self.bump(),
            }
        }
        self.expect(SyntaxKind::RBrace, "`}`");
        self.finish();
    }

    /// `${ expr }` — one typed antiquotation, whose interior is host syntax.
    fn kernel_hole(&mut self) {
        self.start(SyntaxKind::KernelHole);
        self.bump(); // $
        self.expect(SyntaxKind::LBrace, "`{`");
        self.expr();
        self.expect(SyntaxKind::RBrace, "`}`");
        self.finish();
    }

    /// `senza { ... }` — an unmeasured stretch, and the meter back after it.
    fn senza_stmt(&mut self) {
        self.start(SyntaxKind::SenzaStmt);
        self.bump(); // senza
        self.block();
        self.finish();
    }

    /// `key <pitch-class> <mode>;`
    fn key_stmt(&mut self) {
        self.start(SyntaxKind::KeyStmt);
        self.bump(); // key
        // `key a minor;` writes the key out; `key k;` and `key M.k;` name one
        // a template was given or a module holds. One word before the `;`
        // cannot be both a tonic and a mode, so it is a name — and `key a;`
        // was never valid, so nothing that parsed before parses differently
        // now.
        let named = self.at(SyntaxKind::Identifier)
            && (self.nth_significant(1) == Some(SyntaxKind::Semicolon)
                || (self.nth_significant(1) == Some(SyntaxKind::Dot)
                    && self.nth_significant(3) == Some(SyntaxKind::Semicolon)));
        if named {
            self.expr();
        } else {
            self.pitch_class();
            self.expect(SyntaxKind::Identifier, "a mode (`major` or `minor`)");
        }
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
            // A motif parameter is `Pitch` or `Duration` — the same two types
            // the rest of the language names, so they are written and read
            // the same way here.
            self.eat_trivia();
            self.start(SyntaxKind::TypeName);
            if self.at(SyntaxKind::Identifier) || self.at_any(MOVED_TYPE_KEYWORDS) {
                self.respelled_type();
                self.bump();
            } else {
                self.expected("a parameter type (`Pitch` or `Duration`)");
            }
            self.finish();
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

    /// `fragment name { ... }` — material a performance may reorder.
    ///
    /// A motif without parameters, tagged differently: a motif is material a
    /// *composer* reuses, a fragment is material a *performance* arranges. The
    /// namespace is one namespace, so `use` reaches both and a name collision
    /// is the diagnostic the resolver already writes.
    fn fragment_decl(&mut self) {
        self.start(SyntaxKind::FragmentDecl);
        self.bump(); // fragment
        self.expect(SyntaxKind::Identifier, "a fragment name");
        self.block();
        self.finish();
    }

    /// `mobile { a; b; c; }` — its fragments in an order the performance
    /// chooses.
    ///
    /// A list of names, not a block of music: what a mobile arranges is
    /// *material*, and writing the notes inline would mean the same figure
    /// could not be reached from anywhere else.
    fn mobile_stmt(&mut self) {
        self.start(SyntaxKind::MobileStmt);
        self.bump(); // mobile
        self.expect(SyntaxKind::LBrace, "`{`");
        loop {
            if self.at(SyntaxKind::RBrace) {
                self.bump();
                break;
            }
            if self.current().is_none() {
                if let Some(error) = self.unclosed("`mobile` block") {
                    self.errors.push(error);
                }
                break;
            }
            if self.at(SyntaxKind::Identifier) {
                self.bump();
                self.expect(SyntaxKind::Semicolon, "`;`");
            } else {
                self.expected_with_help("a fragment name", "a mobile lists fragments: `mobile { a; b; c; }`");
                self.recover(&[SyntaxKind::RBrace, SyntaxKind::Identifier, SyntaxKind::Semicolon]);
            }
        }
        self.finish();
    }

    /// `improvise 8/1 over "Dm7 | G7";` — a frame with unnotated contents.
    fn improvise_stmt(&mut self) {
        self.start(SyntaxKind::ImproviseStmt);
        self.bump(); // improvise
        self.duration();
        if self.at(SyntaxKind::OverKw) {
            self.bump();
            self.expect(SyntaxKind::String, "the changes to play over, in quotes");
        }
        self.expect(SyntaxKind::Semicolon, "`;`");
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

    /// `part name { clef ...; meter ...; tempo ...; voice ... }`
    ///
    /// A `meter` or a `tempo` here is the part's own, in force for the whole
    /// part: polymeter and polytempo. The part is the granularity because it
    /// is the one every consumer can express — a staff has one set of
    /// barlines in MEI, in `MusicXML` and on paper, so a per-voice barline
    /// grid would be a document nothing could draw.
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
            } else if self.at(SyntaxKind::MeterKw) {
                self.meter_stmt();
            } else if self.at(SyntaxKind::TempoKw) {
                self.tempo_stmt();
            } else if self.at(SyntaxKind::ProfileKw) {
                self.profile_stmt();
            } else if self.at(SyntaxKind::VoiceKw) {
                self.voice_decl();
            } else if self.at(SyntaxKind::MakeKw) {
                self.make_stmt();
            } else {
                self.expected("`clef`, `meter`, `tempo`, `profile`, `voice`, or `make`");
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

    /// `profile name { mark ... dynamic ... }`
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
            if self.at(SyntaxKind::MarkKw) {
                self.rule(SyntaxKind::MarkRule, "a mark name");
            } else if self.at(SyntaxKind::DynamicKw) {
                self.rule(SyntaxKind::DynamicRule, "a dynamic marking");
            } else if self.at(SyntaxKind::GrooveKw) {
                self.rule(SyntaxKind::GrooveRule, "a groove name");
            } else if self.at(SyntaxKind::GraceKw) {
                self.grace_rule();
            } else {
                self.expected("a `mark`, `dynamic`, `groove`, or `grace` rule");
                self.recover(PROFILE_RECOVERY);
            }
        }
        self.finish();
    }

    /// `mark|dynamic|groove <name> { <setting>* }` — one shape, three heads.
    fn rule(&mut self, kind: SyntaxKind, what: &str) {
        self.start(kind);
        self.bump(); // mark | dynamic | groove
        self.expect(SyntaxKind::Identifier, what);
        self.settings_block();
        self.finish();
    }

    /// `grace { <setting>* }` — the same block with no name in front.
    ///
    /// Nameless because a profile has one reading of a grace note, not a
    /// vocabulary of them: `groove` is named because the name chooses the
    /// shape, and there is no such choice here.
    fn grace_rule(&mut self) {
        self.start(SyntaxKind::GraceRule);
        self.bump(); // grace
        self.settings_block();
        self.finish();
    }

    /// `{ <setting>* }` — the body every profile rule shares.
    fn settings_block(&mut self) {
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
    }

    /// `<name> = [-]<number> [unit];` — the unit is the value's, not the
    /// setting's, so `attack = 8 ms;` and `attack = 0.008 s;` both parse.
    ///
    /// A rational is admitted beside a decimal because some settings are
    /// musical rather than numeric: `ratio = 2/3` is a swing, and writing it
    /// `0.667` would put an approximation where §4 wants an exact one. The
    /// sign is a separate token, so `by = -1/64` is a rational with a minus
    /// in front rather than a third number syntax.
    fn setting_stmt(&mut self) {
        self.start(SyntaxKind::SettingStmt);
        self.bump(); // setting name
        self.expect(SyntaxKind::Equals, "`=`");
        if self.at(SyntaxKind::Minus) {
            self.bump();
        }
        // A word is a value too: some settings are a choice between named
        // readings rather than a quantity — `from = principal;`. Which of the
        // two a given setting takes is the reader's answer, not the grammar's,
        // so both parse here and the profile says which it wanted.
        if self.at_any(&[
            SyntaxKind::Float,
            SyntaxKind::Integer,
            SyntaxKind::Rational,
            SyntaxKind::Identifier,
        ]) {
            self.bump();
        } else {
            self.expected("a number or a word");
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
        // `scale` is a processor here and a musical collection everywhere
        // else. The studio vocabulary is deliberately made of identifiers so
        // it can grow without the lexer, and this is the one word
        // the score side also needed; the stage accepts the keyword token so
        // that a signal can still be scaled.
        } else if self.at_any(&[SyntaxKind::Identifier, SyntaxKind::ScaleKw]) {
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
        // A parameter list is what makes this a template's voice rather than
        // a part's; `template` in front of it is what says so out loud.
        if self.at(SyntaxKind::LParen) {
            self.param_list();
        }
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
            // The exit is checked before the dispatch, which is the whole of
            // `| a | b`: the pipe ends the bar it is in, and the voice one
            // level up is what opens the next one.
            if self.in_pipe_bar && !self.continues_a_bar() {
                break;
            }
            if self.at(SyntaxKind::Pipe) {
                self.pipe_bar_stmt();
            } else if self.at_any(&[SyntaxKind::PitchLiteral, SyntaxKind::Identifier, SyntaxKind::LParen]) {
                self.note_stmt();
            } else if self.at(SyntaxKind::RestKw) {
                self.rest_stmt();
            } else if self.at(SyntaxKind::LBracket) {
                self.chord_stmt();
            } else if self.at(SyntaxKind::Semicolon) {
                self.stray_semicolon();
            } else if self.at(SyntaxKind::UseKw) {
                self.use_stmt();
            } else if self.at(SyntaxKind::InKw) {
                self.in_scale_stmt();
            } else if self.at(SyntaxKind::StackKw) {
                self.stack_stmt();
            } else if self.at(SyntaxKind::TransposeKw) {
                self.transpose_stmt();
            } else if self.at(SyntaxKind::RepeatKw) {
                self.repeat_stmt();
            } else if self.at(SyntaxKind::BarKw) {
                self.bar_stmt();
            } else if self.at(SyntaxKind::AssertKw) {
                self.assert_stmt();
            } else if self.at(SyntaxKind::SenzaKw) {
                self.senza_stmt();
            } else if self.at(SyntaxKind::GraceKw) {
                self.grace_stmt();
            } else if self.at(SyntaxKind::MobileKw) {
                self.mobile_stmt();
            } else if self.at(SyntaxKind::ImproviseKw) {
                self.improvise_stmt();
            } else if self.at(SyntaxKind::TempoKw) {
                // The same statements the header and the part write, written
                // where the music reaches them: one kind, one node, two
                // places.
                self.tempo_stmt();
            } else if self.at(SyntaxKind::MeterKw) {
                self.meter_stmt();
            } else if self.at(SyntaxKind::KeyKw) {
                self.key_stmt();
            } else if self.at(SyntaxKind::ClefKw) {
                self.clef_stmt();
            } else if self.at(SyntaxKind::EndingKw) {
                self.ending_stmt();
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
            } else if self.at(SyntaxKind::MarkKw) {
                self.mark_stmt();
            } else if self.at_any(&[SyntaxKind::CrescendoKw, SyntaxKind::DiminuendoKw]) {
                self.hairpin_stmt();
            } else {
                self.expected_with_help(
                    "something to play",
                    "a voice holds notes (`c5/4`), chords (`[c5 e5]/4`), `rest`, `|`, and `use` — run `musa explain syntax` for the rest",
                );
                self.recover(VOICE_RECOVERY);
            }
        }
    }

    /// Whether the statement the parser is on belongs to the bar it is in.
    ///
    /// A bar holds the events of one measure and the marks written among
    /// them. Everything else a voice can write is *at least* a bar long — a
    /// repeat, an ending, a named bar, an unmeasured stretch, an
    /// improvisation — and notation draws those around barlines rather than
    /// inside one, so meeting one closes the bar the way a barline would.
    ///
    /// `meter` and `key` close it too, for the other reason: a time signature
    /// and a key signature are *printed* at a barline, which is why
    /// `examples/modulation.musa` says every change lands on one and why a
    /// meter written mid-bar is already an error.
    ///
    /// A whitelist rather than a blacklist: a statement kind added next year
    /// ends the bar, which is wrong in a way the composer sees, instead of
    /// lengthening it, which is wrong in a way only the bar-length check
    /// notices.
    fn continues_a_bar(&self) -> bool {
        self.current().is_some_and(|kind| {
            matches!(
                kind,
                SyntaxKind::PitchLiteral
                    | SyntaxKind::Identifier
                    | SyntaxKind::LParen
                    | SyntaxKind::RestKw
                    | SyntaxKind::LBracket
                    | SyntaxKind::Semicolon
                    | SyntaxKind::UseKw
                    | SyntaxKind::DynamicKw
                    | SyntaxKind::ClefKw
                    | SyntaxKind::TempoKw
                    | SyntaxKind::MarkKw
                    | SyntaxKind::CrescendoKw
                    | SyntaxKind::DiminuendoKw
                    | SyntaxKind::TupletKw
                    | SyntaxKind::SlurKw
                    | SyntaxKind::GraceKw
            )
        })
    }

    /// `{ ... }` body of a motif, transpose, or repeat.
    fn block(&mut self) {
        // A brace opens a context of its own: a tuplet written inside a `|`
        // bar reads its items as a block, not as more of the bar.
        let enclosing = std::mem::replace(&mut self.in_pipe_bar, false);
        self.block_inner();
        self.in_pipe_bar = enclosing;
    }

    fn block_inner(&mut self) {
        self.start(SyntaxKind::Block);
        self.expect(SyntaxKind::LBrace, "`{`");
        self.voice_items();
        self.expect(SyntaxKind::RBrace, "`}`");
        self.finish();
    }

    /// `<pitch-or-ref> <duration> <articulation>* ~?`
    ///
    /// No terminator: an event is self-delimiting, because a pitch and a
    /// duration are the two things it starts with and nothing else in a voice
    /// starts that way.
    fn note_stmt(&mut self) {
        self.start(SyntaxKind::NoteStmt);
        if self.at(SyntaxKind::LParen)
            || matches!(
                self.nth_significant(1),
                Some(SyntaxKind::UpKw | SyntaxKind::DownKw | SyntaxKind::StepKw)
            )
        {
            self.expr();
        } else {
            self.bump(); // simple pitch literal or pitch reference
        }
        self.duration();
        self.articulations();
        self.tie();
        self.finish();
    }

    /// Zero or more articulations after a duration: the words (`accent
    /// staccato`) and the two marks notation writes (`>` accent, `^`
    /// marcato).
    ///
    /// They live in their own node: a bare identifier in a note statement is
    /// otherwise a pitch or duration parameter reference, and telling the two
    /// apart by counting tokens is exactly the kind of positional rule that
    /// breaks the next time the statement grows a part.
    ///
    /// A note's own duration is read before this runs, so what is left here
    /// is either a word describing the note or the start of the next event.
    fn articulations(&mut self) {
        if !self.at_articulation() {
            return;
        }
        self.start(SyntaxKind::ArticulationList);
        while self.at_articulation() {
            self.bump();
        }
        self.finish();
    }

    /// Whether the parser is on an articulation rather than on the next
    /// event.
    ///
    /// Without a `;` between events, `root 1/8 tenuto d5 1/8` has to be read
    /// the way a player reads it: `tenuto` belongs to the note before it and
    /// `d5` starts the note after. A word that a duration follows is a pitch
    /// — that is what a duration is *for* — so one token of lookahead on a
    /// kind settles it.
    fn at_articulation(&self) -> bool {
        if self.at_any(&[SyntaxKind::Greater, SyntaxKind::Caret]) {
            return true;
        }
        if !self.at(SyntaxKind::Identifier) {
            return false;
        }
        !matches!(
            self.nth_significant(1),
            Some(SyntaxKind::Slash | SyntaxKind::Rational | SyntaxKind::Integer)
        )
    }

    /// The postfix tie mark, tying this statement to the next.
    fn tie(&mut self) {
        if self.at(SyntaxKind::Tilde) {
            self.bump();
        }
    }

    /// `rest <duration>`
    fn rest_stmt(&mut self) {
        self.start(SyntaxKind::RestStmt);
        self.bump(); // rest
        self.duration();
        self.finish();
    }

    /// `[<pitch> ...]<duration>` — `[c3 g3]/2`.
    ///
    /// The bracket says chord, the way it does in ABC and in GUIDO, so the
    /// keyword and the commas were both repeating what it already said.
    fn chord_stmt(&mut self) {
        self.start(SyntaxKind::ChordStmt);
        self.bump(); // [
        self.expect(SyntaxKind::PitchLiteral, "a pitch");
        while self.at(SyntaxKind::PitchLiteral) {
            self.bump();
        }
        self.expect(SyntaxKind::RBracket, "`]`");
        self.duration();
        self.articulations();
        self.tie();
        self.finish();
    }

    /// `stack <pitch> <type><duration>` — a chord sounded in close position.
    ///
    /// The root is written as an absolute pitch because the register is the
    /// whole of what a voicing adds to a chord class. A pitch class is
    /// accepted by the grammar and refused by the compiler, so the mistake is
    /// answered with a sentence about register rather than with `expected a
    /// pitch`.
    fn stack_stmt(&mut self) {
        self.start(SyntaxKind::StackStmt);
        self.bump(); // stack
        if self.at(SyntaxKind::PitchLiteral) {
            self.bump();
        } else {
            // A pitch class is parsed rather than rejected, so the compiler
            // gets to answer with the sentence about register.
            self.pitch_class();
        }
        self.expect(SyntaxKind::Identifier, "a chord type such as `major` or `major7`");
        self.duration();
        self.articulations();
        self.tie();
        self.finish();
    }

    /// `use name(args);` — or `use name;`, when the material takes none.
    fn use_stmt(&mut self) {
        self.start(SyntaxKind::UseStmt);
        self.bump(); // use
        self.expr();
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

    /// `in scale <expr> { ... }` — the enclosed music read in a scale.
    ///
    /// Only `scale` follows `in`. There is no `in key` or `in meter`: those
    /// are structural facts a score states at a place, and a lexical block
    /// that quietly changed one would be a modulation nobody wrote.
    fn in_scale_stmt(&mut self) {
        self.start(SyntaxKind::InScaleStmt);
        self.bump(); // in
        if !self.at(SyntaxKind::ScaleKw) {
            self.expected("`scale` — a scale is the only context entered lexically");
            self.expr();
            self.block();
            self.finish();
            return;
        }
        // `in scale c major { … }` writes the collection out and `in scale s
        // { … }` names one. Two words after `scale` is the literal; anything
        // else is an expression, and the block's `{` is what tells them
        // apart.
        let written_out = matches!(self.nth_significant(1), Some(SyntaxKind::Identifier))
            && matches!(self.nth_significant(2), Some(SyntaxKind::Identifier | SyntaxKind::Hash));
        if written_out {
            self.scale_expr();
        } else {
            self.bump(); // scale
            self.expr();
        }
        self.block();
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
    /// `repeat 4 { … }`, or `repeat 4 to 16 { … }` — a count chosen by the
    /// realization rather than by the composer.
    ///
    /// One statement with two forms rather than two statements: the block, the
    /// recovery, and every consumer are the same, and the only difference is
    /// whether the number is one number or two.
    fn repeat_stmt(&mut self) {
        self.start(SyntaxKind::RepeatStmt);
        self.bump(); // repeat
        self.expect(SyntaxKind::Integer, "a repeat count");
        if self.at(SyntaxKind::ToKw) {
            self.bump();
            self.expect(SyntaxKind::Integer, "the largest number of passes");
        }
        self.block();
        self.finish();
    }

    /// `ending 1 { ... }`
    ///
    /// Parsed wherever a note is, though it only means something inside a
    /// `repeat`. Where it belongs is a question about the music, and the
    /// compiler answers it with a sentence; a syntax error here would only be
    /// able to say the grammar disagreed.
    fn ending_stmt(&mut self) {
        self.start(SyntaxKind::EndingStmt);
        self.bump(); // ending
        self.expect(SyntaxKind::Integer, "which pass this is");
        self.block();
        self.finish();
    }

    /// `bar { ... }` / `bar head { ... }`
    fn bar_stmt(&mut self) {
        if let Some(error) = self.nested_bar() {
            self.errors.push(error);
        }
        // The blank line a composer leaves between two phrases belongs to the
        // voice, not to the bar under it: a bar is written on one line, and a
        // formatter that never descends into it would never see the trivia.
        self.eat_trivia();
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

    /// `assert pitches_in(scale c major) { ... }`
    ///
    /// The parentheses are written even when the claim takes no arguments, so
    /// that `assert fills_meter()` and a name the composer misremembered are
    /// told apart by the grammar rather than by a guess. Which names exist,
    /// and what each one's arguments are, is the compiler's registry and not
    /// the parser's business — a claim nobody has heard of parses, and then
    /// gets a sentence naming the ones that do.
    ///
    /// Assertions nest: two claims about one passage are two assertions, and
    /// writing them one inside the other is how a composer says both.
    fn assert_stmt(&mut self) {
        self.start(SyntaxKind::AssertStmt);
        self.bump(); // assert
        self.expect(SyntaxKind::Identifier, "what is being claimed");
        if self.at(SyntaxKind::LParen) {
            self.expr_arg_list();
        } else {
            self.expected("`(` — a claim is written with its arguments, and one with none is written `()`");
        }
        self.block();
        self.finish();
    }

    /// `| <items…>` — a bar drawn the way notation draws one.
    ///
    /// The same node as `bar { … }`, because it is the same claim: one
    /// measure's worth of music, checked against the meter. What it does not
    /// have is a name, and it does not have one *structurally* —
    /// [`BarStmt::name`](crate::ast::BarStmt::name) reads direct identifier
    /// tokens, and a pipe bar's direct tokens are the pipe and trivia.
    fn pipe_bar_stmt(&mut self) {
        if let Some(error) = self.nested_bar() {
            self.errors.push(error);
        }
        // The blank line a composer leaves between two phrases belongs to the
        // voice, not to the bar under it: a bar is written on one line, and a
        // formatter that never descends into it would never see the trivia.
        self.eat_trivia();
        self.start(SyntaxKind::BarStmt);
        self.bump(); // |
        self.bar_depth = self.bar_depth.saturating_add(1);
        let enclosing = std::mem::replace(&mut self.in_pipe_bar, true);
        self.voice_items();
        self.in_pipe_bar = enclosing;
        self.bar_depth = self.bar_depth.saturating_sub(1);
        self.finish();
    }

    /// The `;` a reader who learned the old syntax will type after a note.
    ///
    /// Worth its own arm rather than the generic complaint: the fix is a
    /// deletion, so every file written before today converts itself one
    /// keystroke at a time.
    fn stray_semicolon(&mut self) {
        let range = self
            .significant()
            .map_or_else(|| TextRange::empty(self.end_size()), |token| token.range);
        if !self.cascading() {
            self.errors.push(
                SyntaxError::new(range, "a note does not end in `;`", "delete this")
                    .with_help("a note, a rest and a chord end themselves; every other statement ends with `;` or `}`")
                    .with_fix("remove `;`", ""),
            );
        }
        self.bump();
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

    /// `mark <name> [<argument>] ;` or `mark <name> [<argument>] { ... }` —
    /// a notation mark that is not written on a note.
    ///
    /// The parser does not know which marks exist, which take an argument, or
    /// which want a block: it accepts the shape and the compiler checks it
    /// against the vocabulary. That is the point of the table — a new mark is
    /// a row, and this function never changes.
    fn mark_stmt(&mut self) {
        self.start(SyntaxKind::MarkStmt);
        self.bump(); // mark
        self.expect(SyntaxKind::Identifier, "a mark such as `breath` or `pedal`");
        if self.at_any(&[SyntaxKind::String, SyntaxKind::Integer, SyntaxKind::Minus]) {
            if self.at(SyntaxKind::Minus) {
                self.bump();
                self.expect(SyntaxKind::Integer, "a whole number");
            } else {
                self.bump();
            }
        }
        if self.at(SyntaxKind::LBrace) {
            self.block();
        } else {
            self.expect(SyntaxKind::Semicolon, "`;` or a block");
        }
        self.finish();
    }

    /// `grace { c5 d5 }` — the notes crushed before the one that follows.
    ///
    /// The block holds pitches and nothing else: a grace note has no written
    /// duration, which is the one shape a `NoteStmt` cannot hold, so it gets a
    /// node of its own rather than a note with an optional duration. Making
    /// the duration optional on every note would let `c5` be written anywhere
    /// and mean nothing.
    ///
    /// A grace note delimits itself the way every other event does: a pitch
    /// and the marks written on it are one word, and the `}` ends the last
    /// one. What separates two grace notes is what separates two notes —
    /// nothing but space.
    fn grace_stmt(&mut self) {
        // The comment above a grace group belongs to the voice, not to the
        // group: the group is written on one line, and a formatter that never
        // descends into it would never see the trivia.
        self.eat_trivia();
        self.start(SyntaxKind::GraceStmt);
        self.bump(); // grace
        if self.at(SyntaxKind::LBrace) {
            self.bump();
            while !self.at(SyntaxKind::RBrace) && self.current().is_some() {
                if self.at_any(&[SyntaxKind::PitchLiteral, SyntaxKind::Identifier]) {
                    self.start(SyntaxKind::GraceNote);
                    self.bump(); // pitch literal or pitch reference
                    self.articulations();
                    self.finish();
                } else if self.at(SyntaxKind::Semicolon) {
                    self.stray_semicolon();
                } else {
                    self.expected("a pitch");
                    self.recover(&[SyntaxKind::Semicolon, SyntaxKind::RBrace, SyntaxKind::PitchLiteral]);
                }
            }
            self.expect(SyntaxKind::RBrace, "`}`");
        } else {
            self.expected("a block of pitches, like `grace { c5 d5 }`");
        }
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
            // `f#m7` is a chord the way `fmaj7` is, but a sharp is its own
            // token, so the root's accidental and the quality after it come
            // in separately. The node is what makes them one word again.
            while self.at(SyntaxKind::Hash) {
                self.bump();
            }
            if self.at(SyntaxKind::Identifier) {
                self.bump(); // the quality, after a sharp split it off
            }
            if self.at(SyntaxKind::Integer) {
                self.bump();
            }
        } else {
            self.expected("a chord such as `am` or `fmaj7`");
        }
        self.finish();
    }

    /// A pitch class with no octave: `a`, `g#`, `bb`.
    ///
    /// A flat is part of the identifier and a sharp is a token of its own, so
    /// this is one token or three. Wrapping it settles that once, here, and
    /// every reader downstream asks the node for its text.
    fn pitch_class(&mut self) {
        if !self.at(SyntaxKind::Identifier) {
            self.expected("a pitch class such as `a` or `g#`");
            return;
        }
        self.start(SyntaxKind::PitchClass);
        self.bump();
        while self.at(SyntaxKind::Hash) {
            self.bump();
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

    /// A duration: `1`, `1/2`, `3/8`, `1/12`, `/4`, `/4.`, …
    ///
    /// It gets a node of its own because `c4/4` puts a bare `4` inside a note
    /// statement, and every reader that finds a duration by taking the first
    /// numeral it sees would read that as a whole note — silently, and on the
    /// path of every note in the language. With a node, a reader that looks in
    /// the wrong place finds nothing instead of finding the wrong thing.
    fn duration(&mut self) {
        self.start(SyntaxKind::Duration);
        self.duration_value("a duration");
        // `g4/4 to 2/1` — written as a quarter, held as long as the
        // performer likes up to a double whole. The first value is the
        // notated one and the second bounds the performed one (roadmap §2).
        if self.at(SyntaxKind::ToKw) {
            self.bump();
            self.duration_value("the longest the note may be held");
        }
        self.finish();
    }

    /// One duration value, in either spelling.
    ///
    /// `/N` says the same thing as `1/N` in one character less than the
    /// pitch beside it, and augmentation dots multiply it by `2 − 2⁻ᵈ`:
    /// `/4.` is 3/8 and `/4..` is 7/16.
    fn duration_value(&mut self, what: &str) {
        if self.at(SyntaxKind::Slash) {
            self.bump(); // `/`
            if self.at(SyntaxKind::Integer) {
                self.bump();
            } else {
                self.expected("a note value such as `4` or `8`");
            }
            while self.at(SyntaxKind::Dot) {
                self.bump();
            }
            return;
        }
        if self.at_any(&[SyntaxKind::Rational, SyntaxKind::Integer, SyntaxKind::Identifier]) {
            self.bump(); // literal or duration-parameter reference
            self.no_dots_on_the_long_form();
            return;
        }
        self.expected(what);
    }

    /// A dot after `3/8` is refused rather than read as 9/16: the long form
    /// already writes 9/16, so two spellings of one duration would be one
    /// spelling too many.
    fn no_dots_on_the_long_form(&mut self) {
        if !self.at(SyntaxKind::Dot) || self.cascading() {
            return;
        }
        if let Some(token) = self.significant() {
            let at = token.range;
            self.errors.push(
                SyntaxError::new(
                    at,
                    "an augmentation dot needs the short form",
                    "this dot has nothing to dot",
                )
                .with_help("`/8.` is a dotted eighth; written as a fraction it is `3/16`"),
            );
        }
        while self.at(SyntaxKind::Dot) {
            self.bump();
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
