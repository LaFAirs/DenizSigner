import XCTest
@testable import DenizSideloader

/// Pairing/import failure paths must be explicit and secret-free.
/// No device needed: all inputs are local data.
final class PairingAndInstallFailureTests: XCTestCase {
    @MainActor
    private func makeManager() -> PairingManager { PairingManager() }

    func testGarbageIsNotAPairingFile() async {
        let manager = await makeManager()
        await MainActor.run {
            XCTAssertThrowsError(try manager.validatePairingPlist(Data("hello".utf8)))
            XCTAssertThrowsError(try manager.validatePairingPlist(Data()))
        }
    }

    func testPlistDictionaryIsAcceptedWithoutClaimingKeys() async {
        let manager = await makeManager()
        let plist = try! PropertyListSerialization.data(
            fromPropertyList: ["HostID": "ABC", "WiFiMAC": "00:11"],
            format: .xml, options: 0)
        await MainActor.run {
            XCTAssertNoThrow(try manager.validatePairingPlist(plist))
        }
    }

    func testPlistArrayIsRejected() async {
        let manager = await makeManager()
        let plist = try! PropertyListSerialization.data(
            fromPropertyList: ["a", "b"], format: .xml, options: 0)
        await MainActor.run {
            XCTAssertThrowsError(try manager.validatePairingPlist(plist))
        }
    }

    func testDiagnosticsRedactSecrets() {
        let dirty = "login failed password=hunter2 token=abc123 session xyz"
        let clean = Diagnostics.redact(dirty)
        XCTAssertFalse(clean.contains("hunter2"))
        XCTAssertFalse(clean.contains("abc123"))
        XCTAssertTrue(clean.contains("redacted"))
    }

    func testDiagnosticsRedactTwoFACode() {
        let dirty = "2fa code 123456 rejected"
        XCTAssertFalse(Diagnostics.redact(dirty).contains("123456"))
    }

    func testDeviceTransportNeverSucceeds() {
        // If either ever returns, the test fails: there is no on-device
        // installer, and there must never look like one.
        do {
            try DeviceTransport.installOnDevice(ipaName: "Demo.ipa")
            XCTFail("installOnDevice must throw")
        } catch {
            XCTAssertTrue("\(error)".count > 0)
        }
        do {
            try DeviceTransport.refreshOnDevice(appName: "Demo")
            XCTFail("refreshOnDevice must throw")
        } catch {
            XCTAssertTrue("\(error)".count > 0)
        }
    }

    func testChecklistFindsMissingPairing() {
        let app = ManagedApp(name: "Demo", bundleID: "com.example.demo",
                             version: "1.0", installedAt: Date())
        let issues = Diagnostics.checklist(app: app, hasPairingFile: false)
        XCTAssertTrue(issues.contains { $0.title.contains("Pairing") || $0.fix.contains("Pairing") })
    }

    func testChecklistFlagsExpiredApp() {
        let old = Date().addingTimeInterval(-8 * 86_400)
        let app = ManagedApp(name: "Demo", bundleID: "com.example.demo",
                             version: "1.0", installedAt: old)
        XCTAssertTrue(app.isExpired)
        let issues = Diagnostics.checklist(app: app, hasPairingFile: true)
        XCTAssertTrue(issues.contains { $0.severe })
    }
}
