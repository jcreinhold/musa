//! Repetition lowering: `repeat`, `passes`, `brackets`, and `ending` unroll written repetition.

use musa_calculus::{Origin, Raw};
use musa_language::ast::AstNode as _;
use musa_language::{SyntaxKind, SyntaxNode};
use num_rational::Ratio;

use crate::lower::{Lowering, applied, whole};
use musa_score::diagnose::{Code, Diagnostic};
use musa_score::origin::SourceSpan;

use super::*;
impl Lowering<'_> {
    /// `repeat 2 { … }` and `repeat 4 to 16 { … }` — every pass, written out,
    /// under one fact that says it was written once.
    ///
    /// # Why the passes are folded rather than left to a reading
    ///
    /// `../../../rules/kernel/06-surface-elaboration.md` §2 makes `repeat n { … }`
    /// "an HIR-level `follow` of `n` evaluations", each iteration's occurrences
    /// carrying a [`musa_score::origin::ExpansionStep::RepeatIteration`] step. That is
    /// not a convenience: a timeline holding one pass is a *different piece of
    /// music* — it is half a bar long where the piece is a bar and a half — and
    /// every consumer that measures rather than draws would read the short one.
    /// [`crate::project`]'s own reader says so out loud: it takes a repeat's body
    /// to end one pass in, `(end − start) / times`, which is an arithmetic
    /// identity on the unrolled span and nonsense on a folded one.
    ///
    /// The page still prints `|:` `:|` rather than three copies, and that is what
    /// the `Fact.Repeat` region over the whole is for — one statement, two
    /// projections, roadmap §2's own example. What the layer table forbids is
    /// letting the *drawing* decide how long the music is.
    ///
    /// # Why the body is bound rather than copied
    ///
    /// One `let` per repeat, referenced once per pass. The passes differ only in
    /// the expansion step stamped on them, so a term written out `n` times would
    /// be `n` copies of one reading for the evaluator to walk — §06's "Sharing
    /// and provenance" is exactly this, and the mark on each reference is what
    /// tells the passes apart (T6).
    ///
    /// # The ranged form
    ///
    /// `repeat 4 to 16` is the piece leaving the count to the performance, and it
    /// is decided *here*, before a term exists. Everything below this line is the
    /// ordinary exact repeat, which is the whole of
    /// `../../../rules/kernel/11-realization.md`'s design in one place.
    pub(crate) fn repeat(&mut self, node: &SyntaxNode, origin: Origin, reading: Reading) -> Option<Raw> {
        let statement = musa_language::ast::RepeatStmt::cast(node.clone())?;
        let span = crate::resolve::trimmed_span(node);
        let (times, range) = self.passes(&statement, span)?;
        let brackets = self.brackets(&statement, times);
        // A repeat nobody plays is silence, and saying so here keeps every
        // arithmetic below over a positive count.
        if times == 0 {
            return Some(Raw::lit(origin, crate::registry::empty_track()));
        }
        // Folded once each, before any pass is built: a body read `times` over
        // would report every diagnostic inside it `times` over, and speak every
        // name it uses that many times.
        let inside = reading.again();
        let body = self.folded(node, statements(node).filter(is_not_an_ending), inside);
        let played: Option<Vec<Raw>> = brackets
            .iter()
            .map(|bracket| self.notated(bracket.syntax(), inside))
            .collect();
        let (body, played) = (body?, played?);
        let ending_extents: Vec<Ratio<i64>> = brackets.iter().map(|held| self.extent(held.syntax())).collect();
        let over = spanned(
            self.reached(statements(node).filter(is_not_an_ending)),
            &ending_extents,
            times,
        );
        let body_name = self.mint("pass");
        let ending_names: Vec<String> = (0..brackets.len()).map(|_| self.mint("ending")).collect();

        let mut passes = Vec::with_capacity(times as usize);
        for iteration in 0..times {
            let taken = Self::expanded(origin, span, iteration, Raw::var(origin, body_name.clone()));
            passes.push(taken);
            // Fewer endings than passes is legal: the last one covers the rest,
            // which is what `1.–3.` means on a volta bracket.
            let Some(index) = last_at_most(iteration, ending_names.len()) else {
                continue;
            };
            let Some(name) = ending_names.get(index) else {
                continue;
            };
            let bracket = u32::try_from(index.saturating_add(1)).unwrap_or(u32::MAX);
            let fact = applied(
                origin,
                Raw::hosted(origin, "Fact.Ending"),
                [
                    whole(origin, u64::from(bracket)),
                    whole(origin, u64::from(iteration.saturating_add(1))),
                ],
            );
            let marker = self.sounded(
                origin,
                reading,
                fact,
                ending_extents.get(index).copied().unwrap_or_default(),
            );
            let taken = Self::expanded(origin, span, iteration, Raw::var(origin, name.clone()));
            passes.push(applied(origin, Raw::hosted(origin, "together"), [marker, taken]));
        }

        let range = range.map(|(least, most)| {
            applied(
                origin,
                Raw::hosted(origin, "Pair.Both"),
                [whole(origin, u64::from(least)), whole(origin, u64::from(most))],
            )
        });
        let fact = applied(
            origin,
            Raw::hosted(origin, "Fact.Repeat"),
            [whole(origin, u64::from(times)), maybe(origin, range)],
        );
        let marker = self.sounded(origin, reading, fact, over);
        let whole_repeat = applied(
            origin,
            Raw::hosted(origin, "together"),
            [marker, followed(origin, passes)],
        );
        // The bindings outermost, so a reference inside any pass is in scope:
        // the body first, then the endings in the order they were written.
        let bound = ending_names
            .into_iter()
            .zip(played)
            .rev()
            .fold(whole_repeat, |inner, (name, ending)| {
                Raw::bind(origin, name, ending, inner)
            });
        Some(Raw::bind(origin, body_name, body, bound))
    }

    /// How many times a repeat plays, and the range it was written with.
    ///
    /// The count is asked once and remembered, because asking the realization
    /// twice would number two decision sites where the piece wrote one — see
    /// [`crate::lower::Lowering::counts`].
    pub(crate) fn passes(
        &mut self,
        statement: &musa_language::ast::RepeatStmt,
        span: SourceSpan,
    ) -> Option<(u32, Option<(u32, u32)>)> {
        let text = statement.count().unwrap_or_default();
        let Some(least) = count_of(&text) else {
            return self.refuse(Self::not_a_count(&text, span));
        };
        let Some(written) = statement.most() else {
            return Some((least, None));
        };
        let Some(most) = count_of(&written).filter(|most| *most >= least) else {
            return self.refuse(
                Diagnostic::error(Code::NotAValue, "a repeat range counts upwards")
                    .at(span, format!("`{least} to {}` never happens", written.trim()))
                    .help("write the smaller number first"),
            );
        };
        let count = self.resolver.decide_count(&self.choice, least, most, span).1;
        self.counts.insert(span, count);
        Some((count, Some((least, most))))
    }

    /// The endings a repeat writes, in order, with what is wrong with them said.
    ///
    /// Three complaints and one answer: the endings are returned whatever was
    /// said about them, because a mis-numbered volta is still a volta and
    /// dropping it would answer a shorter piece than the one written.
    pub(crate) fn brackets(
        &mut self,
        statement: &musa_language::ast::RepeatStmt,
        times: u32,
    ) -> Vec<musa_language::ast::EndingStmt> {
        let mut endings: Vec<musa_language::ast::EndingStmt> = Vec::new();
        // The last ending written so far, until something that is not an ending
        // follows it — which is the one thing about their placement that is
        // wrong: every pass plays the body and then its ending, so music after
        // an ending belongs to no pass.
        let mut open: Option<musa_language::ast::EndingStmt> = None;
        for child in statements(statement.syntax()) {
            let Some(written) = musa_language::ast::EndingStmt::cast(child.clone()) else {
                if let Some(before) = open.take() {
                    self.refuse::<()>(
                        Diagnostic::error(Code::Misplaced, "an ending is the last thing in a repeat")
                            .at(
                                crate::resolve::trimmed_span(before.syntax()),
                                "music is written after this",
                            )
                            .help("move the endings below everything the passes have in common")
                            .note("every pass plays the body, then its ending, so the body comes first"),
                    );
                }
                continue;
            };
            let at = crate::resolve::token_span(written.syntax(), SyntaxKind::Integer)
                .unwrap_or_else(|| crate::resolve::trimmed_span(written.syntax()));
            let expected = u32::try_from(endings.len().saturating_add(1)).unwrap_or(u32::MAX);
            let numbered = written.number().and_then(|text| count_of(&text)).unwrap_or_default();
            if numbered != expected {
                self.refuse::<()>(
                    Diagnostic::error(
                        Code::Misplaced,
                        format!("this ending is pass {expected}, not pass {numbered}"),
                    )
                    .at(at, format!("expected `ending {expected}`"))
                    .help("number the endings from 1, in the order they are played"),
                );
            }
            if expected > times {
                self.refuse::<()>(
                    Diagnostic::error(Code::Misplaced, format!("this repeat never reaches pass {expected}"))
                        .at(at, "no pass plays this")
                        .help(format!("write `repeat {expected}`, or delete this ending")),
                );
            }
            open = Some(written.clone());
            endings.push(written);
        }
        endings
    }

    /// `track`, recorded as the `iteration`-th time through a repeat.
    ///
    /// [`stamped`] for its documented reason, which is sharpest here: a repeat
    /// reads its body once and every pass references that one binding, so a step
    /// applied while reading would be one step written on material the passes
    /// hold in common.
    pub(crate) fn expanded(origin: Origin, span: SourceSpan, iteration: u32, track: Raw) -> Raw {
        stamped(
            origin,
            span,
            musa_score::origin::ExpansionStep::RepeatIteration(iteration),
            track,
        )
    }

    /// `ending 1 { … }` standing outside a repeat.
    ///
    /// A repeat reads its own endings — [`Lowering::repeat`] has to, since which
    /// passes a bracket covers is not a property of the bracket — so what reaches
    /// here is an ending with no repeat above it. It is read where it stands
    /// rather than refused: the old checker refused it because it *expanded*
    /// repeats and an ending with no pass to belong to had nowhere to go, and the
    /// fact this writes says which bracket it is and that it was played once.
    pub(crate) fn ending(&mut self, node: &SyntaxNode, origin: Origin, reading: Reading) -> Option<Raw> {
        let statement = musa_language::ast::EndingStmt::cast(node.clone())?;
        let span = crate::resolve::trimmed_span(node);
        let text = statement.number().unwrap_or_default();
        let Some(bracket) = count_of(&text) else {
            return self.refuse(Self::not_a_count(&text, span));
        };
        let fact = applied(
            origin,
            Raw::hosted(origin, "Fact.Ending"),
            [whole(origin, u64::from(bracket)), whole(origin, u64::from(bracket))],
        );
        self.region(node, origin, reading, fact)
    }

    /// `bar { … }` and `bar refrain { … }` — a measure, and what it claims.
    ///
    /// The braces erase. A bar contributes no occurrence, no payload, and no
    /// time of its own — the kernel's ontology has no bar in it, and where the
    /// barlines fall is [`musa_score::BarLines`]'s answer over the meters — so the
    /// term a bar denotes is exactly the fold of what is inside it. What the
    /// braces contribute is the claim that the music between them fills one
    /// measure of the meter in force where they stand, and that is a
    /// [`Claimed`] rather than a fact.
    ///
    /// The name is not read here. `bar refrain { … }` binds a passage another
    /// voice can answer, which is a *declaration* in the enclosing document
    /// rather than a statement in this block, and reading it here would put the
    /// same name in scope once per voice that mentions it.
    pub(crate) fn bar(&mut self, node: &SyntaxNode, origin: Origin, reading: Reading) -> Option<Raw> {
        let statement = musa_language::ast::BarStmt::cast(node.clone())?;
        let passage = self.notated(node, reading)?;
        self.claims.push(Claimed {
            predicate: musa_score::assert::predicate("fills_meter")?,
            arguments: Vec::new(),
            span: crate::resolve::trimmed_span(node),
            content_end: statement.content_end(),
            noun: "bar",
            // Nothing yet: the fold this bar stands in prepends what comes
            // before it, and so does every fold above that one.
            before: Raw::lit(origin, crate::registry::empty_track()),
            passage: passage.clone(),
        });
        Some(passage)
    }
}
