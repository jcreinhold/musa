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
        .map(|error| SyntaxError::new(error.range(), error.kind().to_string()))
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
    SyntaxKind::TempoKw,
    SyntaxKind::MeterKw,
    SyntaxKind::KeyKw,
    SyntaxKind::MotifKw,
    SyntaxKind::ScoreKw,
    SyntaxKind::PerformanceKw,
];
const SCORE_RECOVERY: &[SyntaxKind] = &[SyntaxKind::RBrace, SyntaxKind::PartKw];
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
const VOICE_RECOVERY: &[SyntaxKind] = &[
    SyntaxKind::Semicolon,
    SyntaxKind::RBrace,
    SyntaxKind::PitchLiteral,
    SyntaxKind::RestKw,
    SyntaxKind::ChordKw,
    SyntaxKind::UseKw,
    SyntaxKind::TransposeKw,
    SyntaxKind::RepeatKw,
    SyntaxKind::SlurKw,
    SyntaxKind::DynamicKw,
    SyntaxKind::TupletKw,
];

struct Parser<'a> {
    source: &'a str,
    tokens: &'a [Token],
    pos: usize,
    events: Vec<Event<'a>>,
    errors: Vec<SyntaxError>,
}

impl<'a> Parser<'a> {
    fn new(source: &'a str, lexed: &'a Lexed) -> Self {
        Self {
            source,
            tokens: lexed.tokens(),
            pos: 0,
            events: Vec::new(),
            errors: Vec::new(),
        }
    }

    /// Parse the whole document and build the tree.
    fn run(mut self) -> (SyntaxNode, Vec<SyntaxError>) {
        self.start(SyntaxKind::Root);
        self.piece_decl();
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

    /// Consume `kind` if present; otherwise record an error and continue.
    fn expect(&mut self, kind: SyntaxKind, what: &str) {
        if self.at(kind) {
            self.bump();
        } else {
            self.error_here(format!("expected {what}"));
        }
    }

    fn error_here(&mut self, message: impl Into<String>) {
        let range = self
            .tokens
            .get(self.pos)
            .map_or_else(|| TextRange::empty(self.end_size()), |token| token.range);
        self.errors.push(SyntaxError::new(range, message));
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
                self.error_here("unclosed `piece` block");
                break;
            }
            if self.at(SyntaxKind::TempoKw) {
                self.tempo_stmt();
            } else if self.at(SyntaxKind::MeterKw) {
                self.meter_stmt();
            } else if self.at(SyntaxKind::KeyKw) {
                self.key_stmt();
            } else if self.at(SyntaxKind::MotifKw) {
                self.motif_decl();
            } else if self.at(SyntaxKind::ScoreKw) {
                self.score_decl();
            } else if self.at(SyntaxKind::PerformanceKw) {
                self.performance_decl();
            } else {
                self.error_here("expected a tempo, meter, key, motif, score, or performance declaration");
                self.recover(PIECE_RECOVERY);
            }
        }
        self.finish();
    }

    /// `tempo <beat> = <bpm>;` — beat is a name (`quarter`) or fraction.
    fn tempo_stmt(&mut self) {
        self.start(SyntaxKind::TempoStmt);
        self.bump(); // tempo
        if self.at_any(&[SyntaxKind::Identifier, SyntaxKind::Rational]) {
            self.bump();
        } else {
            self.error_here("expected a beat unit (`quarter` or `1/4`)");
        }
        self.expect(SyntaxKind::Equals, "`=`");
        self.expect(SyntaxKind::Integer, "a tempo in bpm");
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
                self.error_here("expected a parameter type (`pitch`)");
            }
            if self.at(SyntaxKind::Equals) {
                self.bump();
                if self.at_any(&[SyntaxKind::PitchLiteral, SyntaxKind::Rational, SyntaxKind::Integer]) {
                    self.bump(); // default value (pitch or duration)
                } else {
                    self.error_here("expected a default value");
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
                self.error_here("unclosed `score` block");
                break;
            }
            if self.at(SyntaxKind::PartKw) {
                self.part_decl();
            } else {
                self.error_here("expected a `part` declaration");
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
                self.error_here("unclosed `part` block");
                break;
            }
            if self.at(SyntaxKind::ClefKw) {
                self.clef_stmt();
            } else if self.at(SyntaxKind::ProfileKw) {
                self.profile_stmt();
            } else if self.at(SyntaxKind::VoiceKw) {
                self.voice_decl();
            } else {
                self.error_here("expected `clef`, `profile`, or `voice`");
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
                self.error_here("unclosed `performance` block");
                break;
            }
            if self.at(SyntaxKind::ProfileKw) {
                self.profile_decl();
            } else {
                self.error_here("expected a `profile` declaration");
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
                self.error_here("unclosed `profile` block");
                break;
            }
            if self.at(SyntaxKind::ArticulationKw) {
                self.rule(SyntaxKind::ArticulationRule, "an articulation name");
            } else if self.at(SyntaxKind::DynamicKw) {
                self.rule(SyntaxKind::DynamicRule, "a dynamic marking");
            } else {
                self.error_here("expected an `articulation` or `dynamic` rule");
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
                self.error_here("unclosed rule block");
                break;
            }
            if self.at(SyntaxKind::Identifier) {
                self.setting_stmt();
            } else {
                self.error_here("expected a setting such as `gate = 0.55;`");
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
            self.error_here("expected a number");
        }
        if self.at_any(&[SyntaxKind::UnitMs, SyntaxKind::UnitS]) {
            self.bump();
        }
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
            } else if self.at(SyntaxKind::SlurKw) {
                self.slur_stmt();
            } else if self.at(SyntaxKind::DynamicKw) {
                self.dynamic_stmt();
            } else if self.at(SyntaxKind::TupletKw) {
                self.tuplet_stmt();
            } else {
                self.error_here("expected a note, rest, chord, use, transpose, repeat, slur, dynamic, or tuplet");
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

    /// `use name(args);`
    fn use_stmt(&mut self) {
        self.start(SyntaxKind::UseStmt);
        self.bump(); // use
        self.expect(SyntaxKind::Identifier, "a motif name");
        self.expect(SyntaxKind::LParen, "`(`");
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
                    self.error_here("expected an argument");
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
            self.error_here("expected `up` or `down`");
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

    /// `slur { ... }`
    fn slur_stmt(&mut self) {
        self.start(SyntaxKind::SlurStmt);
        self.bump(); // slur
        self.block();
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
            self.error_here("expected a duration");
        }
    }
}
