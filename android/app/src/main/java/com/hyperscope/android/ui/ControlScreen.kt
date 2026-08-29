package com.hyperscope.android.ui

import androidx.compose.foundation.background
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import com.hyperscope.android.R
import com.hyperscope.android.data.NodeView
import kotlinx.coroutines.delay
import kotlin.math.roundToInt

/**
 * Control tab: pick a node from the top card row, then operate on it —
 * reboot/shutdown, view & stop processes, docker list & start/stop/restart.
 */
@Composable
fun ControlScreen(vm: AppViewModel) {
    val nodes by vm.nodes.collectAsState()
    val selected by vm.selected.collectAsState()
    val active = nodes.firstOrNull { it.config.name == selected } ?: nodes.firstOrNull()

    var showRebootConfirm by remember { mutableStateOf(false) }
    var showShutdownConfirm by remember { mutableStateOf(false) }
    var showProcs by remember { mutableStateOf(false) }
    var showDocker by remember { mutableStateOf(false) }
    var opMsg by remember { mutableStateOf<String?>(null) }
    var showPingDialog by remember { mutableStateOf(false) }
    var pingMsg by remember { mutableStateOf<String?>(null) }
    var pingOk by remember { mutableStateOf<Boolean?>(null) }

    // Hoisted non-null active node for dialogs below (function scope, not Column scope).
    val node = active

    Column(Modifier.fillMaxSize().padding(16.dp).verticalScroll(rememberScrollState())) {
        Text(stringResource(R.string.control), style = MaterialTheme.typography.titleLarge)
        // Show the node this tab currently operates on (the selected node).
        Text(
            (node?.config?.name ?: "—") + " · " + (node?.config?.addr ?: ""),
            style = MaterialTheme.typography.labelMedium,
            color = MaterialTheme.colorScheme.onBackground.copy(alpha = 0.6f),
        )
        Spacer(Modifier.height(12.dp))

        // Node selector row (same layout as dashboard top cards)
        Row(
            Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()),
            horizontalArrangement = Arrangement.spacedBy(10.dp),
        ) {
            nodes.forEach { n ->
                val isSel = n.config.name == selected
                Card(shape = RoundedCornerShape(12.dp),
                    colors = CardDefaults.cardColors(
                        containerColor = if (isSel) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.surface),
                    onClick = {
                        opMsg = null // clear stale result when switching nodes
                        vm.selectNode(n.config.name)
                    }) {
                    Row(Modifier.padding(horizontal = 14.dp, vertical = 10.dp), verticalAlignment = Alignment.CenterVertically) {
                        Box(Modifier.size(8.dp).background(if (n.online) Color(0xFF22C55E) else Color(0xFFDC2626), CircleShape))
                        Spacer(Modifier.width(8.dp))
                        Text(n.config.name,
                            color = if (isSel) MaterialTheme.colorScheme.onPrimary else MaterialTheme.colorScheme.onSurface,
                            style = MaterialTheme.typography.bodyMedium)
                    }
                }
            }
        }
        Spacer(Modifier.height(16.dp))

        if (node == null) {
            Text(stringResource(R.string.no_nodes), color = MaterialTheme.colorScheme.onBackground.copy(alpha = 0.6f))
            return@Column
        }

        // ---- offline: show a message instead of control buttons ----
        if (!node.online) {
            Spacer(Modifier.height(24.dp))
            Box(
                Modifier.fillMaxWidth().padding(20.dp),
                contentAlignment = Alignment.Center,
            ) {
                Column(horizontalAlignment = Alignment.CenterHorizontally) {
                    Text("📡", style = MaterialTheme.typography.headlineMedium)
                    Spacer(Modifier.height(10.dp))
                    Text(stringResource(R.string.node_offline),
                        style = MaterialTheme.typography.titleMedium,
                        color = MaterialTheme.colorScheme.onBackground.copy(alpha = 0.7f))
                    if (!node.error.isNullOrBlank()) {
                        Spacer(Modifier.height(6.dp))
                        Text(node.error!!,
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.onBackground.copy(alpha = 0.5f))
                    }
                }
            }
            return@Column
        }

        // ---- reachability probe ----
        Text(stringResource(R.string.ping), style = MaterialTheme.typography.titleSmall)
        Spacer(Modifier.height(8.dp))
        OutlinedButton(onClick = {
            pingMsg = null
            pingOk = null
            showPingDialog = true
            vm.pingNode(node.config.name) { (ok, text) ->
                pingMsg = text
                pingOk = ok
            }
        }, modifier = Modifier.fillMaxWidth()) {
            Text(stringResource(R.string.ping))
        }
        Spacer(Modifier.height(16.dp))

        // ---- power controls ----
        Text(stringResource(R.string.power), style = MaterialTheme.typography.titleSmall)
        Spacer(Modifier.height(8.dp))
        Row(horizontalArrangement = Arrangement.spacedBy(12.dp)) {
            OutlinedButton(onClick = { showRebootConfirm = true }, modifier = Modifier.weight(1f)) {
                Text(stringResource(R.string.reboot))
            }
            OutlinedButton(onClick = { showShutdownConfirm = true }, modifier = Modifier.weight(1f)) {
                Text(stringResource(R.string.shutdown), color = MaterialTheme.colorScheme.error)
            }
        }
        Spacer(Modifier.height(16.dp))

        // ---- process control ----
        Text(stringResource(R.string.processes), style = MaterialTheme.typography.titleSmall)
        Spacer(Modifier.height(8.dp))
        OutlinedButton(onClick = { showProcs = true }, modifier = Modifier.fillMaxWidth()) {
            Text(stringResource(R.string.view_stop_procs))
        }
        Spacer(Modifier.height(16.dp))

        // ---- docker control ----
        Text(stringResource(R.string.docker), style = MaterialTheme.typography.titleSmall)
        Spacer(Modifier.height(8.dp))
        OutlinedButton(onClick = { showDocker = true }, modifier = Modifier.fillMaxWidth()) {
            Text(stringResource(R.string.docker_manage))
        }

        opMsg?.let {
            Spacer(Modifier.height(12.dp))
            Text(it, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.primary)
        }
    }

    if (showRebootConfirm) {
        AlertDialog(
            onDismissRequest = { showRebootConfirm = false },
            title = { Text(stringResource(R.string.reboot_confirm_title)) },
            text = { Text(stringResource(R.string.reboot_confirm_body, node?.config?.name ?: "")) },
            confirmButton = {
                TextButton(onClick = {
                    showRebootConfirm = false
                    node?.let { vm.rebootNode(it.config.name) { opMsg = it } }
                }) { Text(stringResource(R.string.confirm)) }
            },
            dismissButton = { TextButton(onClick = { showRebootConfirm = false }) { Text(stringResource(R.string.cancel)) } },
        )
    }
    if (showShutdownConfirm) {
        AlertDialog(
            onDismissRequest = { showShutdownConfirm = false },
            title = { Text(stringResource(R.string.shutdown_confirm_title)) },
            text = { Text(stringResource(R.string.shutdown_confirm_body, node?.config?.name ?: "")) },
            confirmButton = {
                TextButton(onClick = {
                    showShutdownConfirm = false
                    node?.let { vm.shutdownNode(it.config.name) { opMsg = it } }
                }) { Text(stringResource(R.string.confirm)) }
            },
            dismissButton = { TextButton(onClick = { showShutdownConfirm = false }) { Text(stringResource(R.string.cancel)) } },
        )
    }
    if (showProcs) node?.let { ProcessDialog(vm, it) { showProcs = false } }
    if (showPingDialog) {
        node?.let {
            PingResultDialog(it.config.name, pingMsg, pingOk) { showPingDialog = false }
        }
    }
    if (showDocker) node?.let { DockerDialog(vm, it) { showDocker = false } }
}

