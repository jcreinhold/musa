/// The automated host for the production Music Device.
///
/// It instantiates the component in this process and out of it, renders it,
/// and writes findings as JSON that `scripts/check-audio-unit.sh` asserts.
/// Prompt 216 asks for host tests for MIDI and event boundaries, reset,
/// format changes, offline render, discontinuities, tail, silence before
/// ready, extension relaunch, and corrupted state — each is a finding below,
/// named so a failure says which one.

import AVFoundation
import AudioToolbox
import Foundation
import MusaAudioUnitKit

let report = Report()

let projectPath = ProcessInfo.processInfo.environment["MUSA_AU_PROJECT"] ?? ""
let partName = ProcessInfo.processInfo.environment["MUSA_AU_PART"] ?? "piano"
let reportPath = ProcessInfo.processInfo.environment["MUSA_AU_REPORT"] ?? "musa-au-report.json"

func selection(rate: Double = 48_000, music: String? = nil, assets: String? = nil) -> MusaSelection {
    MusaSelection(
        project: projectPath,
        piece: nil,
        part: partName,
        sampleRate: rate,
        expectedMusicIdentity: music,
        expectedAssetIdentity: assets
    )
}

func makeUnit() throws -> MusaInstrumentAudioUnit {
    try MusaInstrumentAudioUnit(componentDescription: musaInstrumentDescription, options: [])
}

func note(_ status: UInt8, _ data1: UInt8, _ data2: UInt8) -> [UInt8] { [status, data1, data2] }

// MARK: - The library and the component agree

report.check(
    "abi.version",
    "The component and the Musa library were built against one ABI",
    musa_au_abi_version() == UInt32(MUSA_AU_ABI_VERSION),
    "library says \(musa_au_abi_version()), component was built for \(MUSA_AU_ABI_VERSION)"
)

report.record(
    Finding(
        id: "rt.probe",
        title: "The allocation probe is present and counting",
        outcome: AllocationProbe.isWorking ? .pass : .unsupported,
        detail: AllocationProbe.isWorking
            ? "a deliberate allocation was observed, so a zero below means zero"
            : "DYLD_INSERT_LIBRARIES did not put libMusaAllocProbe.dylib in this process; every allocation finding is reported as unsupported rather than as a pass"
    )
)

// MARK: - Silence before ready

do {
    let unit = try makeUnit()
    let rig = try Rig(unit: unit, capacity: 512)
    let quiet = rig.render(frames: 512, sampleTime: 0)
    report.check(
        "render.silenceBeforeReady",
        "A component with nothing selected renders silence, not noise or an error",
        quiet.allSatisfy { $0 == 0 },
        "512 frames before any source was selected"
    )
    report.check(
        "state.saysWhyItIsSilent",
        "It says why it is silent rather than pretending to be ready",
        !unit.refusal.isEmpty && !unit.isReady,
        unit.refusal
    )
}

// MARK: - Preparation

var prepared: MusaInstrumentAudioUnit?
do {
    let unit = try makeUnit()
    let refusal = unit.selectAndWait(selection())
    report.check(
        "prepare.selectsAPart",
        "The component prepares one part of one piece",
        unit.isReady,
        unit.isReady ? "prepared \(partName)" : refusal
    )
    if unit.isReady {
        prepared = unit
        report.check(
            "prepare.statesItsIdentity",
            "It names the music and asset closure it prepared",
            !((unit.fullStateForDocument?[MusaStateKey.musicIdentity] as? String) ?? "").isEmpty,
            (unit.fullStateForDocument?[MusaStateKey.musicIdentity] as? String) ?? "nothing"
        )
        report.check(
            "prepare.statesWhatItBinds",
            "It states the MIDI dimensions its source binds",
            !unit.boundInputs.isEmpty,
            unit.boundInputs.joined(separator: ", ")
        )
    }
}

// MARK: - Rendering

