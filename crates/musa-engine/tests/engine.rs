//! Engine tests (roadmap §17.5): command-queue round-trips, the seek/loop
//! transport state machine, underrun silence, retired-plan return, and
//! graceful no-device behavior.

// Test helpers use expect() on statically-valid inputs: a failure is a bug in
// the test itself, and panicking is the correct behavior there.
#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
// Frame arithmetic in tests is small and total.
#![allow(clippy::arithmetic_side_effects)]

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use musa_engine::testing::{CallbackCore, Message};
use musa_engine::{AudioEngine, EngineConfig, PreparedPlaybackPlan, TransportCommand};

/// A plan with a real render graph but no events, `total_frames` long.
fn silent_plan(total_frames: u64) -> PreparedPlaybackPlan {
    let options = musa_audio::GraphOptions::default();
    let render_plan = musa_audio::compile_graph(&musa_audio::poly_sine_spec(4), &options).expect("graph");
    PreparedPlaybackPlan::new(render_plan, Vec::new(), total_frames)
}

struct Rig {
    core: CallbackCore,
    commands: rtrb::Producer<Message>,
    retired: rtrb::Consumer<Box<PreparedPlaybackPlan>>,
    position: Arc<AtomicU64>,
    playing: Arc<AtomicBool>,
}

fn rig() -> Rig {
    let (commands, command_consumer) = rtrb::RingBuffer::<Message>::new(16);
    let (retired_producer, retired) = rtrb::RingBuffer::<Box<PreparedPlaybackPlan>>::new(4);
    let position = Arc::new(AtomicU64::new(0));
    let playing = Arc::new(AtomicBool::new(false));
    Rig {
        core: CallbackCore::new(
            command_consumer,
            retired_producer,
            Arc::clone(&position),
            Arc::clone(&playing),
        ),
        commands,
        retired,
        position,
        playing,
    }
}

fn push(rig: &mut Rig, message: Message) {
    rig.commands.push(message).expect("queue has room");
}

#[test]
fn install_then_play_renders_and_advances() {
    let mut rig = rig();
    push(&mut rig, Message::Install(Box::new(silent_plan(1_000_000))));
    push(&mut rig, Message::Transport(TransportCommand::Play));
    let mut output = vec![1.0f32; 256];
    rig.core.process(&mut output);
    assert!(rig.playing.load(Ordering::Relaxed));
    assert_eq!(rig.position.load(Ordering::Relaxed), 128, "one block rendered");
}

#[test]
fn playback_stops_at_the_end_and_zeroes_the_tail() {
    let mut rig = rig();
    push(&mut rig, Message::Install(Box::new(silent_plan(100))));
    push(&mut rig, Message::Transport(TransportCommand::Play));
    let mut output = vec![1.0f32; 512];
    rig.core.process(&mut output);
    assert!(!rig.playing.load(Ordering::Relaxed), "stopped at piece end");
    assert_eq!(rig.position.load(Ordering::Relaxed), 100);
    assert!(
        output.iter().skip(200).all(|s| *s == 0.0),
        "tail after the piece end is silence"
    );
}

#[test]
fn seek_moves_the_transport() {
    let mut rig = rig();
    push(&mut rig, Message::Install(Box::new(silent_plan(1_000_000))));
    push(&mut rig, Message::Transport(TransportCommand::Play));
    push(&mut rig, Message::Transport(TransportCommand::Seek { frame: 5000 }));
    let mut output = vec![0.0f32; 256];
    rig.core.process(&mut output);
    assert_eq!(rig.position.load(Ordering::Relaxed), 5000 + 128);
}

#[test]
fn loop_wraps_within_the_region() {
    let mut rig = rig();
    push(&mut rig, Message::Install(Box::new(silent_plan(1_000_000))));
    push(
        &mut rig,
        Message::Transport(TransportCommand::SetLoop { start: 100, end: 200 }),
    );
    push(&mut rig, Message::Transport(TransportCommand::Seek { frame: 150 }));
    push(&mut rig, Message::Transport(TransportCommand::Play));
    let mut output = vec![0.0f32; 1024];
    rig.core.process(&mut output);
    let position = rig.position.load(Ordering::Relaxed);
    assert!(
        (100..200).contains(&position),
        "position {position} must live inside the loop after wrapping"
    );
    assert!(rig.playing.load(Ordering::Relaxed), "a loop never ends");
}

#[test]
fn underrun_without_a_plan_is_silence() {
    let mut rig = rig();
    let mut output = vec![1.0f32; 64];
    rig.core.process(&mut output);
    assert!(output.iter().all(|s| *s == 0.0));
    push(&mut rig, Message::Install(Box::new(silent_plan(1_000_000))));
    // Installed but not playing: still silence.
    let mut output = vec![1.0f32; 64];
    rig.core.process(&mut output);
    assert!(output.iter().all(|s| *s == 0.0));
}

#[test]
fn replaced_plans_retire_to_the_control_side() {
    let mut rig = rig();
    push(&mut rig, Message::Install(Box::new(silent_plan(1000))));
    let mut output = vec![0.0f32; 8];
    rig.core.process(&mut output); // installs the first plan
    push(&mut rig, Message::Install(Box::new(silent_plan(2000))));
    rig.core.process(&mut output); // installs the second, retires the first
    let retired = rig.retired.pop().expect("a plan was retired");
    assert_eq!(retired.total_frames(), 1000, "the FIRST plan came back");
}

#[test]
fn engine_open_fails_cleanly_or_works() {
    // CI may have no audio device; either a typed EngineError or a working
    // engine is correct — a panic is not.
    match AudioEngine::open(EngineConfig::default()) {
        Ok(engine) => {
            assert!(!engine.is_playing());
            assert_eq!(engine.position(), 0);
        }
        Err(error) => {
            let message = error.to_string();
            assert!(
                message.contains("no audio output device") || message.contains("does not support"),
                "unexpected error: {message}"
            );
        }
    }
}
