import Foundation
import Network
import Constants
import CountriesManager
import CredentialsManager
import MixnetLibrary

public struct MixnetConfig: Codable {
    let apiUrlString: String
    let explorerURLString: String
    let entryGateway: EntryGateway?
    let exitRouter: ExitRouter
    let isTwoHopEnabled: Bool
    let credentialsDataPath: String

    public var name = "NymVPN Mixnet"

    public init(
        apiUrlString: String = Constants.apiUrl.rawValue,
        explorerURLString: String = Constants.explorerURL.rawValue,
        entryGateway: EntryGateway? = .randomLowLatency,
        exitRouter: ExitRouter,
        isTwoHopEnabled: Bool = false,
        name: String = "NymVPN Mixnet",
        credentialsDataPath: String
    ) {
        self.apiUrlString = apiUrlString
        self.explorerURLString = explorerURLString
        self.entryGateway = entryGateway
        self.exitRouter = exitRouter
        self.isTwoHopEnabled = isTwoHopEnabled
        self.name = name
        self.credentialsDataPath = credentialsDataPath
    }
}

// MARK: - VpnConfig -
extension MixnetConfig {
    public func asVpnConfig(mixnetTunnelProvider: MixnetTunnelProvider) throws -> VpnConfig {
        guard
            let apiURL = URL(string: Constants.apiUrl.rawValue),
            let explorerURL = URL(string: Constants.explorerURL.rawValue)
        else {
            throw GeneralNymError.invalidUrl
        }
        return VpnConfig(
            apiUrl: apiURL,
            explorerUrl: explorerURL,
            entryGateway: entryGateway?.entryPoint ?? .randomLowLatency,
            exitRouter: exitRouter.exitPoint,
            enableTwoHop: isTwoHopEnabled,
            tunProvider: mixnetTunnelProvider, 
            credentialDataPath: credentialsDataPath
        )
    }
}
