import Foundation

/// Device transport boundary — the single most important honesty guard
/// in this codebase.
///
/// A sandboxed App Store / sideloaded app on stock iOS CANNOT install
/// IPAs, inject provisioning profiles, or re-sign other apps: there is
/// no public API for it, and private installation APIs are unavailable
/// to third-party apps (they would also break on every iOS update and
/// risk account/certificate revocation).
///
/// Therefore every operation that needs the device installer is typed
/// `Never`-returning here: call sites CANNOT compile in a fake success.
/// The UI routes these actions to the Windows companion (DenizSigner +
/// USB) with precise instructions, and records the pending state so the
/// refresh tracker stays truthful.
enum DeviceTransportError: LocalizedError {
    case companionRequired(operation: String)
    var errorDescription: String? {
        switch self {
        case let .companionRequired(operation):
            return String(format: NSLocalizedString("transport.companion", comment: ""), operation)
        }
    }
}

enum DeviceTransport {
    /// On-device IPA installation. Always throws: must run on Windows.
    static func installOnDevice(ipaName: String) throws -> Never {
        throw DeviceTransportError.companionRequired(
            operation: String(format: NSLocalizedString("transport.op.install", comment: ""), ipaName))
    }

    /// On-device re-signing. Always throws: must run on Windows.
    static func refreshOnDevice(appName: String) throws -> Never {
        throw DeviceTransportError.companionRequired(
            operation: String(format: NSLocalizedString("transport.op.refresh", comment: ""), appName))
    }
}
