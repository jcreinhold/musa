//! Harmonic analysis: harmony declarations and statements, positions, chord symbols, and pitch classes.

use super::engine::Parser;
use crate::SyntaxKind;

impl Parser<'_> {
    /// `harmony { at 1:1 am; ... }` — the chord-symbol lane.
    pub(super) fn harmony_decl(&mut self) {
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
    pub(super) fn harmony_stmt(&mut self) {
        self.start(SyntaxKind::HarmonyStmt);
        self.bump(); // at
        self.position();
        self.chord_symbol();
        self.expect(SyntaxKind::Semicolon, "`;`");
        self.finish();
    }

    /// `<measure>:<beat>` — a position in the piece, in the coordinates a
    /// composer reads off the page.
    pub(super) fn position(&mut self) {
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
    pub(super) fn chord_symbol(&mut self) {
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
    pub(super) fn pitch_class(&mut self) {
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
}
