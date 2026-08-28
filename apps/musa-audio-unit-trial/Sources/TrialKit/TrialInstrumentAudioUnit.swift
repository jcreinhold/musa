//  The trial Music Device.
//
//  It is the instrument in the host's terms (`06-daw-boundary.md` §3): host
//  MIDI comes in, one stereo signal goes out, and it never seeks a whole
//  composition. Everything the render block touches is preallocated before
//  the first block and read through a raw pointer, so no ARC traffic, no
//  Objective-C message send, and no Swift metadata lookup happens where the
//  boundary document (§7) says none may.

import AudioToolbox
import AVFoundation
import Foundation

/// The trial's Music Device, published as `aumu Musa musi`.
public final class TrialInstrumentAudioUnit: AUAudioUnit {
    /// Every parameter the component publishes, with the source identity it
    /// is a projection of. Rule D2: a plug-in parameter is exactly a
    /// source-declared exposed control, never a DSP node.
    public enum Address: AUParameterAddress, CaseIterable {
        case gain = 0
        case detune = 1
        case brightness = 2

        var identifier: String {
            switch self {
            case .gain: return "gain"
            case .detune: return "detune"
            case .brightness: return "brightness"
            }
        }

        var sourceIdentity: String { "trial.instrument.control.\(identifier)" }
    }

    private var state: UnsafeMutablePointer<MusaTrialInstrument>
    private var channels: [UnsafeMutablePointer<Float>] = []
    private var allocatedFrames: AUAudioFrameCount = 0
    private var mainBus: AUAudioUnitBus
    private var auxiliaryBus: AUAudioUnitBus
    private var busses: AUAudioUnitBusArray!
    private var values: [AUParameterAddress: AUValue] = [.init(0): 1.0, .init(1): 0.0, .init(2): 0.5]

    /// The closure this component was made from. A restore that names a
    /// different one is refused rather than approximated.
    public var sourceIdentity = "trial.source.0"
    public var projectIdentity = "trial.project.0"
    public var lockIdentity = "trial.lock.0"
    public var selectedDeclaration = "instrument trial"
    public var sourceClosure = "instrument trial = saw\n"

    /// Why the last restore was refused, or `nil` when it was accepted.
    public private(set) var lastRestoreRefusal: String?

    /// Where this component was told its assets live, as its own sandbox
    /// sees it. Lazily, because a component's initializer is the one place
    /// that must not go looking at the filesystem: doing this eagerly stops
    /// the host from ever receiving the component at all.
    public private(set) lazy var assetStore: String = describeTrialAssetStore()

    /// How many times a staged plan was taken by the render block. Read by
    /// the harness to prove a parameter change crossed without a lock.
    public var appliedPlans: UInt64 { UInt64(applied) }
    private var applied: UInt32 = 0

    public override init(componentDescription: AudioComponentDescription,
                         options: AudioComponentInstantiationOptions = []) throws {
        state = UnsafeMutablePointer<MusaTrialInstrument>.allocate(capacity: 1)
        state.initialize(to: MusaTrialInstrument())
        let format = AVAudioFormat(standardFormatWithSampleRate: 48_000, channels: 2)!
        mainBus = try AUAudioUnitBus(format: format)
        auxiliaryBus = try AUAudioUnitBus(format: format)
        try super.init(componentDescription: componentDescription, options: options)
        mainBus.maximumChannelCount = 2
        auxiliaryBus.maximumChannelCount = 2
        busses = AUAudioUnitBusArray(audioUnit: self, busType: .output, busses: [mainBus, auxiliaryBus])
        musa_trial_instrument_init(state, format.sampleRate)
        maximumFramesToRender = 4_096
        installParameterTree(withBrightness: false)
    }

    deinit {
        releaseChannels()
        state.deallocate()
    }

    public override var outputBusses: AUAudioUnitBusArray { busses }

    /// A Music Device has no audio input; it is driven by MIDI.
    public override var inputBusses: AUAudioUnitBusArray {
        AUAudioUnitBusArray(audioUnit: self, busType: .input, busses: [])
    }

    public override var virtualMIDICableCount: Int { 1 }

    public override var canProcessInPlace: Bool { false }

    // MARK: - Parameters

