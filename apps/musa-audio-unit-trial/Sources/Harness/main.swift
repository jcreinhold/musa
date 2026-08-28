//  The trial harness.
//
//  It is a host, not a test runner: it drives the two components the way a
//  workstation would and writes down what happened. The report is JSON on
//  standard output so `scripts/check-audio-unit-trial.sh` can hold the trial
//  to its own findings, and a running commentary on standard error so a
//  person watching sees the same thing.

import AVFoundation
import Foundation
import MusaTrialKit

let appGroup = ProcessInfo.processInfo.environment["MUSA_TRIAL_APP_GROUP"] ?? "group.dev.musa.audiounittrial"
let report = Report()

report.record(Finding(
    id: "probe.available",
    title: "The allocation probe is inserted and counting",
    outcome: AllocationProbe.isWorking ? .pass : .unsupported,
    detail: AllocationProbe.isWorking
        ? "musa_probe_arm is bound and observed a deliberate allocation"
        : "DYLD_INSERT_LIBRARIES did not put libMusaAllocProbe.dylib in this process; the allocation finding is reported as unsupported rather than as a pass"
))

InstrumentExperiments.run(into: report)
ProcessorExperiments.run(into: report)
SystemExperiments.run(into: report, appGroup: appGroup)

var environment: [String: String] = [
    "harness": "musa-audio-unit-trial",
    "appGroup": appGroup,
    "processName": ProcessInfo.processInfo.processName,
    "operatingSystem": ProcessInfo.processInfo.operatingSystemVersionString,
]
for key in ["MUSA_TRIAL_XCODE", "MUSA_TRIAL_SDK", "MUSA_TRIAL_HOST"] {
    if let value = ProcessInfo.processInfo.environment[key] {
        environment[key] = value
    }
}
print(report.json(environment: environment))

let failed = report.findings.filter { $0.outcome == .fail }
exit(failed.isEmpty ? 0 : 1)
