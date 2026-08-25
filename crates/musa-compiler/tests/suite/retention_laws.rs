//! What a finished compilation leaves behind.
//!
//! A compilation is a finite process over an owned document, so when its
//! `Compilation` is dropped the bytes it took should be gone. They were not:
//! `crate::registry::owned()` built the prelude from scratch on every call, and
//! the prelude's `Globals` is a reference cycle — `Arc<Tables>` holds an
//! `Arc<Defined>`, a `Defined`'s type is a `Value`, a `Value` that is a `Pi`
//! captures an `Env` in its codomain closure, and an `Env` carries the same
//! `Globals`. `Arc` does not collect cycles, so every compilation left one whole
//! prelude behind: 257,208 bytes, the same figure whatever the program was.
//!
//! **Live bytes and not RSS.** RSS is a high-water mark that an allocator is
//! free never to return, and it called this leak clean: `musa check` over all
//! fifty-seven examples in one process peaked at 45 MB against the 35 MB of the
//! largest single one, because the leaked pages were reused for the next
//! compilation's live data. Only an allocator that subtracts on `dealloc` can
//! tell retention from reuse.
//!
//! The tally is per-thread, for the reason the audio real-time law gives about
//! its own: a `#[global_allocator]` is process-wide and this file
//! shares its binary with every other `musa-compiler` integration test, so a
//! process-wide counter would also count whatever runs beside it. libtest gives
//! every test its own thread, so a thread's own tally measures only the code
//! under test — whether the suite runs one test per process (`cargo nextest`)
//! or all of them at once (`cargo test`).

// The accounting harness must implement `GlobalAlloc`, an unsafe trait; it is
// the only unsafe code in this crate's tests, it delegates straight to
// `System`, and it exists only in this test binary.
#![allow(unsafe_code)]

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

use musa_compiler::{CompileOptions, SourceDocument, compile};

thread_local! {
    /// This thread's live bytes: `alloc` adds, `dealloc` subtracts.
    ///
    /// An `isize` because a thread that frees what another thread allocated
    /// goes negative, and a negative tally is a fact about the run rather than
    /// an overflow. `Cell<isize>` and not an atomic: the counter is only ever
    /// touched by its own thread, and an atomic per allocation would slow every
    /// test in this binary to buy nothing.
    static LIVE: Cell<isize> = const { Cell::new(0) };
}

/// This thread's live bytes.
fn live() -> isize {
    LIVE.with(Cell::get)
}

/// Add `bytes` to this thread's tally.
///
/// `try_with` rather than `with`: an allocation during thread teardown, after
/// the local is gone, must not panic out of the allocator.
fn note(bytes: isize) {
    let _ = LIVE.try_with(|tally| tally.set(tally.get().saturating_add(bytes)));
}

/// A `usize` size as the `isize` the tally counts in, saturating rather than
/// wrapping — an allocation larger than `isize::MAX` cannot succeed anyway.
fn signed(bytes: usize) -> isize {
    isize::try_from(bytes).unwrap_or(isize::MAX)
}

struct Accounting;

// Every method forwards to `System` and adjusts the thread's tally by the
// difference the call makes to live bytes. `realloc` is written out rather than
// left to the default so that a grow-in-place is counted once.
unsafe impl GlobalAlloc for Accounting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        note(signed(layout.size()));
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        note(signed(layout.size()).saturating_neg());
        unsafe { System.dealloc(pointer, layout) }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        note(signed(layout.size()));
        unsafe { System.alloc_zeroed(layout) }
    }

    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, new: usize) -> *mut u8 {
        note(signed(new).saturating_sub(signed(layout.size())));
        unsafe { System.realloc(pointer, layout, new) }
    }
}

#[global_allocator]
static ALLOCATOR: Accounting = Accounting;

/// The workload: a piece that elaborates. Small on purpose — the leak this
/// forbids was constant in the program, so the cheapest program that reaches
/// the elaborator states the law as well as the largest one and costs the suite
/// almost nothing.
const PIECE: &str =
    "piece \"held\" { tempo 1/4 = 60; meter 4/4; key c major; score { part p { voice v { c5/1 } } } }\n";

/// How many compilations to run past the first.
///
/// Enough that a per-compilation leak of any size is unmistakable — at the
/// 257,208 bytes this law was written for, twelve rounds separated the first
/// from the last by 2.8 MB — and small enough to stay a fast test.
const ROUNDS: usize = 12;

/// Bytes a run may end up holding over the first without failing.
///
/// Not zero, because a `LazyLock` further down the compiler may legitimately
/// fill on a later round, and not proportional to `ROUNDS`, because anything
/// that scales with the round count is the defect. In practice the measurement
/// is exactly flat.
const SLACK: isize = 64 * 1024;

/// A compilation releases what it held.
///
/// Stated as a *difference between rounds* rather than as an absolute figure:
/// the first compilation legitimately fills process-lifetime tables — the
/// prelude among them — and what must not happen is the second one filling them
/// again.
#[test]
fn a_finished_compilation_holds_nothing_the_next_one_does_not_reuse() {
    let compile_once = || {
        let document = SourceDocument::new(PIECE, "held.musa");
        let compilation = compile(&document, &CompileOptions::default());
        assert!(!compilation.has_errors(), "the workload compiles");
        drop(compilation);
        drop(document);
    };
    compile_once();
    let after_first = live();
    for round in 1..=ROUNDS {
        compile_once();
        let now = live();
        assert!(
            now.saturating_sub(after_first) <= SLACK,
            "round {round} left {} bytes standing over the first round's {after_first}, \
             which is {} a round — a finished compilation is holding something",
            now.saturating_sub(after_first),
            now.saturating_sub(after_first) / isize::try_from(round).unwrap_or(1),
        );
    }
}