/// A component prepared from scratch.
///
/// Every differential comparison below starts from one of these rather than
/// from `reset()`. `HostedInstrument::reset` stops every voice, which is what
/// a host asks of it; it does not rewind the studio's own processor state,
/// and the crate's documentation says so. Comparing two renders across a
/// reset would therefore be comparing two different starting states and
/// blaming the difference on the block size.
func freshRig(capacity: Int, of what: MusaSelection? = nil) throws -> Rig {
    let unit = try makeUnit()
    unit.selectAndWait(what ?? selection())
    return try Rig(unit: unit, capacity: capacity)
}

if let unit = prepared {
    let rig = try Rig(unit: unit, capacity: 2048)
    let phrase: [(Int, [UInt8])] = [
        (0, note(0x90, 60, 100)),
        (733, note(0x90, 67, 64)),
        (1500, note(0x80, 60, 0)),
    ]

    let whole = try freshRig(capacity: 2048).renderPartitioned(frames: 4096, block: 2048, events: phrase)
    report.check(
        "render.sounds",
        "A prepared component sounds when the host plays it",
        whole.contains { abs($0) > 1e-6 },
        "4096 frames of a three-message phrase"
    )

    var partitionHeld = true
    var offending = ""
    for size in [1, 7, 64, 256, 511, 1024] {
        let again = try freshRig(capacity: 2048).renderPartitioned(frames: 4096, block: size, events: phrase)
        if again != whole {
            partitionHeld = false
            offending = "\(size)"
            break
        }
    }
    report.check(
        "render.blockPartition",
        "A different partition of the same frames is unobservable",
        partitionHeld,
        partitionHeld ? "1, 7, 64, 256, 511, and 1024 frames all agree" : "a block of \(offending) frames disagreed"
    )

    let atZero = try freshRig(capacity: 2048).render(frames: 256, sampleTime: 0, events: nil)
    let farAway = try freshRig(capacity: 2048).render(frames: 256, sampleTime: 9_876_543, events: nil)
    report.check(
        "render.noncontiguousSampleTime",
        "A host jump in sample time does not change what a block sounds like",
        atZero == farAway,
        "the same block at sample time 0 and 9876543"
    )

    let supplied = try freshRig(capacity: 2048).render(frames: 256, sampleTime: 0, events: nil, hostSuppliesMemory: true)
    let borrowed = try freshRig(capacity: 2048).render(frames: 256, sampleTime: 0, events: nil, hostSuppliesMemory: false)
    report.check(
        "render.hostSuppliesNoMemory",
        "A host that hands the block null mData gets the component's own buffers",
        supplied == borrowed,
        "256 frames each way"
    )

    // An offline bounce is the same preparation running faster than real
    // time, so it has to be the same frames. `06-daw-boundary.md` §3 names
    // Export and Live as different crossings, not different renderings.
    let offlineRig = try freshRig(capacity: 2048)
    offlineRig.unit.isRenderingOffline = true
    let offline = offlineRig.renderPartitioned(frames: 4096, block: 512, events: phrase)
    let realTime = try freshRig(capacity: 2048).renderPartitioned(frames: 4096, block: 512, events: phrase)
    report.check(
        "render.offline",
        "An offline render produces the frames a real-time render produces",
        offline == realTime,
        "4096 frames each way"
    )

    let held = EventList()
    held.addMIDI(note(0x90, 72, 110), at: 0)
    unit.reset()
    _ = rig.render(frames: 512, sampleTime: 0, events: held)
    unit.reset()
    let afterReset = rig.render(frames: 512, sampleTime: 0, events: nil)
    let settled = rig.render(frames: 2048, sampleTime: 512, events: nil)
    report.check(
        "render.reset",
        "A reset stops every voice and what is already in the studio decays",
        settled.allSatisfy { $0 == 0 },
        "silent 2048 frames after a reset, having been sounding before it",
        numbers: ["peakAfterReset": Double(afterReset.map { abs($0) }.max() ?? 0)]
    )

    unit.reset()
    let list = EventList()
    list.addMIDI(note(0x90, 60, 100), at: -32)
    let clamped = rig.render(frames: 256, sampleTime: 0, events: list)
    report.check(
        "render.negativeEventOffset",
        "An event at a negative offset is clamped into the block, not dropped",
        clamped.contains { abs($0) > 1e-6 },
        "a note-on at sample offset -32 still sounded"
    )

    unit.reset()
    let unsupportedList = EventList()
    unsupportedList.addMIDI([0xC0, 5, 0], at: 0)
    let before = unit.unboundEvents
    _ = rig.render(frames: 64, sampleTime: 0, events: unsupportedList)
    report.check(
        "render.unsupportedMessage",
        "A message with no source binding is dropped, never approximated",
        unit.unboundEvents == before,
        "a program change reached the component and changed nothing"
    )

    report.check(
        "bus.count",
        "The component publishes exactly the outputs the source declares, bus zero first",
        unit.outputBusses.count == unit.projectedOutputs.count && unit.projectedOutputs.first?.role == .main,
        unit.projectedOutputs.map(\.name).joined(separator: ", "),
        numbers: ["buses": Double(unit.outputBusses.count)]
    )
    report.check(
        "bus.names",
        "Each bus carries the name the source gave it, not a workstation's word for a track",
        zip(0..<unit.outputBusses.count, unit.projectedOutputs).allSatisfy { index, output in
            unit.outputBusses[index].name == output.name
        },
        (0..<unit.outputBusses.count).map { unit.outputBusses[$0].name ?? "(unnamed)" }.joined(separator: ", ")
    )

    // The component states an upper bound rather than measuring the studio's
    // decay; the bound has to actually hold.
    unit.reset()
    let struck = EventList()
    struck.addMIDI(note(0x90, 64, 120), at: 0)
    _ = rig.render(frames: 512, sampleTime: 0, events: struck)
    let released = EventList()
    released.addMIDI(note(0x80, 64, 0), at: 0)
    _ = rig.render(frames: 512, sampleTime: 512, events: released)
    let tailFrames = Int(unit.tailTime * unit.outputBusses[0].format.sampleRate)
    var sampleTime = 1024.0
    var remaining = tailFrames
    var peakInsideTail = 0.0
    while remaining > 0 {
        let count = min(2048, remaining)
        let block = rig.render(frames: count, sampleTime: sampleTime, events: nil)
        peakInsideTail = max(peakInsideTail, Double(block.map { abs($0) }.max() ?? 0))
        sampleTime += Double(count)
        remaining -= count
    }
    let past = rig.render(frames: 2048, sampleTime: sampleTime, events: nil)
    let peakPastTail = Double(past.map { abs($0) }.max() ?? 0)
    report.check(
        "render.tail",
        "The declared tail is a real upper bound on what is still sounding",
        peakPastTail <= 1e-6,
        "peak \(peakPastTail) after \(unit.tailTime) s of tail, having peaked \(peakInsideTail) inside it",
        numbers: ["tailSeconds": unit.tailTime, "peakPastTail": peakPastTail, "peakInsideTail": peakInsideTail]
    )

    // MARK: - Real-time instrumentation
    //
    // Every number is published beside the harness's own baseline: an empty
    // render block driven the same way. A rig that allocates on its own
    // account cannot say anything about a component that does not.
    let rounds: UInt32 = 2000
    let idle = MusaAuDriver.empty()
    let driver = rig.driver()
    let armed = EventList()
    armed.addMIDI(note(0x90, 60, 100), at: 0)
    armed.addMIDI(note(0x80, 60, 0), at: 256)

    // The event-list head is resolved *outside* the measured region.
    // `EventList.first` maps an Optional through a closure, and at `-Onone`
    // that costs two allocations — the harness's, not the component's, and
    // exactly the artefact prompt 215 §2 warns a rig will report as a
    // finding if it lets itself be measured.
    let armedHead = armed.first

    func drive(_ which: MusaAuDriver, _ head: UnsafePointer<AURenderEvent>?) {
        _ = which.run(
            with: rig.flagsPointer,
            timestamp: rig.timestampPointer,
            frames: 512,
            outputBusNumber: 0,
            outputData: rig.bufferListPointer,
            events: head,
            rounds: rounds
        )
    }

    // One warm call each before arming: the first invocation binds lazy
    // symbols, and that one-time cost is not what "allocates nothing" means.
    drive(idle, nil)
    drive(driver, armedHead)

    // A component that is not ready takes the early return and writes
    // silence. Measuring that first separates the cost of the block's own
    // shape from the cost of rendering through it.
    let silentUnit = try makeUnit()
    let silentRig = try Rig(unit: silentUnit, capacity: 2048)
    let silentDriver = silentRig.driver()
    _ = silentDriver.run(
        with: silentRig.flagsPointer, timestamp: silentRig.timestampPointer, frames: 512,
        outputBusNumber: 0, outputData: silentRig.bufferListPointer, events: nil, rounds: 1
    )
    let silent = AllocationProbe.measure {
        _ = silentDriver.run(
            with: silentRig.flagsPointer, timestamp: silentRig.timestampPointer, frames: 512,
            outputBusNumber: 0, outputData: silentRig.bufferListPointer, events: nil, rounds: rounds
        )
    }

    let baseline = AllocationProbe.measure { drive(idle, nil) }
    let bare = AllocationProbe.measure { drive(driver, nil) }
    let withEvents = AllocationProbe.measure { drive(driver, armedHead) }
    if let baseline, let bare, let withEvents, let silent {
        report.check(
            "rt.allocation.silence",
            "The block allocates nothing when there is nothing to render",
            silent == 0,
            "\(silent) allocations across \(rounds) blocks before anything was selected, baseline \(baseline)",
            numbers: ["allocations": Double(silent), "baseline": Double(baseline), "blocks": Double(rounds)]
        )
        report.check(
            "rt.allocation.baseline",
            "What the harness's own call path costs",
            baseline == 0,
            "\(baseline) allocations across \(rounds) calls of an empty render block",
            numbers: ["allocations": Double(baseline), "blocks": Double(rounds)]
        )
        report.check(
            "rt.allocation.render",
            "The render block allocates nothing",
            bare == 0,
            "\(bare) allocations across \(rounds) blocks of 512 frames, baseline \(baseline)",
            numbers: ["allocations": Double(bare), "baseline": Double(baseline), "blocks": Double(rounds)]
        )
        report.check(
            "rt.allocation.events",
            "Consuming the host's event list allocates nothing",
            withEvents == 0,
            "\(withEvents) allocations across \(rounds) blocks carrying two MIDI messages, baseline \(baseline)",
            numbers: ["allocations": Double(withEvents), "baseline": Double(baseline), "blocks": Double(rounds)]
        )
    } else {
        for id in ["rt.allocation.baseline", "rt.allocation.silence", "rt.allocation.render", "rt.allocation.events"] {
            report.unsupported(id, "Allocation on the render thread", "the probe was not inserted into this process")
        }
    }
}

