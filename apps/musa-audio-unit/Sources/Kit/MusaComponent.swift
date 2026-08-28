/// What this component is, and what its saved document says.
///
/// One file so that the extension, the containing app, and the automated host
/// all name the same things; a second spelling of a four-character code is
/// how a component becomes undiscoverable for an afternoon.

import AudioToolbox
import Foundation

/// The manufacturer code prompt 215 registered and `auval` validated.
///
/// Not all-lowercase: Apple reserves that space. Not `MUSA` either — the
/// trial registered `Musa`, and this is the code that has been validated on a
/// real system rather than the one that reads best.
public let musaManufacturer = "Musa"

/// The Music Device: `aumu musa Musa`.
public var musaInstrumentDescription: AudioComponentDescription {
    AudioComponentDescription(
        componentType: fourCharacterCode("aumu"),
        componentSubType: fourCharacterCode("musa"),
        componentManufacturer: fourCharacterCode(musaManufacturer),
        componentFlags: 0,
        componentFlagsMask: 0
    )
}

/// A four-character code as Core Audio wants it.
public func fourCharacterCode(_ text: String) -> OSType {
    text.utf8.reduce(OSType(0)) { code, byte in (code << 8) | OSType(byte) }
}

/// The inverse, for a diagnostic that has to name what it saw.
public func fourCharacterText(_ code: OSType) -> String {
    String(decoding: [3, 2, 1, 0].map { UInt8((code >> (8 * $0)) & 0xFF) }, as: UTF8.self)
}

/// Keys in the document state this component saves and restores.
///
/// Prompt 215 measured what may be in here: a property-list-safe dictionary
/// merged onto `super.fullState`, carrying identities and never asset bytes.
public enum MusaStateKey {
    /// The version of this dictionary's own schema.
    public static let version = "musa.state.version"
    /// The ABI version the component that wrote it was built against.
    public static let abiVersion = "musa.state.abiVersion"
    /// Where the source lives, as the containing app granted it.
    public static let project = "musa.state.project"
    /// A security-scoped bookmark to that location.
    ///
    /// Prompt 215 measured that a sandboxed extension reading inside its App
    /// Group container never returns to the host, and that a real App Group
    /// identifier is team-prefixed and provisioned by Apple. So access
    /// arrives *with* the restored state, granted by the containing app,
    /// rather than being looked up by a component that went searching.
    public static let access = "musa.state.access"
    /// Which piece of that project.
    public static let piece = "musa.state.piece"
    /// Which part of that piece.
    public static let part = "musa.state.part"
    /// The compiled music's semantic identity.
    public static let musicIdentity = "musa.state.musicIdentity"
    /// The verified asset closure's identity.
    public static let assetIdentity = "musa.state.assetIdentity"
    /// The MIDI dimensions the instrument's source binds.
    public static let inputs = "musa.state.inputs"
    /// Why the component is silent, when it is.
    public static let refusal = "musa.state.refusal"
}

/// The version of the document schema above.
public let musaStateVersion = 1
