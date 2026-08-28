/// What the automated host writes down.
///
/// The same shape prompt 215's trial used, and for the same reason: a run
/// that prints prose cannot be asserted, and a check that greps prose starts
/// passing the day somebody rewords a sentence.

import Foundation

struct Finding: Encodable {
    enum Outcome: String, Encodable {
        case pass
        case fail
        case unsupported
    }

    let id: String
    let title: String
    let outcome: Outcome
    let detail: String
    var numbers: [String: Double] = [:]
}

final class Report {
    private(set) var findings: [Finding] = []

    func record(_ finding: Finding) {
        findings.append(finding)
        FileHandle.standardError.write(
            "\(finding.outcome.rawValue.padding(toLength: 11, withPad: " ", startingAt: 0)) \(finding.id) — \(finding.detail)\n"
                .data(using: .utf8)!
        )
    }

    func check(_ id: String, _ title: String, _ held: Bool, _ detail: String, numbers: [String: Double] = [:]) {
        record(Finding(id: id, title: title, outcome: held ? .pass : .fail, detail: detail, numbers: numbers))
    }

    func unsupported(_ id: String, _ title: String, _ detail: String) {
        record(Finding(id: id, title: title, outcome: .unsupported, detail: detail))
    }

    func json(environment: [String: String]) throws -> Data {
        struct Document: Encodable {
            let environment: [String: String]
            let findings: [Finding]
        }
        let encoder = JSONEncoder()
        encoder.outputFormatting = [.prettyPrinted, .sortedKeys]
        return try encoder.encode(Document(environment: environment, findings: findings))
    }
}