// MARK: - State

if let unit = prepared {
    let document = unit.fullStateForDocument
    report.check(
        "state.keepsHostClassInfo",
        "The saved document keeps the base class's own class info",
        ["type", "subtype", "manufacturer", "version"].allSatisfy { document?[$0] != nil },
        (document?.keys.sorted().filter { !$0.hasPrefix("musa.") }.joined(separator: ", ")) ?? "nothing"
    )
    report.check(
        "state.isPropertyListSafe",
        "Everything Musa saves is property-list safe",
        PropertyListSerialization.propertyList(document ?? [:], isValidFor: .binary),
        "the document serializes"
    )
    report.check(
        "state.noAssetBytes",
        "It carries asset identity and never asset bytes",
        (document?[MusaStateKey.assetIdentity] as? String) != nil
            && !(document ?? [:]).keys.contains { $0.contains("bytes") },
        "asset identity only"
    )

    do {
        let restored = try makeUnit()
        restored.fullStateForDocument = document
        // The component prepares on a worker; wait for it or say it did not.
        let deadline = Date().addingTimeInterval(20)
        while !restored.isReady, Date() < deadline {
            RunLoop.current.run(until: Date().addingTimeInterval(0.05))
        }
        report.check(
            "state.roundTrip",
            "A restored document brings back the same instrument",
            restored.isReady
                && (restored.fullStateForDocument?[MusaStateKey.musicIdentity] as? String)
                    == (document?[MusaStateKey.musicIdentity] as? String),
            restored.isReady ? "the same music identity came back" : restored.refusal
        )
    }

    do {
        let foreign = try makeUnit()
        var corrupted = document ?? [:]
        corrupted[MusaStateKey.musicIdentity] = "musa.source.somebody-elses"
        foreign.fullStateForDocument = corrupted
        let deadline = Date().addingTimeInterval(20)
        while foreign.refusal.hasPrefix("preparing"), Date() < deadline {
            RunLoop.current.run(until: Date().addingTimeInterval(0.05))
        }
        report.check(
            "state.refusesForeignClosure",
            "State from another source is refused by naming both identities",
            !foreign.isReady && foreign.refusal.contains("somebody-elses"),
            foreign.refusal
        )
    }

    do {
        let broken = try makeUnit()
        broken.fullStateForDocument = [MusaStateKey.version: 999, MusaStateKey.project: projectPath, MusaStateKey.part: partName]
        report.check(
            "state.refusesAFutureVersion",
            "A document from a future version is refused rather than half-read",
            !broken.isReady && broken.refusal.contains("999"),
            broken.refusal
        )
    }

    do {
        let empty = try makeUnit()
        empty.fullStateForDocument = ["nonsense": 3]
        report.check(
            "state.survivesNonsense",
            "A document with none of Musa's keys leaves the component silent, not crashed",
            !empty.isReady,
            empty.refusal
        )
    }
}

