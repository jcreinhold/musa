//! `Timeline<A>`: the finite kernel denotation `(d, E)` and its algebra
//! (docs/kernel/03). Stored flat — construction IS normalization
//! (docs/kernel/05 N1).
//!
//! Rational arithmetic on musa's magnitudes is total; the workspace
//! arithmetic lint is allowed module-wide (see musa-compiler/src/time.rs).
#![allow(clippy::arithmetic_side_effects)]

use num_rational::Ratio;

use crate::occurrence::{Canonical, Occurrence};
use crate::time::{Beat, Span};

/// A finite timeline: ambient extent `d ∈ ℚ≥0` plus a multiset of occurrences
/// supported within `[0, d]`. The extent is real — `(d, ∅)` is silence by
/// absence, not an error (§2, §9).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Timeline<A> {
    extent: Beat,
    occurrences: Vec<Occurrence<A>>,
}

/// Construct a timeline, checking `0 ≤ s ≤ e ≤ d` for every occurrence (K1).
///
/// # Errors
/// [`KernelError::OccurrenceOutOfBounds`](crate::KernelError::OccurrenceOutOfBounds)
/// for the first out-of-extent occurrence.
pub fn timeline<A>(extent: Beat, occurrences: Vec<Occurrence<A>>) -> Result<Timeline<A>, crate::KernelError> {
    if extent.as_ratio() < Ratio::ZERO {
        return Err(crate::KernelError::InvalidSpan {
            start: extent,
            end: extent,
        });
    }
    for occurrence in &occurrences {
        let span = occurrence.span();
        if span.end() > extent {
            return Err(crate::KernelError::OccurrenceOutOfBounds { span, extent });
        }
    }
    Ok(Timeline { extent, occurrences })
}

/// The empty timeline `(0, ∅)` — the two-sided identity of `sequence` (L2).
/// Infallible: no occurrence can violate the bounds of a zero extent.
pub fn zero<A>() -> Timeline<A> {
    Timeline {
        extent: Beat::ZERO,
        occurrences: Vec::new(),
    }
}

/// Temporal succession: `M ; N = (d + e, E ⊎ τ_d(F))` (D2). Laws L1–L3.
pub fn sequence<A>(parts: Vec<Timeline<A>>) -> Timeline<A> {
    let mut offset = Beat::ZERO;
    let mut occurrences = Vec::new();
    for part in parts {
        let part_extent = part.extent;
        occurrences.extend(
            part.occurrences
                .into_iter()
                .map(|occurrence| occurrence.translate(offset)),
        );
        offset = Beat::new(offset.as_ratio() + part_extent.as_ratio());
    }
    Timeline {
        extent: offset,
        occurrences,
    }
}

/// Simultaneous presence: `M ⊕ N = (max(d, e), E ⊎ F)` (D3). Nothing pads the
/// shorter timeline. Laws L4–L6; multiplicity is preserved (X1).
pub fn overlay<A>(parts: Vec<Timeline<A>>) -> Timeline<A> {
    let mut extent = Beat::ZERO;
    let mut occurrences = Vec::new();
    for part in parts {
        extent = extent.max(part.extent);
        occurrences.extend(part.occurrences);
    }
    Timeline { extent, occurrences }
}

/// A timeline observed through a window (D6, §17).
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
pub struct Observation<'a, A> {
    window: Span,
    /// The extent of the timeline being observed. The final instant of a
    /// timeline is observable (D6), and that is a fact about the timeline
    /// rather than about the window: without it, narrowing a full-extent
    /// observation would drop a point occurrence the wider one reported.
    extent: Beat,
    selected: Vec<&'a Occurrence<A>>,
}

impl<'a, A> Observation<'a, A> {
    /// The observation window.
    pub fn window(&self) -> Span {
        self.window
    }

    /// Narrow to `window ∩ self.window()`.
    ///
    /// Total by intersection: there is no containment precondition, so L17
    /// holds for every pair of windows rather than only for nested ones. Two
    /// windows that do not meet observe nothing, which is not the same as a
    /// degenerate window at the extent — hence the explicit empty case rather
    /// than a fabricated span the point-at-the-end rule would then read.
    #[must_use]
    pub fn restrict(&self, window: Span) -> Self {
        let start = self.window.start().max(window.start());
        let end = self.window.end().min(window.end());
        if start > end {
            return Self {
                window: Span::ZERO.translate(start),
                extent: self.extent,
                selected: Vec::new(),
            };
        }
        let window = self.window.clip(window);
        Self {
            window,
            extent: self.extent,
            selected: self
                .selected
                .iter()
                .copied()
                .filter(|occurrence| visible(occurrence.span(), window, self.extent))
                .collect(),
        }
    }

