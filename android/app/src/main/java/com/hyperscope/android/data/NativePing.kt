package com.hyperscope.android.data

import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext

/**
 * Bundled ICMP ping (libping.so, built from native/ping.c via CMake).
 * The native code uses the unprivileged Linux ping socket, so no root is
 * required on Android. JNI call keeps latency minimal (no process spawn).
 */
object NativePing {
    init {
        System.loadLibrary("ping")
    }

    private external fun pingNative(host: String, count: Int): String
    /** Runs a real ICMP ping; returns (ok, output). ok = at least one reply. */
    suspend fun ping(host: String, count: Int = 3): Pair<Boolean, String> =
        withContext(Dispatchers.IO) {
            try {
                val out = pingNative(host, count)
                val ok = out.contains("received") && !out.contains("0 received")
                ok to out
            } catch (e: Throwable) {
                false to "ping failed: ${e.message}"
            }
        }
}
