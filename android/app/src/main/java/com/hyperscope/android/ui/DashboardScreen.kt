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
import androidx.compose.foundation.lazy.grid.GridCells
import androidx.compose.foundation.lazy.grid.LazyVerticalGrid
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Add
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import com.hyperscope.android.R
import com.hyperscope.android.data.NodeView
import java.text.SimpleDateFormat
import java.util.Calendar
import java.util.Date
import java.util.Locale

@Composable
fun DashboardScreen(vm: AppViewModel) {
    // Use plain collectAsState(): collectAsStateWithLifecycle() pauses collection
    // while backgrounded and can fail to push the latest value on return to the
    // foreground (nodes appear offline until something triggers a recomposition).
    val nodes by vm.nodes.collectAsState()
    val selected by vm.selected.collectAsState()
    val sort by vm.sort.collectAsState()
    var showAdd by rememberSaveable { mutableStateOf(false) }
    val sorted = vm.sortedViews()
    val active = sorted.firstOrNull { it.config.name == selected } ?: sorted.firstOrNull()

    Column(Modifier.fillMaxSize()) {
        GreetingBar(vm)
        NodeSectionHeader(sort = sort, onSort = vm::setSort, onAddNode = { showAdd = true })
        NodeBar(nodes = sorted, selected = active?.config?.name, onSelect = vm::selectNode)

        when {
            sorted.isEmpty() -> EmptyState()
            active == null -> Text(stringResource(R.string.no_nodes), Modifier.padding(24.dp))
            !active.online -> OfflineState(active)
            else -> NodeDashboard(active)
        }
    }

    if (showAdd) AddNodeDialog(vm) { showAdd = false }
}

@Composable
private fun GreetingBar(vm: AppViewModel) {
    val now = Calendar.getInstance()
    val hour = now.get(Calendar.HOUR_OF_DAY)
    val greeting = when {
        hour < 6 -> "Good night"
        hour < 12 -> "Good morning"
        hour < 18 -> "Good afternoon"
        else -> "Good evening"
    }
    val time = SimpleDateFormat("HH:mm", Locale.getDefault()).format(Date())
    val date = SimpleDateFormat("EEEE, MMMM d, yyyy", Locale.getDefault()).format(Date())
    Row(Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 8.dp),
        verticalAlignment = Alignment.CenterVertically) {
        Text(greeting, style = MaterialTheme.typography.titleMedium)
        Spacer(Modifier.weight(1f))
        Column(horizontalAlignment = Alignment.End) {
            Text(time, style = MaterialTheme.typography.titleSmall)
            Text(date, style = MaterialTheme.typography.labelSmall,
                color = MaterialTheme.colorScheme.onBackground.copy(alpha = 0.6f))
        }
        Spacer(Modifier.width(10.dp))
        NotificationsBell(vm)
    }
}

@Composable
private fun NodeSectionHeader(sort: String, onSort: (String) -> Unit, onAddNode: () -> Unit) {
    Row(Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 4.dp),
        verticalAlignment = Alignment.CenterVertically) {
        Text(stringResource(R.string.nodes), style = MaterialTheme.typography.titleMedium)
        Spacer(Modifier.weight(1f))
        TextButton(onClick = onAddNode) {
            Icon(Icons.Filled.Add, null); Spacer(Modifier.width(4.dp)); Text(stringResource(R.string.add_node))
        }
    }
    Row(Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()).padding(horizontal = 16.dp),
        horizontalArrangement = Arrangement.spacedBy(8.dp)) {
        SortChip("default", R.string.sort_default, sort, onSort)
        SortChip("cpu", R.string.sort_cpu, sort, onSort)
        SortChip("mem", R.string.sort_mem, sort, onSort)
    }
}

@Composable
private fun SortChip(value: String, labelRes: Int, current: String, onSelect: (String) -> Unit) {
    val active = current == value
    Card(shape = RoundedCornerShape(8.dp),
        colors = CardDefaults.cardColors(
            containerColor = if (active) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.surface),
        onClick = { onSelect(value) }) {
        Text(stringResource(labelRes), modifier = Modifier.padding(horizontal = 12.dp, vertical = 6.dp),
            style = MaterialTheme.typography.labelMedium,
            color = if (active) MaterialTheme.colorScheme.onPrimary else MaterialTheme.colorScheme.onSurface)
    }
}

