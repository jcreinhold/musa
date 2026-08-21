//! The stack-room constants adapter expansion is run under.
//!
//! Typed elaboration and metering live in `musa-calculus` now (`musa_calculus::budget`
//! owns the language's acceptance limits); what remains here is the host-side
//! arithmetic that gives one expansion a real stack: [`crate::phase::with_room`]
//! runs a transformer on a dedicated thread `NESTING * FRAME_CEILING` bytes
//! deep, because the language's own nesting limit is stated in levels and the
//! process stack is stated in bytes, and somebody has to multiply them.
//!
//! The old evaluator's resource accounting — the cost table, the work meter,
//! the resource-error taxonomy — left with it in the course correction.

/// How many evaluator frames may stand inside one another
/// (`../rules/language/02-core-calculus.md` §4).
///
/// The limit that turns a stack overflow into a refusal. The inherited-context
/// recursor descends *through* the transformer's own branches, so one level of
/// source nesting costs a whole chain of `eval`/`apply_closure` frames, and a
/// region deep enough to exhaust the machine's stack would abort the process
/// rather than earn a diagnostic — which a total language whose budget system
/// exists precisely to refuse rather than crash cannot have.
///
/// 256 because it is past anything a person writes and short of anything a
/// host cannot hold. The reference recursor of `crate::expand`'s law suite
/// spends four levels per level of source nesting, so 256 admits regions
/// nested some sixty deep; a transformer that does more per node admits
/// proportionally less, and still more than a real region needs.
pub(crate) const NESTING: u64 = 256;

/// The stack one nested evaluation level may spend, in bytes.
///
/// Not a language constant: [`NESTING`] decides acceptance, and this decides
/// what the implementation must arrange so that the decision can be *reached*.
/// A limit no host can afford to run up to would be a limit that still aborts.
///
/// Measured at 62,876 bytes per level in a debug build on arm64 — `eval` and
/// `eval_builtin` are the two fat frames, at 22,608 and 50,528 bytes, because a
/// debug build gives every arm of a large match its own slots — and 5,968 in a
/// release build. The ceiling doubles the debug measurement, since it has to
/// hold on targets and future arms nobody has measured. Shrinking those two
/// matches is how this number comes down; it is not how the refusal happens.
pub(crate) const FRAME_CEILING: u64 = 128 * 1024;
