package com.hyperscope.android.ui

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.Checkbox
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import com.hyperscope.android.R
import com.hyperscope.android.data.NodeView

/**
 * Node manager dialog (add / batch add / batch delete / .hsxc export) and the
 * custom card-order dialog. Kept in their own file so DashboardScreen stays a
 * readable layout-only composable.
 */
@Composable
internal fun NodeManagerDialog(vm: AppViewModel, nodes: List<NodeView>, onDismiss: () -> Unit) {
    var tab by rememberSaveable { mutableStateOf(0) } // 0 = add, 1 = batch add, 2 = delete, 3 = export
    var selectedNames by rememberSaveable { mutableStateOf(setOf<String>()) }
    var batchText by rememberSaveable { mutableStateOf("") }
    var editingNode by remember { mutableStateOf<String?>(null) }

    AlertDialog(
        onDismissRequest = onDismiss,
        title = {
            Row(verticalAlignment = Alignment.CenterVertically) {
                Text(stringResource(R.string.node_manage))
            }
        },
        text = {
            Column {
                Row(horizontalArrangement = Arrangement.spacedBy(6.dp)) {
                    // All four tabs use the same outlined shape; the active one
                    // is tinted with the primary color so they look consistent.
                    listOf(
                        0 to R.string.add_node,
                        1 to R.string.batch_add,
                        2 to R.string.batch_delete,
                        3 to R.string.batch_export,
                    ).forEach { (t, labelRes) ->
                        OutlinedButton(onClick = { tab = t }, modifier = Modifier.weight(1f),
                            colors = if (tab == t)
                                ButtonDefaults.outlinedButtonColors(contentColor = MaterialTheme.colorScheme.primary)
                            else ButtonDefaults.outlinedButtonColors(),
                            contentPadding = PaddingValues(horizontal = 4.dp, vertical = 8.dp)
                        ) {
                            Text(stringResource(labelRes), maxLines = 1,
                                style = MaterialTheme.typography.labelSmall,
                                softWrap = false, overflow = TextOverflow.Ellipsis)
                        }
                    }
                }
                Spacer(Modifier.height(12.dp))
                when (tab) {
                    0 -> {
                        var name by rememberSaveable { mutableStateOf("") }
                        var addr by rememberSaveable { mutableStateOf("") }
                        var port by rememberSaveable { mutableStateOf("8686") }
                        var key by rememberSaveable { mutableStateOf("") }
                        var group by rememberSaveable { mutableStateOf("") }
                        Column(Modifier.verticalScroll(rememberScrollState())) {
                            OutlinedTextField(value = name, onValueChange = { name = it },
                                label = { Text(stringResource(R.string.node_name_opt)) }, modifier = Modifier.fillMaxWidth())
                            Spacer(Modifier.height(8.dp))
                            OutlinedTextField(value = addr, onValueChange = { addr = it },
                                label = { Text(stringResource(R.string.node_addr)) }, modifier = Modifier.fillMaxWidth())
                            Spacer(Modifier.height(8.dp))
                            Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                                OutlinedTextField(value = port, onValueChange = { port = it },
                                    label = { Text(stringResource(R.string.node_port)) },
                                    keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
                                    modifier = Modifier.weight(1f))
                                OutlinedTextField(value = key, onValueChange = { key = it },
                                    label = { Text(stringResource(R.string.node_key)) },
                                    modifier = Modifier.weight(1f))
                            }
                            Spacer(Modifier.height(8.dp))
                            OutlinedTextField(value = group, onValueChange = { group = it },
                                label = { Text(stringResource(R.string.node_group)) }, modifier = Modifier.fillMaxWidth())
                            Spacer(Modifier.height(12.dp))
                            Button(onClick = {
                                if (addr.isNotBlank()) {
                                    vm.addNode(
                                        name, addr, port.toIntOrNull() ?: 8686, key.trim(),
                                        group = group.trim(),
                                    )
                                    onDismiss()
                                }
                            }, modifier = Modifier.fillMaxWidth()) { Text(stringResource(R.string.add)) }
                        }
                    }
                    1 -> {
                        // Batch add: paste multiple nodes (one per line:
                        // "addr[:port],key[,name]") or import from a local .hsxc file.
                        Column {
                            OutlinedTextField(
                                value = batchText,
                                onValueChange = { batchText = it },
                                label = { Text(stringResource(R.string.batch_add_hint)) },
                                modifier = Modifier.fillMaxWidth(),
                                minLines = 4,
                                maxLines = 6,
                            )
                            Spacer(Modifier.height(8.dp))
                            Button(onClick = {
                                vm.batchAddNodes(batchText)
                                onDismiss()
                            }, enabled = batchText.isNotBlank(), modifier = Modifier.fillMaxWidth()) {
                                Text(stringResource(R.string.batch_add_apply))
                            }
                            Spacer(Modifier.height(8.dp))
                            HorizontalDivider()
                            Spacer(Modifier.height(8.dp))
                            ImportConfigSection(vm)
                        }
                    }
                    2 -> {
                        // Batch delete: checkboxes per node + delete selected.
                        if (nodes.isEmpty()) {
                            Text(stringResource(R.string.no_nodes_short),
                                color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.6f))
                        } else {
                            Column(Modifier.verticalScroll(rememberScrollState())) {
                                nodes.forEach { n ->
                                    Row(
                                        Modifier.fillMaxWidth().padding(vertical = 4.dp)
                                            .clickable {
                                                selectedNames = if (n.config.name in selectedNames)
                                                    selectedNames - n.config.name
                                                else selectedNames + n.config.name
                                            },
                                        verticalAlignment = Alignment.CenterVertically,
                                    ) {
                                        Checkbox(
                                            checked = n.config.name in selectedNames,
                                            onCheckedChange = { checked ->
                                                selectedNames = if (checked) selectedNames + n.config.name
                                                else selectedNames - n.config.name
                                            },
                                        )
                                        Column(Modifier.weight(1f)) {
                                            Text(n.config.name, style = MaterialTheme.typography.bodyMedium)
                                            Text("${n.config.addr}:${n.config.port}",
                                                style = MaterialTheme.typography.labelSmall,
                                                color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.6f))
                                        }
                                        TextButton(onClick = { editingNode = n.config.name },
                                            modifier = Modifier.height(30.dp)) {
                                            Text(stringResource(R.string.node_edit),
                                                style = MaterialTheme.typography.labelSmall,
                                                color = MaterialTheme.colorScheme.primary)
                                        }
                                    }
                                }
                            }
                            Spacer(Modifier.height(12.dp))
                            Button(onClick = {
                                vm.removeNodes(selectedNames)
                                selectedNames = emptySet()
                                onDismiss()
                            }, enabled = selectedNames.isNotEmpty(), modifier = Modifier.fillMaxWidth()) {
                                Text(stringResource(R.string.delete_selected, selectedNames.size))
                            }
                        }
                    }
                    3 -> {
                        // Export all nodes as an encrypted .hsxc file.
                        var exportPass by rememberSaveable { mutableStateOf("") }
                        var exportMsg by rememberSaveable { mutableStateOf("") }
                        val context = LocalContext.current
                        val exportOkText = stringResource(R.string.export_ok)
                        Column {
                            Text(stringResource(R.string.export_hint),
                                style = MaterialTheme.typography.labelSmall,
                                color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.7f))
                            Spacer(Modifier.height(8.dp))
                            OutlinedTextField(
                                value = exportPass,
                                onValueChange = { exportPass = it },
                                label = { Text(stringResource(R.string.export_pass)) },
                                modifier = Modifier.fillMaxWidth(),
                            )
                            Spacer(Modifier.height(8.dp))
                            if (exportMsg.isNotBlank()) {
                                Text(exportMsg,
                                    style = MaterialTheme.typography.labelSmall,
                                    color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.7f))
                                Spacer(Modifier.height(8.dp))
                            }
                            Button(onClick = {
                                vm.exportNodes(exportPass) { data, msg ->
                                    if (data == null) {
                                        exportMsg = msg
                                    } else {
                                        // Write to app files dir and offer a share sheet.
                                        val name = "hyper-nodes-" +
                                            java.text.SimpleDateFormat("yyyy-MM-dd", java.util.Locale.getDefault())
                                                .format(java.util.Date()) + ".hsxc"
                                        val file = java.io.File(context.filesDir, name)
                                        file.writeBytes(data)
                                        val uri = androidx.core.content.FileProvider.getUriForFile(
                                            context, context.packageName + ".fileprovider", file)
                                        val share = android.content.Intent(android.content.Intent.ACTION_SEND).apply {
                                            type = "application/octet-stream"
                                            putExtra(android.content.Intent.EXTRA_STREAM, uri)
                                            addFlags(android.content.Intent.FLAG_GRANT_READ_URI_PERMISSION)
                                        }
                                        context.startActivity(android.content.Intent.createChooser(share, exportOkText))
                                        exportMsg = exportOkText
                                        onDismiss()
                                    }
                                }
                            }, modifier = Modifier.fillMaxWidth()) {
                                Text(stringResource(R.string.batch_export))
                            }
                        }
                    }
                }
            }
        },
        confirmButton = {},
        dismissButton = { TextButton(onClick = onDismiss) { Text(stringResource(R.string.cancel)) } },
    )
    // Edit-node dialog (rename / group / alerts) shown on top of the manager.
    val editing = editingNode?.let { n -> nodes.firstOrNull { it.config.name == n } }
    if (editing != null) {
        EditNodeDialog(vm, editing) { editingNode = null }
    }
}

