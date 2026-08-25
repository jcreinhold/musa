//! Raw notation payload builders; see `notation` module docs.

use musa_calculus::{Origin, Raw};
use musa_syntax::ast::AstNode as _;
use musa_syntax::{SyntaxKind, SyntaxNode};
use num_rational::Ratio;

use crate::lower::{applied, whole};
use musa_score::origin::SourceSpan;

use super::*;

/// How long `times` passes last, given the body's extent and each ending's.
///
/// Pass `i` plays the body and then the ending at [`last_at_most`], so the
/// endings are summed over the passes rather than over themselves: two endings
/// under three passes contribute three ending-lengths, not two.
#[expect(
    clippy::arithmetic_side_effects,
    reason = "`extent`'s argument, in the function that does its multiplication"
)]
pub(crate) fn spanned(body: Ratio<i64>, endings: &[Ratio<i64>], times: u32) -> Ratio<i64> {
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
pub(crate) fn last_at_most(iteration: u32, endings: usize) -> Option<usize> {
    Some(
        usize::try_from(iteration)
            .unwrap_or(usize::MAX)
            .min(endings.checked_sub(1)?),
    )
}

/// The endings a repeat writes, in written order.
pub(crate) fn endings_of(node: &SyntaxNode) -> Vec<musa_syntax::ast::EndingStmt> {
    statements(node)
        .filter_map(musa_syntax::ast::EndingStmt::cast)
        .collect()
}

/// Whether a statement is anything but an ending — a repeat's body.
pub(crate) fn is_not_an_ending(statement: &SyntaxNode) -> bool {
    statement.kind() != SyntaxKind::EndingStmt
}

/// `tracks`, one after another, seeded with `nothing`.
///
/// [`Lowering::folded`]'s shape, without the statements: a repeat's passes are
/// already tracks, and the seed is what makes a repeat of no passes silence
/// rather than a special case.
pub(crate) fn followed(origin: Origin, tracks: Vec<Raw>) -> Raw {
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
/// The former evaluator's 256 nesting levels were reached at some sixty
/// statements: `examples/in-c.musa`'s fifty-three-figure voice is refused for
/// nesting, and a voice of a hundred notes would be. It also costs quadratic
/// work, since each `follow` translates everything accumulated so far.
///
/// `follow` is associative — `musa_events::follow` places each track after the
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
pub(crate) struct Placed {
    /// `(count, track)` for each completed subtree, sizes strictly decreasing.
    stack: Vec<(usize, DurationPiece)>,
}

/// One balanced prefix piece together with the concatenations that built it.
///
/// The children are retained only for duration reads. They do not add a second
/// meaning to the term: [`musa_events::sequence`] defines the parent's duration
/// as the sum of its children, which is the same equation [`Placed`] used to
/// build the parent with `follow`.
#[derive(Clone)]
pub(crate) struct DurationPiece(std::sync::Arc<DurationPieceNode>);

struct DurationPieceNode {
    raw: Raw,
    children: Option<[DurationPiece; 2]>,
}

impl DurationPiece {
    fn leaf(raw: Raw) -> Self {
        Self(std::sync::Arc::new(DurationPieceNode { raw, children: None }))
    }

    fn joined(raw: Raw, earlier: Self, later: Self) -> Self {
        Self(std::sync::Arc::new(DurationPieceNode {
            raw,
            children: Some([earlier, later]),
        }))
    }

    /// The track this piece denotes.
    pub(crate) fn raw(&self) -> &Raw {
        &self.0.raw
    }

    /// The two pieces whose sequence built this one, when it is not a leaf.
    pub(crate) fn children(&self) -> Option<&[Self; 2]> {
        self.0.children.as_ref()
    }

    /// This piece and its duration decomposition under one transformation.
    pub(crate) fn transformed(&self, under: &impl Fn(Raw) -> Raw) -> Self {
        let raw = under(self.raw().clone());
        match self.children() {
            Some([earlier, later]) => Self::joined(raw, earlier.transformed(under), later.transformed(under)),
            None => Self::leaf(raw),
        }
    }
}

impl Placed {
    /// One more track, after everything placed so far.
    pub(crate) fn place(&mut self, origin: Origin, track: Raw) {
        let mut count = 1;
        let mut built = DurationPiece::leaf(track);
        while self.stack.last().is_some_and(|(top, _)| *top == count) {
            let Some((_, earlier)) = self.stack.pop() else {
                break;
            };
            let raw = applied(
                origin,
                Raw::hosted(origin, "follow"),
                [earlier.raw().clone(), built.raw().clone()],
            );
            built = DurationPiece::joined(raw, earlier, built);
            count = count.saturating_mul(2);
        }
        self.stack.push((count, built));
    }

    /// Everything placed so far, in the pieces this counter holds it in.
    ///
    /// Earliest first, so that following them in order is following the music
    /// in order. There are at most `log₂ n` of them and each one is the same
    /// `Raw` every later caller sees, because `place` only ever merges whole
    /// subtrees and never rebuilds one — which is what lets a reader that wants
    /// a *measurement* of the prefix, rather than the prefix itself, pay for
    /// each piece once however many claims stand after it. See
    /// [`crate::document::Document::began`].
    pub(crate) fn pieces(&self) -> impl Iterator<Item = &DurationPiece> {
        self.stack.iter().map(|(_, piece)| piece)
    }

    /// Everything placed so far, as one track.
    ///
    /// Seeded with `nothing`, which is what makes an empty block silence rather
    /// than a special case, and the reason a single statement still reads as
    /// `follow(nothing, t)` — the shape every law written against one statement
    /// already expects.
    pub(crate) fn built(&self, origin: Origin) -> Raw {
        self.stack
            .iter()
            .fold(Raw::lit(origin, crate::registry::empty_track()), |built, (_, next)| {
                applied(origin, Raw::hosted(origin, "follow"), [built, next.raw().clone()])
            })
    }
}

/// A duration in written time, as the literal the track builtins take.
///
/// Not [`plain`], because `Duration` is registered at `Coordinate → Type 0` and
/// the literal a signature accepts is at `Duration ⟨written⟩`: a bare `Duration`
/// is a different type, and the core says so.
pub(crate) fn written_duration(origin: Origin, held: Ratio<i64>) -> Raw {
    Raw::lit(
        origin,
        crate::registry::literal(
            crate::registry::tagged_type("Duration", crate::phase::Coordinate::WrittenTime),
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
pub(crate) fn continuing(origin: Origin, tied: bool, track: Raw) -> Raw {
    if tied {
        Raw::app(origin, Raw::hosted(origin, "tied"), track)
    } else {
        track
    }
}

/// `Scope.Piece`, `Scope.Part n`, `Scope.Voice p v`.
pub(crate) fn scope_of(origin: Origin, scope: musa_score::Scope) -> Raw {
    match scope {
        musa_score::Scope::Piece => Raw::hosted(origin, "Scope.Piece"),
        musa_score::Scope::Part { part } => Raw::app(
            origin,
            Raw::hosted(origin, "Scope.Part"),
            whole(origin, u64::from(part)),
        ),
        musa_score::Scope::Voice { part, voice } => applied(
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
pub(crate) fn stamped(origin: Origin, at: SourceSpan, step: musa_score::origin::ExpansionStep, body: Raw) -> Raw {
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
pub(crate) fn plain<T>(origin: Origin, base: &'static str, value: T) -> Raw
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
pub(crate) fn payload<T>(origin: Origin, base: &'static str, value: T) -> Raw
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
pub(crate) fn optional<T>(origin: Origin, base: &'static str, value: Option<T>) -> Raw
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
pub(crate) fn keyed(origin: Origin, key: musa_score::score::Key) -> Raw {
    Raw::app(origin, Raw::hosted(origin, "Fact.Key"), plain(origin, "Key", key))
}

/// `Fact.Meter(numerator, denominator)`.
pub(crate) fn metered(origin: Origin, meter: musa_score::score::Meter) -> Raw {
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
pub(crate) fn clefed(origin: Origin, clef: musa_score::score::Clef) -> Raw {
    Raw::app(origin, Raw::hosted(origin, "Fact.Clef"), payload(origin, "Clef", clef))
}

/// `Fact.Tempo(metronome, text, ramp)`, from what the statement said.
///
/// The three arguments are the three answers [`crate::resolve::Marking`] holds,
/// in the order `registry::notation` reads them back. `text` is written with
/// [`plain`] rather than [`payload`] because `Text`'s payload is a `String` and
/// the reader asks for one; the other two are opaque.
pub(crate) fn tempo(origin: Origin, marking: &crate::resolve::Marking) -> Raw {
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
pub(crate) fn written_argument(mark: musa_score::Mark) -> &'static str {
    match mark.takes() {
        musa_score::marks::Argument::None => "",
        musa_score::marks::Argument::Text => " \"…\"",
        musa_score::marks::Argument::Number => " 1",
    }
}

/// How `mark`'s row ends: a block for a span, a semicolon for anything else.
pub(crate) fn written_tail(mark: musa_score::Mark) -> &'static str {
    match mark.anchor() {
        musa_score::marks::Anchor::Span => " { … }",
        musa_score::marks::Anchor::Point | musa_score::marks::Anchor::Note(_) => ";",
    }
}

/// The same, from a term that may not be there.
pub(crate) fn maybe(origin: Origin, value: Option<Raw>) -> Raw {
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
pub(crate) fn named(node: &SyntaxNode) -> bool {
    matches!(node.kind(), SyntaxKind::NameExpr | SyntaxKind::PathExpr)
}

/// `3/2`, as a tuplet's two counts, unreduced as the backends need them.
///
/// Both counts are positive, which is not pedantry about the grammar: the ratio
/// is inverted to scale the durations written inside, and `tuplet 0/2` would
/// name a division into no notes. Refused here so that the refusal is one
/// sentence at the statement rather than a division by zero somewhere below.
pub(crate) fn tuplet_ratio(text: &str) -> Option<(u32, u32)> {
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
pub(crate) fn tuplet_factor(node: &SyntaxNode) -> Ratio<i64> {
    musa_syntax::ast::TupletStmt::cast(node.clone())
        .and_then(|statement| statement.ratio())
        .and_then(|text| tuplet_ratio(&text))
        .map_or(Ratio::ONE, |(num, den)| Ratio::new(i64::from(den), i64::from(num)))
}

/// A whole number written as a count.
pub(crate) fn count_of(text: &str) -> Option<u32> {
    text.trim().parse().ok()
}
