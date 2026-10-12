// DenizSideloader — minimal iOS companion for DenizSigner (Windows).
// MIT. Clean-room implementation, no third-party code vendored.
//
// Honest scope: this app tracks managed IPAs, provisioning expiry and
// pairing files ON DEVICE. Signing and (re-)installation run on Windows
// via DenizSigner over USB. Nothing here pretends iOS can install or
// re-sign IPAs by itself (sandboxed apps cannot).

import SwiftUI

@main
struct DenizSideloaderApp: App {
    @StateObject private var library = AppLibrary()
    @StateObject private var pairing = PairingManager()
    @StateObject private var refresh = RefreshManager()
    @StateObject private var log = DiagnosticLog()

    var body: some Scene {
        WindowGroup {
            ContentView()
                .environmentObject(library)
                .environmentObject(pairing)
                .environmentObject(refresh)
                .environmentObject(log)
                .preferredColorScheme(.dark)
        }
    }
}
