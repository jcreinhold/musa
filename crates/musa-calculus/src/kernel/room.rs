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
//! Prompt 165 closes the two holes that note described. `check` and `infer`
//! charge as they descend, and the evaluator carries pending applications and
//! demanded terms on an explicit control stack. The remaining adapter workload
//! is bounded by the constructed-node counter: its traversal may still spend
//! host stack inside one registered operation, but a 1,500-level generated
//! region reaches 1,001,185 of 1,000,000 nodes and refuses. The ceiling below
//! is measured against that finished refusal as well as the direct nesting
//! laws, so every path the language admits reaches a named outcome.
//!
//! The desktop session thread deliberately keeps Rust's 2 MiB default. Every
//! calculus facade enters this seam before it checks or evaluates, including
//! calls made during adapter expansion, so sizing the session thread as well
//! would allocate the same room twice and would still miss non-desktop hosts.

use std::cell::Cell;

use crate::kernel::budget::Budget;

/// The stack one nested evaluation level may spend, in bytes.
///
/// Not a language constant: [`Budget::NESTING`] decides acceptance, and this
/// decides what the implementation must arrange so that the decision can be
/// *reached*. §4.1 says as much — shrinking it is free and changes nothing
/// normative, while raising the *limit* is a cost-table version bump.
///
/// **Re-measured after prompt 165's final traversal and size charges.** Holding
/// [`Budget::NESTING`] at 320 and bisecting this constant on arm64 puts the
/// debug wall between 72 and 80 KiB: 320 × 72 KiB aborts and 320 × 80 KiB
/// reaches the constructed-node refusal. The deepest law is
/// `musa-compiler`'s `a_region_deeper_than_the_budget_allows_is_refused_rather_than_fatal`,
/// which drives a 1,500-level traversal to 1,001,185 of 1,000,000 nodes;
/// the two direct nesting laws also run in the same command:
///
/// ```sh
/// env -u RUST_MIN_STACK cargo nextest run -p musa-calculus -p musa-compiler -E 'test(nested_past_the_limit) or test(a_region_deeper_than_the_budget_allows)'
/// env -u RUST_MIN_STACK cargo nextest run --cargo-profile release -p musa-calculus -p musa-compiler -E 'test(nested_past_the_limit) or test(a_region_deeper_than_the_budget_allows)'
/// ```
///
/// 128 KiB leaves 60% over the passing edge because it has to hold on targets
/// and future arms nobody has measured. Those laws notice when it stops being
/// true by aborting.
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
