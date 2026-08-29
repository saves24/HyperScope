package com.hyperscope.android.data

import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import kotlinx.serialization.json.Json
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.RequestBody.Companion.toRequestBody
import java.io.IOException
import java.security.MessageDigest
import java.security.SecureRandom
import java.security.cert.CertificateException
import java.security.cert.X509Certificate
import java.util.concurrent.TimeUnit
import javax.net.ssl.SSLContext
import javax.net.ssl.TrustManager
import javax.net.ssl.X509TrustManager

/**
 * Talks directly to hyper-node HTTP(S) servers. Each node authenticates with
 * Authorization: Bearer key (the phone never needs a panel).
 *
 * TLS mode: when NodeConfig.tls is true the client connects over https and
 * pins the server certificate to the SHA256 fingerprint stored in certFp
 * (no system CA trust — the fingerprint IS the trust anchor).
 */
class NodeClient(private val identityKey: String = "") {
    private val json = Json { ignoreUnknownKeys = true }

    /**
     * Sign a control command. When a panel identity was imported from a .hsxc
     * config, sign as "panel" (device_id = panel) so nodes that trust the
     * panel accept the command; otherwise fall back to the local device key.
     * Signature payload is ts:nonce:signature (replay protection); the relay
     * and collector verify against cmd:device_id:ts:nonce.
     */
    private fun signedCmd(node: NodeConfig, cmd: String): Pair<String, String> {
        val ts = System.currentTimeMillis() / 1000
        val nonce = java.util.UUID.randomUUID().toString().replace("-", "").take(16)
        val signMsg = { device: String ->
            val m = "$cmd:$device:$ts:$nonce"
            DeviceIdentity.signWithKey(identityKey, m)
                ?: DeviceIdentity.sign(m)
        }
        if (identityKey.isNotBlank()) {
            val sig = signMsg("panel") ?: return "" to ""
            return "panel" to "$ts:$nonce:$sig"
        }
        val deviceId = DeviceIdentity.publicKeyB64()
        val sig = signMsg(deviceId) ?: return "" to ""
        return deviceId to "$ts:$nonce:$sig"
    }

    private val plainClient = OkHttpClient.Builder()
        .connectTimeout(5, TimeUnit.SECONDS)
        .readTimeout(15, TimeUnit.SECONDS)
        .connectionPool(okhttp3.ConnectionPool(1, 30, TimeUnit.SECONDS))
        .build()

    // Clients per pinned fingerprint so each TLS node uses its own trust anchor.
    private val tlsClients = java.util.concurrent.ConcurrentHashMap<String, OkHttpClient>()

    private fun clientFor(node: NodeConfig): OkHttpClient {
        if (!node.tls) return plainClient
        val fp = node.certFp.uppercase()
        return tlsClients.getOrPut(fp) {
            val trust = FingerprintTrustManager(fp)
            val ssl = SSLContext.getInstance("TLS").apply {
                init(null, arrayOf<TrustManager>(trust), SecureRandom())
            }
            OkHttpClient.Builder()
                .connectTimeout(5, TimeUnit.SECONDS)
                .readTimeout(15, TimeUnit.SECONDS)
                .connectionPool(okhttp3.ConnectionPool(1, 30, TimeUnit.SECONDS))
                .sslSocketFactory(ssl.socketFactory, trust)
                // Hostname verification is intentionally disabled because the
                // trust anchor is the node's certificate SHA-256 fingerprint
                // (pinned in FingerprintTrustManager), not a DNS name / CA chain.
                // Nodes are reached by IP inside the LAN, so hostnames never
                // carry trust here — the fingerprint does.
                .hostnameVerifier { _, _ -> true }
                .build()
        }
    }

    /**
     * Drops every pooled connection so the next request opens a fresh TCP
     * socket. Call when returning to the foreground: the OS often reaps
     * connections while backgrounded and reusing them yields 5s connect
     * timeouts that look like "node offline".
     */
    fun resetConnections() {
        plainClient.connectionPool.evictAll()
        tlsClients.values.forEach { it.connectionPool.evictAll() }
    }

