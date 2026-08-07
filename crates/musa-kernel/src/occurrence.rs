//! Occurrences: typed values supported over spans (docs/kernel/03 D0).

use crate::time::Span;

/// Canonical serialization of a payload (docs/kernel/05 N3).
///
/// Keys must be deterministic, total, and injective on values, so semantic
/// equality of payloads is equality of keys. Implementations must never emit
/// addresses, hash-order data, or floats.
pub trait Canonical {
    /// The canonical key; distinct values produce distinct keys.
    fn canonical_key(&self) -> String;
}

impl Canonical for u8 {
    fn canonical_key(&self) -> String {
        self.to_string()
    }
}

impl Canonical for String {
    fn canonical_key(&self) -> String {
        self.clone()
    }
}

/// One typed occurrence: a payload supported over `[start, end)`. Duration is
/// the span's temporal support, never a payload field (D0).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Occurrence<A> {
    span: Span,
    payload: A,
}

impl<A> Occurrence<A> {
    /// An occurrence over `span` carrying `payload`. Bounds relative to the
    /// timeline's extent are checked by [`timeline`](crate::timeline).
    pub fn new(span: Span, payload: A) -> Self {
        Self { span, payload }
    }

    /// The temporal support.
    pub fn span(&self) -> Span {
        self.span
    }

    /// The payload.
    /// The payload, mutably. Only [`Timeline::payloads_mut`] hands this out,
    /// and only for D7-in-place; the span stays immutable.
    ///
    /// [`Timeline::payloads_mut`]: crate::Timeline::payloads_mut
    pub(crate) fn payload_mut(&mut self) -> &mut A {
        &mut self.payload
    }

    pub fn payload(&self) -> &A {
        &self.payload
    }

    /// The occurrence translated by `offset` beats (τ of docs/kernel/03 D2).
    pub(crate) fn translate(self, offset: crate::Beat) -> Self {
        Self {
            span: self.span.translate(offset),
            payload: self.payload,
        }
    }
}

impl<A: Canonical> Occurrence<A> {
    /// The canonical sort/equality key: start, then end, then payload (N2).
    pub(crate) fn canonical_key(&self) -> (crate::Beat, crate::Beat, String) {
        (self.span.start(), self.span.end(), self.payload.canonical_key())
    }
}
