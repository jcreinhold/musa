//! Value declarations: `let`, `fn`, trait methods, signatures, and parameter lists.

use super::engine::Parser;
use crate::{SyntaxError, SyntaxKind};
use text_size::{TextRange, TextSize};

impl Parser<'_> {
    /// `let name = expression;`, with an optional `: type` before the `=`.
    ///
    /// The annotation is written when it says something the expression does
    /// not — a public signature, a narrower type than the value's own — and
    /// omitted when it would only repeat what is already there. Which of the
    /// two a file chose is a fact about the file, so the colon and the type
    /// stay in the tree exactly where they were written.
    pub(super) fn let_decl(&mut self) {
        self.start(SyntaxKind::LetDecl);
        self.visibility();
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
    pub(super) fn fn_decl(&mut self) {
        self.start(SyntaxKind::FnDecl);
        self.fn_signature();
        if self.at(SyntaxKind::Equals) {
            self.old_function_body();
        } else {
            self.block_expr();
        }
        self.finish();
    }

    /// Everything a function declaration writes before its body.
    ///
    /// Its own function because a signature read in two places would be a
    /// signature that could come to mean two things.
    pub(super) fn fn_signature(&mut self) {
        self.visibility();
        self.bump(); // fn
        self.expect(SyntaxKind::Identifier, "a function name");
        self.param_list();
        if self.at(SyntaxKind::Arrow) {
            self.bump();
            self.type_expr();
        }
    }

    /// The one complaint a file written against `fn f() -> τ = e;` gets.
    ///
    /// The whole `= e;` is read, so the tree is the tree the file describes
    /// and the error spans exactly the text the fix replaces. The fix is the
    /// body written back between braces: mechanical, because the expression
    /// is unchanged and only its delimiters moved.
    pub(super) fn old_function_body(&mut self) {
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

    pub(super) fn param_list(&mut self) {
        self.start(SyntaxKind::ParamList);
        self.expect(SyntaxKind::LParen, "`(`");
        while !self.at(SyntaxKind::RParen) && self.current().is_some() {
            self.start(SyntaxKind::Param);
            let inferred = self.at(SyntaxKind::LBrace);
            if inferred {
                self.bump();
            }
            self.expect(SyntaxKind::Identifier, "a parameter name");
            if self.at(SyntaxKind::Colon) {
                self.bump();
                self.type_expr();
            }
            if inferred {
                self.expect(SyntaxKind::RBrace, "`}`");
            }
            if self.at(SyntaxKind::Equals) {
                let equals = self.significant().map(|token| token.range.start());
                self.bump();
                self.expr();
                self.parameter_default(equals);
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

    /// The one complaint a file written against `f(x: τ = e)` gets.
    ///
    /// The default is read, so the tree is the tree the file describes and the
    /// error spans exactly the text the fix deletes. A default is not a
    /// convenience the language withdrew: a call supplies every declared
    /// parameter, and a default is a second opinion about what an argument
    /// list means (`docs/rules/constitution.md` §9).
    pub(super) fn parameter_default(&mut self, equals: Option<TextSize>) {
        let end = self.previous_end();
        if self.cascading() {
            return;
        }
        let Some(equals) = equals else {
            return;
        };
        self.errors.push(
            SyntaxError::new(
                TextRange::new(equals, end),
                "a parameter may not have a default",
                "this default would stand in for an argument",
            )
            .with_help("a call supplies every declared parameter — write the value at each call site instead")
            .with_fix("delete the default", String::new()),
        );
    }
}
