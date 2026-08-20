//! Voice-item statements beyond notes: use/override/transpose/repeat, slurs, marks, phrases, dynamics, and the bar forms.

use super::engine::Parser;
use crate::{SyntaxError, SyntaxKind};
use text_size::TextRange;

impl Parser<'_> {
    /// `use name(args);` — or `use name;`, when the material takes none.
    pub(super) fn use_stmt(&mut self) {
        self.start(SyntaxKind::UseStmt);
        self.bump(); // use
        // This statement's own `with` clause follows the expression, so the
        // expression must not read it as a record update — see
        // `with_is_spoken_for`.
        let outer = std::mem::replace(&mut self.with_is_spoken_for, true);
        self.expr();
        self.with_is_spoken_for = outer;
        // `with { ... }` specializes this occurrence and only this one
        // (roadmap §9). A call that ends there is a block, not a statement,
        // so it takes no `;` — the same shape every other block has.
        if self.at(SyntaxKind::WithKw) {
            self.with_clause();
        } else {
            self.expect(SyntaxKind::Semicolon, "`;`");
        }
        self.finish();
    }

    /// `with { note <n> = <pitch>; ... }` — overrides on one occurrence.
    pub(super) fn with_clause(&mut self) {
        self.start(SyntaxKind::WithClause);
        self.bump(); // with
        self.expect(SyntaxKind::LBrace, "`{`");
        loop {
            if self.at(SyntaxKind::RBrace) || self.current().is_none() {
                break;
            }
            if self.at(SyntaxKind::NoteKw) {
                self.override_stmt();
            } else {
                self.expected("an override such as `note 2 = d5;`");
                self.recover(&[SyntaxKind::Semicolon, SyntaxKind::RBrace, SyntaxKind::NoteKw]);
            }
        }
        self.expect(SyntaxKind::RBrace, "`}`");
        self.finish();
    }

    /// `note <n> = <pitch>;` — the nth note of this occurrence, respelled.
    pub(super) fn override_stmt(&mut self) {
        self.start(SyntaxKind::OverrideStmt);
        self.bump(); // note
        self.expect(SyntaxKind::Integer, "the note's position in the occurrence");
        self.expect(SyntaxKind::Equals, "`=`");
        self.expect(SyntaxKind::PitchLiteral, "a pitch");
        self.expect(SyntaxKind::Semicolon, "`;`");
        self.finish();
    }

    /// `in scale <expr> { ... }` — the enclosed music read in a scale.
    ///
    /// Only `scale` follows `in`. There is no `in key` or `in meter`: those
    /// are structural facts a score states at a place, and a lexical block
    /// that quietly changed one would be a modulation nobody wrote.
    pub(super) fn in_scale_stmt(&mut self) {
        self.start(SyntaxKind::InScaleStmt);
        self.bump(); // in
        if !self.at(SyntaxKind::ScaleKw) {
            self.expected("`scale` — a scale is the only context entered lexically");
            self.expr();
            self.block();
            self.finish();
            return;
        }
        // `in scale c major { … }` writes the collection out and `in scale s
        // { … }` names one. Two words after `scale` is the literal; anything
        // else is an expression, and the block's `{` is what tells them
        // apart.
        let written_out = matches!(self.nth_significant(1), Some(SyntaxKind::Identifier))
            && matches!(self.nth_significant(2), Some(SyntaxKind::Identifier | SyntaxKind::Hash));
        if written_out {
            self.scale_expr();
        } else {
            self.bump(); // scale
            self.expr();
        }
        self.block();
        self.finish();
    }

    /// `transpose up|down <quality><size> { ... }` — e.g. `transpose down P5`.
    pub(super) fn transpose_stmt(&mut self) {
        self.start(SyntaxKind::TransposeStmt);
        self.bump(); // transpose
        if self.at_any(&[SyntaxKind::UpKw, SyntaxKind::DownKw]) {
            self.bump();
        } else {
            self.expected("`up` or `down`");
        }
        self.expect(SyntaxKind::IntervalLiteral, "an interval such as `P5` or `m3`");
        self.block();
        self.finish();
    }

    /// `repeat <n> { ... }`
    /// `repeat 4 { … }`, or `repeat 4 to 16 { … }` — a count chosen by the
    /// realization rather than by the composer.
    ///
    /// One statement with two forms rather than two statements: the block, the
    /// recovery, and every consumer are the same, and the only difference is
    /// whether the number is one number or two.
    pub(super) fn repeat_stmt(&mut self) {
        self.start(SyntaxKind::RepeatStmt);
        self.bump(); // repeat
        self.expect(SyntaxKind::Integer, "a repeat count");
        if self.at(SyntaxKind::ToKw) {
            self.bump();
            self.expect(SyntaxKind::Integer, "the largest number of passes");
        }
        self.block();
        self.finish();
    }

    /// `ending 1 { ... }`
    ///
    /// Parsed wherever a note is, though it only means something inside a
    /// `repeat`. Where it belongs is a question about the music, and the
    /// compiler answers it with a sentence; a syntax error here would only be
    /// able to say the grammar disagreed.
    pub(super) fn ending_stmt(&mut self) {
        self.start(SyntaxKind::EndingStmt);
        self.bump(); // ending
        self.expect(SyntaxKind::Integer, "which pass this is");
        self.block();
        self.finish();
    }

    /// `bar { ... }` / `bar head { ... }`
    pub(super) fn bar_stmt(&mut self) {
        if let Some(error) = self.nested_bar() {
            self.errors.push(error);
        }
        // The blank line a composer leaves between two phrases belongs to the
        // voice, not to the bar under it: a bar is written on one line, and a
        // formatter that never descends into it would never see the trivia.
        self.eat_trivia();
        self.start(SyntaxKind::BarStmt);
        self.bump(); // bar
        // The name is optional and there is nothing to disambiguate: a bar's
        // contents start with `{`, so an identifier here can only be a name.
        if self.at(SyntaxKind::Identifier) {
            self.bump();
        }
        self.bar_depth = self.bar_depth.saturating_add(1);
        self.block();
        self.bar_depth = self.bar_depth.saturating_sub(1);
        self.finish();
    }

    /// `assert pitches_in(scale c major) { ... }`
    ///
    /// The parentheses are written even when the claim takes no arguments, so
    /// that `assert fills_meter()` and a name the composer misremembered are
    /// told apart by the grammar rather than by a guess. Which names exist,
    /// and what each one's arguments are, is the compiler's registry and not
    /// the parser's business — a claim nobody has heard of parses, and then
    /// gets a sentence naming the ones that do.
    ///
    /// Assertions nest: two claims about one passage are two assertions, and
    /// writing them one inside the other is how a composer says both.
    pub(super) fn assert_stmt(&mut self) {
        self.start(SyntaxKind::AssertStmt);
        self.bump(); // assert
        self.expect(SyntaxKind::Identifier, "what is being claimed");
        if self.at(SyntaxKind::LParen) {
            self.expr_arg_list();
        } else {
            self.expected("`(` — a claim is written with its arguments, and one with none is written `()`");
        }
        self.block();
        self.finish();
    }

    /// `| <items…>` — a bar drawn the way notation draws one.
    ///
    /// The same node as `bar { … }`, because it is the same claim: one
    /// measure's worth of music, checked against the meter. What it does not
    /// have is a name, and it does not have one *structurally* —
    /// [`BarStmt::name`](crate::ast::BarStmt::name) reads direct identifier
    /// tokens, and a pipe bar's direct tokens are the pipe and trivia.
    pub(super) fn pipe_bar_stmt(&mut self) {
        if let Some(error) = self.nested_bar() {
            self.errors.push(error);
        }
        // The blank line a composer leaves between two phrases belongs to the
        // voice, not to the bar under it: a bar is written on one line, and a
        // formatter that never descends into it would never see the trivia.
        self.eat_trivia();
        self.start(SyntaxKind::BarStmt);
        self.bump(); // |
        self.bar_depth = self.bar_depth.saturating_add(1);
        let enclosing = std::mem::replace(&mut self.in_pipe_bar, true);
        self.voice_items();
        self.in_pipe_bar = enclosing;
        self.bar_depth = self.bar_depth.saturating_sub(1);
        self.finish();
    }

    /// The `;` a reader who learned the old syntax will type after a note.
    ///
    /// Worth its own arm rather than the generic complaint: the fix is a
    /// deletion, so every file written before today converts itself one
    /// keystroke at a time.
    pub(super) fn stray_semicolon(&mut self) {
        let range = self
            .significant()
            .map_or_else(|| TextRange::empty(self.end_size()), |token| token.range);
        if !self.cascading() {
            self.errors.push(
                SyntaxError::new(range, "a note does not end in `;`", "delete this")
                    .with_help("a note, a rest and a chord end themselves; every other statement ends with `;` or `}`")
                    .with_fix("remove `;`", ""),
            );
        }
        self.bump();
    }

    /// A `bar` inside a `bar`, if that is where the parser is.
    pub(super) fn nested_bar(&self) -> Option<SyntaxError> {
        if self.bar_depth == 0 || self.cascading() {
            return None;
        }
        let range = self
            .significant()
            .map_or_else(|| TextRange::empty(self.end_size()), |token| token.range);
        Some(
            SyntaxError::new(range, "bars do not nest", "this bar is inside another")
                .with_help("close the bar above this one, or delete this `bar`"),
        )
    }

    /// `slur { ... }`
    pub(super) fn slur_stmt(&mut self) {
        self.start(SyntaxKind::SlurStmt);
        self.bump(); // slur
        self.block();
        self.finish();
    }

    /// `mark <name> [<argument>] ;` or `mark <name> [<argument>] { ... }` —
    /// a notation mark that is not written on a note.
    ///
    /// The parser does not know which marks exist, which take an argument, or
    /// which want a block: it accepts the shape and the compiler checks it
    /// against the vocabulary. That is the point of the table — a new mark is
    /// a row, and this function never changes.
    pub(super) fn mark_stmt(&mut self) {
        self.start(SyntaxKind::MarkStmt);
        self.bump(); // mark
        self.expect(SyntaxKind::Identifier, "a mark such as `breath` or `pedal`");
        if self.at_any(&[SyntaxKind::String, SyntaxKind::Integer, SyntaxKind::Minus]) {
            if self.at(SyntaxKind::Minus) {
                self.bump();
                self.expect(SyntaxKind::Integer, "a whole number");
            } else {
                self.bump();
            }
        }
        if self.at(SyntaxKind::LBrace) {
            self.block();
        } else {
            self.expect(SyntaxKind::Semicolon, "`;` or a block");
        }
        self.finish();
    }

    /// `grace { c5 d5 }` — the notes crushed before the one that follows.
    ///
    /// The block holds pitches and nothing else: a grace note has no written
    /// duration, which is the one shape a `NoteStmt` cannot hold, so it gets a
    /// node of its own rather than a note with an optional duration. Making
    /// the duration optional on every note would let `c5` be written anywhere
    /// and mean nothing.
    ///
    /// A grace note delimits itself the way every other event does: a pitch
    /// and the marks written on it are one word, and the `}` ends the last
    /// one. What separates two grace notes is what separates two notes —
    /// nothing but space.
    pub(super) fn grace_stmt(&mut self) {
        // The comment above a grace group belongs to the voice, not to the
        // group: the group is written on one line, and a formatter that never
        // descends into it would never see the trivia.
        self.eat_trivia();
        self.start(SyntaxKind::GraceStmt);
        self.bump(); // grace
        if self.at(SyntaxKind::LBrace) {
            self.bump();
            while !self.at(SyntaxKind::RBrace) && self.current().is_some() {
                if self.at_any(&[SyntaxKind::PitchLiteral, SyntaxKind::Identifier]) {
                    self.start(SyntaxKind::GraceNote);
                    self.bump(); // pitch literal or pitch reference
                    self.articulations();
                    self.finish();
                } else if self.at(SyntaxKind::Semicolon) {
                    self.stray_semicolon();
                } else {
                    self.expected("a pitch");
                    self.recover(&[SyntaxKind::Semicolon, SyntaxKind::RBrace, SyntaxKind::PitchLiteral]);
                }
            }
            self.expect(SyntaxKind::RBrace, "`}`");
        } else {
            self.expected("a block of pitches, like `grace { c5 d5 }`");
        }
        self.finish();
    }

    /// `phrase "A" { ... }` — a named span over the music it wraps.
    pub(super) fn phrase_stmt(&mut self) {
        self.start(SyntaxKind::PhraseStmt);
        self.bump(); // phrase
        self.expect(SyntaxKind::String, "a phrase name in quotes");
        self.block();
        self.finish();
    }

    /// `crescendo to f { ... }` — a hairpin over the notes it wraps.
    ///
    /// The mark it grows to is written; the mark it grows *from* is whatever
    /// dynamic is in force, because that is what a hairpin means on a page.
    pub(super) fn hairpin_stmt(&mut self) {
        self.start(SyntaxKind::HairpinStmt);
        self.bump(); // crescendo or diminuendo
        self.expect(SyntaxKind::ToKw, "`to`");
        self.expect(SyntaxKind::Identifier, "a dynamic such as `f`");
        self.block();
        self.finish();
    }

    /// `section "Exposition" at 1:1;` — a form marker in the score.
    pub(super) fn section_stmt(&mut self) {
        self.start(SyntaxKind::SectionStmt);
        self.bump(); // section
        self.expect(SyntaxKind::String, "a section name in quotes");
        self.expect(SyntaxKind::AtKw, "`at`");
        self.position();
        self.expect(SyntaxKind::Semicolon, "`;`");
        self.finish();
    }

    /// `dynamic <mark>;` — the marking applies from the next event on.
    pub(super) fn dynamic_stmt(&mut self) {
        self.start(SyntaxKind::DynamicStmt);
        self.bump(); // dynamic
        self.expect(SyntaxKind::Identifier, "a dynamic marking such as `p` or `mf`");
        self.expect(SyntaxKind::Semicolon, "`;`");
        self.finish();
    }

    /// `stretch <n>/<d> { ... }` — the block, its durations multiplied.
    pub(super) fn stretch_stmt(&mut self) {
        self.start(SyntaxKind::StretchStmt);
        self.bump(); // stretch
        if self.at_any(&[SyntaxKind::Rational, SyntaxKind::Integer]) {
            self.bump();
        } else {
            self.expected("a factor such as `3/2` or `2`");
        }
        self.block();
        self.finish();
    }

    /// `retrograde { ... }` — the block, backwards.
    pub(super) fn retrograde_stmt(&mut self) {
        self.start(SyntaxKind::RetrogradeStmt);
        self.bump(); // retrograde
        self.block();
        self.finish();
    }

    /// `invert around <pitch> { ... }` — the block, mirrored about a pitch.
    pub(super) fn invert_stmt(&mut self) {
        self.start(SyntaxKind::InvertStmt);
        self.bump(); // invert
        self.expect(SyntaxKind::AroundKw, "`around`");
        self.expect(SyntaxKind::PitchLiteral, "the axis pitch, such as `c5`");
        self.block();
        self.finish();
    }

    /// `tuplet <n>/<d> { ... }` — `n` written values in the time of `d`.
    pub(super) fn tuplet_stmt(&mut self) {
        self.start(SyntaxKind::TupletStmt);
        self.bump(); // tuplet
        self.expect(SyntaxKind::Rational, "a tuplet ratio such as `3/2`");
        self.block();
        self.finish();
    }
}
