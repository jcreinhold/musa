/// The containing app.
///
/// An `.appex` cannot ship alone, so there is an app. It is the source and
/// asset manager and the diagnostic surface — it opens an existing Musa
/// project, chooses a part, grants the extension access to that closure, and
/// says what the component made of it. It is not a second editor: no
/// instrument is declared here, and nothing it does writes `.musa`.
///
/// It is a command-line-shaped app rather than a window because prompt 216
/// owns the *component*, and a real interface would be inventing product
/// decisions the desktop specification owns.

import Foundation
import MusaAudioUnitKit

let arguments = CommandLine.arguments.dropFirst()
guard arguments.count >= 2 else {
    print(
        """
        Musa Audio Unit — the containing app for the `aumu musa Musa` Music Device.

        Usage: MusaAudioUnit <project> <part> [piece]

        Prepares the named part and reports what the component made of it,
        including a security-scoped bookmark a host would restore. The `.musa`
        source stays canonical: nothing here writes to it.
        """
    )
    exit(arguments.isEmpty ? 0 : 2)
}

let project = arguments[arguments.startIndex]
let part = arguments[arguments.index(after: arguments.startIndex)]
let piece = arguments.count > 2 ? arguments[arguments.index(arguments.startIndex, offsetBy: 2)] : nil

let selection = MusaSelection(project: project, piece: piece, part: part, sampleRate: 48_000)
let result = MusaPreparer.prepareNow(selection)
if let instrument = result.instrument {
    print("prepared \(part)")
    print("  music:  \(result.musicIdentity)")
    print("  assets: \(result.assetIdentity)")
    print("  binds:  \(result.inputs.joined(separator: ", "))")

    // Every line below is generated from what the source declared. There is
    // no catalogue of Musa controls in this app: a control it cannot name is
    // a control the source did not write, and a control the source adds
    // appears here without anything being edited.
    print("  controls:")
    if result.controls.isEmpty {
        print("    (this instrument declares no public controls)")
    }
    for control in result.controls {
        let ramp = control.continuous ? "ramps" : "steps"
        print("    \(control.display) — \(control.summary)")
        print(
            "      \(control.identity)  \(control.kind)/\(control.updateRate), \(ramp), "
                + "\(control.minimum)…\(control.maximum), default \(control.defaultValue)"
        )
        print("      address 0x\(String(control.address, radix: 16, uppercase: false))")
    }
    // §6: a loss is named, never implied.
    for loss in result.controlLosses {
        print("    not a host parameter: \(loss)")
    }
    print("  outputs:")
    for (index, output) in result.outputs.enumerated() {
        let role: String
        switch output.role {
        case .main: role = "the piece's main output"
        case .part: role = "this part's own output"
        case .bus: role = "a studio bus this part reaches"
        }
        print("    bus \(index): \(output.name) — \(role)")
    }
    if let bookmark = try? URL(fileURLWithPath: project).bookmarkData(options: [.withSecurityScope]) {
        print("  access: \(bookmark.count) bytes of security-scoped bookmark")
    }
    musa_au_instrument_release(instrument)
} else {
    print("refused: \(result.refusal)")
    exit(1)
}
