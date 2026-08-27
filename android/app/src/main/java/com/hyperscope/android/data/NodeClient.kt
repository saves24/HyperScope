package com.hyperscope.android.data

import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import kotlinx.serialization.json.Json
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.RequestBody.Companion.toRequestBody
import java.io.IOException
import java.util.concurrent.TimeUnit

/**
 * Talks directly to hyper-node HTTP servers. Each node authenticates with
 * `Authorization: Bearer <key>` (the phone never needs a panel).
 */
class NodeClient(
    private val client: OkHttpClient = OkHttpClient.Builder()
        .connectTimeout(5, TimeUnit.SECONDS)
        .readTimeout(15, TimeUnit.SECONDS)
        .build(),
) {
    private val json = Json { ignoreUnknownKeys = true }

    private fun url(node: NodeConfig, path: String): String {
        val scheme = "http" // plaintext key mode; TLS (key|cert-fp) not used on phone yet
        return "$scheme://${node.addr}:${node.port}$path"
    }

    suspend fun system(node: NodeConfig): NodeSystem =
        get(node, "/system").let { runCatching { json.decodeFromString(NodeSystem.serializer(), it) }.getOrDefault(NodeSystem()) }

    suspend fun disks(node: NodeConfig): DisksResponse =
        get(node, "/disks").let { runCatching { json.decodeFromString(DisksResponse.serializer(), it) }.getOrDefault(DisksResponse()) }

    suspend fun processes(node: NodeConfig, sort: String = "mem", limit: Int = 20): ProcessesResponse =
        get(node, "/processes?sort=$sort&limit=$limit").let {
            runCatching { json.decodeFromString(ProcessesResponse.serializer(), it) }.getOrDefault(ProcessesResponse())
        }

    suspend fun health(node: NodeConfig): Boolean =
        try { get(node, "/health").let { it.isNotBlank() } } catch (_: Exception) { false }

    private suspend fun get(node: NodeConfig, path: String): String =
        withContext(Dispatchers.IO) {
            val req = Request.Builder()
                .url(url(node, path))
                .header("Authorization", "Bearer ${node.key}")
                .get()
                .build()
            client.newCall(req).execute().use { resp ->
                if (!resp.isSuccessful) throw IOException("HTTP ${resp.code}")
                resp.body?.string() ?: ""
            }
        }
}
