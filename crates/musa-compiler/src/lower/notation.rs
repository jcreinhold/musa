//! A notated block read as a track term.
//!
//! `docs/rules/language/00-semantics.md` §3 deleted the contextual `music`
//! stage, and with it the *cursor*: placement is no longer read from an ambient
//! context and applied as the evaluator walks, it "is applied by the enclosing
//! voice's left fold". This module is that fold one level down. A block is
//!
//! ```text
//! follow(follow(follow(nothing, s₁), s₂), …)
//! ```
//!
//! over its statements in source order, where each statement contributes the
//! track it denotes. There is no cursor here to delete, because a fold has none:
//! `follow` adds the durations and places the second track after the first, and
//! that *is* what putting one statement after another means (§3's first
//! composition equation, in the kernel's own word).
//!
//! # What a statement contributes
//!
//! One call, and which call is the only thing [`Lowering::statement`] decides:
//!
//! - a note is `sounded` with a `Note` fact, and so is each pitch of a written
//!   simultaneity `[c4 e4 g4]`, folded with `together` — a chord is several
//!   simultaneous `Note` facts, and that is all a bracketed one says;
//! - `stack c4 major/2` is `play`, and is the only statement that is: `play`
//!   takes a `Voicing`, a `Voicing` takes a `ChordClass`, and `stack` is where a
//!   composer writes one;
//! - a rest and every annotation — mark, slur, phrase, tuplet, dynamic, hairpin,
//!   section, harmony, ending, repeat, mobile, improvise, grace — is `sounded`
//!   with its own [`Fact`](crate::prelude) case;
//! - `use e;` is `e` itself, which is what makes §2's "checks that `e` is a
//!   written-time score track" a *typing* statement rather than a step: the core
//!   checks it against `EventTrack ⟨written⟩` because `follow` demands one, and
//!   the refusal arrives at the node this module numbered;
//! - a transformation block — `transpose`, `stretch`, `retrograde`, `invert` —
//!   is the matching track builtin applied to the fold of its body, which is why
//!   the block and function spellings of each are literally one call and their
//!   agreement is a law rather than a convention (§3).
//!
//! # Why the table is one function
//!
//! Twenty-nine node kinds in one `match` reads worse than twenty-nine methods
//! for about four lines and then reads better for the rest of the file: the
//! correspondence between a written statement and the fact it denotes is the
//! thing prompt 141j's mirroring law exists to protect, and a correspondence
//! spread over twenty-nine methods is one nobody can check by reading.
//!
//! # The reading context is a reader
//!
//! [`Reading`] travels *down* into a block and never back up. It carries the
//! scope every fact is constructed at and the scale `step` counts in, and it is
//! passed by value at each nesting rather than mutated, so "lexically scoped"
//! (§3) is a property of the code rather than a discipline. Two consequences are
//! worth stating because they are the ones a reader will look for:
//!
//! - **A pitch is resolved here, and the core never sees a scale.** `in scale`
//!   supplies [`Reading::scale`], `p step n` is finished against it by
//!   [`crate::scale::Frame`] — the same arithmetic the old checker ran — and
//!   what lands in the raw term is a `Pitch` literal. `01-surface.md` §2 says
//!   `in scale` "is lexical rather than captured" and that "an absent scale
//!   makes `step` a type-context diagnostic, not an implicit C-major choice";
//!   both are this function's shape. `in scale` emits no fact, so it is not a
//!   key signature and not a claim of modulation.
//! - **Scope is given, not discovered.** A `music { … }` value is "usable at
//!   several places" (§3), so it has no voice of its own and reads at
//!   [`crate::Scope::Piece`]. [`super::piece`] lowers a *voice's* body by passing
//!   the voice's scope down the same way, and nothing here changed to let it —
//!   which was prompt 141k's prediction and is now prompt 141p's evidence.
//!
//! Expression-position `step` is untouched: [`Lowering::value`] lowers `p step n`
//! to a method call whose scale prompt 142's migration supplies, because an
//! expression is not lexically inside a block and has no `in scale` to read.
//! Two positions, two readings, and neither is ambient in the core.
//!
//! # Where a failure goes
//!
//! Five of the eight track builtins are fallible, and so are `sounded` and
//! `play`: transposition can leave the representable range and a fact cannot
//! last for less than no time. `01-surface.md` §5 fixes what happens to that —
//! "an operation that can fail keeps its failing shape" — so every fallible call
//! this module writes is wrapped in the surface's own `?`, through the machinery
//! [`Lowering`] already carries for a written one, and the questions are drained
//! at the closing brace. A block therefore denotes
//! `Result<EventTrack ⟨written⟩, Text>`: the block is the operation and that is
//! its failing shape. [`Lowering::music`] says why the answer is delimited there
//! rather than at the enclosing function, and why the `Ok` is unconditional.

#[cfg(test)]
mod laws;

use musa_core::{Origin, Raw};
use musa_language::ast::AstNode as _;
use musa_language::{SyntaxKind, SyntaxNode};
use num_rational::Ratio;

use super::{Lowering, applied, child, children, is_expr_node, listed, significant_tokens, whole, writes};
use crate::diagnose::{Code, Diagnostic};
use crate::origin::SourceSpan;
use crate::score::NotatedDuration;

/// What a block reads and never writes.
///
/// Passed by value into every nesting, which is what makes `in scale`'s scope
/// lexical without a stack to push and pop. Small enough to copy: a scope is two
/// words and a scale is a tonic class and a collection.
#[derive(Clone, Copy)]
pub(super) struct Reading {
    /// The scope every fact built here is constructed at (§5.7 requires one).
    scope: crate::Scope,
    /// The collection `step` counts in, when an `in scale` lexically encloses.
    scale: Option<Counting>,
    /// Whether this block stands at one place in the piece.
    ///
    /// The one thing that decides whether a `key`, a `meter`, a `tempo`, or a
    /// `clef` may be written here. Not derivable from [`Reading::scope`]: a
    /// free `music { … }` value reads at [`crate::Scope::Piece`] and so does a
    /// piece's own header, and the difference between them is not what the fact
    /// is *about* but whether "from here onward" has a single here.
    placed: bool,
    /// The meter in force where this block begins.
    ///
    /// Read lexically, exactly as [`Reading::scale`] is, and for the same
    /// reason: `senza { … }` restores the meter that was already in force, and a
    /// fold has no cursor to ask. One statement reads it and one writes it, so
    /// what it costs is a copy of two `u32`s per nesting.
    meter: crate::score::Meter,
}

impl Reading {
    /// The reading a free-standing `music { … }` value is read under.
    ///
    /// [`crate::Scope::Piece`] because a fragment is "usable at several places"
    /// and so has no voice of its own, and no scale because `01-surface.md` §2
    /// refuses an implicit C major.
    fn free() -> Self {
        Self {
            scope: crate::Scope::Piece,
            scale: None,
            placed: false,
            meter: crate::score::Meter::default(),
        }
    }

    /// The reading a body written at one place in the piece is read under.
    pub(super) fn at(scope: crate::Scope) -> Self {
        Self {
            scope,
            scale: None,
            placed: true,
            meter: crate::score::Meter::default(),
        }
    }

    /// The same reading, counting steps in `scale`.
    const fn stepping(self, scale: Counting) -> Self {
        Self {
            scale: Some(scale),
            ..self
        }
    }

    /// The same reading, under `meter`.
    ///
    /// Called once by [`super::piece`] with the meter the piece's header states,
    /// and again by [`Lowering::notated`] at each `meter` a block writes — the
    /// two places a meter can come into force, and the only two.
    pub(super) const fn metered(self, meter: crate::score::Meter) -> Self {
        Self { meter, ..self }
    }
}

/// A claim written over a passage, waiting for the piece to place it.
///
/// Two *terms* rather than a position and a duration, because a fold has no
/// cursor. Where a passage starts is how long the music before it lasts, and how
/// long the passage is is how long it lasts itself; both are questions a normal
/// form answers and neither is known while the block is being read. Recording
/// the two terms therefore records the question instead of guessing at it, and
/// [`crate::document::Document::track`] turns each into the exact rational the
/// claim is checked against.
///
/// [`Claimed::before`] is built up by [`Lowering::notated`] as the claim rises
/// out of the blocks it was written in: each enclosing fold prepends the music
/// standing before the statement it came from, so no level needs to know how
/// deeply it is nested and no level is handed an absolute position it could get
/// wrong.
pub(crate) struct Claimed {
    /// Which claim is made. The registry's own row, so the name and the shapes
    /// its arguments must have are one fact rather than two that could disagree.
    pub(crate) predicate: &'static crate::assert::Predicate,
    /// Its arguments, in written order, as long as `predicate.parameters`.
    pub(crate) arguments: Vec<Argued>,
    /// Where the claim is written — the `assert`, or the `bar`.
    pub(crate) span: SourceSpan,
    /// Where an inserted rest would go: after the last thing written inside the
    /// braces, and absent when nothing is.
    pub(crate) content_end: Option<u32>,
    /// What the sentence naming the passage calls it: a `bar`, or the `passage`
    /// an `assert` was written on.
    pub(crate) noun: &'static str,
    /// The music standing before the passage. Its duration is where the passage
    /// begins, which is what a measure claim is measured against.
    pub(crate) before: Raw,
    /// The passage itself. Its duration is how long the passage lasts, and its
    /// occurrences are the notes a pitch or chord claim is proved against.
    pub(crate) passage: Raw,
}

