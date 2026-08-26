//! Bounded expressive MIDI input and callback-clock calibration.
//!
//! `midir` invokes a real-time callback. That callback reads a fixed channel
//! message, stamps it against a connection-local monotonic clock, and pushes a
//! [`Copy`] fact into one preallocated ring. Pairing, calibration, capture,
//! device refresh, and every musical decision remain on the control side.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

const INPUT_CAPACITY: usize = 2_048;
const CLIENT: &str = "musa";
const CLOCK_JUMP_MICROS: u64 = 1_000_000;

/// Stable platform identity and display name for one MIDI input.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MidiInputDevice {
    /// Opaque stable identifier supplied by the locked backend.
    pub id: String,
    /// Human-readable platform port name.
    pub name: String,
}

/// Fixed channel-message class retained by capture.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MidiMessageKind {
    /// Note attack; `data` is key and `value` is velocity.
    NoteOn,
    /// Key release; `data` is key and `value` is release velocity.
    NoteOff,
    /// Controller change; `data` is controller number.
    ControlChange,
    /// Fourteen-bit signed bend, centered at zero.
    PitchBend,
    /// Channel pressure.
    ChannelPressure,
    /// Polyphonic pressure; `data` is key.
    KeyPressure,
    /// Program change retained as unsupported audition evidence.
    ProgramChange,
}

/// One immutable fixed-layout callback event.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MidiInputEvent {
    /// Backend timestamp in microseconds from its unspecified stable epoch.
    pub device_micros: u64,
    /// Callback arrival in microseconds since this connection opened.
    pub callback_micros: u64,
    /// MIDI cable, zero for MIDI 1.0 backends that expose no cable identity.
    pub cable: u8,
    /// Zero-based MIDI channel.
    pub channel: u8,
    /// Channel-message class.
    pub kind: MidiMessageKind,
    /// Key, controller, or program according to `kind`.
    pub data: u8,
    /// Velocity, controller/pressure value, or signed pitch bend.
    pub value: i16,
}

/// Callback facts that could not enter the bounded event ring.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MidiInputLosses {
    /// Valid fixed events dropped because the queue was full.
    pub queue_overflow: u64,
    /// `SysEx` messages deliberately refused as unbounded payloads.
    pub refused_sysex: u64,
    /// Truncated or invalid channel messages.
    pub malformed: u64,
    /// System messages outside this channel-performance boundary.
    pub unsupported_system: u64,
}

#[derive(Default)]
struct LossCounters {
    queue_overflow: AtomicU64,
    refused_sysex: AtomicU64,
    malformed: AtomicU64,
    unsupported_system: AtomicU64,
}

impl LossCounters {
    fn snapshot(&self) -> MidiInputLosses {
        MidiInputLosses {
            queue_overflow: self.queue_overflow.load(Ordering::Relaxed),
            refused_sysex: self.refused_sysex.load(Ordering::Relaxed),
            malformed: self.malformed.load(Ordering::Relaxed),
            unsupported_system: self.unsupported_system.load(Ordering::Relaxed),
        }
    }
}

/// A connection to one explicitly identified MIDI input, or to none.
pub struct MidiInput {
    _connection: Option<midir::MidiInputConnection<()>>,
    events: rtrb::Consumer<MidiInputEvent>,
    device: Option<MidiInputDevice>,
    losses: Arc<LossCounters>,
}

impl MidiInput {
    /// Enumerate inputs without retaining backend types.
    pub fn devices() -> Vec<MidiInputDevice> {
        let Ok(input) = midir::MidiInput::new(CLIENT) else {
            return Vec::new();
        };
        input
            .ports()
            .into_iter()
            .filter_map(|port| {
                input
                    .port_name(&port)
                    .ok()
                    .map(|name| MidiInputDevice { id: port.id(), name })
            })
            .collect()
    }

