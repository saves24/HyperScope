package com.hyperscope.android.data

import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable

@Serializable
data class LoginRequest(val user: String, val pass: String)

@Serializable
data class LoginResponse(val ok: Boolean? = null, val error: String? = null)

@Serializable
data class NodeSummary(
    val id: String = "",
    val name: String = "",
    val owner: String = "",
    val tls: Boolean = false,
    @SerialName("cert_verified") val certVerified: Boolean = false,
    val status: String = "",
    val online: Boolean = false,
    @SerialName("node_name") val nodeName: String = "",
    val version: String = "",
)

@Serializable
data class NodesResponse(val nodes: List<NodeSummary> = emptyList())

@Serializable
data class ApiOk(val ok: Boolean? = null, val error: String? = null, val name: String? = null)

@Serializable
data class SystemResponse(
    val ok: Boolean? = null,
    val hostname: String? = null,
    val version: String? = null,
    val kernel: String? = null,
    val uptime: String? = null,
    val cpu: Double? = null,
    val memory: MemoryStats? = null,
    val load: String? = null,
    val procs: Long? = null,
    val temp: TempStats? = null,
)

@Serializable
data class MemoryStats(
    val total: Long = 0,
    val used: Long = 0,
    val free: Long = 0,
    val percent: Double? = null,
)

@Serializable
data class TempStats(
    val cpu: Double? = null,
    val gpu: Double? = null,
)

@Serializable
data class HistoryResponse(val points: List<HistoryPoint> = emptyList())

@Serializable
data class HistoryPoint(val ts: Long = 0, val value: Double = 0.0)

@Serializable
data class EventsResponse(val events: List<EventItem> = emptyList())

@Serializable
data class EventItem(val ts: Long = 0, val msg: String = "", val level: String = "")

@Serializable
data class StatusResponse(
    val ok: Boolean? = null,
    val version: String? = null,
    val nodes: Int? = null,
    val users: Int? = null,
)
