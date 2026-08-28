//  What the trial asks of the Music Device.

import AudioToolbox
import AVFoundation
import Foundation
import MusaTrialKit

enum InstrumentExperiments {
    static func run(into report: Report) {
        guard let unit = try? TrialInstrumentAudioUnit(componentDescription: musaTrialInstrumentDescription) else {
            report.check("instantiate.inProcess", "The component loads in this process", false, "initializer threw")
            return
        }
        report.check("instantiate.inProcess", "The component loads in this process", true,
                     "TrialInstrumentAudioUnit constructed directly, no extension involved")
        report.check("abi.version", "The published ABI version is the one the header states",
                     MUSA_TRIAL_ABI_VERSION == 1, "ABI version \(MUSA_TRIAL_ABI_VERSION)")

        try? unit.allocateRenderResources()
        let rig = RenderRig(unit: unit)

        determinism(unit, rig, into: report)
        partition(unit, into: report)
        noncontiguous(unit, rig, into: report)
        hostMemory(unit, into: report)
        resetAndOffline(unit, into: report)
        maximumFrames(unit, into: report)
        parameters(unit, into: report)
        buses(unit, into: report)
        state(unit, into: report)
        realTime(unit, into: report)
    }

    /// One note, rendered twice from a reset instrument, is the same bytes.
    private static func determinism(_ unit: TrialInstrumentAudioUnit, _ rig: RenderRig, into report: Report) {
        func once() -> [Float] {
            unit.reset()
            let events = EventList()
            events.addMIDI(at: 0, (0x90, 60, 100))
            return rig.render(frames: 512, sampleTime: 0, events: events.head).left
        }
        let first = once()
        let second = once()
        report.check("render.determinism", "Two renders of one input are byte-equal",
                     first == second && first.contains(where: { $0 != 0 }),
                     "512 frames, note 60, rendered twice",
                     ["frames": 512])
    }

    /// Theorem R1-batch at the host boundary: a different partition of the
    /// same frames is unobservable.
    private static func partition(_ unit: TrialInstrumentAudioUnit, into report: Report) {
        let rig = RenderRig(unit: unit)
        func render(chunks: [Int]) -> [Float] {
            unit.reset()
            let events = EventList()
            events.addMIDI(at: 0, (0x90, 64, 100))
            var output: [Float] = []
            var at = 0.0
            var first = true
            for chunk in chunks {
                let result = rig.render(frames: chunk, sampleTime: at, events: first ? events.head : nil)
                output += result.left
                at += Double(chunk)
                first = false
            }
            return output
        }
        let whole = render(chunks: [1_024])
        let split = render(chunks: [256, 256, 256, 256])
        report.check("render.blockPartition", "A different partition of the same frames is unobservable",
                     whole == split && whole.count == 1_024,
                     "1×1024 equals 4×256",
                     ["frames": 1_024])
    }

    /// A host may hand over a sample time that jumped. The component reads
    /// host sample time; it does not treat it as musical meaning.
    private static func noncontiguous(_ unit: TrialInstrumentAudioUnit, _ rig: RenderRig, into report: Report) {
        func render(at sampleTime: Double) -> [Float] {
            unit.reset()
            let events = EventList()
            events.addMIDI(at: 0, (0x90, 67, 100))
            return rig.render(frames: 256, sampleTime: sampleTime, events: events.head).left
        }
        let contiguous = render(at: 0)
        let jumped = render(at: 9_876_543.0)
        report.check("render.noncontiguousSampleTime", "A jump in host sample time changes nothing",
                     contiguous == jumped,
                     "the same block at sample time 0 and 9876543")
    }

    /// A host may pass a buffer list with no memory in it.
    private static func hostMemory(_ unit: TrialInstrumentAudioUnit, into report: Report) {
        let rig = RenderRig(unit: unit)
        unit.reset()
        let events = EventList()
        events.addMIDI(at: 0, (0x90, 60, 100))
        let supplied = rig.render(frames: 256, sampleTime: 0, events: events.head, supplyMemory: true)
        unit.reset()
        let events2 = EventList()
        events2.addMIDI(at: 0, (0x90, 60, 100))
        let borrowed = rig.render(frames: 256, sampleTime: 0, events: events2.head, supplyMemory: false)
        report.check("render.hostSuppliesNoMemory", "The component supplies its own buffer when the host does not",
                     borrowed.status == noErr && borrowed.left == supplied.left && !borrowed.left.isEmpty,
                     "null mData produced the same 256 frames as host-supplied memory")
    }