    /// Each observed occurrence with its visible span `whole ∩ window`.
    ///
    /// The occurrence keeps its whole span (§17): observation never moves an
    /// occurrence's claim about where it began.
    pub fn observed(&self) -> impl Iterator<Item = (Span, &'a Occurrence<A>)> + '_ {
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

/// Whether `span` is visible through `window` in a timeline of `extent`.
///
/// The final instant of a timeline is observable: a point occurrence at the
/// extent is shown by a window that ends there, which is what makes
/// observing at the full extent the identity (L16).
fn visible(span: Span, window: Span, extent: Beat) -> bool {
    let is_point = span.start() == span.end();
    let at_the_end = is_point && span.start() == window.end() && window.end() == extent;
    span.visible_through(window) || at_the_end
}

impl<A> Timeline<A> {
    /// The ambient extent `d`.
    pub fn extent(&self) -> Beat {
        self.extent
    }

    /// The occurrence multiset, in construction order. Canonical order is
    /// produced by [`Timeline::normalize`].
    pub fn occurrences(&self) -> &[Occurrence<A>] {
        &self.occurrences
    }

    /// Observe through `window` (D6, §17): the occurrences the window shows,
    /// each keeping its whole span.
    pub fn restrict(&self, window: Span) -> Observation<'_, A> {
        Observation {
            window,
            extent: self.extent,
            selected: self
                .occurrences
                .iter()
                .filter(|occurrence| visible(occurrence.span(), window, self.extent))
                .collect(),
        }
    }

    /// Functorial payload mapping: `Timeline(f)(d, E) = (d, {(s,e,f(a))})`
    /// (D7). Laws L9–L12.
    pub fn map_payload<B>(&self, f: impl Fn(&A) -> B) -> Timeline<B> {
        Timeline {
            extent: self.extent,
            occurrences: self
                .occurrences
                .iter()
                .map(|occurrence| Occurrence::new(occurrence.span(), f(occurrence.payload())))
                .collect(),
        }
    }

    /// D7 in place: every payload, mutably, in storage order.
    ///
    /// The same functor as [`Self::map_payload`] for a caller that is rewriting
    /// `A` into `A` and would otherwise rebuild the occurrence vector to change
    /// a field. It cannot touch spans or the extent — that is the type, not a
    /// convention — which is what makes it safe to hand to an `instantiate`
    /// hook (`10-term-calculus.md` T6). Storage order, not canonical order:
    /// nothing here depends on the order, and canonicalizing to hand out
    /// mutable references would be a sort per instantiation.
    pub fn payloads_mut(&mut self) -> impl Iterator<Item = &mut A> {
        self.occurrences.iter_mut().map(Occurrence::payload_mut)
    }

    /// Exact time scaling by a positive rational (D5, §14). Laws L13–L15.
    ///
    /// # Errors
    /// [`KernelError::NonPositiveScale`](crate::KernelError::NonPositiveScale)
    /// for zero or negative factors.
    pub fn scale(&self, factor: Ratio<i64>) -> Result<Self, crate::KernelError>
    where
        A: Clone,
    {
        if factor <= Ratio::ZERO {
            return Err(crate::KernelError::NonPositiveScale {
                factor: format!("{factor}"),
            });
        }
        Ok(Self {
            extent: Beat::new(self.extent.as_ratio() * factor),
            occurrences: self
                .occurrences
                .iter()
                .map(|occurrence| Occurrence::new(occurrence.span().scale(factor), occurrence.payload().clone()))
                .collect(),
        })
    }
}

impl<A: Canonical> Timeline<A> {
    /// The occurrence multiset in canonical order (N2: start, end, payload
    /// key). Multiplicity is preserved — equal occurrences sort adjacent and
    /// are all retained (K6).
    pub fn canonical_occurrences(&self) -> Vec<&Occurrence<A>> {
        let mut ordered: Vec<&Occurrence<A>> = self.occurrences.iter().collect();
        // Cached: the payload key is a fresh `String`, and `sort_by_key`
        // rebuilds it on every comparison. Building it once per occurrence is
        // the difference between O(n log n) serializations and n.
        ordered.sort_by_cached_key(|occurrence| occurrence.canonical_key());
        ordered
    }

    /// Semantic equality (N4): equal extents and equal canonical occurrence
    /// multisets. Construction history is irrelevant — the kernel is a
    /// semantic quotient (§20, §25).
    pub fn semantic_eq(&self, other: &Self) -> bool {
        self.extent == other.extent
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
            extent: self.extent,
            occurrences: self.canonical_occurrences().into_iter().cloned().collect(),
        }
    }

    /// The occurrences whose support contains `at`, in canonical order
    /// (docs/kernel/03 D8).
    ///
    /// "Contains" is [`Span::contains`]: support is half-open, so a fact is
    /// gone at its end instant, and a point fact is present at its own. The
    /// order is N2's, which makes the answer independent of how the timeline
    /// was built.
    ///
    /// This is a *point* query and a linear scan of a canonically ordered
    /// copy: right for asking once ("what is under the cursor"), wrong for
    /// deriving something about every event. Bulk derivation makes one
    /// ordered pass and uses D8's convention directly — see
    /// `docs/kernel/03-denotational-semantics.md`.
    pub fn covering(&self, at: crate::Beat) -> impl Iterator<Item = &Occurrence<A>> {
        self.canonical_occurrences()
            .into_iter()
            .filter(move |occurrence| occurrence.span().contains(at))
    }

    /// The value in force at `at`: the canonically last occurrence starting
    /// at or before `at` whose payload `select` accepts (docs/kernel/03 D9).
    ///
    /// A fact starting exactly at `at` does prevail there — `dynamic mf` on a
    /// note applies to that note — and where two candidates start together
    /// the later in canonical order wins, which is deterministic because N2's
    /// order is total.
    ///
    /// `select` is how a caller says which facts are context-bearing. The
    /// kernel must not know that a key is context and a note is not (§12), so
    /// the question arrives as a closure rather than as a payload trait; one
    /// timeline can then answer for key, meter, clef and dynamic
    /// independently, with no type per kind.
    ///
    /// The same scan caveat as [`Self::covering`] applies.
    pub fn prevailing<'a, V>(&'a self, at: crate::Beat, select: impl Fn(&'a A) -> Option<V>) -> Option<V> {
        self.canonical_occurrences()
            .into_iter()
            .filter(|occurrence| occurrence.span().start() <= at)
            .filter_map(|occurrence| select(occurrence.payload()))
            .next_back()
    }

    /// A stable digest of the exact framed semantic encoding (N6).
    ///
    /// Equal canonical forms hash equal, in every run and every process:
    /// `self.semantic_eq(other)` implies `self.semantic_hash() ==
    /// other.semantic_hash()`. An unequal hash proves that the framed
    /// semantic encodings differ. Equal hashes are only candidate matches and
    /// require exact confirmation whenever correctness depends on equality.
    ///
    /// The encoding is versioned and uniquely frames the timeline, payload
    /// schema, rational fields, occurrence count, and payload keys. It is
    /// intentionally separate from human [`Display`](std::fmt::Display),
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

    /// The human canonical display (N5). This is deterministic but is not a
    /// persisted identity encoding; payload keys are intentionally readable
    /// and may contain its delimiters.
    fn write_display<W: std::fmt::Write>(&self, out: &mut W) -> std::fmt::Result {
        writeln!(out, "timeline {} {{", self.extent)?;
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

    /// Write N6's private, versioned, uniquely framed semantic bytes.
    fn write_semantic(&self, out: &mut crate::hash::Digest) {
        const DOMAIN: &[u8] = b"musa.timeline.semantic";
        const ENCODING_VERSION: u32 = 2;

        write_bytes(out, DOMAIN);
        out.write(&ENCODING_VERSION.to_be_bytes());
        write_bytes(out, A::OWNER_TYPE_ID.as_bytes());
        out.write(&A::QUOTIENT_VERSION.to_be_bytes());
        write_rational(out, self.extent);

        write_len(out, self.occurrences.len());
        for occurrence in self.canonical_occurrences() {
            write_rational(out, occurrence.span().start());
            write_rational(out, occurrence.span().end());
            write_bytes(out, occurrence.payload().canonical_key().as_bytes());
        }
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

fn write_rational(out: &mut crate::hash::Digest, value: Beat) {
    let ratio = value.as_ratio();
    out.write(&ratio.numer().to_be_bytes());
    out.write(&ratio.denom().to_be_bytes());
}

impl<A: Canonical> std::fmt::Display for Timeline<A> {
    /// The canonical human serialization (N5): deterministic display bytes,
    /// deliberately separate from framed semantic identity (N6).
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.write_display(f)
    }
}