// MARK: - Parameters

/// The second fixture: a part with a studio behind it, so that "the outputs
/// this part reaches" is more than one thing. Named by the script rather than
/// found here, because a harness that went looking for a piece would be
/// choosing what to measure.
let secondProject = ProcessInfo.processInfo.environment["MUSA_AU_PROJECT2"] ?? ""
let secondPart = ProcessInfo.processInfo.environment["MUSA_AU_PART2"] ?? ""

func address(_ unit: MusaInstrumentAudioUnit, _ identity: String) -> UInt64? {
    unit.publishedControls.first { $0.identity == identity }?.address
}

if let unit = prepared {
    let published = unit.publishedControls
    report.check(
        "param.tree.declared",
        "Every parameter is a source-declared control, and every admitted control is a parameter",
        {
            guard let tree = unit.parameterTree else { return false }
            let byAddress = Set(tree.allParameters.map { UInt64($0.address) })
            return !published.isEmpty && byAddress == Set(published.map(\.address))
                && tree.allParameters.allSatisfy { parameter in
                    published.contains { $0.address == UInt64(parameter.address) && $0.display == parameter.displayName }
                }
        }(),
        published.map(\.identity).joined(separator: ", "),
        numbers: ["parameters": Double(unit.parameterTree?.allParameters.count ?? 0)]
    )

    report.check(
        "param.ranges",
        "Each parameter's domain, default, and ramp flag come from the declaration",
        {
            guard let tree = unit.parameterTree else { return false }
            return published.allSatisfy { control in
                guard let parameter = tree.parameter(withAddress: AUParameterAddress(control.address)) else {
                    return false
                }
                let ramps = parameter.flags.contains(.flag_CanRamp)
                return parameter.minValue == control.minimum && parameter.maxValue == control.maximum
                    && parameter.value == control.defaultValue && ramps == control.continuous
            }
        }(),
        published
            .map { "\($0.display) \($0.minimum)…\($0.maximum)@\($0.defaultValue) \($0.continuous ? "ramps" : "steps")" }
            .joined(separator: "; ")
    )

    report.check(
        "param.losses",
        "A declared control that cannot be a float is a named loss, never a coerced parameter",
        !unit.projectionLosses.isEmpty
            && unit.projectionLosses.allSatisfy { $0.contains("is not a host parameter") }
            && unit.projectionLosses.allSatisfy { loss in
                !published.contains { loss.contains($0.identity) }
            },
        unit.projectionLosses.joined(separator: " | "),
        numbers: ["losses": Double(unit.projectionLosses.count)]
    )
}

