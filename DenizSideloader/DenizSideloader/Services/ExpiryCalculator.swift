import Foundation

/// Pure expiry math. Mirrors `src-tauri/src/denizsideloader.rs` so the
/// on-device tracker and the Windows signer agree on every date.
/// Covered by `DenizSideloaderTests/ExpiryCalculatorTests.swift`.
enum ExpiryCalculator {
    /// Free Apple-ID provisioning window. Never claimed as unlimited.
    static let freeAccountValidityDays = 7
    /// Refresh is due while this many (or fewer) days remain.
    static let refreshDueWithinDays = 2
    static let secondsPerDay: TimeInterval = 86_400

    static func expiryDate(from installedAt: Date, validityDays: Int = freeAccountValidityDays) -> Date {
        installedAt.addingTimeInterval(TimeInterval(validityDays) * secondsPerDay)
    }

    /// Whole days remaining (negative when expired).
    static func daysRemaining(until expiry: Date, now: Date = Date()) -> Int {
        Int(floor(expiry.timeIntervalSince(now) / secondsPerDay))
    }

    static func needsRefresh(expiresAt: Date, now: Date = Date()) -> Bool {
        daysRemaining(until: expiresAt, now: now) <= refreshDueWithinDays
    }

    static func isExpired(expiresAt: Date, now: Date = Date()) -> Bool {
        now >= expiresAt
    }

    /// Backoff before retrying a failed refresh (seconds, 1-based
    /// attempt, capped at one hour). Keeps retries user-visible.
    static func retryDelaySeconds(attempt: Int) -> Int {
        let shift = max(0, min(attempt - 1, 6))
        return min(60 * (1 << shift), 3_600)
    }
}
