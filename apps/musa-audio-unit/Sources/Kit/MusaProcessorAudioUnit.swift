/// The MIDI Processor: one checked Musa piece, scheduled on the host's
/// timeline.
///
/// The Music Device plays MIDI a host sends. This plays *the piece*: the host
/// says where its transport is, and this emits the messages that sound there,
/// through the host's own MIDI output block, to whatever instrument the
/// musician routed it to.
///
/// What makes it a processor rather than a sequencer is that it owns no
/// cursor. `06-daw-boundary.md` §3 gives the host the transport, so every
/// render block is a *query* — two binary searches into an immutable array
/// the control worker built — and a seek to bar 200 costs what a seek to bar
/// 1 costs. There is nothing to replay, so looping, scrubbing, and an offline
/// bounce are not special cases of anything.
///
/// It produces MIDI and nothing else: no instrument is instantiated here, no
/// audio is rendered, and no `.musa` source is written.

import AVFoundation
import AudioToolbox
import Foundation

public final class MusaProcessorAudioUnit: AUAudioUnit {
    /// Where the opened schedule crosses between threads. Its own slot: a
    /// processor and an instrument in one session are two components, and
    /// `Target` says they share no mutable render state.
    private let slot = UnsafeMutablePointer<MusaAuScheduleSlot>.allocate(capacity: 1)
    private let worker = MusaScheduleWorker()

    private var outputBus: AUAudioUnitBus
    private var busses: AUAudioUnitBusArray!

    /// What this component was asked to project. Control side only.
    private var selection: MusaScheduleSelection?
    private var musicIdentity = ""
    private var assetIdentity = ""
    private var pieceName = ""
    /// The parts the projection carries, and what it could not carry.
    public private(set) var parts: [MusaSchedulePart] = []
    public private(set) var projectionLosses: [String] = []
    /// Why this component is silent. Empty when it is not.
    public private(set) var refusal = "no source has been selected"

    /// Security-scoped access, held for as long as the component renders.
    private var accessBookmark: Data?
    private var accessedURL: URL?

    /// The greatest number of notes this component can re-enter at a seek.
    ///
    /// Preallocated, because the query happens in the render block and the
    /// block may not allocate. A piece thicker than this at one instant is a
    /// recorded loss rather than a silent truncation: the library returns the
    /// whole count even when it fills fewer slots, which is how the block
    /// knows to say so.
    public static let reentryCapacity = 128

    /// The re-entry scratch, and the notes this component has told the host
    /// are sounding. Both are plain memory the render block owns.
    private let spans: UnsafeMutablePointer<MusaAuSpan>
    private let sounding: UnsafeMutablePointer<MusaAuSpan>
    private let soundingCount: UnsafeMutablePointer<Int32>

    /// Where the last block ended, so the next one can tell "the transport
    /// moved on" from "the host seeked". Plain memory for the same reason
    /// everything else here is: reading a Swift property from the render
    /// thread is reference-count traffic.
    private let cursor: UnsafeMutablePointer<Double>
    private let flowing: UnsafeMutablePointer<Int32>

    /// What the last render block did. Instrumentation, not music.
    private let trace: UnsafeMutablePointer<Int64>
    private typealias Trace = MusaProcessorTrace

    /// The index the last render block's message search began at.
    public var lastSearchStart: Int64 { trace[Trace.searchStart.rawValue] }
    /// How many messages it emitted.
    public var lastEmitted: Int64 { trace[Trace.emitted.rawValue] }
    /// Whether the host has ever offered no musical context. §4 of the
    /// boundary document gives the host the transport, so the honest reading
    /// of its absence is "emit nothing", never "guess a tempo".
    public var sawMissingContext: Bool { trace[Trace.missingContext.rawValue] != 0 }
    /// How many notes the last block re-entered after a seek.
    public var lastReentered: Int64 { trace[Trace.reentered.rawValue] }
    /// The index the last active-note search began at. Greater than zero
    /// after a seek into the middle is what "no whole-piece scan" looks like
    /// from outside.
    public var lastScanStart: Int64 { trace[Trace.scanStart.rawValue] }
    /// How many sounding notes a seek could not re-enter for want of slots.
    public var reentryOverflow: Int64 { trace[Trace.overflowed.rawValue] }
    /// How many times the host's position has jumped.
    public var discontinuities: Int64 { trace[Trace.discontinuities.rawValue] }

