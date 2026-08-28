/// The extension's principal class.
///
/// Everything it does is name an audio unit. The rest of the appex is empty
/// on purpose: a bundle that carries logic is a bundle that has to be
/// reasoned about twice, once here and once in the framework both this and
/// the containing app link.
///
/// One extension publishes both Musa components. That is not a design
/// preference — a containing app that ships two `AudioComponents`-declaring
/// extensions registers only one of them on this system, and registering
/// the second removes the first. `docs/notes/toolchain/two-audio-units-one-container.md`
/// records the measurement. One bundle is still not one component: the two
/// audio units share no mutable state, and the factory below is the only
/// place they meet.

import AudioToolbox
import Foundation
import MusaAudioUnitKit

/// The principal class named by `Config/Extension-Info.plist`. The
/// `@objc` name is not decoration: without it the runtime name is
/// module-qualified, the system cannot find the class, and the component
/// fails to open with `-50` rather than saying what is wrong.
@objc(MusaComponentFactory)
public final class MusaComponentFactory: NSObject, AUAudioUnitFactory {
    /// Which audio unit to build is the host's question, already answered:
    /// it opened a component, and the description it opened carries the
    /// type. Reading it here is cheaper and more honest than a second
    /// registry of our own.
    public func createAudioUnit(with componentDescription: AudioComponentDescription) throws -> AUAudioUnit {
        switch componentDescription.componentType {
        case kAudioUnitType_MusicDevice:
            return try MusaInstrumentAudioUnit(componentDescription: componentDescription, options: [])
        case kAudioUnitType_MIDIProcessor:
            return try MusaProcessorAudioUnit(componentDescription: componentDescription, options: [])
        default:
            throw NSError(
                domain: NSOSStatusErrorDomain,
                code: Int(kAudioUnitErr_InvalidElement),
                userInfo: [NSLocalizedDescriptionKey: "Musa publishes no component of this type"]
            )
        }
    }

    /// Required by `NSExtensionRequestHandling`. An Audio Unit extension is
    /// driven entirely through `createAudioUnit`, so there is nothing here.
    public func beginRequest(with context: NSExtensionContext) {}
}
