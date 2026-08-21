//! Type expressions and respellings.

use super::engine::Parser;
use crate::{SyntaxError, SyntaxKind};

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

impl Parser<'_> {
    /// Right-associative arrow types; product/list/option types are atoms.
    pub(super) fn type_expr(&mut self) {
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
    pub(super) fn type_atom(&mut self) {
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
        // `Pc(12)` — a type carrying an index. `02-core-calculus.md` §1.5 spells
        // an index in parentheses precisely so that it is not the angle-bracket
        // form above: `Pc<A>` takes a type and `Pc(12)` takes a number, and the
        // grammar tells them apart rather than the checker.
        //
        // The index is read as an ordinary expression, because §1.5's grammar is
        // a restriction on what an index may *say* and not a second syntax. An
        // expression outside it is refused where two indices are compared, with
        // the comparison that could not be made; refusing it here would be a
        // complaint with nothing to point at.
        if self.at(SyntaxKind::Identifier) && self.nth_significant(1) == Some(SyntaxKind::LParen) {
            self.start(SyntaxKind::IndexedType);
            self.start(SyntaxKind::TypeName);
            self.bump();
            self.finish();
            self.bump(); // `(`
            self.expr();
            self.expect(SyntaxKind::RParen, "`)`");
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

    /// The one complaint a type written in the old vocabulary gets.
    ///
    /// Located at the word and carrying the word that replaces it, so the
    /// migration is an accepted fix rather than a search. The old spelling is
    /// then read as the type it named, so the rest of the declaration is
    /// checked rather than buried under a cascade.
    pub(super) fn respelled_type(&mut self) {
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
    pub(super) fn respelled_constructor(&mut self) {
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
}
