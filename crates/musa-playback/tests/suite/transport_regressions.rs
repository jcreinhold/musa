//! Regressions for the two callback-core contracts that the transport tests
//! did not pin down:
//!
//! 1. **Progress.** `CallbackCore::process` returns after a bounded amount of
//!    work for *every* transport state. A degenerate loop region must not
//!    spin the audio thread.
//! 2. **No plan is ever dropped in the callback.** A replaced plan always
//!    reaches the control side, even when the retirement queue is saturated;
//!    back-pressure leaves commands queued rather than destroying a plan on
//!    the audio thread.

#![allow(clippy::expect_used)]

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64};
use std::time::Duration;

use super::support::silent_plan;
use musa_playback::testing::{CallbackCore, Message};
use musa_playback::{PreparedPlaybackPlan, TransportCommand};

const BLOCK_FRAMES: usize = 256;

fn plan(total_frames: u64) -> PreparedPlaybackPlan {
    silent_plan(total_frames)
}

struct Harness {
    commands: rtrb::Producer<Message>,
    retired: rtrb::Consumer<Box<PreparedPlaybackPlan>>,
    core: CallbackCore,
}

impl Harness {
    fn new(command_capacity: usize, retired_capacity: usize) -> Self {
        let (commands, command_consumer) = rtrb::RingBuffer::<Message>::new(command_capacity);
        let (retired_producer, retired) = rtrb::RingBuffer::<Box<PreparedPlaybackPlan>>::new(retired_capacity);
        let core = CallbackCore::new(
            command_consumer,
            retired_producer,
            Arc::new(AtomicU64::new(0)),
            Arc::new(AtomicBool::new(false)),
        );
        Self {
            commands,
            retired,
            core,
        }
    }

    fn send(&mut self, message: Message) {
        self.commands
            .push(message)
            .map_err(|_| ())
            .expect("command queue has room");
    }

    fn block(&mut self) {
        let mut output = vec![0.0f32; BLOCK_FRAMES * 2];
        self.core.process(&mut output);
    }
}

/// Run `body` on a worker thread, failing instead of hanging the suite.
fn within(limit: Duration, body: impl FnOnce() + Send + 'static) {
    let (done, wait) = std::sync::mpsc::channel();
    let worker = std::thread::spawn(move || {
        body();
        let _ = done.send(());
    });
    assert!(
        wait.recv_timeout(limit).is_ok(),
        "the callback did not return within {limit:?} — the audio thread would be wedged"
    );
    worker.join().expect("worker thread finished cleanly");
}

/// A loop whose end is not after its start describes no region at all.
/// Honouring it would seek back to `start` forever without filling a single
/// frame, wedging the audio thread.
#[test]
fn regression_degenerate_loop_region_still_returns() {
    for (start, end) in [(512_u64, 512_u64), (900, 400), (0, 0)] {
        within(Duration::from_secs(5), move || {
            let mut harness = Harness::new(16, 16);
            harness.send(Message::Install(Box::new(plan(10_000))));
            harness.send(Message::Transport(TransportCommand::Play));
            harness.send(Message::Transport(TransportCommand::SetLoop { start, end }));
            harness.block();
            harness.block();
        });
    }
}

/// A loop region that starts at or past the end of the piece is likewise
/// unplayable: playback stops rather than spinning.
#[test]
fn regression_loop_beyond_the_piece_stops_instead_of_spinning() {
    within(Duration::from_secs(5), || {
        let mut harness = Harness::new(16, 16);
        harness.send(Message::Install(Box::new(plan(1_000))));
        harness.send(Message::Transport(TransportCommand::Play));
        harness.send(Message::Transport(TransportCommand::SetLoop {
            start: 5_000,
            end: 6_000,
        }));
        harness.block();
    });
}

/// The callback must never destroy a large object. With the retirement
/// queue saturated the core applies back-pressure — commands stay queued — and
/// every replaced plan still reaches the control side once it drains.
#[test]
fn regression_saturated_retirement_queue_never_drops_a_plan() {
    const INSTALLS: usize = 6;
    let mut harness = Harness::new(16, 1);
    for _ in 0..INSTALLS {
        harness.send(Message::Install(Box::new(plan(10_000))));
    }

    // Alternate callback blocks with control-side draining, exactly as the
    // engine does. Every install past the first retires its predecessor.
    let mut recovered = 0usize;
    for _ in 0..(INSTALLS * 4) {
        harness.block();
        while harness.retired.pop().is_ok() {
            recovered = recovered.saturating_add(1);
        }
    }

    assert_eq!(
        recovered,
        INSTALLS - 1,
        "every replaced plan must return to the control side to be dropped there"
    );
}
