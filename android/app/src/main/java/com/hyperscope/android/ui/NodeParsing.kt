package com.hyperscope.android.ui

import com.hyperscope.android.data.ContainerInfo
import com.hyperscope.android.data.NodeConfig
import com.hyperscope.android.data.NodeSystem
import kotlin.math.roundToInt

/**
 * Pure helpers extracted from AppViewModel: node-key parsing, batch-line
 * parsing, and alert detection. These have no VM state dependencies, so they
 * live as top-level functions and are unit-testable in isolation.
 */

/**
 * Splits a node key of the form "key" or "key|SHA256:FINGERPRINT".
 * Returns (plainKey, fingerprintOrEmpty). The fingerprint part (if any)
 * enables TLS with certificate pinning.
 */
internal fun splitKeyAndFingerprint(raw: String): Pair<String, String> {
    val idx = raw.indexOf("|SHA256:")
    if (idx <= 0) return raw.trim() to ""
    val key = raw.substring(0, idx).trim()
    val fp = raw.substring(idx + "|SHA256:".length).trim()
    return key to fp
}

/**
 * One parsed batch-add line: host, port, raw key (may include fingerprint),
 * and display name.
 */
internal data class ParsedBatchLine(
    val host: String,
    val port: Int,
    val key: String,
    val name: String,
)

/**
 * Parses one batch-add line: "addr[:port],key[,name]".
 * Returns null for malformed/blank lines. Never throws.
 */
internal fun parseBatchLine(line: String): ParsedBatchLine? {
    val trimmed = line.trim()
    if (trimmed.isEmpty()) return null
    val parts = trimmed.split(",").map { it.trim() }
    if (parts.size < 2 || parts[0].isEmpty() || parts[1].isEmpty()) return null
    val addrPort = parts[0]
    val host = addrPort.substringBefore(":")
    if (host.isBlank()) return null
    val port = addrPort.substringAfter(":", "8686").toIntOrNull() ?: 8686
    val name = parts.getOrNull(2)?.takeIf { it.isNotBlank() } ?: host
    return ParsedBatchLine(host = host, port = port, key = parts[1], name = name)
}

/**
 * Detects which resource thresholds are exceeded for one node.
 * Returns alert keys like "cpu:95", "mem:88", "disk:70", "temp:90",
 * "docker:<name>" (for each non-running container). Thresholds come from the
 * node config when set, else the defaults (90/90/90/85).
 */
internal fun detectAlertKeys(
    sys: NodeSystem,
    cfg: NodeConfig,
    docker: List<ContainerInfo>,
): List<String> {
    val cpuTh = cfg.alertCpu ?: 90.0
    val memTh = cfg.alertMem ?: 90.0
    val diskTh = cfg.alertDisk ?: 90.0
    val tempTh = cfg.alertTemp ?: 85.0
    val keys = ArrayList<String>()
    if (sys.cpu >= cpuTh) keys.add("cpu:${sys.cpu.roundToInt()}")
    if (sys.mem_percent >= memTh) keys.add("mem:${sys.mem_percent.roundToInt()}")
    if (sys.disk_percent >= diskTh) keys.add("disk:${sys.disk_percent.roundToInt()}")
    sys.cpu_temp_raw?.let { if (it >= tempTh) keys.add("temp:${it.roundToInt()}") }
    docker.filter { !it.running }.forEach { keys.add("docker:${it.name}") }
    return keys
}

/** Human-readable message for an alert key (used by events, webhook, system notification). */
internal fun alertMessage(key: String): String {
    return when {
        key.startsWith("cpu:") -> "CPU high: ${key.removePrefix("cpu:")}%"
        key.startsWith("mem:") -> "Memory high: ${key.removePrefix("mem:")}%"
        key.startsWith("disk:") -> "Disk high: ${key.removePrefix("disk:")}%"
        key.startsWith("temp:") -> "Temperature high: ${key.removePrefix("temp:")}°C"
        key.startsWith("docker:") -> "Container not running: ${key.removePrefix("docker:")}"
        else -> key
    }
}
