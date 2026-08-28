/// One host MIDI message, as this component's ABI spells it.
///
/// A free function rather than a method so the render block can call it
/// without touching an object: everything it reads is on the stack, and
/// everything it returns is a value.
///
/// The component documents MIDI 1.0 voice messages and nothing else. What it
/// does *not* do is guess: a system message, a program change, or a fourteen-
/// bit controller pair has no source binding to reach, so it is dropped here
/// and counted by the library as an unbound event rather than approximated.
///
/// Voice identity is the note number. Musa pairs an attack with its release
/// by identity rather than by pitch, which is what lets a keyboard overlap
/// the same note; MIDI 1.0 has no such identity to offer, so the pitch is the
/// best pairing the wire supports and the component says so rather than
/// inventing one the host did not send.

import AudioToolbox

@inline(__always)
func musaDecodeMIDI(_ event: AUMIDIEvent, _ frameCount: AUAudioFrameCount) -> MusaAuEvent? {
    let bytes = event.data
    let status = bytes.0 & 0xF0
    let data1 = bytes.1 & 0x7F
    let data2 = bytes.2 & 0x7F
    // A host may place an event at a negative offset when it is catching up.
    // Clamping to the start of the block is what every other component does,
    // and it is honest here for the same reason: the event belongs to this
    // block, and the only question is where in it.
    let offset = event.eventSampleTime < 0 ? 0 : UInt32(min(event.eventSampleTime, Int64(frameCount)))

    func made(_ kind: UInt8, _ first: UInt8, _ second: UInt8) -> MusaAuEvent {
        musaEvent(frame: offset, voice: UInt32(data1), kind: kind, data1: first, data2: second)
    }

    switch status {
    case 0x90 where data2 > 0:
        return made(UInt8(MUSA_AU_EVENT_NOTE_ON), data1, data2)
    // A note-on with zero velocity is a note-off; hosts still send it.
    case 0x80, 0x90:
        return made(UInt8(MUSA_AU_EVENT_NOTE_OFF), data1, data2)
    case 0xA0:
        return made(UInt8(MUSA_AU_EVENT_KEY_PRESSURE), data1, data2)
    case 0xB0:
        return musaEvent(frame: offset, voice: 0, kind: UInt8(MUSA_AU_EVENT_CONTROLLER), data1: data1, data2: data2)
    case 0xD0:
        return musaEvent(frame: offset, voice: 0, kind: UInt8(MUSA_AU_EVENT_CHANNEL_PRESSURE), data1: data1, data2: 0)
    case 0xE0:
        return musaEvent(frame: offset, voice: 0, kind: UInt8(MUSA_AU_EVENT_PITCH_BEND), data1: data1, data2: data2)
    default:
        return nil
    }
}

/// One MIDI-shaped event, with the fields a parameter event uses left empty.
///
/// The wire struct carries both kinds, so every construction has to say
/// something about `address`, `ramp`, and `value`. Saying it once here keeps
/// a decoder from quietly meaning a parameter address of whatever was next
/// on the stack.
@inline(__always)
func musaEvent(frame: UInt32, voice: UInt32, kind: UInt8, data1: UInt8, data2: UInt8) -> MusaAuEvent {
    MusaAuEvent(
        address: 0,
        frame: frame,
        voice: voice,
        ramp: 0,
        value: 0,
        kind: kind,
        data1: data1,
        data2: data2,
        reserved: 0
    )
}

/// One host parameter change, at a sample offset inside this block.
///
/// `06-daw-boundary.md` §3: this is an ephemeral performance overlay. It
/// carries a source-declared control's address and a normalized value, and it
/// reaches the instrument through the mapping the source already declared.
/// Nothing here writes `.musa`.
@inline(__always)
func musaParameterEvent(frame: UInt32, address: UInt64, value: Float, ramp: UInt32) -> MusaAuEvent {
    MusaAuEvent(
        address: address,
        frame: frame,
        voice: 0,
        ramp: ramp,
        value: value,
        kind: UInt8(MUSA_AU_EVENT_PARAMETER),
        data1: 0,
        data2: 0,
        reserved: 0
    )
}
