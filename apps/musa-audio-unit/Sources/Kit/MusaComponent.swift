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

/// The MIDI Processor: `aumi musp Musa`.
///
/// A different component from the Music Device, not a mode of it. One
/// projects a piece as MIDI for the host to route; the other plays MIDI the
/// host sends. They share this framework and no mutable state.
public var musaProcessorDescription: AudioComponentDescription {
    AudioComponentDescription(
        componentType: fourCharacterCode("aumi"),
        componentSubType: fourCharacterCode("musp"),
        componentManufacturer: fourCharacterCode(musaManufacturer),
        componentFlags: 0,
        componentFlagsMask: 0
    )
}

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
    /// The address table this component's parameters were published under.
    ///
    /// Saved because an address is what a host's automation lane points at.
    /// A document that carried only the source path would come back with
    /// whatever addresses this version of the projection happened to derive,
    /// and every automation lane written against the old ones would move to
    /// the wrong control — silently, which is the worst way for it to
    /// happen. The table is the library's own encoding, opaque here.
    public static let controlTable = "musa.state.controlTable"
    /// The controls the source declares that could not become parameters,
    /// each already a sentence saying which and why.
    ///
    /// `06-daw-boundary.md` §6: a loss is refused or recorded, never left as
    /// "some information may be lost".
    public static let controlLosses = "musa.state.controlLosses"
    /// The outputs this component projected, in bus order.
    public static let outputs = "musa.state.outputs"
    /// Which reading of the piece a processor projects: `score` or
    /// `performance`.
    public static let scheduleMode = "musa.state.scheduleMode"
    /// Which timeline it counts positions on: `piece` or `host`.
    ///
    /// Saved because it is a choice and never a default. A document that came
    /// back on the other timeline would be the same notes at different
    /// moments, which is a different piece of music.
    public static let scheduleTimeline = "musa.state.scheduleTimeline"
    /// The parts the projection carries, each as `name` and its channel.
    public static let parts = "musa.state.parts"
    /// Why the component is silent, when it is.
    public static let refusal = "musa.state.refusal"
}

/// The version of the document schema above.
///
/// Version 2 added the control table, the projection losses, and the output
/// names. A version 1 document is still read: it named a source and a part,
/// which is everything needed to prepare, and it carried no automation
/// addresses to preserve because that version published no parameters.
///
/// Version 3 added the MIDI Processor's reading and timeline. The two
/// components write disjoint keys into one schema rather than two, because a
/// host that hands the wrong dictionary back should find a key it does not
/// understand rather than a key that means something else.
public let musaStateVersion = 3
