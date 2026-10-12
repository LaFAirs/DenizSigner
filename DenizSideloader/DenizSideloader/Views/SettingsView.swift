import SwiftUI
import UniformTypeIdentifiers

/// Settings: account note, pairing management, device status, privacy,
/// diagnostic logs, app version. No clutter.
struct SettingsView: View {
    @EnvironmentObject private var pairing: PairingManager
    @EnvironmentObject private var log: DiagnosticLog
    @State private var showPairingImporter = false
    @State private var message: String?
    @State private var confirmDeletePairing = false

    var body: some View {
        NavigationStack {
            List {
                Section(String(localized: "settings.account")) {
                    Text("settings.account.body").font(.subheadline).foregroundStyle(Theme.muted)
                }

                Section(String(localized: "settings.pairing")) {
                    if pairing.hasPairingFile {
                        Text(String(format: String(localized: "settings.pairing.ok"),
                                             pairing.presentFileNames.joined(separator: ", ")))
                            .font(.subheadline)
                    } else {
                        Text("settings.pairing.missing").font(.subheadline).foregroundStyle(Theme.warning)
                    }
                    Button(String(localized: "settings.pairing.import")) { showPairingImporter = true }
                    Button(String(localized: "settings.pairing.rescan")) { pairing.rescan() }
                    Button(String(localized: "settings.pairing.delete"), role: .destructive) {
                        confirmDeletePairing = true
                    }
                    .confirmationDialog(String(localized: "settings.pairing.delete"),
                                        isPresented: $confirmDeletePairing,
                                        titleVisibility: .visible) {
                        Button(String(localized: "settings.pairing.delete.confirm"), role: .destructive) {
                            do {
                                try pairing.deletePairingFiles()
                                log.warning("Pairing files deleted by user")
                            } catch {
                                message = error.localizedDescription
                            }
                        }
                        Button(String(localized: "common.cancel"), role: .cancel) {}
                    } message: {
                        Text("settings.pairing.delete.hint")
                    }
                }

                Section(String(localized: "settings.device")) {
                    Text("settings.device.body").font(.subheadline).foregroundStyle(Theme.muted)
                    Text("iOS \(UIDevice.current.systemVersion) · \(UIDevice.current.model)")
                        .font(.caption).foregroundStyle(Theme.muted)
                }

                Section(String(localized: "settings.privacy")) {
                    Text("settings.privacy.body").font(.subheadline).foregroundStyle(Theme.muted)
                }

                Section(String(localized: "settings.logs")) {
                    ForEach(log.entries.suffix(8)) { entry in
                        VStack(alignment: .leading) {
                            Text("[\(entry.level)] \(entry.date.formatted(date: .omitted, time: .shortened))")
                                .font(.caption).foregroundStyle(Theme.muted)
                            Text(entry.message).font(.caption)
                        }
                    }
                    if let url = log.exportURL(),
                       FileManager.default.fileExists(atPath: url.path)
                    {
                        ShareLink(item: url, preview: SharePreview("denizsideloader.log")) {
                            Text("settings.logs.share")
                        }
                    }
                    Button(String(localized: "settings.logs.clear"), role: .destructive) {
                        log.clear()
                    }
                }

                Section(String(localized: "settings.about")) {
                    Text("DenizSideloader \(appVersion)").font(.subheadline)
                    Text("settings.about.honest").font(.caption).foregroundStyle(Theme.muted)
                }

                if let message {
                    Section { Text(message).font(.footnote).foregroundStyle(Theme.warning) }
                }
            }
            .scrollContentBackground(.hidden)
            .background(Theme.background)
            .navigationTitle(String(localized: "settings.tab"))
            .fileImporter(isPresented: $showPairingImporter,
                          allowedContentTypes: [.propertyList, .data]) { result in
                do {
                    let url = try result.get()
                    try pairing.importPairingFile(from: url)
                    log.info("Pairing file imported")
                    message = nil
                } catch {
                    message = error.localizedDescription
                    log.error("Pairing import failed: \(error.localizedDescription)")
                }
            }
        }
    }

    private var appVersion: String {
        let version = Bundle.main.object(forInfoDictionaryKey: "CFBundleShortVersionString") as? String ?? "?"
        let build = Bundle.main.object(forInfoDictionaryKey: "CFBundleVersion") as? String ?? "?"
        return "\(version) (\(build))"
    }
}
