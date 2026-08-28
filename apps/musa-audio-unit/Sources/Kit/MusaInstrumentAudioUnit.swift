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
    /// The source-declared controls this component publishes as parameters,
    /// the ones it could not, and the table their addresses came from.
    /// Control side only.
    private var controls: [MusaControl] = []
    private var controlLosses: [String] = []
    private var controlTable = ""
    /// The outputs the selected instrument reaches, bus zero first.
    private var outputs: [MusaOutput] = []
    /// A projection whose bus array could not be published yet, because the
    /// host had render resources allocated when it arrived. Apple's contract
    /// is that a bus array changes while deallocated, so it waits.
    private var deferredOutputs: [MusaOutput]?
    /// Why this component is silent. Empty when it is not.
    public private(set) var refusal = "no source has been selected"

    /// Security-scoped access, held for as long as the component renders.
    private var accessBookmark: Data?
    private var accessedURL: URL?

    /// The component's own channel buffers: every projected output, one
    /// stereo pair each, for one block.
    ///
    /// Preallocated for two measured reasons. Prompt 215 found a host handing
    /// the render block an `AudioBufferList` with null `mData` and expecting
    /// the component's memory, so there has to be some. And a multi-output
    /// Audio Unit is asked for one bus per render call at the same timestamp,
    /// so the frame every bus shares has to already exist when the first of
    /// those calls returns.
    private var outputStorage: UnsafeMutablePointer<Float>?
    private var capacity: AUAudioFrameCount = 0
    private var outputChannels: UnsafeMutablePointer<UnsafeMutablePointer<Float>?>
    /// Which buses have been served since the last full render, as a bit per
    /// bus, and how many frames that render produced.
    private let servedBuses = UnsafeMutablePointer<UInt32>.allocate(capacity: 1)
    private let servedFrames = UnsafeMutablePointer<UInt32>.allocate(capacity: 1)
    /// How many buses the last allocation sized the storage for.
    private var allocatedBuses = 1

    /// A parameter change made through the tree rather than scheduled by the
    /// host, waiting for the render thread to pick it up.
    ///
    /// A host automating a parameter sends render events, which are already
    /// sample-accurate. A knob — in a host's generic view, or in the
    /// containing app — arrives through `implementorValueObserver` on
    /// whatever thread turned it, and there is no event list to put it in.
    /// So it is staged: the control side writes the value and then bumps a
    /// counter, and the render block applies it at the start of the next
    /// block when the counter has moved. No lock, no allocation, and no
    /// chance of the render thread reading a value that was never written.
    private let stagedValue: UnsafeMutablePointer<Float>
    private let stagedAddress: UnsafeMutablePointer<UInt64>
    private let stagedGeneration: UnsafeMutablePointer<UInt32>
    private let appliedGeneration: UnsafeMutablePointer<UInt32>
    private let stagedCount = UnsafeMutablePointer<UInt32>.allocate(capacity: 1)

    /// How many controls this component can stage a knob turn for.
    ///
    /// Every admitted control still becomes a parameter and every one of them
    /// is still automatable, because automation is an event and needs no slot
    /// here. This bounds only the staging table, which must be allocated once
    /// and never resized under a live render block. A source declaring more
    /// public controls than this records a projection loss saying so rather
    /// than silently having a knob that does nothing.
    private static let stagingCapacity = 64

    private let outputBus: AUAudioUnitBus
    private var busses: AUAudioUnitBusArray!
    private var storedParameterTree: AUParameterTree?

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
        outputBus.name = "main"
        outputChannels = UnsafeMutablePointer<UnsafeMutablePointer<Float>?>.allocate(
            capacity: Int(MUSA_AU_MAX_OUTPUTS) * 2
        )
        outputChannels.initialize(repeating: nil, count: Int(MUSA_AU_MAX_OUTPUTS) * 2)
        stagedValue = UnsafeMutablePointer<Float>.allocate(capacity: Self.stagingCapacity)
        stagedValue.initialize(repeating: 0, count: Self.stagingCapacity)
        stagedAddress = UnsafeMutablePointer<UInt64>.allocate(capacity: Self.stagingCapacity)
        stagedAddress.initialize(repeating: 0, count: Self.stagingCapacity)
        stagedGeneration = UnsafeMutablePointer<UInt32>.allocate(capacity: Self.stagingCapacity)
        stagedGeneration.initialize(repeating: 0, count: Self.stagingCapacity)
        appliedGeneration = UnsafeMutablePointer<UInt32>.allocate(capacity: Self.stagingCapacity)
        appliedGeneration.initialize(repeating: 0, count: Self.stagingCapacity)
        try super.init(componentDescription: componentDescription, options: options)
        musa_au_slot_init(slot)
        overflow.initialize(to: 0)
        servedBuses.initialize(to: 0)
        servedFrames.initialize(to: 0)
        stagedCount.initialize(to: 0)
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
        outputStorage?.deallocate()
        outputChannels.deallocate()
        servedBuses.deallocate()
        servedFrames.deallocate()
        stagedValue.deallocate()
        stagedAddress.deallocate()
        stagedGeneration.deallocate()
        appliedGeneration.deallocate()
        stagedCount.deallocate()
        accessedURL?.stopAccessingSecurityScopedResource()
    }

    // MARK: - Buses and parameters

    override public var outputBusses: AUAudioUnitBusArray { busses }

    /// The parameters this component publishes: the loaded instrument's
    /// source-declared public controls, and nothing else.
    ///
    /// Rule D2: a plug-in parameter is exactly a source-declared exposed
    /// control, never a DSP node, an internal edge, or a host-side
    /// convenience knob. Every descriptor below was generated from the
    /// declaration — identifier, name, domain, default, and whether a ramp
    /// means anything — so there is no catalogue in Swift to fall out of step
    /// with the source. Before a source is selected the tree is `nil`,
    /// because there is no instrument whose controls these would be.
    override public var parameterTree: AUParameterTree? {
        get { storedParameterTree }
        set { _ = newValue }
    }

    /// The controls this component published, with the sentence each
    /// declaration wrote.
    ///
    /// `AUParameter` has a name and no description, so the summary cannot
    /// ride along in the tree. It is published here and saved in the document
    /// instead, which is what lets an interface be generated from project
    /// facts rather than from a hand-written table.
    public var publishedControls: [MusaControl] { controls }

    /// The declared controls that could not become parameters, each already a
    /// sentence naming which and why (§6: recorded, never implied).
    public var projectionLosses: [String] { controlLosses }

    /// The outputs this component projected, in bus order.
    public var projectedOutputs: [MusaOutput] { outputs }

    /// The address table the published parameters were derived under.
    public var publishedControlTable: String { controlTable }

    /// Build the tree for one projection, and wire a knob turn to the render
    /// thread.
    private func makeParameterTree(_ controls: [MusaControl]) -> AUParameterTree? {
        guard !controls.isEmpty else { return nil }
        let parameters = controls.map { control -> AUParameter in
            var flags: AudioUnitParameterOptions = [.flag_IsReadable, .flag_IsWritable]
            // Only where the declaration says the control is continuous. A
            // per-note control is not a slower continuous one, and telling a
            // host it may ramp one would be this component inventing a
            // smoothing the source never declared.
            if control.continuous {
                flags.insert(.flag_CanRamp)
            }
            let parameter = AUParameterTree.createParameter(
                withIdentifier: control.parameterIdentifier,
                name: control.display,
                address: AUParameterAddress(control.address),
                min: control.minimum,
                max: control.maximum,
                // Generic: the domain is a normalized exact rational in
                // `[0,1]`, which is not decibels, hertz, or a percentage, and
                // labelling it as one of those would be a claim about the
                // mapping that only the instrument makes.
                unit: .generic,
                unitName: nil,
                flags: flags,
                valueStrings: nil,
                dependentParameters: nil
            )
            parameter.value = control.defaultValue
            return parameter
        }
        let tree = AUParameterTree.createTree(withChildren: parameters)
        tree.implementorValueObserver = { [weak self] parameter, value in
            self?.stage(parameter.address, value)
        }
        // No `implementorValueProvider`: the tree already caches what was
        // last set, and a provider that read `parameter.value` to answer
        // would be calling itself.
        return tree
    }

    /// Hand the render thread a value a knob just produced.
    private func stage(_ address: AUParameterAddress, _ value: AUValue) {
        let staged = Int(stagedCount.pointee)
        var index = 0
        while index < staged {
            if stagedAddress[index] == UInt64(address) {
                stagedValue[index] = value
                // The value first, then the counter. The render thread reads
                // them in the other order, so a counter it has seen move is a
                // value that was already written.
                stagedGeneration[index] = stagedGeneration[index] &+ 1
                return
            }
            index += 1
        }
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
        controls = result.controls
        controlLosses = result.controlLosses
        controlTable = result.controlTable
        // The table this preparation settled on is what the next one must be
        // given, so a restart keeps every address a host automated against.
        selection?.controlTable = result.controlTable
        if controls.count > Self.stagingCapacity {
            controlLosses.append(
                "\(controls.count) declared controls is more than this component can stage a knob turn for "
                    + "(\(Self.stagingCapacity)); every one is still automatable by the host"
            )
        }
        publishStaging(controls)
        storedParameterTree = makeParameterTree(controls)
        publishOutputs(result.outputs)
        // Publish, then retire what was there. In that order: the block must
        // never see the old pointer after it has been freed, and it can only
        // stop seeing it by the publish happening first.
        let previous = musa_au_slot_publish(slot, instrument)
        preparer.retire(previous)
    }

    /// Give the staging table this projection's addresses.
    ///
    /// Called on the control side while the render block may be running. The
    /// count is written last and lowered first, so the render thread never
    /// reads a slot whose address has not been written.
    private func publishStaging(_ controls: [MusaControl]) {
        stagedCount.pointee = 0
        let staged = min(controls.count, Self.stagingCapacity)
        var index = 0
        while index < staged {
            stagedAddress[index] = controls[index].address
            stagedValue[index] = controls[index].defaultValue
            // Equal generations mean "nothing to apply": preparation already
            // put every declared default where it belongs, and re-applying
            // one here would overwrite a value the instrument's own
            // implementation wrote.
            stagedGeneration[index] = 0
            appliedGeneration[index] = 0
            index += 1
        }
        stagedCount.pointee = UInt32(staged)
    }

    /// Publish the projected outputs as host buses.
    ///
    /// Bus zero is the piece's main output and never moves. The rest are the
    /// declared points this part reaches, under the names the source gave
    /// them: `06-daw-boundary.md` §3 keeps a part, an instrument, and a mixer
    /// track distinct, so nothing here is renamed to a workstation's word.
    private func publishOutputs(_ projected: [MusaOutput]) {
        guard projected.map(\.name) != outputs.map(\.name) else { return }
        guard !renderResourcesAllocated else {
            // Apple's contract: a bus array changes while deallocated. The
            // host will deallocate before it renders the new selection.
            deferredOutputs = projected
            return
        }
        deferredOutputs = nil
        outputs = projected
        let extra = projected.dropFirst().prefix(Int(MUSA_AU_MAX_OUTPUTS) - 1)
        outputBus.name = projected.first?.name ?? "main"
        var made: [AUAudioUnitBus] = [outputBus]
        for output in extra {
            guard let format = AVAudioFormat(standardFormatWithSampleRate: outputBus.format.sampleRate, channels: 2),
                  let bus = try? AUAudioUnitBus(format: format)
            else {
                continue
            }
            bus.maximumChannelCount = 2
            bus.name = output.name
            made.append(bus)
        }
        willChangeValue(forKey: "outputBusses")
        busses = AUAudioUnitBusArray(audioUnit: self, busType: .output, busses: made)
        didChangeValue(forKey: "outputBusses")
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
        if let deferred = deferredOutputs {
            publishOutputs(deferred)
        }
        try super.allocateRenderResources()
        // Only ever grows. `auval` lowers `maximumFramesToRender` for its
        // 22050 Hz pass and then renders 137 frames at 96000 without raising
        // it again; a component whose buffers shrank to match would answer
        // that with `kAudioUnitErr_TooManyFramesToProcess` and fail
        // validation. Holding the high-water mark costs one buffer and is
        // what every host actually expects.
        let frames = max(capacity, maximumFramesToRender)
        let wanted = max(1, min(busses?.count ?? 1, Int(MUSA_AU_MAX_OUTPUTS)))
        if outputStorage == nil || allocatedBuses != wanted || capacity < frames {
            outputStorage?.deallocate()
            let floats = wanted * 2 * Int(frames)
            let storage = UnsafeMutablePointer<Float>.allocate(capacity: floats)
            storage.initialize(repeating: 0, count: floats)
            outputStorage = storage
            allocatedBuses = wanted
            capacity = frames
            var channel = 0
            while channel < wanted * 2 {
                outputChannels[channel] = storage.advanced(by: channel * Int(frames))
                channel += 1
            }
        }
        servedBuses.pointee = 0
        servedFrames.pointee = 0
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
        // What was rendered for the buses a host has not collected yet is
        // about a state this component no longer has. Forgetting it makes the
        // next call a fresh render rather than a stale frame.
        servedBuses.pointee = 0
    }

    // MARK: - Render

    override public var internalRenderBlock: AUInternalRenderBlock {
        // Captured by value, and every one of them is a raw pointer or a
        // number. Capturing `self` would put ARC traffic on the render
        // thread, which is the same class of mistake as allocating on it.
        let slot = self.slot
        let scratch = self.eventBuffer
        let overflow = self.overflow
        let capacity = self.capacity
        let eventCapacity = Self.eventCapacity
        let channels = self.outputChannels
        let buses = self.allocatedBuses
        let served = self.servedBuses
        let servedFrames = self.servedFrames
        let stagedValue = self.stagedValue
        let stagedAddress = self.stagedAddress
        let stagedGeneration = self.stagedGeneration
        let appliedGeneration = self.appliedGeneration
        let stagedCount = self.stagedCount

        return { _, _, frameCount, outputBusNumber, outputData, events, _ in
            let frames = Int(frameCount)
            guard let scratch, frames <= Int(capacity) else {
                return kAudioUnitErr_TooManyFramesToProcess
            }
            let bus = Int(outputBusNumber)
            guard bus < buses, let left = channels[bus * 2], let right = channels[bus * 2 + 1] else {
                return kAudioUnitErr_InvalidParameter
            }
            let buffers = UnsafeMutableAudioBufferListPointer(outputData)

            guard let instrument = musa_au_slot_current(slot) else {
                // Not ready is silence — never stale memory, and never a
                // failure code. A host that has just restored a document is
                // entitled to a few hundred milliseconds of quiet while the
                // worker compiles, and an error here would make it think the
                // component was broken.
                memset(UnsafeMutableRawPointer(left), 0, frames * MemoryLayout<Float>.size)
                memset(UnsafeMutableRawPointer(right), 0, frames * MemoryLayout<Float>.size)
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

            // A multi-output component is asked for one bus per call at the
            // same timestamp, so the cycle's whole frame is produced on the
            // first of those calls and the rest are reads of it. A bus asked
            // for twice, or a different frame count, is a new cycle: that is
            // what makes a host taking only bus zero render every block
            // rather than repeat one.
            let mask: UInt32 = bus < 32 ? (1 << UInt32(bus)) : 0
            let fresh = served.pointee == 0 || (served.pointee & mask) != 0 || servedFrames.pointee != frameCount
            if fresh {
                var count = 0
                // Knob turns first, at the start of the block, before
                // anything the host scheduled inside it.
                var staged = 0
                let stagedTotal = Int(stagedCount.pointee)
                while staged < stagedTotal, count < eventCapacity {
                    let generation = stagedGeneration[staged]
                    if generation != appliedGeneration[staged] {
                        appliedGeneration[staged] = generation
                        scratch[count] = musaParameterEvent(
                            frame: 0,
                            address: stagedAddress[staged],
                            value: stagedValue[staged],
                            ramp: 0
                        )
                        count += 1
                    }
                    staged += 1
                }

                var event = events
                while let current = event {
                    let kind = current.pointee.head.eventType
                    if kind == .MIDI {
                        if count < eventCapacity, let decoded = musaDecodeMIDI(current.pointee.MIDI, frameCount) {
                            scratch[count] = decoded
                            count += 1
                        } else if count >= eventCapacity {
                            overflow.pointee = overflow.pointee &+ 1
                        }
                    } else if kind == .parameter || kind == .parameterRamp {
                        if count < eventCapacity {
                            let parameter = current.pointee.parameter
                            let time = parameter.eventSampleTime
                            let offset = time < 0 ? 0 : UInt32(min(time, Int64(frameCount)))
                            scratch[count] = musaParameterEvent(
                                frame: offset,
                                address: parameter.parameterAddress,
                                value: parameter.value,
                                ramp: parameter.rampDurationSampleFrames
                            )
                            count += 1
                        } else {
                            overflow.pointee = overflow.pointee &+ 1
                        }
                    }
                    event = UnsafePointer(current.pointee.head.next)
                }

                musa_au_render_outputs(
                    instrument,
                    scratch,
                    UInt32(count),
                    channels,
                    UInt32(buses * 2),
                    frameCount
                )
                served.pointee = mask
                servedFrames.pointee = frameCount
            } else {
                served.pointee |= mask
            }

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
            MusaStateKey.controlTable: controlTable,
            MusaStateKey.controlLosses: controlLosses,
            MusaStateKey.outputs: outputs.map(\.name),
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
        // A document from a version this component has never seen cannot be
        // read half-way: its keys may mean something else. An older one can,
        // and does — version 1 named a source and a part and published no
        // parameters, so it has no addresses to preserve and the projection
        // simply derives them.
        if let version = state[MusaStateKey.version] as? Int, version > musaStateVersion {
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
                expectedAssetIdentity: state[MusaStateKey.assetIdentity] as? String,
                // The saved table, so every address a host automated against
                // comes back pointing at the control it was written for.
                controlTable: (state[MusaStateKey.controlTable] as? String).flatMap { $0.isEmpty ? nil : $0 }
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
