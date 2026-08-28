/// What the framework decides on its own.
///
/// The Rust suite proves the library's laws — that a block partition is
/// unobservable, that rendering allocates nothing — against the library's own
/// ABI. The automated host proves the component's behavior against a real
/// `AVAudioUnit`. Between them sits a thin layer of Swift that translates:
/// four-character codes, MIDI 1.0 voice messages, and the atomic slot a
/// prepared instrument is published through. That layer is what this bundle
/// tests, because neither of the other two can reach it directly.

import AudioToolbox
import XCTest

@testable import MusaAudioUnitKit

final class ComponentIdentityTests: XCTestCase {
    func testFourCharacterCodesRoundTrip() {
        for text in ["aumu", "musa", "Musa", "aufx"] {
            XCTAssertEqual(fourCharacterText(fourCharacterCode(text)), text)
        }
    }

    func testTheInstrumentIsTheValidatedTriple() {
        let description = musaInstrumentDescription
        XCTAssertEqual(fourCharacterText(description.componentType), "aumu")
        XCTAssertEqual(fourCharacterText(description.componentSubType), "musa")
        XCTAssertEqual(fourCharacterText(description.componentManufacturer), musaManufacturer)
    }

    func testTheStateKeysAreDistinct() {
        let keys = [
            MusaStateKey.version, MusaStateKey.abiVersion, MusaStateKey.project, MusaStateKey.access,
            MusaStateKey.piece, MusaStateKey.part, MusaStateKey.musicIdentity, MusaStateKey.assetIdentity,
            MusaStateKey.inputs, MusaStateKey.controlTable, MusaStateKey.controlLosses, MusaStateKey.outputs,
            MusaStateKey.refusal,
        ]
        XCTAssertEqual(Set(keys).count, keys.count)
    }

    func testTheFrameworkAndTheLibraryAgreeOnTheAbi() {
        XCTAssertEqual(musa_au_abi_version(), MUSA_AU_ABI_VERSION)
    }
}

final class ControlDescriptorTests: XCTestCase {
    private func control(_ identity: String, continuous: Bool = true) -> MusaControl {
        MusaControl(
            identity: identity,
            display: "expression",
            summary: "how much the player is giving the note",
            kind: "Normalized",
            updateRate: continuous ? "Continuous" : "PerNote",
            address: 0x1234,
            minimum: 0,
            maximum: 1,
            defaultValue: 0.5,
            continuous: continuous
        )
    }

    /// A canonical identity is full of the punctuation `AUParameterNode`
    /// reads as a key path, so the identifier folds it — deterministically,
    /// and without the identity itself changing.
    func testAParameterIdentifierCarriesNoKeyPathPunctuation() {
        let identifier = control("std.performance::expression").parameterIdentifier
        XCTAssertEqual(identifier, "std_performance__expression")
        XCTAssertFalse(identifier.contains("."))
        XCTAssertFalse(identifier.contains(":"))
    }

    func testTwoControlsThatDifferKeepDifferentIdentifiers() {
        XCTAssertNotEqual(
            control("std.performance::expression").parameterIdentifier,
            control("std.performance::emphasis").parameterIdentifier
        )
    }

    func testAnOutputRoleIsWhatTheLibrarySaidItWas() {
        XCTAssertEqual(MusaOutputRole(rawValue: 0), .main)
        XCTAssertEqual(MusaOutputRole(rawValue: 1), .part)
        XCTAssertEqual(MusaOutputRole(rawValue: 2), .bus)
        XCTAssertNil(MusaOutputRole(rawValue: 3))
    }
}

final class ParameterEventTests: XCTestCase {
    func testAParameterEventCarriesItsAddressValueAndRamp() {
        let event = musaParameterEvent(frame: 64, address: 0xABCD, value: 0.25, ramp: 512)
        XCTAssertEqual(event.kind, UInt8(MUSA_AU_EVENT_PARAMETER))
        XCTAssertEqual(event.address, 0xABCD)
        XCTAssertEqual(event.value, 0.25)
        XCTAssertEqual(event.ramp, 512)
        XCTAssertEqual(event.frame, 64)
    }

    /// The wire struct carries both kinds of event, so a MIDI message has to
    /// say what it does *not* mean as well as what it does.
    func testAMIDIEventMeansNothingByItsParameterFields() {
        var message = AUMIDIEvent()
        message.eventSampleTime = 0
        message.length = 3
        message.data = (0x90, 60, 100)
        let decoded = musaDecodeMIDI(message, 512)
        XCTAssertEqual(decoded?.address, 0)
        XCTAssertEqual(decoded?.ramp, 0)
        XCTAssertEqual(decoded?.value, 0)
    }
}

final class MIDIDecodingTests: XCTestCase {
    private func message(_ status: UInt8, _ data1: UInt8, _ data2: UInt8, at time: Int64 = 0) -> AUMIDIEvent {
        var event = AUMIDIEvent()
        event.eventSampleTime = time
        event.length = 3
        event.data = (status, data1, data2)
        return event
    }

