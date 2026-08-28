/// The control side of the MIDI Processor: opening a piece, off the render
/// thread.
///
/// The mirror of `MusaPreparation.swift`, and for the same reasons. Opening a
/// schedule compiles source, verifies an asset closure, and builds the whole
/// piece as an array of messages; all of that reads files and allocates, so it
/// happens on a serial worker and reaches the render block only as a pointer
/// published through `MusaAuScheduleSlot`. Releasing one is destruction, which
/// `06-daw-boundary.md` §7 puts on the same list as allocation, so that
/// happens here too.

import Foundation

/// Which reading of a piece a projection carries.
public enum MusaScheduleMode: String, Sendable, CaseIterable {
    /// The piece as written: notated durations, dynamics uninterpreted.
    case score
    /// The piece as played: the performance a checked source describes.
    case performance

    var code: UInt32 {
        switch self {
        case .score: return UInt32(MUSA_AU_MIDI_SCORE)
        case .performance: return UInt32(MUSA_AU_MIDI_PERFORMANCE)
        }
    }
}

/// Which timeline a projection's positions are counted on.
///
/// Never inferred and never defaulted at this boundary. `06-daw-boundary.md`
/// §6 forbids a silent flattening, and choosing between these two decides
/// whether the host's tempo changes what the piece is.
public enum MusaScheduleTimeline: String, Sendable, CaseIterable {
    /// Seconds on the piece's own exact physical schedule. The host's tempo
    /// is not consulted, so the piece plays at the speed its source says.
    case piece
    /// Quarter notes on the host's musical timeline, so the piece follows the
    /// host's tempo — and a polytempo piece is refused rather than flattened.
    case host

    var code: UInt32 {
        switch self {
        case .piece: return UInt32(MUSA_AU_TIMELINE_PIECE)
        case .host: return UInt32(MUSA_AU_TIMELINE_HOST)
        }
    }
}

/// What a processor was asked to project, as a restored document names it.
public struct MusaScheduleSelection: Equatable, Sendable {
    /// The project directory or `.musa` file.
    public var project: String
    /// Which piece of it; `nil` means the first.
    public var piece: String?
    /// Which reading.
    public var mode: MusaScheduleMode
    /// Which timeline.
    public var timeline: MusaScheduleTimeline
    /// The identities a restored document expects, if it had any.
    public var expectedMusicIdentity: String?
    public var expectedAssetIdentity: String?

    public init(
        project: String,
        piece: String? = nil,
        mode: MusaScheduleMode = .performance,
        timeline: MusaScheduleTimeline = .piece,
        expectedMusicIdentity: String? = nil,
        expectedAssetIdentity: String? = nil
    ) {
        self.project = project
        self.piece = piece
        self.mode = mode
        self.timeline = timeline
        self.expectedMusicIdentity = expectedMusicIdentity
        self.expectedAssetIdentity = expectedAssetIdentity
    }
}

/// One part of the projection, and the channel every message of it carries.
public struct MusaSchedulePart: Equatable, Sendable {
    public let name: String
    public let channel: UInt32
}

/// What one opening produced.
public struct MusaScheduleResult: @unchecked Sendable {
    /// The opened schedule, or nil.
    public let schedule: OpaquePointer?
    /// Why there is not one; empty when there is.
    public let refusal: String
    public let musicIdentity: String
    public let assetIdentity: String
    public let pieceName: String
    public let parts: [MusaSchedulePart]
    /// What this projection could not carry, each already a sentence.
    public let losses: [String]
    /// How many messages the whole piece is.
    public let count: Int
    /// Where it ends, in the timeline's own unit.
    public let extent: Double
}

/// The serial worker a processor opens and retires schedules on.
public final class MusaScheduleWorker: @unchecked Sendable {
    private let queue = DispatchQueue(label: "dev.musa.audiounit.schedule", qos: .userInitiated)

    public init() {}

    /// Open `selection`, calling `completion` on the worker.
    public func open(_ selection: MusaScheduleSelection, completion: @escaping @Sendable (MusaScheduleResult) -> Void) {
        queue.async { completion(Self.openNow(selection)) }
    }

    /// Retire a schedule the render block is no longer reading.
    public func retire(_ schedule: OpaquePointer?) {
        guard let schedule else { return }
        queue.async { musa_au_schedule_release(schedule) }
    }

    /// The same opening, synchronously. For the automated host, which is a
    /// program and not a callback.
    public static func openNow(_ selection: MusaScheduleSelection) -> MusaScheduleResult {
        guard musa_au_abi_version() == UInt32(MUSA_AU_ABI_VERSION) else {
            return refused(
                "this component was built against Musa ABI \(MUSA_AU_ABI_VERSION) "
                    + "and loaded version \(musa_au_abi_version())"
            )
        }
        let opened = selection.project.withCString { project in
            MusaPreparer.withOptionalCString(selection.piece) { piece in
                musa_au_open_schedule(project, piece, selection.mode.code, selection.timeline.code)
            }
        }
        guard let opened else {
            return refused("the Musa library returned nothing at all")
        }
        guard musa_au_schedule_ok(opened) != 0 else {
            let message = String(cString: musa_au_schedule_message(opened))
            musa_au_schedule_release(opened)
            return refused(message)
        }
        let music = String(cString: musa_au_schedule_identity_music(opened))
        let assets = String(cString: musa_au_schedule_identity_assets(opened))

        // Rule D1: the source is the work. A document naming a different
        // closure is a document about a different piece.
        if let expected = selection.expectedMusicIdentity, expected != music {
            musa_au_schedule_release(opened)
            return refused("this state was saved from music \(expected); the source here is \(music)")
        }
        if let expected = selection.expectedAssetIdentity, expected != assets {
            musa_au_schedule_release(opened)
            return refused("this state was saved with assets \(expected); the closure here is \(assets)")
        }

        var parts: [MusaSchedulePart] = []
        let partCount = musa_au_schedule_part_count(opened)
        parts.reserveCapacity(Int(partCount))
        for index in 0..<partCount {
            guard let name = musa_au_schedule_part_name(opened, index) else { continue }
            parts.append(
                MusaSchedulePart(name: String(cString: name), channel: musa_au_schedule_part_channel(opened, index))
            )
        }
        var losses: [String] = []
        let lossCount = musa_au_schedule_loss_count(opened)
        losses.reserveCapacity(Int(lossCount))
        for index in 0..<lossCount {
            guard let loss = musa_au_schedule_loss(opened, index) else { continue }
            losses.append(String(cString: loss))
        }
        return MusaScheduleResult(
            schedule: opened,
            refusal: "",
            musicIdentity: music,
            assetIdentity: assets,
            pieceName: String(cString: musa_au_schedule_identity_piece(opened)),
            parts: parts,
            losses: losses,
            count: Int(musa_au_schedule_count(opened)),
            extent: musa_au_schedule_extent(opened)
        )
    }

    private static func refused(_ why: String) -> MusaScheduleResult {
        MusaScheduleResult(
            schedule: nil,
            refusal: why,
            musicIdentity: "",
            assetIdentity: "",
            pieceName: "",
            parts: [],
            losses: [],
            count: 0,
            extent: 0
        )
    }
}
