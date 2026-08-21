//! Quotes, splices, and events holes.

use super::engine::Parser;
use crate::SyntaxKind;

impl Parser<'_> {
    /// `events EventTrack[WrittenTime, ScoreFact] { … }` — a quoted composition
    /// expression (`docs/rules/language/01-surface.md` §7).
    ///
    /// **Recognized, not read.** The tokens between the braces spell the
    /// event track's own grammar, and `musa-events` owns that grammar: a second
    /// reading of it here would be a second thing to keep in step with the
    /// first. What this crate must find is the shape — where the quote ends,
    /// and where the holes are — because those are the two questions a
    /// lossless tree and an editor ask. The words in between are handed
    /// along as source text.
    ///
    /// The braces are counted rather than matched against a production, so a
    /// `track … { … }` inside the quote does not end it, and a quote that
    /// is never closed ends at the end of the file rather than eating the
    /// declaration after it.
    ///
    /// The head takes two type arguments because an event track is indexed by
    /// both its coordinate and its payload: a quote must say which time it is
    /// written in, since nothing converts one coordinate into another.
    pub(super) fn events_quote(&mut self) {
        self.start(SyntaxKind::EventsQuote);
        self.bump(); // events
        self.expect(SyntaxKind::Identifier, "`EventTrack`");
        self.expect(SyntaxKind::LBracket, "`[`");
        self.expect(SyntaxKind::Identifier, "a coordinate");
        self.expect(SyntaxKind::Comma, "`,`");
        self.expect(SyntaxKind::Identifier, "a payload type");
        self.expect(SyntaxKind::RBracket, "`]`");
        self.expect(SyntaxKind::LBrace, "`{`");
        let mut depth = 0_usize;
        loop {
            match self.current() {
                None => break,
                Some(SyntaxKind::RBrace) if depth == 0 => break,
                Some(SyntaxKind::RBrace) => {
                    depth = depth.saturating_sub(1);
                    self.bump();
                }
                Some(SyntaxKind::LBrace) => {
                    depth = depth.saturating_add(1);
                    self.bump();
                }
                // Every `$` in a quote is a hole attempted: the event track's
                // grammar has no other use for the character, so reading it
                // as one and complaining about what follows says more than
                // "unexpected token" would.
                Some(SyntaxKind::Dollar) => self.events_hole(),
                Some(_) => self.bump(),
            }
        }
        self.expect(SyntaxKind::RBrace, "`}`");
        self.finish();
    }

    /// `${ expr }` — one typed antiquotation, whose interior is host syntax.
    pub(super) fn events_hole(&mut self) {
        self.start(SyntaxKind::EventsHole);
        self.bump(); // $
        self.expect(SyntaxKind::LBrace, "`{`");
        self.expr();
        self.expect(SyntaxKind::RBrace, "`}`");
        self.finish();
    }

    /// `quote at here { … }` — the other quotation, whose body this parser
    /// reads (`docs/rules/language/11-quotation.md` §2).
    ///
    /// The whole of the difference from [`Self::events_quote`] is right here:
    /// that one counts braces and hands the interior along as text, because
    /// the interior is another crate's grammar. This one calls
    /// [`Self::expr`], because the interior is *this* grammar. A quote that
    /// did not share the parser would be a second grammar to keep in step
    /// with the first, and it is the sublanguage-by-subtraction `AGENTS.md`
    /// forbids: everything the dialect lacked would be paid by every adapter
    /// author instead of once here. Sharing it is also what makes the body
    /// formatted, highlighted, and diagnosed by the machinery the file around
    /// it uses, with no second answer to what an expression is.
    ///
    /// The anchor is read at the operand level rather than by [`Self::expr`],
    /// so that the brace after it opens the body and cannot be mistaken for
    /// anything else. `here`, `spot`, `read.head.spot` and `anchor_of(x)` are
    /// all operands; an anchor that needed arithmetic is written in a `let`
    /// above, which is where a computed value belongs anyway.
    pub(super) fn quote_expr(&mut self) {
        self.start(SyntaxKind::QuoteExpr);
        self.bump(); // quote
        self.expect(SyntaxKind::AtKw, "`at`");
        self.operand_expr(true);
        self.expect(SyntaxKind::LBrace, "`{`");
        self.quote_depth = self.quote_depth.saturating_add(1);
        self.expr();
        self.quote_depth = self.quote_depth.saturating_sub(1);
        self.expect(SyntaxKind::RBrace, "`}`");
        self.finish();
    }

    /// `quote { … }` in a pattern — the inverse form
    /// (`docs/rules/language/11-quotation.md` §4).
    ///
    /// The same body production as [`Self::quote_expr`], and no anchor. A
    /// pattern derives no identity because it builds nothing, so there is
    /// nothing for an `at` to point at; writing one would be a value the form
    /// has no use for, and reading it would be the one thing §4 forbids —
    /// a pattern that could speak about where a node came from.
    pub(super) fn quote_pattern(&mut self) {
        self.start(SyntaxKind::QuotePattern);
        self.bump(); // quote
        self.expect(SyntaxKind::LBrace, "`{`");
        self.quote_depth = self.quote_depth.saturating_add(1);
        self.expr();
        self.quote_depth = self.quote_depth.saturating_sub(1);
        self.expect(SyntaxKind::RBrace, "`}`");
        self.finish();
    }

    /// `$x`, `${ e }`, or `$..xs` — one splice, where one node stands.
    ///
    /// All three are read here because all three are one decision made after
    /// the `$`, and reading them apart would mean three lookahead tests where
    /// the character has already said what is coming. `$..xs` is a node like
    /// the others as far as the grammar is concerned; whether the position it
    /// stands in admits a *sequence* is a question about that position, and
    /// the elaborator asks it where the position is known.
    ///
    /// The splice's own expression is read with the quote closed, because it
    /// is host code: `${ dot_count(here, dots) }` is an ordinary call, and a
    /// `$` written inside it belongs to whatever quote is written *there*.
    pub(super) fn splice(&mut self) {
        let sequence =
            self.nth_significant(1) == Some(SyntaxKind::Dot) && self.nth_significant(2) == Some(SyntaxKind::Dot);
        self.start(if sequence {
            SyntaxKind::SequenceSplice
        } else {
            SyntaxKind::Splice
        });
        self.bump(); // $
        if sequence {
            self.bump(); // .
            self.bump(); // .
            // Wrapped, exactly as the shorthand below is: a spread names a
            // list, and a list is an ordinary value of the enclosing scope.
            // Both splice forms therefore hold one expression, and nothing
            // downstream needs a second way to reach a spliced name.
            self.start(SyntaxKind::NameExpr);
            self.expect(SyntaxKind::Identifier, "the name of a list to splice");
            self.finish();
        } else if self.at(SyntaxKind::LBrace) {
            let outer = std::mem::take(&mut self.quote_depth);
            self.bump();
            self.expr();
            self.expect(SyntaxKind::RBrace, "`}`");
            self.quote_depth = outer;
        } else {
            // The shorthand, and the reason it is only a shorthand: a name is
            // the one expression that needs no braces to say where it ends.
            self.start(SyntaxKind::NameExpr);
            self.expect(SyntaxKind::Identifier, "a name to splice, or `{`");
            self.finish();
        }
        self.finish();
    }
}
