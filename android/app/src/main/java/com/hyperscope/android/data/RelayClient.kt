package com.hyperscope.android.data

import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import org.json.JSONObject
import java.io.BufferedReader
import java.io.InputStreamReader
import java.net.Socket
import java.security.SecureRandom
import java.util.Base64

/**
 * Minimal WebSocket + relay client for the HyperScope P2P protocol.
 *
 * The Android panel is a pure outbound client: it queries the node's
 * hyper-relay for a temporary direct address, then connects to it over plain
 * HTTP to fetch metrics (same shape as the existing direct NodeClient path).
 */
object RelayClient {
    private const val RELAY_PORT = 8686
    private val json = JSONObject()

    /**
     * Ask a node's hyper-relay (same machine as the node) to wake the local
     * collector and return a fresh metrics snapshot. Returns the system data
     * object, or null when the relay is offline or the collector cannot run.
     * All relays serve WSS (TLS).
     */
    suspend fun queryCollect(relayHost: String, nodeName: String, tls: Boolean = true): JSONObject? =
        withContext(Dispatchers.IO) {
            try {
                val ws = WebSocketClient("wss://$relayHost:$RELAY_PORT/ws", true)
                ws.connect(5000)
                val query = JSONObject()
                    .put("type", "query")
                    .put("node", nodeName)
                ws.sendText(query.toString())
                val resp = ws.receiveText(5000)
                ws.close()
                val v = JSONObject(resp)
                if (v.optString("type") == "data") v.optJSONObject("data") else null
            } catch (e: Exception) {
                null
            }
        }

    /**
     * Send a control command through the relay (spawns hyper-node control).
     * The device signs the command (device_id + signature) so the relay can
     * authorize it against the node's trusted-device list.
     * Returns true when the relay acknowledged (result ok).
     */
    suspend fun sendCmd(relayHost: String, nodeName: String, cmd: String, tls: Boolean = true, auth: Pair<String, String> = "" to ""): Boolean =
        withContext(Dispatchers.IO) {
            try {
                val ws = WebSocketClient("wss://$relayHost:$RELAY_PORT/ws", true)
                ws.connect(5000)
                val msg = JSONObject()
                    .put("type", "cmd")
                    .put("node", nodeName)
                    .put("cmd", cmd)
                    .put("device_id", auth.first)
                    .put("signature", auth.second)
                ws.sendText(msg.toString())
                val resp = ws.receiveText(5000)
                ws.close()
                val v = JSONObject(resp)
                // result is a JSON string containing {"type":"result","ok":bool}
                v.optString("result").let { inner ->
                    try { JSONObject(inner).optBoolean("ok", false) } catch (_: Exception) { false }
                }
            } catch (e: Exception) {
                false
            }
        }
}

/** Minimal RFC 6455 WebSocket client (client frames; optional TLS) for relay calls. */
class WebSocketClient(private val url: String, private val useTls: Boolean = false) {
    private var socket: Socket? = null
    private var input: java.io.BufferedInputStream? = null
    private val rng = SecureRandom()