/** Dialog to freely reorder node cards with up/down buttons. */
@Composable
internal fun NodeOrderDialog(vm: AppViewModel, nodes: List<NodeView>, onDismiss: () -> Unit) {
    var order by remember { mutableStateOf(nodes.map { it.config.name }) }

    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text(stringResource(R.string.sort_order)) },
        text = {
            Column(Modifier.verticalScroll(rememberScrollState())) {
                order.forEachIndexed { idx, name ->
                    Row(Modifier.fillMaxWidth().padding(vertical = 4.dp), verticalAlignment = Alignment.CenterVertically) {
                        Text("${idx + 1}.", style = MaterialTheme.typography.bodyMedium,
                            color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.5f), modifier = Modifier.width(32.dp))
                        Text(name, style = MaterialTheme.typography.bodyMedium, modifier = Modifier.weight(1f))
                        // Up/down buttons: IconButton has no default text padding so
                        // the arrows render cleanly at the right edge.
                        IconButton(onClick = { if (idx > 0) { order = order.toMutableList().also { val t = it[idx]; it[idx] = it[idx - 1]; it[idx - 1] = t } } },
                            enabled = idx > 0) { Text("↑", style = MaterialTheme.typography.titleMedium) }
                        IconButton(onClick = { if (idx < order.lastIndex) { order = order.toMutableList().also { val t = it[idx]; it[idx] = it[idx + 1]; it[idx + 1] = t } } },
                            enabled = idx < order.lastIndex) { Text("↓", style = MaterialTheme.typography.titleMedium) }
                    }
                }
            }
        },
        confirmButton = {
            TextButton(onClick = { vm.setNodeOrder(order); onDismiss() }) { Text(stringResource(R.string.done)) }
        },
        dismissButton = { TextButton(onClick = onDismiss) { Text(stringResource(R.string.cancel)) } },
    )
}

