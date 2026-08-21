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
        if self.at(SyntaxKind::Less) {
            self.type_params();
        }
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
            self.expect(SyntaxKind::Identifier, "a parameter name");
            if self.at(SyntaxKind::Colon) {
                self.bump();
                self.type_expr();
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
    pub(super) fn bracketed_parameter(&mut self) {
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

    /// `<A, B>` — the type parameters a declaration abstracts over.
    pub(super) fn type_params(&mut self) {
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
}
