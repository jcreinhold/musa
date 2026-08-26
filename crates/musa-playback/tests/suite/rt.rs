//! The real-time contract enforced: the callback
//! path (`CallbackCore::process` with install/transport churn) must not
//! allocate.
//!
//! A `#[global_allocator]` is process-wide, and this file shares its binary
//! with every other `musa-playback` integration test (one test target per crate
//! — see `docs/notes/toolchain/slow-test-suite.md`), so a global counter would
//! also count whatever the tests running beside it allocate. The count is kept
//! per thread instead: libtest gives every test its own thread, so a thread's
//! own tally measures only the code under test, whether the suite runs one
//! test per process (`cargo nextest`) or all of them at once (`cargo test`).

// The allocation-counting harness implements `GlobalAlloc` (an unsafe
// trait); it delegates straight to `System` and exists only here.
#![allow(unsafe_code)]
#![allow(clippy::expect_used)]

use std::alloc::{GlobalAlloc, System};
use std::cell::Cell;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Instant;

use super::support::{silent_plan, source_plan};
use musa_playback::testing::{AuditionMessage, CallbackCore, Message, MidiCallbackHarness};
use musa_playback::{AuditionEvent, PreparedPlaybackPlan, TransportCommand};

thread_local! {
    /// Allocations made by the current thread since it started.
    ///
    /// `const`-initialised and `Drop`-free, so reading it is a plain
    /// thread-local access: no lazy initialisation and no destructor to
    /// register, and hence nothing inside the allocator that could allocate
    /// and recurse.
    static ALLOCS: Cell<u64> = const { Cell::new(0) };
}

