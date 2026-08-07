//! The real-time callback core: command consumption, plan installation,
//! transport state, and the render loop — everything that runs inside the
//! CPAL callback, factored so tests can drive it against a fake output
//! without a device (roadmap §17.5).
//!
//! Real-time rules (§13.2): `CallbackCore::process` never allocates, locks,
//! does I/O, logs, or destroys large objects. Retired plans cross to the
//! control thread on an `rtrb` queue and are dropped there; if the queue is
//! full the plan waits in `pending_retire` for the next block.
#![allow(clippy::arithmetic_side_effects)]

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use musa_audio::{EventSlice, RenderPlan};
use musa_compiler::PerformanceEvent;

/// A playback plan prepared entirely on the control side: compiled graph,
/// scheduled events, total duration. The callback only executes it.
pub struct PreparedPlaybackPlan {
    render_plan: RenderPlan,
    events: Vec<PerformanceEvent>,
    /// Total frames including the release tail.
    total_frames: u64,
}

impl PreparedPlaybackPlan {
    /// Bundle a compiled render plan with its scheduled events.
    pub fn new(render_plan: RenderPlan, events: Vec<PerformanceEvent>, total_frames: u64) -> Self {
        Self {
            render_plan,
            events,
            total_frames,
        }
    }

    /// The total duration in frames (tail included).
    pub fn total_frames(&self) -> u64 {
        self.total_frames
    }
}

/// Transport commands cross to the audio thread on the command queue.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransportCommand {
    /// Start (or resume) playback.
    Play,
    /// Stop playback (position kept).
    Stop,
    /// Move to an absolute frame.
    Seek {
        /// The target frame.
        frame: u64,
    },
    /// Loop `[start, end)`.
    SetLoop {
        /// Loop start frame.
        start: u64,
        /// Loop end frame (exclusive).
        end: u64,
    },
    /// Stop looping.
    ClearLoop,
}

/// Control → audio messages.
pub enum Message {
    Install(Box<PreparedPlaybackPlan>),
    Transport(TransportCommand),
}

/// Everything inside the callback: queues, the installed plan, transport.
/// Public only through the doc-hidden testing hook.
pub struct CallbackCore {
    commands: rtrb::Consumer<Message>,
    retired: rtrb::Producer<Box<PreparedPlaybackPlan>>,
    /// A retired plan that did not fit the queue last block; retried each
    /// block (dropping it here would violate §13.2).
    pending_retire: Option<Box<PreparedPlaybackPlan>>,
    installed: Option<Box<PreparedPlaybackPlan>>,
    playing: bool,
    loop_region: Option<(u64, u64)>,
    position: Arc<AtomicU64>,
    playing_flag: Arc<AtomicBool>,
}

impl CallbackCore {
    /// Construct from queue ends (testing hook and engine internals).
    pub fn new(
        commands: rtrb::Consumer<Message>,
        retired: rtrb::Producer<Box<PreparedPlaybackPlan>>,
        position: Arc<AtomicU64>,
        playing_flag: Arc<AtomicBool>,
    ) -> Self {
        Self {
            commands,
            retired,
            pending_retire: None,
            installed: None,
            playing: false,
            loop_region: None,
            position,
            playing_flag,
        }
    }

    /// Retire a plan to the control thread; never drop one here. If the
    /// queue is full the plan waits in `pending_retire` for the next block.
    fn retire(&mut self, plan: Box<PreparedPlaybackPlan>) {
        match self.retired.push(plan) {
            Ok(()) => {}
            Err(rtrb::PushError::Full(plan)) => {
                debug_assert!(self.pending_retire.is_none());
                self.pending_retire = Some(plan);
            }
        }
    }

    /// Apply all pending control messages at a block boundary.
    fn consume_commands(&mut self) {
        if let Some(pending) = self.pending_retire.take() {
            match self.retired.push(pending) {
                Ok(()) => {}
                Err(rtrb::PushError::Full(plan)) => self.pending_retire = Some(plan),
            }
        }
        while let Ok(message) = self.commands.pop() {
            match message {
                Message::Install(plan) => {
                    if let Some(old) = self.installed.replace(plan) {
                        self.retire(old);
                    }
                }
                Message::Transport(command) => self.apply(command),
            }
        }
    }

    fn apply(&mut self, command: TransportCommand) {
        match command {
            TransportCommand::Play => {
                if self.installed.is_some() {
                    self.playing = true;
                    self.playing_flag.store(true, Ordering::Relaxed);
                }
            }
            TransportCommand::Stop => {
                self.playing = false;
                self.playing_flag.store(false, Ordering::Relaxed);
            }
            TransportCommand::Seek { frame } => {
                if let Some(plan) = self.installed.as_mut() {
                    plan.render_plan.seek(frame);
                    self.position.store(frame, Ordering::Relaxed);
                }
            }
            TransportCommand::SetLoop { start, end } => {
                self.loop_region = Some((start, end));
            }
            TransportCommand::ClearLoop => self.loop_region = None,
        }
    }

    /// One callback invocation: fill `output` (interleaved stereo).
    /// Underruns emit silence — never panic, never block (§13.2).
    pub fn process(&mut self, output: &mut [f32]) {
        self.consume_commands();
        let frames = output.len() / 2;
        let Some(installed) = self.installed.as_mut() else {
            output.fill(0.0);
            return;
        };
        if !self.playing {
            output.fill(0.0);
            return;
        }
        let mut done = 0usize;
        while done < frames {
            let cursor = installed.render_plan.cursor();
            // Next hard boundary: loop end or piece end.
            let boundary = self
                .loop_region
                .map_or(installed.total_frames, |(_, end)| end.min(installed.total_frames));
            if cursor >= boundary {
                if let Some((start, _)) = self.loop_region {
                    installed.render_plan.seek(start);
                } else {
                    self.playing = false;
                    self.playing_flag.store(false, Ordering::Relaxed);
                    break;
                }
                continue;
            }
            let count = ((boundary - cursor) as usize).min(frames - done);
            let Some(chunk) = output.get_mut(2 * done..2 * (done + count)) else {
                break;
            };
            installed
                .render_plan
                .render(&EventSlice::new(&installed.events), chunk, count);
            done += count;
        }
        self.position.store(installed.render_plan.cursor(), Ordering::Relaxed);
        // Anything after an early stop is silence.
        if done < frames
            && let Some(rest) = output.get_mut(2 * done..)
        {
            rest.fill(0.0);
        }
    }
}
