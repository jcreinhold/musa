//! Extent: how long a written passage holds — `reached`, `lasts`, `played`.

use musa_syntax::ast::AstNode as _;
use musa_syntax::{SyntaxKind, SyntaxNode};
use num_rational::Ratio;

use crate::lower::Lowering;

use super::*;
impl Lowering<'_> {
    pub(crate) fn extent(&self, node: &SyntaxNode) -> Ratio<i64> {
        self.reached(statements(node))
    }

    /// The same sum over a chosen subsequence, which is what a repeat's body is.
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "`Ratio<i64>` addition is exact mathematical arithmetic rather than raw integer ops; `extent` measures written durations"
    )]
    pub(crate) fn reached(&self, statements: impl Iterator<Item = SyntaxNode>) -> Ratio<i64> {
        let mut total = Ratio::ZERO;
        for statement in statements {
            total += self.lasts(&statement);
        }
        total
    }

    /// How long one statement lasts.
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "`Ratio<i64>` multiplication is exact mathematical arithmetic rather than raw integer ops; `extent` reads a tuplet's written ratio"
    )]
    pub(crate) fn lasts(&self, statement: &SyntaxNode) -> Ratio<i64> {
        match statement.kind() {
            SyntaxKind::NoteStmt
            | SyntaxKind::RestStmt
            | SyntaxKind::ChordStmt
            | SyntaxKind::StackStmt
            | SyntaxKind::ImproviseStmt => {
                // The decided length first, because `c5/4 to 2/1` is drawn as a
                // quarter and *sounds* whatever [`Lowering::held`] chose, and
                // what a region has to measure is the sounding. Asking the
                // realization again would mint a second site — the same rule
                // [`Lowering::times`] follows for a ranged repeat.
                let span = crate::resolve::trimmed_span(statement);
                if let Some(sounds) = self.holds.get(&span) {
                    return *sounds;
                }
                crate::resolve::parse_duration(statement).map_or(Ratio::ZERO, |written| written.value.as_ratio())
            }
            // The one statement whose length is not its body's. Named here
            // rather than left to the recursion because a repeat plays its body
            // once per pass, and a sum that counted it once would measure a
            // three-pass repeat as one.
            SyntaxKind::RepeatStmt => self.played(statement),
            // The other one, and for the mirror-image reason: a tuplet plays its
            // body in less time than the body writes. Named here rather than
            // left to the recursion so that the ratio is read off the statement
            // that wrote it — which makes this a measurement of the tree, and
            // lets a region enclosing a tuplet ask how far it reaches without
            // the tuplet's body having been lowered first.
            SyntaxKind::TupletStmt => self.extent(statement) * tuplet_factor(statement),
            _ => self.extent(statement),
        }
    }

    /// How long every pass of a repeat lasts, endings included.
    pub(crate) fn played(&self, node: &SyntaxNode) -> Ratio<i64> {
        let endings = endings_of(node);
        spanned(
            self.reached(statements(node).filter(is_not_an_ending)),
            &endings
                .iter()
                .map(|held| self.extent(held.syntax()))
                .collect::<Vec<_>>(),
            self.times(node),
        )
    }

    /// How many passes the repeat at `node` plays.
    ///
    /// The written number for the exact form; for a ranged one, the count the
    /// realization already chose. [`Lowering::passes`] made that decision and
    /// kept it exactly so this can be asked without making a second one.
    pub(crate) fn times(&self, node: &SyntaxNode) -> u32 {
        let span = crate::resolve::trimmed_span(node);
        if let Some(decided) = self.counts.get(&span) {
            return *decided;
        }
        musa_syntax::ast::RepeatStmt::cast(node.clone())
            .and_then(|statement| statement.count())
            .and_then(|text| count_of(&text))
            .unwrap_or_default()
    }
}