    /// Connect by stable id. With no preference, connect only when exactly
    /// one input exists; several devices require an explicit choice.
    pub fn open(preferred_id: Option<&str>) -> Self {
        let (mut producer, events) = rtrb::RingBuffer::<MidiInputEvent>::new(INPUT_CAPACITY);
        let losses = Arc::new(LossCounters::default());
        let Some((mut input, port, device)) = choose(preferred_id) else {
            return Self {
                _connection: None,
                events,
                device: None,
                losses,
            };
        };
        input.ignore(midir::Ignore::SysexAndTime | midir::Ignore::ActiveSense);
        let callback_losses = Arc::clone(&losses);
        let opened = Instant::now();
        let connection = input.connect(
            &port,
            CLIENT,
            move |device_micros, message, ()| {
                let callback_micros = u64::try_from(opened.elapsed().as_micros()).unwrap_or(u64::MAX);
                receive_message(&mut producer, &callback_losses, device_micros, callback_micros, message);
            },
            (),
        );
        match connection {
            Ok(connection) => Self {
                _connection: Some(connection),
                events,
                device: Some(device),
                losses,
            },
            Err(error) => {
                tracing::warn!(error = %error.kind(), "could not open the MIDI input port");
                Self {
                    _connection: None,
                    events,
                    device: None,
                    losses,
                }
            }
        }
    }

    /// Connected device, if its port opened successfully.
    pub const fn device(&self) -> Option<&MidiInputDevice> {
        self.device.as_ref()
    }

    /// Backward-compatible display name for the connected port.
    pub fn port(&self) -> Option<&str> {
        self.device.as_ref().map(|device| device.name.as_str())
    }

    /// Whether the selected stable identity is still enumerated.
    pub fn is_present(&self) -> bool {
        self.device
            .as_ref()
            .is_some_and(|selected| Self::devices().iter().any(|device| device.id == selected.id))
    }

    /// Next immutable callback fact, without blocking.
    pub fn poll(&mut self) -> Option<MidiInputEvent> {
        self.events.pop().ok()
    }

    /// Cumulative bounded-loss counters.
    pub fn losses(&self) -> MidiInputLosses {
        self.losses.snapshot()
    }
}

fn receive_message(
    producer: &mut rtrb::Producer<MidiInputEvent>,
    losses: &LossCounters,
    device_micros: u64,
    callback_micros: u64,
    message: &[u8],
) {
    match decode(device_micros, callback_micros, message) {
        Decode::Event(event) => push_event(producer, losses, event),
        Decode::SysEx => {
            losses.refused_sysex.fetch_add(1, Ordering::Relaxed);
        }
        Decode::Malformed => {
            losses.malformed.fetch_add(1, Ordering::Relaxed);
        }
        Decode::System => {
            losses.unsupported_system.fetch_add(1, Ordering::Relaxed);
        }
    }
}

/// Device-free probe of the exact MIDI callback decode-and-enqueue path.
///
/// This is a hidden verification hook, not application API. Construction is
/// control-side; [`Self::receive`] is the operation subject to the callback
/// real-time contract.
#[doc(hidden)]
pub struct MidiCallbackHarness {
    producer: rtrb::Producer<MidiInputEvent>,
    consumer: rtrb::Consumer<MidiInputEvent>,
    losses: LossCounters,
}

impl MidiCallbackHarness {
    /// Allocate the bounded ring before entering the measured callback path.
    #[must_use]
    pub fn new() -> Self {
        let (producer, consumer) = rtrb::RingBuffer::new(INPUT_CAPACITY);
        Self {
            producer,
            consumer,
            losses: LossCounters::default(),
        }
    }

    /// Decode and enqueue one backend callback message.
    pub fn receive(&mut self, device_micros: u64, callback_micros: u64, message: &[u8]) {
        receive_message(
            &mut self.producer,
            &self.losses,
            device_micros,
            callback_micros,
            message,
        );
    }

    /// Drain one event on the simulated control side.
    pub fn poll(&mut self) -> Option<MidiInputEvent> {
        self.consumer.pop().ok()
    }

    /// Current callback loss facts.
    pub fn losses(&self) -> MidiInputLosses {
        self.losses.snapshot()
    }
}

