//! The finite event-track core (docs/rules/kernel/): exact ambient musical
//! time tagged by coordinate, typed occurrences, `follow`, `together`,
//! restriction, payload mapping, time scaling, normalization, and semantic
//! equality.
//!
//! Owns: the denotation `(d, E)` — an ambient duration `d ∈ ℚ≥0` and a finite
//! multiset of occurrences `(s, e, a)`, `0 ≤ s ≤ e ≤ d` — and its algebra.
//! Must never contain: musical semantics (payloads are opaque, §12),
//! provenance interpretation (provenance rides inside payloads, §20), floats
//! for symbolic time (§4), silence/rest objects (§2), a monadic `join` (§16),
//! or anything from the sound layer — no machine, no audio frame.
//!
//! Facade (docs/rules/kernel/00-purpose.md, §26). The basis is [`empty`],
//! [`event`], [`follow`], [`together`], [`EventTrack::map_payloads`], and
//! [`EventTrack::duration`]; [`track`], [`EventTrack::scale`],
//! [`EventTrack::restrict`], [`EventTrack::covering`],
//! [`EventTrack::prevailing`], [`EventTrack::normalize`],
//! [`EventTrack::semantic_eq`], and [`EventTrack::semantic_hash`] are retained
//! beyond it because they have named callers. `EventTrack` stores flat tracks
//! directly — construction IS normalization (docs/rules/kernel/05 N1);
//! `normalize` re-canonicalizes occurrence order.
//!
//! Invariants: occurrences form a multiset — equal occurrences never collapse
//! (§6); time is ambient — uncovered regions are silent by absence, nothing
//! represents silence (§2); semantic equality is equality of canonical forms,
//! never of construction history (§25); the coordinate is a type index, so
//! tracks in two coordinates never combine and no operation converts one.

mod editor;
mod error;
mod hash;
mod occurrence;
mod progress;
mod term;
mod text;
mod time;
mod track;

pub use crate::editor::{TokenClass, bindings, classify, keyword_doc};
pub use crate::error::KernelError;
pub use crate::hash::{SemanticHash, stable_digest};
pub use crate::occurrence::{Canonical, Occurrence};
pub use crate::progress::Progress;
pub use crate::term::{Term, evaluate, evaluate_marked};
pub use crate::text::{
    Document, FORMAT_VERSION, Opaque, PayloadText, TextPayload, notes, parse, parse_expression, print, read,
};
pub use crate::time::{Coordinate, Duration, PerformedTime, PhysicalTime, Position, Span, WrittenTime};
pub use crate::track::{EventTrack, Observation, empty, event, follow, together, track};
