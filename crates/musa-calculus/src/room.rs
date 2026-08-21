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
//! it bounds the stack only where the *charge* tracks the descent. Through
//! `eval` and `quote` it does, at every depth: a chain of `let` a thousand deep
//! is refused at the 257th level, because those two charge as they descend.
//! Through the elaborator it does not. `Elaborator::check` and
//! `Elaborator::infer` stand inside one another and are charged nothing, and
//! they reach the bottom of a raw term before the values on the way back up
//! charge anything — so the counter reaches the limit `NESTING` levels from the
//! *bottom* of the term rather than from the top, and what the room buys is a
//! term some way past the limit rather than one arbitrarily past it. Measured
//! on a raw `let` chain in a debug build: 756 levels are refused and 1,256
//! abort. A term deep in the elaborator without being deep in the evaluator —
//! the left-nested application spine
//! `docs/plan/prompts/164-diagnostics-and-performance.md` measures, where
//! `infer` stood 516 frames deep while the nesting counter read 2 — is bounded
//! by no limit this room can be derived from at all.
//!
//! Both are one missing charge, and no amount of room substitutes for it: an
//! unbounded descent outruns any fixed stack. Charging that recursion is prompt
//! 144's third step, and it is a cost-table question rather than a stack one —
//! at 256 it would refuse a 128-note voice, which is a version bump argued in
//! §4 rather than a threshold moved to make a suite pass.

use std::cell::Cell;

use crate::budget::Budget;

/// The stack one nested evaluation level may spend, in bytes.
///
/// Not a language constant: [`Budget::NESTING`] decides acceptance, and this
/// decides what the implementation must arrange so that the decision can be
/// *reached*. §4.1 says as much — shrinking it is free and changes nothing
/// normative, while raising the *limit* is a cost-table version bump.
///
/// Measured on the chain real programs drive, `infer → check → eval → quote`,
/// rather than on `eval` recursing into itself — which is what the ~2 KiB note
/// above `eval.rs`'s `fn pi` measures, and why that number reads five times too
/// low for a program. A 256-level constructor tower elaborates on a thread of
/// 2,560 KiB and overflows one of 2,304 KiB, so a level costs under 10 KiB in a
/// debug build on arm64; the same tower wants between 512 and 768 KiB in a
/// release build, under 3 KiB a level. A chain of `let` measures the same, and
/// `eval` and `quote` alone measure under 4 KiB.
///
/// The ceiling is a little over three times the debug measurement, because it
/// has to hold for the deepest chain in the crate on targets and future arms
/// nobody has measured. To re-measure, replace the room below with a fixed size
/// and halve it until the law aborts rather than refuses:
///
/// ```sh
/// cargo nextest run -p musa-calculus -E 'test(elaborating_a_term_nested)'
/// cargo nextest run -p musa-calculus --cargo-profile release -E 'test(elaborating_a_term_nested)'
/// ```
///
/// `budget_laws.rs`'s `elaborating_a_term_nested_past_the_limit_is_refused` is
/// what notices when this stops being true, and it notices by aborting.
const FRAME_CEILING: u64 = 32 * 1024;

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
        if let Ok(running) = std::thread::Builder::new().stack_size(room).spawn_scoped(scope, run)
            && let Err(panic) = running.join()
        {
            std::panic::resume_unwind(panic);
        }
    });
    answer.unwrap_or_else(work)
}
