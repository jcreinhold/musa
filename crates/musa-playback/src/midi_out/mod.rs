//! Live MIDI output: Musa's checked projection, played to a workstation.
//!
//! This module never decides anything musical. It is handed a schedule that
//! `musa-notation` already decided — the same one the Standard MIDI writer
//! reads — and its whole job is to put those exact bytes on a port at the
//! moment they are due (`docs/rules/across-stages/06-daw-boundary.md` §2).
//!
//! Three pieces, and the split is the point:
//!
//! - [`sender::Sender`] is the run. It is pure: a schedule, a clock reading,
//!   and a cable. Nothing in its loop allocates, and no test of it needs a
//!   device.
//! - [`cable::Cable`] is one platform's ports. macOS publishes `CoreMIDI`
//!   virtual sources or sends to a chosen destination; every other platform
//!   has none, and says so rather than pretending.
//! - [`MidiOutput`] is the facade: it opens the ports, spawns the one thread
//!   that pumps the sender, and reports what it did.
//!
//! Musa's own transport is the clock authority here (§4). Nothing in this
//! module reads a host clock, follows one, or claims alignment with one.

// Microsecond arithmetic over values this module bounds itself.
#![allow(clippy::arithmetic_side_effects)]

mod cable;
#[cfg(target_os = "macos")]
mod coremidi_cable;
mod sender;

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::thread::JoinHandle;

use crate::error::EngineError;
use crate::midi_out::cable::{Cable, Clock, ProcessClock};
use crate::midi_out::sender::{Route, Sender, Totals};

/// How long ahead of the clock one pump hands the cable.
const WINDOW_MICROS: u64 = 4_000;

/// How far past its moment a note may still be attacked.
const TOLERANCE_MICROS: u64 = 20_000;

/// One sounding part, as live output needs to know it.
///
/// The name becomes a port name a workstation shows in its input menu, and
/// the channel is the one the schedule already allocated. Neither is decided
/// here.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LiveMidiPart {
    /// The part's name, as the score wrote it.
    pub name: String,
    /// The channel its notes sound on, zero-based.
    pub channel: u8,
}

/// One channel message, placed in exact microseconds from the run's start.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LiveMidiPacket {
    /// Microseconds from the first pump of the run.
    pub micros: u64,
    /// Which part of the run this belongs to, indexing the parts given to
    /// [`MidiOutput::open`].
    pub part: usize,
    /// Status and up to two data bytes, exactly as the schedule decided them.
    pub bytes: [u8; 3],
    /// How many of `bytes` go on the wire. Notes are three; the transport
    /// stream a leading session sends is one or three.
    pub len: u8,
}

/// How many ports the run publishes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum MidiOutputMode {
    /// One named port a part: a workstation sees each part separately and
    /// assigns its own instrument to each.
    #[default]
    SourcePerPart,
    /// One port carrying every part on its own channel, which is what a
    /// simpler project wants to record onto a single track.
    SingleChannelized,
}

/// Where the run sends.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum MidiOutputTarget {
    /// Publish virtual sources of Musa's own, which a workstation receives
    /// from. Apple's term for what we publish is a *source*.
    #[default]
    VirtualSources,
    /// Send to a destination the host already offers, named by the stable id
    /// [`MidiOutput::endpoints`] reported.
    Destination(String),
}

/// One MIDI destination the host offers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MidiEndpoint {
    /// Stable platform identifier.
    pub id: String,
    /// Human-readable name, as the host states it.
    pub name: String,
}

/// What a run needs before it opens.
#[derive(Clone, Debug)]
pub struct MidiOutputConfig {
    /// How many ports to publish.
    pub mode: MidiOutputMode,
    /// Where to send.
    pub target: MidiOutputTarget,
    /// What the published ports are called, before the part name is added.
    pub client: String,
    /// Publish one further port carrying the transport stream, for a session
    /// where Musa is the clock authority.
    ///
    /// It is a port of its own so a workstation can be told which input to
    /// take its clock from without also taking notes from it, and so the
    /// notes' ports stay exactly what prompt 213 published.
    pub clock: bool,
}

impl Default for MidiOutputConfig {
    fn default() -> Self {
        Self {
            mode: MidiOutputMode::default(),
            target: MidiOutputTarget::default(),
            client: "Musa".to_owned(),
            clock: false,
        }
    }
}

/// One published port and what it carries.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MidiPortReport {
    /// Index of the port within the run.
    pub port: usize,
    /// The name a workstation shows.
    pub name: String,
    /// The parts on it, in schedule order.
    pub parts: Vec<String>,
}

