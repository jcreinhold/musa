//! `EventTrack<C,A>`: the finite event-track denotation `(d, E)` and its algebra
//! (docs/rules/events/03). Stored flat — construction IS normalization
//! (docs/rules/events/05 N1).
//!
//! Rational arithmetic on musa's magnitudes is total; the workspace
//! arithmetic lint is allowed module-wide (see musa-compiler/src/time.rs).
#![allow(clippy::arithmetic_side_effects)]

use num_rational::Ratio;

use crate::occurrence::{Canonical, Occurrence};
use crate::time::{Coordinate, Duration, Position, Span};

/// A finite event track in coordinate `C`.
///
/// An ambient duration `d ∈ ℚ≥0` plus a multiset of occurrences supported
/// within `[0, d]`. The duration is real — `(d, ∅)` is silence by absence,
/// not an error (§2, §9).
///
/// The coordinate is part of the type and not part of the value (D0): two
/// tracks in different coordinates are different types and never combine, and
/// no operation here changes coordinate.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EventTrack<C: Coordinate, A> {
    duration: Duration<C>,
    occurrences: Vec<Occurrence<C, A>>,
}

/// Construct a track, checking `0 ≤ s ≤ e ≤ d` for every occurrence (K1).
///
/// Beyond D1's two named literals because it has the callers the literals do
/// not: an elaborator and the interchange reader both arrive with a duration
/// and a finished occurrence list, and rebuilding that from `event` and
/// `together` would allocate a track per occurrence to denote the same value.
///
/// # Errors
/// [`EventsError::OccurrenceOutOfBounds`](crate::EventsError::OccurrenceOutOfBounds)
/// for the first occurrence outside the duration.
pub fn track<C: Coordinate, A>(
    duration: Duration<C>,
    occurrences: Vec<Occurrence<C, A>>,
) -> Result<EventTrack<C, A>, crate::EventsError> {
    let reach = duration.reach();
    for occurrence in &occurrences {
        let span = occurrence.span();
        if span.end() > reach {
            return Err(crate::EventsError::OccurrenceOutOfBounds {
                span: span.to_string(),
                duration: duration.to_string(),
            });
        }
    }
    Ok(EventTrack { duration, occurrences })
}

/// The empty track `(d, ∅)` (D1). Infallible: no occurrence can violate the
/// bounds of a track that has none.
///
/// `empty(Duration::ZERO)` is the two-sided identity of [`follow`] (L2).
pub fn empty<C: Coordinate, A>(duration: Duration<C>) -> EventTrack<C, A> {
    EventTrack {
        duration,
        occurrences: Vec::new(),
    }
}

/// The track `(d, {(0, d, a)})`: one occurrence filling its own track (D1).
///
/// # Errors
/// Infallible in practice — the single occurrence spans exactly `[0, d]` — but
/// it builds a [`Span`], so the error is the span constructor's and is stated
/// rather than unwrapped.
pub fn event<C: Coordinate, A>(duration: Duration<C>, payload: A) -> Result<EventTrack<C, A>, crate::EventsError> {
    let span = Span::new(Position::ZERO, duration.reach())?;
    Ok(EventTrack {
        duration,
        occurrences: vec![Occurrence::new(span, payload)],
    })
}

/// Temporal succession: `follow(M, N) = (d + e, E ⊎ τ_d(F))` (D2). Laws L1–L3.
pub fn follow<C: Coordinate, A>(parts: Vec<EventTrack<C, A>>) -> EventTrack<C, A> {
    let mut offset = Duration::ZERO;
    let mut occurrences = Vec::new();
    for part in parts {
        let part_duration = part.duration;
        occurrences.extend(
            part.occurrences
                .into_iter()
                .map(|occurrence| occurrence.translate(offset)),
        );
        offset = offset.plus(part_duration);
    }
    EventTrack {
        duration: offset,
        occurrences,
    }
}

/// Simultaneous presence: `together(M, N) = (max(d, e), E ⊎ F)` (D3). Nothing
/// pads the shorter track. Laws L4–L6; multiplicity is preserved (X1).
pub fn together<C: Coordinate, A>(parts: Vec<EventTrack<C, A>>) -> EventTrack<C, A> {
    let mut duration = Duration::ZERO;
    let mut occurrences = Vec::new();
    for part in parts {
        duration = duration.max(part.duration);
        occurrences.extend(part.occurrences);
    }
    EventTrack { duration, occurrences }
}

