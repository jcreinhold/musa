//! The Parser token cursor and event machinery: walking the lexed token
//! stream, emitting start/token/finish events, and the shared helper
//! vocabulary every grammar module parses with.
//!
//! [`Parser`] is the one mutable state in the parser: the lexed tokens, the
//! position in them, the events that build the tree, and the flags that carry
//! the few pieces of grammar state a syntax needs to know about (bar nesting,
//! quote depth, whether `with` is spoken for). Everything the grammar modules
//! do—start a node, expect a token, recover—goes through these methods so the
//! modules below are statements about *what* is parsed, not *how*.

use crate::language::SyntaxNode;
use crate::{Lexed, SyntaxError, SyntaxKind, Token};
use rowan::GreenNodeBuilder;
use text_size::{TextRange, TextSize};

/// What a file's lexical root turned out to be, as far as the parser can tell.
///
/// Whether the file still owes a piece: a file that declares a module tree is a
/// package's own bookkeeping and is not music at all.
#[derive(Clone, Copy, Default)]
pub(super) struct RootShape {
    pub(super) declares_modules: bool,
}

/// One step of tree construction.
pub(super) enum Event<'a> {
    StartNode(SyntaxKind),
    Token(SyntaxKind, &'a str),
    FinishNode,
}

/// Which grammar the written-pitch operators draw their operands from.
///
/// `01-surface.md` §1 puts `step`, `up`, and `down` above arithmetic, so
/// `c4 up M3 + P5` transposes by the sum of two intervals. A music statement
/// writes a *duration* after its pitch, where `/4` is that duration and `-`
/// opens a negative rational, so a note's pitch takes its operands from level
/// 1 and says arithmetic with parentheses.
#[derive(Clone, Copy)]
pub(super) enum Operand {
    /// Level 3 — the expression grammar.
    Arithmetic,
    /// Level 1 — a note statement's pitch.
    Written,
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

/// The cursor the whole grammar parses with.
///
/// Fields are `pub(super)` because the grammar modules are sibling files under
/// `parser`, each splitting their own `impl Parser` across them.
pub(super) struct Parser<'a> {
    pub(super) source: &'a str,
    pub(super) tokens: &'a [Token],
    pub(super) pos: usize,
    pub(super) events: Vec<Event<'a>>,
    pub(super) errors: Vec<SyntaxError>,
    /// Whether the end of the file has already been blamed for something.
    ///
    /// One missing `}` closes every block above it, so a file that ends early
    /// produces one complaint per open block — all at the same character, all
    /// saying the same thing. The innermost is reported first, because that is
    /// the order the parser unwinds in, and it is also the one closest to what
    /// was actually being written.
    pub(super) blamed_the_end: bool,
    /// How many `bar` bodies enclose the position being parsed.
    ///
    /// Bars do not nest, and the parser is where that is said: a bar claims to
    /// be one measure, and a measure inside a measure is not a thing the
    /// notation has a mark for.
    pub(super) bar_depth: u32,
    /// Whether the items being parsed are inside a `|` bar.
    ///
    /// A `|` both closes the bar it stands after and opens the one it stands
    /// before, so the same token means "stop" one level down and "start" one
    /// level up. This flag is which of the two the parser is looking at.
    pub(super) in_pipe_bar: bool,
    /// Whether a `with` standing after the expression being parsed belongs to
    /// something else.
    ///
    /// The word is spoken for twice. `use theme() with { note 3 = a5; }`
    /// specializes one occurrence of some material, and `p with { dots = d }`
    /// rebuilds a record; both put `with {` directly after an expression, so
    /// the expression parser cannot tell them apart by looking. The statement
    /// is the one that owns its `with`, so `use_stmt` sets this while reading
    /// its own expression and the record form stands down. Nothing is lost:
    /// `use` takes music, and a record is not music, so the suppressed reading
    /// was never a program. Parentheses restore it if one is ever wanted.
    pub(super) with_is_spoken_for: bool,
    /// How many quote bodies enclose the position being parsed.
    ///
    /// `$` is part of a quote's grammar and has no meaning outside one
    /// (`docs/rules/language/11-quotation.md` §2), so this is what decides
    /// whether the character opens a splice or is a stray token. It counts
    /// rather than flags because a quote may nest inside a splice, and it
    /// drops back to zero while a splice's own expression is read — the
    /// expression is host code, and a `$` in it belongs to whatever quote is
    /// written *there*.
    pub(super) quote_depth: u32,
}

