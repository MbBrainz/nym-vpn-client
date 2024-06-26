import SwiftUI
import AppSettings
import Home
import Theme
import Tunnels

@main
struct NymVPNmacOSNEApp: App {
    init() {
        setup()
    }

    var body: some Scene {
        WindowGroup {
            GeometryReader { proxy in
                NavigationStack {
                    HomeView(viewModel: HomeViewModel(screenSize: proxy.size, selectedNetwork: .mixnet))
                }
            }
            .environmentObject(AppSettings.shared)
            .environmentObject(TunnelsManager.shared)
        }
    }
}

extension NymVPNmacOSNEApp {
    func setup() {
        ThemeConfiguration.setup()
    }
}