/// A track observed through a window (D6, §17).
///
/// Borrowed and cheap: an observation selects occurrences, it does not copy
/// them, and it stores nothing that can be computed. In particular the
/// *visible* span — `whole ∩ window` — is derived on the way out rather than
/// kept beside the whole span, so the two can never disagree.
///
/// Observations compose: narrowing one intersects the windows, which makes
/// L17 hold by construction rather than by a precondition a caller has to
/// respect.
#[derive(Clone, Debug)]
pub struct Observation<'a, C: Coordinate, A> {
    window: Span<C>,
    /// The duration of the track being observed. The final instant of a track
    /// is observable (D6), and that is a fact about the track rather than
    /// about the window: without it, narrowing a full-duration observation
    /// would drop a point occurrence the wider one reported.
    duration: Duration<C>,
    selected: Vec<&'a Occurrence<C, A>>,
}

impl<'a, C: Coordinate, A> Observation<'a, C, A> {
    /// The observation window.
    pub fn window(&self) -> Span<C> {
        self.window
    }

    /// Narrow to `window ∩ self.window()`.
    ///
    /// Total by intersection: there is no containment precondition, so L17
    /// holds for every pair of windows rather than only for nested ones. Two
    /// windows that do not meet observe nothing, which is not the same as a
    /// degenerate window at the duration — hence the explicit empty case
    /// rather than a fabricated span the point-at-the-end rule would then read.
    #[must_use]
    pub fn restrict(&self, window: Span<C>) -> Self {
        let start = self.window.start().max(window.start());
        let end = self.window.end().min(window.end());
        if start > end {
            return Self {
                window: Span::ZERO.translate(Duration::new_raw(start.as_ratio())),
                duration: self.duration,
                selected: Vec::new(),
            };
        }
        let window = self.window.clip(window);
        Self {
            window,
            duration: self.duration,
            selected: self
                .selected
                .iter()
                .copied()
                .filter(|occurrence| visible(occurrence.span(), window, self.duration))
                .collect(),
        }
    }

    /// Each observed occurrence with its visible span `whole ∩ window`.
    ///
    /// The occurrence keeps its whole span (§17): observation never moves an
    /// occurrence's claim about where it began.
    pub fn observed(&self) -> impl Iterator<Item = (Span<C>, &'a Occurrence<C, A>)> + '_ {
        let window = self.window;
        self.selected
            .iter()
            .copied()
            .map(move |occurrence| (occurrence.span().clip(window), occurrence))
    }

    /// Whether the window shows nothing.
    pub fn is_empty(&self) -> bool {
        self.selected.is_empty()
    }
}

/// Whether `span` is visible through `window` in a track of `duration`.
///
/// The final instant of a track is observable: a point occurrence at the
/// duration is shown by a window that ends there, which is what makes
/// observing at the full duration the identity (L16).
fn visible<C: Coordinate>(span: Span<C>, window: Span<C>, duration: Duration<C>) -> bool {
    let is_point = span.start() == span.end();
    let at_the_end = is_point && span.start() == window.end() && window.end() == duration.reach();
    span.visible_through(window) || at_the_end
}

impl<C: Coordinate, A> EventTrack<C, A> {
    /// The ambient duration `d`.
    pub fn duration(&self) -> Duration<C> {
        self.duration
    }

    /// The occurrence multiset, in construction order. Canonical order is
    /// produced by [`EventTrack::normalize`].
    pub fn occurrences(&self) -> &[Occurrence<C, A>] {
        &self.occurrences
    }

    /// Observe through `window` (D6, §17): the occurrences the window shows,
    /// each keeping its whole span.
    pub fn restrict(&self, window: Span<C>) -> Observation<'_, C, A> {
        Observation {
            window,
            duration: self.duration,
            selected: self
                .occurrences
                .iter()
                .filter(|occurrence| visible(occurrence.span(), window, self.duration))
                .collect(),
        }
    }

    /// Functorial payload mapping: `map_payloads(f)(d, E) = (d, {(s,e,f(a))})`
    /// (D7). Laws L9–L12.
    pub fn map_payloads<B>(&self, f: impl Fn(&A) -> B) -> EventTrack<C, B> {
        EventTrack {
            duration: self.duration,
            occurrences: self
                .occurrences
                .iter()
                .map(|occurrence| Occurrence::new(occurrence.span(), f(occurrence.payload())))
                .collect(),
        }
    }

