import SwiftUI
import UniformTypeIdentifiers

/// Apps: import IPAs, see icon/version/expiry, renew signing
/// (via Windows), update, and diagnose failed installs.
struct AppsView: View {
    @EnvironmentObject private var library: AppLibrary
    @EnvironmentObject private var pairing: PairingManager
    @EnvironmentObject private var log: DiagnosticLog
    @State private var showImporter = false
    @State private var selected: ManagedApp?
    @State private var message: String?

    var body: some View {
        NavigationStack {
            List {
                Section {
                    Button(String(localized: "apps.import")) { showImporter = true }
                        .buttonStyle(LargeButtonStyle())
                }
                .listRowBackground(Color.clear)

                Section(String(localized: "apps.managed")) {
                    ForEach(library.apps) { app in
                        Button { selected = app } label: {
                            VStack(alignment: .leading, spacing: 4) {
                                Text(app.name).font(.headline).foregroundStyle(.white)
                                Text("\(app.bundleID)")
                                    .font(.caption).foregroundStyle(Theme.muted)
                                Text(String(format: String(localized: "apps.version"),
                                                     app.version, app.daysRemaining))
                                    .font(.caption).foregroundStyle(Theme.muted)
                                ExpiryBadge(app: app)
                            }
                            .padding(.vertical, 4)
                        }
                    }
                    .onDelete { offsets in
                        for i in offsets {
                            let app = library.apps[i]
                            library.remove(id: app.id)
                            log.info("Removed \(app.name) from tracking")
                        }
                    }
                }

                if let message {
                    Section { Text(message).font(.footnote).foregroundStyle(Theme.warning) }
                }
            }
            .scrollContentBackground(.hidden)
            .background(Theme.background)
            .navigationTitle(String(localized: "apps.tab"))
            .fileImporter(isPresented: $showImporter, allowedContentTypes: [.init(filenameExtension: "ipa") ?? .data]) { result in
                do {
                    let url = try result.get()
                    let app = try library.importIPA(from: url)
                    log.info("Imported \(app.name)")
                    message = nil
                } catch let warning as ImportWarning {
                    message = warning.localizedDescription
                } catch {
                    message = error.localizedDescription
                    log.error("Import failed: \(error.localizedDescription)")
                }
            }
            .sheet(item: $selected) { app in
                AppDetailView(app: app)
            }
        }
    }
}

/// Detail: renew/update/diagnose for one app.
struct AppDetailView: View {
    @EnvironmentObject private var library: AppLibrary
    @EnvironmentObject private var pairing: PairingManager
    @EnvironmentObject private var log: DiagnosticLog
    @Environment(\.dismiss) private var dismiss
    @State var app: ManagedApp
    @State private var issues: [DiagnosticIssue] = []
    @State private var info: String?

    var body: some View {
        NavigationStack {
            ScrollView {
                VStack(spacing: 16) {
                    Card {
                        VStack(alignment: .leading, spacing: 6) {
                            Text(app.name).font(.title2.bold())
                            Text(app.bundleID).font(.caption).foregroundStyle(Theme.muted)
                            Text(String(format: String(localized: "apps.version"),
                                                 app.version, app.daysRemaining))
                                .font(.subheadline)
                            Text(String(format: String(localized: "apps.expires"),
                                                 app.expiresAt.formatted(date: .long, time: .shortened)))
                                .font(.subheadline).foregroundStyle(Theme.muted)
                            ExpiryBadge(app: app)
                        }
                    }

                    // Renewal runs on Windows — this button explains and
                    // records the pending state instead of faking success.
                    Button(String(localized: "apps.renew")) {
                        do {
                            try DeviceTransport.refreshOnDevice(appName: app.name)
                        } catch {
                            info = error.localizedDescription
                            log.warning("Renew requested for \(app.name): routed to Windows companion")
                        }
                    }
                    .buttonStyle(LargeButtonStyle())

                    Button(String(localized: "apps.diagnose")) {
                        issues = log.diagnose(app: app, hasPairingFile: pairing.hasPairingFile)
                    }
                    .buttonStyle(LargeButtonStyle(filled: false))

                    if let info {
                        Card { Text(info).font(.subheadline).foregroundStyle(Theme.warning) }
                    }

                    ForEach(issues) { issue in
                        Card {
                            VStack(alignment: .leading, spacing: 4) {
                                Text(issue.title).font(.headline)
                                    .foregroundStyle(issue.severe ? Theme.danger : .white)
                                Text(issue.fix).font(.subheadline).foregroundStyle(Theme.muted)
                            }
                        }
                    }
                }
                .padding()
            }
            .background(Theme.background)
            .navigationTitle(app.name)
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .confirmationAction) {
                    Button(String(localized: "common.done")) { dismiss() }
                }
            }
        }
    }
}