    public override init(
        componentDescription: AudioComponentDescription,
        options: AudioComponentInstantiationOptions = []
    ) throws {
        spans = UnsafeMutablePointer<MusaAuSpan>.allocate(capacity: Self.reentryCapacity)
        sounding = UnsafeMutablePointer<MusaAuSpan>.allocate(capacity: Self.reentryCapacity)
        soundingCount = UnsafeMutablePointer<Int32>.allocate(capacity: 1)
        cursor = UnsafeMutablePointer<Double>.allocate(capacity: 1)
        flowing = UnsafeMutablePointer<Int32>.allocate(capacity: 1)
        trace = UnsafeMutablePointer<Int64>.allocate(capacity: Trace.count)
        guard let format = AVAudioFormat(standardFormatWithSampleRate: 48_000, channels: 2) else {
            throw NSError(domain: NSOSStatusErrorDomain, code: Int(kAudioUnitErr_FormatNotSupported))
        }
        outputBus = try AUAudioUnitBus(format: format)
        try super.init(componentDescription: componentDescription, options: options)
        soundingCount.initialize(to: 0)
        cursor.initialize(to: 0)
        flowing.initialize(to: 0)
        trace.initialize(repeating: 0, count: Trace.count)
        musa_au_schedule_slot_init(slot)
        outputBus.maximumChannelCount = 2
        busses = AUAudioUnitBusArray(audioUnit: self, busType: .output, busses: [outputBus])
        maximumFramesToRender = 4_096
    }

    deinit {
        if let schedule = musa_au_schedule_slot_current(slot) {
            musa_au_schedule_release(schedule)
        }
        accessedURL?.stopAccessingSecurityScopedResource()
        spans.deallocate()
        sounding.deallocate()
        soundingCount.deallocate()
        cursor.deallocate()
        flowing.deallocate()
        trace.deinitialize(count: Trace.count)
        trace.deallocate()
        slot.deallocate()
    }

    public override var outputBusses: AUAudioUnitBusArray { busses }

    public override var inputBusses: AUAudioUnitBusArray {
        AUAudioUnitBusArray(audioUnit: self, busType: .input, busses: [])
    }

    /// One MIDI cable, named for the piece it carries.
    public override var midiOutputNames: [String] {
        [pieceName.isEmpty ? "Musa" : "Musa: \(pieceName)"]
    }

    /// Whether a schedule is installed and ready to be read.
    public var isReady: Bool { musa_au_schedule_slot_current(slot) != nil }

    /// How many messages the installed piece is; zero when there is none.
    public var messageCount: Int {
        guard let schedule = musa_au_schedule_slot_current(slot) else { return 0 }
        return Int(musa_au_schedule_count(schedule))
    }

    /// Where the installed piece ends, in its timeline's own unit.
    public var extent: Double {
        guard let schedule = musa_au_schedule_slot_current(slot) else { return 0 }
        return musa_au_schedule_extent(schedule)
    }

    /// The index the installed piece would search to for a position, without
    /// rendering. What the harness reads to see that a seek is a search.
    public func index(at position: Double) -> UInt32 {
        guard let schedule = musa_au_schedule_slot_current(slot) else { return 0 }
        return musa_au_schedule_lower_bound(schedule, position)
    }

    /// Forget what the last block did, so one experiment cannot read
    /// another's trace.
    public func clearTrace() {
        trace.update(repeating: 0, count: Trace.count)
    }

    /// Open `selection` on the worker and install it when it arrives.
    public func select(_ selection: MusaScheduleSelection, bookmark: Data? = nil) {
        self.selection = selection
        accessBookmark = bookmark
        refusal = "opening \(selection.project)"
        worker.open(selection) { [weak self] result in
            DispatchQueue.main.async { self?.install(result) }
        }
    }

    /// The same, synchronously. For the automated host, which has nothing to
    /// do while it waits and wants the refusal in the same breath.
    @discardableResult
    public func selectAndWait(_ selection: MusaScheduleSelection, bookmark: Data? = nil) -> String {
        self.selection = selection
        accessBookmark = bookmark
        install(MusaScheduleWorker.openNow(selection))
        return refusal
    }

    private func install(_ result: MusaScheduleResult) {
        musicIdentity = result.musicIdentity
        assetIdentity = result.assetIdentity
        refusal = result.refusal
        guard let schedule = result.schedule else { return }
        pieceName = result.pieceName
        parts = result.parts
        projectionLosses = result.losses
        // Publish, then retire what was there: the block must never see the
        // old pointer after it has been freed, and it can only stop seeing it
        // by the publish happening first.
        let previous = musa_au_schedule_slot_publish(slot, schedule)
        worker.retire(previous)
        // A new piece is a new timeline. Whatever the last one had sounding
        // is not sounding in this one, and the next block is a seek.
        soundingCount.pointee = 0
        flowing.pointee = 0
    }