    /// D7 in place: every payload, mutably, in storage order.
    ///
    /// The same functor as [`Self::map_payloads`] for a caller that is
    /// rewriting `A` into `A` and would otherwise rebuild the occurrence vector
    /// to change a field. It cannot touch spans or the duration — that is the
    /// type, not a convention — which is what makes it safe to hand to an
    /// `instantiate` hook (`10-term-calculus.md` T6). Storage order, not
    /// canonical order: nothing here depends on the order, and canonicalizing
    /// to hand out mutable references would be a sort per instantiation.
    pub fn payloads_mut(&mut self) -> impl Iterator<Item = &mut A> {
        self.occurrences.iter_mut().map(Occurrence::payload_mut)
    }

    /// Exact time scaling by a positive rational (D5, §14). Laws L13–L15.
    ///
    /// # Errors
    /// [`EventsError::NonPositiveScale`](crate::EventsError::NonPositiveScale)
    /// for zero or negative factors.
    pub fn scale(&self, factor: Ratio<i64>) -> Result<Self, crate::EventsError>
    where
        A: Clone,
    {
        if factor <= Ratio::ZERO {
            return Err(crate::EventsError::NonPositiveScale {
                factor: format!("{factor}"),
            });
        }
        Ok(Self {
            duration: self.duration.times(factor),
            occurrences: self
                .occurrences
                .iter()
                .map(|occurrence| Occurrence::new(occurrence.span().scale(factor), occurrence.payload().clone()))
                .collect(),
        })
    }
}

impl<C: Coordinate, A: Canonical> EventTrack<C, A> {
    /// The occurrence multiset in canonical order (N2: start, end, payload
    /// key). Multiplicity is preserved — equal occurrences sort adjacent and
    /// are all retained (K6).
    pub fn canonical_occurrences(&self) -> Vec<&Occurrence<C, A>> {
        let mut ordered: Vec<&Occurrence<C, A>> = self.occurrences.iter().collect();
        // Cached: the payload key is a fresh `String`, and `sort_by_key`
        // rebuilds it on every comparison. Building it once per occurrence is
        // the difference between O(n log n) serializations and n.
        ordered.sort_by_cached_key(|occurrence| occurrence.canonical_key());
        ordered
    }

    /// Semantic equality (N4): equal durations and equal canonical occurrence
    /// multisets. Construction history is irrelevant — the event track is a
    /// semantic quotient (§20, §25). The coordinate does not appear because it
    /// is the type: two tracks in different coordinates cannot be compared.
    pub fn semantic_eq(&self, other: &Self) -> bool {
        self.duration == other.duration
            && self
                .canonical_occurrences()
                .iter()
                .map(|occurrence| occurrence.canonical_key())
                .eq(other
                    .canonical_occurrences()
                    .iter()
                    .map(|occurrence| occurrence.canonical_key()))
    }

    /// Re-canonicalize into the normal form (N1): same denotation,
    /// canonically ordered occurrences.
    #[must_use]
    pub fn normalize(&self) -> Self
    where
        A: Clone,
    {
        Self {
            duration: self.duration,
            occurrences: self.canonical_occurrences().into_iter().cloned().collect(),
        }
    }

    /// The occurrences whose support contains `at`, in canonical order
    /// (docs/rules/events/03 D10).
    ///
    /// "Contains" is [`Span::contains`]: support is half-open, so a fact is
    /// gone at its end instant, and a point fact is present at its own. The
    /// order is N2's, which makes the answer independent of how the track was
    /// built.
    ///
    /// This is a *point* query and a linear scan of a canonically ordered
    /// copy: right for asking once ("what is under the cursor"), wrong for
    /// deriving something about every event. Bulk derivation makes one
    /// ordered pass and uses D10's convention directly — see
    /// `docs/rules/events/03-denotational-semantics.md`.
    pub fn covering(&self, at: Position<C>) -> impl Iterator<Item = &Occurrence<C, A>> {
        self.canonical_occurrences()
            .into_iter()
            .filter(move |occurrence| occurrence.span().contains(at))
    }

    /// The value in force at `at`: the canonically last occurrence starting
    /// at or before `at` whose payload `select` accepts (docs/rules/events/03 D11).
    ///
    /// A fact starting exactly at `at` does prevail there — `dynamic mf` on a
    /// note applies to that note — and where two candidates start together
    /// the later in canonical order wins, which is deterministic because N2's
    /// order is total.
    ///
    /// `select` is how a caller says which facts are context-bearing. The
    /// event track must not know that a key is context and a note is not (§12), so
    /// the question arrives as a closure rather than as a payload trait; one
    /// track can then answer for key, meter, clef and dynamic independently,
    /// with no type per kind.
    ///
    /// The same scan caveat as [`Self::covering`] applies.
    pub fn prevailing<'a, V>(&'a self, at: Position<C>, select: impl Fn(&'a A) -> Option<V>) -> Option<V> {
        self.canonical_occurrences()
            .into_iter()
            .filter(|occurrence| occurrence.span().start() <= at)
            .filter_map(|occurrence| select(occurrence.payload()))
            .next_back()
    }
}

