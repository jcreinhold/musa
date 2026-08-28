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
    func render(frames: Int, sampleTime: Double, events: EventList? = nil, hostSuppliesMemory: Bool = true) -> [Float] {
        timestamp.pointee.mSampleTime = sampleTime
        for index in 0..<2 {
            list[index].mDataByteSize = UInt32(frames * MemoryLayout<Float>.size)
            list[index].mData = hostSuppliesMemory ? UnsafeMutableRawPointer(storage[index]) : nil
        }
        _ = block(
            flags,
            timestamp,
            AUAudioFrameCount(frames),
            0,
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
        var out: [Float] = []
        var start = 0
        while start < frames {
            let count = min(size, frames - start)
            let inside = events.filter { $0.0 >= start && $0.0 < start + count }
            let eventList = EventList()
            for (offset, bytes) in inside {
                eventList.addMIDI(bytes, at: AUEventSampleTime(offset - start))
            }
            out.append(
                contentsOf: render(frames: count, sampleTime: Double(start), events: inside.isEmpty ? nil : eventList)
            )
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
