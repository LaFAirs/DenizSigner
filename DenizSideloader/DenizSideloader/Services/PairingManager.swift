import Foundation

/// Pairing-file management on device.
///
/// The pairing file itself is created on Windows by DenizSigner
/// (lockdown + remote-pairing handshake over USB) and placed into this
/// app's Documents folder via AFC. Here the user can verify it exists,
/// re-import it from the Files app, or delete it. This manager never
/// fabricates pairing data and never uploads it anywhere.
@MainActor
final class PairingManager: ObservableObject {
    @Published private(set) var presentFileNames: [String] = []
    @Published private(set) var lastCheck: Date?

    /// File names DenizSigner looks for (first entry is ours).
    static let knownFileNames = [
        "ALTPairingFile.mobiledevicepairing",
        "pairingFile.plist",
    ]

    init() { rescan() }

    var hasPairingFile: Bool { !presentFileNames.isEmpty }

    func rescan() {
        presentFileNames = Self.knownFileNames.filter { name in
            FileManager.default.fileExists(atPath: documentsURL.appendingPathComponent(name).path)
        }
        lastCheck = Date()
    }

    /// Imports a pairing file chosen in the Files app. Validates that it
    /// is a readable XML plist before accepting it.
    func importPairingFile(from sourceURL: URL) throws {
        let scoped = sourceURL.startAccessingSecurityScopedResource()
        defer { if scoped { sourceURL.stopAccessingSecurityScopedResource() } }
        let data = try Data(contentsOf: sourceURL)
        try validatePairingPlist(data)
        let dest = documentsURL.appendingPathComponent("ALTPairingFile.mobiledevicepairing")
        try data.write(to: dest, options: .atomic)
        rescan()
    }

    func deletePairingFiles() throws {
        for name in presentFileNames {
            try FileManager.default.removeItem(at: documentsURL.appendingPathComponent(name))
        }
        rescan()
    }

    // MARK: - Private

    private var documentsURL: URL {
        FileManager.default.urls(for: .documentDirectory, in: .userDomainMask)[0]
    }

    /// Accepts any non-empty XML plist dictionary. The exact key set
    /// differs by iOS version (lockdown vs. merged lockdown+RSD), so we
    /// check structure, not keys — and say so in the UI.
    func validatePairingPlist(_ data: Data) throws {
        guard !data.isEmpty, data.count < 10_000_000 else { throw PairingError.notAPlist }
        guard let plist = try? PropertyListSerialization.propertyList(from: data, format: nil),
              plist is [String: Any]
        else { throw PairingError.notAPlist }
    }
}

enum PairingError: LocalizedError {
    case notAPlist
    var errorDescription: String? {
        NSLocalizedString("pairing.error.notaplist", comment: "")
    }
}
