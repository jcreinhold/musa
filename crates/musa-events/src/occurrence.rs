//! Occurrences: typed values supported over spans (docs/rules/events/03 D0).

use crate::time::{Coordinate, Duration, Position, Span};

/// Admitted semantic equality for a payload (docs/rules/events/05 N3, 12).
///
/// Key equality defines the payload's admitted equality. A key must be
/// deterministic, total, and complete for those equality classes; it need
/// not distinguish stored values which the declared quotient intentionally
/// identifies. Implementations must never emit addresses, hash-order data,
/// or floats. Changing the observed fields or their encoding requires a new
/// [`QUOTIENT_VERSION`](Self::QUOTIENT_VERSION).
pub trait Canonical {
    /// Stable owner of this payload schema, framed into temporal identity.
    const OWNER_TYPE_ID: &'static str;

    /// Version of the equality projection and canonical-key encoding.
    const QUOTIENT_VERSION: u32;

    /// The canonical key; equal keys are definitionally equal payloads.
    fn canonical_key(&self) -> String;
}

impl Canonical for u8 {
    const OWNER_TYPE_ID: &'static str = "musa.events.u8";
    const QUOTIENT_VERSION: u32 = 1;

    fn canonical_key(&self) -> String {
        self.to_string()
    }
}

impl Canonical for String {
    const OWNER_TYPE_ID: &'static str = "rust.alloc.string.String";
    const QUOTIENT_VERSION: u32 = 1;

    fn canonical_key(&self) -> String {
        self.clone()
    }
}

/// One typed occurrence: a payload supported over `[start, end)` in coordinate
/// `C`. Duration is the span's temporal support, never a payload field (D0).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Occurrence<C: Coordinate, A> {
    span: Span<C>,
    payload: A,
}

impl<C: Coordinate, A> Occurrence<C, A> {
    /// An occurrence over `span` carrying `payload`. Bounds relative to the
    /// track's duration are checked by [`track`](crate::track).
    pub fn new(span: Span<C>, payload: A) -> Self {
        Self { span, payload }
    }

    /// The temporal support.
    pub fn span(&self) -> Span<C> {
        self.span
    }

    /// The payload, mutably. Only [`EventTrack::payloads_mut`] hands this out,
    /// and only for D7-in-place; the span stays immutable.
    ///
    /// [`EventTrack::payloads_mut`]: crate::EventTrack::payloads_mut
    pub(crate) fn payload_mut(&mut self) -> &mut A {
        &mut self.payload
    }

    /// The payload.
    pub fn payload(&self) -> &A {
        &self.payload
    }

    /// The occurrence translated later by `offset` (τ of docs/rules/events/03 D2).
    pub(crate) fn translate(self, offset: Duration<C>) -> Self {
        Self {
            span: self.span.translate(offset),
            payload: self.payload,
        }
    }
}

impl<C: Coordinate, A: Canonical> Occurrence<C, A> {
    /// The canonical sort/equality key: start, then end, then payload (N2).
    pub(crate) fn canonical_key(&self) -> (Position<C>, Position<C>, String) {
        (self.span.start(), self.span.end(), self.payload.canonical_key())
    }
}