    /// Reset, and offline rendering. A host loops, and a host bounces; the
    /// component must sound the same either way, because neither is a
    /// musical fact (`06-daw-boundary.md` §4).
    private static func resetAndOffline(_ unit: TrialInstrumentAudioUnit, into report: Report) {
        let rig = RenderRig(unit: unit)
        func render() -> [Float] {
            let events = EventList()
            events.addMIDI(at: 0, (0x90, 62, 100))
            return rig.render(frames: 256, sampleTime: 0, events: events.head).left
        }
        unit.reset()
        let fresh = render()
        _ = rig.render(frames: 256, sampleTime: 256)
        unit.reset()
        let afterReset = render()
        report.check("render.reset", "A reset instrument renders what a fresh one does",
                     fresh == afterReset && !fresh.isEmpty,
                     "256 frames before and after a loop and a reset")

        let wasOffline = unit.isRenderingOffline
        unit.isRenderingOffline = true
        unit.reset()
        let offline = render()
        unit.isRenderingOffline = wasOffline
        report.check("render.offline", "Offline rendering produces the same frames as real time",
                     offline == fresh,
                     "the same 256 frames with isRenderingOffline set")
    }

    /// The largest block the host declared, and a small one after it.
    private static func maximumFrames(_ unit: TrialInstrumentAudioUnit, into report: Report) {
        unit.deallocateRenderResources()
        unit.maximumFramesToRender = 2_048
        do {
            try unit.allocateRenderResources()
        } catch {
            report.check("render.maximumFrames", "The declared maximum block renders", false, "allocate threw: \(error)")
            return
        }
        let rig = RenderRig(unit: unit)
        unit.reset()
        let events = EventList()
        events.addMIDI(at: 0, (0x90, 60, 100))
        let large = rig.render(frames: 2_048, sampleTime: 0, events: events.head)
        let small = rig.render(frames: 32, sampleTime: 2_048)
        report.check("render.maximumFrames", "The declared maximum block renders, and a small one after it",
                     large.status == noErr && small.status == noErr && large.left.count == 2_048,
                     "2048 then 32 frames at maximumFramesToRender 2048",
                     ["maximum": 2_048])
    }

    /// Rule D2, and a ramp the host schedules inside a block.
    private static func parameters(_ unit: TrialInstrumentAudioUnit, into report: Report) {
        let published = (unit.parameterTree?.allParameters ?? []).map(\.identifier).sorted()
        report.check("param.tree.projection", "Published parameters are the declared controls and nothing else",
                     published == ["detune", "gain"],
                     "tree publishes \(published.joined(separator: ", "))",
                     ["count": Double(published.count)])

        let rig = RenderRig(unit: unit)
        unit.reset()
        let loud = EventList()
        loud.addMIDI(at: 0, (0x90, 60, 127))
        let before = rig.render(frames: 256, sampleTime: 0, events: loud.head).left
        unit.reset()
        let ramped = EventList()
        ramped.addMIDI(at: 0, (0x90, 60, 127))
        ramped.addParameter(at: 0, address: 0, value: 0.25, ramp: 128)
        let after = rig.render(frames: 256, sampleTime: 0, events: ramped.head).left
        let peakBefore = before.map(abs).max() ?? 0
        let peakAfter = after.map(abs).max() ?? 0
        report.check("param.ramp", "A scheduled parameter event reaches the render block",
                     peakAfter < peakBefore * 0.6 && peakAfter > 0,
                     "gain 1.0 peaked at \(peakBefore), a ramp to 0.25 peaked at \(peakAfter)",
                     ["peakBefore": Double(peakBefore), "peakAfter": Double(peakAfter)])

        unit.installParameterTree(withBrightness: true)
        let extended = (unit.parameterTree?.allParameters ?? []).map(\.identifier).sorted()
        report.check("param.tree.replacement", "The published tree can be replaced while loaded",
                     extended == ["brightness", "detune", "gain"],
                     "tree became \(extended.joined(separator: ", "))",
                     ["count": Double(extended.count)])
        unit.installParameterTree(withBrightness: false)
    }

