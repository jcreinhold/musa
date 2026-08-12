//! The real-time contract as a test, not a hope (§13.2): `RenderPlan::render`
//! must not allocate. This lives in its own test binary so no other test's
//! allocations can land inside the measurement window.

// The allocation-counting harness must implement `GlobalAlloc`, an unsafe
// trait; this is the only unsafe code in the workspace, it delegates
// straight to `System`, and it exists only in this test binary.
#![allow(unsafe_code)]
#![allow(clippy::expect_used)]

use std::alloc::{GlobalAlloc, System};
use std::sync::atomic::{AtomicUsize, Ordering};

use musa_audio::{EventSlice, GraphOptions, ProcessorSpec, StudioGraphSpec, compile_graph};

static ALLOCS: AtomicUsize = AtomicUsize::new(0);

struct CountingAllocator;

unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: std::alloc::Layout) -> *mut u8 {
        ALLOCS.fetch_add(1, Ordering::SeqCst);
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
    let before = ALLOCS.load(Ordering::SeqCst);
    plan.render(&EventSlice::empty(), &mut output, 2048);
    let after = ALLOCS.load(Ordering::SeqCst);
    assert_eq!(before, after, "render allocated {} times", after.saturating_sub(before));
}
