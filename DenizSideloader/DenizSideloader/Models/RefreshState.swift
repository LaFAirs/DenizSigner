import Foundation

/// Refresh bookkeeping. Persisted locally; never leaves the device.
struct RefreshState: Codable {
    /// The user opted into expiry reminders (local notifications only).
    var remindersEnabled: Bool
    /// Last time a Windows-assisted refresh completed for ANY app.
    var lastSuccessfulRefresh: Date?
    /// Consecutive failed refresh attempts (resets on success).
    var consecutiveFailures: Int
    /// How many times the user was warned about an imminent expiry.
    var warningsShown: Int

    static let `default` = RefreshState(
        remindersEnabled: true, lastSuccessfulRefresh: nil,
        consecutiveFailures: 0, warningsShown: 0)
}