/// What one open run is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MidiOutputReport {
    /// How many ports were published.
    pub mode: MidiOutputMode,
    /// How near its moment a message lands, in microseconds.
    ///
    /// `CoreMIDI` accepts a future host timestamp, and reading the host clock
    /// it counts in is FFI, which this workspace does not allow itself. So
    /// each message is handed to its port as it comes due, which places it
    /// within one scheduling window of its moment rather than exactly on it.
    /// `06-daw-boundary.md` §6 asks every loss to be refused or recorded;
    /// this is a recorded one, and every report carries it.
    pub window_micros: u64,
    /// Where the run sends, in words a person can read back.
    pub target: String,
    /// Each published port.
    pub ports: Vec<MidiPortReport>,
}

/// What a run has done so far.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MidiOutputCounters {
    /// Messages the cable accepted.
    pub sent: u64,
    /// Messages sent after their moment had already passed.
    pub late: u64,
    /// Attacks dropped because their moment was long past, plus any packet
    /// naming a part the run never routed.
    pub dropped: u64,
    /// Messages a port refused.
    pub refused: u64,
}

/// A live MIDI projection of one schedule.
///
/// Opening publishes the ports; [`Self::start`] begins one run; [`Self::stop`]
/// ends it with the panic sequence. Dropping does the same as stopping, so a
/// run cannot outlive its owner with notes held down.
pub struct MidiOutput {
    report: MidiOutputReport,
    routes: Vec<Route>,
    ports: usize,
    running: Arc<Running>,
    thread: Option<JoinHandle<(Box<dyn Cable>, Box<dyn Clock>)>>,
    cable: Option<Box<dyn Cable>>,
    clock: Option<Box<dyn Clock>>,
}

/// The little the control side and the send thread share.
#[derive(Default)]
struct Running {
    stop: AtomicBool,
    sent: AtomicU64,
    late: AtomicU64,
    dropped: AtomicU64,
    refused: AtomicU64,
}

impl Running {
    fn record(&self, totals: Totals) {
        self.sent.store(totals.sent, Ordering::Relaxed);
        self.late.store(totals.late, Ordering::Relaxed);
        self.dropped.store(totals.dropped, Ordering::Relaxed);
        self.refused.store(totals.refused, Ordering::Relaxed);
    }

    fn counters(&self) -> MidiOutputCounters {
        MidiOutputCounters {
            sent: self.sent.load(Ordering::Relaxed),
            late: self.late.load(Ordering::Relaxed),
            dropped: self.dropped.load(Ordering::Relaxed),
            refused: self.refused.load(Ordering::Relaxed),
        }
    }
}

impl MidiOutput {
    /// Every MIDI destination the host offers, without retaining a platform
    /// type for any of them.
    pub fn endpoints() -> Vec<MidiEndpoint> {
        #[cfg(target_os = "macos")]
        {
            coremidi_cable::endpoints()
        }
        #[cfg(not(target_os = "macos"))]
        {
            Vec::new()
        }
    }

    /// What opening would publish, without publishing it.
    ///
    /// A machine with no workstation attached — a build server, a Linux
    /// desktop — can still be told which ports a run would name and how
    /// exactly this platform would place a message in time. Nothing here
    /// touches the host.
    pub fn preview(config: &MidiOutputConfig, parts: &[LiveMidiPart]) -> MidiOutputReport {
        PortPlan::of(config, parts).describe(config)
    }

    /// Publish the ports one run of `parts` needs.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::NoMidiOutput`] where the platform has no MIDI
    /// output at all, [`EngineError::UnknownMidiDestination`] when a named
    /// destination is not connected, and [`EngineError::MidiOutput`] when the
    /// host refused to publish.
    pub fn open(config: &MidiOutputConfig, parts: &[LiveMidiPart]) -> Result<Self, EngineError> {
        let plan = PortPlan::of(config, parts);
        let cable = open_cable(config, &plan)?;
        Ok(Self::with_backend(config, plan, cable, Box::new(ProcessClock::new())))
    }

    fn with_backend(config: &MidiOutputConfig, plan: PortPlan, cable: Box<dyn Cable>, clock: Box<dyn Clock>) -> Self {
        let ports = plan.names.len();
        let report = plan.describe(config);
        Self {
            report,
            routes: plan.routes,
            ports,
            running: Arc::new(Running::default()),
            thread: None,
            cable: Some(cable),
            clock: Some(clock),
        }
    }

    /// Which `part` index a transport packet carries.
    ///
    /// The transport stream is not a part and has no channel, but it travels
    /// in the same packet list as the notes so that one send loop walks one
    /// schedule. It routes one past the last sounding part, which is the
    /// clock port.
    #[must_use]
    pub const fn clock_part(parts: &[LiveMidiPart]) -> usize {
        parts.len()
    }

    /// What this run published.
    pub const fn report(&self) -> &MidiOutputReport {
        &self.report
    }

    /// What this run has done so far.
    pub fn counters(&self) -> MidiOutputCounters {
        self.running.counters()
    }

