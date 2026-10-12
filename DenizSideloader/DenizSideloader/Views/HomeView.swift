import SwiftUI
import UniformTypeIdentifiers

/// Home: device/account status at a glance, managed apps with signing
/// state, and the big install button. Beginners need no manual.
struct HomeView: View {
    @EnvironmentObject private var library: AppLibrary
    @EnvironmentObject private var pairing: PairingManager
    @EnvironmentObject private var refresh: RefreshManager
    @EnvironmentObject private var log: DiagnosticLog
    @State private var showImporter = false
    @State private var notice: String?

    var body: some View {
        NavigationStack {
            ScrollView {
                VStack(spacing: 16) {
                    Card {
                        VStack(alignment: .leading, spacing: 8) {
                            Text("home.status").font(.headline)
                            StatusRow(label: "home.apps",
                                      value: "\(library.apps.count)")
                            StatusRow(label: "home.due",
                                      value: "\(refresh.dueApps(from: library.apps).count)",
                                      highlight: !refresh.dueApps(from: library.apps).isEmpty)
                            StatusRow(label: "home.pairing",
                                      value: pairing.hasPairingFile
                                          ? String(localized: "home.pairing.ok")
                                          : String(localized: "home.pairing.missing"),
                                      highlight: !pairing.hasPairingFile)
                            if let last = refresh.state.lastSuccessfulRefresh {
                                StatusRow(label: "home.lastrefresh",
                                          value: last.formatted(date: .abbreviated, time: .shortened))
                            } else {
                                StatusRow(label: "home.lastrefresh",
                                          value: String(localized: "home.lastrefresh.never"))
                            }
                        }
                    }

                    if let notice {
                        Card {
                            Text(notice).font(.subheadline).foregroundStyle(Theme.warning)
                        }
                    }

                    Button(String(localized: "home.install")) { showImporter = true }
                        .buttonStyle(LargeButtonStyle())
                        .accessibilityHint("home.install.hint")

                    Text("home.install.note")
                        .font(.footnote)
                        .foregroundStyle(Theme.muted)

                    ForEach(library.apps) { app in
                        Card {
                            VStack(alignment: .leading, spacing: 4) {
                                Text(app.name).font(.headline)
                                Text("\(app.bundleID) · \(app.version)")
                                    .font(.caption).foregroundStyle(Theme.muted)
                                ExpiryBadge(app: app)
                            }
                        }
                    }
                    if library.apps.isEmpty {
                        Text("home.empty").font(.subheadline).foregroundStyle(Theme.muted)
                    }
                }
                .padding()
            }
            .background(Theme.background)
            .navigationTitle("DenizSideloader")
            .fileImporter(isPresented: $showImporter, allowedContentTypes: [.init(filenameExtension: "ipa") ?? .data]) { result in
                do {
                    let url = try result.get()
                    try library.importIPA(from: url)
                    log.info("Imported \(url.lastPathComponent)")
                    notice = nil
                } catch let warning as ImportWarning {
                    notice = warning.localizedDescription
                    log.warning("Import without metadata: \(warning.localizedDescription)")
                } catch {
                    notice = error.localizedDescription
                    log.error("Import failed: \(error.localizedDescription)")
                }
            }
        }
    }
}

struct StatusRow: View {
    var label: LocalizedStringKey
    var value: String
    var highlight = false
    var body: some View {
        HStack {
            Text(label).foregroundStyle(Theme.muted)
            Spacer()
            Text(value).bold().foregroundStyle(highlight ? Theme.warning : .white)
        }.font(.subheadline)
    }
}

struct ExpiryBadge: View {
    var app: ManagedApp
    var body: some View {
        let text: String = if app.isExpired {
            String(format: String(localized: "badge.expired"), app.name)
        } else {
            String(format: String(localized: "badge.days"), app.daysRemaining)
        }
        return Text(text)
            .font(.caption).bold()
            .padding(.horizontal, 10).padding(.vertical, 4)
            .background(app.isExpired ? Theme.danger : (app.needsRefresh ? Theme.warning : Theme.accent))
            .foregroundStyle(.white)
            .clipShape(Capsule())
    }
}
