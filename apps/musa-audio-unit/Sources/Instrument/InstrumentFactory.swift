/// The extension's principal class.
///
/// Everything it does is name the audio unit. The rest of the appex is empty
/// on purpose: a bundle that carries logic is a bundle that has to be
/// reasoned about twice, once here and once in the framework both this and
/// the containing app link.

import AudioToolbox
import Foundation
import MusaAudioUnitKit

/// The principal class named by `Config/Instrument-Info.plist`. The
/// `@objc` name is not decoration: without it the runtime name is
/// module-qualified, the system cannot find the class, and the component
/// fails to open with `-50` rather than saying what is wrong.
@objc(MusaInstrumentFactory)
public final class MusaInstrumentFactory: NSObject, AUAudioUnitFactory {
    public func createAudioUnit(with componentDescription: AudioComponentDescription) throws -> AUAudioUnit {
        try MusaInstrumentAudioUnit(componentDescription: componentDescription, options: [])
    }

    /// Required by `NSExtensionRequestHandling`. An Audio Unit extension is
    /// driven entirely through `createAudioUnit`, so there is nothing here.
    public func beginRequest(with context: NSExtensionContext) {}
}
