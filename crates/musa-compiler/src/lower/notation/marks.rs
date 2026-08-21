//! Marking lowering: `marked` and `mark_argument` turn marks and articulations into payloads.

use musa_calculus::{Origin, Raw};
use musa_language::SyntaxNode;
use musa_language::ast::AstNode as _;
use num_rational::Ratio;

use crate::lower::{Lowering, applied, listed};
use musa_score::diagnose::{Code, Diagnostic};
use musa_score::origin::SourceSpan;

use super::*;
impl Lowering<'_> {
    /// `mark breath;` and `mark pedal { … }`.
    ///
    /// The one annotation that is a point *or* a region depending on how it was
    /// written, which is exactly what [`crate::elaborate::FactKind::Mark`]'s own
    /// documentation says decides it: "which of the two this occurrence is, is
    /// its span — a point's is empty".
    ///
    /// # Why the row's shape is checked while reading
    ///
    /// `musa_score::marks`'s table says three things about each row — where it is
    /// anchored, what argument it takes, and which note slot it fills — and all
    /// three are claims about *how the statement was written*, which is the same
    /// rule [`Lowering::specialized`] states at length: a property of the text is
    /// answered where the text is. A `Mark` reaches the core as a payload
    /// literal, so by the time the term is evaluated `mark pedal;` and
    /// `mark pedal { … }` are one value with different spans and nothing left to
    /// refuse. [`Lowering::articulations`] answers the fourth question — a mark
    /// written on a note — for the same reason, from the other side.
    ///
    /// Four refusals, one per way the written shape can disagree with the row,
    /// and each names the row rather than the grammar: a vocabulary whose
    /// diagnostics were about braces would be a vocabulary with no shape.
    pub(crate) fn marked(&mut self, node: &SyntaxNode, origin: Origin, reading: Reading) -> Option<Raw> {
        use musa_score::marks::Anchor;

        let statement = musa_language::ast::MarkStmt::cast(node.clone())?;
        let span = crate::resolve::trimmed_span(node);
        let text = statement.name().unwrap_or_default();
        let Some(mark) = musa_score::Mark::parse(&text) else {
            return self.refuse(
                Diagnostic::error(Code::UnknownWord, format!("`{text}` is not a mark"))
                    .at(span, "unknown mark")
                    .help(crate::resolve::suggest(
                        &text,
                        &musa_score::marks::statement_names(),
                        "marks",
                    )),
            );
        };
        if mark.slot().is_some() {
            return self.refuse(
                Diagnostic::error(Code::Misplaced, format!("`{mark}` is written on a note"))
                    .at(span, "not a statement of its own")
                    .help(format!("write it after a duration: `g4 1/4 {mark}`")),
            );
        }
        let argument = self.mark_argument(mark, &statement, origin, span)?;
        let fact = applied(
            origin,
            Raw::hosted(origin, "Fact.Mark"),
            [plain(origin, "Mark", mark), argument],
        );
        match (mark.anchor(), statement.has_block()) {
            (Anchor::Span, true) => self.region(node, origin, reading, fact),
            (Anchor::Point, false) => Some(self.sounded(origin, reading, fact, Ratio::ZERO)),
            (Anchor::Span, false) => self.refuse(
                Diagnostic::error(Code::Misplaced, format!("`{mark}` covers music"))
                    .at(span, "no music under it")
                    .help(format!("wrap what it covers: `mark {mark} {{ … }}`")),
            ),
            (Anchor::Point, true) => self.refuse(
                Diagnostic::error(Code::Misplaced, format!("`{mark}` stands at one place"))
                    .at(span, "it covers nothing")
                    .help(format!("write it on its own: `mark {mark};`")),
            ),
            // A note-anchored mark returned above; this arm is here because the
            // match is total.
            (Anchor::Note(_), _) => None,
        }
    }

    /// The argument this `mark` statement wrote, as the field `Fact.Mark` takes.
    ///
    /// [`None`] with a refusal reported when what was written and what the row
    /// takes disagree. Agreement is a term rather than a `MarkArgument`, because
    /// the two ways to agree — the row takes nothing and nothing was written, or
    /// the row takes something and that something was written — are one answer
    /// to the caller and two only here. Handing back the built field keeps the
    /// distinction where the checking is.
    ///
    /// The example in the help is built from the row, because "write an
    /// argument" is advice and `mark rehearsal "A";` is a repair.
    pub(crate) fn mark_argument(
        &mut self,
        mark: musa_score::Mark,
        statement: &musa_language::ast::MarkStmt,
        origin: Origin,
        span: SourceSpan,
    ) -> Option<Raw> {
        use musa_score::marks::{Argument, MarkArgument};

        let written = match (statement.text(), statement.number()) {
            (Some(text), _) => Some(MarkArgument::Text(text)),
            (None, Some(number)) => number.parse().ok().map(MarkArgument::Number),
            (None, None) => None,
        };
        match (mark.takes(), written) {
            (Argument::None, None) => Some(maybe(origin, None)),
            (wanted, Some(given)) if wanted == given.kind() => {
                Some(maybe(origin, Some(plain(origin, "MarkArgument", given))))
            }
            (Argument::None, Some(_)) => self.refuse(
                Diagnostic::error(Code::Misplaced, format!("`{mark}` is written on its own"))
                    .at(span, "nothing follows the name"),
            ),
            (wanted, given) => {
                let wanted = match wanted {
                    Argument::Text => "a name in quotes",
                    Argument::Number => "a whole number",
                    Argument::None => "nothing",
                };
                let saw = if given.is_some() { "the wrong kind" } else { "nothing" };
                self.refuse(
                    Diagnostic::error(Code::Misplaced, format!("`{mark}` is written with {wanted}"))
                        .at(span, saw)
                        .help(format!("for example `mark {mark}{};`", written_argument(mark))),
                )
            }
        }
    }

    /// The articulation names written after a duration, as a `List Mark`.
    ///
    /// Half of "the anchor decides where a mark is written"; [`Lowering::marked`]
    /// is the other half. A row whose anchor is [`musa_score::marks::Anchor::Note`]
    /// may be written here and no other row may, because a staccato dot has no
    /// extent and a pedal has both an extent and an identity —
    /// [`crate::elaborate::FactKind::Mark`]'s own documentation draws that line.
    ///
    /// So a name in the wrong half of the table is told which half it is in,
    /// rather than "is not a mark", which would be a lie about a word the
    /// vocabulary contains. The help spells the row's whole shape, since a pedal
    /// needs a block and a rehearsal letter needs its letter: "write it as a
    /// statement" alone would be advice that does not compile.
    ///
    /// Reported and passed over rather than refused: an unusable articulation is
    /// one field of one note, and stopping the walk here would bury every other
    /// thing wrong with the voice behind a spelling mistake.
    pub(crate) fn articulations(&mut self, origin: Origin, names: &[String], span: SourceSpan) -> Raw {
        let mut marks = Vec::new();
        for name in names {
            match musa_score::Mark::parse(name) {
                Some(mark) if mark.slot().is_some() => marks.push(plain(origin, "Mark", mark)),
                Some(mark) => {
                    let statement = format!("mark {mark}{}{}", written_argument(mark), written_tail(mark));
                    self.resolver.report(
                        Diagnostic::error(Code::Misplaced, format!("`{name}` is not written on a note"))
                            .at(span, "this mark stands on its own")
                            .help(format!("write it as a statement: `{statement}`")),
                    );
                }
                None => {
                    self.resolver.report(
                        Diagnostic::error(Code::UnknownWord, format!("`{name}` is not a mark"))
                            .at(span, "unknown mark")
                            .help(crate::resolve::suggest(name, &musa_score::marks::note_names(), "marks")),
                    );
                }
            }
        }
        listed(origin, marks)
    }
}
