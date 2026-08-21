//! Name resolution over the parsed piece: reference indexing, binding names,
//! and the concrete readings of the front matter (profiles, parts, marks,
//! meter and key).
//!
//! The seven submodules split the one resolution concern by role: `types`
//! holds the reference and binding data the pass shares, `resolver` the
//! [`Resolver`] itself plus the span and token utilities, `registration`
//! the registration of motifs, fragments and bars, `suggest` the
//! close-name suggestions for misspellings, `profiles` the reading of
//! performance profiles, `parts` the per-part facts and markings, and
//! `checks` the two sanity checks that run after resolution.

mod checks;
mod parts;
mod profiles;
mod registration;
mod resolver;
mod suggest;
mod types;

pub(crate) use checks::{check_groove_has_a_meter, check_measure_sanity};
pub(crate) use parts::{Marking, parse_duration, parse_key, parse_meter, parse_ratio, part_facts, tempo_marking};
pub(crate) use profiles::parse_profiles;
pub(crate) use registration::{
    lower_header, lower_studio, merge_profiles, register_bars, register_fragments, register_motifs,
};
pub(crate) use resolver::{Resolver, source_span_of, span_of, token_span, token_text, trimmed_span};
pub(crate) use suggest::{suggest, suggest_name};
pub(crate) use types::{Material, ReferenceIndex};
pub use types::{NameKind, NameReference, SourceLocation};