    /// Two output buses of one component, pulled separately.
    private static func buses(_ unit: TrialInstrumentAudioUnit, into report: Report) {
        let rig = RenderRig(unit: unit)
        report.check("bus.count", "The component publishes two output buses",
                     unit.outputBusses.count == 2,
                     "outputBusses.count is \(unit.outputBusses.count)",
                     ["buses": Double(unit.outputBusses.count)])
        unit.reset()
        let events = EventList()
        events.addMIDI(at: 0, (0x90, 60, 100))
        let main = rig.render(frames: 256, sampleTime: 0, bus: 0, events: events.head).left
        unit.reset()
        let events2 = EventList()
        events2.addMIDI(at: 0, (0x90, 60, 100))
        let aux = rig.render(frames: 256, sampleTime: 0, bus: 1, events: events2.head).left
        let halved = zip(main, aux).allSatisfy { abs($0 * 0.5 - $1) < 1e-6 }
        report.check("bus.multipleOutputs", "Each output bus renders its own signal",
                     halved && !main.isEmpty,
                     "bus 1 is bus 0 at half scale, frame for frame")
    }

    /// Saved state is a presentation: it names its closure and refuses a
    /// foreign one.
    private static func state(_ unit: TrialInstrumentAudioUnit, into report: Report) {
        unit.parameterTree?.parameter(withAddress: 0)?.value = 0.375
        guard let saved = unit.fullState else {
            report.check("state.roundTrip", "State saves and restores", false, "fullState was nil")
            return
        }
        unit.parameterTree?.parameter(withAddress: 0)?.value = 1.0
        unit.fullState = saved
        let restored = unit.parameterTree?.parameter(withAddress: 0)?.value ?? 0
        report.check("state.roundTrip", "State saves and restores",
                     abs(restored - 0.375) < 1e-6 && unit.lastRestoreRefusal == nil,
                     "gain returned to \(restored)")

        let document = unit.fullStateForDocument ?? [:]
        let hasClosure = document[TrialStateKey.sourceClosure] != nil
        let hasIdentities = document[TrialStateKey.projectIdentity] != nil
            && document[TrialStateKey.lockIdentity] != nil
            && document[TrialStateKey.sourceIdentity] != nil
        let plistSafe = PropertyListSerialization.propertyList(document, isValidFor: .binary)
        report.check("state.documentCarriesClosure", "Document state carries the closure and its identities, and is property-list safe",
                     hasClosure && hasIdentities && plistSafe,
                     "closure \(hasClosure), identities \(hasIdentities), plist-safe \(plistSafe)")
        let classInfo = ["type", "subtype", "manufacturer", "version"]
        report.check("state.keepsHostClassInfo", "Saved state keeps the host's own class-info keys beside Musa's",
                     classInfo.allSatisfy { document[$0] != nil },
                     "class info carries \(classInfo.filter { document[$0] != nil }.joined(separator: ", "))")
        report.check("state.noAssetBytes", "Document state names assets by digest rather than carrying them",
                     document[TrialStateKey.assetDigests] is [String],
                     "asset digests only")

        var foreign = saved
        foreign[TrialStateKey.sourceIdentity] = "trial.source.other"
        unit.fullState = foreign
        report.check("state.refusesForeignClosure", "Restoring against a different closure is refused, naming both",
                     (unit.lastRestoreRefusal ?? "").contains("trial.source.other")
                         && (unit.lastRestoreRefusal ?? "").contains("trial.source.0"),
                     unit.lastRestoreRefusal ?? "no refusal was recorded")
        unit.fullState = saved
    }

