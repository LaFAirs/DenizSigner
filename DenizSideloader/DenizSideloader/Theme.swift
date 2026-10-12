import SwiftUI

/// Minimal dark-blue design language. Single source for colors,
/// corner radii and the large-button style used across all four tabs.
enum Theme {
    static let background = Color(red: 0.04, green: 0.09, blue: 0.17)
    static let card = Color(red: 0.08, green: 0.15, blue: 0.26)
    static let accent = Color(red: 0.35, green: 0.62, blue: 1.0)
    static let warning = Color(red: 1.0, green: 0.72, blue: 0.30)
    static let danger = Color(red: 1.0, green: 0.42, blue: 0.42)
    static let muted = Color(red: 0.62, green: 0.70, blue: 0.82)
    static let corner: CGFloat = 16
}

struct LargeButtonStyle: ButtonStyle {
    var filled: Bool = true
    func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .font(.headline)
            .frame(maxWidth: .infinity, minHeight: 52)
            .background(filled ? Theme.accent : Theme.card)
            .foregroundStyle(.white)
            .clipShape(RoundedRectangle(cornerRadius: Theme.corner))
            .opacity(configuration.isPressed ? 0.75 : 1.0)
    }
}

struct Card<Content: View>: View {
    let content: Content
    init(@ViewBuilder content: () -> Content) { self.content = content() }
    var body: some View {
        content
            .padding()
            .frame(maxWidth: .infinity, alignment: .leading)
            .background(Theme.card)
            .clipShape(RoundedRectangle(cornerRadius: Theme.corner))
    }
}
