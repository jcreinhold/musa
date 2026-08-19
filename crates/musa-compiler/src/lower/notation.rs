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

use super::{
    Lowering, applied, child, children, is_expr_node, is_type_node, listed, significant_tokens, whole, writes,
};
use crate::diagnose::{Code, Diagnostic};
use crate::origin::{DeclarationId, SourceSpan};
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
    /// Decides the `source_span` every event built here carries: material
    /// usable at several places came from every one of them, so an unplaced
    /// reading writes [`crate::elaborate::SHARED_ORIGIN`] and each use fills it
    /// in. Not derivable from [`Reading::scope`]: a free `music { … }` value
    /// reads at [`crate::Scope::Piece`] and so does a piece's own header, and
    /// the difference between them is not what the fact is *about* but whether
    /// the text is spoken in one place.
    placed: bool,
    /// Whether an enclosing `repeat` speaks this block more than once.
    ///
    /// A second bit rather than a wider meaning for [`Reading::placed`],
    /// because the two questions come apart at exactly one construct and answer
    /// different callers. A repeat body *is* written at one place, so its
    /// events keep their own span and each pass is told apart by
    /// [`crate::origin::ExpansionStep::RepeatIteration`] — but it is played `n`
    /// times, so a statement meaning "from here onward" has `n` heres. Folding
    /// the two together would either hand every note in a repeat
    /// [`crate::elaborate::SHARED_ORIGIN`] forever or admit a meter change that
    /// takes effect at a different bar on every pass.
    ///
    /// Set for *any* repeat rather than for a repeat of more than one pass:
    /// `repeat 4 to 16 { … }` leaves the count to the performance
    /// (`11-realization.md`), so "played once" is not a property this reading
    /// may rely on.
    repeated: bool,
    /// The declaration this block is, or is written in.
    ///
    /// A motif, a fragment, a named bar, a free `music` value, and a voice each
    /// ask [`super::Sites::declaring`] for one, and everything read under the
    /// reading they make names it. Carried here rather than on the walk because
    /// a walk reads several: [`super::piece`] enters every voice of the score
    /// with one [`Lowering`], and a body nested inside a voice belongs to that
    /// voice however deep it is.
    declaration: DeclarationId,
    /// The meter in force where this block begins.
    ///
    /// Read lexically, exactly as [`Reading::scale`] is, and for the same
    /// reason: `senza { … }` restores the meter that was already in force, and a
    /// fold has no cursor to ask. One statement reads it and one writes it, so
    /// what it costs is a copy of two `u32`s per nesting.
    meter: crate::score::Meter,
    /// How much an enclosing tuplet scales the durations written here.
    ///
    /// Read lexically like [`Reading::scale`] and [`Reading::meter`], and
    /// *multiplied* rather than replaced, because tuplets nest: a triplet inside
    /// a triplet scales by four ninths, not by two thirds announced twice.
    /// Roadmap §2's "notated duration ≠ performed duration" one row before
    /// performance — a triplet eighth is drawn as an eighth and lasts a twelfth,
    /// so the value moves and the spelling stays.
    tuplet: Ratio<i64>,
}

/// One name the document declares as material, and what a `mobile` asks of it.
///
/// The two questions are together because they are answered together — see
/// [`Lowering::declared_material`] — and a caller that had the kind without the
/// extent would have to walk the document a second time to place the fragment
/// it just accepted.
struct Reach {
    /// Whether the name is a motif, a bar, or a fragment.
    material: crate::resolve::Material,
    /// How far the material reaches, as written.
    reaches: Ratio<i64>,
}

/// The named material a document declares, by name.
type Declarations = std::collections::HashMap<String, Reach>;

impl Reading {
    /// The reading a free-standing `music { … }` value is read under.
    ///
    /// [`crate::Scope::Piece`] because a fragment is "usable at several places"
    /// and so has no voice of its own, and no scale because `01-surface.md` §2
    /// refuses an implicit C major.
    fn free(declaration: DeclarationId) -> Self {
        Self {
            scope: crate::Scope::Piece,
            scale: None,
            placed: false,
            repeated: false,
            declaration,
            meter: crate::score::Meter::default(),
            tuplet: Ratio::ONE,
        }
    }

    /// The reading a body written at one place in the piece is read under.
    pub(super) fn at(scope: crate::Scope, declaration: DeclarationId) -> Self {
        Self {
            scope,
            scale: None,
            placed: true,
            repeated: false,
            declaration,
            meter: crate::score::Meter::default(),
            tuplet: Ratio::ONE,
        }
    }

    /// The same reading, inside a `repeat` that speaks it again.
    ///
    /// One-way: nothing nested inside a repeat is played once, so a reading
    /// never becomes unrepeated on the way down.
    const fn again(self) -> Self {
        Self { repeated: true, ..self }
    }

    /// Whether a statement meaning "from here onward" may stand in this block.
    ///
    /// Both bits, and neither alone: the here must exist ([`Reading::placed`])
    /// and there must be exactly one of it ([`Reading::repeated`]).
    const fn permanent(self) -> bool {
        self.placed && !self.repeated
    }