impl Default for MidiCallbackHarness {
    fn default() -> Self {
        Self::new()
    }
}

fn choose(preferred_id: Option<&str>) -> Option<(midir::MidiInput, midir::MidiInputPort, MidiInputDevice)> {
    let input = midir::MidiInput::new(CLIENT).ok()?;
    let ports = input.ports();
    let ids = ports.iter().map(midir::MidiInputPort::id).collect::<Vec<_>>();
    let index = selection_index(&ids, preferred_id)?;
    let port = ports.get(index)?.clone();
    let device = MidiInputDevice {
        id: port.id(),
        name: input.port_name(&port).ok()?,
    };
    Some((input, port, device))
}

fn selection_index(ids: &[String], preferred_id: Option<&str>) -> Option<usize> {
    match preferred_id {
        Some(id) => ids.iter().position(|available| available == id),
        None if ids.len() == 1 => Some(0),
        None => None,
    }
}

fn push_event(producer: &mut rtrb::Producer<MidiInputEvent>, losses: &LossCounters, event: MidiInputEvent) {
    if producer.push(event).is_err() {
        losses.queue_overflow.fetch_add(1, Ordering::Relaxed);
    }
}

enum Decode {
    Event(MidiInputEvent),
    SysEx,
    Malformed,
    System,
}

fn decode(device_micros: u64, callback_micros: u64, message: &[u8]) -> Decode {
    let Some(&status) = message.first() else {
        return Decode::Malformed;
    };
    if status == 0xF0 || status == 0xF7 {
        return Decode::SysEx;
    }
    if !(0x80..0xF0).contains(&status) {
        return Decode::System;
    }
    let channel = status & 0x0F;
    let common = |kind, data, value| {
        Decode::Event(MidiInputEvent {
            device_micros,
            callback_micros,
            cable: 0,
            channel,
            kind,
            data,
            value,
        })
    };
    match status & 0xF0 {
        0x80 => match message {
            [_, key @ 0..=127, velocity @ 0..=127] => common(MidiMessageKind::NoteOff, *key, i16::from(*velocity)),
            _ => Decode::Malformed,
        },
        0x90 => match message {
            [_, key @ 0..=127, 0] => common(MidiMessageKind::NoteOff, *key, 0),
            [_, key @ 0..=127, velocity @ 1..=127] => common(MidiMessageKind::NoteOn, *key, i16::from(*velocity)),
            _ => Decode::Malformed,
        },
        0xA0 => match message {
            [_, key @ 0..=127, pressure @ 0..=127] => common(MidiMessageKind::KeyPressure, *key, i16::from(*pressure)),
            _ => Decode::Malformed,
        },
        0xB0 => match message {
            [_, controller @ 0..=127, value @ 0..=127] => {
                common(MidiMessageKind::ControlChange, *controller, i16::from(*value))
            }
            _ => Decode::Malformed,
        },
        0xC0 => match message {
            [_, program @ 0..=127] => common(MidiMessageKind::ProgramChange, *program, 0),
            _ => Decode::Malformed,
        },
        0xD0 => match message {
            [_, pressure @ 0..=127] => common(MidiMessageKind::ChannelPressure, 0, i16::from(*pressure)),
            _ => Decode::Malformed,
        },
        0xE0 => match message {
            [_, low @ 0..=127, high @ 0..=127] => {
                let unsigned = i16::from(*low) | (i16::from(*high) << 7);
                common(MidiMessageKind::PitchBend, 0, unsigned.saturating_sub(8_192))
            }
            _ => Decode::Malformed,
        },
        _ => Decode::System,
    }
}

/// Quality of the current device-to-callback clock fit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MidiClockQuality {
    /// Fewer than two observations.
    Uncalibrated,
    /// A bounded affine fit is available.
    Calibrated,
    /// A discontinuity started a new fit segment.
    Discontinuous,
}

