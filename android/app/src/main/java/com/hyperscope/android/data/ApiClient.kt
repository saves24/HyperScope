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

class ApiClient(
    private val client: OkHttpClient = OkHttpClient.Builder()
        .connectTimeout(10, TimeUnit.SECONDS)
        .readTimeout(20, TimeUnit.SECONDS)
        .writeTimeout(20, TimeUnit.SECONDS)
        .build(),
) {
    private val json = Json { ignoreUnknownKeys = true }

    suspend fun login(baseUrl: String, user: String, pass: String): String {
        // Returns the ts-token. The panel sets it as a cookie; for a native
        // client we read the Set-Cookie header and store the token.
        val body = json.encodeToString(LoginRequest.serializer(), LoginRequest(user, pass))
        val resp = post(baseUrl, "/api/login", body, token = null)
        val setCookie = resp.headers["Set-Cookie"] ?: ""
        val token = setCookie
            .split(";")
            .firstOrNull { it.trim().startsWith("ts-token=") }
            ?.substringAfter("=")
            ?.trim()
        if (token.isNullOrEmpty()) {
            throw IOException("Login failed: no token returned (${resp.code})")
        }
        return token
    }

    suspend fun fetchNodes(baseUrl: String, token: String): NodesResponse =
        get(baseUrl, "/api/nodes", token).let {
            json.decodeFromString(NodesResponse.serializer(), it.body!!.string())
        }

    suspend fun fetchSystem(baseUrl: String, token: String, nodeId: String): SystemResponse =
        get(baseUrl, "/api/node/id/${nodeId}/system", token).let {
            runCatching {
                json.decodeFromString(SystemResponse.serializer(), it.body!!.string())
            }.getOrDefault(SystemResponse())
        }

    suspend fun fetchHistory(
        baseUrl: String,
        token: String,
        nodeId: String,
        metric: String,
        range: String,
    ): HistoryResponse =
        get(baseUrl, "/api/node/id/${nodeId}/history?metric=$metric&range=$range", token).let {
            runCatching {
                json.decodeFromString(HistoryResponse.serializer(), it.body!!.string())
            }.getOrDefault(HistoryResponse())
        }

    suspend fun fetchEvents(baseUrl: String, token: String): EventsResponse =
        get(baseUrl, "/api/events", token).let {
            runCatching {
                json.decodeFromString(EventsResponse.serializer(), it.body!!.string())
            }.getOrDefault(EventsResponse())
        }

    suspend fun fetchStatus(baseUrl: String): StatusResponse =
        get(baseUrl, "/api/status", token = null).let {
            runCatching {
                json.decodeFromString(StatusResponse.serializer(), it.body!!.string())
            }.getOrDefault(StatusResponse())
        }

    private suspend fun get(baseUrl: String, path: String, token: String?): okhttp3.Response =
        withContext(Dispatchers.IO) {
            val b = Request.Builder().url(baseUrl + path).get()
            if (!token.isNullOrEmpty()) b.header("Authorization", "Bearer $token")
            client.newCall(b.build()).execute()
        }

    private suspend fun post(baseUrl: String, path: String, body: String, token: String?): okhttp3.Response =
        withContext(Dispatchers.IO) {
            val b = Request.Builder()
                .url(baseUrl + path)
                .post(body.toRequestBody("application/json".toMediaType()))
            if (!token.isNullOrEmpty()) b.header("Authorization", "Bearer $token")
            client.newCall(b.build()).execute()
        }
}