    /// Replace the published tree while the component is loaded.
    ///
    /// A host is expected to notice through KVO. The trial exercises it
    /// because Musa's tree is a projection of source that the composer can
    /// edit while the component is loaded, so replacement is not an edge case
    /// for us — it is the ordinary path.
    public func installParameterTree(withBrightness: Bool) {
        let published: [Address] = withBrightness ? [.gain, .detune, .brightness] : [.gain, .detune]
        let parameters = published.map { address -> AUParameter in
            let range: (AUValue, AUValue, AudioUnitParameterUnit) = {
                switch address {
                case .gain: return (0.0, 1.0, .linearGain)
                case .detune: return (-100.0, 100.0, .cents)
                case .brightness: return (0.0, 1.0, .generic)
                }
            }()
            let parameter = AUParameterTree.createParameter(
                withIdentifier: address.identifier,
                name: address.identifier,
                address: address.rawValue,
                min: range.0,
                max: range.1,
                unit: range.2,
                unitName: nil,
                flags: [.flag_IsReadable, .flag_IsWritable, .flag_CanRamp],
                valueStrings: nil,
                dependentParameters: nil
            )
            parameter.value = values[address.rawValue] ?? 0
            return parameter
        }
        let tree = AUParameterTree.createTree(withChildren: parameters)
        tree.implementorValueObserver = { [weak self] parameter, value in
            self?.record(address: parameter.address, value: value)
        }
        tree.implementorValueProvider = { [weak self] parameter in
            self?.values[parameter.address] ?? 0
        }
        parameterTree = tree
    }

    private func record(address: AUParameterAddress, value: AUValue) {
        values[address] = value
        stagePlan()
    }

    /// Prepare a plan on the control side and hand it over wait-free.
    private func stagePlan() {
        var plan = MusaTrialPlan()
        musa_trial_plan_prepare(&plan,
                                mainBus.format.sampleRate,
                                values[Address.gain.rawValue] ?? 1.0,
                                values[Address.detune.rawValue] ?? 0.0)
        musa_trial_instrument_stage(state, &plan)
    }

    // MARK: - Resources

    public override func allocateRenderResources() throws {
        try super.allocateRenderResources()
        releaseChannels()
        allocatedFrames = maximumFramesToRender
        channels = (0..<2).map { _ -> UnsafeMutablePointer<Float> in
            let buffer = UnsafeMutablePointer<Float>.allocate(capacity: Int(allocatedFrames))
            buffer.initialize(repeating: 0, count: Int(allocatedFrames))
            return buffer
        }
        musa_trial_instrument_init(state, mainBus.format.sampleRate)
        stagePlan()
        _ = musa_trial_instrument_apply_staged(state)
    }

    public override func deallocateRenderResources() {
        releaseChannels()
        super.deallocateRenderResources()
    }

    private func releaseChannels() {
        for buffer in channels {
            buffer.deinitialize(count: Int(allocatedFrames))
            buffer.deallocate()
        }
        channels = []
    }

    public override func reset() {
        musa_trial_instrument_reset(state)
    }

    // MARK: - Rendering

    public override var internalRenderBlock: AUInternalRenderBlock {
        let state = self.state
        let left = channels.first
        let right = channels.count > 1 ? channels[1] : nil
        return { actionFlags, _, frameCount, outputBusNumber, outputData, events, _ in
            _ = actionFlags
            _ = musa_trial_instrument_apply_staged(state)
            var event = events
            while let head = event {
                Self.consume(head.pointee, into: state)
                event = UnsafePointer(head.pointee.head.next)
            }
            let list = UnsafeMutableAudioBufferListPointer(outputData)
            // A host may hand over a buffer list with no memory in it and
            // expect the component to supply its own. Both shapes are
            // exercised, because Logic uses the second.
            if list.count > 0, list[0].mData == nil, let left, let right {
                list[0].mData = UnsafeMutableRawPointer(left)
                list[0].mDataByteSize = frameCount * UInt32(MemoryLayout<Float>.size)
                if list.count > 1 {
                    list[1].mData = UnsafeMutableRawPointer(right)
                    list[1].mDataByteSize = frameCount * UInt32(MemoryLayout<Float>.size)
                }
            }
            guard list.count > 0, let first = list[0].mData else { return kAudioUnitErr_NoConnection }
            let outLeft = first.assumingMemoryBound(to: Float.self)
            let outRight = list.count > 1 ? list[1].mData?.assumingMemoryBound(to: Float.self) ?? outLeft : outLeft
            musa_trial_instrument_render(state, outLeft, outRight, frameCount)
            // The auxiliary bus is the same signal at half scale, which is
            // enough to prove the host can pull two buses of one component
            // without either changing the other.
            if outputBusNumber == 1 {
                for frame in 0..<Int(frameCount) {
                    outLeft[frame] *= 0.5
                    outRight[frame] *= 0.5
                }
            }
            return noErr
        }
    }