    /// Whether a run is still in progress.
    pub fn is_running(&self) -> bool {
        self.thread.as_ref().is_some_and(|thread| !thread.is_finished())
    }

    /// Begin one run. Every buffer it will use is allocated here, on the
    /// control side, before the send loop starts.
    ///
    /// A finished or stopped run gives its ports back, so the same output
    /// plays the next plan without republishing anything.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::MidiOutput`] when a run is already in progress:
    /// replacing a plan mid-flight is [`Self::stop`] and then `start`, so the
    /// notes of the old plan are released rather than left holding.
    pub fn start(&mut self, packets: Vec<LiveMidiPacket>) -> Result<(), EngineError> {
        if self.is_running() {
            return Err(EngineError::MidiOutput(
                "a MIDI output run is already in progress".to_owned(),
            ));
        }
        self.join();
        let (Some(mut cable), Some(clock)) = (self.cable.take(), self.clock.take()) else {
            return Err(EngineError::MidiOutput(
                "this MIDI output was already closed".to_owned(),
            ));
        };
        let mut sender = Sender::new(
            packets,
            self.routes.clone(),
            self.ports,
            WINDOW_MICROS,
            TOLERANCE_MICROS,
        );
        let running = Arc::clone(&self.running);
        running.stop.store(false, Ordering::Relaxed);
        self.thread = Some(std::thread::spawn(move || {
            let rest = std::time::Duration::from_micros(WINDOW_MICROS / 2);
            loop {
                if running.stop.load(Ordering::Relaxed) {
                    sender.panic(cable.as_mut(), clock.now());
                    break;
                }
                sender.pump(cable.as_mut(), clock.now());
                running.record(sender.totals());
                if sender.is_drained() {
                    // A schedule that ends with a note still down is a
                    // schedule we refuse to walk away from.
                    if !sender.is_finished() {
                        sender.panic(cable.as_mut(), clock.now());
                    }
                    break;
                }
                std::thread::sleep(rest);
            }
            running.record(sender.totals());
            // The ports and the clock go back to the control side, so the
            // same output can play a second plan without republishing.
            (cable, clock)
        }));
        Ok(())
    }

    /// End the run, releasing every note it left sounding.
    pub fn stop(&mut self) {
        self.running.stop.store(true, Ordering::Relaxed);
        self.join();
    }

    fn join(&mut self) {
        if let Some(thread) = self.thread.take()
            && let Ok((cable, clock)) = thread.join()
        {
            self.cable = Some(cable);
            self.clock = Some(clock);
        }
        self.running.stop.store(false, Ordering::Relaxed);
    }
}

impl Drop for MidiOutput {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Publish the ports, where this platform has any.
#[cfg(target_os = "macos")]
fn open_cable(config: &MidiOutputConfig, plan: &PortPlan) -> Result<Box<dyn Cable>, EngineError> {
    coremidi_cable::open(config, &plan.names)
}

/// Nothing but macOS publishes MIDI, and saying so is better than a silent
/// run that reaches nothing.
#[cfg(not(target_os = "macos"))]
fn open_cable(_config: &MidiOutputConfig, _plan: &PortPlan) -> Result<Box<dyn Cable>, EngineError> {
    Err(EngineError::NoMidiOutput)
}

/// Where a run sends, in words a person can read back.
fn describe(target: &MidiOutputTarget) -> String {
    match target {
        MidiOutputTarget::VirtualSources => "virtual sources".to_owned(),
        MidiOutputTarget::Destination(id) => format!("destination {id}"),
    }
}

/// Which ports a run publishes, and which one each part sounds on.
struct PortPlan {
    names: Vec<String>,
    routes: Vec<Route>,
    report: Vec<MidiPortReport>,
}

impl PortPlan {
    /// What this plan is, as a caller reads it.
    fn describe(&self, config: &MidiOutputConfig) -> MidiOutputReport {
        MidiOutputReport {
            mode: config.mode,
            window_micros: WINDOW_MICROS,
            target: describe(&config.target),
            ports: self.report.clone(),
        }
    }

    /// Append the transport port, where the session leads.
    ///
    /// It is always last, so a part's port index is the one prompt 213 gave
    /// it whether or not a clock is running.
    fn with_clock(mut self, config: &MidiOutputConfig, parts: &[LiveMidiPart]) -> Self {
        if !config.clock {
            return self;
        }
        let port = self.names.len();
        let name = format!("{} · clock", config.client);
        debug_assert_eq!(self.routes.len(), MidiOutput::clock_part(parts));
        self.routes.push(Route { port, channel: 0 });
        self.report.push(MidiPortReport {
            port,
            name: name.clone(),
            parts: Vec::new(),
        });
        self.names.push(name);
        self
    }

    fn of(config: &MidiOutputConfig, parts: &[LiveMidiPart]) -> Self {
        Self::sounding(config, parts).with_clock(config, parts)
    }