@Composable
private fun ProcessDialog(vm: AppViewModel, node: NodeView, onDismiss: () -> Unit) {
    var msg by remember { mutableStateOf<String?>(null) }
    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text(stringResource(R.string.processes) + " — ${node.config.name}") },
        text = {
            Column(Modifier.verticalScroll(rememberScrollState())) {
                node.processes.take(30).forEach { p ->
                    Row(Modifier.fillMaxWidth().padding(vertical = 3.dp), verticalAlignment = Alignment.CenterVertically) {
                        Text("${p.pid}", style = MaterialTheme.typography.labelSmall,
                            color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.5f), modifier = Modifier.width(56.dp))
                        Text(p.name, style = MaterialTheme.typography.bodySmall, modifier = Modifier.weight(1f))
                        Text("${p.cpu.roundToInt()}%", style = MaterialTheme.typography.labelSmall,
                            color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.6f), modifier = Modifier.width(44.dp))
                        TextButton(onClick = { vm.killProcessOnNode(node.config.name, p.pid.toInt()) { msg = it } },
                            modifier = Modifier.height(32.dp)) {
                            Text(stringResource(R.string.stop), color = MaterialTheme.colorScheme.error,
                                style = MaterialTheme.typography.labelMedium)
                        }
                    }
                }
                msg?.let {
                    Spacer(Modifier.height(8.dp))
                    Text(it, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.primary)
                }
            }
        },
        confirmButton = { TextButton(onClick = onDismiss) { Text(stringResource(R.string.done)) } },
    )
}