impl<'a> Parser<'a> {
    pub(super) fn new(source: &'a str, lexed: &'a Lexed) -> Self {
        Self {
            source,
            tokens: lexed.tokens(),
            pos: 0,
            events: Vec::new(),
            errors: Vec::new(),
            blamed_the_end: false,
            bar_depth: 0,
            in_pipe_bar: false,
            with_is_spoken_for: false,
            quote_depth: 0,
        }
    }

    /// Parse the whole document and build the tree.
    pub(super) fn run(mut self) -> (SyntaxNode, Vec<SyntaxError>) {
        self.start(SyntaxKind::Root);
        // A file is a piece, a library, or a module file
        // (`docs/rules/language/01-surface.md` §1). Which one it is is written
        // at the top of it rather than inferred from what it happens to
        // contain: a library with a `score` in it is then a parse error rather
        // than a rule someone has to remember.
        //
        // What may precede a piece or a library is the file's lexical root:
        // imports, values, functions, and type declarations. One file is one
        // piece. A module file is `mod …;` and nothing else, and the
        // elaborator holds it to that — the parser reads the same preamble
        // either way, so what is written beside the `mod`s is a question about
        // meaning rather than about shape.
        let shape = self.root_preamble();
        if self.at(SyntaxKind::LibraryKw) {
            self.library_decl();
        } else if self.at(SyntaxKind::PieceKw) || !shape.declares_modules {
            // A file that declares no module tree still owes a piece, and
            // saying so here is how `piece_decl` reports the one it cannot
            // find.
            self.piece_decl();
        }
        self.eat_trivia();
        self.finish();
        let node = SyntaxNode::new_root(self.build_tree());
        (node, self.errors)
    }

    pub(super) fn start(&mut self, kind: SyntaxKind) {
        self.events.push(Event::StartNode(kind));
    }

    /// Wrap everything emitted since `checkpoint` in a new parent node.
    pub(super) fn start_at(&mut self, checkpoint: usize, kind: SyntaxKind) {
        self.events.insert(checkpoint, Event::StartNode(kind));
    }

    pub(super) fn finish(&mut self) {
        self.events.push(Event::FinishNode);
    }

    pub(super) fn eat_trivia(&mut self) {
        while let Some(token) = self.tokens.get(self.pos) {
            if token.kind.is_trivia() {
                self.emit_token(token);
                self.advance();
            } else {
                break;
            }
        }
    }

    /// Write one lexer token into the tree.
    ///
    /// The one funnel every significant token and every piece of trivia goes
    /// through, which is why the three composite literals are split *here*
    /// rather than at each site that bumps one. `self.at(PitchLiteral)` asks
    /// about the lexer's token and the lexer is unchanged, so every site that
    /// tests a literal is untouched and every site that writes one gets the
    /// parts without saying so.
    ///
    /// A composite literal becomes a node of its own kind whose children are
    /// its parts, each a token with its own range. The node's text is the
    /// concatenation of its children — [`super::literals::parts`] partitions
    /// the spelling — so the tree stays lossless and every printed byte is
    /// unchanged.
    pub(super) fn emit_token(&mut self, token: &Token) {
        let start = usize::from(token.range.start());
        let end = usize::from(token.range.end());
        let Some(text) = self.source.get(start..end) else {
            return;
        };
        let Some(parts) = super::literals::parts(token.kind, text) else {
            self.events.push(Event::Token(token.kind, text));
            return;
        };
        self.events.push(Event::StartNode(token.kind));
        for (kind, piece) in parts {
            self.events.push(Event::Token(kind, piece));
        }
        self.events.push(Event::FinishNode);
    }

    pub(super) fn advance(&mut self) {
        self.pos = self.pos.saturating_add(1);
    }

    /// Consume the next significant token (emitting any trivia before it).
    pub(super) fn bump(&mut self) {
        self.eat_trivia();
        if let Some(token) = self.tokens.get(self.pos) {
            let token = *token;
            self.emit_token(&token);
            self.advance();
        }
    }