/** Edit a node's local config: rename, group, alert thresholds. */
@Composable
internal fun EditNodeDialog(vm: AppViewModel, node: NodeView, onDismiss: () -> Unit) {
    var name by remember { mutableStateOf(node.config.name) }
    var group by remember { mutableStateOf(node.config.group) }
    var alertCpu by remember { mutableStateOf(node.config.alertCpu?.toString() ?: "") }
    var alertMem by remember { mutableStateOf(node.config.alertMem?.toString() ?: "") }
    var alertDisk by remember { mutableStateOf(node.config.alertDisk?.toString() ?: "") }
    var alertTemp by remember { mutableStateOf(node.config.alertTemp?.toString() ?: "") }
    var msg by remember { mutableStateOf<String?>(null) }

    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text(stringResource(R.string.node_edit) + " — " + node.config.name) },
        text = {
            Column(Modifier.verticalScroll(rememberScrollState())) {
                OutlinedTextField(
                    value = name,
                    onValueChange = { name = it },
                    label = { Text(stringResource(R.string.node_name_opt)) },
                    modifier = Modifier.fillMaxWidth(),
                    singleLine = true,
                )
                Spacer(Modifier.height(6.dp))
                OutlinedTextField(
                    value = group,
                    onValueChange = { group = it },
                    label = { Text(stringResource(R.string.node_group)) },
                    modifier = Modifier.fillMaxWidth(),
                    singleLine = true,
                )
                Spacer(Modifier.height(6.dp))
                // Alert thresholds (blank = default)
                Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                    OutlinedTextField(value = alertCpu, onValueChange = { alertCpu = it },
                        label = { Text(stringResource(R.string.node_alert_cpu)) }, modifier = Modifier.weight(1f),
                        singleLine = true)
                    OutlinedTextField(value = alertMem, onValueChange = { alertMem = it },
                        label = { Text(stringResource(R.string.node_alert_mem)) }, modifier = Modifier.weight(1f),
                        singleLine = true)
                }
                Spacer(Modifier.height(6.dp))
                Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                    OutlinedTextField(value = alertDisk, onValueChange = { alertDisk = it },
                        label = { Text(stringResource(R.string.node_alert_disk)) }, modifier = Modifier.weight(1f),
                        singleLine = true)
                    OutlinedTextField(value = alertTemp, onValueChange = { alertTemp = it },
                        label = { Text(stringResource(R.string.node_alert_temp)) }, modifier = Modifier.weight(1f),
                        singleLine = true)
                }
                msg?.let {
                    Spacer(Modifier.height(8.dp))
                    Text(it, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.primary)
                }
            }
        },
        confirmButton = {
            TextButton(onClick = {
                if (name.isBlank()) { msg = "name required"; return@TextButton }
                vm.updateNodeConfig(
                    oldName = node.config.name,
                    newName = name.trim(),
                    group = group.trim(),
                    alertCpu = alertCpu.toDoubleOrNull(),
                    alertMem = alertMem.toDoubleOrNull(),
                    alertDisk = alertDisk.toDoubleOrNull(),
                    alertTemp = alertTemp.toDoubleOrNull(),
                )
                onDismiss()
            }) { Text(stringResource(R.string.done)) }
        },
        dismissButton = { TextButton(onClick = onDismiss) { Text(stringResource(R.string.cancel)) } },
    )
}