/// The energy of one phrase, rendered from a fresh preparation.
func energy(_ frames: [Float]) -> Double { frames.reduce(0) { $0 + Double(abs($1)) } }

let phraseOnly: [(Int, [UInt8])] = [(0, note(0x90, 60, 100))]

if let expression = prepared.flatMap({ address($0, "std.performance::expression") }) {
    func withKnob(_ value: Float) throws -> Double {
        let rig = try freshRig(capacity: 4096)
        guard let tree = rig.unit.parameterTree,
              let parameter = tree.parameter(withAddress: AUParameterAddress(expression))
        else {
            return -1
        }
        parameter.value = value
        return energy(rig.renderPartitioned(frames: 4096, block: 512, midi: phraseOnly, parameters: []))
    }
    let quiet = try withKnob(0.05)
    let loud = try withKnob(1.0)
    report.check(
        "param.setValue",
        "A value set through the parameter tree reaches the instrument's declared mapping",
        loud > quiet * 2,
        "energy \(quiet) at 0.05 against \(loud) at 1.0",
        numbers: ["quiet": quiet, "loud": loud]
    )

    func withScheduled(_ value: Float) throws -> Double {
        let rig = try freshRig(capacity: 4096)
        return energy(
            rig.renderPartitioned(
                frames: 4096,
                block: 512,
                midi: phraseOnly,
                parameters: [(0, expression, value, 0)]
            )
        )
    }
    let scheduledQuiet = try withScheduled(0.05)
    let scheduledLoud = try withScheduled(1.0)
    report.check(
        "param.scheduled",
        "A parameter event the host schedules does the same thing the knob does",
        scheduledLoud > scheduledQuiet * 2,
        "energy \(scheduledQuiet) at 0.05 against \(scheduledLoud) at 1.0"
    )

    // A ramp is gradual: it differs from the point change while it is
    // running, and has stopped differing well after it has arrived.
    let point = try freshRig(capacity: 4096).renderPartitioned(
        frames: 12_288,
        block: 512,
        midi: phraseOnly,
        parameters: [(256, expression, 0.1, 0)]
    )
    let ramped = try freshRig(capacity: 4096).renderPartitioned(
        frames: 12_288,
        block: 512,
        midi: phraseOnly,
        parameters: [(256, expression, 0.1, 4_096)]
    )
    func difference(_ range: Range<Int>) -> Double {
        var total = 0.0
        var index = range.lowerBound
        while index < range.upperBound {
            total += abs(Double(point[index]) - Double(ramped[index]))
            index += 1
        }
        return total / Double(range.count)
    }
    let during = difference(1_024..<4_096)
    let after = difference(20_000..<24_576)
    report.check(
        "param.ramp",
        "A ramp is gradual while it runs and has converged once it has arrived",
        during > 0 && after < during / 10,
        "mean difference \(during) inside the ramp, \(after) long after it",
        numbers: ["during": during, "after": after]
    )

    // §4, Theorem R1-batch: a different partition of the same frames is
    // unobservable, and parameter events are placed by sample offset, so
    // they are exactly where that could break.
    let history: [(Int, UInt64, Float, UInt32)] = [
        (0, expression, 1.0, 0),
        (301, expression, 0.2, 640),
        (2_000, expression, 0.8, 0),
    ]
    let midi: [(Int, [UInt8])] = [(11, note(0x90, 60, 100)), (1_500, note(0x90, 67, 90))]
    let whole = try freshRig(capacity: 6_000).renderPartitioned(
        frames: 6_000,
        block: 6_000,
        midi: midi,
        parameters: history
    )
    var partitionHeld = true
    var partitionDetail = "every block size agreed"
    for size in [1, 64, 512, 2_048] {
        let partitioned = try freshRig(capacity: 6_000).renderPartitioned(
            frames: 6_000,
            block: size,
            midi: midi,
            parameters: history
        )
        if partitioned != whole {
            partitionHeld = false
            partitionDetail = "a block size of \(size) changed the music"
            break
        }
    }
    report.check(
        "param.blockPartition",
        "A parameter history is the same music under every host block partition",
        partitionHeld,
        partitionDetail
    )

    let stray = try freshRig(capacity: 512)
    let before = stray.unit.unboundEvents
    _ = stray.renderPartitioned(frames: 512, block: 512, midi: [], parameters: [(0, 0xDEAD_BEEF, 0.5, 0)])
    report.check(
        "param.unknownAddress",
        "An address the source does not publish is counted, never guessed at",
        stray.unit.unboundEvents == before + 1,
        "\(stray.unit.unboundEvents - before) unbound event after one stray address"
    )
}

