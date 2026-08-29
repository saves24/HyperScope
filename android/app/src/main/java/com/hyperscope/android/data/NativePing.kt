package com.hyperscope.android.data

import android.content.Context
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import java.io.File

/**
 * Runs the bundled ICMP ping binary (assets/ping, arm64 static ELF).
 * The app copies it to its private files dir and executes it, because
 * Android cannot execute assets directly. Returns the raw ping output
 * and whether any reply was received.
 */
object NativePing {
    private const val ASSET = "ping"

    /**
     * Executes the bundled ping against [host]. [count] packets are sent.
     * Returns (ok, outputText); ok is true when at least one reply arrived.
     */
    suspend fun ping(context: Context, host: String, count: Int = 4): Pair<Boolean, String> =
        withContext(Dispatchers.IO) {
            try {
                val bin = ensureBinary(context)
                val proc = ProcessBuilder(bin, host, count.toString())
                    .redirectErrorStream(true)
                    .start()
                val out = proc.inputStream.bufferedReader().readText()
                val exit = proc.waitFor()
                val ok = exit == 0 || out.contains("received")
                ok to out
            } catch (e: Exception) {
                false to "ping failed: ${e.message}"
            }
        }

    private fun ensureBinary(context: Context): String {
        val dir = File(context.filesDir, "bin")
        if (!dir.exists()) dir.mkdirs()
        val target = File(dir, ASSET)
        if (target.exists() && target.length() > 0) return target.absolutePath
        context.assets.open(ASSET).use { input ->
            target.outputStream().use { output -> input.copyTo(output) }
        }
        target.setExecutable(true, true)
        return target.absolutePath
    }
}
