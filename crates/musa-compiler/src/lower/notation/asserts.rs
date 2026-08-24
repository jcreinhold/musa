//! Asserts: `asserted` and the `claimed_predicate`/`claim_arguments` pair that report what a claim refused.

use musa_calculus::{Origin, Raw};
use musa_syntax::SyntaxNode;
use musa_syntax::ast::AstNode as _;

use crate::lower::{Lowering, applied, child, is_expr_node};
use musa_score::diagnose::{Code, Diagnostic};

use super::*;
impl Lowering<'_> {
    /// `assert pitches_in(scale c major) { … }` — a claim, and the passage it
    /// is about.
    ///
    /// The braces erase for [`Self::bar`]'s reason and the same [`Claimed`] is
    /// pushed; what an `assert` adds is a claim the composer chose and the
    /// arguments they chose it with. Those arguments are read here rather than
    /// where the claim is proved because this is where they are *written*: an
    /// unknown claim, a wrong count of arguments, and a word that names no
    /// policy are all mistakes in the source, and a pass that met them after
    /// elaboration would have to invent a span to report them at.
    ///
    /// What is *not* read here is any argument's value. An expression has none
    /// until the document it stands in is elaborated, so the reading annotates
    /// it with the type its shape declares and records the term —
    /// [`crate::document::Document::passage`] evaluates it and
    /// [`crate::registry::argument`] reads it back.
    ///
    /// The one trace an assertion leaves in the music is
    /// [`musa_score::origin::ExpansionStep::Assertion`] on the facts inside the
    /// braces, which is Origin and therefore invisible to `≈facts` — the
    /// identity `05-verification.md` asks for, that a claim which holds gives
    /// back exactly the passage it was written on. The step carries the claim
    /// *as the source spells it*, for [`Self::under_scale`]'s reason: `pitches_in`
    /// alone would tell a reader an assertion was here and not which one, and
    /// the value the arguments have is a question this reading cannot answer.
    pub(crate) fn asserted(&mut self, node: &SyntaxNode, origin: Origin, reading: Reading) -> Option<Raw> {
        let statement = musa_syntax::ast::AssertStmt::cast(node.clone())?;
        let predicate = self.claimed_predicate(&statement, node)?;
        let arguments = self.claim_arguments(predicate, &statement, node)?;
        let span = crate::resolve::trimmed_span(node);
        let passage = self.notated(node, reading)?;
        self.claims.push(Claimed {
            predicate,
            arguments,
            span,
            content_end: statement.content_end(),
            noun: "passage",
            // Nothing yet, exactly as a bar records nothing: the fold this
            // assertion stands in prepends what comes before it.
            before: Vec::new(),
            // Without the step, because a claim is proved against what sounds
            // and provenance is not part of that. The music the *voice* gets
            // carries it.
            passage: passage.clone(),
        });
        Some(stamped(
            origin,
            span,
            musa_score::origin::ExpansionStep::Assertion {
                claim: spelled_claim(predicate, &statement),
            },
            passage,
        ))
    }

    /// The registry row `statement` names, or a refusal that lists the family.
    ///
    /// The whole family in the note rather than only the nearest spelling,
    /// because `CLAIMS` is six rows and a composer who misremembered one is
    /// better served by reading all six than by being guessed at.
    pub(crate) fn claimed_predicate(
        &mut self,
        statement: &musa_syntax::ast::AssertStmt,
        node: &SyntaxNode,
    ) -> Option<&'static musa_score::assert::Predicate> {
        let name = statement.claim().unwrap_or_default();
        if let Some(predicate) = musa_score::assert::predicate(&name) {
            return Some(predicate);
        }
        let known: Vec<&str> = musa_score::assert::names().collect();
        self.refuse(
            Diagnostic::error(Code::UnknownName, format!("nothing is claimed by `{name}`"))
                .at(claim_span(statement, node), "not a claim musa can prove")
                .maybe_help(
                    musa_score::diagnose::nearest(&name, known.iter().copied())
                        .map(|near| format!("did you mean `{near}`?")),
                )
                .note(format!("the claims are: {}", known.join(", "))),
        )
    }

    /// Every argument `statement` writes, in the two states [`Argued`] has.
    ///
    /// The arity is checked first and the whole statement is refused when it is
    /// wrong, because an argument read against the wrong parameter would be
    /// refused for a reason that is not the mistake: `voices(scale c major)`
    /// written with one argument too many should say so once, not report a
    /// scale where a count was wanted.
    pub(crate) fn claim_arguments(
        &mut self,
        predicate: &'static musa_score::assert::Predicate,
        statement: &musa_syntax::ast::AssertStmt,
        node: &SyntaxNode,
    ) -> Option<Vec<Argued>> {
        use musa_score::assert::{Argument, ParamType};

        let written = statement.args();
        if written.len() != predicate.parameters.len() {
            let wanted: Vec<&str> = predicate
                .parameters
                .iter()
                .map(|parameter| parameter.as_str())
                .collect();
            let name = predicate.name;
            return self.refuse(
                Diagnostic::error(
                    Code::WrongArity,
                    format!(
                        "`{name}` takes {}, and {} written",
                        musa_score::assert::spell_arguments(predicate.parameters.len()),
                        musa_score::assert::spell_written(written.len())
                    ),
                )
                .at(claim_span(statement, node), "this claim's arguments do not match it")
                .help(if wanted.is_empty() {
                    format!("`{name}()` — it reads the passage and needs nothing else")
                } else {
                    format!("`{name}({})`", wanted.join(", "))
                })
                .note(predicate.checks),
            );
        }
        let mut arguments = Vec::with_capacity(written.len());
        for (argument, shape) in written.iter().zip(predicate.parameters) {
            let written = argument.syntax();
            let origin = self.origin(written);
            let named = |name: &str| Raw::var(origin, name);
            arguments.push(match *shape {
                ParamType::Policy => Argued::Word(Argument::Policy(self.policy(written)?)),
                ParamType::Rule => Argued::Word(Argument::Rule(self.rule_named(written)?)),
                ParamType::Scale => self.claim_value(written, origin, named("Scale"))?,
                ParamType::Chord => self.claim_value(written, origin, named("ChordClass"))?,
                ParamType::Count => self.claim_value(written, origin, named("Nat"))?,
                // The one applied type among the six, and the reason the shape
                // decides the annotation rather than a name: a range is a pair
                // of written pitches and a claim reads one per voice.
                ParamType::Ranges => {
                    let range = applied(origin, named("Pair"), [named("Pitch"), named("Pitch")]);
                    self.claim_value(written, origin, applied(origin, named("List"), [range]))?
                }
            });
        }
        Some(arguments)
    }

    /// One argument that is a value, checked at the type its shape declares.
    ///
    /// Annotated rather than inferred, so that an argument of the wrong type is
    /// a conversion the core refuses against the type the registry declares —
    /// one answer to "what may stand here", from the declaration, rather than a
    /// second table of expected types beside it.
    pub(crate) fn claim_value(&mut self, node: &SyntaxNode, origin: Origin, ty: Raw) -> Option<Argued> {
        let written = child(node, is_expr_node)?;
        let term = self.expr(&written)?;
        Some(Argued::Value(Raw::annot(origin, term, ty)))
    }

    /// One of [`musa_score::assert::Realization`]'s three words.
    pub(crate) fn policy(&mut self, node: &SyntaxNode) -> Option<musa_score::assert::Realization> {
        let word = word(node);
        if let Some(policy) = musa_score::assert::Realization::named(&word) {
            return Some(policy);
        }
        let spellings: Vec<&str> = musa_score::assert::Realization::ALL
            .iter()
            .map(|policy| policy.as_str())
            .collect();
        self.refuse(
            Diagnostic::error(Code::UnknownWord, format!("`{word}` is not a realization policy"))
                .at(crate::resolve::trimmed_span(node), "expected one of three words")
                .maybe_help(
                    musa_score::diagnose::nearest(&word, spellings.iter().copied())
                        .map(|near| format!("did you mean `{near}`?")),
                )
                .note(
                    "`exactly` is set equality, `may_omit` lets a member be missing, \
                     and `may_add` lets other notes sound",
                ),
        )
    }

    /// The id of a voice-leading rule an assertion may name.
    pub(crate) fn rule_named(&mut self, node: &SyntaxNode) -> Option<musa_score::analysis::RuleName> {
        let word = word(node);
        if let Some(rule) = musa_score::analysis::assertable().find(|rule| rule.id() == word) {
            return Some(rule);
        }
        let assertable: Vec<&str> = musa_score::analysis::assertable().map(|rule| rule.id()).collect();
        self.refuse(
            Diagnostic::error(
                Code::UnknownWord,
                format!("`{word}` is not a rule this claim can check"),
            )
            .at(
                crate::resolve::trimmed_span(node),
                "expected the id of a voice-leading rule",
            )
            .maybe_help(
                musa_score::diagnose::nearest(&word, assertable.iter().copied())
                    .map(|near| format!("did you mean `{near}`?")),
            )
            .help(format!("the rules a source may assert are: {}", assertable.join(", ")))
            .note(
                "every other rule is reported by `musa analyze --kind voice-leading`, \
                 which says how strongly a style holds it rather than failing the build",
            ),
        )
    }
}