/// This thread's allocation count.
fn allocs() -> u64 {
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

const ROUTED: &str = concat!(
    "piece \"rt routing\" { tempo 1/4 = 60; meter 4/4; score { ",
    "part a { voice v { c4/1 } } part b { voice v { g4/1 } } } studio { ",
    "patch p { oscillator(sine) |> output; } assign a -> p; assign b -> p; ",
    "route a -> master; route b -> master; } }",
);

#[test]
fn the_callback_path_allocates_nothing() {
    let (mut commands, command_consumer) = rtrb::RingBuffer::<Message>::new(16);
    let (retired_producer, mut retired) = rtrb::RingBuffer::<Box<PreparedPlaybackPlan>>::new(4);
    let (mut audition, audition_consumer) = rtrb::RingBuffer::new(32);
    let position = Arc::new(AtomicU64::new(0));
    let playing = Arc::new(AtomicBool::new(false));
    let mut core =
        CallbackCore::new_with_audition(command_consumer, retired_producer, audition_consumer, position, playing);

    // Preload: install + play + loop + a queued replacement plan. The
    // measured blocks then cover install, retire, and transport handling.
    let routed = source_plan(ROUTED, 0);
    let audition_target = routed.audition_target("b").expect("routed part");
    commands.push(Message::Install(Box::new(routed))).expect("queue");
    commands
        .push(Message::Transport(TransportCommand::Play))
        .expect("queue");
    commands
        .push(Message::Transport(TransportCommand::SetLoop { start: 0, end: 512 }))
        .expect("queue");

    let mut output = vec![0.0f32; 512];
    core.process(&mut output); // warm-up: installs, applies transport

    audition
        .push(AuditionMessage {
            target: audition_target,
            event: AuditionEvent::NoteOn {
                voice: 1,
                note: 60,
                velocity: 96,
            },
        })
        .expect("audition queue");
    let replacement = Box::new(silent_plan(2_000_000));

    let before = allocs();
    // Consume live audition while its prepared target is still installed.
    core.process(&mut output);
    // Then cover replacement install, retirement, and transport handling in
    // the same measured callback window.
    commands.push(Message::Install(replacement)).expect("queue");
    commands
        .push(Message::Transport(TransportCommand::Seek { frame: 100 }))
        .expect("queue");
    for _ in 0..8 {
        core.process(&mut output);
    }
    let after = allocs();
    // Drain retired plans on the control side.
    while retired.pop().is_ok() {}
    assert_eq!(
        before,
        after,
        "callback path allocated {} times",
        after.saturating_sub(before)
    );
}

#[test]
fn the_midi_input_callback_allocates_nothing() {
    let mut callback = MidiCallbackHarness::new();
    callback.receive(1, 2, &[0x90, 60, 96]);
    assert!(callback.poll().is_some());

    let before = allocs();
    for stamp in 0..1_000 {
        callback.receive(stamp, stamp.saturating_add(2), &[0xB0, 64, 127]);
        assert!(callback.poll().is_some());
    }
    let after = allocs();
    assert_eq!(
        before,
        after,
        "MIDI callback allocated {} times",
        after.saturating_sub(before)
    );
    assert_eq!(callback.losses().queue_overflow, 0);
}

#[test]
fn latency_probe_reports_bounded_software_paths() {
    const SAMPLES: usize = 2_000;
    const MIDI_BATCH: usize = 64;
    let mut midi = MidiCallbackHarness::new();
    let mut midi_nanos = Vec::with_capacity(SAMPLES);
    for sample in 0..SAMPLES {
        let started = Instant::now();
        for offset in 0..MIDI_BATCH {
            let stamp = sample.saturating_mul(MIDI_BATCH).saturating_add(offset);
            let stamp = u64::try_from(stamp).unwrap_or(u64::MAX);
            midi.receive(stamp, stamp.saturating_add(2), &[0x90, 60, 96]);
            assert!(midi.poll().is_some());
        }
        let batch = u64::try_from(MIDI_BATCH).unwrap_or(1);
        midi_nanos.push(u64::try_from(started.elapsed().as_nanos()).unwrap_or(u64::MAX) / batch);
    }

    let (mut commands, command_consumer) = rtrb::RingBuffer::<Message>::new(4);
    let (retired_producer, _retired) = rtrb::RingBuffer::<Box<PreparedPlaybackPlan>>::new(2);
    let (mut audition, audition_consumer) = rtrb::RingBuffer::new(8);
    let position = Arc::new(AtomicU64::new(0));
    let playing = Arc::new(AtomicBool::new(false));
    let mut core =
        CallbackCore::new_with_audition(command_consumer, retired_producer, audition_consumer, position, playing);
    let plan = source_plan(ROUTED, 0);
    let target = plan.audition_target("b").expect("routed part");
    commands.push(Message::Install(Box::new(plan))).expect("queue");
    let mut output = [0.0; 128];
    core.process(&mut output);
    let mut render_nanos = Vec::with_capacity(SAMPLES);
    for voice in 0..SAMPLES {
        let voice = u32::try_from(voice).unwrap_or(u32::MAX);
        audition
            .push(AuditionMessage {
                target,
                event: AuditionEvent::NoteOn {
                    voice,
                    note: 60,
                    velocity: 96,
                },
            })
            .expect("queue");
        let started = Instant::now();
        core.process(&mut output);
        render_nanos.push(u64::try_from(started.elapsed().as_nanos()).unwrap_or(u64::MAX));
        audition
            .push(AuditionMessage {
                target,
                event: AuditionEvent::NoteOff { voice, velocity: 0 },
            })
            .expect("queue");
        core.process(&mut output);
    }

    let (midi_p50, midi_p95, midi_max) = latency_summary(&mut midi_nanos);
    let (render_p50, render_p95, render_max) = latency_summary(&mut render_nanos);
    eprintln!(
        "MIDI callback decode+ring ns: p50={midi_p50} p95={midi_p95} max={midi_max}; \
         audition event+128-frame block ns: p50={render_p50} p95={render_p95} max={render_max}"
    );
}

fn latency_summary(samples: &mut [u64]) -> (u64, u64, u64) {
    samples.sort_unstable();
    let last = samples.len().saturating_sub(1);
    (
        samples.get(last / 2).copied().unwrap_or_default(),
        samples.get(last.saturating_mul(95) / 100).copied().unwrap_or_default(),
        samples.get(last).copied().unwrap_or_default(),
    )
}

/// The measurement is the current thread's, not the process's.
///
/// Without this the contract above is only as strong as the test binary is
/// empty: the allocator is installed process-wide, so a sibling test
/// allocating on another thread used to land inside the window and fail
/// `the_callback_path_allocates_nothing` for reasons having nothing to do with
/// the callback.
#[test]
fn the_counter_ignores_other_threads() {
    let stop = Arc::new(AtomicBool::new(false));
    let progress = Arc::new(AtomicU64::new(0));
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

/// The callback path says nothing, at any level.
///
/// A held law rather than a measured one, and deliberately so: the allocation
/// test above catches a logging macro only when it *happens* to allocate, and
/// `tracing::trace!("x")` with no fields does not. Logging is I/O — a
/// subscriber may lock, may write to a file, may block on a pipe — so the rule
/// (roadmap §13) is that the callback does none of it, and the only way to
/// hold that for every level is to read the source.
///
/// `core.rs` is the whole of what runs under the real-time constraint:
/// everything the callback touches after `CallbackCore::process` is called
/// lives there or in the render plan, which is preallocated on the control
/// side. `engine.rs` is the control side and logs freely.
#[test]
fn the_callback_module_contains_no_logging() {
    let source = include_str!("../../src/core.rs");
    for (number, line) in source.lines().enumerate() {
        // The doc comments in `core.rs` discuss what may not happen there, so
        // a naive search for the word would find the rule and not a breach.
        let code = line.split("//").next().unwrap_or("");
        assert!(
            !code.contains("tracing::"),
            "crates/musa-playback/src/core.rs:{}: the audio callback does no I/O, and logging is I/O \
             (roadmap §13). Say it on the control side, in engine.rs, before the plan crosses the queue.",
            number.saturating_add(1)
        );
    }
}
