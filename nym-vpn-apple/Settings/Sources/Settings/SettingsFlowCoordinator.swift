import SwiftUI
import AppSettings

struct SettingsFlowCoordinator<Content: View>: View {
    @ObservedObject var state: SettingsFlowState
    let content: () -> Content

    var body: some View {
        content()
            .navigationDestination(for: SettingsLink.self, destination: linkDestination)
    }

    @ViewBuilder private func linkDestination(link: SettingsLink) -> some View {
        switch link {
        case .theme:
            AppearanceView(viewModel: AppearanceViewModel(path: $state.path, appSettings: AppSettings.shared))
        case .support:
            SupportView(viewModel: SupportViewModel(path: $state.path))
        }
    }
}