@Composable
private fun DockerDialog(vm: AppViewModel, node: NodeView, onDismiss: () -> Unit) {
    var msg by remember { mutableStateOf<String?>(null) }
    val containers = node.docker.containers
    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text(stringResource(R.string.docker) + " — ${node.config.name}") },
        text = {
            Column(Modifier.verticalScroll(rememberScrollState())) {
                if (containers.isEmpty()) {
                    Text(stringResource(R.string.docker_hint), style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.6f))
                } else {
                    containers.forEach { c ->
                        Row(Modifier.fillMaxWidth().padding(vertical = 3.dp), verticalAlignment = Alignment.CenterVertically) {
                            Column(Modifier.weight(1f)) {
                                Text(c.name, style = MaterialTheme.typography.bodySmall)
                                Text(c.state + " · " + c.image, style = MaterialTheme.typography.labelSmall,
                                    color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.6f))
                            }
                            if (c.running) {
                                TextButton(onClick = { vm.dockerActionOnNode(node.config.name, c.name, "stop") { msg = it } },
                                    modifier = Modifier.height(30.dp)) {
                                    Text(stringResource(R.string.stop), color = MaterialTheme.colorScheme.error,
                                        style = MaterialTheme.typography.labelMedium)
                                }
                                TextButton(onClick = { vm.dockerActionOnNode(node.config.name, c.name, "restart") { msg = it } },
                                    modifier = Modifier.height(30.dp)) {
                                    Text(stringResource(R.string.restart), style = MaterialTheme.typography.labelMedium)
                                }
                            } else {
                                TextButton(onClick = { vm.dockerActionOnNode(node.config.name, c.name, "start") { msg = it } },
                                    modifier = Modifier.height(30.dp)) {
                                    Text(stringResource(R.string.start), color = MaterialTheme.colorScheme.primary,
                                        style = MaterialTheme.typography.labelMedium)
                                }
                            }
                        }
                    }
                }
                msg?.let {
                    Spacer(Modifier.height(8.dp))
                    Text(it, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.primary)
                }
            }
        },
        confirmButton = { TextButton(onClick = onDismiss) { Text(stringResource(R.string.done)) } },
    )
}


/** Ping result dialog: shows reachability probe output. */
@Composable
fun PingResultDialog(nodeName: String, msg: String?, ok: Boolean?, onDismiss: () -> Unit) {
    // Safety: close automatically if the native call never returns, so the
    // dialog cannot stay open and block the UI.
    LaunchedEffect(msg) {
        if (msg == null) {
            delay(8000)
            onDismiss()
        }
    }
    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text(stringResource(R.string.ping) + " — $nodeName") },
        text = {
            Column {
                if (ok != null) {
                    Text(
                        if (ok) stringResource(R.string.ping_ok) + " ✓"
                        else stringResource(R.string.ping_fail) + " ✗",
                        style = MaterialTheme.typography.titleSmall,
                        color = if (ok) Color(0xFF22C55E) else Color(0xFFDC2626),
                    )
                    Spacer(Modifier.height(6.dp))
                }
                if (msg == null) {
                    Text(stringResource(R.string.ping_running), style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.6f))
                } else {
                    Text(msg, style = MaterialTheme.typography.bodySmall)
                }
            }
        },
        confirmButton = { TextButton(onClick = onDismiss) { Text(stringResource(R.string.done)) } },
    )
}