    /// Kind of the next significant token, or `None` at end of input.
    pub(super) fn current(&self) -> Option<SyntaxKind> {
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

    pub(super) fn at(&self, kind: SyntaxKind) -> bool {
        self.current() == Some(kind)
    }

    pub(super) fn at_any(&self, kinds: &[SyntaxKind]) -> bool {
        self.current().is_some_and(|kind| kinds.contains(&kind))
    }

    /// The word that opens the declaration standing here, looking past a
    /// `private`.
    ///
    /// `private` is a marker on a declaration and never a declaration itself,
    /// so every dispatch that admits one asks this rather than [`Self::at`].
    /// One token of lookahead is the whole of it — there is no second modifier
    /// for the two to be written in either order.
    pub(super) fn opener(&self) -> Option<SyntaxKind> {
        match self.current() {
            Some(SyntaxKind::PrivateKw) => self.nth_significant(1),
            other => other,
        }
    }

    /// Whether `kind` opens the declaration standing here, `private` or not.
    pub(super) fn opens(&self, kind: SyntaxKind) -> bool {
        self.opener() == Some(kind)
    }

    pub(super) fn opens_any(&self, kinds: &[SyntaxKind]) -> bool {
        self.opener().is_some_and(|kind| kinds.contains(&kind))
    }

    /// Consume a `private` marker if the declaration being parsed carries one.
    ///
    /// Called after the declaration's node is started, so the marker is a token
    /// *inside* it: `LetDecl`, `EnumCase`, and the rest keep every accessor
    /// they had, and asking whether one is private is one child lookup rather
    /// than a wrapper node every reader would have to see through.
    pub(super) fn visibility(&mut self) {
        if self.at(SyntaxKind::PrivateKw) {
            self.bump();
        }
    }

    /// The kind `n` significant tokens ahead (`0` is [`Self::current`]).
    ///
    /// The studio grammar is the only place that needs lookahead: `name =`,
    /// `name(`, and a bare `name` are three different productions that share
    /// a first token, and distinguishing them by backtracking would cost the
    /// tree its losslessness.
    pub(super) fn nth_significant(&self, n: usize) -> Option<SyntaxKind> {
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
    pub(super) fn expect(&mut self, kind: SyntaxKind, what: &str) {
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
    pub(super) fn previous_end(&self) -> TextSize {
        self.tokens
            .get(..self.pos)
            .unwrap_or_default()
            .iter()
            .rev()
            .find(|token| !token.kind.is_trivia())
            .map_or_else(|| TextSize::from(0), |token| token.range.end())
    }

    /// Record "expected X, found Y" at the current token without consuming.
    pub(super) fn expected(&mut self, what: &str) {
        if self.cascading() {
            return;
        }
        let error = self.expected_error(what);
        self.errors.push(error);
    }

    /// The same, plus a line saying what to do.
    pub(super) fn expected_with_help(&mut self, what: &str, help: &str) {
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
    pub(super) fn cascading(&self) -> bool {
        self.significant().is_some_and(|token| token.kind == SyntaxKind::Error)
    }

    /// The shared body of the three above.
    ///
    /// Naming what was actually found is most of what makes a parse error
    /// legible: `expected ';', found '}'` locates the mistake a line earlier
    /// than `expected ';'` does, because the reader can see which construct
    /// ran off its end.
    pub(super) fn expected_error(&self, what: &str) -> SyntaxError {
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
    pub(super) fn unclosed(&mut self, what: &str) -> Option<SyntaxError> {
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

    pub(super) fn unclosed_list(&mut self) -> Option<SyntaxError> {
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
    pub(super) fn may_blame_the_end(&mut self) -> bool {
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
    pub(super) fn significant(&self) -> Option<&Token> {
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
    pub(super) fn word(&self) -> Option<&'a str> {
        let token = self.significant()?;
        self.source
            .get(usize::from(token.range.start())..usize::from(token.range.end()))
    }

    pub(super) fn at_word(&self, word: &str) -> bool {
        self.at(SyntaxKind::Identifier) && self.word() == Some(word)
    }

    /// Whether the cursor is on `some` or `none` — the constructors as they
    /// were spelled before they took their type's capital.
    pub(super) fn at_constructor(&self) -> bool {
        self.at_word("some") || self.at_word("none")
    }

    pub(super) fn end_size(&self) -> TextSize {
        TextSize::from(u32::try_from(self.source.len()).unwrap_or(u32::MAX))
    }

    /// Wrap tokens in an `ERROR` node until a recovery point. A recovery
    /// semicolon is consumed (it terminated the broken construct); anything
    /// else is left for the enclosing parse.
    pub(super) fn recover(&mut self, recovery: &[SyntaxKind]) {
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

    pub(super) fn build_tree(&self) -> rowan::GreenNode {
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
}