    func testNoteOnCarriesPitchAndVelocity() {
        let decoded = musaDecodeMIDI(message(0x90, 60, 100), 512)
        XCTAssertEqual(decoded?.kind, UInt8(MUSA_AU_EVENT_NOTE_ON))
        XCTAssertEqual(decoded?.voice, 60)
        XCTAssertEqual(decoded?.data1, 60)
        XCTAssertEqual(decoded?.data2, 100)
    }

    func testAZeroVelocityNoteOnIsANoteOff() {
        XCTAssertEqual(musaDecodeMIDI(message(0x90, 60, 0), 512)?.kind, UInt8(MUSA_AU_EVENT_NOTE_OFF))
        XCTAssertEqual(musaDecodeMIDI(message(0x80, 60, 64), 512)?.kind, UInt8(MUSA_AU_EVENT_NOTE_OFF))
    }

    func testANoteOffPairsWithItsAttackByPitch() {
        let on = musaDecodeMIDI(message(0x90, 55, 90), 512)
        let off = musaDecodeMIDI(message(0x80, 55, 0), 512)
        XCTAssertEqual(on?.voice, off?.voice)
    }

    func testTheContinuousDimensionsAreNotVoiced() {
        XCTAssertEqual(musaDecodeMIDI(message(0xB0, 74, 20), 512)?.kind, UInt8(MUSA_AU_EVENT_CONTROLLER))
        XCTAssertEqual(musaDecodeMIDI(message(0xB0, 74, 20), 512)?.voice, 0)
        XCTAssertEqual(musaDecodeMIDI(message(0xD0, 40, 0), 512)?.kind, UInt8(MUSA_AU_EVENT_CHANNEL_PRESSURE))
        XCTAssertEqual(musaDecodeMIDI(message(0xE0, 0, 64), 512)?.kind, UInt8(MUSA_AU_EVENT_PITCH_BEND))
    }

    func testKeyPressureKeepsItsVoice() {
        let decoded = musaDecodeMIDI(message(0xA0, 72, 30), 512)
        XCTAssertEqual(decoded?.kind, UInt8(MUSA_AU_EVENT_KEY_PRESSURE))
        XCTAssertEqual(decoded?.voice, 72)
    }

    func testWhatHasNoBindingIsDroppedRatherThanGuessed() {
        // Program change and system common: no source-declared dimension to
        // reach, so the decoder says so instead of approximating one.
        XCTAssertNil(musaDecodeMIDI(message(0xC0, 4, 0), 512))
        XCTAssertNil(musaDecodeMIDI(message(0xF0, 0, 0), 512))
    }

    func testOffsetsAreClampedIntoTheBlock() {
        XCTAssertEqual(musaDecodeMIDI(message(0x90, 60, 100, at: -128), 512)?.frame, 0)
        XCTAssertEqual(musaDecodeMIDI(message(0x90, 60, 100, at: 200), 512)?.frame, 200)
        XCTAssertEqual(musaDecodeMIDI(message(0x90, 60, 100, at: 9_000), 512)?.frame, 512)
    }
}

final class PublicationSlotTests: XCTestCase {
    /// The slot stores and hands back pointers; it never dereferences one, so
    /// a test may use addresses that stand for instruments without there
    /// being any.
    private func stand(for address: Int) -> OpaquePointer {
        guard let pointer = OpaquePointer(bitPattern: address) else {
            preconditionFailure("a nonzero address is a valid opaque pointer")
        }
        return pointer
    }

    func testAFreshSlotIsEmpty() {
        var slot = MusaAuSlot()
        musa_au_slot_init(&slot)
        XCTAssertNil(musa_au_slot_current(&slot))
        XCTAssertEqual(musa_au_slot_generation(&slot), 0)
    }

    func testPublishingHandsBackWhatItReplaced() {
        var slot = MusaAuSlot()
        musa_au_slot_init(&slot)
        let first = stand(for: 0x1000)
        let second = stand(for: 0x2000)

        XCTAssertNil(musa_au_slot_publish(&slot, first))
        XCTAssertEqual(musa_au_slot_current(&slot), first)
        XCTAssertEqual(musa_au_slot_publish(&slot, second), first)
        XCTAssertEqual(musa_au_slot_current(&slot), second)
    }

    func testTheGenerationSeparatesNotReadyFromReadyAndSilent() {
        var slot = MusaAuSlot()
        musa_au_slot_init(&slot)
        XCTAssertEqual(musa_au_slot_generation(&slot), 0)
        _ = musa_au_slot_publish(&slot, stand(for: 0x1000))
        XCTAssertEqual(musa_au_slot_generation(&slot), 1)
        _ = musa_au_slot_publish(&slot, nil)
        XCTAssertEqual(musa_au_slot_generation(&slot), 2)
        XCTAssertNil(musa_au_slot_current(&slot))
    }
}
