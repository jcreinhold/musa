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
    pub(super) fn type_expr(&mut self) {
        let checkpoint = self.events.len();
        self.type_atom();
        let mut applied = false;
        while self.at(SyntaxKind::LParen) {
            self.start_at(checkpoint, SyntaxKind::ApplyExpr);
            self.type_arg_list();
            self.finish();
            applied = true;
        }
        if applied {
            self.start_at(checkpoint, SyntaxKind::TypeExpr);
            self.finish();
        }
        if self.at(SyntaxKind::Arrow) {
            self.start_at(checkpoint, SyntaxKind::FunctionType);
            self.bump();
            self.type_expr();
            self.finish();
        }
    }

    /// Arguments to an application written in a type position. They are terms:
    /// a type, a value, or a function type are all checked against the reached
    /// Π's domain by elaboration.
    fn type_arg_list(&mut self) {
        self.start(SyntaxKind::ExprArgList);
        self.bump();
        while !self.at(SyntaxKind::RParen) && self.current().is_some() {
            self.start(SyntaxKind::ExprArg);
            let function_type = (0..64)
                .map_while(|offset| self.nth_significant(offset))
                .take_while(|kind| !matches!(kind, SyntaxKind::Comma | SyntaxKind::RParen))
                .any(|kind| kind == SyntaxKind::Arrow);
            if self.at(SyntaxKind::LParen) || function_type {
                self.type_expr();
            } else {
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

    /// A type is an identifier, a parenthesized or product type, or one of the
    /// two the compiler parameterizes.
    ///
    /// No keyword stands here. `key` is a statement and `Key` is a type, and
    /// the capital is what tells them apart — which is why this reads a plain
    /// [`SyntaxKind::Identifier`] rather than a list of the keywords a type
    /// was allowed to also be. The removed spellings are read only to be
    /// reported.
    pub(super) fn type_atom(&mut self) {
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
        if self.at_any(&[
            SyntaxKind::Integer,
            SyntaxKind::Rational,
            SyntaxKind::TrueKw,
            SyntaxKind::FalseKw,
            SyntaxKind::String,
        ]) {
            self.start(SyntaxKind::LiteralExpr);
            self.bump();
            self.finish();
            return;
        }
        self.eat_trivia();
        // A type constructor is a term. Give its application the same
        // `NameExpr`/`ApplyExpr` tree as every other call; the checker decides
        // whether each argument inhabits `Type`, `Nat`, or another domain.
        if self.at_any(&[
            SyntaxKind::Identifier,
            SyntaxKind::ListKw,
            SyntaxKind::OptionKw,
            SyntaxKind::ResultKw,
        ]) && self.nth_significant(1) == Some(SyntaxKind::LParen)
        {
            self.start(SyntaxKind::NameExpr);
            self.bump();
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
             D♭ are two things here and one `Pc(12)`"
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