/// One argument of a written claim, in the state the reading leaves it in.
///
/// Two cases because [`crate::assert::ParamType`] has two kinds in it. Four of
/// the six shapes are *values* — a scale, a chord, a count, a list of ranges —
/// and a value has no value until the document that wrote it is elaborated, so
/// what a block can record is the raw term and no more. The other two are
/// **words**: a realization policy and the id of a voice-leading rule are
/// spellings the registry reads, not terms the language can produce, so they are
/// resolved where they stand and their refusals are reported there.
///
/// Recording the difference rather than erasing it is what keeps
/// [`crate::registry::argument`] free of a case it could never meet: the reading
/// never hands it a word.
pub(crate) enum Argued {
    /// A word this reading already resolved.
    Word(crate::assert::Argument),
    /// An expression, annotated with the type its shape declares so that the
    /// core checks it rather than a second table beside the core.
    Value(Raw),
}

/// The collection an `in scale` put in force.
///
/// Two cases rather than one scale, because `in scale mode { … }` names a
/// collection a template was handed and a template's argument has no degrees
/// until an instance site supplies one. What the two share is everything
/// `in scale` itself does — it contributes no fact and only changes what a
/// `step` reads — so the difference surfaces at exactly one statement.
#[derive(Clone, Copy)]
enum Counting {
    /// A scale the source spelled, whose degrees this reading can count
    /// through at the note that asks.
    Written(crate::scale::Scale),
    /// A scale a binder supplies.
    Bound,
}

/// One of the four statements whose meaning is "from here onward".
///
/// A closed set rather than a `SyntaxKind`, because it is the set
/// `00-semantics.md` §3 names — "a block may not contain a key, meter, tempo, or
/// clef change" — and both readers of it want the same four and no others.
#[derive(Clone, Copy)]
pub(super) enum Context {
    /// `key g major;`
    Key,
    /// `meter 3/4;`
    Meter,
    /// `tempo 1/4 = 96;`
    Tempo,
    /// `clef bass;`
    Clef,
}

impl Context {
    /// What a refusal calls this, in the words a composer wrote it in.
    const fn what(self) -> &'static str {
        match self {
            Self::Key => "a key change",
            Self::Meter => "a meter change",
            Self::Tempo => "a tempo marking",
            Self::Clef => "a clef change",
        }
    }
}

/// Whether a node kind is one of the written notation statements.
///
/// The kinds [`musa_language::ast::VoiceItem`] admits, which is the grammar's
/// own answer to "what may stand in a block". Written out rather than derived
/// from the typed enum because this module reads nodes, and a kind the grammar
/// grows without a reading here should fail [`Lowering::statement`]'s table
/// rather than be silently skipped by the fold.
fn is_statement(kind: SyntaxKind) -> bool {
    matches!(
        kind,
        SyntaxKind::NoteStmt
            | SyntaxKind::RestStmt
            | SyntaxKind::ChordStmt
            | SyntaxKind::UseStmt
            | SyntaxKind::TransposeStmt
            | SyntaxKind::InScaleStmt
            | SyntaxKind::StackStmt
            | SyntaxKind::RepeatStmt
            | SyntaxKind::BarStmt
            | SyntaxKind::AssertStmt
            | SyntaxKind::SenzaStmt
            | SyntaxKind::EndingStmt
            | SyntaxKind::SlurStmt
            | SyntaxKind::DynamicStmt
            | SyntaxKind::TupletStmt
            | SyntaxKind::StretchStmt
            | SyntaxKind::RetrogradeStmt
            | SyntaxKind::InvertStmt
            | SyntaxKind::PhraseStmt
            | SyntaxKind::MarkStmt
            | SyntaxKind::GraceStmt
            | SyntaxKind::HairpinStmt
            | SyntaxKind::TempoStmt
            | SyntaxKind::MeterStmt
            | SyntaxKind::KeyStmt
            | SyntaxKind::ClefStmt
            | SyntaxKind::MobileStmt
            | SyntaxKind::ImproviseStmt
            | SyntaxKind::SectionStmt
            | SyntaxKind::HarmonyStmt
    )
}

/// An argument read as the word it spells rather than as an expression.
///
/// Every significant token joined, because a policy and a rule id are both
/// written as one identifier and anything else is a mistake this reading
/// wants to *quote back*: `may omit` spelled with a space should say what
/// was written, not what its first token was.
fn word(node: &SyntaxNode) -> String {
    significant_tokens(node).map(|token| token.text().to_owned()).collect()
}

/// Where a claim's name is written, falling back to the whole statement.
fn claim_span(statement: &musa_language::ast::AssertStmt, node: &SyntaxNode) -> SourceSpan {
    statement.claim_span().map_or_else(
        || crate::resolve::trimmed_span(node),
        |(start, end)| SourceSpan::new(start, end),
    )
}

/// The statements `node` encloses, in written order.
///
/// A `music` expression holds its items directly and everything else holds them
/// under one [`SyntaxKind::Block`], because the parser opens a fresh bar context
/// at a brace and records that by starting a node. One function rather than a
/// condition at each of the eleven callers: which of the two shapes a form has
/// is the grammar's answer, and the fold has no reason to know it.
fn statements(node: &SyntaxNode) -> impl Iterator<Item = SyntaxNode> {
    let holder = node
        .children()
        .find(|child| child.kind() == SyntaxKind::Block)
        .unwrap_or_else(|| node.clone());
    holder.children().filter(|child| is_statement(child.kind()))
}