impl<C: Coordinate, A: Canonical> EventTrack<C, A> {
    /// A stable digest of the exact framed semantic encoding (N6).
    ///
    /// Equal canonical forms hash equal, in every run and every process:
    /// `self.semantic_eq(other)` implies `self.semantic_hash() ==
    /// other.semantic_hash()`. An unequal hash proves that the framed
    /// semantic encodings differ. Equal hashes are only candidate matches and
    /// require exact confirmation whenever correctness depends on equality.
    ///
    /// The encoding is versioned and uniquely frames the track, coordinate,
    /// payload schema, rational fields, occurrence count, and payload keys. The
    /// coordinate is framed as data even though it is carried as a type,
    /// because bytes have no type parameters: without it, a written-time track
    /// and a performed-time track with the same rationals would hash equal
    /// (`../across-stages/04-identity-and-realization.md` §2), and a cache
    /// keyed on the digest would return one for the other.
    ///
    /// It is intentionally separate from human [`Display`](std::fmt::Display),
    /// whose unescaped payload text is not an injective record encoding. It
    /// covers whatever the payload's
    /// [`canonical_key`](Canonical::canonical_key) covers — for musa's score
    /// facts, provenance included. A pure re-indentation moves source spans
    /// and changes the hash: the question this answers is "is this the same
    /// compiled piece", never "does it sound the same".
    pub fn semantic_hash(&self) -> crate::SemanticHash {
        let mut digest = crate::hash::Digest::new();
        self.write_semantic(&mut digest);
        digest.finish()
    }

    /// Write N6's private, versioned, uniquely framed semantic bytes, in the
    /// field order `04-identity-and-realization.md` §3 fixes.
    fn write_semantic(&self, out: &mut crate::hash::Digest) {
        const DOMAIN: &[u8] = b"musa.event-track.semantic";
        /// Version 3 adds the coordinate tag. Versions 1 and 2 are refused
        /// rather than reinterpreted (`docs/plan/clean-break-ledger.md` §3).
        const ENCODING_VERSION: u32 = 3;

        write_bytes(out, DOMAIN);
        out.write(&ENCODING_VERSION.to_be_bytes());
        write_bytes(out, C::NAME.as_bytes());
        write_bytes(out, A::OWNER_TYPE_ID.as_bytes());
        out.write(&A::QUOTIENT_VERSION.to_be_bytes());
        write_rational(out, self.duration.as_ratio());

        write_len(out, self.occurrences.len());
        for occurrence in self.canonical_occurrences() {
            write_rational(out, occurrence.span().start().as_ratio());
            write_rational(out, occurrence.span().end().as_ratio());
            write_bytes(out, occurrence.payload().canonical_key().as_bytes());
        }
    }
}

impl<C: Coordinate, A: Canonical> EventTrack<C, A> {
    /// The human canonical display (N5). This is deterministic but is not a
    /// persisted identity encoding; payload keys are intentionally readable
    /// and may contain its delimiters.
    fn write_display<W: std::fmt::Write>(&self, out: &mut W) -> std::fmt::Result {
        writeln!(out, "track {} {{", self.duration)?;
        for occurrence in self.canonical_occurrences() {
            writeln!(
                out,
                "  occurrence {} from {} to {};",
                occurrence.payload().canonical_key(),
                occurrence.span().start(),
                occurrence.span().end()
            )?;
        }
        writeln!(out, "}}")
    }
}

fn write_len(out: &mut crate::hash::Digest, len: usize) {
    // Every supported Rust target has `usize` no wider than `u64`.
    let len = u64::try_from(len).unwrap_or(u64::MAX);
    out.write(&len.to_be_bytes());
}

fn write_bytes(out: &mut crate::hash::Digest, bytes: &[u8]) {
    write_len(out, bytes.len());
    out.write(bytes);
}

fn write_rational(out: &mut crate::hash::Digest, value: Ratio<i64>) {
    out.write(&value.numer().to_be_bytes());
    out.write(&value.denom().to_be_bytes());
}

impl<C: Coordinate, A: Canonical> std::fmt::Display for EventTrack<C, A> {
    /// The canonical human serialization (N5): deterministic display bytes,
    /// deliberately separate from framed semantic identity (N6).
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.write_display(f)
    }
}
