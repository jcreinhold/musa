//  The extension point the system instantiates.
//
//  It exists only to hand back the Music Device that lives in the shared
//  framework, so the class the host loads out of process is byte for byte the
//  class the harness loads in process.

import AudioToolbox
import Foundation
import MusaTrialKit

/// The principal class named by `Config/Instrument-Info.plist`.
@objc(TrialInstrumentFactory)
public final class TrialInstrumentFactory: NSObject, AUAudioUnitFactory {
    /// `AUAudioUnitFactory` inherits `NSExtensionRequestHandling`, and the
    /// system never sends this extension point a request: the Audio Unit is
    /// asked for below, and nothing else is.
    public func beginRequest(with context: NSExtensionContext) {}

    public func createAudioUnit(with componentDescription: AudioComponentDescription) throws -> AUAudioUnit {
        try TrialInstrumentAudioUnit(componentDescription: componentDescription, options: [])
    }
}
