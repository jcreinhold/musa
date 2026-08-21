//! The real-time contract as a test, not a hope (§13.2): `RenderPlan::render`
//! must not allocate.
//!
//! A `#[global_allocator]` is process-wide, and this file shares its binary
//! with every other `musa-dsp` integration test (one test target per crate —
//! see `docs/notes/toolchain/slow-test-suite.md`), so a global counter would
//! also count whatever the tests running beside it allocate. The count is kept
//! per thread instead: libtest gives every test its own thread, so a thread's
//! own tally measures only the code under test, whether the suite runs one
//! test per process (`cargo nextest`) or all of them at once (`cargo test`).

// The allocation-counting harness must implement `GlobalAlloc`, an unsafe
// trait; it is the only unsafe code in this crate, it delegates straight to
// `System`, and it exists only in this test binary.
#![allow(unsafe_code)]
#![allow(clippy::expect_used)]

use std::alloc::{GlobalAlloc, System};
use std::cell::Cell;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use musa_dsp::{EventSlice, GraphOptions, ProcessorSpec, StudioGraphSpec, compile_graph};

thread_local! {
    /// Allocations made by the current thread since it started.
    ///
    /// `const`-initialised and `Drop`-free, so reading it is a plain
    /// thread-local access: no lazy initialisation and no destructor to
    /// register, and hence nothing inside the allocator that could allocate
    /// and recurse.
    static ALLOCS: Cell<usize> = const { Cell::new(0) };
}

/// This thread's allocation count.
fn allocs() -> usize {
    ALLOCS.with(Cell::get)
}

struct CountingAllocator;

unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: std::alloc::Layout) -> *mut u8 {
        // `try_with` rather than `with`: an allocation during thread teardown,
        // after the local is gone, must not panic out of the allocator.
        let _ = ALLOCS.try_with(|count| count.set(count.get().wrapping_add(1)));
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: std::alloc::Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
}

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

#[test]
fn render_allocates_nothing() {
    let mut spec = StudioGraphSpec::new();
    let sine = spec.add_node(ProcessorSpec::Sine);
    spec.set_param(sine, "frequency", 440.0).expect("param");
    let pan = spec.add_node(ProcessorSpec::Pan);
    let mixer = spec.add_node(ProcessorSpec::Mixer { inputs: 1 });
    spec.connect(sine, 0, pan, 0);
    spec.connect(pan, 0, mixer, 0);
    spec.set_output(mixer);
    let options = GraphOptions::default();
    let mut plan = compile_graph(&spec, &options).expect("compiles");
    let mut output = vec![0.0; 4096];
    plan.render(&EventSlice::empty(), &mut output, 2048); // warm up
    let before = allocs();
    plan.render(&EventSlice::empty(), &mut output, 2048);
    let after = allocs();
    assert_eq!(before, after, "render allocated {} times", after.saturating_sub(before));
}

/// The measurement is the current thread's, not the process's.
///
/// Without this the contract above is only as strong as the test binary is
/// empty: the allocator is installed process-wide, so a sibling test
/// allocating on another thread used to land inside the window and fail
/// `render_allocates_nothing` for reasons having nothing to do with `render`.
#[test]
fn the_counter_ignores_other_threads() {
    let stop = Arc::new(AtomicBool::new(false));
    let progress = Arc::new(AtomicUsize::new(0));
    let allocating = {
        let stop = Arc::clone(&stop);
        let progress = Arc::clone(&progress);
        std::thread::spawn(move || {
            while !stop.load(Ordering::SeqCst) {
                drop(std::hint::black_box(Box::new(0_u8)));
                progress.fetch_add(1, Ordering::SeqCst);
            }
        })
    };

    // Open the window only once the other thread is allocating, and hold it
    // open until it has allocated far more than a stray count would need.
    while progress.load(Ordering::SeqCst) == 0 {
        std::thread::yield_now();
    }
    let before = allocs();
    let start = progress.load(Ordering::SeqCst);
    while progress.load(Ordering::SeqCst) < start.saturating_add(10_000) {
        std::thread::yield_now();
    }
    let after = allocs();

    stop.store(true, Ordering::SeqCst);
    allocating.join().expect("the allocating thread finishes");
    assert_eq!(
        before,
        after,
        "counted {} of another thread's allocations",
        after.saturating_sub(before)
    );
}
