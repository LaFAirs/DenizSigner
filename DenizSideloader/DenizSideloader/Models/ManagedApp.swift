import Foundation

/// One IPA tracked by DenizSideloader.
/// `installedAt` is the date the provisioning profile was issued
/// (set when DenizSigner installs/refreshes the app). Free Apple IDs
/// expire ~7 days later; paid teams carry their own profile date.
struct ManagedApp: Identifiable, Codable, Hashable {
    var id: UUID
    var name: String
    var bundleID: String
    var version: String
    var installedAt: Date
    var validityDays: Int

    init(id: UUID = UUID(), name: String, bundleID: String, version: String,
         installedAt: Date = Date(), validityDays: Int = ExpiryCalculator.freeAccountValidityDays) {
        self.id = id
        self.name = name
        self.bundleID = bundleID
        self.version = version
        self.installedAt = installedAt
        self.validityDays = validityDays
    }

    var expiresAt: Date { ExpiryCalculator.expiryDate(from: installedAt, validityDays: validityDays) }
    var daysRemaining: Int { ExpiryCalculator.daysRemaining(until: expiresAt) }
    var needsRefresh: Bool { ExpiryCalculator.needsRefresh(expiresAt: expiresAt) }
    var isExpired: Bool { ExpiryCalculator.isExpired(expiresAt: expiresAt) }
}