    public override func allocateRenderResources() throws {
        try super.allocateRenderResources()
        soundingCount.pointee = 0
        flowing.pointee = 0
    }

    public override func reset() {
        soundingCount.pointee = 0
        flowing.pointee = 0
        cursor.pointee = 0
    }

    public override var internalRenderBlock: AUInternalRenderBlock {
        let slot = self.slot
        // Captured once, deliberately: the host installs its blocks before it
        // fetches this one, and prompt 215 measured that the ordering holds.
        let context = musicalContextBlock
        let transport = transportStateBlock
        let emit = midiOutputEventBlock
        let sampleRate = outputBus.format.sampleRate
        let spans = self.spans
        let sounding = self.sounding
        let soundingCount = self.soundingCount
        let cursor = self.cursor
        let flowing = self.flowing
        let trace = self.trace
        let capacity = Self.reentryCapacity

        return { _, timestamp, frameCount, _, outputData, _, _ in
            // A MIDI Processor still fills its audio buses, with silence.
            // Leaving them alone would hand the host whatever was there.
            //
            // The `while` is not a style choice. Iterating
            // `0..<list.count` over an `UnsafeMutableAudioBufferListPointer`
            // allocates twice a block, which the probe sees and a host with
            // a real deadline would too.
            let list = UnsafeMutableAudioBufferListPointer(outputData)
            var buffer = 0
            while buffer < list.count {
                if let data = list[buffer].mData {
                    memset(data, 0, Int(list[buffer].mDataByteSize))
                }
                buffer += 1
            }
            trace[Trace.emitted.rawValue] = 0
            trace[Trace.reentered.rawValue] = 0
            trace[Trace.overflowed.rawValue] = 0

            guard let schedule = musa_au_schedule_slot_current(slot), let emit else {
                trace[Trace.missingContext.rawValue] = 1
                return noErr
            }
            let onHost = musa_au_schedule_timeline(schedule) == UInt32(MUSA_AU_TIMELINE_HOST)

            // How wide this block is, in the schedule's own unit. On the
            // host's timeline that needs the host's tempo; on the piece's own
            // it is seconds and the host's tempo is not consulted at all.
            var perFrame = 1.0 / sampleRate
            if onHost {
                var tempo = 0.0
                var beat = 0.0
                guard let context, context(&tempo, nil, nil, &beat, nil, nil), tempo > 0 else {
                    // §6: no grid, no guess. A host that offers no tempo has
                    // not told this component where its quarter notes are.
                    trace[Trace.missingContext.rawValue] = 1
                    return noErr
                }
                perFrame = tempo / (60.0 * sampleRate)
                cursorSeek(cursor, to: beat, flowing: flowing, trace: trace)
            } else {
                var position = timestamp.pointee.mSampleTime / sampleRate
                if let transport {
                    var flags = AUHostTransportStateFlags(rawValue: 0)
                    var samples = 0.0
                    if transport(&flags, &samples, nil, nil) {
                        position = samples / sampleRate
                    }
                }
                cursorSeek(cursor, to: position, flowing: flowing, trace: trace)
            }

            // A stopped transport is silence, and silence means the notes
            // this component said were sounding are not.
            var playing = true
            if let transport {
                var flags = AUHostTransportStateFlags(rawValue: 0)
                if transport(&flags, nil, nil, nil) {
                    playing = flags.contains(.moving)
                }
            }
            guard playing else {
                releaseSounding(sounding, soundingCount, emit)
                flowing.pointee = 0
                return noErr
            }

            let start = cursor.pointee
            let end = start + perFrame * Double(frameCount)

            // A discontinuity — a seek, a loop wrap, the first block after
            // the transport started — is where re-entry happens. Everything
            // this component said was sounding is released, and everything
            // the new position lands inside is attacked.
            if flowing.pointee == 0 {
                releaseSounding(sounding, soundingCount, emit)
                trace[Trace.scanStart.rawValue] = Int64(musa_au_schedule_active_scan_start(schedule, start))
                let found = Int(musa_au_schedule_active(schedule, start, spans, UInt32(capacity)))
                let taken = min(found, capacity)
                for index in 0..<taken {
                    let span = spans[index]
                    emitPacket(emit, 0, span.status, span.note, span.velocity)
                    sounding[index] = span
                }
                soundingCount.pointee = Int32(taken)
                trace[Trace.reentered.rawValue] = Int64(taken)
                trace[Trace.overflowed.rawValue] = Int64(max(0, found - taken))
                flowing.pointee = 1
            }

            let low = musa_au_schedule_lower_bound(schedule, start)
            let high = musa_au_schedule_lower_bound(schedule, end)
            trace[Trace.searchStart.rawValue] = Int64(low)
            var emitted: Int64 = 0
            var index = low
            var event = MusaAuScheduleEvent(position: 0, part: 0, status: 0, data1: 0, data2: 0, reserved: 0)
            while index < high {
                guard musa_au_schedule_event(schedule, index, &event) != 0 else { break }
                let offset = AUEventSampleTime((event.position - start) / perFrame)
                emitPacket(emit, offset, event.status, event.data1, event.data2)
                track(sounding, soundingCount, event)
                emitted += 1
                index += 1
            }
            trace[Trace.emitted.rawValue] = emitted
            cursor.pointee = end
            return noErr
        }
    }

