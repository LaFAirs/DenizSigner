import Foundation

/// On-device diagnostics: in-memory ring buffer + persisted log file,
/// failure checklists, and log export. Secrets are stripped BEFORE any
/// entry is stored (never after the fact).
@MainActor
final class DiagnosticLog: ObservableObject {
    struct Entry: Identifiable, Codable {
        var id: UUID = UUID()
        var date: Date = Date()
        var level: String
        var message: String
    }

    @Published private(set) var entries: [Entry] = []
    private let maxEntries = 500

    func info(_ message: String) { add(level: "INFO", message) }
    func warning(_ message: String) { add(level: "WARN", message) }
    func error(_ message: String) { add(level: "ERROR", message) }

    private func add(level: String, _ message: String) {
        entries.append(Entry(level: level, message: Diagnostics.redact(message)))
        if entries.count > maxEntries {
            entries.removeFirst(entries.count - maxEntries)
        }
        PersistedLog.append(line: "[\(level)] \(Diagnostics.redact(message))")
    }

    func clear() {
        entries.removeAll()
        PersistedLog.clear()
    }

    /// Checklist for a failed installation/refresh. Each issue names the
    /// concrete fix; unknown causes say so instead of guessing.
    func diagnose(app: ManagedApp?, hasPairingFile: Bool) -> [DiagnosticIssue] {
        Diagnostics.checklist(app: app, hasPairingFile: hasPairingFile)
    }

    func exportURL() -> URL? { PersistedLog.fileURL() }
}

struct DiagnosticIssue: Identifiable {
    var id = UUID()
    var title: String
    var fix: String
    var severe: Bool
}

enum Diagnostics {
    /// Strips anything that looks like a credential before logging.
    /// Runs on every entry (log call sites must not pre-filter).
    static func redact(_ message: String) -> String {
        var out = message
        // password=..., token=..., 2fa/code digits near keywords
        let patterns = [
            #"(?i)(password|passwd|pwd|token|secret|session|cookie|private[_-]?key|2fa|verification[_-]?code)\s*[:=]\s*\S+"#,
            #"(?i)(apple[_-]?id\s+)?(password|code)\s+[0-9]{4,}"#,
        ]
        for pattern in patterns {
            if let rx = try? NSRegularExpression(pattern: pattern) {
                out = rx.stringByReplacingMatches(in: out, range: NSRange(out.startIndex..., in: out),
                                                  withTemplate: "$1: ‹redacted›")
            }
        }
        return out
    }

    static func checklist(app: ManagedApp?, hasPairingFile: Bool) -> [DiagnosticIssue] {
        var issues: [DiagnosticIssue] = []
        guard let app else {
            return [DiagnosticIssue(
                title: NSLocalizedString("diag.noapp", comment: ""),
                fix: NSLocalizedString("diag.noapp.fix", comment: ""), severe: false)]
        }
        if app.isExpired {
            issues.append(DiagnosticIssue(
                title: String(format: NSLocalizedString("diag.expired", comment: ""), app.name),
                fix: NSLocalizedString("diag.expired.fix", comment: ""), severe: true))
        } else if app.needsRefresh {
            issues.append(DiagnosticIssue(
                title: String(format: NSLocalizedString("diag.due", comment: ""), app.name, app.daysRemaining),
                fix: NSLocalizedString("diag.due.fix", comment: ""), severe: false))
        }
        if !hasPairingFile {
            issues.append(DiagnosticIssue(
                title: NSLocalizedString("diag.nopairing", comment: ""),
                fix: NSLocalizedString("diag.nopairing.fix", comment: ""), severe: true))
        }
        if app.bundleID == "unknown" {
            issues.append(DiagnosticIssue(
                title: NSLocalizedString("diag.nometa", comment: ""),
                fix: NSLocalizedString("diag.nometa.fix", comment: ""), severe: false))
        }
        if issues.isEmpty {
            issues.append(DiagnosticIssue(
                title: NSLocalizedString("diag.clean", comment: ""),
                fix: NSLocalizedString("diag.clean.fix", comment: ""), severe: false))
        }
        return issues
    }
}

private enum PersistedLog {
    static func fileURL() -> URL? {
        let dir = FileManager.default.urls(for: .documentDirectory, in: .userDomainMask)[0]
            .appendingPathComponent("Logs", isDirectory: true)
        try? FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        return dir.appendingPathComponent("denizsideloader.log")
    }

    static func append(line: String) {
        guard let url = fileURL() else { return }
        let stamped = "\(ISO8601DateFormatter().string(from: Date())) \(line)\n"
        guard let data = stamped.data(using: .utf8) else { return }
        if FileManager.default.fileExists(atPath: url.path) {
            if let handle = try? FileHandle(forWritingTo: url) {
                try? handle.seekToEnd()
                try? handle.write(contentsOf: data)
                try? handle.close()
            }
        } else {
            try? data.write(to: url)
        }
    }

    static func clear() {
        guard let url = fileURL() else { return }
        try? FileManager.default.removeItem(at: url)
    }
}