impl Lowering<'_> {
    /// `music { … }` — a block of notation, as the track it denotes.
    ///
    /// The entry [`Lowering::value`] calls, and the entry both declarations call.
    /// Reads at [`Reading::free`], because a written `music` expression is a
    /// value and a value has no voice.
    ///
    /// # A block asks nothing
    ///
    /// It used to. Every constructor this module reaches for answered
    /// `Result τ Text`, so the fold was written in the surface's own `?` and
    /// drained at the brace, and a block denoted `Result<EventTrack ⟨written⟩,
    /// Text>` — a type §2 never wrote and no author could use without unwrapping
    /// it first. Prompt 141m gave a δ-rule somewhere to say no, so a fact that
    /// cannot sound refuses the program at its own span instead of answering a
    /// failure nobody can act on, and the constructors went total.
    ///
    /// What is left is the fold itself: a block denotes `EventTrack ⟨written⟩`,
    /// which is what §2's twenty signatures say it denotes. The questions
    /// machinery stays where it belongs, serving a `?` the author wrote.
    pub(crate) fn music(&mut self, node: &SyntaxNode) -> Option<Raw> {
        self.notated(node, Reading::free())
    }

    /// `motif turn(root: Pitch) { … }` — the function §2 says it is.
    ///
    /// "A named `fn turn(...) -> EventTrack[WrittenTime, ScoreFact] { music {
    /// body } }` with a `Motif` role retained for lints, extraction, editing,
    /// and Origin", and the role is `Sites`' already: the block's statements are
    /// numbered from the motif's own nodes, so a fact built here points at the
    /// note inside the motif rather than at the `use` that expanded it.
    ///
    /// # Why no written return type
    ///
    /// §2 spells `-> EventTrack[WrittenTime, ScoreFact]`, and it is now the type
    /// the body has. Spelling it here would still be *checking*, which this
    /// module does not do: the core infers it from the body, which is the one
    /// place that can tell, and a written annotation that agreed with the
    /// inference would be the same fact stated twice.
    ///
    /// # Why the parameters come from the typed AST
    ///
    /// A `fn`'s parameter is a [`SyntaxKind::Param`] node and a motif's is not:
    /// the motif grammar writes the name as a bare token beside a `TypeName`,
    /// which is why [`super::items::written_parameters`] finds none here.
    /// [`musa_language::ast::MotifDecl::params`] is the grammar's own reading of
    /// that shape, so it is used rather than re-derived — and because it yields
    /// names without nodes, each binder is numbered at the declaration. That is
    /// the honest answer: there is no node to point at, and inventing a
    /// plausible one would point a reader at code that is not the cause.
    pub(super) fn motif(&mut self, node: &SyntaxNode) -> Option<super::items::Definition> {
        let origin = self.origin(node);
        let name = super::items::declared_name(node)?;
        let written = musa_language::ast::MotifDecl::cast(node.clone())?.params();
        let mut value = self.music(node)?;
        for parameter in written.iter().rev() {
            value = musa_core::Raw::lam(origin, parameter.name.as_str(), value);
        }
        Some(super::items::Definition {
            origin,
            name,
            ty: None,
            value,
        })
    }

    /// `fragment name { … }` — the `let` §2 says it is.
    ///
    /// No parameters, because the grammar gives it none: a fragment that took
    /// one would be a motif, which is the distinction §2 draws by giving them
    /// two words.
    pub(super) fn fragment(&mut self, node: &SyntaxNode) -> Option<super::items::Definition> {
        Some(super::items::Definition {
            origin: self.origin(node),
            name: super::items::declared_name(node)?,
            ty: None,
            value: self.music(node)?,
        })
    }

    /// The left fold of `node`'s statements, seeded with `nothing`.
    ///
    /// Every statement is read even after one is refused, so a block with three
    /// bad statements reports three diagnostics rather than the first. The
    /// answer is [`None`] if any of them was refused, because a fold that
    /// quietly dropped a statement would answer a *different piece of music*
    /// than the one written.
    ///
    /// A claim raised while a statement is read is placed relative to *this*
    /// block, so every claim that came out of that statement has the music
    /// standing before it prepended before the fold moves on. One level of that
    /// at each nesting is what makes [`Claimed::before`] absolute by the time a
    /// voice is finished, without any block having to know where it stands.
    pub(super) fn notated(&mut self, node: &SyntaxNode, reading: Reading) -> Option<Raw> {
        let origin = self.origin(node);
        let mut built = Raw::lit(origin, crate::registry::empty_track());
        let mut whole = true;
        let mut reading = reading;
        for statement in statements(node) {
            // A `meter` is in force from where it is written, so it is read
            // *before* the statement that wrote it is folded and stays in force
            // for everything after — which is the whole of what makes the
            // restoring half of `senza` the meter a composer expects.
            if let Some(written) = musa_language::ast::MeterStmt::cast(statement.clone())
                && let Some(meter) = crate::resolve::parse_meter(&written)
            {
                reading = reading.metered(meter);
            }
            let raised = self.claims.len();
            match self.statement(&statement, reading) {
                Some(next) => {
                    for claim in self.claims.iter_mut().skip(raised) {
                        let inside = claim.before.clone();
                        claim.before = applied(origin, Raw::var(origin, "follow"), [built.clone(), inside]);
                    }
                    built = applied(origin, Raw::var(origin, "follow"), [built, next]);
                }
                None => whole = false,
            }
        }
        whole.then_some(built)
    }

    /// One notation statement, as the track it denotes.
    ///
    /// The table this module exists for. Every arm answers a track; the ones
    /// that refuse say why at the node that caused it, and the two kinds of
    /// refusal are deliberately different words — see [`Lowering::misplaced`].
    fn statement(&mut self, node: &SyntaxNode, reading: Reading) -> Option<Raw> {
        let origin = self.origin(node);
        let span = crate::resolve::trimmed_span(node);
        match node.kind() {
            SyntaxKind::NoteStmt => self.note(node, origin, reading),
            SyntaxKind::RestStmt => {
                let (duration, free) = self.notated_duration(node, span)?;
                let held = duration.value.as_ratio();
                let fact = applied(
                    origin,
                    Raw::var(origin, "Fact.Rest"),
                    [
                        payload(origin, "NotatedDuration", duration),
                        listed(origin, Vec::new()),
                        optional(origin, "FreeDuration", free),
                    ],
                );
                Some(self.sounded(origin, reading, fact, held))
            }
            SyntaxKind::ChordStmt => self.chord_statement(node, origin, reading),
            SyntaxKind::StackStmt => self.stack(node, origin, reading),
            SyntaxKind::GraceStmt => self.grace(node, origin, reading),
            SyntaxKind::UseStmt => self.used(node),

            // The transformation blocks: each is one track builtin applied to
            // the fold of its body, which is the whole of §3's "the function and
            // block spellings invoke the same semantic action".
            SyntaxKind::TransposeStmt => {
                let text = musa_language::ast::TransposeStmt::cast(node.clone())
                    .and_then(|stmt| stmt.interval())
                    .unwrap_or_default();
                let Some(interval) = crate::Interval::parse(&text, writes(node, SyntaxKind::DownKw)) else {
                    return self.refuse(
                        Diagnostic::error(Code::NotAValue, format!("`{text}` is not an interval"))
                            .at(span, "unknown interval")
                            .note("a quality and a number: `P5`, `M3`, `m6`, `A4`, `d5`"),
                    );
                };
                let body = self.notated(node, reading)?;
                let call = applied(
                    origin,
                    Raw::var(origin, "transpose"),
                    [plain(origin, "Interval", interval), body],
                );
                Some(call)
            }
            SyntaxKind::StretchStmt => {
                let text = musa_language::ast::StretchStmt::cast(node.clone())
                    .and_then(|stmt| stmt.factor())
                    .unwrap_or_default();
                let factor = crate::resolve::parse_ratio(&text)
                    .or_else(|| text.parse::<i64>().ok().map(Ratio::from_integer))
                    .filter(|factor| *factor > Ratio::ZERO);
                let Some(factor) = factor else {
                    return self.refuse(
                        Diagnostic::error(Code::NotAValue, format!("`{text}` is not a stretch factor"))
                            .at(span, "expected a positive number, like `2` or `3/2`"),
                    );
                };
                let body = self.notated(node, reading)?;
                let call = applied(
                    origin,
                    Raw::var(origin, "stretch"),
                    [plain(origin, "Ratio", factor), body],
                );
                Some(call)
            }
            SyntaxKind::RetrogradeStmt => {
                let body = self.notated(node, reading)?;
                Some(Raw::app(origin, Raw::var(origin, "retrograde"), body))
            }
            SyntaxKind::InvertStmt => {
                let text = musa_language::ast::InvertStmt::cast(node.clone())
                    .and_then(|stmt| stmt.axis())
                    .unwrap_or_default();
                let Some(axis) = crate::WrittenPitch::parse(&text) else {
                    return self.refuse(
                        Diagnostic::error(Code::NotAValue, format!("`{text}` is not a pitch"))
                            .at(span, "inversion needs a pitch to mirror about"),
                    );
                };
                let body = self.notated(node, reading)?;
                let call = applied(origin, Raw::var(origin, "invert"), [plain(origin, "Pitch", axis), body]);
                Some(call)
            }

            // `in scale` changes what a `step` reads and denotes its body. It is
            // the one statement that contributes no fact of its own.
            SyntaxKind::InScaleStmt => {
                let Some(written) = musa_language::ast::InScaleStmt::cast(node.clone()).and_then(|s| s.scale_expr())
                else {
                    return self.refuse(
                        Diagnostic::error(Code::NotAValue, "`in scale` needs a scale")
                            .at(span, "expected a scale, such as `c dorian`"),
                    );
                };
                let scale = self.counting(&written)?;
                self.notated(node, reading.stepping(scale))
            }

            // The region annotations: a fact over the span its body covers.
            SyntaxKind::SlurStmt => self.region(node, origin, reading, Raw::var(origin, "Fact.Slur")),
            SyntaxKind::PhraseStmt => {
                let name = musa_language::ast::PhraseStmt::cast(node.clone())
                    .and_then(|stmt| stmt.name())
                    .unwrap_or_default();
                let fact = Raw::app(origin, Raw::var(origin, "Fact.Phrase"), plain(origin, "Text", name));
                self.region(node, origin, reading, fact)
            }
            SyntaxKind::TupletStmt => {
                let text = musa_language::ast::TupletStmt::cast(node.clone())
                    .and_then(|stmt| stmt.ratio())
                    .unwrap_or_default();
                let Some((num, den)) = tuplet_ratio(&text) else {
                    return self.refuse(
                        Diagnostic::error(Code::NotAValue, format!("`{text}` is not a tuplet ratio"))
                            .at(span, "expected something like `3/2`"),
                    );
                };
                let fact = applied(
                    origin,
                    Raw::var(origin, "Fact.Tuplet"),
                    [whole(origin, u64::from(num)), whole(origin, u64::from(den))],
                );
                self.region(node, origin, reading, fact)
            }
            SyntaxKind::HairpinStmt => {
                let statement = musa_language::ast::HairpinStmt::cast(node.clone())?;
                let text = statement.target().unwrap_or_default();
                let Some(target) = crate::score::DynamicMark::parse(&text) else {
                    return self.refuse(Self::not_a_dynamic(&text, span));
                };
                let grows = if statement.grows() { "Bool.True" } else { "Bool.False" };
                let fact = applied(
                    origin,
                    Raw::var(origin, "Fact.Hairpin"),
                    [
                        Raw::var(origin, grows),
                        payload(origin, "DynamicMark", target),
                        // The grammar writes `cres.`/`dim.` and nothing about
                        // shape, so every hairpin is a straight line — the same
                        // value the old checker put in the timeline, put in the
                        // term instead of invented downstream.
                        payload(origin, "Progress", musa_kernel::Progress::linear()),
                    ],
                );
                self.region(node, origin, reading, fact)
            }
            SyntaxKind::RepeatStmt => self.repeat(node, origin, reading),
            SyntaxKind::EndingStmt => self.ending(node, origin, reading),
            SyntaxKind::MobileStmt => self.mobile(node, origin, reading),
            SyntaxKind::ImproviseStmt => self.improvise(node, origin, reading),

            // The point annotations: a fact at the instant it is written.
            SyntaxKind::DynamicStmt => {
                let text = musa_language::ast::DynamicStmt::cast(node.clone())
                    .and_then(|stmt| stmt.mark())
                    .unwrap_or_default();
                let Some(mark) = crate::score::DynamicMark::parse(&text) else {
                    return self.refuse(Self::not_a_dynamic(&text, span));
                };
                let fact = Raw::app(
                    origin,
                    Raw::var(origin, "Fact.Dynamic"),
                    payload(origin, "DynamicMark", mark),
                );
                Some(self.sounded(origin, reading, fact, Ratio::ZERO))
            }
            SyntaxKind::SectionStmt => {
                let name = musa_language::ast::SectionStmt::cast(node.clone())
                    .and_then(|stmt| stmt.name())
                    .unwrap_or_default();
                let fact = Raw::app(origin, Raw::var(origin, "Fact.Section"), plain(origin, "Text", name));
                Some(self.sounded(origin, reading, fact, Ratio::ZERO))
            }
            SyntaxKind::HarmonyStmt => {
                let text = musa_language::ast::HarmonyStmt::cast(node.clone())
                    .and_then(|stmt| stmt.symbol())
                    .map(|symbol| symbol.text())
                    .unwrap_or_default();
                let Some(symbol) = crate::harmony::ChordSymbol::parse(&text) else {
                    return self.refuse(
                        Diagnostic::error(Code::NotAValue, format!("`{text}` is not a chord symbol"))
                            .at(span, "expected something like `Cmaj7` or `F#m7b5`"),
                    );
                };
                let fact = Raw::app(
                    origin,
                    Raw::var(origin, "Fact.Harmony"),
                    payload(origin, "ChordSymbol", symbol),
                );
                Some(self.sounded(origin, reading, fact, Ratio::ZERO))
            }
            SyntaxKind::MarkStmt => self.marked(node, origin, reading),

            // §3's own list: "from here onward" has no unique meaning in a value
            // usable at several places — so each of the four is refused where
            // there is no single here, and is a point where there is one.
            SyntaxKind::KeyStmt => self.context(node, origin, reading, span, Context::Key),
            SyntaxKind::MeterStmt => self.context(node, origin, reading, span, Context::Meter),
            SyntaxKind::TempoStmt => self.context(node, origin, reading, span, Context::Tempo),
            SyntaxKind::ClefStmt => self.context(node, origin, reading, span, Context::Clef),

            SyntaxKind::BarStmt => self.bar(node, origin, reading),
            SyntaxKind::SenzaStmt => self.senza(node, origin, reading, span),

            SyntaxKind::AssertStmt => self.asserted(node, origin, reading),
            _ => None,
        }
    }

    // ---- the statements with enough reading to want their own function ----

    /// `c5/4`, `(c5 step 2)/8 staccato`, `g4/4 to 2/1 ~`.
    ///
    /// One `play` of a one-note voicing rather than a `sounded` of a `Fact.Note`,
    /// for the reason 141j's Design gives: `play` is what turns written pitches
    /// into `Note` facts, and a note is a chord of one.
    fn note(&mut self, node: &SyntaxNode, origin: Origin, reading: Reading) -> Option<Raw> {
        let statement = musa_language::ast::NoteStmt::cast(node.clone())?;
        let span = crate::resolve::trimmed_span(node);
        let pitch = self.pitch_term(&statement, node, origin, reading)?;
        let (duration, free) = self.notated_duration(node, span)?;
        let held = duration.value.as_ratio();
        let fact = applied(
            origin,
            Raw::var(origin, "Fact.Note"),
            [
                pitch,
                payload(origin, "NotatedDuration", duration),
                self.articulations(origin, &statement.articulations(), span),
                optional(origin, "FreeDuration", free),
            ],
        );
        Some(self.sounded(origin, reading, fact, held))
    }

    /// `[c4 e4 g4]/2` — the written pitches sounding together.
    ///
    /// Every pitch is its own `Note` fact over the same span, which is what a
    /// chord *is* at this layer (§3's `map_note_pitches` clause says so from the
    /// other side). Read as one `sounded` per pitch folded with `together`, and
    /// not as a `play`, because `play` takes a `Voicing`, a `Voicing` takes a
    /// `ChordClass`, and a written simultaneity is not required to spell one:
    /// `[c4 c#4]` names no chord class and is still a sounding. `stack` is the
    /// statement that names one.
    fn chord_statement(&mut self, node: &SyntaxNode, origin: Origin, reading: Reading) -> Option<Raw> {
        let statement = musa_language::ast::ChordStmt::cast(node.clone())?;
        let span = crate::resolve::trimmed_span(node);
        let (duration, free) = self.notated_duration(node, span)?;
        let held = duration.value.as_ratio();
        let articulations = self.articulations(origin, &statement.articulations(), span);
        let written = statement.pitches();
        if written.is_empty() {
            return self.refuse(
                Diagnostic::error(Code::NotAValue, "a chord with no notes is a rest")
                    .at(span, "no pitches inside these brackets")
                    .help("write `rest` for silence, or the pitches this chord sounds"),
            );
        }
        let mut sounding = None;
        for text in written {
            let Some(pitch) = crate::WrittenPitch::parse(&text) else {
                return self.refuse(Self::not_a_pitch(&text, span));
            };
            let fact = applied(
                origin,
                Raw::var(origin, "Fact.Note"),
                [
                    plain(origin, "Pitch", pitch),
                    payload(origin, "NotatedDuration", duration.clone()),
                    articulations.clone(),
                    optional(origin, "FreeDuration", free),
                ],
            );
            let one = self.sounded(origin, reading, fact, held);
            sounding = Some(match sounding {
                None => one,
                Some(built) => applied(origin, Raw::var(origin, "together"), [built, one]),
            });
        }
        sounding
    }

    /// `stack c4 major7/2` — a chord class voiced from a written bass.
    ///
    /// The one statement that reaches `play`'s `Voicing` argument, because it is
    /// the one that names a chord class: `[c4 e4 g4]` is three pitches and
    /// `stack c4 major/2` is a C major triad voiced upward from `c4`.
    fn stack(&mut self, node: &SyntaxNode, origin: Origin, reading: Reading) -> Option<Raw> {
        let statement = musa_language::ast::StackStmt::cast(node.clone())?;
        let span = crate::resolve::trimmed_span(node);
        let text = statement.root().unwrap_or_default();
        let Some(bass) = crate::WrittenPitch::parse(&text) else {
            return self.refuse(Self::not_a_pitch(&text, span));
        };
        let word = statement.chord_type().unwrap_or_default();
        let Some(kind) = crate::chord::ChordType::named(&word) else {
            return self.refuse(
                Diagnostic::error(Code::UnknownName, format!("unknown chord type `{word}`"))
                    .at(span, "not a named chord type")
                    .note("a chord type is the content; the symbol written above the staff is a separate annotation"),
            );
        };
        let (duration, _) = self.notated_duration(node, span)?;
        let class = crate::chord::ChordClass::new(bass.pitch_class(), kind);
        let Ok(voicing) = crate::chord::Voicing::close_position(class, bass) else {
            return self.refuse(
                Diagnostic::error(Code::OutOfRange, "this chord does not stack above that bass")
                    .at(span, "the written coordinates leave Musa's exact range"),
            );
        };
        let call = applied(
            origin,
            Raw::var(origin, "play"),
            [
                self.provenance(origin, span),
                scope_of(origin, reading.scope),
                // `plain` and not `payload`: `play` reads a `Voicing` and not an
                // `Opaque<Voicing>`, and a literal at the wrong Rust type
                // downcasts to nothing, which the core reports as this
                // compiler's table disagreeing with itself.
                plain(origin, "Voicing", voicing),
                written_duration(origin, duration.value.as_ratio()),
            ],
        );
        Some(call)
    }

    /// `grace { c5 d5 }` — the notes crushed before the one they lean on.
    ///
    /// Each grace note is a *point* `Fact.Grace` carrying its index, which is
    /// load-bearing rather than decorative: every grace note in one group shares
    /// a start and an end, so without the index `grace { c5 d5 }` and
    /// `grace { d5 c5 }` normalize to the same timeline.
    fn grace(&mut self, node: &SyntaxNode, origin: Origin, reading: Reading) -> Option<Raw> {
        let span = crate::resolve::trimmed_span(node);
        let mut built = Raw::lit(origin, crate::registry::empty_track());
        for (index, note) in musa_language::ast::GraceStmt::cast(node.clone())?
            .notes()
            .into_iter()
            .enumerate()
        {
            let text = note.pitch().unwrap_or_default();
            let Some(pitch) = crate::WrittenPitch::parse(&text) else {
                return self.refuse(Self::not_a_pitch(&text, span));
            };
            let at = self.origin(note.syntax());
            let fact = applied(
                at,
                Raw::var(at, "Fact.Grace"),
                [
                    plain(at, "Pitch", pitch),
                    self.articulations(at, &note.articulations(), span),
                    whole(at, index as u64),
                ],
            );
            let one = self.sounded(at, reading, fact, Ratio::ZERO);
            built = applied(origin, Raw::var(origin, "follow"), [built, one]);
        }
        Some(built)
    }

    /// `mark breath;` and `mark pedal { … }`.
    ///
    /// The one annotation that is a point *or* a region depending on how it was
    /// written, which is exactly what [`crate::elaborate::FactKind::Mark`]'s own
    /// documentation says decides it: "which of the two this occurrence is, is
    /// its span — a point's is empty".
    fn marked(&mut self, node: &SyntaxNode, origin: Origin, reading: Reading) -> Option<Raw> {
        let statement = musa_language::ast::MarkStmt::cast(node.clone())?;
        let span = crate::resolve::trimmed_span(node);
        let text = statement.name().unwrap_or_default();
        let Some(mark) = crate::Mark::parse(&text) else {
            return self.refuse(
                Diagnostic::error(Code::UnknownWord, format!("`{text}` is not a mark"))
                    .at(span, "unknown mark")
                    .help(crate::resolve::suggest(
                        &text,
                        &crate::marks::statement_names(),
                        "marks",
                    )),
            );
        };
        let argument = statement.text().map(crate::marks::MarkArgument::Text).or_else(|| {
            statement
                .number()
                .and_then(|written| written.parse().ok())
                .map(crate::marks::MarkArgument::Number)
        });
        let fact = applied(
            origin,
            Raw::var(origin, "Fact.Mark"),
            [
                payload(origin, "Mark", mark),
                optional(origin, "MarkArgument", argument),
            ],
        );
        if statement.has_block() {
            self.region(node, origin, reading, fact)
        } else {
            Some(self.sounded(origin, reading, fact, Ratio::ZERO))
        }
    }

    /// `repeat 2 { … }` and `repeat 4 to 16 { … }`.
    ///
    /// The fact and the body, not the body expanded. A repeat over its passes is
    /// what the timeline holds and the page prints once between barlines; which
    /// pass a performance takes is a *reading* of the fact, and readings are not
    /// this module's (roadmap §2's own example of the layer table).
    fn repeat(&mut self, node: &SyntaxNode, origin: Origin, reading: Reading) -> Option<Raw> {
        let statement = musa_language::ast::RepeatStmt::cast(node.clone())?;
        let span = crate::resolve::trimmed_span(node);
        let text = statement.count().unwrap_or_default();
        let (times, range) = match text.split_once(" to ") {
            Some((least, most)) => {
                let (Some(least), Some(most)) = (count_of(least), count_of(most)) else {
                    return self.refuse(Self::not_a_count(&text, span));
                };
                (least, Some((least, most)))
            }
            None => {
                let Some(times) = count_of(&text) else {
                    return self.refuse(Self::not_a_count(&text, span));
                };
                (times, None)
            }
        };
        let range = range.map(|(least, most)| {
            applied(
                origin,
                Raw::var(origin, "Pair.Both"),
                [whole(origin, u64::from(least)), whole(origin, u64::from(most))],
            )
        });
        let fact = applied(
            origin,
            Raw::var(origin, "Fact.Repeat"),
            [whole(origin, u64::from(times)), maybe(origin, range)],
        );
        self.region(node, origin, reading, fact)
    }

    /// `ending 1 { … }`.
    ///
    /// Read wherever it stands, and not refused for standing outside a `repeat`.
    /// The old checker refused that because it *expanded* repeats and an ending
    /// with no pass to belong to had nowhere to go; this module writes the fact
    /// and lets the pass that reads passes decide, which is the same move as
    /// leaving a repeat unexpanded above.
    fn ending(&mut self, node: &SyntaxNode, origin: Origin, reading: Reading) -> Option<Raw> {
        let statement = musa_language::ast::EndingStmt::cast(node.clone())?;
        let span = crate::resolve::trimmed_span(node);
        let text = statement.number().unwrap_or_default();
        let Some(bracket) = count_of(&text) else {
            return self.refuse(Self::not_a_count(&text, span));
        };
        let fact = applied(
            origin,
            Raw::var(origin, "Fact.Ending"),
            [whole(origin, u64::from(bracket)), whole(origin, u64::from(bracket))],
        );
        self.region(node, origin, reading, fact)
    }

    /// `bar { … }` and `bar refrain { … }` — a measure, and what it claims.
    ///
    /// The braces erase. A bar contributes no occurrence, no payload, and no
    /// time of its own — the kernel's ontology has no bar in it, and where the
    /// barlines fall is [`crate::BarLines`]'s answer over the meters — so the
    /// term a bar denotes is exactly the fold of what is inside it. What the
    /// braces contribute is the claim that the music between them fills one
    /// measure of the meter in force where they stand, and that is a
    /// [`Claimed`] rather than a fact.
    ///
    /// The name is not read here. `bar refrain { … }` binds a passage another
    /// voice can answer, which is a *declaration* in the enclosing document
    /// rather than a statement in this block, and reading it here would put the
    /// same name in scope once per voice that mentions it.
    fn bar(&mut self, node: &SyntaxNode, origin: Origin, reading: Reading) -> Option<Raw> {
        let statement = musa_language::ast::BarStmt::cast(node.clone())?;
        let passage = self.notated(node, reading)?;
        self.claims.push(Claimed {
            predicate: crate::assert::predicate("fills_meter")?,
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
    fn asserted(&mut self, node: &SyntaxNode, origin: Origin, reading: Reading) -> Option<Raw> {
        let statement = musa_language::ast::AssertStmt::cast(node.clone())?;
        let predicate = self.claimed_predicate(&statement, node)?;
        let arguments = self.claim_arguments(predicate, &statement, node)?;
        let passage = self.notated(node, reading)?;
        self.claims.push(Claimed {
            predicate,
            arguments,
            span: crate::resolve::trimmed_span(node),
            content_end: statement.content_end(),
            noun: "passage",
            // Nothing yet, exactly as a bar records nothing: the fold this
            // assertion stands in prepends what comes before it.
            before: Raw::lit(origin, crate::registry::empty_track()),
            passage: passage.clone(),
        });
        Some(passage)
    }

    /// The registry row `statement` names, or a refusal that lists the family.
    ///
    /// The whole family in the note rather than only the nearest spelling,
    /// because `CLAIMS` is six rows and a composer who misremembered one is
    /// better served by reading all six than by being guessed at.
    fn claimed_predicate(
        &mut self,
        statement: &musa_language::ast::AssertStmt,
        node: &SyntaxNode,
    ) -> Option<&'static crate::assert::Predicate> {
        let name = statement.claim().unwrap_or_default();
        if let Some(predicate) = crate::assert::predicate(&name) {
            return Some(predicate);
        }
        let known: Vec<&str> = crate::assert::names().collect();
        self.refuse(
            Diagnostic::error(Code::UnknownName, format!("nothing is claimed by `{name}`"))
                .at(claim_span(statement, node), "not a claim musa can prove")
                .maybe_help(
                    crate::diagnose::nearest(&name, known.iter().copied())
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
    fn claim_arguments(
        &mut self,
        predicate: &'static crate::assert::Predicate,
        statement: &musa_language::ast::AssertStmt,
        node: &SyntaxNode,
    ) -> Option<Vec<Argued>> {
        use crate::assert::{Argument, ParamType};

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
                        crate::assert::spell_arguments(predicate.parameters.len()),
                        crate::assert::spell_written(written.len())
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
    fn claim_value(&mut self, node: &SyntaxNode, origin: Origin, ty: Raw) -> Option<Argued> {
        let written = child(node, is_expr_node)?;
        let term = self.expr(&written)?;
        Some(Argued::Value(Raw::annot(origin, term, ty)))
    }

    /// One of [`crate::assert::Realization`]'s three words.
    fn policy(&mut self, node: &SyntaxNode) -> Option<crate::assert::Realization> {
        let word = word(node);
        if let Some(policy) = crate::assert::Realization::named(&word) {
            return Some(policy);
        }
        let spellings: Vec<&str> = crate::assert::Realization::ALL
            .iter()
            .map(|policy| policy.as_str())
            .collect();
        self.refuse(
            Diagnostic::error(Code::UnknownWord, format!("`{word}` is not a realization policy"))
                .at(crate::resolve::trimmed_span(node), "expected one of three words")
                .maybe_help(
                    crate::diagnose::nearest(&word, spellings.iter().copied())
                        .map(|near| format!("did you mean `{near}`?")),
                )
                .note(
                    "`exactly` is set equality, `may_omit` lets a member be missing, \
                     and `may_add` lets other notes sound",
                ),
        )
    }

    /// The id of a voice-leading rule an assertion may name.
    fn rule_named(&mut self, node: &SyntaxNode) -> Option<crate::analysis::RuleName> {
        let word = word(node);
        if let Some(rule) = crate::analysis::assertable().find(|rule| rule.id() == word) {
            return Some(rule);
        }
        let assertable: Vec<&str> = crate::analysis::assertable().map(|rule| rule.id()).collect();
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
                crate::diagnose::nearest(&word, assertable.iter().copied())
                    .map(|near| format!("did you mean `{near}`?")),
            )
            .help(format!("the rules a source may assert are: {}", assertable.join(", ")))
            .note(
                "every other rule is reported by `musa analyze --kind voice-leading`, \
                 which says how strongly a style holds it rather than failing the build",
            ),
        )
    }

    /// `senza { … }` — an unmeasured stretch, with the meter put back after it.
    ///
    /// Three statements' worth of fold and no mechanism: `meter none`, the body,
    /// and the meter that was in force. That is what makes it sugar — the
    /// barlines stop for exactly as long as the braces say, and the meter that
    /// resumes is the one that was already there, so there is no second meter to
    /// keep in step with the first.
    ///
    /// The restoring meter is [`Reading::meter`], read lexically. The replaced
    /// path asked its cursor which meter change it had passed, which is the same
    /// answer by a longer route whenever a meter is written where it is read —
    /// and this reading refuses `meter` inside reusable material, so there is no
    /// other case.
    fn senza(&mut self, node: &SyntaxNode, origin: Origin, reading: Reading, span: SourceSpan) -> Option<Raw> {
        if !reading.placed {
            return self.misplaced("an unmeasured stretch", span);
        }
        let opened = self.sounded_at(
            origin,
            crate::Scope::Piece,
            metered(origin, crate::score::Meter::NONE),
            Ratio::ZERO,
        );
        // Under `meter none` for its whole length, so the body is read with the
        // unmeasured meter in force: a `senza` inside a `senza` restores the one
        // its own braces opened, which is the one that was in force there.
        let body = self.notated(node, reading.metered(crate::score::Meter::NONE))?;
        let closed = self.sounded_at(origin, crate::Scope::Piece, metered(origin, reading.meter), Ratio::ZERO);
        Some(applied(
            origin,
            Raw::var(origin, "follow"),
            [applied(origin, Raw::var(origin, "follow"), [opened, body]), closed],
        ))
    }

    /// `mobile { a; b; c; }` — the fragments as written.
    ///
    /// The order this performance chose is empty here, for the reason the repeat
    /// count is the written one: choosing is a reading, and a fragment usable at
    /// several places cannot have chosen already.
    fn mobile(&mut self, node: &SyntaxNode, origin: Origin, reading: Reading) -> Option<Raw> {
        let statement = musa_language::ast::MobileStmt::cast(node.clone())?;
        let fragments = statement
            .fragments()
            .into_iter()
            .map(|name| plain(origin, "Text", name))
            .collect();
        let fact = applied(
            origin,
            Raw::var(origin, "Fact.Mobile"),
            [listed(origin, fragments), listed(origin, Vec::new())],
        );
        self.region(node, origin, reading, fact)
    }

    /// `improvise 8/1 over "Dm7 | G7";` — a frame that sounds as silence.
    fn improvise(&mut self, node: &SyntaxNode, origin: Origin, reading: Reading) -> Option<Raw> {
        let statement = musa_language::ast::ImproviseStmt::cast(node.clone())?;
        let span = crate::resolve::trimmed_span(node);
        let (duration, _) = self.notated_duration(node, span)?;
        let over = statement.over().map(|text| plain(origin, "Text", text));
        let fact = Raw::app(origin, Raw::var(origin, "Fact.Improvise"), maybe(origin, over));
        Some(self.sounded(origin, reading, fact, duration.value.as_ratio()))
    }

    /// `use e;` — the track `e` denotes, folded on.
    ///
    /// No call at all: §2 says `use e;` "checks that `e` is a written-time score
    /// track", and the check is the core's, because `follow`'s signature demands
    /// one and the refusal lands at the origin this module gave the node.
    fn used(&mut self, node: &SyntaxNode) -> Option<Raw> {
        let called = child(node, is_expr_node)?;
        self.value(&called)
    }

    // ---- the pieces every arm above is written out of ----

    /// `sounded(origin, scope, fact, held)`.
    ///
    /// The construction §5.7 requires: every fact carries an origin and a scope,
    /// and neither is something a source line writes or a `fn` pointer invents.
    /// The reading supplies both, which is why the call has four arguments where
    /// the statement had none.
    ///
    /// Nothing to ask any more, and nothing to answer: a length a fact cannot
    /// sound for is refused by the rule at this origin (prompt 141m), so what
    /// comes back is a call and every caller gets one. The `Option` that used to
    /// be here was the `?` this reading wrote, and there is no `?` left to write.
    fn sounded(&self, origin: Origin, reading: Reading, fact: Raw, held: Ratio<i64>) -> Raw {
        self.sounded_at(origin, reading.scope, fact, held)
    }

    /// The same, at a scope the reading does not supply.
    ///
    /// Three callers want one: a `clef` written in a voice is the *part's*
    /// clef, a `key` or a `meter` written in a voice is the *piece's*, and the
    /// header facts [`super::piece`] builds belong to the part or the piece that
    /// wrote them rather than to any voice. §5.7 asks which scope a fact is
    /// constructed at, and the answer is not always the scope it was written in.
    pub(super) fn sounded_at(&self, origin: Origin, scope: crate::Scope, fact: Raw, held: Ratio<i64>) -> Raw {
        applied(
            origin,
            Raw::var(origin, "sounded"),
            [
                self.provenance_at(origin),
                scope_of(origin, scope),
                fact,
                written_duration(origin, held),
            ],
        )
    }

    /// One of the four statements whose meaning is "from here onward".
    ///
    /// Refused where there is no single here — a `music` value is "usable at
    /// several places" — and a **point** where there is one, which is what the
    /// replaced path made of a context statement written among a voice's items
    /// too. A point rather than a region because a left fold cannot see what
    /// comes after the statement it is reading, and because "from here onward"
    /// is answered by the context rules in [`crate::scope`] reading the
    /// occurrences in order rather than by the extent written on any one of them.
    fn context(
        &mut self,
        node: &SyntaxNode,
        origin: Origin,
        reading: Reading,
        span: SourceSpan,
        which: Context,
    ) -> Option<Raw> {
        if !reading.placed {
            return self.misplaced(which.what(), span);
        }
        // Where the *fact* belongs, which is not where it was written: a key,
        // a meter, and a tempo are things the piece does, and a clef is one
        // player's staff. Both rules are `elaborate.rs`'s, unchanged.
        let scope = match which {
            Context::Key | Context::Meter | Context::Tempo => crate::Scope::Piece,
            Context::Clef => match reading.scope {
                crate::Scope::Part { part } | crate::Scope::Voice { part, .. } => crate::Scope::Part { part },
                crate::Scope::Piece => {
                    return self.refuse(
                        Diagnostic::error(Code::Misplaced, "a clef change belongs to a part")
                            .at(span, "written outside any part")
                            .help("write it in the part, or in a voice of that part")
                            .note("a clef is one player's staff, and the piece as a whole is read on none"),
                    );
                }
            },
        };
        let fact = self.fact(node, origin, span, which)?;
        Some(self.sounded_at(origin, scope, fact, Ratio::ZERO))
    }

    /// The `Fact` one of the four states, without the placement around it.
    ///
    /// Shared with [`super::piece`], which writes the same four facts over the
    /// region a header covers rather than at the point a body reached. Reading
    /// them twice would be two answers to what `key g major;` means.
    pub(super) fn fact(&mut self, node: &SyntaxNode, origin: Origin, span: SourceSpan, which: Context) -> Option<Raw> {
        match which {
            Context::Key => {
                let statement = musa_language::ast::KeyStmt::cast(node.clone())?;
                // `key k;` names a key rather than spelling one, and the parser
                // wrote the name as an expression child for exactly this
                // reading. The ordinary value reading answers it, so a key a
                // template was handed reaches `Fact.Key` the same way a written
                // one does.
                if let Some(written) = child(node, is_expr_node) {
                    let named = self.value(&written)?;
                    return Some(Raw::app(origin, Raw::var(origin, "Fact.Key"), named));
                }
                let Some(key) = crate::resolve::parse_key(&statement) else {
                    return self.refuse(
                        Diagnostic::error(Code::NotAValue, "this key cannot be read")
                            .at(span, "expected a note and a mode, like `a minor`"),
                    );
                };
                Some(keyed(origin, key))
            }
            Context::Meter => {
                let statement = musa_language::ast::MeterStmt::cast(node.clone())?;
                let Some(meter) = crate::resolve::parse_meter(&statement) else {
                    return self.refuse(
                        Diagnostic::error(Code::NotAValue, "this meter cannot be read")
                            .at(span, "expected `4/4`, or `none`"),
                    );
                };
                Some(metered(origin, meter))
            }
            Context::Tempo => {
                let statement = musa_language::ast::TempoStmt::cast(node.clone())?;
                let marking = crate::resolve::tempo_marking(self.resolver, &statement);
                Some(tempo(origin, &marking))
            }
            Context::Clef => {
                let written = musa_language::ast::ClefStmt::cast(node.clone())
                    .and_then(|statement| statement.name())
                    .unwrap_or_default();
                let Some(clef) = crate::score::Clef::parse(&written) else {
                    return self.refuse(
                        Diagnostic::error(Code::UnknownWord, format!("`{written}` is not a clef"))
                            .at(span, "not a clef musa reads")
                            .help(crate::resolve::suggest(&written, crate::score::Clef::NAMES, "clefs")),
                    );
                };
                Some(clefed(origin, clef))
            }
        }
    }

    /// A fact over the region its body covers, and the body under it.
    ///
    /// `together(sounded(…, duration(body)), body)` is what "over the region it
    /// spans" means with no `duration` query available: the fold already knows
    /// what it built, so the extent is computed here and written as a literal
    /// rather than asked of the term. That is 141j's argument for registering
    /// `follow` rather than `duration`, used.
    fn region(&mut self, node: &SyntaxNode, origin: Origin, reading: Reading, fact: Raw) -> Option<Raw> {
        let body = self.notated(node, reading)?;
        let over = extent(node);
        let marker = self.sounded(origin, reading, fact, over);
        Some(applied(origin, Raw::var(origin, "together"), [marker, body]))
    }

    /// A written duration and the freedom written on it.
    fn notated_duration(
        &mut self,
        node: &SyntaxNode,
        span: SourceSpan,
    ) -> Option<(NotatedDuration, Option<crate::score::FreeDuration>)> {
        let Some(duration) = crate::resolve::parse_duration(node) else {
            // A duration written as a *parameter* is a value, and turning a
            // `Duration` value into the `NotatedDuration` a fact carries needs a
            // word no ownership table names. Prompt 142 owns it, with the voice.
            if musa_language::ast::Duration::of(node)
                .and_then(|written| written.parameter())
                .is_some()
            {
                return self.not_yet(node, "a duration written as a parameter", "a voice to belong to");
            }
            return self.refuse(
                Diagnostic::error(Code::NotAValue, "this statement has no duration")
                    .at(span, "expected a duration")
                    .note("a duration is a fraction or a whole number of whole notes: `1/4`, `3/8`, `1`"),
            );
        };
        Some((duration, None))
    }

    /// The `Pitch` a note statement sounds, as a term.
    ///
    /// A spelled pitch folds to a literal, which is what lets the fold finish a
    /// note into a value; a *named* one does not, because a parameter has no
    /// value until an instance site supplies one. Both answer a `Raw` at
    /// `Pitch`, so the `Fact.Note` written around them is one expression rather
    /// than two — what the second costs is that the `sounded` enclosing it stays
    /// a neutral term until the site applies it, which is the ordinary behaviour
    /// of a builtin under an unapplied binder (`02-core-calculus.md` §5.8).
    ///
    /// Which of the two a statement wrote is the parser's answer and not a
    /// second grammar here: `c4` lexes as a pitch literal and `root` as an
    /// identifier, and the statement's own tokens say which is there.
    fn pitch_term(
        &mut self,
        statement: &musa_language::ast::NoteStmt,
        node: &SyntaxNode,
        origin: Origin,
        reading: Reading,
    ) -> Option<Raw> {
        if let Some(written) = statement.pitch_expr() {
            return self.pitch_of(&written, reading);
        }
        let text = statement.pitch().unwrap_or_default();
        if writes(node, SyntaxKind::Identifier) {
            return Some(Raw::var(origin, text.as_str()));
        }
        let Some(pitch) = crate::WrittenPitch::parse(&text) else {
            return self.refuse(Self::not_a_pitch(&text, crate::resolve::trimmed_span(node)));
        };
        Some(plain(origin, "Pitch", pitch))
    }

    /// A pitch expression as a term, whatever it names.
    ///
    /// `up`/`down` becomes `pitch_transposed`, which reduces to a literal when
    /// both arguments are ones and stays a neutral spine when either is a
    /// binder — so one reading serves `c5/4` and `(root up M2)/4` and the fold
    /// never asks which it got. `step` is the exception: no registered
    /// operation walks a frame of a collection, so it still folds here and
    /// still needs a base it can read.
    fn pitch_of(&mut self, node: &SyntaxNode, reading: Reading) -> Option<Raw> {
        let origin = self.origin(node);
        match node.kind() {
            SyntaxKind::ParenExpr | SyntaxKind::BlockExpr => {
                let inner = child(node, is_expr_node)?;
                self.pitch_of(&inner, reading)
            }
            SyntaxKind::NameExpr | SyntaxKind::PathExpr => self.value(node),
            SyntaxKind::PitchExpr => {
                let parts = children(node, is_expr_node);
                let base = self.pitch_of(parts.first()?, reading)?;
                let interval = self.interval_of(parts.get(1)?, writes(node, SyntaxKind::DownKw))?;
                Some(applied(origin, Raw::var(origin, "pitch_transposed"), [base, interval]))
            }
            _ => {
                let pitch = self.written_pitch(node, reading)?;
                Some(plain(origin, "Pitch", pitch))
            }
        }
    }

    /// The interval a transposition moves by, as a term.
    ///
    /// A named one is inverted by `interval_inverse` rather than by parsing the
    /// spelling with a sign, because `down` is a direction the source wrote and
    /// the value it applies to may not arrive until an instance site.
    fn interval_of(&mut self, node: &SyntaxNode, down: bool) -> Option<Raw> {
        let origin = self.origin(node);
        if named(node) {
            let held = self.value(node)?;
            return Some(if down {
                Raw::app(origin, Raw::var(origin, "interval_inverse"), held)
            } else {
                held
            });
        }
        let text = node.to_string().trim().to_owned();
        let Some(interval) = crate::Interval::parse(&text, down) else {
            return self.refuse(
                Diagnostic::error(Code::NotAValue, format!("`{text}` is not an interval"))
                    .at(crate::resolve::trimmed_span(node), "unknown interval"),
            );
        };
        Some(plain(origin, "Interval", interval))
    }

    /// A written pitch, finished under the scale in force.
    ///
    /// The one place `in scale` is read. `p step n` needs a scale and says so
    /// when there is none; `p up M3` does not and never asks.
    fn written_pitch(&mut self, node: &SyntaxNode, reading: Reading) -> Option<crate::WrittenPitch> {
        let span = crate::resolve::trimmed_span(node);
        match node.kind() {
            SyntaxKind::ParenExpr | SyntaxKind::BlockExpr => {
                let inner = child(node, is_expr_node)?;
                self.written_pitch(&inner, reading)
            }
            SyntaxKind::LiteralExpr | SyntaxKind::NameExpr => {
                let text = node.to_string().trim().to_owned();
                crate::WrittenPitch::parse(&text).or_else(|| self.refuse(Self::not_a_pitch(&text, span)))
            }
            SyntaxKind::PitchExpr => {
                let parts = children(node, is_expr_node);
                let base = self.written_pitch(parts.first()?, reading)?;
                let text = parts.get(1)?.to_string().trim().to_owned();
                let Some(interval) = crate::Interval::parse(&text, writes(node, SyntaxKind::DownKw)) else {
                    return self.refuse(
                        Diagnostic::error(Code::NotAValue, format!("`{text}` is not an interval"))
                            .at(span, "unknown interval"),
                    );
                };
                base.transpose(interval).or_else(|| self.out_of_range(span))
            }
            SyntaxKind::StepExpr => {
                let parts = children(node, is_expr_node);
                let base = self.written_pitch(parts.first()?, reading)?;
                let written = parts.get(1)?.to_string().trim().to_owned();
                let Ok(count) = written.parse::<i64>() else {
                    return self.refuse(Self::not_a_count(&written, span));
                };
                let steps = if writes(node, SyntaxKind::DownKw) {
                    let Some(down) = count.checked_neg() else {
                        return self.out_of_range(span);
                    };
                    down
                } else {
                    count
                };
                let scale = match reading.scale {
                    Some(Counting::Written(scale)) => scale,
                    Some(Counting::Bound) => {
                        return self.refuse(
                            Diagnostic::error(Code::UnsupportedLanguageStage, "`step` needs a scale it can count")
                                .at(span, "the enclosing `in scale` names a scale rather than spelling one")
                                .help("write the collection out, as `in scale c major { … }`")
                                .note(
                                    "a step walks a frame of the collection around this pitch, and no registered \
                                     operation builds one",
                                ),
                        );
                    }
                    None => {
                        return self.refuse(
                            Diagnostic::error(Code::Misplaced, "`step` needs a scale to count in")
                                .at(span, "no `in scale` encloses this")
                                .help("put the passage in `in scale c major { … }`, naming the collection this steps through")
                                .note("an absent scale is never an implicit C major: a step is a coordinate move and a coordinate needs a system"),
                        );
                    }
                };
                self.stepped(base, scale, steps, span)
            }
            _ => self.refuse(
                Diagnostic::error(Code::NotAValue, "this is not a written pitch")
                    .at(span, "expected a pitch, a transposition, or a scale step"),
            ),
        }
    }

    /// `base`, moved `steps` degrees through `scale`.
    ///
    /// The old checker's arithmetic, called where the old checker called it. It
    /// is `crate::scale`'s and none of it moves — what changes is only that the
    /// answer becomes a literal here rather than being deferred to a stage that
    /// no longer exists.
    fn stepped(
        &mut self,
        base: crate::WrittenPitch,
        scale: crate::scale::Scale,
        steps: i64,
        span: SourceSpan,
    ) -> Option<crate::WrittenPitch> {
        let Some(frame) = crate::scale::Frame::around(scale, base) else {
            return self.out_of_range(span);
        };
        let Some(degree) = frame.locate(base) else {
            return self.refuse(
                Diagnostic::error(Code::NotAValue, format!("`{base}` is not in {scale}"))
                    .at(span, "this scale has no degree there")
                    .note("a step counts through the collection in force, so the pitch it starts from has to be one of its degrees"),
            );
        };
        let moved = degree.step(steps)?;
        frame.pitch(moved).or_else(|| self.out_of_range(span))
    }

    /// What an `in scale` put in force: a spelled collection, or a bound name.
    ///
    /// The two are told apart by the node the parser built and not by trying to
    /// read the text both ways: `in scale c dorian` writes a scale expression
    /// and `in scale mode` writes a name, and asking the CST which one is there
    /// is the same reading [`super::values`] does one statement over.
    fn counting(&mut self, node: &SyntaxNode) -> Option<Counting> {
        if named(node) {
            // Read for its refusals — an unbound name is still an error here —
            // and discarded, because `in scale` contributes no fact of its own
            // and a `step` under a bound scale is refused where it is written.
            self.value(node)?;
            return Some(Counting::Bound);
        }
        self.written_scale(node).map(Counting::Written)
    }

    /// `scale c dorian`, as the collection it names.
    fn written_scale(&mut self, node: &SyntaxNode) -> Option<crate::scale::Scale> {
        let span = crate::resolve::trimmed_span(node);
        let written = node.to_string();
        let mut words = written.split_whitespace().skip_while(|word| *word == "scale");
        let tonic = words.next().unwrap_or_default();
        let Some(tonic) = crate::PitchClass::parse(tonic) else {
            return self.refuse(
                Diagnostic::error(Code::NotAValue, format!("`{tonic}` is not a pitch class"))
                    .at(span, "expected a spelled tonic, such as `c` or `f#`"),
            );
        };
        let word = words.next().unwrap_or_default();
        let Some(collection) = crate::scale::Collection::named(word) else {
            return self.refuse(
                Diagnostic::error(Code::UnknownName, format!("unknown collection `{word}`"))
                    .at(span, "not a named scale collection")
                    .note("a mode is a rotation of the diatonic collection; other collections are their own values"),
            );
        };
        Some(crate::scale::Scale::new(tonic, collection))
    }

    /// The articulation names written after a duration, as a `List Mark`.
    fn articulations(&mut self, origin: Origin, names: &[String], span: SourceSpan) -> Raw {
        let mut marks = Vec::new();
        for name in names {
            match crate::Mark::parse(name) {
                Some(mark) => marks.push(payload(origin, "Mark", mark)),
                None => {
                    self.resolver.report(
                        Diagnostic::error(Code::UnknownWord, format!("`{name}` is not an articulation"))
                            .at(span, "unknown articulation")
                            .help(crate::resolve::suggest(name, &crate::marks::note_names(), "marks")),
                    );
                }
            }
        }
        listed(origin, marks)
    }

    /// The `Origin` argument a constructed fact carries (§5.7).
    ///
    /// `pub(super)` for one caller outside this module: [`super::values`] reads a
    /// written `play(v, d)` as the four-argument application, and the two
    /// arguments it supplies are these.
    pub(super) fn provenance_at(&self, origin: Origin) -> Raw {
        let span = self.sites.span(origin).unwrap_or_default();
        self.provenance(origin, span)
    }

    /// The same, when the caller already holds the span.
    #[expect(
        clippy::unused_self,
        reason = "reads as a sibling of `provenance_at`, which needs the table"
    )]
    fn provenance(&self, origin: Origin, span: SourceSpan) -> Raw {
        let written = crate::origin::Origin {
            source_span: span,
            definition_span: span,
            // Zero, which `factext.rs` already reads as "no declaration to
            // name" — it prints `#n` only for a non-zero one. This module walks
            // a block, and which declaration encloses it is what prompt 142's
            // piece walk knows.
            declaration: crate::origin::DeclarationId(0),
            // Empty by construction: expansion happened in the phase, and what
            // this module reads is the answer it left behind.
            expansion_path: Vec::new(),
        };
        Raw::lit(origin, crate::registry::provenance_literal(written))
    }

    /// A statement §3 forbids inside a value usable at several places.
    ///
    /// *Misplaced* rather than *unsupported*: these are permanent answers, and
    /// the same statement in a score is perfectly legal. A diagnostic that said
    /// "not supported yet" would be a promise nobody intends to keep.
    fn misplaced<T>(&mut self, what: &str, span: SourceSpan) -> Option<T> {
        self.refuse(
            Diagnostic::error(Code::Misplaced, format!("{what} cannot stand in a `music` value"))
                .at(span, "this says \"from here onward\"")
                .help("write it in the voice or part this music is used in")
                .note(
                    "a `music` value is usable at several places, and \"from here onward\" has no unique meaning there",
                ),
        )
    }

    fn not_a_pitch(text: &str, span: SourceSpan) -> Diagnostic {
        Diagnostic::error(Code::NotAValue, format!("`{text}` is not a pitch"))
            .at(span, "expected a pitch")
            .note("a pitch is a letter, an optional `#` or `b`, and an octave: `c4`, `g#5`, `bb3`")
    }

    fn not_a_dynamic(text: &str, span: SourceSpan) -> Diagnostic {
        Diagnostic::error(Code::UnknownWord, format!("`{text}` is not a dynamic marking"))
            .at(span, "unknown marking")
            .help(crate::resolve::suggest(
                text,
                crate::score::DynamicMark::NAMES,
                "markings",
            ))
    }

    fn not_a_count(text: &str, span: SourceSpan) -> Diagnostic {
        Diagnostic::error(Code::NotAValue, format!("`{text}` is not a count")).at(span, "expected a whole number")
    }

    fn out_of_range<T>(&mut self, span: SourceSpan) -> Option<T> {
        self.refuse(
            Diagnostic::error(Code::OutOfRange, "this pitch leaves Musa's exact range")
                .at(span, "the written coordinates do not fit"),
        )
    }
}

/// How long `node`'s statements last, read off the source.
///
/// Written durations are what a block is made of, so the extent of one is a sum
/// this module can do — and doing it here is what keeps a `duration` builtin off
/// the source, which would make a track's length part of every caller's control
/// flow. Saturating rather than wrapping: a block long enough to overflow exact
/// rational addition is one no page could hold.
///
/// Two arms and no list of point annotations: a statement that sounds carries a
/// written duration, and one that does not encloses no statements, so it sums to
/// nothing by the recursion rather than by being named. A list would have to
/// name `mark` twice — `mark accel { … }` spans its body and `mark fermata;`
/// does not — which is the shape of an enumeration that has stopped agreeing
/// with what it enumerates.
#[expect(
    clippy::arithmetic_side_effects,
    reason = "`Ratio<i64>` addition is exact mathematical arithmetic rather than raw integer ops, which is the same argument `realize.rs`, `assert.rs`, `resolve.rs`, and `time.rs` make at module scope; scoped to this function because it is the only arithmetic here"
)]
pub(super) fn extent(node: &SyntaxNode) -> Ratio<i64> {
    let mut total = Ratio::ZERO;
    for statement in statements(node) {
        let each = match statement.kind() {
            SyntaxKind::NoteStmt
            | SyntaxKind::RestStmt
            | SyntaxKind::ChordStmt
            | SyntaxKind::StackStmt
            | SyntaxKind::ImproviseStmt => {
                crate::resolve::parse_duration(&statement).map_or(Ratio::ZERO, |written| written.value.as_ratio())
            }
            _ => extent(&statement),
        };
        total += each;
    }
    total
}

