package net.nymtech

import android.content.Intent
import android.os.ParcelFileDescriptor
import net.mullvad.talpid.TalpidVpnService
import net.nymtech.vpn_client.Action
import net.nymtech.vpn_client.NymVpnClient
import net.nymtech.vpn_client.VpnClient
import timber.log.Timber

class NymVpnService : TalpidVpnService() {

    private var vpnClient : VpnClient = NymVpnClient()
    private lateinit var vpnThread: Thread

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        Timber.d("On start received")
        return if (intent?.action == Action.START.name) {

            Timber.d("VPN start intent")
            startVpn()
            START_STICKY
        } else {
            Timber.d("VPN stopping intent")
            stopVpn()
            START_NOT_STICKY
        }
    }

    override fun onCreate() {
        connectivityListener.register(this)
    }

    private fun startVpn() {
        super.createTun()
        vpnThread = Thread {
            try {
                // Create a new VPN Builder
                val builder = Builder()

                // Set the VPN parameters
                builder.setSession("nymtun")


                // Establish the VPN connection
                builder.establish()?.let {
                    Timber.d("Interface created")
                    start()
                }
//                }
            } catch (e: Exception) {
                // Handle VPN connection errors
                e.printStackTrace()
            } finally {
                //stopVpn()
            }
        }

        vpnThread.start()
    }

    private fun stopVpn() {
        try {
            stopSelf()
        } catch (e: Exception) {
            e.printStackTrace()
        }
    }

    override fun onDestroy() {
        super.onDestroy()
        connectivityListener.unregister()
        stopVpn()
    }

    private fun start() {
        vpnClient.connect("FR", "FR", this)
    }
}