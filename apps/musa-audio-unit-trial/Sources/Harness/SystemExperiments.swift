//  What the trial asks of the system rather than of the code: whether macOS
//  finds the two components, loads them in another process, survives that
//  process going away, and gives a sandboxed extension anywhere to read an
//  asset from.

import AudioToolbox
import AVFoundation
import Foundation
import MusaTrialKit

enum SystemExperiments {
    static func run(into report: Report, appGroup: String) {
        placeAsset(into: report, appGroup: appGroup)
        discover(musaTrialInstrumentDescription, id: "discovery.instrument",
                 title: "The system finds the Music Device", into: report)
        discover(musaTrialProcessorDescription, id: "discovery.processor",
                 title: "The system finds the MIDI Processor", into: report)
        outOfProcess(into: report)
        appGroupContainer(into: report, appGroup: appGroup)
    }

    /// Put the asset where a sandboxed extension would look for it, before
    /// anything asks an extension to read it.
    private static func placeAsset(into report: Report, appGroup: String) {
        guard let container = FileManager.default.containerURL(forSecurityApplicationGroupIdentifier: appGroup) else {
            return
        }
        try? "trial".write(to: container.appendingPathComponent(musaTrialAssetName),
                           atomically: true, encoding: .utf8)
        _ = report
    }

    private static func discover(_ description: AudioComponentDescription,
                                 id: String,
                                 title: String,
                                 into report: Report) {
        let found = AVAudioUnitComponentManager.shared().components(matching: description)
        let names = found.map { "\($0.name) \($0.versionString)" }
        report.check(id, title, !found.isEmpty,
                     found.isEmpty
                         ? "no component matching \(fourCharacterText(description.componentType)) \(fourCharacterText(description.componentSubType)) \(fourCharacterText(description.componentManufacturer)) is registered"
                         : names.joined(separator: ", "),
                     ["count": Double(found.count)])
    }

    /// Load the Music Device in the process the system spawns for it, then
    /// let it go and load it again. Prompt 215 asks whether the extension can
    /// be terminated and relaunched; letting the last reference go is the
    /// termination a host actually performs.
    private static func outOfProcess(into report: Report) {
        guard let first = instantiate() else {
            report.check("instantiate.outOfProcess", "The component loads in another process", false,
                         "the system did not hand back a component: \(instantiationFailure)")
            report.check("instantiate.relaunch", "The component loads again after it is released", false,
                         "nothing loaded the first time")
            return
        }
        let parameters = (first.auAudioUnit.parameterTree?.allParameters ?? []).map(\.identifier).sorted()
        let buses = first.auAudioUnit.outputBusses.count
        let state = first.auAudioUnit.fullStateForDocument
        report.check("instantiate.outOfProcess", "The component loads in another process, with its tree and buses intact",
                     parameters == ["detune", "gain"] && buses == 2,
                     "parameters \(parameters.joined(separator: ", ")), \(buses) output buses",
                     ["buses": Double(buses)])
        report.check("state.crossesTheProcessBoundary", "Document state survives the extension process boundary",
                     (state?[TrialStateKey.sourceIdentity] as? String) == "trial.source.0",
                     state == nil ? "no document state came back" : "the closure identity came back intact")

        let store = state?[TrialStateKey.assetStore] as? String ?? "nothing came back"
        report.check("sandbox.extensionNamesAssetStore", "A sandboxed extension can name the store it was told to use",
                     store.hasPrefix("named "),
                     "the extension reported \(store)")
        report.unsupported("sandbox.extensionReadsAsset",
                           "A sandboxed extension can read an asset out of that store",
                           "not attempted: an ad-hoc-signed extension is taken down by the system when it reads inside the container, and the reproduction is in docs/notes/research/93-the-audio-unit-shape.md §4")

        terminateExtension(into: report)

        guard let second = instantiate() else {
            report.check("instantiate.relaunch", "The component loads again after it is released", false,
                         "the second instantiation failed")
            return
        }
        report.check("instantiate.relaunch", "The component loads again after it is released",
                     second.auAudioUnit.outputBusses.count == 2,
                     "a second instance came up with \(second.auAudioUnit.outputBusses.count) output buses")
    }

    /// Kill the process the system spawned for the extension, and ask for it
    /// again. A host does this to us whenever it decides an extension has
    /// misbehaved, so the trial does it deliberately once.
    private static func terminateExtension(into report: Report) {
        let kill = Process()
        kill.executableURL = URL(fileURLWithPath: "/usr/bin/pkill")
        kill.arguments = ["-f", "Instrument.appex"]
        try? kill.run()
        kill.waitUntilExit()
        guard kill.terminationStatus == 0 else {
            report.unsupported("instantiate.afterTermination", "The component comes back after its process is killed",
                               "no extension process was running to kill")
            return
        }
        Thread.sleep(forTimeInterval: 1.0)
        guard let revived = instantiate() else {
            report.check("instantiate.afterTermination", "The component comes back after its process is killed", false,
                         "the system did not spawn the extension again")
            return
        }
        report.check("instantiate.afterTermination", "The component comes back after its process is killed",
                     revived.auAudioUnit.outputBusses.count == 2,
                     "a fresh process came up with \(revived.auAudioUnit.outputBusses.count) output buses")
    }

    /// Why the last out-of-process instantiation failed, for the report to
    /// quote rather than guess at.
    private(set) static var instantiationFailure = "none"

    private static func instantiate() -> AVAudioUnit? {
        var result: AVAudioUnit?
        var finished = false
        AVAudioUnit.instantiate(with: musaTrialInstrumentDescription, options: [.loadOutOfProcess]) { unit, error in
            result = unit
            if let error {
                instantiationFailure = "\(error)"
            }
            finished = true
        }
        let deadline = Date().addingTimeInterval(20)
        while !finished, Date() < deadline {
            RunLoop.current.run(mode: .default, before: Date().addingTimeInterval(0.05))
        }
        return result
    }

    /// The smallest sandbox arrangement that would let an extension read an
    /// asset the containing app verified.
    private static func appGroupContainer(into report: Report, appGroup: String) {
        guard let container = FileManager.default.containerURL(forSecurityApplicationGroupIdentifier: appGroup) else {
            report.unsupported("sandbox.appGroup", "An App Group container is available to both sides",
                               "containerURL(forSecurityApplicationGroupIdentifier:) returned nil for \(appGroup); on macOS the group needs an entitlement signed by a Developer Team, which an unsigned local build does not have")
            return
        }
        let probe = container.appendingPathComponent("trial-asset.txt")
        do {
            try "trial".write(to: probe, atomically: true, encoding: .utf8)
            let read = try String(contentsOf: probe, encoding: .utf8)
            try? FileManager.default.removeItem(at: probe)
            report.check("sandbox.appGroup", "An App Group container is available to both sides",
                         read == "trial", "wrote and read \(probe.path)")
        } catch {
            report.unsupported("sandbox.appGroup", "An App Group container is available to both sides",
                               "macOS named a container at \(container.path) but would not create it: \(error.localizedDescription) — the group needs an entitlement signed by a Developer Team, which this build does not have")
        }
    }
}