/// A duration in written time, as the literal the track builtins take.
///
/// Not [`plain`], because `Duration` is registered at `Coordinate → Type 0` and
/// the literal a signature accepts is at `Duration ⟨written⟩`: a bare `Duration`
/// is a different type, and the core says so.
fn written_duration(origin: Origin, held: Ratio<i64>) -> Raw {
    Raw::lit(
        origin,
        crate::registry::literal(
            crate::registry::tagged_type("Duration", crate::core::Coordinate::WrittenTime),
            held,
        ),
    )
}

/// `Scope.Piece`, `Scope.Part n`, `Scope.Voice p v`.
pub(super) fn scope_of(origin: Origin, scope: crate::Scope) -> Raw {
    match scope {
        crate::Scope::Piece => Raw::var(origin, "Scope.Piece"),
        crate::Scope::Part { part } => Raw::app(origin, Raw::var(origin, "Scope.Part"), whole(origin, u64::from(part))),
        crate::Scope::Voice { part, voice } => applied(
            origin,
            Raw::var(origin, "Scope.Voice"),
            [whole(origin, u64::from(part)), whole(origin, u64::from(voice))],
        ),
    }
}

/// A literal at a plain base type whose payload has a written spelling.
fn plain<T>(origin: Origin, base: &'static str, value: T) -> Raw
where
    T: PartialEq + std::fmt::Debug + std::fmt::Display + Send + Sync + 'static,
{
    Raw::lit(
        origin,
        crate::registry::literal(crate::registry::plain_type(base), value),
    )
}

/// A literal at a plain base type whose payload has none.
fn payload<T>(origin: Origin, base: &'static str, value: T) -> Raw
where
    T: Clone + PartialEq + std::fmt::Debug + Send + Sync + 'static,
{
    Raw::lit(origin, crate::registry::opaque_literal(base, value))
}

/// `Option.None` or `Option.Some v`, from a payload that may not be there.
fn optional<T>(origin: Origin, base: &'static str, value: Option<T>) -> Raw
where
    T: Clone + PartialEq + std::fmt::Debug + Send + Sync + 'static,
{
    maybe(origin, value.map(|held| payload(origin, base, held)))
}

// ---- the four context facts, written ----
//
// Free functions rather than arms of [`Lowering::fact`] because there are two
// readers and only one of them starts from a node: a *part's* clef, meter, and
// tempo are read by [`crate::resolve::part_facts`], which already carries the
// refusals a second clef and an unreadable meter need, and what is left for
// [`super::piece`] to do is write down what that reading answered. Splitting the
// parse from the writing is what keeps one spelling of `Fact.Key` in the
// compiler rather than two.

/// `Fact.Key(key)`, from a key the source spelled out.
pub(super) fn keyed(origin: Origin, key: crate::score::Key) -> Raw {
    Raw::app(origin, Raw::var(origin, "Fact.Key"), plain(origin, "Key", key))
}

/// `Fact.Meter(numerator, denominator)`.
pub(super) fn metered(origin: Origin, meter: crate::score::Meter) -> Raw {
    applied(
        origin,
        Raw::var(origin, "Fact.Meter"),
        [
            whole(origin, u64::from(meter.numerator())),
            whole(origin, u64::from(meter.denominator())),
        ],
    )
}

/// `Fact.Clef(clef)`.
pub(super) fn clefed(origin: Origin, clef: crate::score::Clef) -> Raw {
    Raw::app(origin, Raw::var(origin, "Fact.Clef"), payload(origin, "Clef", clef))
}

/// `Fact.Tempo(metronome, text, ramp)`, from what the statement said.
///
/// The three arguments are the three answers [`crate::resolve::Marking`] holds,
/// in the order `registry::notation` reads them back. `text` is written with
/// [`plain`] rather than [`payload`] because `Text`'s payload is a `String` and
/// the reader asks for one; the other two are opaque.
pub(super) fn tempo(origin: Origin, marking: &crate::resolve::Marking) -> Raw {
    applied(
        origin,
        Raw::var(origin, "Fact.Tempo"),
        [
            optional(origin, "Metronome", marking.metronome),
            maybe(origin, marking.text.clone().map(|text| plain(origin, "Text", text))),
            optional(origin, "Ramp", marking.ramp.clone()),
        ],
    )
}

/// The same, from a term that may not be there.
fn maybe(origin: Origin, value: Option<Raw>) -> Raw {
    match value {
        None => Raw::var(origin, "Option.None"),
        Some(held) => Raw::app(origin, Raw::var(origin, "Option.Some"), held),
    }
}

/// Whether `node` *names* a value rather than spelling one out.
///
/// The parser already told the two apart, and this reads its answer rather than
/// trying the text both ways: `key g major` and `scale c dorian` are their own
/// expression forms, and a bare name or a qualified path is a reference to a
/// binder — a template's parameter, or a value a module holds.
fn named(node: &SyntaxNode) -> bool {
    matches!(node.kind(), SyntaxKind::NameExpr | SyntaxKind::PathExpr)
}

/// `3/2`, as a tuplet's two counts, unreduced as the backends need them.
fn tuplet_ratio(text: &str) -> Option<(u32, u32)> {
    let (num, den) = text.split_once('/')?;
    Some((count_of(num)?, count_of(den)?))
}

/// A whole number written as a count.
fn count_of(text: &str) -> Option<u32> {
    text.trim().parse().ok()
}