    /// The render block, measured rather than asserted.
    private static func realTime(_ unit: TrialInstrumentAudioUnit, into report: Report) {
        let rig = RenderRig(unit: unit)
        unit.reset()
        let warm = EventList()
        warm.addMIDI(at: 0, (0x90, 60, 100))
        _ = rig.renderQuietly(frames: 512, sampleTime: 0, events: warm.head)

        // Every event list is built before the probe is armed. The trial is
        // measuring the render block, not the host that feeds it.
        let rounds = 2_000
        let lists: [EventList] = (0..<24).map { index in
            let events = EventList()
            events.addMIDI(at: 0, (0x90, UInt8(48 + index), 100))
            events.addParameter(at: 0, address: 0, value: 0.5, ramp: 64)
            return events
        }
        let heads: [UnsafePointer<AURenderEvent>?] = lists.map(\.head)
        rig.prepare(frames: 512)
        // A control reading first, through the same C caller. Calling an
        // `AUInternalRenderBlock` from Swift costs one heap allocation per
        // call — a fact about the Swift caller, not about the component —
        // so every reading below goes through `MusaTrialDrive` instead.
        let empty: AUInternalRenderBlock = { _, _, _, _, _, _, _ in noErr }
        let idle = MusaTrialDriver(renderBlock: empty)
        let driver = MusaTrialDriver(renderBlock: rig.renderBlock)
        // One warm call each, before the probe is armed: the first invocation
        // binds lazy symbols, and that one-time cost is not what "the render
        // block allocates nothing" is about.
        _ = idle.run(with: rig.flagsPointer, timestamp: rig.timestampPointer, frames: 512,
                     outputBusNumber: 0, outputData: rig.bufferListPointer, events: nil, rounds: 1)
        _ = driver.run(with: rig.flagsPointer, timestamp: rig.timestampPointer, frames: 512,
                       outputBusNumber: 0, outputData: rig.bufferListPointer, events: heads[0], rounds: 1)
        let baseline = AllocationProbe.measure {
            _ = idle.run(with: rig.flagsPointer, timestamp: rig.timestampPointer, frames: 512,
                         outputBusNumber: 0, outputData: rig.bufferListPointer, events: nil, rounds: UInt32(rounds))
        }
        if let baseline {
            report.record(Finding(
                id: "rt.allocation.hostBaseline",
                title: "What the harness's own call path costs",
                outcome: baseline == 0 ? .pass : .fail,
                detail: "\(baseline) allocations across \(rounds) calls of an empty render block driven from C",
                numbers: ["allocations": Double(baseline), "calls": Double(rounds)]
            ))
        }
        // Two readings, because they answer different questions: the first is
        // the render block on its own, the second is the render block reading
        // the host's event list. A cost that appears only in the second is a
        // cost of consuming events, and that is where an AUv3 component in
        // Swift is most likely to reach the heap.
        let bare = AllocationProbe.measure {
            _ = driver.run(with: rig.flagsPointer, timestamp: rig.timestampPointer, frames: 512,
                           outputBusNumber: 0, outputData: rig.bufferListPointer, events: nil, rounds: UInt32(rounds))
        }
        let withEvents = AllocationProbe.measure {
            _ = driver.run(with: rig.flagsPointer, timestamp: rig.timestampPointer, frames: 512,
                           outputBusNumber: 0, outputData: rig.bufferListPointer, events: heads[0], rounds: UInt32(rounds))
        }
        if let bare, let withEvents {
            report.check("rt.allocation", "The render block allocates nothing",
                         bare == 0,
                         "\(bare) allocations across \(rounds) blocks of 512 frames with no events",
                         ["allocations": Double(bare), "blocks": Double(rounds)])
            report.check("rt.allocation.events", "Consuming the host's event list allocates nothing",
                         withEvents == 0,
                         "\(withEvents) allocations across \(rounds) blocks carrying one MIDI and one parameter-ramp event",
                         ["allocations": Double(withEvents), "blocks": Double(rounds)])
        } else {
            report.unsupported("rt.allocation", "The render block allocates nothing",
                               "the allocation probe was not inserted into this process")
            report.unsupported("rt.allocation.events", "Consuming the host's event list allocates nothing",
                               "the allocation probe was not inserted into this process")
        }

        var worst = 0.0
        var total = 0.0
        let events = EventList()
        events.addMIDI(at: 0, (0x90, 60, 100))
        _ = rig.renderQuietly(frames: 512, sampleTime: 0, events: events.head)
        rig.prepare(frames: 512)
        for round in 0..<rounds {
            let started = DispatchTime.now().uptimeNanoseconds
            _ = rig.invoke(frames: 512, sampleTime: Double(round * 512))
            let elapsed = Double(DispatchTime.now().uptimeNanoseconds - started) / 1_000.0
            worst = max(worst, elapsed)
            total += elapsed
        }
        let budget = 512.0 / 48_000.0 * 1_000_000.0
        report.check("rt.timing", "One block is rendered well inside its own duration",
                     worst < budget,
                     "worst \(String(format: "%.1f", worst)) µs, mean \(String(format: "%.1f", total / Double(rounds))) µs, budget \(String(format: "%.1f", budget)) µs",
                     ["worstMicros": worst, "meanMicros": total / Double(rounds), "budgetMicros": budget])
    }
}
