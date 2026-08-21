//! Live MIDI input: a keyboard's note-ons and note-offs, delivered to the
//! control side over an `rtrb` queue.
//!
//! **The MIDI thread is a real-time thread.** midir hands us a callback on a
//! thread it owns, and the real-time rules apply to it exactly as they apply
//! to the audio callback: no allocation, no locks, no I/O, no logging. All it
//! does is decode three bytes and push a `Copy` event into a preallocated ring.
//! Everything musical — spelling a note number as a written pitch, deciding
//! whether two keys are a chord — happens on the control side, where it can
//! see the score.
//!
//! **No device is normal, not an error.** A laptop with no keyboard plugged
//! in, and CI with no MIDI stack at all, must both work: [`MidiInput::open`]
//! therefore always succeeds and reports [`MidiInput::port`] as `None`. A
//! session with no keyboard polls an empty queue, which costs one atomic load.
//!
//! This is input only. Recording a performance and MIDI output to external
//! devices are not part of the workbench.

/// How many events the ring holds between polls.
///
/// A control side that polls at any human rate never sees it full; a control
/// side that has stopped polling entirely — a window that went away — drops
/// the oldest work rather than blocking the keyboard's thread.
const INPUT_CAPACITY: usize = 256;

/// The client name a MIDI host shows for this application.
const CLIENT: &str = "musa";

/// One key pressed or released on a connected keyboard.
///
/// `Copy` and three fields wide on purpose: this is what crosses the queue,
/// and the queue is written from a real-time thread.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MidiInputEvent {
    /// The MIDI note number (60 is middle C).
    pub note: u8,
    /// True for a press.
    ///
    /// Velocity is read but not carried: entry does not interpret it
    /// (dynamics are the profile's job, not the keyboard's), and its only
    /// other role — telling a zero-velocity note-on from a real press — is
    /// settled in [`decode`] before the event travels.
    pub pressed: bool,
}

/// A connection to one MIDI keyboard, or to none.
///
/// Dropping it disconnects. The control side polls with [`Self::poll`];
/// nothing here blocks.
pub struct MidiInput {
    /// Held for the connection's lifetime; dropping it closes the port.
    _connection: Option<midir::MidiInputConnection<()>>,
    events: rtrb::Consumer<MidiInputEvent>,
    port: Option<String>,
}

impl MidiInput {
    /// Connect to a keyboard: `preferred` by name if it is there, otherwise
    /// the first port the host lists, otherwise nothing at all.
    ///
    /// Never fails. A host that cannot be reached, a port that cannot be
    /// opened, and a machine with no keyboard are the same situation from
    /// here — there is no keyboard — and the caller's only question is
    /// [`Self::port`].
    pub fn open(preferred: Option<&str>) -> Self {
        let (mut producer, events) = rtrb::RingBuffer::<MidiInputEvent>::new(INPUT_CAPACITY);
        let Some((input, port, name)) = choose(preferred) else {
            return Self {
                _connection: None,
                events,
                port: None,
            };
        };
        let connection = input.connect(
            &port,
            CLIENT,
            move |_timestamp, message, ()| {
                if let Some(event) = decode(message) {
                    // A full ring means nobody is reading; dropping the event
                    // is the only choice that keeps this thread real-time.
                    producer.push(event).ok();
                }
            },
            (),
        );
        match connection {
            Ok(connection) => Self {
                _connection: Some(connection),
                events,
                port: Some(name),
            },
            Err(error) => {
                // Not on the MIDI thread, so logging is allowed here.
                tracing::warn!(error = %error.kind(), "could not open the MIDI input port");
                Self {
                    _connection: None,
                    events,
                    port: None,
                }
            }
        }
    }

    /// The port this is listening to, or `None` when no keyboard is connected.
    pub fn port(&self) -> Option<&str> {
        self.port.as_deref()
    }

    /// The next event, or `None` when the keyboard has been quiet.
    pub fn poll(&mut self) -> Option<MidiInputEvent> {
        self.events.pop().ok()
    }
}

/// The port to listen to: `preferred` by name, else the first one listed
/// (the zero-setup rule, applied to the keyboard).
fn choose(preferred: Option<&str>) -> Option<(midir::MidiInput, midir::MidiInputPort, String)> {
    let mut input = midir::MidiInput::new(CLIENT).ok()?;
    // Musa reads note-ons and note-offs; clock, active sensing, and sysex are
    // noise on this path and are dropped before they reach the callback.
    input.ignore(midir::Ignore::All);
    let ports = input.ports();
    let named = |port: &midir::MidiInputPort| input.port_name(port).ok();
    let found = preferred
        .and_then(|wanted| {
            ports
                .iter()
                .find(|port| named(port).as_deref() == Some(wanted))
                .cloned()
        })
        .or_else(|| ports.first().cloned())?;
    let name = named(&found)?;
    Some((input, found, name))
}

/// Decode a channel-voice message. Anything that is not a note-on or
/// note-off is not entry, and is dropped.
///
/// A note-on with velocity 0 is a release — every keyboard made since the
/// 1980s spells it that way, and reading it as a press would leave notes
/// hanging.
fn decode(message: &[u8]) -> Option<MidiInputEvent> {
    let [status, note, velocity] = *message else {
        return None;
    };
    if note > 127 {
        return None;
    }
    match status & 0xF0 {
        0x90 => Some(MidiInputEvent {
            note,
            pressed: velocity > 0,
        }),
        0x80 => Some(MidiInputEvent { note, pressed: false }),
        _ => None,
    }
}

#[cfg(test)]
mod midi_laws {
    use super::{MidiInput, MidiInputEvent, decode};

    /// A press is a press, a release is a release, and the zero-velocity
    /// note-on every keyboard sends is the second one.
    #[test]
    fn note_messages_decode_to_presses_and_releases() {
        assert_eq!(
            decode(&[0x90, 60, 64]),
            Some(MidiInputEvent {
                note: 60,
                pressed: true
            })
        );
        assert_eq!(
            decode(&[0x90, 60, 0]),
            Some(MidiInputEvent {
                note: 60,
                pressed: false
            })
        );
        assert_eq!(
            decode(&[0x80, 60, 64]),
            Some(MidiInputEvent {
                note: 60,
                pressed: false
            })
        );
    }

    /// The channel a keyboard sends on is not something entry asks about, so
    /// all sixteen decode alike.
    #[test]
    fn every_channel_is_read() {
        for channel in 0..16u8 {
            let pressed = decode(&[0x90 | channel, 64, 100]).map(|event| event.pressed);
            assert_eq!(pressed, Some(true), "channel {channel}");
        }
    }

    /// Control changes, pitch bend, clock, and truncated messages are not
    /// notes and must not become them.
    #[test]
    fn other_messages_are_not_entry() {
        for message in [
            [0xB0, 7, 100].as_slice(),
            [0xE0, 0, 64].as_slice(),
            [0xF8].as_slice(),
            [0x90, 60].as_slice(),
        ] {
            assert_eq!(decode(message), None, "{message:?}");
        }
    }

    /// The one law the no-device case has to satisfy: opening works, and
    /// polling it is quiet. CI has no keyboard and must still be green.
    #[test]
    fn a_machine_with_no_keyboard_still_opens() {
        let mut input = MidiInput::open(None);
        // A developer's machine may well have a port; either way this is not
        // an error and polling answers without blocking.
        assert!(input.poll().is_none());
    }
}