@Composable
private fun NodeBar(nodes: List<NodeView>, selected: String?, onSelect: (String) -> Unit) {
    Row(
        Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()).padding(horizontal = 14.dp, vertical = 10.dp),
        horizontalArrangement = Arrangement.spacedBy(10.dp),
    ) {
        nodes.forEach { n ->
            val isSel = n.config.name == selected
            Card(shape = RoundedCornerShape(12.dp),
                colors = CardDefaults.cardColors(
                    containerColor = if (isSel) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.surface),
                onClick = { onSelect(n.config.name) }) {
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
}

@Composable
private fun EmptyState() {
    Column(Modifier.fillMaxSize().padding(30.dp), horizontalAlignment = Alignment.CenterHorizontally) {
        Spacer(Modifier.height(80.dp))
        Text(stringResource(R.string.no_nodes), style = MaterialTheme.typography.titleMedium)
        Spacer(Modifier.height(8.dp))
        Text(stringResource(R.string.no_nodes_hint), style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onBackground.copy(alpha = 0.6f))
    }
}

@Composable
private fun OfflineState(view: NodeView) {
    Column(Modifier.fillMaxSize().padding(30.dp), horizontalAlignment = Alignment.CenterHorizontally) {
        Spacer(Modifier.height(80.dp))
        Text(stringResource(R.string.node_offline) + " ${view.config.name}", style = MaterialTheme.typography.titleMedium)
        Spacer(Modifier.height(8.dp))
        Text(view.error ?: stringResource(R.string.node_offline_hint), style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onBackground.copy(alpha = 0.6f))
    }
}

@Composable
private fun NodeDashboard(view: NodeView) {
    val sys = view.system
    LazyVerticalGrid(
        columns = GridCells.Fixed(2),
        modifier = Modifier.fillMaxSize().padding(horizontal = 12.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp),
        horizontalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        item(key = "system") {
            GridCard(stringResource(R.string.system), Modifier.fillMaxWidth()) {
                KV(stringResource(R.string.host), sys?.node_name ?: "--")
                KV(stringResource(R.string.version), sys?.version ?: "--")
                KV(stringResource(R.string.kernel), sys?.kernel ?: "--")
                KV(stringResource(R.string.load), sys?.loadavg ?: "--")
                KV(stringResource(R.string.uptime), sys?.uptime ?: "--")
                KV(stringResource(R.string.procs), sys?.processes?.toString() ?: "--")
            }
        }
        item(key = "cpu") {
            GridCard("CPU", Modifier.fillMaxWidth()) {
                KV(stringResource(R.string.usage), "${sys?.cpu ?: 0.0}%")
                KV(stringResource(R.string.cores), sys?.cpu_cores?.toString() ?: "--")
                KV(stringResource(R.string.freq), sys?.cpu_mhz ?: "--")
            }
        }
        item(key = "mem") {
            GridCard(stringResource(R.string.mem), Modifier.fillMaxWidth()) {
                KV(stringResource(R.string.used_total), "${sys?.mem_used ?: "--"} / ${sys?.mem_total ?: "--"}")
                KV(stringResource(R.string.usage), "${sys?.mem_percent ?: 0.0}%")
            }
        }
        item(key = "disk") {
            GridCard(stringResource(R.string.disk), Modifier.fillMaxWidth()) {
                KV(stringResource(R.string.used_total), "${sys?.disk_used ?: "--"} / ${sys?.disk_total ?: "--"}")
                KV(stringResource(R.string.usage), "${sys?.disk_percent ?: 0.0}%")
                view.disks.take(2).forEach { KV("  ${it.name}", "${it.used} / ${it.total}") }
            }
        }
        item(key = "temp") {
            GridCard(stringResource(R.string.temp), Modifier.fillMaxWidth()) {
                KV("CPU", sys?.cpu_temp ?: "N/A")
                KV("GPU", sys?.gpu_temp ?: "N/A")
            }
        }
        item(key = "procs") {
            GridCard(stringResource(R.string.top_procs), Modifier.fillMaxWidth()) {
                view.processes.take(5).forEach { KV(" ${it.pid}", "${it.name} · ${it.cpu}%") }
            }
        }
    }
}

@Composable
private fun GridCard(title: String, modifier: Modifier = Modifier, content: @Composable () -> Unit) {
    Card(modifier = modifier, shape = RoundedCornerShape(14.dp),
        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface),
        elevation = CardDefaults.cardElevation(defaultElevation = 3.dp)) {
        Column(Modifier.padding(12.dp)) {
            Text(title, style = MaterialTheme.typography.labelMedium,
                color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.7f))
            Spacer(Modifier.height(8.dp))
            content()
        }
    }
}

@Composable
private fun KV(label: String, value: String) {
    Row(Modifier.fillMaxWidth().padding(vertical = 2.dp)) {
        Text(label, modifier = Modifier.weight(1f), style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.6f))
        Text(value, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurface)
    }
}

@Composable
private fun AddNodeDialog(vm: AppViewModel, onDismiss: () -> Unit) {
    var name by rememberSaveable { mutableStateOf("") }
    var addr by rememberSaveable { mutableStateOf("") }
    var port by rememberSaveable { mutableStateOf("5000") }
    var key by rememberSaveable { mutableStateOf("") }

    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text(stringResource(R.string.add_node)) },
        text = {
            Column {
                OutlinedTextField(value = name, onValueChange = { name = it },
                    label = { Text(stringResource(R.string.node_name_opt)) }, modifier = Modifier.fillMaxWidth())
                Spacer(Modifier.height(8.dp))
                OutlinedTextField(value = addr, onValueChange = { addr = it },
                    label = { Text(stringResource(R.string.node_addr)) }, modifier = Modifier.fillMaxWidth())
                Spacer(Modifier.height(8.dp))
                OutlinedTextField(value = port, onValueChange = { port = it },
                    label = { Text(stringResource(R.string.node_port)) }, modifier = Modifier.fillMaxWidth())
                Spacer(Modifier.height(8.dp))
                OutlinedTextField(value = key, onValueChange = { key = it },
                    label = { Text(stringResource(R.string.node_key)) }, modifier = Modifier.fillMaxWidth())
            }
        },
        confirmButton = {
            Button(onClick = {
                if (addr.isNotBlank()) {
                    vm.addNode(name, addr, port.toIntOrNull() ?: 5000, key.trim())
                    onDismiss()
                }
            }) { Text(stringResource(R.string.add)) }
        },
        dismissButton = { TextButton(onClick = onDismiss) { Text(stringResource(R.string.cancel)) } },
    )
}
