/// The Music Device: one checked Musa instrument, rendered from host MIDI.
///
/// What this class is *not* is as load-bearing as what it is. It holds no
/// musical knowledge, keeps no second copy of the score, and never advances a
/// composition: `06-daw-boundary.md` §3 gives the host the timeline, and a
/// Music Device that sought a piece would be claiming otherwise.
///
/// Four things here are measured requirements from prompt 215 rather than
/// taste, and each says so where it is:
///
/// 1. the initializer touches no filesystem;
/// 2. `allocateRenderResources` preallocates this component's own buffers;
/// 3. `fullState` extends `super.fullState` rather than replacing it;
/// 4. the render block captures raw pointers only — no `self`, no Swift
///    object, nothing reference-counted.

import AVFoundation
import AudioToolbox
import Foundation

public final class MusaInstrumentAudioUnit: AUAudioUnit {
    /// Where the prepared instrument crosses between threads.
    private let slot = UnsafeMutablePointer<MusaAuSlot>.allocate(capacity: 1)
    /// The control worker.
    private let preparer = MusaPreparer()

    /// What this component was asked to render. Control side only.
    private var selection: MusaSelection?
    /// What the last preparation produced, for the saved document and for the
    /// containing app's diagnostics. Control side only.
    private var musicIdentity = ""
    private var assetIdentity = ""
    private var inputs: [String] = []
    /// Why this component is silent. Empty when it is not.
    public private(set) var refusal = "no source has been selected"

    /// Security-scoped access, held for as long as the component renders.
    private var accessBookmark: Data?
    private var accessedURL: URL?

    /// The component's own channel buffers.
    ///
    /// Preallocated because prompt 215 measured a host handing the render
    /// block an `AudioBufferList` with null `mData` and expecting the
    /// component's memory. Allocating them in the block is the version of
    /// this that works until the first time it does not.
    private var left: UnsafeMutablePointer<Float>?
    private var right: UnsafeMutablePointer<Float>?
    private var capacity: AUAudioFrameCount = 0

    private let outputBus: AUAudioUnitBus
    private var busses: AUAudioUnitBusArray!

    /// Where the host's event list is decoded into. Preallocated for the same
    /// reason the channel buffers are: a block that built an array would
    /// allocate once per render.
    private var eventBuffer: UnsafeMutablePointer<MusaAuEvent>?
    /// How many events beyond `eventCapacity` the host has sent. Read by the
    /// containing app; written by the block, which is the only writer.
    private let overflow = UnsafeMutablePointer<UInt32>.allocate(capacity: 1)

    /// One block's worth of events. Generous: Apple's own limit on a render
    /// block is the frame count, and a host sending more MIDI messages than
    /// this in one block is doing something a component cannot render
    /// meaningfully anyway.
    private static let eventCapacity = 1024

    override public init(
        componentDescription: AudioComponentDescription,
        options: AudioComponentInstantiationOptions = []
    ) throws {
        // No filesystem here. Prompt 215 measured an eager read in an
        // extension's initializer making the component undiscoverable — the
        // host's instantiation callback never arrives at all, with no crash
        // and nothing in the log. Everything this component knows, it learns
        // from restored state or from the containing app.
        let format = AVAudioFormat(standardFormatWithSampleRate: 48_000, channels: 2)!
        outputBus = try AUAudioUnitBus(format: format)
        outputBus.maximumChannelCount = 2
        try super.init(componentDescription: componentDescription, options: options)
        musa_au_slot_init(slot)
        overflow.initialize(to: 0)
        eventBuffer = UnsafeMutablePointer<MusaAuEvent>.allocate(capacity: Self.eventCapacity)
        busses = AUAudioUnitBusArray(audioUnit: self, busType: .output, busses: [outputBus])
        maximumFramesToRender = 4096
    }

    deinit {
        if let instrument = musa_au_slot_current(slot) {
            musa_au_instrument_release(instrument)
        }
        slot.deallocate()
        overflow.deallocate()
        eventBuffer?.deallocate()
        left?.deallocate()
        right?.deallocate()
        accessedURL?.stopAccessingSecurityScopedResource()
    }

    // MARK: - Buses and parameters

    override public var outputBusses: AUAudioUnitBusArray { busses }

