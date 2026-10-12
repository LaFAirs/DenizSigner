import Foundation
import UserNotifications

/// Refresh orchestration with honest platform limits.
///
/// What it DOES: tracks expiry per app, reminds the user BEFORE the
/// 7-day window closes (local notifications + in-app warnings), retries
/// bookkeeping with backoff, and records the last successful refresh.
///
/// What it does NOT claim: background auto-refresh. iOS suspends apps,
/// `BGAppRefreshTask` is best-effort and never guaranteed, so the UI
/// says "reminders", never "automatic renewal". The actual re-signing
/// runs on Windows via DenizSigner over USB.
@MainActor
final class RefreshManager: ObservableObject {
    @Published var state = RefreshState.default
    @Published var lastAttempt: Date?
    @Published var lastError: String?

    private let stateKey = "denizsideloader.refreshState.v1"

    init() {
        if let data = UserDefaults.standard.data(forKey: stateKey),
           let decoded = try? JSONDecoder().decode(RefreshState.self, from: data)
        {
            state = decoded
        }
    }

    /// Apps that need a Windows-assisted refresh now, soonest first.
    func dueApps(from apps: [ManagedApp], now: Date = Date()) -> [ManagedApp] {
        apps.filter { $0.needsRefresh }.sorted { $0.expiresAt < $1.expiresAt }
    }

    /// Records a user-confirmed completed refresh (done on Windows).
    func recordSuccess(date: Date = Date()) {
        state.lastSuccessfulRefresh = date
        state.consecutiveFailures = 0
        lastError = nil
        save()
    }

    /// Records a failed attempt and returns when to retry next.
    @discardableResult
    func recordFailure(message: String, date: Date = Date()) -> Date {
        state.consecutiveFailures += 1
        lastAttempt = date
        lastError = Diagnostics.redact(message)
        save()
        return date.addingTimeInterval(
            TimeInterval(ExpiryCalculator.retryDelaySeconds(attempt: state.consecutiveFailures)))
    }

    /// Schedules a local reminder before expiry. Returns false when the
    /// user disabled reminders or notifications are denied — the caller
    /// shows the in-app warning instead (no silent promises).
    func scheduleReminder(for app: ManagedApp) async -> Bool {
        guard state.remindersEnabled else { return false }
        let center = UNUserNotificationCenter.current()
        let settings = await center.notificationSettings()
        guard settings.authorizationStatus == .authorized
            || settings.authorizationStatus == .provisional
        else { return false }
        // Remind 2 days before expiry and again on the last day.
        let fireDates = [
            app.expiresAt.addingTimeInterval(-2 * ExpiryCalculator.secondsPerDay),
            app.expiresAt.addingTimeInterval(-12 * 3_600),
        ].filter { $0 > Date() }
        for (index, date) in fireDates.enumerated() {
            let content = UNMutableNotificationContent()
            content.title = "DenizSideloader"
            content.body = String(format: NSLocalizedString("refresh.reminder.body", comment: ""),
                                         app.name, app.daysRemaining)
            content.sound = .default
            var components = Calendar.current.dateComponents(
                [.year, .month, .day, .hour, .minute], from: date)
            components.second = 0
            let trigger = UNCalendarNotificationTrigger(dateMatching: components, repeats: false)
            let id = "expiry-\(app.id.uuidString)-\(index)"
            try? await center.add(UNNotificationRequest(identifier: id, content: content, trigger: trigger))
        }
        return !fireDates.isEmpty
    }

    func requestNotificationPermission() async -> Bool {
        do {
            return try await UNUserNotificationCenter.current()
                .requestAuthorization(options: [.alert, .sound, .badge])
        } catch {
            return false
        }
    }

    private func save() {
        if let data = try? JSONEncoder().encode(state) {
            UserDefaults.standard.set(data, forKey: stateKey)
        }
    }
}
