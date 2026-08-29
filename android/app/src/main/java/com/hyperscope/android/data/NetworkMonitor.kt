package com.hyperscope.android.data

import android.content.Context
import android.net.ConnectivityManager
import android.net.Network
import android.net.NetworkCapabilities
import android.widget.Toast

/**
 * Watches connectivity type (Wi-Fi / cellular / none) and shows a toast when
 * the type changes. Helpful for diagnosing why nodes appear unreachable after
 * switching networks (relay addresses may only be reachable on the LAN).
 */
object NetworkMonitor {
    fun register(context: Context) {
        val cm = context.getSystemService(Context.CONNECTIVITY_SERVICE) as ConnectivityManager
        var lastType = describe(cm)
        cm.registerDefaultNetworkCallback(object : ConnectivityManager.NetworkCallback() {
            override fun onAvailable(network: Network) {
                val cur = describe(cm)
                if (cur != lastType && cur != "none") {
                    lastType = cur
                    Toast.makeText(context, "Network: $cur", Toast.LENGTH_SHORT).show()
                } else {
                    lastType = cur
                }
            }
            override fun onLost(network: Network) {
                val cur = describe(cm)
                if (cur != lastType) {
                    lastType = cur
                    Toast.makeText(context, "Network: $cur", Toast.LENGTH_SHORT).show()
                }
            }
        })
    }

    private fun describe(cm: ConnectivityManager): String {
        val caps = cm.getNetworkCapabilities(cm.activeNetwork) ?: return "none"
        return when {
            caps.hasTransport(NetworkCapabilities.TRANSPORT_WIFI) -> "Wi-Fi"
            caps.hasTransport(NetworkCapabilities.TRANSPORT_CELLULAR) -> "cellular"
            else -> "other"
        }
    }
}
