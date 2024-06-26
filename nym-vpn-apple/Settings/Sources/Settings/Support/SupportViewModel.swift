import SwiftUI
import Constants
import UIComponents

struct SupportViewModel {
    private let faqLink = Constants.supportURL.rawValue
    private let emailLink = "mailto:support@nymvpn.com"
    private let matrixLink = "https://matrix.to/#/%23NymVPN:nymtech.chat"
    private let discordLink = "https://discord.com/invite/nym"

    let title = "support".localizedString

    @Binding var path: NavigationPath
    var sections: [SettingsListItemViewModel] {
        [
            faqSectionViewModel(),
            emailSectionViewModel(),
            matrixSectionViewModel(),
            discordSectionViewModel()
        ]
    }

    init(path: Binding<NavigationPath>) {
        _path = path
    }
}

// MARK: - Navigation -
extension SupportViewModel {
    func navigateBack() {
        if !path.isEmpty { path.removeLast() }
    }

    func openExternalURL(urlString: String?) {
        guard let urlString, let url = URL(string: urlString) else { return }
        #if os(iOS)
        UIApplication.shared.open(url)
        #else
        NSWorkspace.shared.open(url)
        #endif
    }
}

// MARK: - Sections -

private extension SupportViewModel {
    func faqSectionViewModel() -> SettingsListItemViewModel {
        SettingsListItemViewModel(
            accessory: .arrow,
            title: "checkFAQ".localizedString,
            imageName: "faq",
            position: SettingsListItemPosition(isFirst: true, isLast: true),
            action: {
                openExternalURL(urlString: faqLink)
            }
        )
    }

    func emailSectionViewModel() -> SettingsListItemViewModel {
        SettingsListItemViewModel(
            accessory: .arrow,
            title: "sendEmail".localizedString,
            imageName: "email",
            position: SettingsListItemPosition(isFirst: true, isLast: true),
            action: {
                openExternalURL(urlString: emailLink)
            }
        )
    }

    func matrixSectionViewModel() -> SettingsListItemViewModel {
        SettingsListItemViewModel(
            accessory: .arrow,
            title: "joinMatrix".localizedString,
            imageName: "matrix",
            position: SettingsListItemPosition(isFirst: true, isLast: true),
            action: {
                openExternalURL(urlString: matrixLink)
            }
        )
    }

    func discordSectionViewModel() -> SettingsListItemViewModel {
        SettingsListItemViewModel(
            accessory: .arrow,
            title: "joinDiscord".localizedString,
            imageName: "discord",
            position: SettingsListItemPosition(isFirst: true, isLast: true),
            action: {
                openExternalURL(urlString: discordLink)
            }
        )
    }
}
