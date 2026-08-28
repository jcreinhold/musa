//  What the trial asks of the MIDI Processor.
//
//  The claim under test is the one that decides whether prompts 218's design
//  survives: a component that owns a finite schedule and is told where the
//  host is can answer with the occurrences that belong there, without ever
//  stepping from the beginning.

import AudioToolbox
import AVFoundation
import Foundation
import MusaTrialKit

enum ProcessorExperiments {
    /// Where the harness pretends the host's playhead is.
    private static let beat = UnsafeMutablePointer<Double>.allocate(capacity: 1)
    private static let tempo = UnsafeMutablePointer<Double>.allocate(capacity: 1)
    private static let moving = UnsafeMutablePointer<Double>.allocate(capacity: 1)
    /// Where emitted MIDI lands. Preallocated, because the block that writes
    /// into it runs on the render thread.
    private static let emitted = UnsafeMutablePointer<UInt8>.allocate(capacity: 4_096)
    private static let emittedCount = UnsafeMutablePointer<Int>.allocate(capacity: 1)

    static func run(into report: Report) {
        beat.initialize(to: 0)
        tempo.initialize(to: 120)
        moving.initialize(to: 1)
        emitted.initialize(repeating: 0, count: 4_096)
        emittedCount.initialize(to: 0)

        guard let unit = try? TrialProcessorAudioUnit(componentDescription: musaTrialProcessorDescription) else {
            report.check("processor.instantiate", "The MIDI Processor loads in this process", false, "initializer threw")
            return
        }
        report.check("processor.instantiate", "The MIDI Processor loads in this process", true,
                     "\(unit.occurrenceCount) occurrences published")

        report.check("processor.schedule.randomAccess", "The schedule is addressed by position, not by replay",
                     unit.index(forBeat: 0) == 0 && unit.index(forBeat: 2.0) == 4 && unit.index(forBeat: 3.5) == 7,
                     "beat 0 → 0, beat 2 → 4, beat 3.5 → 7")

        install(on: unit)
        try? unit.allocateRenderResources()
        let rig = RenderRig(unit: unit)

        // A block at the beginning, then a block after a seek. The second
        // must start its search where the host is, not where the first ended.
        beat.pointee = 0
        unit.clearTrace()
        emittedCount.pointee = 0
        _ = rig.renderQuietly(frames: 4_096, sampleTime: 0)
        let fromStart = (start: unit.lastSearchStart, emitted: unit.lastEmitted)

        beat.pointee = 2.0
        unit.clearTrace()
        emittedCount.pointee = 0
        _ = rig.renderQuietly(frames: 4_096, sampleTime: 100_000)
        let afterSeek = (start: unit.lastSearchStart, emitted: unit.lastEmitted)

        report.check("processor.seek.doesNotReplay", "A seek searches to the host's position rather than replaying",
                     fromStart.start == 0 && afterSeek.start == 4 && afterSeek.emitted > 0,
                     "from the beginning the search started at \(fromStart.start); after a seek to beat 2 it started at \(afterSeek.start)",
                     ["startAtZero": Double(fromStart.start), "startAfterSeek": Double(afterSeek.start)])

        report.check("processor.emitsHostMIDI", "Scheduled occurrences leave through the host's MIDI output block",
                     emittedCount.pointee > 0 && emittedCount.pointee % 3 == 0,
                     "\(emittedCount.pointee / 3) messages after the seek",
                     ["messages": Double(emittedCount.pointee / 3)])

        // A stopped transport.
        beat.pointee = 0
        moving.pointee = 0
        unit.clearTrace()
        emittedCount.pointee = 0
        _ = rig.renderQuietly(frames: 4_096, sampleTime: 200_000)
        report.check("processor.transportStopped", "A stopped transport emits nothing",
                     emittedCount.pointee == 0,
                     "\(emittedCount.pointee) bytes emitted while the host reported stopped")
        moving.pointee = 1

        // No musical context at all.
        guard let bare = try? TrialProcessorAudioUnit(componentDescription: musaTrialProcessorDescription) else { return }
        bare.midiOutputEventBlock = unit.midiOutputEventBlock
        try? bare.allocateRenderResources()
        let bareRig = RenderRig(unit: bare)
        emittedCount.pointee = 0
        _ = bareRig.renderQuietly(frames: 512, sampleTime: 0)
        report.check("processor.missingContext", "No host musical context emits nothing and says so",
                     bare.sawMissingContext && emittedCount.pointee == 0,
                     "the component recorded the absence instead of assuming a tempo")
    }

    private static func install(on unit: TrialProcessorAudioUnit) {
        let beat = self.beat
        let tempo = self.tempo
        let moving = self.moving
        let emitted = self.emitted
        let emittedCount = self.emittedCount
        unit.musicalContextBlock = { currentTempo, _, _, currentBeat, _, _ in
            currentTempo?.pointee = tempo.pointee
            currentBeat?.pointee = beat.pointee
            return true
        }
        unit.transportStateBlock = { flags, currentSample, _, _ in
            flags?.pointee = moving.pointee > 0 ? [.moving] : []
            currentSample?.pointee = 0
            return true
        }
        unit.midiOutputEventBlock = { _, _, length, bytes in
            let at = emittedCount.pointee
            guard at + length <= 4_096 else { return noErr }
            for index in 0..<length {
                emitted[at + index] = bytes[index]
            }
            emittedCount.pointee = at + length
            return noErr
        }
    }
}
