import XCTest
@testable import DenizSideloader

/// Expiry math on device must agree with the Windows signer
/// (`src-tauri/src/denizsideloader.rs`). Fixed clock, no flakiness.
final class ExpiryCalculatorTests: XCTestCase {
    let day: TimeInterval = 86_400
    var now: Date!

    override func setUp() {
        super.setUp()
        now = Date(timeIntervalSince1970: 1_800_000_000)
    }

    func testFreeAccountValidityIsSevenDays() {
        XCTAssertEqual(ExpiryCalculator.freeAccountValidityDays, 7)
        let expiry = ExpiryCalculator.expiryDate(from: now)
        XCTAssertEqual(expiry.timeIntervalSince(now), 7 * day, accuracy: 0.001)
    }

    func testFreshProfileHasSevenDaysLeft() {
        let expiry = ExpiryCalculator.expiryDate(from: now)
        XCTAssertEqual(ExpiryCalculator.daysRemaining(until: expiry, now: now), 7)
        XCTAssertFalse(ExpiryCalculator.needsRefresh(expiresAt: expiry, now: now))
        XCTAssertFalse(ExpiryCalculator.isExpired(expiresAt: expiry, now: now))
    }

    func testRefreshDueWithinTwoDays() {
        let expiry = now.addingTimeInterval(7 * day)
        XCTAssertFalse(ExpiryCalculator.needsRefresh(expiresAt: expiry, now: now.addingTimeInterval(4 * day)))
        XCTAssertTrue(ExpiryCalculator.needsRefresh(expiresAt: expiry, now: now.addingTimeInterval(5 * day)))
        XCTAssertTrue(ExpiryCalculator.needsRefresh(expiresAt: expiry, now: expiry))
    }

    func testExpiredAndNegativeRemaining() {
        let expiry = now.addingTimeInterval(7 * day)
        let past = expiry.addingTimeInterval(100)
        XCTAssertTrue(ExpiryCalculator.isExpired(expiresAt: expiry, now: past))
        XCTAssertTrue(ExpiryCalculator.needsRefresh(expiresAt: expiry, now: past))
        XCTAssertLessThan(ExpiryCalculator.daysRemaining(until: expiry, now: past), 0)
    }

    func testRetryBackoffCapsAtOneHour() {
        XCTAssertEqual(ExpiryCalculator.retryDelaySeconds(attempt: 1), 60)
        XCTAssertEqual(ExpiryCalculator.retryDelaySeconds(attempt: 2), 120)
        XCTAssertEqual(ExpiryCalculator.retryDelaySeconds(attempt: 3), 240)
        XCTAssertEqual(ExpiryCalculator.retryDelaySeconds(attempt: 7), 3_600)
        XCTAssertEqual(ExpiryCalculator.retryDelaySeconds(attempt: 99), 3_600)
    }

    func testManagedAppDerivesExpiry() {
        let app = ManagedApp(name: "Demo", bundleID: "com.example.demo",
                             version: "1.0", installedAt: now)
        XCTAssertEqual(app.daysRemaining, 7)
        XCTAssertFalse(app.needsRefresh)
        XCTAssertFalse(app.isExpired)
        XCTAssertEqual(app.expiresAt.timeIntervalSince(now), 7 * day, accuracy: 0.001)
    }
}
