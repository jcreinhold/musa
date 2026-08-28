//  The extension point the system instantiates for the MIDI Processor.

import AudioToolbox
import Foundation
import MusaTrialKit

/// The principal class named by `Config/Processor-Info.plist`.
@objc(TrialProcessorFactory)
public final class TrialProcessorFactory: NSObject, AUAudioUnitFactory {
    /// `AUAudioUnitFactory` inherits `NSExtensionRequestHandling`, and the
    /// system never sends this extension point a request: the Audio Unit is
    /// asked for below, and nothing else is.
    public func beginRequest(with context: NSExtensionContext) {}

    public func createAudioUnit(with componentDescription: AudioComponentDescription) throws -> AUAudioUnit {
        try TrialProcessorAudioUnit(componentDescription: componentDescription, options: [])
    }
}