    /// The same reading, inside a tuplet that scales durations by `factor`.
    ///
    /// Multiplies rather than replaces, which is the whole of what makes nested
    /// tuplets compose: `tuplet 3/2 { tuplet 5/4 { … } }` scales by eight
    /// fifteenths, and neither statement has to know the other is there.
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "`Ratio<i64>` multiplication is exact mathematical arithmetic rather than raw integer ops, the same argument `Lowering::lasts` makes below; scoped here because it is the only arithmetic on a reading"
    )]
    fn inside(self, factor: Ratio<i64>) -> Self {
        Self {
            tuplet: self.tuplet * factor,
            ..self
        }
    }

    /// How long a duration written under this reading lasts, and the freedom on
    /// it.
    ///
    /// [`crate::score::NotatedDuration::scaled`] and not `stretched`: a tuplet
    /// keeps the symbol the engraver draws and moves only what it sounds for, so
    /// a triplet eighth stays spelled `1/8` and carries the value `1/12`. The
    /// freedom moves with it because it is measured in the same time — `c5/4 to
    /// 2/1` inside a triplet may be held to two thirds of a double whole, not to
    /// a double whole.
    fn lasting(
        self,
        written: (NotatedDuration, Option<crate::score::FreeDuration>),
    ) -> (NotatedDuration, Option<crate::score::FreeDuration>) {
        let (duration, free) = written;
        if self.tuplet == Ratio::ONE {
            return (duration, free);
        }
        (duration.scaled(self.tuplet), free.map(|held| held.scaled(self.tuplet)))
    }

    /// The same reading, counting steps in `scale`.
    const fn stepping(self, scale: Counting) -> Self {
        Self {
            scale: Some(scale),
            ..self
        }
    }

    /// The same reading, counting steps in the collection `key` suggests.
    ///
    /// The other half of "an absent scale is never an implicit C major": an
    /// absent scale under a *written* key is not absent and not implicit — the
    /// author wrote `key c minor`, and C natural minor is the collection that
    /// says. [`crate::scale::signature_scale`] is the same reading `key_scale`
    /// gives a program that asks for it as a value, so the default a step takes
    /// and the default a composer can name are one collection.
    ///
    /// A default and not a fact: an `in scale` inside overrides it by the
    /// ordinary nesting, because [`Self::stepping`] is applied to the reading
    /// this produced.
    pub(super) fn keyed(self, key: crate::score::Key) -> Self {
        self.stepping(Counting::Written(crate::scale::signature_scale(key)))
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

/// A written claim, spelled back: `pitches_in(scale c major)`, `fills_meter()`.
///
/// What Origin view prints for an `assert` and what `crate::factext` parses
/// back, so it is written from the source rather than from
/// [`crate::assert::Claim`]: the arguments are terms here, and a term has no
/// value until the document it stands in is elaborated. The name comes from the
/// registry row instead of the token, because the row was found by matching that
/// token exactly and a `&'static str` cannot be a spelling nothing claims.
///
/// Runs of whitespace close up so that a claim written across two lines reads as
/// one, which is the only difference this allows itself from the bytes.
fn spelled_claim(predicate: &crate::assert::Predicate, statement: &musa_language::ast::AssertStmt) -> String {
    let arguments: Vec<String> = statement
        .args()
        .iter()
        .map(|argument| {
            argument
                .syntax()
                .to_string()
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ")
        })
        .collect();
    format!("{}({})", predicate.name, arguments.join(", "))
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
        // The one place a block of notation that is not a voice begins, so the
        // one place that asks for its number: a motif, a fragment, a named bar,
        // and a `music` value all arrive here, and each is a declaration a fact
        // can name.
        let declaration = self.sites.declaring();
        self.notated(node, Reading::free(declaration))
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
    /// `motif name(param: τ, …) { … }` — the `fn` `01-surface.md` §2 says it
    /// desugars to, so the parameters are [`super::items::written_parameters`]
    /// like a `fn`'s and each annotation that was written binds: a `d:
    /// Duration<WrittenTime>` the grammar reads is not a spelling the λ drops.
    pub(super) fn motif(&mut self, node: &SyntaxNode) -> Option<super::items::Definition> {
        let origin = self.origin(node);
        let name = super::items::declared_name(node)?;
        let parameters = super::items::written_parameters(node);
        let mut value = self.music(node)?;
        for parameter in parameters.iter().rev() {
            let at = self.origin(parameter);
            let bound = super::items::declared_name(parameter)?;
            value = match child(parameter, is_type_node) {
                Some(written) => {
                    let domain = self.ty(&written)?;
                    musa_core::Raw::annotated_lam(at, bound, domain, value)
                }
                None => musa_core::Raw::lam(at, bound, value),
            };
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

    /// `bar refrain { … }` — the same `let` a fragment is, for the bar that has
    /// a name.
    ///
    /// A named bar is a declaration written where it sounds. The braces sound in
    /// place, which is [`Lowering::bar`]'s reading and states the measure claim;
    /// the name binds the same passage for every `use` that answers it. The
    /// caller has already read the name off the statement and hands it over —
    /// there is nothing here to re-derive from the tokens.
    ///
    /// # Why the passage is read a second time rather than shared
    ///
    /// The two readings are at two scopes. What sounds between the braces is the
    /// *voice's*, read under the meter and collection in force where it stands;
    /// what a `use` plays is material relabelled to wherever it is played, which
    /// is [`Lowering::music`]'s `Reading::free` and the same reading a fragment
    /// gets. Sharing one term would have to pick one of them, and either choice
    /// puts a fact in a scope its author did not write.
    ///
    /// # Why it is a definition and not a binder over the rest of the voice
    ///
    /// A binder introduced inside the fold is invisible to the *claims* the fold
    /// raises. A [`Claimed`] holds `before` and `passage` as terms that
    /// [`crate::document::Document::passage`] elaborates in the document's own
    /// context, so a claim written after `use refrain;` — which
    /// `examples/refrain.musa` writes — would carry a free `refrain` into a
    /// context that never bound it. Making the bar a declaration is what the
    /// replaced core did, and it is the reading under which one name-resolution
    /// mechanism answers every use of the name, claims included.
    pub(crate) fn named_bar(&mut self, node: &SyntaxNode, name: &str) -> Option<super::items::Definition> {
        Some(super::items::Definition {
            origin: self.origin(node),
            name: musa_core::Name::from(name),
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
        self.folded(node, statements(node), reading)
    }

    /// The same fold over a chosen subsequence of `node`'s statements.
    ///
    /// One caller passes anything but every statement: [`Lowering::repeat`]
    /// leaves the endings out of the body, because a volta is not part of what
    /// the passes have in common. `node` is still handed over, because the
    /// origin a claim is placed against belongs to the block rather than to the
    /// statements that survived a filter.
    fn folded(
        &mut self,
        node: &SyntaxNode,
        statements: impl Iterator<Item = SyntaxNode>,
        reading: Reading,
    ) -> Option<Raw> {
        let origin = self.origin(node);
        let mut placed = Placed::default();
        let mut whole = true;
        let mut reading = reading;
        for statement in statements {
            // A `meter` is in force from where it is written, so it is read
            // *before* the statement that wrote it is folded and stays in force
            // for everything after — which is the whole of what makes the
            // restoring half of `senza` the meter a composer expects.
            if let Some(written) = musa_language::ast::MeterStmt::cast(statement.clone())
                && let Some(meter) = crate::resolve::parse_meter(&written)
            {
                reading = reading.metered(meter);
            }
            // A `key` is read the same way and for the same reason: it is in
            // force from where it is written, and what it puts in force for a
            // `step` is the collection it suggests. Two statements read
            // lexically, and both write only forward.
            if let Some(written) = musa_language::ast::KeyStmt::cast(statement.clone())
                && let Some(key) = crate::resolve::parse_key(&written)
            {
                reading = reading.keyed(key);
            }
            let raised = self.claims.len();
            match self.statement(&statement, reading) {
                Some(next) => {
                    // Asked for only when a claim was raised: the prefix is a
                    // term of its own, and building one per statement would
                    // spend nodes on blocks that claim nothing.
                    if self.claims.len() > raised {
                        let before = placed.built(origin);
                        for claim in self.claims.iter_mut().skip(raised) {
                            let inside = claim.before.clone();
                            claim.before = applied(origin, Raw::hosted(origin, "follow"), [before.clone(), inside]);
                        }
                    }
                    placed.place(origin, next);
                }
                None => whole = false,
            }
        }
        whole.then(|| placed.built(origin))
    }

    /// `node`'s body under one transformation, and every claim raised inside it
    /// under the same one.
    ///
    /// A claim is about the passage *as instantiated* (`05-verification.md`), and
    /// a transformation block is part of how it was instantiated:
    /// `transpose up m2 { assert pitches_in(scale c major) { … } }` claims about
    /// the transposed notes, which is what makes one motif under two
    /// transpositions two verdicts. [`Claimed`] holds terms rather than notes —
    /// the notes do not exist until the document is elaborated — so the
    /// transformation is applied to the *terms*, here, where the reading still
    /// knows which one it is.
    ///
    /// Both of a claim's terms travel, for two reasons that happen to agree.
    /// `passage` is the music the claim is about, so a `transpose` changes what
    /// sounds in it; `before` is read only for its duration, so a `stretch`
    /// changes where the passage begins. [`Lowering::folded`] threads the
    /// *prefix* into `before` for the same reason this threads the
    /// *transformation* into both, and the two compose in written order — the
    /// fold prepends outside whatever this wrapped inside.
    ///
    /// The four transformation blocks are the whole of it, because they are the
    /// four statements that change what their body sounds. `in scale`, `senza`,
    /// a region, and a repeat each enclose a body and leave what is inside the
    /// braces sounding exactly as written, so a claim under one of those is
    /// already about the right music.
    fn transforming(&mut self, node: &SyntaxNode, reading: Reading, under: impl Fn(Raw) -> Raw) -> Option<Raw> {
        let raised = self.claims.len();
        let body = self.notated(node, reading)?;
        for claim in self.claims.iter_mut().skip(raised) {
            claim.before = under(claim.before.clone());
            claim.passage = under(claim.passage.clone());
        }
        Some(under(body))
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
                let (field, held, free) = self.notated_duration(node, span, reading)?;
                let fact = applied(
                    origin,
                    Raw::hosted(origin, "Fact.Rest"),
                    [
                        field,
                        listed(origin, Vec::new()),
                        optional(origin, "FreeDuration", free),
                    ],
                );
                Some(self.sounded_term(origin, reading, fact, held))
            }
            SyntaxKind::ChordStmt => self.chord_statement(node, origin, reading),
            SyntaxKind::StackStmt => self.stack(node, origin, reading),
            SyntaxKind::GraceStmt => self.grace(node, origin, reading),
            SyntaxKind::UseStmt => self.used(node, origin, reading),

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
                self.transforming(node, reading, |body| {
                    applied(
                        origin,
                        Raw::hosted(origin, "transpose"),
                        [plain(origin, "Interval", interval), body],
                    )
                })
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
                self.transforming(node, reading, |body| {
                    applied(
                        origin,
                        Raw::hosted(origin, "stretch"),
                        [plain(origin, "Ratio", factor), body],
                    )
                })
            }
            SyntaxKind::RetrogradeStmt => self.transforming(node, reading, |body| {
                Raw::app(origin, Raw::hosted(origin, "retrograde"), body)
            }),
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
                self.transforming(node, reading, |body| {
                    applied(
                        origin,
                        Raw::hosted(origin, "invert"),
                        [plain(origin, "Pitch", axis), body],
                    )
                })
            }

            // `in scale` changes what a `step` reads and denotes its body. It is
            // the one statement that contributes no fact of its own — and the
            // one that still has to *say* it was here, because a spelling like
            // `eb4` is the same written pitch whether the source wrote it or a
            // step arrived at it, and Origin view is where a reader asks which.
            SyntaxKind::InScaleStmt => {
                let Some(written) = musa_language::ast::InScaleStmt::cast(node.clone()).and_then(|s| s.scale_expr())
                else {
                    return self.refuse(
                        Diagnostic::error(Code::NotAValue, "`in scale` needs a scale")
                            .at(span, "expected a scale, such as `c dorian`"),
                    );
                };
                let scale = self.counting(&written)?;
                let body = self.notated(node, reading.stepping(scale))?;
                Some(Self::under_scale(node, origin, &written, body))
            }

            // The region annotations: a fact over the span its body covers.
            SyntaxKind::SlurStmt => self.region(node, origin, reading, Raw::hosted(origin, "Fact.Slur")),
            SyntaxKind::PhraseStmt => {
                let name = musa_language::ast::PhraseStmt::cast(node.clone())
                    .and_then(|stmt| stmt.name())
                    .unwrap_or_default();
                let fact = Raw::app(origin, Raw::hosted(origin, "Fact.Phrase"), plain(origin, "Text", name));
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
                    Raw::hosted(origin, "Fact.Tuplet"),
                    [whole(origin, u64::from(num)), whole(origin, u64::from(den))],
                );
                // The one region that changes what its body *means* rather than
                // only annotating it: `tuplet 3/2` is three in the time of two,
                // so an eighth written inside lasts a twelfth. The factor is put
                // in the reading rather than applied to the finished track,
                // because a track has no duration to rescale — the values are
                // already literals in the facts by then — and because reading it
                // lexically is what makes tuplets nest without either statement
                // knowing about the other.
                self.region(node, origin, reading.inside(tuplet_factor(node)), fact)
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
                    Raw::hosted(origin, "Fact.Hairpin"),
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
                    Raw::hosted(origin, "Fact.Dynamic"),
                    payload(origin, "DynamicMark", mark),
                );
                Some(self.sounded(origin, reading, fact, Ratio::ZERO))
            }
            SyntaxKind::SectionStmt => {
                let name = musa_language::ast::SectionStmt::cast(node.clone())
                    .and_then(|stmt| stmt.name())
                    .unwrap_or_default();
                let fact = Raw::app(origin, Raw::hosted(origin, "Fact.Section"), plain(origin, "Text", name));
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
                    Raw::hosted(origin, "Fact.Harmony"),
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
        let (field, held, free) = self.notated_duration(node, span, reading)?;
        let fact = applied(
            origin,
            Raw::hosted(origin, "Fact.Note"),
            [
                pitch,
                field,
                self.articulations(origin, &statement.articulations(), span),
                optional(origin, "FreeDuration", free),
            ],
        );
        Some(continuing(
            origin,
            statement.tied(),
            self.sounded_term(origin, reading, fact, held),
        ))
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
        let (field, held, free) = self.notated_duration(node, span, reading)?;
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
                Raw::hosted(origin, "Fact.Note"),
                [
                    plain(origin, "Pitch", pitch),
                    field.clone(),
                    articulations.clone(),
                    optional(origin, "FreeDuration", free),
                ],
            );
            let one = self.sounded_term(origin, reading, fact, held.clone());
            sounding = Some(match sounding {
                None => one,
                Some(built) => applied(origin, Raw::hosted(origin, "together"), [built, one]),
            });
        }
        // Outside the `together` and not on each note, because a chord's `~` is
        // written once and is about the chord: marking the notes separately
        // would say the same thing as many times as there are pitches.
        sounding.map(|track| continuing(origin, statement.tied(), track))
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
            // A pitch class is refused with its own sentence rather than the
            // generic one. `stack c major7` is not a typo for a pitch: it names
            // a class, and the answer is that stacking sounds notes and a class
            // chooses no octave. Supplying one would be Musa deciding a
            // register the composer did not write.
            if statement.root_is_class() {
                return self.refuse(
                    Diagnostic::error(Code::NotAValue, "a stacked chord needs a register")
                        .at(span, "expected a written pitch here")
                        .note("`stack c4 major7/2` sounds notes, and a pitch class chooses no octave"),
                );
            }
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
        let (_, held, _) = self.notated_duration(node, span, reading)?;
        let class = crate::chord::ChordClass::new(bass.pitch_class(), kind);
        let Ok(voicing) = crate::chord::Voicing::close_position(class, bass) else {
            return self.refuse(
                Diagnostic::error(Code::OutOfRange, "this chord does not stack above that bass")
                    .at(span, "the written coordinates leave Musa's exact range"),
            );
        };
        let call = applied(
            origin,
            Raw::hosted(origin, "play"),
            [
                self.provenance(origin, span, reading.placed, reading.declaration),
                scope_of(origin, reading.scope),
                // `plain` and not `payload`: `play` reads a `Voicing` and not an
                // `Opaque<Voicing>`, and a literal at the wrong Rust type
                // downcasts to nothing, which the core reports as this
                // compiler's table disagreeing with itself.
                plain(origin, "Voicing", voicing),
                held,
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
                Raw::hosted(at, "Fact.Grace"),
                [
                    plain(at, "Pitch", pitch),
                    self.articulations(at, &note.articulations(), span),
                    whole(at, index as u64),
                ],
            );
            let one = self.sounded(at, reading, fact, Ratio::ZERO);
            built = applied(origin, Raw::hosted(origin, "follow"), [built, one]);
        }
        Some(built)
    }

    /// `mark breath;` and `mark pedal { … }`.
    ///
    /// The one annotation that is a point *or* a region depending on how it was
    /// written, which is exactly what [`crate::elaborate::FactKind::Mark`]'s own
    /// documentation says decides it: "which of the two this occurrence is, is
    /// its span — a point's is empty".
    ///
    /// # Why the row's shape is checked while reading
    ///
    /// `crate::marks`'s table says three things about each row — where it is
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
    fn marked(&mut self, node: &SyntaxNode, origin: Origin, reading: Reading) -> Option<Raw> {
        use crate::marks::Anchor;

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
    fn mark_argument(
        &mut self,
        mark: crate::Mark,
        statement: &musa_language::ast::MarkStmt,
        origin: Origin,
        span: SourceSpan,
    ) -> Option<Raw> {
        use crate::marks::{Argument, MarkArgument};

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

    /// `repeat 2 { … }` and `repeat 4 to 16 { … }` — every pass, written out,
    /// under one fact that says it was written once.
    ///
    /// # Why the passes are folded rather than left to a reading
    ///
    /// `../../../rules/kernel/06-surface-elaboration.md` §2 makes `repeat n { … }`
    /// "an HIR-level `follow` of `n` evaluations", each iteration's occurrences
    /// carrying a [`crate::origin::ExpansionStep::RepeatIteration`] step. That is
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
    fn repeat(&mut self, node: &SyntaxNode, origin: Origin, reading: Reading) -> Option<Raw> {
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
    fn passes(
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
    fn brackets(
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
    fn expanded(origin: Origin, span: SourceSpan, iteration: u32, track: Raw) -> Raw {
        stamped(
            origin,
            span,
            crate::origin::ExpansionStep::RepeatIteration(iteration),
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
    fn ending(&mut self, node: &SyntaxNode, origin: Origin, reading: Reading) -> Option<Raw> {
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
    ///
    /// The one trace an assertion leaves in the music is
    /// [`crate::origin::ExpansionStep::Assertion`] on the facts inside the
    /// braces, which is Origin and therefore invisible to `≈facts` — the
    /// identity `05-verification.md` asks for, that a claim which holds gives
    /// back exactly the passage it was written on. The step carries the claim
    /// *as the source spells it*, for [`Self::under_scale`]'s reason: `pitches_in`
    /// alone would tell a reader an assertion was here and not which one, and
    /// the value the arguments have is a question this reading cannot answer.
    fn asserted(&mut self, node: &SyntaxNode, origin: Origin, reading: Reading) -> Option<Raw> {
        let statement = musa_language::ast::AssertStmt::cast(node.clone())?;
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
            before: Raw::lit(origin, crate::registry::empty_track()),
            // Without the step, because a claim is proved against what sounds
            // and provenance is not part of that. The music the *voice* gets
            // carries it.
            passage: passage.clone(),
        });
        Some(stamped(
            origin,
            span,
            crate::origin::ExpansionStep::Assertion {
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
        if !reading.permanent() {
            return self.misplaced("an unmeasured stretch", span);
        }
        let opened = self.sounded_at(
            origin,
            crate::Scope::Piece,
            reading.placed,
            reading.declaration,
            metered(origin, crate::score::Meter::NONE),
            Ratio::ZERO,
        );
        // Under `meter none` for its whole length, so the body is read with the
        // unmeasured meter in force: a `senza` inside a `senza` restores the one
        // its own braces opened, which is the one that was in force there.
        let body = self.notated(node, reading.metered(crate::score::Meter::NONE))?;
        let closed = self.sounded_at(
            origin,
            crate::Scope::Piece,
            reading.placed,
            reading.declaration,
            metered(origin, reading.meter),
            Ratio::ZERO,
        );
        Some(applied(
            origin,
            Raw::hosted(origin, "follow"),
            [applied(origin, Raw::hosted(origin, "follow"), [opened, body]), closed],
        ))
    }

    /// `mobile { a; b; c; }` — its fragments in an order the performance chose.
    ///
    /// The mobile rule of `docs/rules/kernel/11-realization.md`, which is the
    /// repeat rule again: the timeline holds the fragments *in the order they
    /// are played*, and one region fact carries the instruction the page prints
    /// over them. A timeline that held them as written and left the order to a
    /// later stage would be a timeline nothing downstream could measure — the
    /// same argument [`Lowering::repeat`] makes for unrolling.
    fn mobile(&mut self, node: &SyntaxNode, origin: Origin, reading: Reading) -> Option<Raw> {
        let statement = musa_language::ast::MobileStmt::cast(node.clone())?;
        let span = crate::resolve::trimmed_span(node);
        let names = statement.fragments();
        // One fragment in any order is the fragment. The refusal is the useful
        // part: a `mobile` with one name is almost always a half-finished edit.
        if names.len() < 2 {
            return self.refuse(
                Diagnostic::error(Code::NotAValue, "a mobile arranges at least two fragments")
                    .at(span, format!("this one lists {}", names.len()))
                    .help("write the fragment out instead, or add the ones it is arranged with"),
            );
        }
        let declared = self.declared_material(node);
        let tokens = statement.fragment_tokens();
        let order = self.resolver.decide_order(&self.choice, &names, span);
        let mut played = Vec::with_capacity(order.len());
        let mut over = Ratio::ZERO;
        // Every name is read even after one is refused: a mobile listing two
        // motifs is two mistakes, and stopping at the first would hide the
        // second behind a recompile.
        let mut arranged = true;
        for index in &order {
            let Some(name) = names.get(*index as usize) else {
                continue;
            };
            // The list and its tokens are read off the same tokens, so they
            // cannot disagree about which span this name is.
            let at = tokens.get(*index as usize).map_or(span, crate::resolve::source_span_of);
            let Some(track) = self.arranged(origin, reading, &declared, name, span, at) else {
                arranged = false;
                continue;
            };
            over += declared.get(name.as_str()).map_or(Ratio::ZERO, |held| held.reaches);
            played.push(track);
        }
        if !arranged {
            return None;
        }
        let fact = applied(
            origin,
            Raw::hosted(origin, "Fact.Mobile"),
            [
                listed(
                    origin,
                    names.into_iter().map(|name| plain(origin, "Text", name)).collect(),
                ),
                listed(
                    origin,
                    order.iter().map(|index| whole(origin, u64::from(*index))).collect(),
                ),
            ],
        );
        let marker = self.sounded(origin, reading, fact, over);
        Some(applied(
            origin,
            Raw::hosted(origin, "together"),
            [marker, followed(origin, played)],
        ))
    }

    /// One fragment of a mobile, by name, against what the document declares.
    ///
    /// A mobile arranges *fragments* and nothing else: a motif takes arguments
    /// and a bar is a measure, and neither is material a performance is invited
    /// to reorder. The refusal names what was found, so the mistake is one
    /// sentence rather than a type error one layer down.
    ///
    /// What comes back is what [`Lowering::used`] builds for `use f;`, because
    /// a name in a mobile's list *is* a use of that fragment — played here, in
    /// this voice, and recorded as an expansion so Origin view can say where a
    /// note came from.
    ///
    /// `declared` is handed in rather than looked up. It used to be
    /// `crate::resolve::Resolver::motifs`, which the replaced pass filled while
    /// it walked a piece's header and this reading never does — so under the new
    /// lowering every name in a mobile was refused as undeclared, which is
    /// nineteen refusals for `examples/mobile.musa` alone. The reading was
    /// already walking the document for the extents; asking that one walk what
    /// each name *is* keeps the fix on the side that has the answer, instead of
    /// filling one pass's map from another pass.
    fn arranged(
        &mut self,
        origin: Origin,
        reading: Reading,
        declared: &Declarations,
        name: &str,
        span: SourceSpan,
        at: SourceSpan,
    ) -> Option<Raw> {
        match declared.get(name).map(|held| held.material) {
            Some(crate::resolve::Material::Fragment) => {}
            Some(other) => {
                return self.refuse(
                    Diagnostic::error(
                        Code::Misplaced,
                        format!("`{name}` is a {}, not a fragment", other.word()),
                    )
                    .at(span, "a mobile arranges fragments")
                    .help(format!("declare it as `fragment {name} {{ … }}`")),
                );
            }
            None => {
                let known: Vec<&str> = declared.keys().map(String::as_str).collect();
                let help = crate::resolve::suggest_name(name, &known);
                return self.refuse(
                    Diagnostic::error(Code::UnknownName, format!("cannot find `{name}`"))
                        .at(span, "not declared in this piece")
                        .help(help),
                );
            }
        }
        self.resolver
            .references
            .record_use(crate::resolve::NameKind::Fragment, name, at);
        Some(Self::spoken(origin, reading, span, Raw::var(origin, name)))
    }

    /// The named material this document declares — what each name is, and how
    /// far it reaches.
    ///
    /// Both halves in one walk because a mobile asks both of every name it
    /// lists, and they are answered in the same place: a fragment's extent is
    /// the sum of the statements between its braces, and what makes it a
    /// fragment rather than a motif or a bar is which keyword opened them. A
    /// region's extent is otherwise read off the statements inside its own
    /// braces, and a mobile has none — what it writes are *names*.
    ///
    /// One pass over the document rather than a search per name, so a mobile of
    /// fifty-three figures costs one walk instead of fifty-three.
    fn declared_material(&self, node: &SyntaxNode) -> Declarations {
        use musa_language::ast::{BarStmt, FragmentDecl, MotifDecl};
        let Some(root) = node.ancestors().last() else {
            return Declarations::new();
        };
        let mut declared = Declarations::new();
        for held in root.descendants() {
            // A `bar` with no name declares nothing: it is a measure written
            // where it sounds, and only a *named* one is material `use` — or a
            // mobile — could ever reach.
            let named = if let Some(fragment) = FragmentDecl::cast(held.clone()) {
                fragment.name().map(|name| (name, crate::resolve::Material::Fragment))
            } else if let Some(motif) = MotifDecl::cast(held.clone()) {
                motif.name().map(|name| (name, crate::resolve::Material::Motif))
            } else if let Some(bar) = BarStmt::cast(held.clone()) {
                bar.name().map(|name| (name, crate::resolve::Material::Bar))
            } else {
                None
            };
            let Some((name, material)) = named else { continue };
            declared.insert(
                name,
                Reach {
                    material,
                    reaches: self.extent(&held),
                },
            );
        }
        declared
    }

    /// `improvise 8/1 over "Dm7 | G7";` — a frame that sounds as silence.
    fn improvise(&mut self, node: &SyntaxNode, origin: Origin, reading: Reading) -> Option<Raw> {
        let statement = musa_language::ast::ImproviseStmt::cast(node.clone())?;
        let span = crate::resolve::trimmed_span(node);
        let (_, held, _) = self.notated_duration(node, span, reading)?;
        let over = statement.over().map(|text| plain(origin, "Text", text));
        let fact = Raw::app(origin, Raw::hosted(origin, "Fact.Improvise"), maybe(origin, over));
        Some(self.sounded_term(origin, reading, fact, held))
    }

    /// `use e;` — the track `e` denotes, in this block's scope, folded on.
    ///
    /// Nothing checks that `e` is a track: §2 says `use e;` "checks that `e` is a
    /// written-time score track", and the check is the core's, because `scoped`'s
    /// signature demands one and the refusal lands at the origin this module gave
    /// the node.
    ///
    /// Two calls, and they are the two things a `use` says about material that
    /// was written somewhere else.
    ///
    /// `scoped` is what makes reusable material reusable. `e` was read at
    /// [`crate::Scope::Piece`] — a fragment is "usable at several places" and so
    /// has none of its own — and this is the place, so its facts take the scope
    /// of the block that played them. Here rather than around the whole voice,
    /// because a voice may itself write `key g major;`, which is a piece-scoped
    /// fact deliberately ([`Self::context`]) and would be relabelled into one
    /// voice's private key by a wrapper that could not tell the two apart. A
    /// `use` inside free material relabels `Piece` to `Piece` and costs a
    /// reduction step, which is the price of the rule having no exception.
    ///
    /// `instanced` is what makes it *this* playing of it.
    /// [`crate::origin::ExpansionStep::MotifApplication`] is the step Origin view reads to tell
    /// a composer's own notes from material spoken by name, and every consumer of
    /// it — the derivation graph's key, the fact-text spelling, and the
    /// repeat-agreement rule in [`crate::project`], which writes a repeat out
    /// rather than complaining when the repeat came from shared material — asks
    /// that question of a *use site*. The builtin and not a stamp applied while
    /// reading, for its own documented reason: the facts do not exist until the
    /// term is evaluated, and the ones a function `e` calls produced were read in
    /// another declaration entirely.
    ///
    /// Inside `scoped` rather than outside, because relabelling a scope and
    /// recording an expansion commute and the nesting should read the way the
    /// sentence does: this material, played here, belongs to this voice.
    fn used(&mut self, node: &SyntaxNode, origin: Origin, reading: Reading) -> Option<Raw> {
        let called = child(node, is_expr_node)?;
        let material = self.value(&called)?;
        let specialized = self.specialized(node, origin, material)?;
        Some(Self::spoken(
            origin,
            reading,
            crate::resolve::trimmed_span(node),
            specialized,
        ))
    }

    /// `material` with this `use`'s `with { note n = p; }` clause applied, or
    /// `material` unchanged when it wrote none.
    ///
    /// One `respelled` per override, innermost first, so a clause that names
    /// two notes reads as two edits of one occurrence rather than one edit of a
    /// list. Inside [`Lowering::spoken`]'s `instanced` rather than around it,
    /// because the respelling happened *within* this playing of the material
    /// and `04-provenance.md` reads the path outside-in: the note comes out
    /// `motif ▸ specialized`, which is the order a reader asking about it walks.
    ///
    /// **Three refusals here and three in the builtin, and the split is not
    /// arbitrary.** A position of zero, a clause with no pitch, and one note
    /// named twice are properties of the *text*, so they are answered where the
    /// text is. How many notes the occurrence has, whether the one named is a
    /// chord, and whether it is a rest are properties of the material, and the
    /// material is a term until it is evaluated — 141k's fold reported those by
    /// elaborating the body while it walked, and there is nothing to elaborate
    /// here. [`crate::registry::track`]'s `respelled` answers them on 141m's
    /// refusal channel instead.
    fn specialized(&mut self, node: &SyntaxNode, origin: Origin, material: Raw) -> Option<Raw> {
        use musa_language::ast::AstNode as _;

        let Some(call) = musa_language::ast::UseStmt::cast(node.clone()) else {
            return Some(material);
        };
        let mut named: Vec<u64> = Vec::new();
        let mut specialized = material;
        for each in call.overrides() {
            let at = crate::resolve::trimmed_span(each.syntax());
            let Some(position) = each
                .position()
                .and_then(|text| text.parse::<u64>().ok())
                .filter(|counted| *counted > 0)
            else {
                return self.refuse(
                    Diagnostic::error(Code::OutOfRange, "notes are counted from `note 1`")
                        .at(at, "there is no note 0")
                        .note("the first note of the occurrence is `note 1`"),
                );
            };
            let Some(pitch) = each.pitch().as_deref().and_then(crate::pitch::WrittenPitch::parse) else {
                return self.refuse(
                    Diagnostic::error(Code::NotAValue, "this override names no pitch")
                        .at(at, "expected a pitch")
                        .note("`note 2 = f5;` writes `f5` onto the second note"),
                );
            };
            if named.contains(&position) {
                return self.refuse(
                    Diagnostic::error(Code::DuplicateName, format!("note {position} is overridden twice"))
                        .at(at, "the second of two")
                        .note("one note takes one spelling, so one of these two says nothing"),
                );
            }
            named.push(position);
            let step = crate::lower::expansion(at, crate::origin::ExpansionStep::Specialization { override_site: at });
            specialized = applied(
                origin,
                Raw::hosted(origin, "respelled"),
                [
                    Raw::lit(origin, crate::registry::origin_literal(step)),
                    crate::lower::whole(origin, position),
                    plain(origin, "Pitch", pitch),
                    specialized,
                ],
            );
        }
        Some(specialized)
    }

    /// `material`, played here — the two builtins [`Lowering::used`] documents,
    /// with `at` as the call site.
    ///
    /// Its own function because [`Lowering::arranged`] wants the same two: a
    /// name in a mobile's list is material spoken by name at a place, which is
    /// what `use` is.
    fn spoken(origin: Origin, reading: Reading, at: SourceSpan, material: Raw) -> Raw {
        let played = stamped(
            origin,
            at,
            crate::origin::ExpansionStep::MotifApplication { call_site: at },
            material,
        );
        applied(
            origin,
            Raw::hosted(origin, "scoped"),
            [scope_of(origin, reading.scope), played],
        )
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
        self.sounded_at(origin, reading.scope, reading.placed, reading.declaration, fact, held)
    }

    /// The same, with the sounding duration already a term: a duration written
    /// as a parameter is a value the evaluation reads, not one this lowering
    /// can bake.
    fn sounded_term(&self, origin: Origin, reading: Reading, fact: Raw, held: Raw) -> Raw {
        self.sounded_in(origin, reading.scope, reading.placed, reading.declaration, fact, held)
    }

    /// The same, at a scope the reading does not supply.
    ///
    /// Three callers want one: a `clef` written in a voice is the *part's*
    /// clef, a `key` or a `meter` written in a voice is the *piece's*, and the
    /// header facts [`super::piece`] builds belong to the part or the piece that
    /// wrote them rather than to any voice. §5.7 asks which scope a fact is
    /// constructed at, and the answer is not always the scope it was written in.
    pub(super) fn sounded_at(
        &self,
        origin: Origin,
        scope: crate::Scope,
        placed: bool,
        declaration: DeclarationId,
        fact: Raw,
        held: Ratio<i64>,
    ) -> Raw {
        self.sounded_in(origin, scope, placed, declaration, fact, written_duration(origin, held))
    }

    /// The call, with all four arguments in hand.
    ///
    /// `pub(super)` for [`super::piece`]'s header: a header fact's `held` is
    /// the piece's evaluated extent, which is a term, not the ratio a
    /// statement's own written duration gives [`Self::sounded_at`].
    pub(super) fn sounded_in(
        &self,
        origin: Origin,
        scope: crate::Scope,
        placed: bool,
        declaration: DeclarationId,
        fact: Raw,
        held: Raw,
    ) -> Raw {
        applied(
            origin,
            Raw::hosted(origin, "sounded"),
            [
                self.provenance_at(origin, placed, declaration),
                scope_of(origin, scope),
                fact,
                held,
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
        if !reading.permanent() {
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
        Some(self.sounded_at(origin, scope, reading.placed, reading.declaration, fact, Ratio::ZERO))
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
                    return Some(Raw::app(origin, Raw::hosted(origin, "Fact.Key"), named));
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
    ///
    /// `inside` is the reading the *body* is read under, which for every region
    /// but a tuplet is the reading the region itself stands in. The marker is
    /// built under it too, because the one thing [`Lowering::sounded`] takes from
    /// a reading is its scope and a region never changes that. What the marker
    /// spans is [`Lowering::lasts`] rather than [`Lowering::extent`], because a
    /// tuplet's own ratio is part of how long the *statement* lasts and no part
    /// of how long its body reaches.
    fn region(&mut self, node: &SyntaxNode, origin: Origin, inside: Reading, fact: Raw) -> Option<Raw> {
        let body = self.notated(node, inside)?;
        let over = self.lasts(node);
        let marker = self.sounded(origin, inside, fact, over);
        Some(applied(origin, Raw::hosted(origin, "together"), [marker, body]))
    }

    /// A written duration and the freedom written on it, as long as it lasts
    /// where it stands.
    ///
    /// The reading is read last rather than first: [`Lowering::held`] decides
    /// how far a `to` is taken in the time the composer *wrote*, on the
    /// realization's own sixteenth-note grid, and [`Reading::lasting`] then
    /// carries the decided value into the tuplet's time. Deciding first and
    /// scaling after is what keeps [`Lowering::lasts`] a measurement of the
    /// written tree plus the decisions already recorded, rather than one that
    /// depends on a tuplet's body having been lowered before the region above it
    /// asks how far it reaches.
    /// A statement's duration as the two *terms* the fact and the span need:
    /// the `NotatedDuration` field of the fact, and the `Duration ⟨written⟩`
    /// the fact sounds for. A written fraction bakes both in as literals; a
    /// parameter is a term already, so the field goes through the
    /// `notated_duration` δ word, which evaluation fires once the call has
    /// given the parameter its value.
    ///
    /// The pair is read here rather than at each caller for
    /// [`NotatedDuration::spelled`]'s reason: one duration has one spelling,
    /// and six statements reading one duration six ways is six spellings of
    /// it.
    ///
    /// Inside a tuplet a parameter's term is scaled the way
    /// [`Reading::lasting`] scales a written one, and the spelling the rule
    /// then derives is of the *sounding* value: a computed duration has no
    /// written symbol for the annotation to point back at, which is the one
    /// asymmetry with the written case a parameter cannot avoid.
    fn notated_duration(
        &mut self,
        node: &SyntaxNode,
        span: SourceSpan,
        reading: Reading,
    ) -> Option<(Raw, Raw, Option<crate::score::FreeDuration>)> {
        let origin = self.origin(node);
        let Some(duration) = crate::resolve::parse_duration(node) else {
            if let Some(parameter) = musa_language::ast::Duration::of(node).and_then(|written| written.parameter()) {
                let term = if reading.tuplet == Ratio::ONE {
                    Raw::var(origin, parameter)
                } else {
                    applied(
                        origin,
                        Raw::hosted(origin, "duration_scale"),
                        [Raw::var(origin, parameter), plain(origin, "Ratio", reading.tuplet)],
                    )
                };
                let field = applied(origin, Raw::hosted(origin, "notated_duration"), [term.clone()]);
                return Some((field, term, None));
            }
            return self.refuse(
                Diagnostic::error(Code::NotAValue, "this statement has no duration")
                    .at(span, "expected a duration")
                    .note("a duration is a fraction or a whole number of whole notes: `1/4`, `3/8`, `1`"),
            );
        };
        let (duration, free) = match musa_language::ast::Duration::of(node).and_then(|written| written.held_to()) {
            None => reading.lasting((duration, None)),
            Some(most) => reading.lasting(self.held(duration, &most, span)?),
        };
        let held = written_duration(origin, duration.value.as_ratio());
        Some((payload(origin, "NotatedDuration", duration), held, free))
    }

    /// `g4/4 to 2/1` — a quarter the performer may hold to a double whole.
    ///
    /// Roadmap §2's row with both values kept rather than one standing in for
    /// the other: what comes back as the duration is what the note *sounds*, so
    /// everything after it lands where it should, and the
    /// [`crate::score::FreeDuration`] beside it is what recovers the symbol the
    /// engraver draws.
    ///
    /// The decided length is remembered under `span` for the same reason
    /// [`Lowering::passes`] remembers a count: [`Lowering::lasts`] asks a second
    /// time when an enclosing region measures how far its body reaches, and
    /// asking the realization again would mint a second site.
    fn held(
        &mut self,
        duration: NotatedDuration,
        most: &str,
        span: SourceSpan,
    ) -> Option<(NotatedDuration, Option<crate::score::FreeDuration>)> {
        let Some(written) = crate::resolve::parse_ratio(most) else {
            return self.refuse(
                Diagnostic::error(Code::NotAValue, format!("`{most}` is not a duration"))
                    .at(span, "expected the longest this note may be held")
                    .help("write a duration such as `2/1`"),
            );
        };
        let written = crate::MusicalDuration::new(written);
        if written.as_ratio() < duration.value.as_ratio() {
            return self.refuse(
                Diagnostic::error(Code::NotAValue, "a held note counts upwards")
                    .at(span, format!("`{}` is longer than `{most}`", duration.spelling))
                    .help("write the written value first and the longest hold second"),
            );
        }
        let least = duration.value;
        let sounds = self
            .resolver
            .decide_duration(&self.choice, least.as_ratio(), written.as_ratio(), span);
        self.holds.insert(span, sounds);
        Some((
            NotatedDuration {
                value: crate::MusicalDuration::new(sounds),
                spelling: duration.spelling,
                pieces: vec![crate::MusicalDuration::new(sounds)],
            },
            Some(crate::score::FreeDuration { least, most: written }),
        ))
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
                Some(applied(
                    origin,
                    Raw::hosted(origin, "pitch_transposed"),
                    [base, interval],
                ))
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
                Raw::app(origin, Raw::hosted(origin, "interval_inverse"), held)
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

    /// `body`, with one [`crate::origin::ExpansionStep::ScaleContext`] step on
    /// every fact it made.
    ///
    /// [`stamped`] is the operation, and it is [`Lowering::used`]'s and
    /// [`Lowering::asserted`]'s too.
    ///
    /// The step carries the collection *as the source spells it*, which is what
    /// Origin view prints and what `factext` parses back. A collection this
    /// reading could not spell — a bound `in scale mode { … }` — records the
    /// words the author wrote, because the step is a record of the source and
    /// not of the value.
    fn under_scale(node: &SyntaxNode, origin: Origin, written: &SyntaxNode, body: Raw) -> Raw {
        stamped(
            origin,
            crate::resolve::trimmed_span(node),
            crate::origin::ExpansionStep::ScaleContext {
                scale: format!(
                    "scale {}",
                    written.to_string().trim().trim_start_matches("scale").trim()
                ),
            },
            body,
        )
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
    ///
    /// Half of "the anchor decides where a mark is written"; [`Lowering::marked`]
    /// is the other half. A row whose anchor is [`crate::marks::Anchor::Note`]
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
    fn articulations(&mut self, origin: Origin, names: &[String], span: SourceSpan) -> Raw {
        let mut marks = Vec::new();
        for name in names {
            match crate::Mark::parse(name) {
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
    pub(super) fn provenance_at(&self, origin: Origin, placed: bool, declaration: DeclarationId) -> Raw {
        let span = self.sites.span(origin).unwrap_or_default();
        self.provenance(origin, span, placed, declaration)
    }

    /// The same, when the caller already holds the span.
    ///
    /// `placed` is [`Reading::placed`], and it decides the one field a *shared*
    /// body cannot know: `source_span` is where an event came from, and material
    /// usable at several places came from every one of them. So an unplaced
    /// reading writes [`crate::elaborate::SHARED_ORIGIN`] there and each use
    /// fills it in — `instanced` at a `use`, the same conditional fill `scoped`
    /// already performs for [`crate::Scope::Piece`]. `definition_span` is the
    /// span either way, because that is what *wrote* the event and a body is
    /// written once however many times it is spoken.
    ///
    /// `declaration` is [`Reading::declaration`] — the motif, fragment, named
    /// bar, `music` value, or voice this block is. Zero is a legal answer and
    /// means what `factext.rs` already reads it as, "no declaration to name":
    /// a fact built by a `play` call in a `fn` body is written in no block, and
    /// which declaration it ends up in is the caller's the same way its span is.
    #[expect(
        clippy::unused_self,
        reason = "reads as a sibling of `provenance_at`, which needs the table"
    )]
    fn provenance(&self, origin: Origin, span: SourceSpan, placed: bool, declaration: DeclarationId) -> Raw {
        let written = crate::origin::Origin {
            source_span: if placed { span } else { crate::elaborate::SHARED_ORIGIN },
            definition_span: span,
            declaration,
            // Empty by construction: expansion happened in the phase, and what
            // this module reads is the answer it left behind.
            expansion_path: Vec::new(),
        };
        Raw::lit(origin, crate::registry::origin_literal(written))
    }

    /// A statement §3 forbids inside a value usable at several places.
    ///
    /// *Misplaced* rather than *unsupported*: these are permanent answers, and
    /// the same statement in a score is perfectly legal. A diagnostic that said
    /// "not supported yet" would be a promise nobody intends to keep.
    ///
    /// The sentence names **material** rather than the `music` value it is most
    /// often written in, because a `motif` body and a `repeat` block reach here
    /// too and neither is a `music` value: a message that named one spelling
    /// would be false at two of its three call sites.
    fn misplaced<T>(&mut self, what: &str, span: SourceSpan) -> Option<T> {
        self.refuse(
            Diagnostic::error(Code::Misplaced, format!("{what} belongs to the piece, not to material"))
                .at(span, "this says \"from here onward\"")
                .help("write it in the voice or part this music is used in")
                .note("material is usable at several places, and \"from here onward\" has no unique meaning there"),
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
impl Lowering<'_> {
    pub(super) fn extent(&self, node: &SyntaxNode) -> Ratio<i64> {
        self.reached(statements(node))
    }

    /// The same sum over a chosen subsequence, which is what a repeat's body is.
    fn reached(&self, statements: impl Iterator<Item = SyntaxNode>) -> Ratio<i64> {
        let mut total = Ratio::ZERO;
        for statement in statements {
            total += self.lasts(&statement);
        }
        total
    }

    /// How long one statement lasts.
    fn lasts(&self, statement: &SyntaxNode) -> Ratio<i64> {
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
    fn played(&self, node: &SyntaxNode) -> Ratio<i64> {
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
    fn times(&self, node: &SyntaxNode) -> u32 {
        let span = crate::resolve::trimmed_span(node);
        if let Some(decided) = self.counts.get(&span) {
            return *decided;
        }
        musa_language::ast::RepeatStmt::cast(node.clone())
            .and_then(|statement| statement.count())
            .and_then(|text| count_of(&text))
            .unwrap_or_default()
    }
}

/// How long `times` passes last, given the body's extent and each ending's.
///
/// Pass `i` plays the body and then the ending at [`last_at_most`], so the
/// endings are summed over the passes rather than over themselves: two endings
/// under three passes contribute three ending-lengths, not two.
#[expect(
    clippy::arithmetic_side_effects,
    reason = "`extent`'s argument, in the function that does its multiplication"
)]
fn spanned(body: Ratio<i64>, endings: &[Ratio<i64>], times: u32) -> Ratio<i64> {
    let mut total = body * Ratio::from_integer(i64::from(times));
    for iteration in 0..times {
        let Some(index) = last_at_most(iteration, endings.len()) else {
            break;
        };
        total += endings.get(index).copied().unwrap_or_default();
    }
    total
}

/// Which ending pass `iteration` takes, and [`None`] when there are none.
///
/// The last one covers every pass after it, which is what `1.–3.` means on a
/// volta bracket.
fn last_at_most(iteration: u32, endings: usize) -> Option<usize> {
    Some(
        usize::try_from(iteration)
            .unwrap_or(usize::MAX)
            .min(endings.checked_sub(1)?),
    )
}

/// The endings a repeat writes, in written order.
fn endings_of(node: &SyntaxNode) -> Vec<musa_language::ast::EndingStmt> {
    statements(node)
        .filter_map(musa_language::ast::EndingStmt::cast)
        .collect()
}

/// Whether a statement is anything but an ending — a repeat's body.
fn is_not_an_ending(statement: &SyntaxNode) -> bool {
    statement.kind() != SyntaxKind::EndingStmt
}

/// `tracks`, one after another, seeded with `nothing`.
///
/// [`Lowering::folded`]'s shape, without the statements: a repeat's passes are
/// already tracks, and the seed is what makes a repeat of no passes silence
/// rather than a special case.
fn followed(origin: Origin, tracks: Vec<Raw>) -> Raw {
    let mut placed = Placed::default();
    for track in tracks {
        placed.place(origin, track);
    }
    placed.built(origin)
}

/// Tracks placed one after another, kept as a stack of balanced subtrees.
///
/// The obvious accumulator is a left spine — `follow(follow(follow(nothing, a),
/// b), c)` — and it is as deep as the block is long. The evaluator descends that
/// spine, spending several frames per `follow`, so
/// [`crate::core_budget::NESTING`]'s 256 levels are reached at some sixty
/// statements: `examples/in-c.musa`'s fifty-three-figure voice is refused for
/// nesting, and a voice of a hundred notes would be. It also costs quadratic
/// work, since each `follow` translates everything accumulated so far.
///
/// `follow` is associative — `musa_kernel::follow` places each track after the
/// one before it, and where the brackets fall does not move a single occurrence
/// — so the same music can be written as a *balanced* tree, whose depth is the
/// logarithm of the count. That is what this builds.
///
/// # The stack
///
/// Completed subtrees, in written order, with strictly decreasing sizes that are
/// powers of two. Placing one more merges equal-sized neighbours exactly as
/// incrementing a binary counter carries, so nothing is ever rebuilt and every
/// subtree is shared. What the block denotes is [`Self::built`], the stack folded
/// down — at most log₂ n terms — and what stands *before* statement *i* is the
/// same fold of the stack as it stood then, sharing all of it rather than
/// copying a prefix.
#[derive(Default)]
struct Placed {
    /// `(count, track)` for each completed subtree, sizes strictly decreasing.
    stack: Vec<(usize, Raw)>,
}

impl Placed {
    /// One more track, after everything placed so far.
    fn place(&mut self, origin: Origin, track: Raw) {
        let mut count = 1;
        let mut built = track;
        while self.stack.last().is_some_and(|&(top, _)| top == count) {
            let (_, earlier) = self.stack.pop().unwrap_or_else(|| unreachable!("just looked at it"));
            built = applied(origin, Raw::hosted(origin, "follow"), [earlier, built]);
            count *= 2;
        }
        self.stack.push((count, built));
    }

    /// Everything placed so far, as one track.
    ///
    /// Seeded with `nothing`, which is what makes an empty block silence rather
    /// than a special case, and the reason a single statement still reads as
    /// `follow(nothing, t)` — the shape every law written against one statement
    /// already expects.
    fn built(&self, origin: Origin) -> Raw {
        self.stack
            .iter()
            .fold(Raw::lit(origin, crate::registry::empty_track()), |built, (_, next)| {
                applied(origin, Raw::hosted(origin, "follow"), [built, next.clone()])
            })
    }
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

/// `track`, or `tied(track)` when a `~` was written on the statement.
///
/// Every notation statement could carry one and only two can: the grammar puts
/// `~` on a note and on a chord, and a tie between anything else is not a tie.
/// So the mark is applied where those two are read rather than in
/// [`Lowering::statement`], which would have to ask the other fifteen a question
/// they have no way to answer.
///
/// The joining is [`super::piece`]'s, once per voice — see the `joined` rule for
/// why it cannot be done any nearer to here.
fn continuing(origin: Origin, tied: bool, track: Raw) -> Raw {
    if tied {
        Raw::app(origin, Raw::hosted(origin, "tied"), track)
    } else {
        track
    }
}

/// `Scope.Piece`, `Scope.Part n`, `Scope.Voice p v`.
pub(super) fn scope_of(origin: Origin, scope: crate::Scope) -> Raw {
    match scope {
        crate::Scope::Piece => Raw::hosted(origin, "Scope.Piece"),
        crate::Scope::Part { part } => Raw::app(
            origin,
            Raw::hosted(origin, "Scope.Part"),
            whole(origin, u64::from(part)),
        ),
        crate::Scope::Voice { part, voice } => applied(
            origin,
            Raw::hosted(origin, "Scope.Voice"),
            [whole(origin, u64::from(part)), whole(origin, u64::from(voice))],
        ),
    }
}

/// `body`, with `step` recorded at the front of every fact it makes.
///
/// The four enclosures that leave a mark and no music — `in scale`, a `use` of
/// reusable material, one pass of a `repeat`, and an `assert` — say so through
/// this one call, because "these facts were made inside this expansion" is one
/// thing to say.
///
/// It is the `instanced` builtin rather than a stamp applied while reading,
/// because the facts do not exist until the term is evaluated: a `use` inside
/// the braces answers notes some earlier declaration built, and a walk over the
/// written tree would reach the notes spelled here and miss those.
/// [`crate::registry::track`]'s `INSTANCED` makes the argument in full, and is
/// also where the step lands in *front* of whatever the body already recorded,
/// which is what makes a path read outside-in.
fn stamped(origin: Origin, at: SourceSpan, step: crate::origin::ExpansionStep, body: Raw) -> Raw {
    applied(
        origin,
        Raw::hosted(origin, "instanced"),
        [
            Raw::lit(
                origin,
                crate::registry::origin_literal(crate::lower::expansion(at, step)),
            ),
            body,
        ],
    )
}

/// A literal at a plain base type whose payload has a written spelling.
///
/// # Which of the two writers a domain takes
///
/// Whether a domain implements [`std::fmt::Display`] is not a formatting
/// preference here; it selects the *representation*, and the reader in
/// [`crate::registry`] selects the same way. A domain that has a spelling is
/// written by this function and read back by `rules::read`; one that has none is
/// wrapped in `notation::Opaque` by [`payload`] and read back by
/// `notation::unwrapped`. Neither downcast can see the other's wrapper, so a
/// domain written by one and read by the other is a rule that computes nothing
/// at arguments it declares it accepts — a defect the core reports against the
/// builtin rather than against the source. The pairing belongs to the base type,
/// not to the call site: pick by asking whether `T` has a `Display`.
fn plain<T>(origin: Origin, base: &'static str, value: T) -> Raw
where
    T: PartialEq + std::fmt::Debug + std::fmt::Display + Send + Sync + 'static,
{
    Raw::lit(
        origin,
        crate::registry::literal(crate::registry::plain_type(base), value),
    )
}

/// A literal at a plain base type whose payload has none — see [`plain`] for
/// which domains take which of the two.
fn payload<T>(origin: Origin, base: &'static str, value: T) -> Raw
where
    T: Clone + PartialEq + std::fmt::Debug + Send + Sync + 'static,
{
    Raw::lit(origin, crate::registry::opaque_literal(base, value))
}

/// `Option.None` or `Option.Some v`, from an unspelled payload that may not be
/// there.
///
/// Only the [`payload`] half: the four optional fields any fact carries —
/// `FreeDuration`, `Metronome`, `Ramp` — are all domains without a spelling, and
/// a spelled one writes `maybe(origin, value.map(…))` where it stands rather
/// than through a second helper that could pick the wrong wrapper.
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
    Raw::app(origin, Raw::hosted(origin, "Fact.Key"), plain(origin, "Key", key))
}

/// `Fact.Meter(numerator, denominator)`.
pub(super) fn metered(origin: Origin, meter: crate::score::Meter) -> Raw {
    applied(
        origin,
        Raw::hosted(origin, "Fact.Meter"),
        [
            whole(origin, u64::from(meter.numerator())),
            whole(origin, u64::from(meter.denominator())),
        ],
    )
}

/// `Fact.Clef(clef)`.
pub(super) fn clefed(origin: Origin, clef: crate::score::Clef) -> Raw {
    Raw::app(origin, Raw::hosted(origin, "Fact.Clef"), payload(origin, "Clef", clef))
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
        Raw::hosted(origin, "Fact.Tempo"),
        [
            optional(origin, "Metronome", marking.metronome),
            maybe(origin, marking.text.clone().map(|text| plain(origin, "Text", text))),
            optional(origin, "Ramp", marking.ramp.clone()),
        ],
    )
}

/// The argument `mark`'s row takes, spelled as an author would write it.
///
/// A placeholder rather than a value, because this is what goes in a help: the
/// reader is being shown the shape of the statement they meant to write, and a
/// made-up rehearsal letter would be a suggestion to write that letter.
fn written_argument(mark: crate::Mark) -> &'static str {
    match mark.takes() {
        crate::marks::Argument::None => "",
        crate::marks::Argument::Text => " \"…\"",
        crate::marks::Argument::Number => " 1",
    }
}

/// How `mark`'s row ends: a block for a span, a semicolon for anything else.
fn written_tail(mark: crate::Mark) -> &'static str {
    match mark.anchor() {
        crate::marks::Anchor::Span => " { … }",
        crate::marks::Anchor::Point | crate::marks::Anchor::Note(_) => ";",
    }
}

/// The same, from a term that may not be there.
fn maybe(origin: Origin, value: Option<Raw>) -> Raw {
    match value {
        None => Raw::hosted(origin, "Option.None"),
        Some(held) => Raw::app(origin, Raw::hosted(origin, "Option.Some"), held),
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
///
/// Both counts are positive, which is not pedantry about the grammar: the ratio
/// is inverted to scale the durations written inside, and `tuplet 0/2` would
/// name a division into no notes. Refused here so that the refusal is one
/// sentence at the statement rather than a division by zero somewhere below.
fn tuplet_ratio(text: &str) -> Option<(u32, u32)> {
    let (num, den) = text.split_once('/')?;
    let (num, den) = (count_of(num)?, count_of(den)?);
    (num > 0 && den > 0).then_some((num, den))
}

/// How much a tuplet scales the durations written inside it.
///
/// The inverse of what it says: `tuplet 3/2` is three notes in the time of two,
/// so an eighth written inside it lasts two thirds of an eighth. A ratio this
/// cannot read is [`Ratio::ONE`], because the same statement is refused with its
/// own sentence by [`Lowering::statement`] and a length nothing will ask for is
/// better left unscaled than guessed at.
fn tuplet_factor(node: &SyntaxNode) -> Ratio<i64> {
    musa_language::ast::TupletStmt::cast(node.clone())
        .and_then(|statement| statement.ratio())
        .and_then(|text| tuplet_ratio(&text))
        .map_or(Ratio::ONE, |(num, den)| Ratio::new(i64::from(den), i64::from(num)))
}

/// A whole number written as a count.
fn count_of(text: &str) -> Option<u32> {
    text.trim().parse().ok()
}
