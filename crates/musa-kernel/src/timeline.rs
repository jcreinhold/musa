//! `Timeline<A>`: the finite kernel denotation `(d, E)` and its algebra
//! (docs/kernel/03). Stored flat — construction IS normalization
//! (docs/kernel/05 N1).
//!
//! Rational arithmetic on musa's magnitudes is total; the workspace
//! arithmetic lint is allowed module-wide (see musa-compiler/src/time.rs).
#![allow(clippy::arithmetic_side_effects)]

use num_rational::Ratio;

use crate::occurrence::{Canonical, ObservedOccurrence, Occurrence};
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

/// The result of observing a timeline through a window (D6).
#[derive(Clone, Debug)]
pub struct RestrictedView<'a, A> {
    window: Span,
    observed: Vec<ObservedOccurrence<'a, A>>,
}

impl<'a, A> RestrictedView<'a, A> {
    /// The observation window.
    pub fn window(&self) -> Span {
        self.window
    }

    /// Observed occurrences, each with whole and visible spans.
    pub fn observed(&self) -> &[ObservedOccurrence<'a, A>] {
        &self.observed
    }
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

    /// Ambient extension: `(e, E)` for `e ≥ d`, introducing nothing (D4).
    ///
    /// # Errors
    /// [`KernelError::ShrinkingExtension`](crate::KernelError::ShrinkingExtension)
    /// when `new_extent < self.extent()` — cropping is [`Timeline::restrict`].
    pub fn extend(&self, new_extent: Beat) -> Result<Self, crate::KernelError>
    where
        A: Clone,
    {
        if new_extent < self.extent {
            return Err(crate::KernelError::ShrinkingExtension {
                from: self.extent,
                to: new_extent,
            });
        }
        Ok(Self {
            extent: new_extent,
            occurrences: self.occurrences.clone(),
        })
    }

    /// Observe through `window` (D6, §17): occurrences intersecting the
    /// window report both whole and visible spans.
    pub fn restrict(&self, window: Span) -> RestrictedView<'_, A> {
        let observed = self
            .occurrences
            .iter()
            .filter(|occurrence| {
                let span = occurrence.span();
                // Point occurrences at the ambient extent's end are observable
                // through a window ending at the extent; otherwise `restrict`
                // at the full extent would not be the identity (L16).
                let is_point = span.start() == span.end();
                let point_at_windows_extent_end = is_point && span.start() == window.end();
                span.visible_through(window) || (point_at_windows_extent_end && window.end() == self.extent)
            })
            .map(|occurrence| {
                ObservedOccurrence::new(occurrence.span(), occurrence.span().clip(window), occurrence.payload())
            })
            .collect();
        RestrictedView { window, observed }
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
        ordered.sort_by_key(|occurrence| occurrence.canonical_key());
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
}

impl<A: Canonical> std::fmt::Display for Timeline<A> {
    /// The canonical serialization (N5): deterministic bytes for golden
    /// tests and semantic hashes.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "timeline {} {{", self.extent)?;
        for occurrence in self.canonical_occurrences() {
            writeln!(
                f,
                "  occurrence {} from {} to {};",
                occurrence.payload().canonical_key(),
                occurrence.span().start(),
                occurrence.span().end()
            )?;
        }
        writeln!(f, "}}")
    }
}
