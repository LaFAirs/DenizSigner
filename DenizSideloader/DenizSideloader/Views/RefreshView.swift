import SwiftUI

/// Refresh: expiry tracking, reminders, retry with backoff, last
/// success, and imminent-expiry warnings. Never promises background
/// auto-renewal — iOS does not guarantee it.
struct RefreshView: View {
    @EnvironmentObject private var library: AppLibrary
    @EnvironmentObject private var refresh: RefreshManager
    @EnvironmentObject private var log: DiagnosticLog
    @State private var status: String?

    var body: some View {
        NavigationStack {
            ScrollView {
                VStack(spacing: 16) {
                    Card {
                        VStack(alignment: .leading, spacing: 8) {
                            Text("refresh.how").font(.headline)
                            Text("refresh.how.body").font(.subheadline).foregroundStyle(Theme.muted)
                        }
                    }

                    Toggle(String(localized: "refresh.reminders"), isOn: Binding(
                        get: { refresh.state.remindersEnabled },
                        set: { refresh.state.remindersEnabled = $0 }
                    ))
                    .padding().background(Theme.card)
                    .clipShape(RoundedRectangle(cornerRadius: Theme.corner))
                    .onChange(of: refresh.state.remindersEnabled) { _, enabled in
                        if enabled {
                            Task {
                                let granted = await refresh.requestNotificationPermission()
                                status = granted
                                    ? String(localized: "refresh.reminders.on")
                                    : String(localized: "refresh.reminders.denied")
                            }
                        }
                    }

                    Button(String(localized: "refresh.check")) { checkAll() }
                        .buttonStyle(LargeButtonStyle())

                    if let last = refresh.state.lastSuccessfulRefresh {
                        Card {
                            Text(String(format: String(localized: "refresh.last"),
                                                 last.formatted(date: .abbreviated, time: .shortened)))
                                .font(.subheadline)
                        }
                    } else {
                        Card {
                            Text("refresh.last.never").font(.subheadline).foregroundStyle(Theme.muted)
                        }
                    }

                    if refresh.state.consecutiveFailures > 0 {
                        Card {
                            Text(String(format: String(localized: "refresh.failures"),
                                                 refresh.state.consecutiveFailures))
                                .font(.subheadline).foregroundStyle(Theme.danger)
                        }
                    }

                    if let status {
                        Card { Text(status).font(.subheadline).foregroundStyle(Theme.warning) }
                    }

                    ForEach(refresh.dueApps(from: library.apps)) { app in
                        Card {
                            VStack(alignment: .leading, spacing: 6) {
                                Text(app.name).font(.headline)
                                ExpiryBadge(app: app)
                                Button(String(localized: "refresh.markdone")) {
                                    library.markRefreshed(id: app.id)
                                    refresh.recordSuccess()
                                    log.info("Refresh confirmed for \(app.name)")
                                    status = nil
                                }
                                .buttonStyle(LargeButtonStyle(filled: false))
                                Text("refresh.markdone.hint")
                                    .font(.caption).foregroundStyle(Theme.muted)
                            }
                        }
                    }
                }
                .padding()
            }
            .background(Theme.background)
            .navigationTitle(String(localized: "refresh.tab"))
        }
    }

    private func checkAll() {
        let due = refresh.dueApps(from: library.apps)
        Task {
            var scheduled = 0
            for app in due {
                if await refresh.scheduleReminder(for: app) { scheduled += 1 }
            }
            if due.isEmpty {
                status = String(localized: "refresh.allgood")
            } else if scheduled == 0 {
                status = String(localized: "refresh.noreminder")
            } else {
                status = String(format: String(localized: "refresh.scheduled"), scheduled, due.count)
            }
            log.info("Refresh check: \(due.count) due, \(scheduled) reminders scheduled")
        }
    }
}
