//  What the harness needs to be a host: a finding, a report, a render rig,
//  and a binding to the allocation probe.

import AudioToolbox
import AVFoundation
import Foundation
import MusaTrialKit

/// One measured claim. `unsupported` is a real answer and is never quietly
/// turned into a pass: prompt 215 exists to record what the platform will not
/// do as plainly as what it will.
struct Finding {
    enum Outcome: String {
        case pass
        case fail
        case unsupported
    }

    let id: String
    let title: String
    let outcome: Outcome
    let detail: String
    var numbers: [String: Double] = [:]
}

/// Everything the run measured, written as JSON on standard output.
final class Report {
    private(set) var findings: [Finding] = []

    func record(_ finding: Finding) {
        findings.append(finding)
        FileHandle.standardError.write("\(finding.outcome.rawValue.padding(toLength: 11, withPad: " ", startingAt: 0)) \(finding.id) — \(finding.detail)\n".data(using: .utf8)!)
    }

    func check(_ id: String, _ title: String, _ condition: Bool, _ detail: String, _ numbers: [String: Double] = [:]) {
        record(Finding(id: id, title: title, outcome: condition ? .pass : .fail, detail: detail, numbers: numbers))
    }

    func unsupported(_ id: String, _ title: String, _ detail: String) {
        record(Finding(id: id, title: title, outcome: .unsupported, detail: detail))
    }

    func json(environment: [String: String]) -> String {
        let entries = findings.map { finding -> [String: Any] in
            [
                "id": finding.id,
                "title": finding.title,
                "outcome": finding.outcome.rawValue,
                "detail": finding.detail,
                "numbers": finding.numbers,
            ]
        }
        let payload: [String: Any] = ["environment": environment, "findings": entries]
        let data = try! JSONSerialization.data(withJSONObject: payload, options: [.prettyPrinted, .sortedKeys])
        return String(decoding: data, as: UTF8.self)
    }
}

/// The allocation probe, if `DYLD_INSERT_LIBRARIES` put it in this process.
enum AllocationProbe {
    private typealias Arm = @convention(c) (Int32) -> Void
    private typealias Count = @convention(c) () -> UInt64
    private typealias Reset = @convention(c) () -> Void

    private static let arm = unsafeBitCast(dlsym(UnsafeMutableRawPointer(bitPattern: -2), "musa_probe_arm"), to: Arm?.self)
    private static let count = unsafeBitCast(dlsym(UnsafeMutableRawPointer(bitPattern: -2), "musa_probe_allocations"), to: Count?.self)
    private static let reset = unsafeBitCast(dlsym(UnsafeMutableRawPointer(bitPattern: -2), "musa_probe_reset"), to: Reset?.self)

    /// Whether the probe is present *and* counting. A probe that loaded but
    /// does not observe a deliberate allocation is not evidence of anything.
    static var isWorking: Bool {
        guard let arm, let count, let reset else { return false }
        reset()
        arm(1)
        let sample = malloc(64)
        free(sample)
        arm(0)
        return count() > 0
    }

    /// Count the allocations that happen inside `body`.
    static func measure(_ body: () -> Void) -> UInt64? {
        guard let arm, let count, let reset else { return nil }
        reset()
        arm(1)
        body()
        arm(0)
        return count()
    }
}

/// A manual host: it owns the buffers, chooses the block sizes, writes the
/// timestamps, and builds the render-event list itself, which is the only way
/// to ask a component what it does at a boundary a real host rarely reaches.
final class RenderRig {
    let unit: AUAudioUnit
    private let block: AUInternalRenderBlock
    private let channels: Int
    private let capacity: Int
    private let storage: [UnsafeMutablePointer<Float>]
    private let list: UnsafeMutableAudioBufferListPointer
    private let flags = UnsafeMutablePointer<AudioUnitRenderActionFlags>.allocate(capacity: 1)
    private let timestamp = UnsafeMutablePointer<AudioTimeStamp>.allocate(capacity: 1)

    init(unit: AUAudioUnit, channels: Int = 2, capacity: Int = 4_096) {
        self.unit = unit
        self.channels = channels
        self.capacity = capacity
        block = unit.internalRenderBlock
        flags.initialize(to: AudioUnitRenderActionFlags())
        timestamp.initialize(to: AudioTimeStamp())
        storage = (0..<channels).map { _ in
            let buffer = UnsafeMutablePointer<Float>.allocate(capacity: capacity)
            buffer.initialize(repeating: 0, count: capacity)
            return buffer
        }
        list = AudioBufferList.allocate(maximumBuffers: channels)
    }

    deinit {
        for buffer in storage {
            buffer.deinitialize(count: capacity)
            buffer.deallocate()
        }
        free(list.unsafeMutablePointer)
        flags.deinitialize(count: 1)
        flags.deallocate()
        timestamp.deinitialize(count: 1)
        timestamp.deallocate()
    }

