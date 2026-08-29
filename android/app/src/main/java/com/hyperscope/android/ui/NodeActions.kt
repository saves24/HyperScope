package com.hyperscope.android.ui

import com.hyperscope.android.data.HsxCodec
import com.hyperscope.android.data.NodeConfig
import com.hyperscope.android.data.NodeView
import com.hyperscope.android.data.SettingsStore

/**
 * Pure node-config operations (no VM state): build a NodeConfig from user
 * input, import/export .hsxc payloads, and dedupe helpers. These take their
 * inputs and return results, so AppViewModel stays a thin state holder.
 */

/** Builds a NodeConfig from the add-node form fields (fingerprint-aware key). */
internal fun buildNodeConfig(
    name: String,
    addr: String,
    port: Int,
    key: String,
    group: String = "",
    webhook: String = "",
    alertCpu: Double? = null,
    alertMem: Double? = null,
    alertDisk: Double? = null,
    alertTemp: Double? = null,
): NodeConfig {
    val (keyPart, fpPart) = splitKeyAndFingerprint(key)
    return NodeConfig(
        name = name.ifBlank { addr },
        addr = addr,
        port = port,
        key = keyPart,
        tls = fpPart.isNotBlank(),
        certFp = fpPart,
        group = group,
        webhook = webhook,
        alertCpu = alertCpu,
        alertMem = alertMem,
        alertDisk = alertDisk,
        alertTemp = alertTemp,
        // All nodes run in relay mode (no direct HTTP path).
        push = true,
    )
}

/** Serializes the config list into an encrypted .hsxc ByteArray. */
internal fun exportHsx(nodes: List<NodeConfig>, passphrase: String): ByteArray {
    val hsxNodes = nodes.map { n ->
        val keyWithFp = if (n.certFp.isNotBlank()) "${n.key}|SHA256:${n.certFp}" else n.key
        HsxCodec.HsxNode(name = n.name, addr = n.addr, port = n.port, key = keyWithFp, tls = n.tls)
    }
    val payload = HsxCodec.HsxPayload(nodes = hsxNodes)
    val jsonText = HsxCodec.jsonExport.encodeToString(HsxCodec.HsxPayload.serializer(), payload)
    return HsxCodec.encrypt(jsonText.toByteArray(Charsets.UTF_8), passphrase)
}

/** Decrypts a .hsxc file and maps each entry to a NodeConfig (TLS-aware). */
internal fun importHsxNodes(
    fileBytes: ByteArray,
    passphrase: String,
    existingNames: Set<String>,
): Pair<List<NodeConfig>, String> {
    val payload = HsxCodec.decrypt(fileBytes, passphrase)
    val nodes = payload.nodes
        .filter { !existingNames.contains(it.name) && it.name.isNotBlank() }
        .map { n ->
            val (keyPart, fpPart) = splitKeyAndFingerprint(n.key)
            NodeConfig(
                name = n.name,
                addr = n.addr,
                port = n.port.takeIf { p -> p > 0 } ?: 8686,
                key = keyPart,
                tls = n.tls || fpPart.isNotBlank(),
                certFp = fpPart,
            )
        }
    // The panel embeds its identity private key so imported clients can sign
    // commands as "panel" and be accepted by nodes that already trust it.
    return nodes to payload.identity_key
}
