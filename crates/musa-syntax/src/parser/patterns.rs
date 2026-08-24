//! Match patterns and constructor bindings.

use super::MODULE_NAME;
use super::engine::Parser;
use crate::SyntaxKind;

impl Parser<'_> {
    pub(super) fn pattern(&mut self) {
        self.start(SyntaxKind::Pattern);
        match self.current() {
            Some(SyntaxKind::QuoteKw) => self.quote_pattern(),
            Some(SyntaxKind::Identifier) if self.at_word("none") => {
                self.respelled_constructor();
                self.bump();
            }
            Some(SyntaxKind::Identifier) if self.at_word("some") => {
                self.respelled_constructor();
                self.bump();
                self.held_pattern();
            }
            // `Tying::Untied`, `Reading::Refused { why = w }` — a case named
            // in its type's namespace. The bare spelling is the arm below and
            // means the same thing: which of the two a word is is a question
            // about what is declared, and §1.3 has the checker answer it
            // against the scrutinee's type rather than the parser guess.
            Some(SyntaxKind::Identifier) if self.at_path_separator_next() => {
                self.bump();
                while self.at_path_separator() {
                    self.bump();
                    self.bump();
                    if self.at_any(MODULE_NAME) {
                        self.bump();
                    } else {
                        self.expected("a case name");
                        break;
                    }
                }
                if self.at(SyntaxKind::LParen) {
                    self.constructor_bindings();
                } else if self.at(SyntaxKind::LBrace) {
                    self.record_pattern();
                }
            }
            // `Pending { read = r }` — a record pattern, naming the fields
            // this arm cares about and nothing about the rest.
            Some(SyntaxKind::Identifier) if self.nth_significant(1) == Some(SyntaxKind::LBrace) => {
                self.bump();
                self.record_pattern();
            }
            // `{ read = r }` — the same pattern without the name in front of
            // it, and the form §1.2 actually implies: a record *is* its
            // fields, so the type's name is a reading aid rather than part of
            // what is being matched. There is no block to be confused with
            // here — a pattern position admits no expression — so the brace
            // needs no lookahead to be read.
            Some(SyntaxKind::LBrace) => self.record_pattern(),
            // `Sounded(pitch, held)` — a declared constructor, taking one
            // binding per field. A bare name is still one token here, because
            // whether `Silence` is a constructor or a binding is a question
            // about what is declared, which the checker answers against the
            // type being matched.
            Some(SyntaxKind::Identifier) if self.nth_significant(1) == Some(SyntaxKind::LParen) => {
                self.bump();
                self.constructor_bindings();
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
                self.held_pattern();
            }
            Some(SyntaxKind::LBracket) => {
                self.bump();
                if !self.at(SyntaxKind::RBracket) {
                    self.pattern();
                    self.expect(SyntaxKind::Comma, "`,`");
                    self.expect(SyntaxKind::Dot, "`.`");
                    self.expect(SyntaxKind::Dot, "`.`");
                    self.pattern();
                }
                self.expect(SyntaxKind::RBracket, "`]`");
            }
            Some(SyntaxKind::LParen) => {
                self.bump();
                self.pattern();
                self.expect(SyntaxKind::Comma, "`,`");
                self.pattern();
                while self.at(SyntaxKind::Comma) {
                    self.bump();
                    self.pattern();
                }
                self.expect(SyntaxKind::RParen, "`)`");
            }
            _ => self.expected("a match pattern"),
        }
        self.finish();
    }

    /// Whether `::` follows the token at the cursor.
    pub(super) fn at_path_separator_next(&self) -> bool {
        self.nth_significant(1) == Some(SyntaxKind::Colon) && self.nth_significant(2) == Some(SyntaxKind::Colon)
    }

    /// `(pitch, Loud(count))` — one *pattern* per field of a constructor
    /// pattern.
    ///
    /// Patterns and not bindings. `01-surface.md` §1 says "a sub-position holds
    /// another pattern rather than only a binder" and `02-core-calculus.md`
    /// §6.2's case tree splits a sub-position the same way it splits the
    /// subject, so the grammar is recursive because the compiler already is. A
    /// bare lowercase name is still a binder — that reading is the
    /// [`Self::pattern`] arm it always was, reached one level down.
    pub(super) fn constructor_bindings(&mut self) {
        self.bump(); // `(`
        while !self.at(SyntaxKind::RParen) && self.current().is_some() {
            // A pattern that reads nothing would spin here: `pattern` reports
            // and does not consume where it recognizes no opening token, so the
            // loop, not the callee, is what has to notice.
            let before = self.pos;
            self.pattern();
            if self.pos == before {
                break;
            }
            if self.at(SyntaxKind::Comma) {
                self.bump();
            } else {
                break;
            }
        }
        self.expect(SyntaxKind::RParen, "`)`");
    }

    /// `(held)` — the one sub-pattern `Some`, `Ok` and `Err` each take.
    ///
    /// Their own function because they are spelled as keywords and so cannot
    /// reach [`Self::constructor_bindings`] through the identifier arm, and
    /// because each takes exactly one field: a second would be a pattern for a
    /// constructor that does not exist.
    fn held_pattern(&mut self) {
        self.expect(SyntaxKind::LParen, "`(`");
        if !self.at(SyntaxKind::RParen) {
            self.pattern();
        }
        self.expect(SyntaxKind::RParen, "`)`");
    }

    /// `{ read = r, taken }` — the fields a record pattern names.
    ///
    /// The head is already read. A field's sub-position holds another pattern,
    /// which is what makes `Pending { read = Reading { refusal = why } }`
    /// writable; the shorthand `taken` binds a field to its own name, and is
    /// the form an arm that only wants the value writes.
    pub(super) fn record_pattern(&mut self) {
        self.start(SyntaxKind::RecordPattern);
        self.bump(); // `{`
        while !self.at(SyntaxKind::RBrace) && self.current().is_some() {
            if !self.at(SyntaxKind::Identifier) {
                self.expected("a field name");
                self.recover(&[SyntaxKind::Comma, SyntaxKind::RBrace]);
                continue;
            }
            self.start(SyntaxKind::FieldPattern);
            self.bump();
            if self.at(SyntaxKind::Equals) {
                self.bump();
                self.pattern();
            }
            self.finish();
            if self.at(SyntaxKind::Comma) {
                self.bump();
            } else {
                break;
            }
        }
        self.expect(SyntaxKind::RBrace, "`}`");
        self.finish();
    }
}
