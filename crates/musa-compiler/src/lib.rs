//! The pipeline: from parsed source to the immutable musical values
//! `musa-score` defines.
//!
//! Pipeline (docs/plan/code-map/stage-pipeline.md): CST → expansion-aware
//! elaboration → temporal events (`musa-events`) → `ScoreSnapshot` adapter.
//! One semantic path: the event track — not the surface grammar — defines the
//! ontology.
//!
//! Owns: name resolution, unit checking, motif expansion, imports and
//! packages, elaboration through `musa-calculus`, the studio spec, lint, and
//! the documentation and reference readers — every stage that *works out* an
//! answer.
//!
//! Does not own the answers. Written pitch, chords, scales, exact musical
//! time, marks, the score snapshot, the performance plan, provenance,
//! diagnostics, and analysis are `musa-score`, one layer down, and this crate
//! depends on it rather than containing it. That was true before it was a
//! crate — no module there named a pass — and the boundary is what keeps it
//! true.
//!
//! Must never expose: pass internals (resolution tables, expansion
//! machinery); transient `slotmap` keys as serialized identities. Must never
//! contain: notation planning, DSP, MIDI numbers in the score, or floating-
//! point musical time.
//!
//! Facade (roadmap §15.3): [`compile`] and `lower_performance`.
//!
//! Invariants: expansion always terminates (the language has no recursion);
//! every expanded event carries the full provenance path explaining why it
//! exists.
/// Measurement seams for the benchmark suite. Not an interface: see the
/// module docs (roadmap §17.7).
#[doc(hidden)]
pub mod bench;
mod compile;
mod core;
mod core_budget;
mod data;
mod docs;
/// A whole document, elaborated through `musa-calculus`.
///
/// Nothing reaches it yet, and prompt 141o's Design says why: the walk is built
/// and proved one prompt before the cutover that wires it, so that a wrong walk
/// is distinguishable from a wrong migration. The expectation rather than an
/// `allow` is the point — prompt 142 calling it makes this unfulfilled, and the
/// compiler says so.
mod document;
mod elaborate;
mod events_text;
mod expand;
mod factext;
mod imports;
mod infer;
mod lint;
/// The surface CST read as a [`musa_calculus::Raw`].
///
/// Nothing reaches it yet, and prompt 141g's Design says why: the reading is
/// built and proved one prompt before the cutover that wires it, so that a wrong
/// reading is distinguishable from a wrong migration. The expectation rather
/// than an `allow` is the point — prompt 142 calling it makes this unfulfilled,
/// and the compiler says so.
mod lower;
mod module;
mod package;
/// The compiler's own `data` declarations, reachable only from [`registry`].
mod prelude;
mod project;
mod quote;
mod reference;
/// The compiler's own operations as `musa-calculus` registrations.
///
/// Nothing reaches it yet, and prompt 141e's Design says why: the registry is
/// built and proved one prompt before the cutover that uses it, so that a wrong
/// signature is distinguishable from a wrong migration. The expectation rather
/// than an `allow` is the point — prompt 142 wiring the elaborator makes it
/// unfulfilled, and the compiler says so.
mod registry;
mod resolve;
mod studio;
mod template;

pub use crate::compile::{Compilation, CompileOptions, DocumentKind, SourceDocument, compile, format_document};
pub use crate::docs::{ItemDoc, ItemSource, ParameterDoc, TypeNote};
#[doc(hidden)]
pub use crate::elaborate::events_normal_form;
#[doc(hidden)]
pub use crate::events_text::{
    EventsCheck, check_events_text, events_normalized_text, events_text, events_text_meaning,
};
pub use crate::expand::{AdapterEdit, AdapterEditError, AdapterPrintError, adapter_edits, adapter_print};
pub use crate::imports::{
    ImportSources, STANDARD_LIBRARY_LANGUAGE_VERSION, resolve_import, standard_library_module,
    standard_library_modules, standard_library_source,
};
/// Events text as an editor sees it, re-exported so a language server can
/// colour and outline an event track document without a second copy of the grammar
/// and without depending on `musa-events` itself.
///
/// The names carry `events` because a shell holds these beside
/// `musa-syntax`'s classification of *surface* text. Two classifiers over
/// two grammars are two things, and the unqualified word belongs to the
/// language a composer actually writes.
pub use musa_events::{EventsTokenClass, events_bindings, events_classify, events_keyword_doc};

pub use crate::reference::standard_library_reference;
pub use crate::resolve::{NameKind, NameReference, SourceLocation};
pub use crate::studio::{
    Assignment, Modulation, NodeIndex, ParamSpec, Patch, Processor, Route, Send, StudioNode, StudioSpec, Unit, Value,
};
/// The event track's semantic digest, re-exported so a consumer can hold a
/// compilation's identity without depending on the event track directly.
pub use musa_events::SemanticHash;