    suspend fun system(node: NodeConfig): NodeSystem {
        // Relay-mode nodes: the relay wakes the local collector and returns
        // the full snapshot directly — no HTTP, no listening port.
        if (node.push) {
            val data = RelayClient.queryCollect(node.addr, node.name, node.tls, node.certFp)
                ?: throw IOException("relay collect failed (node offline?)")
            return runCatching {
                json.decodeFromString(NodeSystem.serializer(), data.toString())
            }.getOrDefault(NodeSystem())
        }
        return get(node, "/system").let { runCatching { json.decodeFromString(NodeSystem.serializer(), it) }.getOrDefault(NodeSystem()) }
    }

    // The relay snapshot contains disks/processes_list/docker/traffic/io, so
    // these endpoints read from the cached snapshot instead of opening a
    // direct HTTP port (the collector is never resident).

    suspend fun disks(node: NodeConfig): DisksResponse {
        if (node.push) {
            val snap = RelayClient.queryCollect(node.addr, node.name, node.tls, node.certFp) ?: return DisksResponse()
            val arr = snap.optJSONArray("disks") ?: return DisksResponse()
            val list = mutableListOf<DiskInfo>()
            for (i in 0 until arr.length()) list.add(json.decodeFromString(DiskInfo.serializer(), arr.getJSONObject(i).toString()))
            return DisksResponse(disks = list)
        }
        return get(node, "/disks").let { runCatching { json.decodeFromString(DisksResponse.serializer(), it) }.getOrDefault(DisksResponse()) }
    }

    suspend fun processes(node: NodeConfig, sort: String = "mem", limit: Int = 20): ProcessesResponse {
        if (node.push) {
            val snap = RelayClient.queryCollect(node.addr, node.name, node.tls, node.certFp) ?: return ProcessesResponse()
            val arr = snap.optJSONArray("processes_list") ?: return ProcessesResponse()
            val all = mutableListOf<ProcInfo>()
            for (i in 0 until arr.length()) all.add(json.decodeFromString(ProcInfo.serializer(), arr.getJSONObject(i).toString()))
            val sorted = when (sort) {
                "cpu" -> all.sortedByDescending { it.cpu }
                "pid" -> all.sortedBy { it.pid }
                else -> all.sortedByDescending { it.rss_mb }
            }
            return ProcessesResponse(processes = sorted.take(limit))
        }
        return get(node, "/processes?sort=$sort&limit=$limit").let {
            runCatching { json.decodeFromString(ProcessesResponse.serializer(), it) }.getOrDefault(ProcessesResponse())
        }
    }

    suspend fun docker(node: NodeConfig): DockerResponse {
        if (node.push) {
            val snap = RelayClient.queryCollect(node.addr, node.name, node.tls, node.certFp) ?: return DockerResponse()
            val containers = snap.optJSONObject("docker")?.optJSONArray("containers") ?: return DockerResponse()
            val list = mutableListOf<ContainerInfo>()
            for (i in 0 until containers.length()) list.add(json.decodeFromString(ContainerInfo.serializer(), containers.getJSONObject(i).toString()))
            return DockerResponse(containers = list)
        }
        return get(node, "/docker").let { runCatching { json.decodeFromString(DockerResponse.serializer(), it) }.getOrDefault(DockerResponse()) }
    }

    suspend fun traffic(node: NodeConfig): TrafficInfo {
        if (node.push) {
            val snap = RelayClient.queryCollect(node.addr, node.name, node.tls, node.certFp) ?: return TrafficInfo()
            val t = snap.optJSONObject("traffic") ?: return TrafficInfo()
            return runCatching { json.decodeFromString(TrafficInfo.serializer(), t.toString()) }.getOrDefault(TrafficInfo())
        }
        return get(node, "/traffic").let { runCatching { json.decodeFromString(TrafficInfo.serializer(), it) }.getOrDefault(TrafficInfo()) }
    }

