//! The stack §4.1's nesting limit needs in order to be a refusal.
//!
//! `docs/rules/language/02-core-calculus.md` §4.1 has two halves and this is the
//! second one:
//!
//! > A limit is only a refusal if the machine survives long enough to print it.
//! > The implementation therefore owes a second, non-normative obligation: **the
//! > compiler must run checking and evaluation with at least `nesting limit ×
//! > frame ceiling` bytes of stack**, where the frame ceiling is a stated
//! > measured bound on what one level costs.
//!
//! [`Budget::NESTING`](crate::Budget::NESTING) is the guard; this is what makes
//! the guard's decision *reachable*. Neither half stands in for the other: room
//! alone only moves the cliff, and a limit alone only promises a diagnostic the
//! process may not live to print. Without this module a tower of 220 `Nat.Succ`
//! elaborated on `cargo nextest`'s test thread aborted with `has overflowed its
//! stack` at 215 levels — thirty-odd levels short of the limit that was supposed
//! to refuse it, and an abort is not one of §4's three outcomes.
//!
//! **This decides nothing about acceptance.** The budget does not move with the
//! room, so every host — a 2 MiB desktop session thread, a test thread, a wasm
//! shell with no threads at all — accepts and refuses exactly the same programs.
//! They differ only in what they survive, and this exists so that they do not
//! differ in that either.
//!
//! **What it does not repair.** The room is derived from the nesting limit, so
//! it bounds the stack only where the *charge* tracks the descent. Through the
//! traversal and `quote` it does, at every depth, because those two charge as
//! they descend. Through `eval` the question no longer arises: prompt 165a gave
//! the evaluator an explicit control stack, so its pending work is heap data
//! and its depth costs no host frames at all — what a `let` chain a thousand
//! deep now meets is the *step* budget, and only a chain nested in the values
//! rather than in the bodies is a descent at all.
//! Through the elaborator it does not. `Elaborator::check` and
//! `Elaborator::infer` stand inside one another and are charged nothing, and
//! they reach the bottom of a raw term before the values on the way back up
//! charge anything — so the counter reaches the limit `NESTING` levels from the
//! *bottom* of the term rather than from the top, and what the room buys is a
//! term some way past the limit rather than one arbitrarily past it. Measured
//! on a raw `let` chain in a debug build, at the 256-level limit that preceded
//! prompt 155a's bump: 756 levels are refused and 1,256 abort. Both scale with
//! the limit, because the room does. A term deep in the elaborator without being deep in the evaluator —
//! the left-nested application spine
//! `docs/plan/prompts/165-diagnostics-and-performance.md` measures, where
//! `infer` stood 516 frames deep while the nesting counter read 2 — is bounded
//! by no limit this room can be derived from at all.
//!
//! Both are one missing charge, and no amount of room substitutes for it: an
//! unbounded descent outruns any fixed stack. Charging that recursion is prompt
//! 144's third step, and it is a cost-table question rather than a stack one —
//! at the limit as it stands it would refuse a voice of a few hundred notes,
//! which is a version bump argued in §4 rather than a threshold moved to make a
//! suite pass.

use std::cell::Cell;

use crate::kernel::budget::Budget;