    /// This component publishes no parameters, and that is a decision.
    ///
    /// Rule D2: a plug-in parameter is exactly a source-declared exposed
    /// control, never a DSP node. Projecting those controls — with stable
    /// addresses, dependent kinds, and precise losses — is prompt 217's whole
    /// Target. Inventing a `gain` here so the tree is not empty would break
    /// D2 to make a screenshot look finished, so the tree is empty until 217
    /// fills it from the instrument's own signature.
    override public var parameterTree: AUParameterTree? {
        get { nil }
        set { _ = newValue }
    }

    override public var canProcessInPlace: Bool { false }

    /// How long this component may still sound after the host stops sending
    /// it anything.
    ///
    /// A stated bound rather than a measurement. `musa-dsp` prepares a graph
    /// whose decay is a property of the studio the source declares, and it
    /// publishes no length for it; a component that invented a number from
    /// the instrument it happened to load would be reporting a different tail
    /// per document. So this is an upper bound the automated host checks
    /// against what the component actually does — `render.tail` renders past
    /// it and asserts silence — and a host that honors it never truncates a
    /// release.
    override public var tailTime: TimeInterval { Self.declaredTailTime }

    /// The bound above. Two seconds is longer than any release the prepared
    /// path produces and short enough that a host's offline bounce does not
    /// grow noticeably.
    public static let declaredTailTime: TimeInterval = 2.0

    // MARK: - Selection

    /// Point this component at a part of a piece, and prepare it.
    ///
    /// Control side. Returns immediately; the component stays silent and
    /// says why until the worker publishes.
    public func select(_ selection: MusaSelection, bookmark: Data? = nil) {
        self.selection = selection
        accessBookmark = bookmark
        refusal = "preparing \(selection.part)"
        var request = selection
        request.sampleRate = outputBus.format.sampleRate
        preparer.prepare(request) { [weak self] result in
            DispatchQueue.main.async { self?.install(result) }
        }
    }

    /// Prepare synchronously. For the automated host, which has nothing to do
    /// while it waits and wants the failure in the same breath.
    @discardableResult
    public func selectAndWait(_ selection: MusaSelection, bookmark: Data? = nil) -> String {
        self.selection = selection
        accessBookmark = bookmark
        var request = selection
        request.sampleRate = outputBus.format.sampleRate
        install(MusaPreparer.prepareNow(request))
        return refusal
    }

    private func install(_ result: MusaPreparationResult) {
        musicIdentity = result.musicIdentity
        assetIdentity = result.assetIdentity
        inputs = result.inputs
        refusal = result.refusal
        guard let instrument = result.instrument else { return }
        // Publish, then retire what was there. In that order: the block must
        // never see the old pointer after it has been freed, and it can only
        // stop seeing it by the publish happening first.
        let previous = musa_au_slot_publish(slot, instrument)
        preparer.retire(previous)
    }

    /// Whether this component has something to render.
    public var isReady: Bool { musa_au_slot_current(slot) != nil }

    /// The MIDI dimensions the selected instrument's source binds.
    ///
    /// Everything else a host sends is a loss and is counted rather than
    /// guessed at; `unboundEvents` is the running count.
    public var boundInputs: [String] { inputs }

    /// How many host events the instrument's source bound nothing for.
    public var unboundEvents: UInt32 {
        guard let instrument = musa_au_slot_current(slot) else { return 0 }
        return musa_au_unbound_events(instrument)
    }

    // MARK: - Resources

    override public func allocateRenderResources() throws {
        try super.allocateRenderResources()
        let frames = maximumFramesToRender
        if capacity < frames || left == nil {
            left?.deallocate()
            right?.deallocate()
            left = UnsafeMutablePointer<Float>.allocate(capacity: Int(frames))
            right = UnsafeMutablePointer<Float>.allocate(capacity: Int(frames))
            capacity = frames
        }
        left?.initialize(repeating: 0, count: Int(capacity))
        right?.initialize(repeating: 0, count: Int(capacity))
        // The host may have negotiated a different rate than the one this
        // instrument was prepared for. Preparation is exact for one rate
        // (§4), so this is a new preparation, not a resample.
        if let selection, abs(selection.sampleRate - outputBus.format.sampleRate) > 0.5 {
            select(selection, bookmark: accessBookmark)
        }
    }

    override public func deallocateRenderResources() {
        super.deallocateRenderResources()
    }

    override public func reset() {
        if let instrument = musa_au_slot_current(slot) {
            musa_au_reset(instrument)
        }
    }

    // MARK: - Render

