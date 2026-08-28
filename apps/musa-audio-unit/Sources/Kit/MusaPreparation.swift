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
    /// The address table a restored document was saved with, if it had one.
    ///
    /// Carried through rather than interpreted: it is the library that
    /// decides which control keeps which address, and a component that
    /// second-guessed it would be the one moving a host's automation.
    public var controlTable: String?

    public init(
        project: String,
        piece: String? = nil,
        part: String,
        sampleRate: Double,
        expectedMusicIdentity: String? = nil,
        expectedAssetIdentity: String? = nil,
        controlTable: String? = nil
    ) {
        self.project = project
        self.piece = piece
        self.part = part
        self.sampleRate = sampleRate
        self.expectedMusicIdentity = expectedMusicIdentity
        self.expectedAssetIdentity = expectedAssetIdentity
        self.controlTable = controlTable
    }
}

/// One source-declared control, as a host parameter would describe it.
///
/// Everything here came from the declaration `musa-project` projected under
/// Rule D2. There is no catalogue on this side and nothing is defaulted here:
/// a field this struct cannot fill is a field the source did not write.
public struct MusaControl: Equatable, Sendable {
    /// The canonical source identity — the thing an address is keyed by.
    public let identity: String
    /// What the declaration calls it.
    public let display: String
    /// The sentence the declaration wrote about it.
    public let summary: String
    /// The declared control kind, spelled as the source spells it.
    public let kind: String
    /// How often the declaration says it may move.
    public let updateRate: String
    /// The stable address a host's automation points at.
    public let address: UInt64
    public let minimum: Float
    public let maximum: Float
    public let defaultValue: Float
    /// Whether the declaration calls it continuous — the only thing that
    /// makes a host ramp meaningful.
    public let continuous: Bool

    /// The identifier a parameter tree may carry.
    ///
    /// `AUParameterNode` treats a period as a key-path separator, and a
    /// canonical identity has two of them plus a `::`. So the punctuation is
    /// folded to underscores, deterministically, and `identity` remains the
    /// thing addresses and tables are keyed by. Nothing reads this back.
    public var parameterIdentifier: String {
        String(identity.map { $0 == "." || $0 == ":" ? "_" : $0 })
    }
}

/// One projected output bus.
public struct MusaOutput: Equatable, Sendable {
    /// The declared name: a part, a room, the main output. Never renamed to
    /// a workstation's word for a track.
    public let name: String
    public let role: MusaOutputRole
}

/// What a projected output *is* in the source.
public enum MusaOutputRole: UInt32, Sendable {
    /// The piece's main output. Bus zero, always present.
    case main = 0
    /// This part's own output.
    case part = 1
    /// A studio bus the part reaches.
    case bus = 2
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
    /// The source-declared controls this preparation admitted as parameters.
    public let controls: [MusaControl]
    /// The declared controls it could not, each already a sentence.
    public let controlLosses: [String]
    /// The address table these controls were published under, to be saved
    /// with the document and handed back on the next preparation.
    public let controlTable: String
    /// The outputs this instrument reaches, in bus order, bus zero first.
    public let outputs: [MusaOutput]
}

/// Prepares instruments, one at a time, off the render thread.
public final class MusaPreparer {
    private let queue = DispatchQueue(label: "dev.musa.audiounit.preparation", qos: .userInitiated)

    public init() {}

