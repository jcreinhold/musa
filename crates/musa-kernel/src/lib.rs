//! The finite temporal kernel (docs/kernel/, course correction §§3–17): exact
//! ambient musical time, typed occurrences, `sequence`, `overlay`, ambient
//! extension, restriction, payload mapping, time scaling, normalization, and
//! semantic equality.
//!
//! Owns: the denotation `(d, E)` — an ambient extent `d ∈ ℚ≥0` and a finite
//! multiset of occurrences `(s, e, a)`, `0 ≤ s ≤ e ≤ d` — and its algebra.
//! Must never contain: musical semantics (payloads are opaque, §12),
//! provenance interpretation (provenance rides inside payloads, §20), floats
//! for symbolic time (§4), silence/rest objects (§2), or a monadic `join`
//! (§16).
//!
//! Facade (docs/kernel/03, §26): [`timeline`], [`sequence`], [`overlay`],
//! [`Timeline::extend`], [`Timeline::restrict`], [`Timeline::map_payload`],
//! [`Timeline::scale`], [`Timeline::normalize`], [`Timeline::semantic_eq`].
//! `Timeline` stores flat timelines directly — construction IS normalization
//! (docs/kernel/05 N1); `normalize` re-canonicalizes occurrence order.
//!
//! Invariants: occurrences form a multiset — equal occurrences never collapse
//! (§6); time is ambient — uncovered regions are silent by absence, nothing
//! represents silence (§2); semantic equality is equality of canonical forms,
//! never of construction history (§25).

mod error;
mod occurrence;
mod time;
mod timeline;

pub use crate::error::KernelError;
pub use crate::occurrence::{Canonical, Occurrence};
pub use crate::time::{Beat, Span};
pub use crate::timeline::{Observation, Timeline, overlay, sequence, timeline, zero};