// MARK: - The address table

if let unit = prepared {
    let document = unit.fullStateForDocument
    let table = document?[MusaStateKey.controlTable] as? String
    report.check(
        "state.controlTable",
        "The saved document carries the address table and the projection losses",
        !(table ?? "").isEmpty && (document?[MusaStateKey.controlLosses] as? [String])?.isEmpty == false,
        table.map { "\($0.split(separator: "\n").count) lines of table" } ?? "no table"
    )

    let restored = try makeUnit()
    restored.fullStateForDocument = document
    let deadline = Date().addingTimeInterval(20)
    while !restored.isReady, Date() < deadline {
        RunLoop.current.run(until: Date().addingTimeInterval(0.05))
    }
    report.check(
        "param.addressesStable",
        "Every address comes back where the document left it",
        restored.isReady
            && restored.publishedControls.map { [$0.identity, String($0.address)] }
                == unit.publishedControls.map { [$0.identity, String($0.address)] },
        restored.isReady ? "\(restored.publishedControls.count) addresses unchanged" : restored.refusal
    )
}

// MARK: - Outputs

if !secondProject.isEmpty {
    let multi = MusaSelection(project: secondProject, piece: nil, part: secondPart, sampleRate: 48_000)
    let unit = try makeUnit()
    let refusal = unit.selectAndWait(multi)
    report.check(
        "bus.identity",
        "A part with a studio behind it publishes the points it actually reaches",
        unit.isReady && unit.projectedOutputs.count > 1 && unit.projectedOutputs.first?.role == .main
            && unit.projectedOutputs.dropFirst().contains { $0.role == .bus },
        unit.isReady
            ? unit.projectedOutputs.map { "\($0.name) (\($0.role))" }.joined(separator: ", ")
            : refusal
    )

    if unit.isReady {
        let count = unit.projectedOutputs.count
        let struck: [(Int, [UInt8])] = [(0, note(0x90, 64, 100)), (3_000, note(0x80, 64, 0))]
        let all = try freshRig(capacity: 512, of: multi).renderBuses(
            frames: 8_000,
            block: 256,
            midi: struck,
            parameters: [],
            buses: count
        )
        let alone = try freshRig(capacity: 512, of: multi).renderBuses(
            frames: 8_000,
            block: 256,
            midi: struck,
            parameters: [],
            buses: 1
        )
        report.check(
            "bus.mainUnchanged",
            "Bus zero says the same thing whether a host takes one output or all of them",
            all[0] == alone[0],
            "the GarageBand fallback is the same main output Logic gets, not a different mix"
        )
        report.check(
            "bus.independent",
            "An extra bus is neither silence nor a copy of the main output",
            all.dropFirst().allSatisfy { bus in bus.contains { $0 != 0 } && bus != all[0] },
            "\(count - 1) extra buses, each sounding and each its own"
        )
    }
}