    /// A C string for a value that may not be there.
    ///
    /// The optional arguments this ABI takes are genuinely optional — a piece
    /// nobody named, a table nothing saved — and `nil` is the spelling of
    /// that. Nesting `withCString` by hand for each one is how one of them
    /// ends up as an empty string that the library then has to interpret.
    static func withOptionalCString<R>(_ value: String?, _ body: (UnsafePointer<CChar>?) -> R) -> R {
        guard let value else { return body(nil) }
        return value.withCString { body($0) }
    }

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
                inputs: [],
                controls: [],
                controlLosses: [],
                controlTable: "",
                outputs: []
            )
        }
        let rate = UInt32(max(1, selection.sampleRate.rounded()))
        let preparation = selection.project.withCString { project in
            selection.part.withCString { part in
                withOptionalCString(selection.piece) { piece in
                    withOptionalCString(selection.controlTable) { table in
                        musa_au_prepare(project, piece, part, rate, table)
                    }
                }
            }
        }
        guard let preparation else {
            return MusaPreparationResult(
                instrument: nil,
                refusal: "the Musa library returned nothing at all",
                musicIdentity: "",
                assetIdentity: "",
                inputs: [],
                controls: [],
                controlLosses: [],
                controlTable: "",
                outputs: []
            )
        }
        defer { musa_au_preparation_release(preparation) }

        guard musa_au_preparation_ok(preparation) != 0 else {
            return MusaPreparationResult(
                instrument: nil,
                refusal: String(cString: musa_au_preparation_message(preparation)),
                musicIdentity: "",
                assetIdentity: "",
                inputs: [],
                controls: [],
                controlLosses: [],
                controlTable: "",
                outputs: []
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
                inputs: [],
                controls: [],
                controlLosses: [],
                controlTable: "",
                outputs: []
            )
        }
        if let expected = selection.expectedAssetIdentity, expected != assets {
            return MusaPreparationResult(
                instrument: nil,
                refusal: "this state was saved with assets \(expected); the closure here is \(assets)",
                musicIdentity: music,
                assetIdentity: assets,
                inputs: [],
                controls: [],
                controlLosses: [],
                controlTable: "",
                outputs: []
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
        let controls = readControls(preparation)
        let losses = readTexts(preparation, musa_au_preparation_loss_count, musa_au_preparation_loss)
        let table = String(cString: musa_au_preparation_control_table(preparation))
        let outputs = readOutputs(preparation)

        guard let instrument = musa_au_preparation_take(preparation) else {
            return MusaPreparationResult(
                instrument: nil,
                refusal: "the preparation succeeded but yielded no instrument",
                musicIdentity: music,
                assetIdentity: assets,
                inputs: inputs,
                controls: controls,
                controlLosses: losses,
                controlTable: table,
                outputs: outputs
            )
        }
        return MusaPreparationResult(
            instrument: instrument,
            refusal: "",
            musicIdentity: music,
            assetIdentity: assets,
            inputs: inputs,
            controls: controls,
            controlLosses: losses,
            controlTable: table,
            outputs: outputs
        )
    }

    /// Every published control, in the order the declaration wrote them.
    ///
    /// No filtering happens here. What may be a parameter was decided by
    /// Rule D2 in `musa-project`; a component that dropped one more would be
    /// a second, undocumented admission rule.
    private static func readControls(_ preparation: OpaquePointer) -> [MusaControl] {
        var controls: [MusaControl] = []
        let count = musa_au_preparation_control_count(preparation)
        controls.reserveCapacity(Int(count))
        for index in 0..<count {
            var numbers = MusaAuControl()
            guard musa_au_preparation_control(preparation, index, &numbers) != 0 else { continue }
            controls.append(
                MusaControl(
                    identity: text(musa_au_preparation_control_identity(preparation, index)),
                    display: text(musa_au_preparation_control_display(preparation, index)),
                    summary: text(musa_au_preparation_control_summary(preparation, index)),
                    kind: text(musa_au_preparation_control_kind(preparation, index)),
                    updateRate: text(musa_au_preparation_control_update_rate(preparation, index)),
                    address: numbers.address,
                    minimum: numbers.minimum,
                    maximum: numbers.maximum,
                    defaultValue: numbers.default_value,
                    continuous: numbers.flags & UInt32(MUSA_AU_CONTROL_CONTINUOUS) != 0
                )
            )
        }
        return controls
    }

    private static func readOutputs(_ preparation: OpaquePointer) -> [MusaOutput] {
        var outputs: [MusaOutput] = []
        let count = musa_au_preparation_output_count(preparation)
        outputs.reserveCapacity(Int(count))
        for index in 0..<count {
            guard let name = musa_au_preparation_output(preparation, index) else { continue }
            let role = MusaOutputRole(rawValue: musa_au_preparation_output_role(preparation, index)) ?? .bus
            outputs.append(MusaOutput(name: String(cString: name), role: role))
        }
        return outputs
    }

    private static func readTexts(
        _ preparation: OpaquePointer,
        _ count: (OpaquePointer?) -> UInt32,
        _ at: (OpaquePointer?, UInt32) -> UnsafePointer<CChar>?
    ) -> [String] {
        var texts: [String] = []
        let total = count(preparation)
        texts.reserveCapacity(Int(total))
        for index in 0..<total {
            if let value = at(preparation, index) {
                texts.append(String(cString: value))
            }
        }
        return texts
    }

    private static func text(_ value: UnsafePointer<CChar>?) -> String {
        value.map { String(cString: $0) } ?? ""
    }
}