/// The stack one nested evaluation level may spend, in bytes.
///
/// Not a language constant: [`Budget::NESTING`] decides acceptance, and this
/// decides what the implementation must arrange so that the decision can be
/// *reached*. §4.1 says as much — shrinking it is free and changes nothing
/// normative, while raising the *limit* is a cost-table version bump.
///
/// **Re-measured at prompt 165a, and it went up.** The evaluator no longer
/// descends on the host's stack, so a nesting level is no longer bought mostly
/// by `eval` frames costing about 2 KiB each. It is bought by the descents that
/// remain — §5.9's traversal, `quote`, and the elaborator's own uncharged
/// `check`/`infer` — and those cost far more per level, so the *same* limit
/// now needs three or four times the room. The counter got scarcer and each
/// unit of it got more expensive; the product is what this constant tracks.
///
/// Measured by holding [`Budget::NESTING`] at 320 and bisecting this constant
/// until the deepest law aborts rather than refuses. In a debug build on arm64
/// the wall is between 60 and 64 KiB a level — 320 × 60 KiB overflows and
/// 320 × 64 KiB does not — and in a release build between 8 and 16 KiB, the
/// same four-to-one ratio the previous measurement found. The deepest law is
/// `musa-compiler`'s `a_region_deeper_than_the_budget_allows_is_refused_rather_than_fatal`,
/// which drives the traversal, rather than either of the two in `budget_laws.rs`:
///
/// ```sh
/// env -u RUST_MIN_STACK cargo nextest run -p musa-calculus -p musa-compiler -E 'test(nested_past_the_limit) or test(a_region_deeper_than_the_budget_allows)'
/// env -u RUST_MIN_STACK cargo nextest run --cargo-profile release -p musa-calculus -p musa-compiler -E 'test(nested_past_the_limit) or test(a_region_deeper_than_the_budget_allows)'
/// ```
///
/// 128 KiB is a little over twice the debug wall, because it has to hold on
/// targets and future arms nobody has measured. Those laws are what notice when
/// it stops being true, and they notice by aborting.
const FRAME_CEILING: u64 = 128 * 1024;

thread_local! {
    /// Whether this thread is one [`with_room`] already made.
    ///
    /// A facade operation reached from inside another one — a host δ-rule that
    /// normalizes, a checker that declares — is already standing on the room
    /// this arranges, and a second thread would give it no more depth while
    /// costing it a whole stack.
    static ROOMY: Cell<bool> = const { Cell::new(false) };
}

/// Run `work` with `NESTING × FRAME_CEILING` bytes of stack under it.
///
/// The room is derived rather than picked, so raising the published limit
/// cannot quietly outrun the stack that honours it.
///
/// A panic inside is a defect in this crate and is resumed on this side, so it
/// still looks like one rather than like an operation that quietly answered
/// nothing.
///
/// A host with no threads — the wasm shell — runs on the stack it was linked
/// with, where the room is a link-time setting instead; so does a host that
/// would not give us one. `work` is `Fn` rather than `FnOnce` for exactly that
/// path: the fallback has to be able to do work the thread never took.
///
/// **The two are not the same event and only one of them is silent.** The wasm
/// shell is a host that never had threads and whose room is arranged
/// elsewhere, so taking that path says nothing. A threaded host refusing us a
/// thread is §4.1's obligation going unmet: the work runs anyway, because most
/// terms are nowhere near the limit and failing them all would be a worse
/// answer than the risk, but a term deep enough to earn a refusal will now
/// abort instead of printing one — and an abort with nothing said beforehand
/// is indistinguishable from a compiler that crashed. So it is traced. This is
/// the one thing this crate learns and cannot return: roadmap §15.12 gives it
/// `tracing` and observes that "a crate whose every failure is a returned
/// diagnostic has nothing left to trace", which is true of all of it but this
/// — a host resource that went missing is not one of §4's three outcomes and
/// has no diagnostic to be.
pub(crate) fn with_room<T: Send>(work: impl Fn() -> T + Send + Sync) -> T {
    if cfg!(target_family = "wasm") || ROOMY.with(Cell::get) {
        return work();
    }
    let room = usize::try_from(Budget::NESTING.saturating_mul(FRAME_CEILING)).unwrap_or(usize::MAX);
    let mut answer = None;
    std::thread::scope(|scope| {
        let run = || {
            ROOMY.with(|roomy| roomy.set(true));
            answer = Some(work());
        };
        match std::thread::Builder::new().stack_size(room).spawn_scoped(scope, run) {
            Ok(running) => {
                if let Err(panic) = running.join() {
                    std::panic::resume_unwind(panic);
                }
            }
            Err(refused) => tracing::warn!(
                room,
                nesting = Budget::NESTING,
                error = %refused,
                "the host would not give us the stack §4.1 derives from the nesting limit; \
                 checking and evaluation run on the caller's own stack, where a term deep enough \
                 to be refused may abort the process instead of earning the refusal"
            ),
        }
    });
    answer.unwrap_or_else(work)
}
