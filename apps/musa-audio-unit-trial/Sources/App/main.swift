//  The containing app.
//
//  An Audio Unit extension on macOS is discovered through the app that
//  contains it, so this app exists to be that container and nothing else. It
//  prints what it holds and exits, which is enough for the system to register
//  the two extensions and enough for a person to check that it did.

import Foundation
import MusaTrialKit

let bundle = Bundle.main
print("MusaAudioUnitTrial \(bundle.bundleIdentifier ?? "?")")
print("instrument: \(fourCharacterText(musaTrialInstrumentDescription.componentType))"
    + " \(fourCharacterText(musaTrialInstrumentDescription.componentSubType))"
    + " \(fourCharacterText(musaTrialInstrumentDescription.componentManufacturer))")
print("processor: \(fourCharacterText(musaTrialProcessorDescription.componentType))"
    + " \(fourCharacterText(musaTrialProcessorDescription.componentSubType))"
    + " \(fourCharacterText(musaTrialProcessorDescription.componentManufacturer))")
let plugins = bundle.builtInPlugInsURL.map { url in
    (try? FileManager.default.contentsOfDirectory(atPath: url.path))?.sorted() ?? []
} ?? []
print("extensions: \(plugins.joined(separator: ", "))")