// MARK: - Format change

if prepared != nil {
    let unit = try makeUnit()
    _ = unit.selectAndWait(selection())
    let format = AVAudioFormat(standardFormatWithSampleRate: 44_100, channels: 2)!
    do {
        try unit.outputBusses[0].setFormat(format)
        try unit.allocateRenderResources()
        let deadline = Date().addingTimeInterval(20)
        while !unit.isReady, Date() < deadline {
            RunLoop.current.run(until: Date().addingTimeInterval(0.05))
        }
        report.check(
            "render.formatChange",
            "A host that renegotiates the rate gets a new preparation, not a resample",
            unit.isReady,
            unit.isReady ? "prepared again at 44100" : unit.refusal
        )
    } catch {
        report.unsupported("render.formatChange", "A host may renegotiate the rate", "\(error)")
    }
}

// MARK: - Out of process

func instantiateOutOfProcess() -> AVAudioUnit? {
    var result: AVAudioUnit?
    let done = DispatchSemaphore(value: 0)
    AVAudioUnit.instantiate(with: musaInstrumentDescription, options: [.loadOutOfProcess]) { unit, _ in
        result = unit
        done.signal()
    }
    let deadline = Date().addingTimeInterval(20)
    while done.wait(timeout: .now() + 0.05) == .timedOut, Date() < deadline {
        RunLoop.current.run(until: Date().addingTimeInterval(0.05))
    }
    return result
}

