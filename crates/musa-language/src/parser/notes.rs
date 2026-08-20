//! The note-item grammar: notes, rests, chords, stacks, ties, articulations, and durations.

use super::engine::Parser;
use crate::{SyntaxError, SyntaxKind};

impl Parser<'_> {
    pub(super) fn music_expr(&mut self) {
        self.start(SyntaxKind::MusicExpr);
        self.bump();
        self.expect(SyntaxKind::LBrace, "`{`");
        self.voice_items();
        self.expect(SyntaxKind::RBrace, "`}`");
        self.finish();
    }

    /// `<pitch-or-ref> <duration> <articulation>* ~?`
    ///
    /// No terminator: an event is self-delimiting, because a pitch and a
    /// duration are the two things it starts with and nothing else in a voice
    /// starts that way.
    pub(super) fn note_stmt(&mut self) {
        self.start(SyntaxKind::NoteStmt);
        if self.at(SyntaxKind::LParen)
            || matches!(
                self.nth_significant(1),
                Some(SyntaxKind::UpKw | SyntaxKind::DownKw | SyntaxKind::StepKw)
            )
        {
            self.written_pitch();
        } else {
            self.bump(); // simple pitch literal or pitch reference
        }
        self.duration();
        self.articulations();
        self.tie();
        self.finish();
    }

    /// Zero or more articulations after a duration: the words (`accent
    /// staccato`) and the two marks notation writes (`>` accent, `^`
    /// marcato).
    ///
    /// They live in their own node: a bare identifier in a note statement is
    /// otherwise a pitch or duration parameter reference, and telling the two
    /// apart by counting tokens is exactly the kind of positional rule that
    /// breaks the next time the statement grows a part.
    ///
    /// A note's own duration is read before this runs, so what is left here
    /// is either a word describing the note or the start of the next event.
    pub(super) fn articulations(&mut self) {
        if !self.at_articulation() {
            return;
        }
        self.start(SyntaxKind::ArticulationList);
        while self.at_articulation() {
            self.bump();
        }
        self.finish();
    }

    /// Whether the parser is on an articulation rather than on the next
    /// event.
    ///
    /// Without a `;` between events, `root 1/8 tenuto d5 1/8` has to be read
    /// the way a player reads it: `tenuto` belongs to the note before it and
    /// `d5` starts the note after. A word that a duration follows is a pitch
    /// — that is what a duration is *for* — so one token of lookahead on a
    /// kind settles it.
    pub(super) fn at_articulation(&self) -> bool {
        if self.at_any(&[SyntaxKind::Greater, SyntaxKind::Caret]) {
            return true;
        }
        if !self.at(SyntaxKind::Identifier) {
            return false;
        }
        !matches!(
            self.nth_significant(1),
            Some(SyntaxKind::Slash | SyntaxKind::Rational | SyntaxKind::Integer)
        )
    }

    /// The postfix tie mark, tying this statement to the next.
    pub(super) fn tie(&mut self) {
        if self.at(SyntaxKind::Tilde) {
            self.bump();
        }
    }

    /// `rest <duration>`
    pub(super) fn rest_stmt(&mut self) {
        self.start(SyntaxKind::RestStmt);
        self.bump(); // rest
        self.duration();
        self.finish();
    }

    /// `[<pitch> ...]<duration>` — `[c3 g3]/2`.
    ///
    /// The bracket says chord, the way it does in ABC and in GUIDO, so the
    /// keyword and the commas were both repeating what it already said.
    pub(super) fn chord_stmt(&mut self) {
        self.start(SyntaxKind::ChordStmt);
        self.bump(); // [
        self.expect(SyntaxKind::PitchLiteral, "a pitch");
        while self.at(SyntaxKind::PitchLiteral) {
            self.bump();
        }
        self.expect(SyntaxKind::RBracket, "`]`");
        self.duration();
        self.articulations();
        self.tie();
        self.finish();
    }

    /// `stack <pitch> <type><duration>` — a chord sounded in close position.
    ///
    /// The root is written as an absolute pitch because the register is the
    /// whole of what a voicing adds to a chord class. A pitch class is
    /// accepted by the grammar and refused by the compiler, so the mistake is
    /// answered with a sentence about register rather than with `expected a
    /// pitch`.
    pub(super) fn stack_stmt(&mut self) {
        self.start(SyntaxKind::StackStmt);
        self.bump(); // stack
        if self.at(SyntaxKind::PitchLiteral) {
            self.bump();
        } else {
            // A pitch class is parsed rather than rejected, so the compiler
            // gets to answer with the sentence about register.
            self.pitch_class();
        }
        self.expect(SyntaxKind::Identifier, "a chord type such as `major` or `major7`");
        self.duration();
        self.articulations();
        self.tie();
        self.finish();
    }

    /// A duration: `1`, `1/2`, `3/8`, `1/12`, `/4`, `/4.`, …
    ///
    /// It gets a node of its own because `c4/4` puts a bare `4` inside a note
    /// statement, and every reader that finds a duration by taking the first
    /// numeral it sees would read that as a whole note — silently, and on the
    /// path of every note in the language. With a node, a reader that looks in
    /// the wrong place finds nothing instead of finding the wrong thing.
    pub(super) fn duration(&mut self) {
        self.start(SyntaxKind::Duration);
        self.duration_value("a duration");
        // `g4/4 to 2/1` — written as a quarter, held as long as the
        // performer likes up to a double whole. The first value is the
        // notated one and the second bounds the performed one (roadmap §2).
        if self.at(SyntaxKind::ToKw) {
            self.bump();
            self.duration_value("the longest the note may be held");
        }
        self.finish();
    }

    /// One duration value, in either spelling.
    ///
    /// `/N` says the same thing as `1/N` in one character less than the
    /// pitch beside it, and augmentation dots multiply it by `2 − 2⁻ᵈ`:
    /// `/4.` is 3/8 and `/4..` is 7/16.
    pub(super) fn duration_value(&mut self, what: &str) {
        if self.at(SyntaxKind::Slash) {
            self.bump(); // `/`
            if self.at(SyntaxKind::Integer) {
                self.bump();
            } else {
                self.expected("a note value such as `4` or `8`");
            }
            while self.at(SyntaxKind::Dot) {
                self.bump();
            }
            return;
        }
        if self.at_any(&[SyntaxKind::Rational, SyntaxKind::Integer, SyntaxKind::Identifier]) {
            self.bump(); // literal or duration-parameter reference
            self.no_dots_on_the_long_form();
            return;
        }
        self.expected(what);
    }

    /// A dot after `3/8` is refused rather than read as 9/16: the long form
    /// already writes 9/16, so two spellings of one duration would be one
    /// spelling too many.
    pub(super) fn no_dots_on_the_long_form(&mut self) {
        if !self.at(SyntaxKind::Dot) || self.cascading() {
            return;
        }
        if let Some(token) = self.significant() {
            let at = token.range;
            self.errors.push(
                SyntaxError::new(
                    at,
                    "an augmentation dot needs the short form",
                    "this dot has nothing to dot",
                )
                .with_help("`/8.` is a dotted eighth; written as a fraction it is `3/16`"),
            );
        }
        while self.at(SyntaxKind::Dot) {
            self.bump();
        }
    }
}