    // MARK: - Document state

    public override var fullState: [String: Any]? {
        get {
            var state = super.fullState ?? [:]
            for (key, value) in musaState() {
                state[key] = value
            }
            return state
        }
        set {
            super.fullState = newValue
            restore(newValue)
        }
    }

    public override var fullStateForDocument: [String: Any]? {
        get {
            var state = super.fullStateForDocument ?? [:]
            for (key, value) in musaState() {
                state[key] = value
            }
            return state
        }
        set {
            super.fullStateForDocument = newValue
            restore(newValue)
        }
    }

    private func musaState() -> [String: Any] {
        var state: [String: Any] = [
            MusaStateKey.version: musaStateVersion,
            MusaStateKey.abiVersion: Int(musa_au_abi_version()),
            MusaStateKey.musicIdentity: musicIdentity,
            MusaStateKey.assetIdentity: assetIdentity,
            MusaStateKey.controlLosses: projectionLosses,
            MusaStateKey.parts: parts.map { ["name": $0.name, "channel": Int($0.channel)] },
            MusaStateKey.refusal: refusal,
        ]
        if let selection {
            state[MusaStateKey.project] = selection.project
            state[MusaStateKey.scheduleMode] = selection.mode.rawValue
            state[MusaStateKey.scheduleTimeline] = selection.timeline.rawValue
            if let piece = selection.piece {
                state[MusaStateKey.piece] = piece
            }
        }
        if let accessBookmark {
            state[MusaStateKey.access] = accessBookmark
        }
        return state
    }

    private func restore(_ state: [String: Any]?) {
        guard let state, let project = state[MusaStateKey.project] as? String else { return }
        if let version = state[MusaStateKey.version] as? Int, version > musaStateVersion {
            refusal = "this state is version \(version); this component writes version \(musaStateVersion)"
            return
        }
        // A word this component does not know is a refusal, not a default:
        // restoring the piece's own timeline for a document that asked for
        // the host's would be the same notes at different moments.
        guard let mode = MusaScheduleMode(rawValue: state[MusaStateKey.scheduleMode] as? String ?? "performance") else {
            refusal = "this state names a reading this component does not know"
            return
        }
        guard let timeline = MusaScheduleTimeline(
            rawValue: state[MusaStateKey.scheduleTimeline] as? String ?? "piece"
        ) else {
            refusal = "this state names a timeline this component does not know"
            return
        }
        var bookmark = state[MusaStateKey.access] as? Data
        var located = project
        if let data = bookmark, let url = resolve(data) {
            located = url.path
        } else if bookmark != nil {
            refusal = "the host restored a bookmark this component could not resolve"
            bookmark = nil
        }
        select(
            MusaScheduleSelection(
                project: located,
                piece: state[MusaStateKey.piece] as? String,
                mode: mode,
                timeline: timeline,
                expectedMusicIdentity: state[MusaStateKey.musicIdentity] as? String,
                expectedAssetIdentity: state[MusaStateKey.assetIdentity] as? String
            ),
            bookmark: bookmark
        )
    }