    suspend fun io(node: NodeConfig): IoInfo {
        if (node.push) {
            val snap = RelayClient.queryCollect(node.addr, node.name, node.tls, node.certFp) ?: return IoInfo()
            val i = snap.optJSONObject("io") ?: return IoInfo()
            return runCatching { json.decodeFromString(IoInfo.serializer(), i.toString()) }.getOrDefault(IoInfo())
        }
        return get(node, "/io").let { runCatching { json.decodeFromString(IoInfo.serializer(), it) }.getOrDefault(IoInfo()) }
    }

    suspend fun reboot(node: NodeConfig) {
        if (node.push) { RelayClient.sendCmd(node.addr, node.name, "reboot", node.tls, signedCmd(node, "reboot"), node.certFp); return }
        post(node, "/reboot")
    }
    suspend fun shutdown(node: NodeConfig) {
        if (node.push) { RelayClient.sendCmd(node.addr, node.name, "shutdown", node.tls, signedCmd(node, "shutdown"), node.certFp); return }
        post(node, "/shutdown")
    }
    suspend fun killProcess(node: NodeConfig, pid: Int) {
        if (node.push) { RelayClient.sendCmd(node.addr, node.name, "/processes/$pid/kill", node.tls, signedCmd(node, "/processes/$pid/kill"), node.certFp); return }
        post(node, "/processes/$pid/kill")
    }

    suspend fun dockerAction(node: NodeConfig, container: String, action: String) {
        if (node.push) { RelayClient.sendCmd(node.addr, node.name, "/docker/$container/$action", node.tls, signedCmd(node, "/docker/$container/$action"), node.certFp); return }
        post(node, "/docker/$container/$action")
    }

    private fun url(node: NodeConfig, path: String): String {
        val scheme = if (node.tls) "https" else "http"
        return "$scheme://${node.addr}:${node.port}$path"
    }

    private suspend fun post(node: NodeConfig, path: String): String =
        withContext(Dispatchers.IO) {
            val req = Request.Builder()
                .url(url(node, path))
                .header("Authorization", "Bearer ${node.key}")
                .post("".toRequestBody("application/json".toMediaType()))
                .build()
            clientFor(node).newCall(req).execute().use { resp ->
                if (!resp.isSuccessful) throw IOException("HTTP ${resp.code}")
                resp.body?.string() ?: ""
            }
        }

    private suspend fun get(node: NodeConfig, path: String): String =
        withContext(Dispatchers.IO) {
            val req = Request.Builder()
                .url(url(node, path))
                .header("Authorization", "Bearer ${node.key}")
                .get()
                .build()
            clientFor(node).newCall(req).execute().use { resp ->
                if (!resp.isSuccessful) throw IOException("HTTP ${resp.code}")
                resp.body?.string() ?: ""
            }
        }
}

/** X509TrustManager that accepts a certificate only when its SHA256 fingerprint matches. */
private class FingerprintTrustManager(private val expectedFp: String) : X509TrustManager {
    private fun matches(cert: X509Certificate): Boolean {
        val digest = MessageDigest.getInstance("SHA-256").digest(cert.encoded)
        val fp = digest.joinToString("") { "%02X".format(it) }
        return fp == expectedFp
    }

    override fun checkClientTrusted(chain: Array<out X509Certificate>?, authType: String?) {
        throw CertificateException("client certs not used")
    }

    override fun checkServerTrusted(chain: Array<out X509Certificate>?, authType: String?) {
        if (chain.isNullOrEmpty()) throw CertificateException("empty cert chain")
        if (!matches(chain[0])) throw CertificateException("certificate fingerprint mismatch")
    }

    override fun getAcceptedIssuers(): Array<X509Certificate> = arrayOf()
}
