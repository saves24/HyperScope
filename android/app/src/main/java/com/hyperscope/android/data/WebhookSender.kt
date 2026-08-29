package com.hyperscope.android.data

import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.RequestBody.Companion.toRequestBody
import org.json.JSONObject
import java.util.concurrent.TimeUnit

/**
 * Sends alert webhook pushes (ntfy / Bark / Server Chan / generic JSON POST).
 * Best-effort: short timeout, failures are swallowed by the caller. Payload
 * matches the web panel format so the same endpoints can be used from both.
 */
object WebhookSender {
    private val client = OkHttpClient.Builder()
        .connectTimeout(4, TimeUnit.SECONDS)
        .readTimeout(4, TimeUnit.SECONDS)
        .build()

    suspend fun send(url: String, node: String, key: String, message: String) {
        withContext(Dispatchers.IO) {
            val payload = JSONObject()
                .put("event", "alert")
                .put("node", node)
                .put("key", key)
                .put("message", message)
                .put("time", java.text.SimpleDateFormat("yyyy-MM-dd HH:mm:ss", java.util.Locale.getDefault())
                    .format(java.util.Date()))
                .toString()
            val body = payload.toRequestBody("application/json".toMediaType())
            val req = Request.Builder()
                .url(url)
                .post(body)
                .header("User-Agent", "hyperscope-android/1.0")
                .build()
            client.newCall(req).execute().use { }
        }
    }
}