// The out-of-process section needs the component registered with the
// system, which `scripts/check-audio-unit.sh` does around this run. The
// Thread Sanitizer pass sets `MUSA_AU_SKIP_HOSTED` instead: spawning system
// extensions under TSan measures the system, and every race this component
// can have of its own is in the in-process path above.
let skipHosted = ProcessInfo.processInfo.environment["MUSA_AU_SKIP_HOSTED"] != nil

let components = skipHosted
    ? []
    : AVAudioUnitComponentManager.shared().components(matching: musaInstrumentDescription)
if skipHosted {
    report.unsupported(
        "discovery.instrument",
        "Core Audio finds the registered Music Device",
        "MUSA_AU_SKIP_HOSTED was set, so nothing out of process was attempted"
    )
} else {
    report.check(
    "discovery.instrument",
    "Core Audio finds the registered Music Device",
    !components.isEmpty,
    components.first.map { "\($0.name) \($0.versionString)" } ?? "nothing matched aumu musa Musa",
    numbers: ["count": Double(components.count)]
    )
}

if !components.isEmpty {
    if let first = instantiateOutOfProcess() {
        report.check(
            "instantiate.outOfProcess",
            "It loads in its own process with its buses intact",
            first.auAudioUnit.outputBusses.count == 1,
            "\(first.auAudioUnit.outputBusses.count) output bus out of process"
        )
        let document = first.auAudioUnit.fullStateForDocument
        report.check(
            "state.crossesTheProcessBoundary",
            "Its saved document survives the process boundary",
            (document?[MusaStateKey.version] as? Int) == musaStateVersion,
            document == nil ? "no document state came back" : "version \(musaStateVersion) came back"
        )
        if let second = instantiateOutOfProcess() {
            report.check(
                "instantiate.relaunch",
                "A second instance comes up beside the first",
                second.auAudioUnit.outputBusses.count == 1,
                "a second instance came up"
            )
        } else {
            report.check("instantiate.relaunch", "A second instance comes up beside the first", false, "it did not")
        }

        let kill = Process()
        kill.executableURL = URL(fileURLWithPath: "/usr/bin/pkill")
        kill.arguments = ["-f", "MusaInstrument.appex"]
        try? kill.run()
        kill.waitUntilExit()
        Thread.sleep(forTimeInterval: 1)
        if let third = instantiateOutOfProcess() {
            report.check(
                "instantiate.afterTermination",
                "A fresh process comes up after the extension is killed",
                third.auAudioUnit.outputBusses.count == 1,
                "a fresh process came up"
            )
        } else {
            report.check(
                "instantiate.afterTermination",
                "A fresh process comes up after the extension is killed",
                false,
                "nothing came back within twenty seconds"
            )
        }
    } else {
        report.check("instantiate.outOfProcess", "It loads in its own process", false, "nothing came back in twenty seconds")
    }
}

// MARK: - Write it down

let environment = [
    "harness": "musa-audio-unit",
    "processName": ProcessInfo.processInfo.processName,
    "operatingSystem": ProcessInfo.processInfo.operatingSystemVersionString,
    "project": projectPath,
    "part": partName,
    "MUSA_AU_XCODE": ProcessInfo.processInfo.environment["MUSA_AU_XCODE"] ?? "",
    "MUSA_AU_SDK": ProcessInfo.processInfo.environment["MUSA_AU_SDK"] ?? "",
    "MUSA_AU_HOST": ProcessInfo.processInfo.environment["MUSA_AU_HOST"] ?? "",
]
try report.json(environment: environment).write(to: URL(fileURLWithPath: reportPath))