/// Immutable summary of one affine clock-fit segment.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MidiClockCalibration {
    /// Device-to-callback scale.
    pub scale: f64,
    /// Callback-microsecond intercept.
    pub offset_micros: f64,
    /// Observations in this segment.
    pub samples: u32,
    /// Greatest absolute residual observed in microseconds.
    pub max_residual_micros: f64,
    /// Discontinuities observed since connection.
    pub discontinuities: u32,
    /// Current fit quality.
    pub quality: MidiClockQuality,
}

/// One event paired with its calibrated connection-local time.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CalibratedMidiEvent {
    /// Immutable raw callback fact.
    pub raw: MidiInputEvent,
    /// Calibrated microseconds since this MIDI connection opened.
    pub calibrated_micros: u64,
    /// Fit summary after admitting this observation.
    pub calibration: MidiClockCalibration,
}

/// Bounded online affine calibration of device time to callback time.
#[derive(Clone, Debug, Default)]
pub struct MidiClockCalibrator {
    origin_device: u64,
    origin_callback: u64,
    last_device: u64,
    last_callback: u64,
    samples: u32,
    sum_x: f64,
    sum_y: f64,
    sum_xx: f64,
    sum_xy: f64,
    scale: f64,
    offset: f64,
    max_residual: f64,
    discontinuities: u32,
    discontinuous: bool,
}

impl MidiClockCalibrator {
    /// Admit one event and map its immutable device timestamp.
    pub fn observe(&mut self, event: MidiInputEvent) -> CalibratedMidiEvent {
        let jumped = self.samples > 0
            && (event.device_micros <= self.last_device
                || event
                    .device_micros
                    .abs_diff(self.last_device)
                    .abs_diff(event.callback_micros.abs_diff(self.last_callback))
                    > CLOCK_JUMP_MICROS);
        if self.samples == 0 || jumped {
            if jumped {
                self.discontinuities = self.discontinuities.saturating_add(1);
            }
            self.origin_device = event.device_micros;
            self.origin_callback = event.callback_micros;
            self.samples = 0;
            self.sum_x = 0.0;
            self.sum_y = 0.0;
            self.sum_xx = 0.0;
            self.sum_xy = 0.0;
            self.scale = 1.0;
            self.offset = 0.0;
            self.max_residual = 0.0;
            self.discontinuous = jumped;
        }
        let x = event.device_micros.saturating_sub(self.origin_device) as f64;
        let y = event.callback_micros.saturating_sub(self.origin_callback) as f64;
        self.samples = self.samples.saturating_add(1);
        self.sum_x += x;
        self.sum_y += y;
        self.sum_xx = x.mul_add(x, self.sum_xx);
        self.sum_xy = x.mul_add(y, self.sum_xy);
        let count = f64::from(self.samples);
        let denominator = count.mul_add(self.sum_xx, -(self.sum_x * self.sum_x));
        if self.samples >= 2 && denominator.abs() > f64::EPSILON {
            self.scale = (count.mul_add(self.sum_xy, -(self.sum_x * self.sum_y)) / denominator).clamp(0.5, 2.0);
            self.offset = self.scale.mul_add(-self.sum_x, self.sum_y) / count;
        }
        let fitted = self.scale.mul_add(x, self.offset);
        self.max_residual = self.max_residual.max((y - fitted).abs());
        self.last_device = event.device_micros;
        self.last_callback = event.callback_micros;
        let calibrated_micros = self
            .origin_callback
            .saturating_add(fitted.max(0.0).min(u64::MAX as f64) as u64);
        CalibratedMidiEvent {
            raw: event,
            calibrated_micros,
            calibration: self.calibration(),
        }
    }

    /// Current immutable calibration summary.
    pub fn calibration(&self) -> MidiClockCalibration {
        MidiClockCalibration {
            scale: self.scale,
            offset_micros: self
                .scale
                .mul_add(-(self.origin_device as f64), self.offset + self.origin_callback as f64),
            samples: self.samples,
            max_residual_micros: self.max_residual,
            discontinuities: self.discontinuities,
            quality: if self.discontinuous {
                MidiClockQuality::Discontinuous
            } else if self.samples >= 2 {
                MidiClockQuality::Calibrated
            } else {
                MidiClockQuality::Uncalibrated
            },
        }
    }
}

