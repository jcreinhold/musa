//! The expression grammar: precedence climbs, atoms, records, blocks, matches, and call arguments.

use super::MODULE_NAME;
use super::engine::{Operand, Parser};
use crate::{SyntaxError, SyntaxKind};

/// `01-surface.md` §1's level 6 — non-associative, and `>` is not among them.
const COMPARISON_OPERATORS: &[SyntaxKind] = &[SyntaxKind::EqualsEquals, SyntaxKind::Less];
/// Level 3.
const ADDITIVE_OPERATORS: &[SyntaxKind] = &[SyntaxKind::Plus, SyntaxKind::Minus];
/// Level 2.
const MULTIPLICATIVE_OPERATORS: &[SyntaxKind] = &[SyntaxKind::Star, SyntaxKind::Slash];

impl Parser<'_> {
    /// An ordinary expression: `01-surface.md` §1's six levels of binding,
    /// loosest first.
    ///
    /// The table is fixed and closed. There is no user-defined symbol and no
    /// precedence declaration, because a table an import can extend is a table
    /// that makes a program's *parse* depend on what it imported.
    pub(super) fn expr(&mut self) {
        // Taken rather than read: the suppression is about *this* expression's
        // trailing `with`, and everything nested inside it — a call's
        // arguments, a parenthesized subexpression — is an ordinary place
        // where a record update is exactly what `with` means. Every operand of
        // this expression sees it, and only the rightmost can be followed by a
        // `with` at all.
        let spoken_for = std::mem::take(&mut self.with_is_spoken_for);
        if self.at(SyntaxKind::LetKw) {
            self.let_expr(spoken_for);
        } else {
            self.comparison_expr(spoken_for);
        }
    }

    /// `let name = value; body` — a local binding, which is one expression.
    ///
    /// Read here rather than in [`Self::block_expr`] because a body is not
    /// always a block: a match arm's result and a `quote at here { … }`'s
    /// interior are both [`Self::expr`], and a binding is worth exactly as
    /// much in those places as between braces. Nothing about the block
    /// changes — it still holds one expression, and a `let` is one, so
    /// `{ e1; e2 }` is still the error [`Self::second_expression_in_a_block`]
    /// names (`docs/rules/language/01-surface.md` §1).
    ///
    /// Several bindings are several of these: the body is another
    /// [`Self::expr`], so `let a = …; let b = …; e` nests rightward with no
    /// list of bindings and no scope table. The suppression a trailing `with`
    /// is subject to belongs to the body and not to the value, because the
    /// value ends at its `;` and only the body can be followed by one.
    fn let_expr(&mut self, spoken_for: bool) {
        self.start(SyntaxKind::LetExpr);
        self.bump(); // let
        self.expect(SyntaxKind::Identifier, "a binding name");
        if self.at(SyntaxKind::Colon) {
            self.bump();
            self.type_expr();
        }
        self.expect(SyntaxKind::Equals, "`=`");
        self.expr();
        self.expect(SyntaxKind::Semicolon, "`;`");
        self.with_is_spoken_for = spoken_for;
        self.expr();
        self.finish();
    }

    /// Level 6 — `==` and `<`, non-associative.
    ///
    /// A chain is rejected rather than given a reading: `a == b == c` in a
    /// language whose `==` answers `Bool` would otherwise compare a boolean
    /// with `c`. `>` is not here — `10-traits.md` §5 gives `Ord` one method,
    /// and `>` after a note is the accent mark.
    pub(super) fn comparison_expr(&mut self, spoken_for: bool) {
        let checkpoint = self.events.len();
        self.pitch_expr(spoken_for, Operand::Arithmetic);
        if self.at_any(COMPARISON_OPERATORS) {
            self.start_at(checkpoint, SyntaxKind::BinaryExpr);
            self.bump();
            self.pitch_expr(spoken_for, Operand::Arithmetic);
            self.finish();
        }
    }

    /// Level 3 — `+` and `-`, left-associative.
    pub(super) fn additive_expr(&mut self, spoken_for: bool) {
        let checkpoint = self.events.len();
        self.multiplicative_expr(spoken_for);
        while self.at_any(ADDITIVE_OPERATORS) {
            self.start_at(checkpoint, SyntaxKind::BinaryExpr);
            self.bump();
            self.multiplicative_expr(spoken_for);
            self.finish();
        }
    }

    /// Level 2 — `*` and `/`, left-associative.
    pub(super) fn multiplicative_expr(&mut self, spoken_for: bool) {
        let checkpoint = self.events.len();
        self.operand_expr(spoken_for);
        while self.at_any(MULTIPLICATIVE_OPERATORS) {
            self.start_at(checkpoint, SyntaxKind::BinaryExpr);
            self.bump();
            self.operand_expr(spoken_for);
            self.finish();
        }
    }

    /// Level 1 — an atom and the postfixes that bind directly to it.
    pub(super) fn operand_expr(&mut self, spoken_for: bool) {
        let checkpoint = self.events.len();
        self.expr_atom();
        // One loop for all five postfixes, so they compose in the order they
        // are written and a reader never has to know which of them binds
        // tighter: `read(here)?` asks its question of the call's answer, and
        // `later? with { dots = more }` updates the payload the question
        // yielded. `p with { f = e } with { g = h }` is likewise two updates,
        // the second of the first's result, and `xs[i].m(y)` is a method call
        // on the element.
        loop {
            if self.at(SyntaxKind::LParen) {
                self.start_at(checkpoint, SyntaxKind::ApplyExpr);
                self.expr_arg_list();
                self.finish();
            } else if self.at_method_call() {
                self.start_at(checkpoint, SyntaxKind::MethodCallExpr);
                self.bump(); // `.`
                self.bump(); // the method's name
                self.expr_arg_list();
                self.finish();
            } else if self.at(SyntaxKind::LBracket) {
                self.start_at(checkpoint, SyntaxKind::IndexExpr);
                self.bump(); // `[`
                self.expr();
                self.expect(SyntaxKind::RBracket, "`]`");
                self.finish();
            } else if self.at(SyntaxKind::WithKw) && !spoken_for {
                self.start_at(checkpoint, SyntaxKind::RecordUpdateExpr);
                self.field_update_list();
                self.finish();
            } else if self.at(SyntaxKind::Question) {
                self.start_at(checkpoint, SyntaxKind::QuestionExpr);
                self.bump();
                self.finish();
            } else {
                break;
            }
        }
    }

    pub(super) fn expr_atom(&mut self) {
        match self.current() {
            Some(SyntaxKind::SyntaxKw) => self.syntax_region(),
            Some(SyntaxKind::Identifier) if self.at_constructor() => self.option_expr(),
            Some(
                SyntaxKind::Identifier
                | SyntaxKind::RepeatKw
                | SyntaxKind::TransposeKw
                | SyntaxKind::StretchKw
                | SyntaxKind::RetrogradeKw
                | SyntaxKind::InvertKw,
            ) => self.name_or_record_literal(),
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
            Some(SyntaxKind::FnKw) => self.lambda_expr(),
            Some(SyntaxKind::MatchKw) => self.match_expr(),
            Some(SyntaxKind::IfKw) => self.if_expr(),
            Some(SyntaxKind::MusicKw) => self.music_expr(),
            Some(SyntaxKind::EventsKw) => self.events_quote(),
            Some(SyntaxKind::QuoteKw) => self.quote_expr(),
            Some(SyntaxKind::Dollar) if self.quote_depth > 0 => self.splice(),
            Some(SyntaxKind::ScaleKw) => self.scale_expr(),
            Some(SyntaxKind::KeyKw) => self.key_expr(),
            Some(SyntaxKind::ChordKw) => self.chord_expr(),
            _ => self.expected("an expression"),
        }
    }

    /// A written name, a `::` path, or a record literal headed by either.
    ///
    /// The three are one function because they are one prefix: nothing decides
    /// between them until the tokens after the name have been read, and reading
    /// them here means the head is written down once.
    pub(super) fn name_or_record_literal(&mut self) {
        let checkpoint = self.events.len();
        self.bump(); // the name
        if self.at_path_separator() {
            // `Tying::Untied`, `std::tonal::TokenKind::PitchLiteral`. The
            // parser counts no segments: §1.5's capitalization rule decides
            // where the module prefix ends, and that is a question about what
            // the names denote.
            while self.at_path_separator() {
                self.bump();
                self.bump();
                if self.at_any(MODULE_NAME) {
                    self.bump();
                } else {
                    self.expected("a name in the type's namespace");
                    break;
                }
            }
            self.start_at(checkpoint, SyntaxKind::PathExpr);
        } else {
            // `Module.member`, `held.region.span` — one name in as many words
            // as it was written in. A dot only ever reads this way here: a
            // dotted duration follows a rational, never a name.
            //
            // A loop rather than one step, because §1.2 lets a record hold a
            // record and reading a field of a field is then the ordinary way
            // to say what a program means. Where the module path stops and the
            // projection starts is not a question the parser answers: it is a
            // question about what is *declared*, which is the same reason a
            // bare word in a pattern stays one token here.
            //
            // A method call on a *name* is read here too, and deliberately:
            // `low.rise()` reaches a module's function and `x.equal(y)` calls
            // a method, and the two are the same three tokens. Which one a
            // file wrote is decided by what `low` and `x` denote, which is the
            // same question this loop already declines to answer. So the name
            // is read whole and the call is an application of it, and
            // `10-traits.md` §6's exact-receiver lookup runs where the answer
            // is known. [`SyntaxKind::MethodCallExpr`] is for the receivers a
            // name cannot spell — `f(x).m(y)`, `xs[i].m(y)` — where there is
            // no name for the segments to join onto.
            while self.at(SyntaxKind::Dot) && self.nth_significant(1) == Some(SyntaxKind::Identifier) {
                self.bump();
                self.bump();
            }
            self.start_at(checkpoint, SyntaxKind::NameExpr);
        }
        self.finish();
        if self.at_field_init() {
            self.start_at(checkpoint, SyntaxKind::RecordLiteralExpr);
            self.field_init_list();
            self.finish();
        }
    }

    /// Whether the cursor is on the two `:` tokens that spell `::`.
    pub(super) fn at_path_separator(&self) -> bool {
        self.at(SyntaxKind::Colon) && self.nth_significant(1) == Some(SyntaxKind::Colon)
    }

    /// Whether a record literal's brace opens here, rather than a block or the
    /// brace of an enclosing form.
    ///
    /// Three tokens of lookahead rather than a capitalization test or a
    /// suppression flag, because the question has an answer and this is it: a
    /// record literal writes at least one `field = value` — §1.2 gives it no
    /// empty form — and nothing else in this language puts `=` directly after
    /// a brace. So `match p { … }`, `if p { … }`, and a name that merely
    /// happens to precede a block are all read correctly, and no author has to
    /// learn a list of positions where a literal is suppressed.
    pub(super) fn at_field_init(&self) -> bool {
        self.at(SyntaxKind::LBrace)
            && self.nth_significant(1) == Some(SyntaxKind::Identifier)
            && self.nth_significant(2) == Some(SyntaxKind::Equals)
    }

    /// `{ read = r, dots = d }` — the fields of a record literal.
    pub(super) fn field_init_list(&mut self) {
        self.bump(); // `{`
        while !self.at(SyntaxKind::RBrace) && self.current().is_some() {
            self.start(SyntaxKind::FieldInit);
            self.expect(SyntaxKind::Identifier, "a field name");
            self.expect(SyntaxKind::Equals, "`=`");
            self.expr();
            self.finish();
            if self.at(SyntaxKind::Comma) {
                self.bump();
            } else {
                break;
            }
        }
        self.expect(SyntaxKind::RBrace, "`}`");
    }

    /// `with { field = expr, ... }` — the tail of a record update.
    ///
    /// The subject is already on the stack when this runs; what is parsed here
    /// is only which fields are being replaced. An empty brace pair is a parse
    /// error rather than an identity: writing `p with { }` says nothing that
    /// writing `p` does not, and reading it as `p` would make an empty update
    /// a silent no-op that looks like an unfinished edit.
    pub(super) fn field_update_list(&mut self) {
        self.bump(); // with
        self.expect(SyntaxKind::LBrace, "`{`");
        let mut any = false;
        loop {
            if self.at(SyntaxKind::RBrace) || self.current().is_none() {
                if !any {
                    self.expected("a field to replace, such as `dots = more`");
                }
                break;
            }
            any = true;
            self.field_update();
            if self.at(SyntaxKind::Comma) {
                self.bump();
            } else {
                break;
            }
        }
        self.expect(SyntaxKind::RBrace, "`}`");
    }

    /// `read.refusal = why` — one replaced place of a record update.
    ///
    /// The left-hand side is a path of field names, not an expression. `p with
    /// { f(x).g = y }` is a syntax error rather than a puzzle: a replacement
    /// names a place in the record, and an expression that computes one names
    /// no place to put the answer back.
    pub(super) fn field_update(&mut self) {
        self.start(SyntaxKind::FieldUpdate);
        if !self.at(SyntaxKind::Identifier) {
            self.expected("a field name");
            self.recover(&[SyntaxKind::Comma, SyntaxKind::RBrace]);
            self.finish();
            return;
        }
        self.start(SyntaxKind::FieldPath);
        self.bump();
        while self.at(SyntaxKind::Dot) {
            self.bump();
            self.expect(SyntaxKind::Identifier, "a field name");
        }
        self.finish();
        self.expect(SyntaxKind::Equals, "`=`");
        self.expr();
        self.finish();
    }

    /// `fn (x: τ, …) -> τ { e }` — an anonymous function.
    ///
    /// A declaration's own words without its name, so the parts are the parts
    /// a declaration already has: the parameter list, an optional result
    /// type, and a braced body holding one expression. Nothing here is
    /// declared, which is why it may stand wherever an expression may: the
    /// value it makes is what a higher-order call is specialized with.
    pub(super) fn lambda_expr(&mut self) {
        self.start(SyntaxKind::LambdaExpr);
        self.bump(); // fn
        self.param_list();
        if self.at(SyntaxKind::Arrow) {
            self.bump();
            self.type_expr();
        }
        self.block_expr();
        self.finish();
    }

    /// `{ expression }` — a block, which is a delimiter and not a sequence.
    ///
    /// Exactly one expression, because there is no statement here to be the
    /// second one: `⟦{ e }⟧ = ⟦e⟧`, so a block adds a shape to the surface
    /// and nothing to the calculus. Anything after the first expression is
    /// reported as the rule it breaks rather than read as a sequence nobody
    /// wrote a semantics for.
    pub(super) fn block_expr(&mut self) {
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
    pub(super) fn second_expression_in_a_block(&mut self) {
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

    pub(super) fn expr_arg_list(&mut self) {
        self.start(SyntaxKind::ExprArgList);
        self.bump();
        while !self.at(SyntaxKind::RParen) && self.current().is_some() {
            if self.at_supplied_arg() {
                self.supplied_arg();
                if !self.at(SyntaxKind::Comma) {
                    break;
                }
                self.bump();
                continue;
            }
            self.start(SyntaxKind::ExprArg);
            // A label is one colon, and `::` is two. `f(in: key c major)`
            // names an argument; `f(Delimiter::Layout)` writes a path, and
            // reading its first segment as a label would eat the name and
            // leave the second colon where an expression has to start. The
            // two are told apart by the token after the colon and by nothing
            // else, which is why the path form gets the third token of
            // lookahead rather than a capitalization test: `10-traits.md` §6
            // gives `::` to namespaces, and a lowercase item is as legal
            // after it as an uppercase one.
            if self.at(SyntaxKind::Identifier)
                && self.nth_significant(1) == Some(SyntaxKind::Colon)
                && !self.at_path_separator_next()
            {
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

    /// Whether `{A = …}` starts here rather than a block or a record.
    ///
    /// Three tokens of lookahead and no more: `{` then a name then `=` is the
    /// supplied-parameter form, and nothing else in an argument list begins
    /// that way. A block's first statement cannot be `A =` — assignment is
    /// `assign`, `01-surface.md` §1.4 — and a record literal writes its type
    /// before the brace.
    fn at_supplied_arg(&self) -> bool {
        self.at(SyntaxKind::LBrace)
            && self.nth_significant(1) == Some(SyntaxKind::Identifier)
            && self.nth_significant(2) == Some(SyntaxKind::Equals)
    }

    /// `{A = Nat}` — one type parameter supplied by name.
    fn supplied_arg(&mut self) {
        self.start(SyntaxKind::SuppliedArg);
        self.bump(); // `{`
        self.bump(); // the binder's name
        self.bump(); // `=`
        self.expr();
        self.expect(SyntaxKind::RBrace, "`}`");
        self.finish();
    }

    pub(super) fn paren_or_product_expr(&mut self) {
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

    pub(super) fn list_expr(&mut self) {
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

    pub(super) fn option_expr(&mut self) {
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
    pub(super) fn result_expr(&mut self) {
        self.start(SyntaxKind::ResultExpr);
        self.bump();
        self.expect(SyntaxKind::LParen, "`(`");
        self.expr();
        self.expect(SyntaxKind::RParen, "`)`");
        self.finish();
    }

    /// `if condition { consequent } else { alternative }`.
    ///
    /// The `else` is not optional, which is what makes this one production
    /// with no dangling-else question: an `if` is an expression and has to
    /// have a value in both cases. A ladder is written by putting another `if`
    /// after `else`, and it nests in the alternative rather than becoming a
    /// list of rungs — `else if` is two words the parser reads one at a time,
    /// not a third keyword.
    pub(super) fn if_expr(&mut self) {
        self.start(SyntaxKind::IfExpr);
        self.bump(); // if
        self.expr();
        self.block_expr();
        self.expect(SyntaxKind::ElseKw, "`else`");
        if self.at(SyntaxKind::IfKw) {
            self.if_expr();
        } else {
            self.block_expr();
        }
        self.finish();
    }

    pub(super) fn match_expr(&mut self) {
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

    /// Whether the cursor is on the `.name(` that opens a method call on a
    /// receiver no name could have spelled.
    ///
    /// Three tokens, because two would not tell a method call from a
    /// projection: `held.region` reaches a field and `held.region(x)` calls a
    /// method, and the `(` is the whole of the difference. A method call whose
    /// receiver *is* a name is read by [`Self::name_or_record_literal`]
    /// instead, for the reason written there.
    pub(super) fn at_method_call(&self) -> bool {
        self.at(SyntaxKind::Dot)
            && self.nth_significant(1) == Some(SyntaxKind::Identifier)
            && self.nth_significant(2) == Some(SyntaxKind::LParen)
    }

    /// `<name>(<args>)`
    pub(super) fn call_expr(&mut self) {
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
    pub(super) fn arg(&mut self) {
        self.start(SyntaxKind::Arg);
        if self.at(SyntaxKind::Identifier) && self.nth_significant(1) == Some(SyntaxKind::Colon) {
            self.bump(); // argument name
            self.bump(); // :
        }
        self.value();
        self.finish();
    }

    /// A number with an optional unit, a nested construction, or a name.
    pub(super) fn value(&mut self) {
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
}
