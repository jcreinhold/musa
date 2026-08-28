/// The control side: what happens off the render thread.
///
/// Preparation compiles source, verifies an asset closure, and builds a
/// prepared graph. All of that reads files and allocates, so it happens here,
/// on a serial worker, and reaches the render block only as a pointer
/// published through `MusaAuSlot`.
///
/// Retirement happens here too, and that is the half people forget: releasing
/// a prepared graph is destruction, and `06-daw-boundary.md` §7 puts
/// destruction on the same list as allocation.

import Foundation

/// What a component was asked to render, as a restored document names it.
public struct MusaSelection: Equatable, Sendable {
    /// The project directory or `.musa` file.
    public var project: String
    /// Which piece of it; `nil` means the first.
    public var piece: String?
    /// Which part of that piece.
    public var part: String
    /// The rate the host will render at.
    public var sampleRate: Double
    /// The identities a restored document expects, if it had any. A
    /// preparation that does not match them is refused rather than rendered.
    public var expectedMusicIdentity: String?
    public var expectedAssetIdentity: String?

    public init(
        project: String,
        piece: String? = nil,
        part: String,
        sampleRate: Double,
        expectedMusicIdentity: String? = nil,
        expectedAssetIdentity: String? = nil
    ) {
        self.project = project
        self.piece = piece
        self.part = part
        self.sampleRate = sampleRate
        self.expectedMusicIdentity = expectedMusicIdentity
        self.expectedAssetIdentity = expectedAssetIdentity
    }
}

/// What one preparation produced.
public struct MusaPreparationResult: Sendable {
    /// The prepared instrument, or `nil` when there is not one.
    public let instrument: OpaquePointer?
    /// Why there is not one, or why it was refused. Empty on success.
    public let refusal: String
    /// What it was prepared from.
    public let musicIdentity: String
    public let assetIdentity: String
    /// The MIDI dimensions the instrument's source binds.
    public let inputs: [String]
}

/// Prepares instruments, one at a time, off the render thread.
public final class MusaPreparer {
    private let queue = DispatchQueue(label: "dev.musa.audiounit.preparation", qos: .userInitiated)

    public init() {}

    /// Prepare `selection`, calling `completion` on the worker.
    ///
    /// Nothing about this is real-time safe, which is the point of it being
    /// somewhere else.
    public func prepare(_ selection: MusaSelection, completion: @escaping @Sendable (MusaPreparationResult) -> Void) {
        queue.async { completion(Self.prepareNow(selection)) }
    }

    /// Retire an instrument the render block is no longer reading.
    ///
    /// Asynchronous because releasing is destruction, and the caller may be
    /// holding a lock the render thread would rather it did not.
    public func retire(_ instrument: OpaquePointer?) {
        guard let instrument else { return }
        queue.async { musa_au_instrument_release(instrument) }
    }

    /// The same preparation, synchronously. For the automated host, which is
    /// a program and not a callback.
    public static func prepareNow(_ selection: MusaSelection) -> MusaPreparationResult {
        guard musa_au_abi_version() == UInt32(MUSA_AU_ABI_VERSION) else {
            return MusaPreparationResult(
                instrument: nil,
                refusal: "this component was built against Musa ABI \(MUSA_AU_ABI_VERSION) "
                    + "and loaded version \(musa_au_abi_version())",
                musicIdentity: "",
                assetIdentity: "",
                inputs: []
            )
        }
        let rate = UInt32(max(1, selection.sampleRate.rounded()))
        let preparation = selection.project.withCString { project in
            selection.part.withCString { part in
                if let piece = selection.piece {
                    return piece.withCString { musa_au_prepare(project, $0, part, rate) }
                }
                return musa_au_prepare(project, nil, part, rate)
            }
        }
        guard let preparation else {
            return MusaPreparationResult(
                instrument: nil,
                refusal: "the Musa library returned nothing at all",
                musicIdentity: "",
                assetIdentity: "",
                inputs: []
            )
        }
        defer { musa_au_preparation_release(preparation) }

        guard musa_au_preparation_ok(preparation) != 0 else {
            return MusaPreparationResult(
                instrument: nil,
                refusal: String(cString: musa_au_preparation_message(preparation)),
                musicIdentity: "",
                assetIdentity: "",
                inputs: []
            )
        }
        let music = String(cString: musa_au_preparation_identity_music(preparation))
        let assets = String(cString: musa_au_preparation_identity_assets(preparation))

        // Rule D1: the source is the work. A document that named a different
        // closure is a document about a different piece, and rendering it
        // anyway would be this component deciding which piece the composer
        // meant.
        if let expected = selection.expectedMusicIdentity, expected != music {
            return MusaPreparationResult(
                instrument: nil,
                refusal: "this state was saved from music \(expected); the source here is \(music)",
                musicIdentity: music,
                assetIdentity: assets,
                inputs: []
            )
        }
        if let expected = selection.expectedAssetIdentity, expected != assets {
            return MusaPreparationResult(
                instrument: nil,
                refusal: "this state was saved with assets \(expected); the closure here is \(assets)",
                musicIdentity: music,
                assetIdentity: assets,
                inputs: []
            )
        }

        var inputs: [String] = []
        let count = musa_au_preparation_input_count(preparation)
        inputs.reserveCapacity(Int(count))
        for index in 0..<count {
            if let name = musa_au_preparation_input(preparation, index) {
                inputs.append(String(cString: name))
            }
        }
        guard let instrument = musa_au_preparation_take(preparation) else {
            return MusaPreparationResult(
                instrument: nil,
                refusal: "the preparation succeeded but yielded no instrument",
                musicIdentity: music,
                assetIdentity: assets,
                inputs: inputs
            )
        }
        return MusaPreparationResult(
            instrument: instrument,
            refusal: "",
            musicIdentity: music,
            assetIdentity: assets,
            inputs: inputs
        )
    }
}
