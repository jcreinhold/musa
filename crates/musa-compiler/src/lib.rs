//! Semantic core: from parsed source to immutable musical snapshots.
//!
//! Owns: name resolution, unit checking, semantic diagnostics, the high-level
//! compositional model (motifs, transformations, references), motif expansion,
//! exact rational musical time, score normalization, performance lowering,
//! provenance (`Origin`/`ExpansionStep`), and the public immutable snapshots
//! (`ScoreSnapshot`, `PerformancePlan`, `StudioSpec`) — design roadmap §15.3.
//!
//! Must never expose: compiler pass internals (resolution tables, expansion
//! machinery); transient `slotmap` keys as serialized identities. Must never
//! contain: notation planning, DSP, MIDI numbers in the score, or floating-
//! point musical time.
//!
//! Intended facade (roadmap §15.3), to be implemented by prompts 05–06 and 10:
//!
//! ```text
//! pub fn compile(syntax: &ParsedDocument, options: &CompileOptions) -> Compilation;
//! pub fn lower_performance(score: &ScoreSnapshot, options: &PerformanceOptions)
//!     -> Result<PerformancePlan, PerformanceError>;
//! ```
//!
//! Invariants: a `ScoreSnapshot` is finite, sorted by onset, immutable, and
//! keeps written pitch spelling (D♯ ≠ E♭); every expanded event carries the
//! full provenance path explaining why it exists; expansion always terminates
//! (the language has no recursion).
