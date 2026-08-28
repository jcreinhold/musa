//  The trial MIDI Processor.
//
//  It is the piece in the host's terms (`06-daw-boundary.md` §3): host
//  transport comes in, scheduled MIDI goes out, and the host routes the
//  result wherever it likes. The schedule is finite and randomly accessed, so
//  a seek is a binary search rather than a replay from the beginning — the
//  distinction prompt 215 exists to prove is available.

import AudioToolbox
import AVFoundation
import Foundation

/// The trial's MIDI Processor, published as `aumi Musa musp`.
public final class TrialProcessorAudioUnit: AUAudioUnit {
    private var schedule: UnsafeMutablePointer<MusaTrialSchedule>
    private var outputBus: AUAudioUnitBus
    private var busses: AUAudioUnitBusArray!

    /// What the last render block did, kept in plain memory rather than in
    /// Swift properties: reading a class property from the render thread
    /// costs reference-count traffic, and this trial is measuring exactly
    /// that kind of cost.
    private let trace: UnsafeMutablePointer<Int64>
    private enum Trace: Int {
        case searchStart = 0
        case emitted = 1
        case missingContext = 2
        static let count = 3
    }

    /// The index the last render block searched to.
    public var lastSearchStart: Int64 { trace[Trace.searchStart.rawValue] }

    /// How many occurrences the last render block emitted.
    public var lastEmitted: Int64 { trace[Trace.emitted.rawValue] }

    /// Whether the host ever offered no musical context. §4 of the boundary
    /// document says a component consumes the host's transport, so the honest
    /// reading of its absence is "emit nothing", not "guess a tempo".
    public var sawMissingContext: Bool { trace[Trace.missingContext.rawValue] != 0 }

    public override init(componentDescription: AudioComponentDescription,
                         options: AudioComponentInstantiationOptions = []) throws {
        schedule = UnsafeMutablePointer<MusaTrialSchedule>.allocate(capacity: 1)
        schedule.initialize(to: MusaTrialSchedule())
        trace = UnsafeMutablePointer<Int64>.allocate(capacity: Trace.count)
        trace.initialize(repeating: 0, count: Trace.count)
        let format = AVAudioFormat(standardFormatWithSampleRate: 48_000, channels: 2)!
        outputBus = try AUAudioUnitBus(format: format)
        try super.init(componentDescription: componentDescription, options: options)
        outputBus.maximumChannelCount = 2
        busses = AUAudioUnitBusArray(audioUnit: self, busType: .output, busses: [outputBus])
        musa_trial_schedule_init(schedule)
        publishTrialPiece()
        maximumFramesToRender = 4_096
    }

    deinit {
        trace.deinitialize(count: Trace.count)
        trace.deallocate()
        schedule.deallocate()
    }

    /// Forget what the last block did, so one experiment cannot read another's
    /// trace.
    public func clearTrace() {
        trace.update(repeating: 0, count: Trace.count)
    }

    public override var outputBusses: AUAudioUnitBusArray { busses }

    public override var inputBusses: AUAudioUnitBusArray {
        AUAudioUnitBusArray(audioUnit: self, busType: .input, busses: [])
    }

    public override var midiOutputNames: [String] { ["Musa trial piece"] }

    /// The finite piece this component projects: four beats, one note each.
    ///
    /// A production processor would publish the checked score here. The trial
    /// publishes something small enough that a wrong reading is visible.
    public func publishTrialPiece() {
        musa_trial_schedule_init(schedule)
        for beat in 0..<8 {
            var event = MusaTrialEvent()
            event.beat = Double(beat) * 0.5
            event.length_beats = 0.25
            event.note = UInt8(60 + beat)
            event.velocity = 100
            _ = musa_trial_schedule_add(schedule, event)
        }
    }

    /// How many occurrences the published piece holds.
    public var occurrenceCount: Int { Int(schedule.pointee.count) }

    /// The index the schedule would search to for a beat, without rendering.
    public func index(forBeat beat: Double) -> UInt32 {
        musa_trial_schedule_lower_bound(schedule, beat)
    }

    public override var internalRenderBlock: AUInternalRenderBlock {
        let schedule = self.schedule
        // Captured once, deliberately: the host is contracted to install its
        // blocks before it fetches this one, and the trial measures whether
        // that ordering actually holds.
        let context = musicalContextBlock
        let transport = transportStateBlock
        let emit = midiOutputEventBlock
        let sampleRate = outputBus.format.sampleRate
        let trace = self.trace
        return { _, _, frameCount, _, outputData, _, _ in
            let list = UnsafeMutableAudioBufferListPointer(outputData)
            for index in 0..<list.count {
                if let data = list[index].mData {
                    memset(data, 0, Int(list[index].mDataByteSize))
                }
            }
            guard let context, let emit else {
                trace[Trace.missingContext.rawValue] = 1
                return noErr
            }
            var tempo = 0.0
            var beat = 0.0
            guard context(&tempo, nil, nil, &beat, nil, nil), tempo > 0 else {
                trace[Trace.missingContext.rawValue] = 1
                return noErr
            }
            var playing = true
            if let transport {
                var flags = AUHostTransportStateFlags(rawValue: 0)
                if transport(&flags, nil, nil, nil) {
                    playing = flags.contains(.moving)
                }
            }
            guard playing else { return noErr }
            let beatsPerFrame = tempo / (60.0 * sampleRate)
            let endBeat = beat + beatsPerFrame * Double(frameCount)
            let start = musa_trial_schedule_lower_bound(schedule, beat)
            var index = start
            var emitted: Int64 = 0
            while index < schedule.pointee.count {
                let event = withUnsafePointer(to: &schedule.pointee.events) { base in
                    base.withMemoryRebound(to: MusaTrialEvent.self, capacity: Int(MUSA_TRIAL_MAX_EVENTS)) {
                        $0[Int(index)]
                    }
                }
                if event.beat >= endBeat { break }
                let offset = AUEventSampleTime((event.beat - beat) / beatsPerFrame)
                // A stack tuple, not an array: an array literal here would
                // allocate on the render thread, and the probe would say so.
                var packet: (UInt8, UInt8, UInt8) = (0x90, event.note & 0x7F, event.velocity & 0x7F)
                _ = withUnsafeMutableBytes(of: &packet) { raw -> OSStatus in
                    guard let base = raw.baseAddress else { return noErr }
                    return emit(offset, 0, 3, base.assumingMemoryBound(to: UInt8.self))
                }
                emitted += 1
                index += 1
            }
            trace[Trace.searchStart.rawValue] = Int64(start)
            trace[Trace.emitted.rawValue] = emitted
            return noErr
        }
    }
}