#[cfg(test)]
mod midi_laws {
    use super::*;

    fn event(message: &[u8]) -> MidiInputEvent {
        let Decode::Event(event) = decode(10, 12, message) else {
            std::process::abort()
        };
        event
    }

    #[test]
    fn expressive_channel_messages_retain_values_and_channels() {
        assert_eq!(
            (event(&[0x92, 60, 99]).kind, event(&[0x92, 60, 99]).channel),
            (MidiMessageKind::NoteOn, 2)
        );
        assert_eq!(event(&[0x92, 60, 0]).kind, MidiMessageKind::NoteOff);
        assert_eq!((event(&[0xB3, 64, 127]).data, event(&[0xB3, 64, 127]).value), (64, 127));
        assert_eq!(event(&[0xE0, 0, 0]).value, -8_192);
        assert_eq!(event(&[0xE0, 0, 64]).value, 0);
        assert_eq!(event(&[0xE0, 127, 127]).value, 8_191);
        assert_eq!(event(&[0xA0, 60, 42]).kind, MidiMessageKind::KeyPressure);
        assert_eq!(event(&[0xD0, 42]).kind, MidiMessageKind::ChannelPressure);
    }

    #[test]
    fn malformed_system_and_sysex_are_distinct_losses() {
        assert!(matches!(decode(0, 0, &[0x90, 60]), Decode::Malformed));
        assert!(matches!(decode(0, 0, &[0xF8]), Decode::System));
        assert!(matches!(decode(0, 0, &[0xF0, 1, 0xF7]), Decode::SysEx));
    }

    #[test]
    fn affine_clock_fit_preserves_timing_and_detects_a_jump() {
        let mut clock = MidiClockCalibrator::default();
        for point in 0..8u64 {
            let calibrated = clock.observe(MidiInputEvent {
                device_micros: 1_000 + point * 1_000,
                callback_micros: 50 + point * 1_001,
                cable: 0,
                channel: 0,
                kind: MidiMessageKind::NoteOn,
                data: 60,
                value: 64,
            });
            assert!(calibrated.calibrated_micros.abs_diff(50 + point * 1_001) <= 1);
        }
        assert_eq!(clock.calibration().quality, MidiClockQuality::Calibrated);
        let jumped = clock.observe(MidiInputEvent {
            device_micros: 2,
            callback_micros: 9_000,
            cable: 0,
            channel: 0,
            kind: MidiMessageKind::NoteOff,
            data: 60,
            value: 0,
        });
        assert_eq!(jumped.calibration.quality, MidiClockQuality::Discontinuous);
        assert_eq!(jumped.calibration.discontinuities, 1);
    }

    #[test]
    fn no_device_is_a_normal_empty_connection() {
        let mut input = MidiInput::open(Some("musa-test-device-that-does-not-exist"));
        assert!(input.device().is_none());
        assert!(input.poll().is_none());
    }

    #[test]
    fn several_devices_require_identity_and_hotplug_never_substitutes() {
        let ids = vec!["keys-a".to_owned(), "keys-b".to_owned()];
        let first = ids.get(..1).unwrap_or_default();
        assert_eq!(selection_index(&ids, None), None);
        assert_eq!(selection_index(&ids, Some("keys-b")), Some(1));
        assert_eq!(selection_index(first, Some("keys-b")), None);
        assert_eq!(selection_index(first, None), Some(0));
    }

    #[test]
    fn a_full_callback_ring_counts_loss_without_growing() {
        let (mut producer, mut consumer) = rtrb::RingBuffer::new(1);
        let losses = LossCounters::default();
        let event = event(&[0x90, 60, 90]);
        push_event(&mut producer, &losses, event);
        push_event(&mut producer, &losses, event);
        assert_eq!(losses.snapshot().queue_overflow, 1);
        assert_eq!(consumer.pop().ok(), Some(event));
        assert!(consumer.pop().is_err());
    }
}
