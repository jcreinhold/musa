//! The render side allocates nothing.
//!
//! The Audio Unit trial measured this on the Objective-C side and found two
//! artefacts before it found the truth (`docs/notes/research/93-the-audio-unit-shape.md`
//! §2): a rig that measures its own setup reports the rig. So this counts
//! Rust-side allocations only while a render is in flight, and proves the
//! counter first against a block that deliberately allocates.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use super::differential::{Fixture, prepare};
use musa_au::{MusaAuEvent, musa_au_instrument_release, musa_au_render};

static ARMED: AtomicBool = AtomicBool::new(false);
static COUNT: AtomicU64 = AtomicU64::new(0);

/// The system allocator, counting while armed.
///
/// Deliberately the whole test binary's allocator: an allocation this misses
/// because it happened somewhere else is exactly the kind this test exists to
/// catch.
struct Counting;

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if ARMED.load(Ordering::Relaxed) {
            COUNT.fetch_add(1, Ordering::Relaxed);
        }
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        unsafe { System.dealloc(pointer, layout) }
    }

    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        if ARMED.load(Ordering::Relaxed) {
            COUNT.fetch_add(1, Ordering::Relaxed);
        }
        unsafe { System.realloc(pointer, layout, new_size) }
    }
}

#[global_allocator]
static ALLOCATOR: Counting = Counting;

/// Run `work` with the counter armed, returning how many allocations it made.
fn measure(work: impl FnOnce()) -> u64 {
    COUNT.store(0, Ordering::Relaxed);
    ARMED.store(true, Ordering::Relaxed);
    work();
    ARMED.store(false, Ordering::Relaxed);
    COUNT.load(Ordering::Relaxed)
}

/// The counter counts. Without this the zero below means nothing.
#[test]
fn the_allocation_counter_is_working() {
    let counted = measure(|| {
        let deliberate: Vec<u8> = Vec::with_capacity(64);
        std::hint::black_box(&deliberate);
    });
    assert!(counted >= 1, "the counter observed no allocation from a deliberate one");
}

/// Rendering allocates nothing, with events and without.
///
/// The `#[ignore]` is because this prepares a real instrument twice and
/// renders two thousand blocks; it protects the render side's core promise,
/// and what still covers the boundary in the fast suite is
/// `boundary.rs`'s negative cases and `differential.rs`'s partition law. What
/// is deferred is breadth of event kinds, which the differential covers for
/// correctness but not for allocation.
#[test]
#[ignore = "slow: prepares a real instrument and renders two thousand blocks"]
fn slow_rendering_allocates_nothing() {
    let fixture = Fixture::new();
    let instrument = prepare(&fixture);
    let mut left = vec![0.0f32; 512];
    let mut right = vec![0.0f32; 512];
    let events = [
        MusaAuEvent {
            frame: 3,
            voice: 1,
            kind: musa_au::MUSA_AU_EVENT_NOTE_ON,
            data1: 60,
            data2: 100,
            reserved: 0,
        },
        MusaAuEvent {
            frame: 200,
            voice: 1,
            kind: musa_au::MUSA_AU_EVENT_PITCH_BEND,
            data1: 0,
            data2: 96,
            reserved: 0,
        },
    ];

    let empty = measure(|| {
        for _ in 0..2000 {
            unsafe {
                musa_au_render(
                    instrument,
                    std::ptr::null(),
                    0,
                    left.as_mut_ptr(),
                    right.as_mut_ptr(),
                    512,
                );
            }
        }
    });
    assert_eq!(empty, 0, "rendering an empty block allocated {empty} times");

    let carrying = measure(|| {
        for _ in 0..2000 {
            unsafe {
                musa_au_render(
                    instrument,
                    events.as_ptr(),
                    2,
                    left.as_mut_ptr(),
                    right.as_mut_ptr(),
                    512,
                );
            }
        }
    });
    unsafe { musa_au_instrument_release(instrument) };
    assert_eq!(carrying, 0, "rendering a block with events allocated {carrying} times");
}