    fun connect(timeoutMs: Int) {
        val u = java.net.URI(url)
        val port = if (u.port > 0) u.port else if (useTls) 443 else 80
        val raw = Socket()
        raw.connect(java.net.InetSocketAddress(u.host, port), timeoutMs)
        raw.soTimeout = timeoutMs
        val s: Socket = if (useTls) {
            // Trust-all for self-signed relay certs (the relay is a
            // zero-privilege pipe; pinning adds little).
            val ctx = javax.net.ssl.SSLContext.getInstance("TLS")
            ctx.init(null, arrayOf<javax.net.ssl.X509TrustManager>(object : javax.net.ssl.X509TrustManager {
                override fun checkClientTrusted(chain: Array<out java.security.cert.X509Certificate>?, authType: String?) {}
                override fun checkServerTrusted(chain: Array<out java.security.cert.X509Certificate>?, authType: String?) {}
                override fun getAcceptedIssuers(): Array<java.security.cert.X509Certificate> = arrayOf()
            }), java.security.SecureRandom())
            val sf = ctx.socketFactory
            (sf.createSocket(raw, u.host, port, true) as javax.net.ssl.SSLSocket).apply {
                startHandshake()
            }
        } else {
            raw
        }
        socket = s

        val keyBytes = ByteArray(16).also { rng.nextBytes(it) }
        val key = Base64.getEncoder().encodeToString(keyBytes)
        val path = if (u.path.isEmpty()) "/" else u.path
        val handshake = buildString {
            append("GET $path HTTP/1.1\r\n")
            append("Host: ${u.host}:$port\r\n")
            append("Upgrade: websocket\r\n")
            append("Connection: Upgrade\r\n")
            append("Sec-WebSocket-Key: $key\r\n")
            append("Sec-WebSocket-Version: 13\r\n\r\n")
        }
        s.getOutputStream().write(handshake.toByteArray(Charsets.UTF_8))
        s.getOutputStream().flush()

        // Use a BufferedInputStream (not BufferedReader) for the handshake so
        // no WS frame bytes get swallowed by a character buffer.
        val input = java.io.BufferedInputStream(s.getInputStream())
        val sb = StringBuilder()
        var status = ""
        // Read the status line.
        var c = input.read()
        while (c != -1 && c != '\n'.code) {
            sb.append(c.toChar()); c = input.read()
        }
        status = sb.toString().trim()
        if (!status.contains("101")) throw IllegalStateException("handshake failed: $status")
        // Drain remaining header lines.
        while (true) {
            val line = StringBuilder()
            c = input.read()
            if (c == -1) break
            while (c != '\n'.code && c != -1) { line.append(c.toChar()); c = input.read() }
            if (line.isEmpty() || line.toString() == "\r") break
        }
        this.input = input
    }

    fun sendText(text: String) {
        val data = text.toByteArray(Charsets.UTF_8)
        val mask = ByteArray(4).also { rng.nextBytes(it) }
        val out = socket!!.getOutputStream()
        out.write(0x81) // FIN + text
        val len = data.size
        if (len < 126) {
            out.write(0x80 or len)
        } else if (len < 65536) {
            out.write(0x80 or 126)
            out.write((len shr 8) and 0xFF)
            out.write(len and 0xFF)
        } else {
            out.write(0x80 or 127)
            for (i in 7 downTo 0) out.write((len shr (8 * i)) and 0xFF)
        }
        out.write(mask)
        // Byte xor needs kotlin.experimental; use Int math to avoid it.
        for (i in data.indices) out.write((data[i].toInt() xor mask[i % 4].toInt()) and 0xFF)
        out.flush()
    }

    fun receiveText(timeoutMs: Int): String {
        socket!!.soTimeout = timeoutMs
        val input = this.input ?: socket!!.getInputStream()
        // Read frame header.
        var b0 = input.read()
        var b1 = input.read()
        var len = (b1 and 0x7F)
        if (len == 126) {
            len = (input.read() shl 8) or input.read()
        } else if (len == 127) {
            len = 0
            for (i in 0 until 8) len = (len shl 8) or input.read()
        }
        val masked = (b1 and 0x80) != 0
        val mask = if (masked) ByteArray(4).also { input.read(it) } else null
        val payload = ByteArray(len)
        var read = 0
        while (read < len) {
            val n = input.read(payload, read, len - read)
            if (n < 0) break
            read += n
        }
        if (mask != null) {
            for (i in payload.indices) {
                payload[i] = ((payload[i].toInt() xor mask[i % 4].toInt()) and 0xFF).toByte()
            }
        }
        return String(payload, Charsets.UTF_8)
    }

    fun close() {
        try {
            socket?.close()
        } catch (_: Exception) {
        }
        socket = null
    }
}
