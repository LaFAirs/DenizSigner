import SwiftUI

/// Four-tab root. Every tab is usable without a manual.
struct ContentView: View {
    var body: some View {
        TabView {
            HomeView()
                .tabItem { Label("home.tab", systemImage: "house.fill") }
            AppsView()
                .tabItem { Label("apps.tab", systemImage: "square.stack.3d.up.fill") }
            RefreshView()
                .tabItem { Label("refresh.tab", systemImage: "arrow.clockwise.circle.fill") }
            SettingsView()
                .tabItem { Label("settings.tab", systemImage: "gearshape.fill") }
        }
        .tint(Theme.accent)
    }
}
