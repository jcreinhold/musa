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
//! composition equation, in the event track's own word).
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
//!   [`musa_score::scale::Frame`] — the same arithmetic the old checker ran — and
//!   what lands in the raw term is a `Pitch` literal. `01-surface.md` §2 says
//!   `in scale` "is lexical rather than captured" and that "an absent scale
//!   makes `step` a type-context diagnostic, not an implicit C-major choice";
//!   both are this function's shape. `in scale` emits no fact, so it is not a
//!   key signature and not a claim of modulation.
//! - **Scope is given, not discovered.** A `music { … }` value is "usable at
//!   several places" (§3), so it has no voice of its own and reads at
//!   [`musa_score::Scope::Piece`]. [`super::piece`] lowers a *voice's* body by passing
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

mod asserts;
mod extent;
mod facts;
mod helpers;
#[cfg(test)]
mod laws;
mod marks;
mod materials;
mod raw;
pub(super) use raw::{
    Placed, clefed, continuing, count_of, endings_of, followed, is_not_an_ending, keyed, last_at_most, maybe, metered,
    named, optional, payload, plain, scope_of, spanned, stamped, tempo, tuplet_factor, tuplet_ratio, written_argument,
    written_duration, written_tail,
};

mod repeats;
mod statement;

use musa_calculus::Raw;
use musa_syntax::ast::AstNode as _;
use musa_syntax::{SyntaxKind, SyntaxNode};
use num_rational::Ratio;

use super::significant_tokens;
use musa_score::origin::{DeclarationId, SourceSpan};
use musa_score::score::NotatedDuration;

/// What a block reads and never writes.
///
/// Passed by value into every nesting, which is what makes `in scale`'s scope
/// lexical without a stack to push and pop. Small enough to copy: a scope is two
/// words and a scale is a tonic class and a collection.
#[derive(Clone, Copy)]
pub(crate) struct Reading {
    /// The scope every fact built here is constructed at (§5.7 requires one).
    scope: musa_score::Scope,
    /// The collection `step` counts in, when an `in scale` lexically encloses.
    scale: Option<Counting>,
    /// Whether this block stands at one place in the piece.
    ///
    /// Decides the `source_span` every event built here carries: material
    /// usable at several places came from every one of them, so an unplaced
    /// reading writes [`crate::elaborate::SHARED_ORIGIN`] and each use fills it
    /// in. Not derivable from [`Reading::scope`]: a free `music { … }` value
    /// reads at [`musa_score::Scope::Piece`] and so does a piece's own header, and
    /// the difference between them is not what the fact is *about* but whether
    /// the text is spoken in one place.
    placed: bool,
    /// Whether an enclosing `repeat` speaks this block more than once.
    ///
    /// A second bit rather than a wider meaning for [`Reading::placed`],
    /// because the two questions come apart at exactly one construct and answer
    /// different callers. A repeat body *is* written at one place, so its
    /// events keep their own span and each pass is told apart by
    /// [`musa_score::origin::ExpansionStep::RepeatIteration`] — but it is played `n`
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
    meter: musa_score::score::Meter,
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
pub(crate) struct Reach {
    /// Whether the name is a motif, a bar, or a fragment.
    material: crate::resolve::Material,
    /// How far the material reaches, as written.
    reaches: Ratio<i64>,
}

/// The named material a document declares, by name.
pub(super) type Declarations = std::collections::HashMap<String, Reach>;

impl Reading {
    /// The reading a free-standing `music { … }` value is read under.
    ///
    /// [`musa_score::Scope::Piece`] because a fragment is "usable at several places"
    /// and so has no voice of its own, and no scale because `01-surface.md` §2
    /// refuses an implicit C major.
    pub(super) fn free(declaration: DeclarationId) -> Self {
        Self {
            scope: musa_score::Scope::Piece,
            scale: None,
            placed: false,
            repeated: false,
            declaration,
            meter: musa_score::score::Meter::default(),
            tuplet: Ratio::ONE,
        }
    }

