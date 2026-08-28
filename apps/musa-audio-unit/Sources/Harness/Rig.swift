/// Driving a component without a workstation.
///
/// Everything below renders through `internalRenderBlock` directly, the way
/// prompt 215's trial harness did, so that a failure names the component
/// rather than an audio device.

import AVFoundation
import AudioToolbox
import Foundation
import MusaAudioUnitKit

/// A preallocated buffer list and the pointers a render call needs.
final class Rig {
    let unit: MusaInstrumentAudioUnit
    private let block: AUInternalRenderBlock
    private let flags = UnsafeMutablePointer<AudioUnitRenderActionFlags>.allocate(capacity: 1)
    private let timestamp = UnsafeMutablePointer<AudioTimeStamp>.allocate(capacity: 1)
    private var list: UnsafeMutableAudioBufferListPointer
    private var storage: [UnsafeMutablePointer<Float>] = []

    init(unit: MusaInstrumentAudioUnit, capacity: Int) throws {
        self.unit = unit
        unit.maximumFramesToRender = AUAudioFrameCount(capacity)
        try unit.allocateRenderResources()
        block = unit.internalRenderBlock
        flags.initialize(to: [])
        timestamp.initialize(to: AudioTimeStamp())
        timestamp.pointee.mFlags = .sampleTimeValid
        list = AudioBufferList.allocate(maximumBuffers: 2)
        for index in 0..<2 {
            let channel = UnsafeMutablePointer<Float>.allocate(capacity: capacity)
            channel.initialize(repeating: 0, count: capacity)
            storage.append(channel)
            list[index] = AudioBuffer(
                mNumberChannels: 1,
                mDataByteSize: UInt32(capacity * MemoryLayout<Float>.size),
                mData: UnsafeMutableRawPointer(channel)
            )
        }
    }

    deinit {
        flags.deallocate()
        timestamp.deallocate()
        for channel in storage { channel.deallocate() }
        free(list.unsafeMutablePointer)
    }

    /// The pointers a driver written in Objective-C needs, so that the
    /// allocation numbers are about the component and not about Swift
    /// handing a block across a language boundary once per call.
    var flagsPointer: UnsafeMutablePointer<AudioUnitRenderActionFlags> { flags }
    var timestampPointer: UnsafeMutablePointer<AudioTimeStamp> { timestamp }
    var bufferListPointer: UnsafeMutablePointer<AudioBufferList> { list.unsafeMutablePointer }

    /// A driver holding this component's render block.
    func driver() -> MusaAuDriver { MusaAuDriver(renderBlock: block) }

    /// Render one block at `sampleTime`, returning interleaved stereo frames.
    ///
    /// `bus` is which output the host is asking for. A multi-output component
    /// is called once per bus at the same timestamp, so a caller that wants
    /// several of them asks for each in turn without moving `sampleTime`.
    func render(
        frames: Int,
        sampleTime: Double,
        events: EventList? = nil,
        hostSuppliesMemory: Bool = true,
        bus: Int = 0
    ) -> [Float] {
        timestamp.pointee.mSampleTime = sampleTime
        for index in 0..<2 {
            list[index].mDataByteSize = UInt32(frames * MemoryLayout<Float>.size)
            list[index].mData = hostSuppliesMemory ? UnsafeMutableRawPointer(storage[index]) : nil
        }
        _ = block(
            flags,
            timestamp,
            AUAudioFrameCount(frames),
            bus,
            list.unsafeMutablePointer,
            events?.first,
            nil
        )
        var out: [Float] = []
        out.reserveCapacity(frames * 2)
        let left = list[0].mData!.assumingMemoryBound(to: Float.self)
        let right = list[1].mData!.assumingMemoryBound(to: Float.self)
        for frame in 0..<frames {
            out.append(left[frame])
            out.append(right[frame])
        }
        return out
    }

    /// Render `frames` in blocks of `block`, delivering events at absolute
    /// offsets, and return every frame produced.
    func renderPartitioned(frames: Int, block size: Int, events: [(Int, [UInt8])]) -> [Float] {
        renderPartitioned(frames: frames, block: size, midi: events, parameters: [])
    }

