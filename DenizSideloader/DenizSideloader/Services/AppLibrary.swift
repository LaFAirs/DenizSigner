import Foundation

/// Local library of tracked IPAs. Persisted in UserDefaults on-device;
/// no accounts, no cloud, no network.
@MainActor
final class AppLibrary: ObservableObject {
    @Published private(set) var apps: [ManagedApp] = []
    private let storageKey = "denizsideloader.managedApps.v1"

    init() { load() }

    var dueForRefresh: [ManagedApp] { apps.filter(\.needsRefresh) }
    var expired: [ManagedApp] { apps.filter(\.isExpired) }

    /// Imports an IPA file into the app sandbox and records it.
    /// Metadata parsing is best-effort: when it fails the file is still
    /// kept and the user confirms name/version manually. Nothing is
    /// claimed that was not actually read from the archive.
    @discardableResult
    func importIPA(from sourceURL: URL, installDate: Date = Date()) throws -> ManagedApp {
        let fileName = sourceURL.lastPathComponent
        let dest = try copyIntoSandbox(sourceURL: sourceURL, fileName: fileName)
        let meta: IpaMetadata?
        do {
            meta = try IpaParser.parse(ipaURL: dest)
        } catch {
            meta = nil
        }
        let app = ManagedApp(
            name: meta?.displayName ?? fileName.replacingOccurrences(of: ".ipa", with: ""),
            bundleID: meta?.bundleID ?? "unknown",
            version: meta?.version ?? "?",
            installedAt: installDate)
        apps.append(app)
        save()
        if meta == nil {
            // Surface the parsing gap instead of inventing metadata.
            throw ImportWarning.metadataUnreadable(underlying: nil)
        }
        return app
    }

    func update(_ app: ManagedApp) {
        if let i = apps.firstIndex(where: { $0.id == app.id }) {
            apps[i] = app
            save()
        }
    }

    /// Marks an app refreshed (new provisioning date). Called after a
    /// successful Windows-assisted refresh via DenizSigner.
    func markRefreshed(id: ManagedApp.ID, date: Date = Date()) {
        if let i = apps.firstIndex(where: { $0.id == id }) {
            apps[i].installedAt = date
            save()
        }
    }

    func remove(id: ManagedApp.ID) {
        apps.removeAll { $0.id == id }
        save()
    }

    // MARK: - Private

    private var sandboxIPADir: URL {
        let dir = FileManager.default.urls(for: .documentDirectory, in: .userDomainMask)[0]
            .appendingPathComponent("IPAs", isDirectory: true)
        try? FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        return dir
    }

    private func copyIntoSandbox(sourceURL: URL, fileName: String) throws -> URL {
        let scoped = sourceURL.startAccessingSecurityScopedResource()
        defer { if scoped { sourceURL.stopAccessingSecurityScopedResource() } }
        let dest = sandboxIPADir.appendingPathComponent(fileName)
        if FileManager.default.fileExists(atPath: dest.path) {
            try FileManager.default.removeItem(at: dest)
        }
        try FileManager.default.copyItem(at: sourceURL, to: dest)
        return dest
    }

    private func load() {
        guard let data = UserDefaults.standard.data(forKey: storageKey),
              let decoded = try? JSONDecoder().decode([ManagedApp].self, from: data)
        else { return }
        apps = decoded
    }

    private func save() {
        if let data = try? JSONEncoder().encode(apps) {
            UserDefaults.standard.set(data, forKey: storageKey)
        }
    }
}

/// Non-fatal import outcome: the file was kept, metadata needs review.
enum ImportWarning: LocalizedError {
    case metadataUnreadable(underlying: Error?)
    var errorDescription: String? {
        NSLocalizedString("apps.import.nometa", comment: "")
    }
}
