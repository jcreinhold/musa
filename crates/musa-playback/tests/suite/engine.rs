//! Engine tests: command-queue round-trips, the seek/loop
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

use super::support::{silent_plan, source_plan};
use musa_playback::testing::{CallbackCore, Message};
use musa_playback::{AudioEngine, EngineConfig, EngineError, PreparedPlaybackPlan, TransportCommand};

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

fn isolated_source(first_part: &str) -> String {
    format!(
        "piece \"live routing\" {{ tempo 1/4 = 60; meter 4/4; score {{ \
         part a {{ voice v {{ {first_part} }} }} part b {{ voice v {{ a4/1 }} }} }} \
         studio {{ patch shared {{ oscillator(sine) |> gain(-24 dB) |> output; }} \
         assign a -> shared; assign b -> shared; route b -> master; }} }}"
    )
}

fn callback_block(source: &str) -> Vec<f32> {
    let mut rig = rig();
    push(&mut rig, Message::Install(Box::new(source_plan(source, 0))));
    push(&mut rig, Message::Transport(TransportCommand::Play));
    let mut output = vec![0.0; 512];
    rig.core.process(&mut output);
    output
}

#[test]
fn live_plan_delivers_each_part_only_to_its_instance() {
    let crowded = isolated_source("[c2 d2 e2 f2 g2 a2 b2 c3 d3 e3 f3 g3 a3 b3 c4 d4 e4]/1");
    let silent = isolated_source("rest/1");
    assert_eq!(callback_block(&crowded), callback_block(&silent));
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

/// Every `EngineError` a caller can be handed says what went wrong without
/// naming CPAL.
///
/// The fast half of the contract below: an error crossing this crate's
/// boundary is one of four typed things, and each renders a sentence a person
/// can act on. No device is touched, so it runs in microseconds and holds on
/// a headless machine.
#[test]
fn every_engine_error_says_what_went_wrong() {
    for (error, expected) in [
        (EngineError::NoOutputDevice, "no audio output device available"),
        (
            EngineError::UnsupportedStreamConfig { rate: 44_100 },
            "the output device does not support 44100 Hz stereo f32 output",
        ),
        (
            EngineError::Stream("the device went away".to_owned()),
            "audio stream failure: the device went away",
        ),
        (EngineError::QueueFull, "engine command queue is full"),
    ] {
        assert_eq!(error.to_string(), expected);
    }
}

/// Opening the engine either works or fails with a typed error — never a
/// panic, and never a CPAL type reaching the caller.
///
/// **Ignored because it is slow, and slow for a reason no code here owns:**
/// `AudioEngine::open` asks `CoreAudio` for a real output device, and on macOS
/// that call takes upwards of fifteen seconds on a cold audio subsystem. It
/// is the only test in the workspace that touches hardware, and leaving it in
/// the default suite put a sixteen-second floor under every routine run of
/// twelve hundred tests that otherwise finish in three.
///
/// What still holds in the fast suite: the six `EngineCore` tests above run
/// the whole real-time contract — install, play, seek, loop, underrun, plan
/// retirement — against the callback directly, with no device;
/// `every_engine_error_says_what_went_wrong` holds the typed-error half of
/// *this* test in microseconds.
///
/// What is deferred: that the device-opening path is reachable at all, and
/// that a machine which *has* a device gets a stopped engine at position
/// zero. That is worth having on demand and is not worth sixteen seconds of
/// every run.
#[test]
#[ignore = "slow: opens a real audio device; run with --run-ignored all"]
fn slow_engine_open_fails_cleanly_or_works() {
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