    /// Render one block, optionally supplying no memory so the component has
    /// to provide its own, and optionally at a sample time of the host's
    /// choosing rather than the next contiguous one.
    @discardableResult
    func render(frames: Int,
                sampleTime: Double,
                bus: Int = 0,
                events: UnsafePointer<AURenderEvent>? = nil,
                supplyMemory: Bool = true) -> (status: AUAudioUnitStatus, left: [Float], right: [Float]) {
        for index in 0..<channels {
            list[index] = AudioBuffer(
                mNumberChannels: 1,
                mDataByteSize: UInt32(frames * MemoryLayout<Float>.size),
                mData: supplyMemory ? UnsafeMutableRawPointer(storage[index]) : nil
            )
        }
        let status = invoke(frames: frames, sampleTime: sampleTime, bus: bus, events: events)
        func read(_ index: Int) -> [Float] {
            guard index < list.count, let data = list[index].mData else { return [] }
            return Array(UnsafeBufferPointer(start: data.assumingMemoryBound(to: Float.self), count: frames))
        }
        return (status, read(0), read(min(1, channels - 1)))
    }

    /// The component's render block, so a caller written in C can drive it.
    var renderBlock: AUInternalRenderBlock { block }

    /// The pointers the rig hands the render block, exposed so a measurement
    /// can call something else through exactly the same path.
    var flagsPointer: UnsafeMutablePointer<AudioUnitRenderActionFlags> { flags }
    var timestampPointer: UnsafeMutablePointer<AudioTimeStamp> { timestamp }
    var bufferListPointer: UnsafeMutablePointer<AudioBufferList> { list.unsafeMutablePointer }

    /// Point the buffer list at this rig's own memory. Separate from
    /// `invoke` so an allocation measurement can cover the component's render
    /// block and not the host that set the table.
    func prepare(frames: Int) {
        for index in 0..<channels {
            list[index] = AudioBuffer(
                mNumberChannels: 1,
                mDataByteSize: UInt32(frames * MemoryLayout<Float>.size),
                mData: UnsafeMutableRawPointer(storage[index])
            )
        }
    }

    /// Call the render block and nothing else.
    func invoke(frames: Int,
                sampleTime: Double,
                bus: Int = 0,
                events: UnsafePointer<AURenderEvent>? = nil) -> AUAudioUnitStatus {
        timestamp.pointee.mSampleTime = sampleTime
        timestamp.pointee.mFlags = .sampleTimeValid
        return block(flags, timestamp, AUAudioFrameCount(frames), bus, list.unsafeMutablePointer, events, nil)
    }

    /// Render without reading the result back, for the measurements where
    /// copying the output would be the thing measured.
    @discardableResult
    func renderQuietly(frames: Int, sampleTime: Double, events: UnsafePointer<AURenderEvent>? = nil) -> AUAudioUnitStatus {
        prepare(frames: frames)
        return invoke(frames: frames, sampleTime: sampleTime, events: events)
    }
}

/// A render-event list built by hand, kept alive for as long as the caller
/// holds it.
final class EventList {
    private let storage: UnsafeMutablePointer<AURenderEvent>
    private var count = 0
    private let capacity: Int

    init(capacity: Int = 32) {
        self.capacity = capacity
        storage = UnsafeMutablePointer<AURenderEvent>.allocate(capacity: capacity)
        storage.initialize(repeating: AURenderEvent(), count: capacity)
    }

    deinit {
        storage.deinitialize(count: capacity)
        storage.deallocate()
    }

    func addMIDI(at sampleTime: AUEventSampleTime, _ bytes: (UInt8, UInt8, UInt8)) {
        guard count < capacity else { return }
        storage[count].MIDI.eventSampleTime = sampleTime
        storage[count].MIDI.eventType = .MIDI
        storage[count].MIDI.length = 3
        storage[count].MIDI.cable = 0
        storage[count].MIDI.data = bytes
        link()
    }

    func addParameter(at sampleTime: AUEventSampleTime,
                      address: AUParameterAddress,
                      value: AUValue,
                      ramp: AUAudioFrameCount = 0) {
        guard count < capacity else { return }
        storage[count].parameter.eventSampleTime = sampleTime
        storage[count].parameter.eventType = ramp > 0 ? .parameterRamp : .parameter
        storage[count].parameter.rampDurationSampleFrames = ramp
        storage[count].parameter.parameterAddress = address
        storage[count].parameter.value = value
        link()
    }

    private func link() {
        if count > 0 {
            storage[count - 1].head.next = UnsafeMutablePointer(storage.advanced(by: count))
        }
        count += 1
        storage[count - 1].head.next = nil
    }

    var head: UnsafePointer<AURenderEvent>? {
        count > 0 ? UnsafePointer(storage) : nil
    }
}
