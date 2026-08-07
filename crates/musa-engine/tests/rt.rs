//! The real-time contract enforced (roadmap §13.2, §17.5): the callback
//! path (`CallbackCore::process` with install/transport churn) must not
//! allocate. Own test binary so no other test's allocations pollute the
//! measurement window.

// The allocation-counting harness implements `GlobalAlloc` (an unsafe
// trait); it delegates straight to `System` and exists only here.
#![allow(unsafe_code)]
#![allow(clippy::expect_used)]

use std::alloc::{GlobalAlloc, System};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use musa_engine::testing::{CallbackCore, Message};
use musa_engine::{PreparedPlaybackPlan, TransportCommand};

static ALLOCS: AtomicU64 = AtomicU64::new(0);

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

fn silent_plan(total_frames: u64) -> PreparedPlaybackPlan {
    let options = musa_audio::GraphOptions::default();
    let render_plan = musa_audio::compile_graph(&musa_audio::poly_sine_spec(8), &options).expect("graph");
    PreparedPlaybackPlan::new(render_plan, Vec::new(), total_frames)
}

#[test]
fn the_callback_path_allocates_nothing() {
    let (mut commands, command_consumer) = rtrb::RingBuffer::<Message>::new(16);
    let (retired_producer, mut retired) = rtrb::RingBuffer::<Box<PreparedPlaybackPlan>>::new(4);
    let position = Arc::new(AtomicU64::new(0));
    let playing = Arc::new(AtomicBool::new(false));
    let mut core = CallbackCore::new(command_consumer, retired_producer, position, playing);

    // Preload: install + play + loop + a queued replacement plan. The
    // measured blocks then cover install, retire, and transport handling.
    commands
        .push(Message::Install(Box::new(silent_plan(1_000_000))))
        .expect("queue");
    commands
        .push(Message::Transport(TransportCommand::Play))
        .expect("queue");
    commands
        .push(Message::Transport(TransportCommand::SetLoop { start: 0, end: 512 }))
        .expect("queue");

    let mut output = vec![0.0f32; 512];
    core.process(&mut output); // warm-up: installs, applies transport

    // Queue more work for the measured blocks: a replacement install (which
    // retires the current plan) and seeks.
    commands
        .push(Message::Install(Box::new(silent_plan(2_000_000))))
        .expect("queue");
    commands
        .push(Message::Transport(TransportCommand::Seek { frame: 100 }))
        .expect("queue");

    let before = ALLOCS.load(Ordering::SeqCst);
    for _ in 0..8 {
        core.process(&mut output);
    }
    let after = ALLOCS.load(Ordering::SeqCst);
    // Drain retired plans on the control side.
    while retired.pop().is_ok() {}
    assert_eq!(
        before,
        after,
        "callback path allocated {} times",
        after.saturating_sub(before)
    );
}
