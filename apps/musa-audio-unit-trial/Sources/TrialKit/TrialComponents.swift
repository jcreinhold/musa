//  What the two trial components are, in one place.
//
//  The four-character codes here must agree with `Config/*-Info.plist`. They
//  are repeated rather than shared because an `Info.plist` is read by the
//  system before any of this code runs, and a trial that hid the duplication
//  behind a build setting would be trialling the build setting.

import AudioToolbox
import Foundation

/// The manufacturer code every Musa component is published under.
public let musaTrialManufacturer: OSType = fourCharacterCode("Musa")

/// The trial Music Device: host MIDI in, stereo out.
public let musaTrialInstrumentDescription = AudioComponentDescription(
    componentType: kAudioUnitType_MusicDevice,
    componentSubType: fourCharacterCode("musi"),
    componentManufacturer: musaTrialManufacturer,
    componentFlags: 0,
    componentFlagsMask: 0
)

/// The trial MIDI Processor: host transport in, scheduled MIDI out.
public let musaTrialProcessorDescription = AudioComponentDescription(
    componentType: kAudioUnitType_MIDIProcessor,
    componentSubType: fourCharacterCode("musp"),
    componentManufacturer: musaTrialManufacturer,
    componentFlags: 0,
    componentFlagsMask: 0
)

/// A four-character code, the way Core Audio spells an identity.
public func fourCharacterCode(_ text: String) -> OSType {
    var code: OSType = 0
    for byte in text.utf8.prefix(4) {
        code = (code << 8) | OSType(byte)
    }
    return code
}

/// The same identity spelled back, for a report a person reads.
public func fourCharacterText(_ code: OSType) -> String {
    let bytes = [UInt8((code >> 24) & 0xFF), UInt8((code >> 16) & 0xFF), UInt8((code >> 8) & 0xFF), UInt8(code & 0xFF)]
    return String(decoding: bytes, as: UTF8.self)
}

/// The App Group both sides of the trial look for an asset in.
///
/// `06-daw-boundary.md` §3 keeps asset bytes in the verified store and names
/// them by digest in state. That store has to be somewhere a sandboxed
/// extension can read, and on macOS an App Group container is the smallest
/// arrangement that reaches one.
public let musaTrialAppGroup = "group.dev.musa.audiounittrial"

/// What the trial writes into that container, and reads back out.
public let musaTrialAssetName = "trial-asset.txt"

/// Name the App Group container this component was told its assets live in.
///
/// It stops at naming, and that is a measured decision rather than a partial
/// implementation. An ad-hoc-signed extension can *name* the container and is
/// taken down by the system the moment it reads a file inside it, because a
/// real App Group identifier is team-prefixed and provisioned by Apple. Both
/// halves of that are recorded in `docs/notes/research/93-the-audio-unit-shape.md`
/// §4, and the production consequence is stated there: a component reaches
/// its assets through what the host restored, not through a container it
/// went looking for.
public func describeTrialAssetStore() -> String {
    guard let container = FileManager.default.containerURL(forSecurityApplicationGroupIdentifier: musaTrialAppGroup) else {
        return "refused: no container for \(musaTrialAppGroup)"
    }
    return "named \(container.lastPathComponent)/\(musaTrialAssetName)"
}

/// The keys a saved component state uses.
///
/// `docs/rules/across-stages/06-daw-boundary.md` §3 makes saved state a
/// presentation like any other: it names the project, source, and lock
/// identities it was made from, and restoring it against a different closure
/// is refused with those identities rather than approximated. These are the
/// names the trial writes so the refusal can be measured.
public enum TrialStateKey {
    public static let version = "musa.state.version"
    public static let abiVersion = "musa.state.abiVersion"
    public static let projectIdentity = "musa.state.projectIdentity"
    public static let sourceIdentity = "musa.state.sourceIdentity"
    public static let lockIdentity = "musa.state.lockIdentity"
    public static let sourceClosure = "musa.state.sourceClosure"
    public static let selectedDeclaration = "musa.state.selectedDeclaration"
    public static let assetDigests = "musa.state.assetDigests"
    public static let assetStore = "musa.state.assetStore"
    public static let parameters = "musa.state.parameters"
    public static let parameterAddress = "address"
    public static let parameterIdentifier = "identifier"
    public static let parameterSourceIdentity = "sourceIdentity"
    public static let parameterValue = "value"
}

/// The version of the state schema above.
public let musaTrialStateVersion = 1