    fn sounding(config: &MidiOutputConfig, parts: &[LiveMidiPart]) -> Self {
        let client = config.client.as_str();
        match config.mode {
            MidiOutputMode::SourcePerPart => {
                let names = parts
                    .iter()
                    .map(|part| format!("{client} · {}", part.name))
                    .collect::<Vec<_>>();
                Self {
                    routes: parts
                        .iter()
                        .enumerate()
                        .map(|(port, part)| Route {
                            port,
                            channel: part.channel,
                        })
                        .collect(),
                    report: names
                        .iter()
                        .zip(parts)
                        .enumerate()
                        .map(|(port, (name, part))| MidiPortReport {
                            port,
                            name: name.clone(),
                            parts: vec![part.name.clone()],
                        })
                        .collect(),
                    names,
                }
            }
            MidiOutputMode::SingleChannelized => {
                let name = client.to_owned();
                Self {
                    routes: parts
                        .iter()
                        .map(|part| Route {
                            port: 0,
                            channel: part.channel,
                        })
                        .collect(),
                    report: vec![MidiPortReport {
                        port: 0,
                        name: name.clone(),
                        parts: parts.iter().map(|part| part.name.clone()).collect(),
                    }],
                    names: vec![name],
                }
            }
        }
    }
}

#[doc(hidden)]
pub(crate) mod testing {
    //! A whole run with no device and no thread in it.
    //!
    //! The laws that hold live output to the Standard MIDI file drive this,
    //! not [`super::MidiOutput`]: a fake clock the test advances by hand and a
    //! cable that keeps what it was given make the run's every decision
    //! observable and repeatable, on any platform.

    use super::cable::{Batch, Cable, Refused};
    use super::sender::Sender;
    use super::{LiveMidiPacket, LiveMidiPart, MidiOutputConfig, MidiOutputCounters, MidiPortReport, PortPlan};

    /// A cable that keeps every batch it is handed.
    #[derive(Default)]
    struct FakeCable {
        /// Every message accepted, as `(port, timestamp, bytes, length)`.
        received: Vec<(usize, u64, [u8; 3], u8)>,
        /// Ports that refuse everything, standing in for a vanished endpoint.
        broken: Vec<usize>,
    }

    impl Cable for FakeCable {
        fn send(&mut self, port: usize, batch: &Batch) -> Result<(), Refused> {
            if self.broken.contains(&port) {
                return Err(Refused);
            }
            self.received
                .extend(batch.iter().map(|(at, wire)| (port, *at, wire.bytes, wire.len)));
            Ok(())
        }
    }

    /// One run, driven by hand.
    pub struct MidiOutputHarness {
        sender: Sender,
        cable: FakeCable,
        ports: Vec<MidiPortReport>,
    }

    impl MidiOutputHarness {
        /// Prepare the same run [`super::MidiOutput::start`] would, against a
        /// fake cable.
        #[must_use]
        pub fn new(
            config: &MidiOutputConfig,
            parts: &[LiveMidiPart],
            packets: Vec<LiveMidiPacket>,
            window: u64,
            tolerance: u64,
        ) -> Self {
            let plan = PortPlan::of(config, parts);
            let ports = plan.names.len();
            Self {
                sender: Sender::new(packets, plan.routes, ports, window, tolerance),
                cable: FakeCable::default(),
                ports: plan.report,
            }
        }

        /// What opening would publish.
        #[must_use]
        pub fn ports(&self) -> &[MidiPortReport] {
            &self.ports
        }

        /// Give the recording cable room for `messages` before the measured
        /// loop starts, so an allocation law measures the loop and not the
        /// bookkeeping standing in for a device.
        pub fn reserve(&mut self, messages: usize) {
            self.cable.received.reserve(messages);
        }

        /// Make one port refuse everything from now on.
        pub fn break_port(&mut self, port: usize) {
            self.cable.broken.push(port);
        }

        /// Run one scheduling window, as if the clock read `now`.
        pub fn pump(&mut self, now: u64) {
            self.sender.pump(&mut self.cable, now);
        }

        /// Run windows until the schedule is drained, one window apart.
        pub fn run(&mut self, window: u64) {
            let mut now = 0;
            while !self.sender.is_drained() {
                self.pump(now);
                now += window;
            }
        }

        /// Stop the run as [`super::MidiOutput::stop`] would.
        pub fn panic_at(&mut self, now: u64) {
            self.sender.panic(&mut self.cable, now);
        }

        /// Every message the cable accepted.
        #[must_use]
        pub fn received(&self) -> &[(usize, u64, [u8; 3], u8)] {
            &self.cable.received
        }

        /// What the run has done so far.
        #[must_use]
        pub fn counters(&self) -> MidiOutputCounters {
            self.sender.totals().into()
        }
    }
}
