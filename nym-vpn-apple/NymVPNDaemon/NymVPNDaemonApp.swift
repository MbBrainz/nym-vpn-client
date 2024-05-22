import SwiftUI
import AppSettings
import Constants
import Home
import HelperManager
import Theme
import SentryManager

@main
struct NymVPNDaemonApp: App {
    @StateObject private var appSettings = AppSettings.shared

    init() {
        setup()
    }

    var body: some Scene {
        WindowGroup {
            NavigationStack {
                if !appSettings.welcomeScreenDidDisplay {
                    WelcomeView(viewModel: WelcomeViewModel())
                        .transition(.slide)
                } else {
                    HomeView(viewModel: HomeViewModel(selectedNetwork: .mixnet5hop))
                        .transition(.slide)
                }
            }
            .frame(minWidth: 390, idealWidth: 390, minHeight: 800, idealHeight: 800)
            .animation(.default, value: appSettings.welcomeScreenDidDisplay)
            .environmentObject(appSettings)
        }
        .windowResizability(.contentMinSize)
    }
}

private extension NymVPNDaemonApp {
    func setup() {
        ThemeConfiguration.setup()
        SentryManager.shared.setup()
        HelperManager.shared.setup(helperName: Constants.helperName.rawValue)
    }
}