    /// The single-bus form, which is what most measurements want.
    func renderPartitioned(
        frames: Int,
        block size: Int,
        midi: [(Int, [UInt8])],
        parameters: [(Int, UInt64, Float, UInt32)]
    ) -> [Float] {
        renderBuses(frames: frames, block: size, midi: midi, parameters: parameters, buses: 1)[0]
    }

    /// The same, with parameter changes among the messages.
    ///
    /// Each parameter entry is an absolute frame, an address, a value, and a
    /// ramp length in frames — zero for a point change.
    func renderBuses(
        frames: Int,
        block size: Int,
        midi: [(Int, [UInt8])],
        parameters: [(Int, UInt64, Float, UInt32)],
        buses: Int
    ) -> [[Float]] {
        var out = [[Float]](repeating: [], count: buses)
        var start = 0
        while start < frames {
            let count = min(size, frames - start)
            let insideMIDI = midi.filter { $0.0 >= start && $0.0 < start + count }
            let insideParameters = parameters.filter { $0.0 >= start && $0.0 < start + count }
            let eventList = EventList()
            // In time order, the way a host delivers them: the component
            // trusts the list to be sorted, so a harness that shuffled it
            // would be measuring its own mistake.
            let combined: [(Int, Bool, Int)] =
                insideMIDI.enumerated().map { ($0.element.0, true, $0.offset) }
                    + insideParameters.enumerated().map { ($0.element.0, false, $0.offset) }
            for (offset, isMIDI, index) in combined.sorted(by: { $0.0 < $1.0 }) {
                if isMIDI {
                    eventList.addMIDI(insideMIDI[index].1, at: AUEventSampleTime(offset - start))
                } else {
                    let (_, address, value, ramp) = insideParameters[index]
                    eventList.addParameter(address, value: value, ramp: ramp, at: AUEventSampleTime(offset - start))
                }
            }
            let list = eventList.isEmpty ? nil : eventList
            for bus in 0..<buses {
                out[bus].append(
                    contentsOf: render(frames: count, sampleTime: Double(start), events: bus == 0 ? list : nil, bus: bus)
                )
            }
            start += count
        }
        return out
    }
}

/// A linked list of `AURenderEvent` values, built once and kept alive.
final class EventList {
    private var storage: [UnsafeMutablePointer<AURenderEvent>] = []

    deinit {
        for event in storage { event.deallocate() }
    }

    var first: UnsafePointer<AURenderEvent>? {
        storage.first.map { UnsafePointer($0) }
    }

    var isEmpty: Bool { storage.isEmpty }

    /// One host parameter change, exactly as a workstation's automation lane
    /// delivers it: an address, a normalized value, and a ramp in frames.
    func addParameter(_ address: UInt64, value: Float, ramp: UInt32, at time: AUEventSampleTime) {
        let event = UnsafeMutablePointer<AURenderEvent>.allocate(capacity: 1)
        event.initialize(to: AURenderEvent())
        event.pointee.head.next = nil
        event.pointee.head.eventSampleTime = time
        event.pointee.head.eventType = ramp > 0 ? .parameterRamp : .parameter
        event.pointee.parameter.rampDurationSampleFrames = ramp
        event.pointee.parameter.parameterAddress = AUParameterAddress(address)
        event.pointee.parameter.value = value
        storage.last?.pointee.head.next = UnsafeMutablePointer(event)
        storage.append(event)
    }

    func addMIDI(_ bytes: [UInt8], at time: AUEventSampleTime) {
        let event = UnsafeMutablePointer<AURenderEvent>.allocate(capacity: 1)
        event.initialize(to: AURenderEvent())
        event.pointee.head.eventType = .MIDI
        event.pointee.head.next = nil
        event.pointee.MIDI.eventSampleTime = time
        event.pointee.MIDI.cable = 0
        event.pointee.MIDI.length = UInt16(bytes.count)
        event.pointee.MIDI.data.0 = bytes.count > 0 ? bytes[0] : 0
        event.pointee.MIDI.data.1 = bytes.count > 1 ? bytes[1] : 0
        event.pointee.MIDI.data.2 = bytes.count > 2 ? bytes[2] : 0
        storage.last?.pointee.head.next = UnsafeMutablePointer(event)
        storage.append(event)
    }
}
