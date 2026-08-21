//! Musical operand grammar: written pitches, scale and key expressions, and chords.

use super::engine::{Operand, Parser};
use crate::SyntaxKind;

impl Parser<'_> {
    /// A note statement's pitch: levels 1, 4, and 5, and not the arithmetic
    /// between them.
    ///
    /// In a music statement a duration follows the pitch, so `/4` is that
    /// duration and `-` is a negative rational rather than a subtraction.
    /// `c5 up 2 /4` is therefore a transposed quarter note, and an author who
    /// means arithmetic writes `(base + 2) up 2` — the parentheses being the
    /// ordinary way to say that an expression is one operand.
    pub(super) fn written_pitch(&mut self) {
        let spoken_for = std::mem::take(&mut self.with_is_spoken_for);
        self.pitch_expr(spoken_for, Operand::Written);
    }

    /// Level 5 — `up` and `down`, non-associative: `p up M2 down m2` needs
    /// parentheses.
    pub(super) fn pitch_expr(&mut self, spoken_for: bool, operand: Operand) {
        let checkpoint = self.events.len();
        self.step_expr(spoken_for, operand);
        if self.at_any(&[SyntaxKind::UpKw, SyntaxKind::DownKw]) {
            self.start_at(checkpoint, SyntaxKind::PitchExpr);
            self.bump();
            self.step_expr(spoken_for, operand);
            self.finish();
        }
    }

    /// Level 4 — `step`, left-associative, so `p step 1 up m2` steps first.
    pub(super) fn step_expr(&mut self, spoken_for: bool, operand: Operand) {
        let checkpoint = self.events.len();
        self.pitch_operand(spoken_for, operand);
        while self.at(SyntaxKind::StepKw) {
            self.start_at(checkpoint, SyntaxKind::StepExpr);
            self.bump();
            // The direction is optional and defaults to up, which is what
            // `c5 step 2` reads as on the page.
            if self.at_any(&[SyntaxKind::UpKw, SyntaxKind::DownKw]) {
                self.bump();
            }
            self.pitch_operand(spoken_for, operand);
            self.finish();
        }
    }

    /// Whichever grammar the written-pitch operators draw their operands from.
    pub(super) fn pitch_operand(&mut self, spoken_for: bool, operand: Operand) {
        match operand {
            Operand::Arithmetic => self.additive_expr(spoken_for),
            Operand::Written => self.operand_expr(spoken_for),
        }
    }

    /// `scale <tonic> <collection>` — a collection rooted on a pitch class.
    pub(super) fn scale_expr(&mut self) {
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
    pub(super) fn key_expr(&mut self) {
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
    pub(super) fn chord_expr(&mut self) {
        self.start(SyntaxKind::ChordExpr);
        self.bump(); // chord
        self.pitch_class();
        self.expect(SyntaxKind::Identifier, "a chord type such as `major` or `major7`");
        self.finish();
    }
}