    override public var internalRenderBlock: AUInternalRenderBlock {
        // Captured by value, and every one of them is a raw pointer or a
        // number. Capturing `self` would put ARC traffic on the render
        // thread, which is the same class of mistake as allocating on it.
        let slot = self.slot
        let left = self.left
        let right = self.right
        let scratch = self.eventBuffer
        let overflow = self.overflow
        let capacity = self.capacity
        let eventCapacity = Self.eventCapacity

        return { _, _, frameCount, _, outputData, events, _ in
            let frames = Int(frameCount)
            guard let left, let right, let scratch, frames <= Int(capacity) else {
                return kAudioUnitErr_TooManyFramesToProcess
            }
            let buffers = UnsafeMutableAudioBufferListPointer(outputData)

            guard let instrument = musa_au_slot_current(slot) else {
                // Not ready is silence — never stale memory, and never a
                // failure code. A host that has just restored a document is
                // entitled to a few hundred milliseconds of quiet while the
                // worker compiles, and an error here would make it think the
                // component was broken.
                var index = 0
                while index < buffers.count {
                    let buffer = buffers[index]
                    if let data = buffer.mData {
                        memset(data, 0, Int(buffer.mDataByteSize))
                    }
                    index += 1
                }
                return noErr
            }

            var count = 0
            var event = events
            while let current = event {
                if current.pointee.head.eventType == .MIDI, count < eventCapacity {
                    let midi = current.pointee.MIDI
                    if let decoded = musaDecodeMIDI(midi, frameCount) {
                        scratch[count] = decoded
                        count += 1
                    }
                } else if current.pointee.head.eventType == .MIDI {
                    overflow.pointee = overflow.pointee &+ 1
                }
                event = UnsafePointer(current.pointee.head.next)
            }

            musa_au_render(instrument, scratch, UInt32(count), left, right, frameCount)

            // Fill whatever the host gave us, and hand it our own memory when
            // it gave us none. Prompt 215 measured a host doing exactly that.
            // A `while` over an index rather than `for index in 0..<count`.
            // Prompt 216 measured the second one allocating once per
            // iteration in an unoptimized build — Swift's range iterator is
            // not specialized at `-Onone` — and a render block that is
            // real-time only in Release is not real-time.
            let bytes = frames * MemoryLayout<Float>.size
            var channel = 0
            while channel < buffers.count {
                let source = channel == 0 ? left : right
                if let destination = buffers[channel].mData {
                    destination.copyMemory(from: UnsafeRawPointer(source), byteCount: bytes)
                } else {
                    buffers[channel].mData = UnsafeMutableRawPointer(source)
                    buffers[channel].mDataByteSize = UInt32(bytes)
                }
                channel += 1
            }
            return noErr
        }
    }

    /// How many MIDI messages this component has had to drop, saturating.
    public var droppedEvents: UInt32 { overflow.pointee }

    // MARK: - State

    /// The saved document.
    ///
    /// Built onto `super.fullState`, never in place of it. Prompt 215
    /// measured `auval` failing with `Class Data does not have required
    /// field:<type> == componentType` when the base class's keys were
    /// discarded, and a component that fails validation is a component no
    /// host will load.
    override public var fullState: [String: Any]? {
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

    /// The same, for a host saving its own document.
    ///
    /// Identical content here: everything Musa saves is an identity or a
    /// path, all of it plist-safe, and none of it is asset bytes. A preset
    /// and a project both need exactly this to name the same piece again.
    override public var fullStateForDocument: [String: Any]? {
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
            MusaStateKey.inputs: inputs,
            MusaStateKey.refusal: refusal,
        ]
        if let selection {
            state[MusaStateKey.project] = selection.project
            state[MusaStateKey.part] = selection.part
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
        guard let state, let project = state[MusaStateKey.project] as? String,
              let part = state[MusaStateKey.part] as? String
        else {
            return
        }
        if let version = state[MusaStateKey.version] as? Int, version != musaStateVersion {
            refusal = "this state is version \(version); this component writes version \(musaStateVersion)"
            return
        }
        // Access arrives with the state rather than being looked up.
        var bookmark = state[MusaStateKey.access] as? Data
        var located = project
        if let data = bookmark, let url = resolve(data) {
            located = url.path
        } else if bookmark != nil {
            refusal = "the host restored a bookmark this component could not resolve"
            bookmark = nil
        }
        select(
            MusaSelection(
                project: located,
                piece: state[MusaStateKey.piece] as? String,
                part: part,
                sampleRate: outputBus.format.sampleRate,
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