    /// The reading a body written at one place in the piece is read under.
    pub(super) fn at(scope: musa_score::Scope, declaration: DeclarationId) -> Self {
        Self {
            scope,
            scale: None,
            placed: true,
            repeated: false,
            declaration,
            meter: musa_score::score::Meter::default(),
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
    /// [`musa_score::score::NotatedDuration::scaled`] and not `stretched`: a tuplet
    /// keeps the symbol the engraver draws and moves only what it sounds for, so
    /// a triplet eighth stays spelled `1/8` and carries the value `1/12`. The
    /// freedom moves with it because it is measured in the same time — `c5/4 to
    /// 2/1` inside a triplet may be held to two thirds of a double whole, not to
    /// a double whole.
    fn lasting(
        self,
        written: (NotatedDuration, Option<musa_score::score::FreeDuration>),
    ) -> (NotatedDuration, Option<musa_score::score::FreeDuration>) {
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
    /// says. [`musa_score::scale::signature_scale`] is the same reading `key_scale`
    /// gives a program that asks for it as a value, so the default a step takes
    /// and the default a composer can name are one collection.
    ///
    /// A default and not a fact: an `in scale` inside overrides it by the
    /// ordinary nesting, because [`Self::stepping`] is applied to the reading
    /// this produced.
    pub(super) fn keyed(self, key: musa_score::score::Key) -> Self {
        self.stepping(Counting::Written(musa_score::scale::signature_scale(key)))
    }

    /// The same reading, under `meter`.
    ///
    /// Called once by [`super::piece`] with the meter the piece's header states,
    /// and again by [`Lowering::notated`] at each `meter` a block writes — the
    /// two places a meter can come into force, and the only two.
    pub(super) const fn metered(self, meter: musa_score::score::Meter) -> Self {
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
///
/// It is prepended in *pieces* rather than as one term, and that is a
/// measurement rather than a preference. A fold hands every claim raised under
/// it the same prefix, and building that prefix into one term per claim means
/// elaborating it once per claim: a hundred-bar voice elaborated the music
/// before bar one a hundred times, before bar two ninety-nine times, and
/// compiling `tests/fixtures/large-score.musa` took 13.33 seconds in a release
/// build against `06-frame-budgets.md`'s 400 ms. The pieces are
/// [`Placed`](super::raw::Placed)'s own subtrees, shared by `Arc` across every
/// claim the fold raises, so [`crate::document::Document::began`] can read each
/// one's duration once and add them up. Sound because the event track's
/// `sequence` adds durations: measuring a prefix in pieces and measuring it
/// whole are one number by the ontology, which
/// [`a_prefix_measured_in_pieces_is_the_prefix_measured_whole`](crate::lower::piece::laws)
/// checks rather than assumes.
pub(crate) struct Claimed {
    /// Which claim is made. The registry's own row, so the name and the shapes
    /// its arguments must have are one fact rather than two that could disagree.
    pub(crate) predicate: &'static musa_score::assert::Predicate,
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
    /// The music standing before the passage, earliest piece first. Its total
    /// duration is where the passage begins, which is what a measure claim is
    /// measured against; the pieces are the module doc's subject and an empty
    /// list is a passage that begins at zero.
    pub(crate) before: Vec<Raw>,
    /// The passage itself. Its duration is how long the passage lasts, and its
    /// occurrences are the notes a pitch or chord claim is proved against.
    pub(crate) passage: Raw,
}

/// One argument of a written claim, in the state the reading leaves it in.
///
/// Two cases because [`musa_score::assert::ParamType`] has two kinds in it. Four of
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
    Word(musa_score::assert::Argument),
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
pub(crate) enum Counting {
    /// A scale the source spelled, whose degrees this reading can count
    /// through at the note that asks.
    Written(musa_score::scale::Scale),
    /// A scale a binder supplies.
    Bound,
}

/// One of the four statements whose meaning is "from here onward".
///
/// A closed set rather than a `SyntaxKind`, because it is the set
/// `00-semantics.md` §3 names — "a block may not contain a key, meter, tempo, or
/// clef change" — and both readers of it want the same four and no others.
#[derive(Clone, Copy)]
pub(crate) enum Context {
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
/// The kinds [`musa_syntax::ast::VoiceItem`] admits, which is the grammar's
/// own answer to "what may stand in a block". Written out rather than derived
/// from the typed enum because this module reads nodes, and a kind the grammar
/// grows without a reading here should fail [`Lowering::statement`]'s table
/// rather than be silently skipped by the fold.
pub(super) fn is_statement(kind: SyntaxKind) -> bool {
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
pub(super) fn word(node: &SyntaxNode) -> String {
    significant_tokens(node)
        .map(|token| crate::lower::lexeme_text(&token))
        .collect()
}

/// A written claim, spelled back: `pitches_in(scale c major)`, `fills_meter()`.
///
/// What Origin view prints for an `assert` and what `crate::factext` parses
/// back, so it is written from the source rather than from
/// [`musa_score::assert::Claim`]: the arguments are terms here, and a term has no
/// value until the document it stands in is elaborated. The name comes from the
/// registry row instead of the token, because the row was found by matching that
/// token exactly and a `&'static str` cannot be a spelling nothing claims.
///
/// Runs of whitespace close up so that a claim written across two lines reads as
/// one, which is the only difference this allows itself from the bytes.
pub(super) fn spelled_claim(
    predicate: &musa_score::assert::Predicate,
    statement: &musa_syntax::ast::AssertStmt,
) -> String {
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
pub(super) fn claim_span(statement: &musa_syntax::ast::AssertStmt, node: &SyntaxNode) -> SourceSpan {
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
pub(super) fn statements(node: &SyntaxNode) -> impl Iterator<Item = SyntaxNode> {
    let holder = node
        .children()
        .find(|child| child.kind() == SyntaxKind::Block)
        .unwrap_or_else(|| node.clone());
    holder.children().filter(|child| is_statement(child.kind()))
}