    /// Consume one host render event. Real-time safe by construction: every
    /// branch reads the event and writes preallocated state.
    private static func consume(_ event: AURenderEvent, into state: UnsafeMutablePointer<MusaTrialInstrument>) {
        switch event.head.eventType {
        case .MIDI:
            var midi = event.MIDI
            withUnsafeBytes(of: &midi.data) { bytes in
                guard bytes.count >= 3 else { return }
                let status = bytes[0] & 0xF0
                let note = bytes[1] & 0x7F
                let velocity = bytes[2] & 0x7F
                if status == 0x90, velocity > 0 {
                    musa_trial_instrument_note_on(state, note, velocity)
                } else if status == 0x80 || (status == 0x90 && velocity == 0) {
                    musa_trial_instrument_note_off(state, note)
                }
            }
        case .parameter, .parameterRamp:
            let parameter = event.parameter
            var plan = state.pointee.plan
            if parameter.parameterAddress == Address.gain.rawValue {
                plan.gain = parameter.value
                state.pointee.plan = plan
            } else if parameter.parameterAddress == Address.detune.rawValue {
                // Detuning needs the note table recomputed, which is control-
                // side work. The render block records the request and leaves
                // the arithmetic to whoever staged the next plan.
                plan.detune = parameter.value
                state.pointee.plan.detune = plan.detune
            }
        default:
            break
        }
    }

    // MARK: - State

    // `auval` reads a component's state as classic class info and requires
    // the `type`, `subtype`, `manufacturer`, and `version` keys the base
    // class writes. Musa's document snapshot is *added* to those keys, never
    // substituted for them: a presentation carries its identity beside the
    // host's, not instead of it.
    public override var fullState: [String: Any]? {
        get { documentState(includingClosure: false) }
        set {
            super.fullState = newValue
            restore(newValue)
        }
    }

    public override var fullStateForDocument: [String: Any]? {
        get { documentState(includingClosure: true) }
        set {
            super.fullStateForDocument = newValue
            restore(newValue)
        }
    }

    private func documentState(includingClosure: Bool) -> [String: Any] {
        var saved: [String: Any] = super.fullState ?? [:]
        for (key, value) in [
            TrialStateKey.version: musaTrialStateVersion,
            TrialStateKey.abiVersion: Int(MUSA_TRIAL_ABI_VERSION),
            TrialStateKey.projectIdentity: projectIdentity,
            TrialStateKey.sourceIdentity: sourceIdentity,
            TrialStateKey.lockIdentity: lockIdentity,
            TrialStateKey.selectedDeclaration: selectedDeclaration,
            // Asset bytes stay in the verified store and are named by digest.
            // A component state that carried them would be a second copy of
            // the closure, which §1 forbids.
            TrialStateKey.assetDigests: ["trial.asset.0"],
            TrialStateKey.assetStore: assetStore,
        ] as [String: Any] {
            saved[key] = value
        }
        if includingClosure {
            saved[TrialStateKey.sourceClosure] = sourceClosure
        }
        saved[TrialStateKey.parameters] = (parameterTree?.allParameters ?? []).map { parameter in
            [
                TrialStateKey.parameterAddress: Int(parameter.address),
                TrialStateKey.parameterIdentifier: parameter.identifier,
                TrialStateKey.parameterSourceIdentity: Address(rawValue: parameter.address)?.sourceIdentity ?? "",
                TrialStateKey.parameterValue: Double(parameter.value),
            ] as [String: Any]
        }
        return saved
    }

    private func restore(_ saved: [String: Any]?) {
        guard let saved else { return }
        lastRestoreRefusal = nil
        guard let version = saved[TrialStateKey.version] as? Int, version == musaTrialStateVersion else {
            lastRestoreRefusal = "state version is not \(musaTrialStateVersion)"
            return
        }
        guard let incoming = saved[TrialStateKey.sourceIdentity] as? String else {
            lastRestoreRefusal = "state names no source identity"
            return
        }
        guard incoming == sourceIdentity else {
            lastRestoreRefusal = "state was made from \(incoming), this component is \(sourceIdentity)"
            return
        }
        for entry in saved[TrialStateKey.parameters] as? [[String: Any]] ?? [] {
            guard let address = entry[TrialStateKey.parameterAddress] as? Int,
                  let value = entry[TrialStateKey.parameterValue] as? Double else { continue }
            parameterTree?.parameter(withAddress: AUParameterAddress(address))?.value = AUValue(value)
        }
    }
}