    private func resolve(_ bookmark: Data) -> URL? {
        var stale = false
        guard let url = try? URL(
            resolvingBookmarkData: bookmark,
            options: [.withSecurityScope],
            relativeTo: nil,
            bookmarkDataIsStale: &stale
        ) else {
            return nil
        }
        accessedURL?.stopAccessingSecurityScopedResource()
        guard url.startAccessingSecurityScopedResource() else { return nil }
        accessedURL = url
        return url
    }
}

/// The slots of the render block's trace array.
///
/// File scope rather than nested in the class: the free functions the block
/// calls write it too, and an index spelled twice is an index that drifts.
enum MusaProcessorTrace: Int {
    case searchStart = 0
    case emitted = 1
    case missingContext = 2
    case reentered = 3
    case scanStart = 4
    case overflowed = 5
    case discontinuities = 6
    static let count = 7
}

/// Note the host's position, and whether it continued from where the last
/// block ended.
///
/// A free function taking pointers, not a method: the render block captures
/// no `self`, and a nested closure would capture the enclosing context.
private func cursorSeek(
    _ cursor: UnsafeMutablePointer<Double>,
    to position: Double,
    flowing: UnsafeMutablePointer<Int32>,
    trace: UnsafeMutablePointer<Int64>
) {
    // One block's worth of drift is continuity; anything else is the host
    // having moved. The tolerance is generous on purpose — a host that
    // rounds its beat position should not read as a seek every block.
    let continued = flowing.pointee != 0 && abs(position - cursor.pointee) < 1e-6
    if !continued {
        flowing.pointee = 0
        let slot = MusaProcessorTrace.discontinuities.rawValue
        trace[slot] = trace[slot] + 1
    }
    cursor.pointee = position
}

/// Emit one three-byte channel message at a sample offset.
private func emitPacket(
    _ emit: AUMIDIOutputEventBlock,
    _ offset: AUEventSampleTime,
    _ status: UInt8,
    _ data1: UInt8,
    _ data2: UInt8
) {
    // A stack tuple, not an array: an array literal here would allocate on
    // the render thread, and the probe would say so.
    var packet: (UInt8, UInt8, UInt8) = (status, data1 & 0x7F, data2 & 0x7F)
    _ = withUnsafeMutableBytes(of: &packet) { raw -> OSStatus in
        guard let base = raw.baseAddress else { return noErr }
        return emit(offset, 0, 3, base.assumingMemoryBound(to: UInt8.self))
    }
}

/// Turn off everything this component told the host was sounding.
/// Release everything this component said was sounding.
///
/// The packet is built and handed over here rather than through
/// `emitPacket`: passing an Objective-C block down a second Swift frame
/// copies it, once per call, on the render thread. `MusaAuDrive.h` records
/// the same trap from the other direction.
private func releaseSounding(
    _ sounding: UnsafeMutablePointer<MusaAuSpan>,
    _ count: UnsafeMutablePointer<Int32>,
    _ emit: AUMIDIOutputEventBlock
) {
    var index = 0
    while index < Int(count.pointee) {
        let span = sounding[index]
        var packet: (UInt8, UInt8, UInt8) = (0x80 | (span.status & 0x0F), span.note & 0x7F, 0)
        _ = withUnsafeMutableBytes(of: &packet) { raw -> OSStatus in
            guard let base = raw.baseAddress else { return noErr }
            return emit(0, 0, 3, base.assumingMemoryBound(to: UInt8.self))
        }
        index += 1
    }
    count.pointee = 0
}

/// Keep the sounding table current with what was just emitted, so the next
/// seek knows what to release.
private func track(
    _ sounding: UnsafeMutablePointer<MusaAuSpan>,
    _ count: UnsafeMutablePointer<Int32>,
    _ event: MusaAuScheduleEvent
) {
    let kind = event.status & 0xF0
    let channel = event.status & 0x0F
    if kind == 0x90 && event.data2 > 0 {
        let index = Int(count.pointee)
        guard index < MusaProcessorAudioUnit.reentryCapacity else { return }
        sounding[index] = MusaAuSpan(
            start: event.position,
            end: event.position,
            part: event.part,
            status: event.status,
            note: event.data1,
            velocity: event.data2,
            reserved: 0
        )
        count.pointee = Int32(index + 1)
    } else if kind == 0x80 || kind == 0x90 {
        var index = Int(count.pointee) - 1
        while index >= 0 {
            if sounding[index].status & 0x0F == channel && sounding[index].note == event.data1 {
                let last = Int(count.pointee) - 1
                sounding[index] = sounding[last]
                count.pointee = Int32(last)
                return
            }
            index -= 1
        }
    }
}
