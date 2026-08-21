//! The real-time callback core: command consumption, plan installation,
//! transport state, and the render loop — everything that runs inside the
//! CPAL callback, factored so tests can drive it against a fake output
//! without a device.
//!
//! Real-time rules: `CallbackCore::process` never allocates, locks,
//! does I/O, logs, or destroys large objects. Retired plans cross to the
//! control thread on an `rtrb` queue and are dropped there; if the queue is
//! full the plan waits in `pending_retire` and the core stops consuming
//! commands until it drains, so a plan is never destroyed here.
//!
//! Arithmetic in this module is per-block, never per-sample, so the checked
//! and saturating forms below cost nothing measurable and remove every
//! overflow and underflow path from the audio thread.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use musa_audio::{EventSlice, RenderPlan};
use musa_score::PerformanceEvent;

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
    /// block (dropping it here would violate the real-time rules). While it is occupied
    /// the core stops consuming commands, so at most one plan is ever held
    /// back and none is ever destroyed on the audio thread.
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
        debug_assert!(
            self.pending_retire.is_none(),
            "retire is only reached with room to hold back"
        );
        match self.retired.push(plan) {
            Ok(()) => {}
            Err(rtrb::PushError::Full(plan)) => self.pending_retire = Some(plan),
        }
    }

    /// Apply pending control messages at a block boundary.
    ///
    /// Consumption stops while a plan is held back, which is what makes the
    /// no-drop guarantee structural rather than a hope: a command that would
    /// retire a second plan simply stays queued until the control thread
    /// drains the retirement queue. Transport commands behind it are delayed
    /// by a block or two, which is inaudible; destroying a `RenderPlan` in
    /// the callback would not be.
    fn consume_commands(&mut self) {
        if let Some(pending) = self.pending_retire.take() {
            match self.retired.push(pending) {
                Ok(()) => {}
                Err(rtrb::PushError::Full(plan)) => {
                    self.pending_retire = Some(plan);
                    return;
                }
            }
        }
        while self.pending_retire.is_none() {
            let Ok(message) = self.commands.pop() else { break };
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
            // A region whose end is not strictly after its start describes no
            // frames at all. Honouring it would seek back to `start` forever
            // without filling a sample, wedging the audio thread, so it is
            // rejected here — the one place that can still refuse it cheaply.
            TransportCommand::SetLoop { start, end } => {
                if start < end {
                    self.loop_region = Some((start, end));
                }
            }
            TransportCommand::ClearLoop => self.loop_region = None,
        }
    }

    /// One callback invocation: fill `output` (interleaved stereo).
    /// Underruns emit silence — never panic, never block.
    pub fn process(&mut self, output: &mut [f32]) {
        self.consume_commands();
        let frames = output.len().saturating_div(2);
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
                // Wrap only to a start that lies before the boundary;
                // otherwise the region is unplayable against this plan and
                // wrapping would make no progress. Stop instead.
                match self.loop_region {
                    Some((start, _)) if start < boundary => installed.render_plan.seek(start),
                    Some(_) | None => {
                        self.playing = false;
                        self.playing_flag.store(false, Ordering::Relaxed);
                        break;
                    }
                }
                continue;
            }
            // `cursor < boundary` and `done < frames`, so `count >= 1`: every
            // iteration that reaches here advances, which is what bounds the
            // loop at `frames` iterations.
            let remaining = boundary.saturating_sub(cursor);
            let count = usize::try_from(remaining)
                .unwrap_or(usize::MAX)
                .min(frames.saturating_sub(done));
            let Some(chunk) = output.get_mut(done.saturating_mul(2)..done.saturating_add(count).saturating_mul(2))
            else {
                break;
            };
            installed
                .render_plan
                .render(&EventSlice::new(&installed.events), chunk, count);
            done = done.saturating_add(count);
        }
        self.position.store(installed.render_plan.cursor(), Ordering::Relaxed);
        // Anything after an early stop is silence.
        if done < frames
            && let Some(rest) = output.get_mut(done.saturating_mul(2)..)
        {
            rest.fill(0.0);
        }
    }
}
