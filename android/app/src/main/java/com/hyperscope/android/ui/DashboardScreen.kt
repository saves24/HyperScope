package com.hyperscope.android.ui

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
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
import androidx.compose.foundation.lazy.grid.items
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
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import com.hyperscope.android.data.NodeView
import java.text.SimpleDateFormat
import java.util.Calendar
import java.util.Date
import java.util.Locale

@Composable
fun DashboardScreen(vm: AppViewModel) {
    val nodes by vm.nodes.collectAsStateWithLifecycle()
    val selected by vm.selected.collectAsStateWithLifecycle()
    val sort by vm.sort.collectAsStateWithLifecycle()
    var showAdd by rememberSaveable { mutableStateOf(false) }
    val sorted = vm.sortedViews()
    val active = sorted.firstOrNull { it.config.name == selected } ?: sorted.firstOrNull()

    Column(Modifier.fillMaxSize()) {
        GreetingBar()
        NodeSectionHeader(sort = sort, onSort = vm::setSort, onAddNode = { showAdd = true })
        // Node cards (top, like the web panel's node grid)
        NodeBar(nodes = sorted, selected = active?.config?.name, onSelect = vm::selectNode)

        when {
            sorted.isEmpty() -> EmptyState()
            active == null -> Text("节点不存在", Modifier.padding(24.dp))
            !active.online -> OfflineState(active)
            else -> NodeDashboard(active)
        }
    }

    if (showAdd) AddNodeDialog(vm) { showAdd = false }
}

@Composable
private fun GreetingBar() {
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
    }
}

@Composable
private fun NodeSectionHeader(sort: String, onSort: (String) -> Unit, onAddNode: () -> Unit) {
    Row(Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 4.dp),
        verticalAlignment = Alignment.CenterVertically) {
        Text("Nodes", style = MaterialTheme.typography.titleMedium)
        Spacer(Modifier.weight(1f))
        TextButton(onClick = onAddNode) { Icon(Icons.Filled.Add, null); Spacer(Modifier.width(4.dp)); Text("Add Node") }
    }
    Row(Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()).padding(horizontal = 16.dp),
        horizontalArrangement = Arrangement.spacedBy(8.dp)) {
        SortChip("default", "Default", sort, onSort)
        SortChip("cpu", "CPU", sort, onSort)
        SortChip("mem", "Memory", sort, onSort)
    }
}

@Composable
private fun SortChip(value: String, label: String, current: String, onSelect: (String) -> Unit) {
    val active = current == value
    Card(shape = RoundedCornerShape(8.dp),
        colors = CardDefaults.cardColors(
            containerColor = if (active) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.surface),
        onClick = { onSelect(value) }) {
        Text(label, modifier = Modifier.padding(horizontal = 12.dp, vertical = 6.dp),
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
        Text("还没有节点", style = MaterialTheme.typography.titleMedium)
        Spacer(Modifier.height(8.dp))
        Text("点上方「Add Node」添加一台运行 hyper-node 的机器",
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onBackground.copy(alpha = 0.6f))
    }
}

@Composable
private fun OfflineState(view: NodeView) {
    Column(Modifier.fillMaxSize().padding(30.dp), horizontalAlignment = Alignment.CenterHorizontally) {
        Spacer(Modifier.height(80.dp))
        Text("无法连接 ${view.config.name}", style = MaterialTheme.typography.titleMedium)
        Spacer(Modifier.height(8.dp))
        Text(view.error ?: "检查地址/端口/API Key", style = MaterialTheme.typography.bodySmall,
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
            GridCard("系统", Modifier.fillMaxWidth()) {
                KV("主机", sys?.node_name ?: "--")
                KV("版本", sys?.version ?: "--")
                KV("内核", sys?.kernel ?: "--")
                KV("负载", sys?.loadavg ?: "--")
                KV("运行", sys?.uptime ?: "--")
                KV("进程", sys?.processes?.toString() ?: "--")
            }
        }
        item(key = "cpu") {
            GridCard("CPU", Modifier.fillMaxWidth()) {
                KV("使用率", "${sys?.cpu ?: 0.0}%")
                KV("核心", sys?.cpu_cores?.toString() ?: "--")
                KV("频率", sys?.cpu_mhz ?: "--")
            }
        }
        item(key = "mem") {
            GridCard("内存", Modifier.fillMaxWidth()) {
                KV("已用/总", "${sys?.mem_used ?: "--"} / ${sys?.mem_total ?: "--"}")
                KV("使用率", "${sys?.mem_percent ?: 0.0}%")
            }
        }
        item(key = "disk") {
            GridCard("磁盘", Modifier.fillMaxWidth()) {
                KV("已用/总", "${sys?.disk_used ?: "--"} / ${sys?.disk_total ?: "--"}")
                KV("使用率", "${sys?.disk_percent ?: 0.0}%")
                view.disks.take(2).forEach { KV("  ${it.name}", "${it.used} / ${it.total}") }
            }
        }
        item(key = "temp") {
            GridCard("温度", Modifier.fillMaxWidth()) {
                KV("CPU", sys?.cpu_temp ?: "N/A")
                KV("GPU", sys?.gpu_temp ?: "N/A")
            }
        }
        item(key = "procs") {
            GridCard("进程 TOP", Modifier.fillMaxWidth()) {
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
        title = { Text("Add Node") },
        text = {
            Column {
                OutlinedTextField(value = name, onValueChange = { name = it },
                    label = { Text("名称（可选）") }, modifier = Modifier.fillMaxWidth())
                Spacer(Modifier.height(8.dp))
                OutlinedTextField(value = addr, onValueChange = { addr = it },
                    label = { Text("节点地址，如 192.168.1.100") }, modifier = Modifier.fillMaxWidth())
                Spacer(Modifier.height(8.dp))
                OutlinedTextField(value = port, onValueChange = { port = it },
                    label = { Text("端口（默认 5000）") }, modifier = Modifier.fillMaxWidth())
                Spacer(Modifier.height(8.dp))
                OutlinedTextField(value = key, onValueChange = { key = it },
                    label = { Text("API Key（节点上 hyper-node key show 获取）") },
                    modifier = Modifier.fillMaxWidth())
            }
        },
        confirmButton = {
            Button(onClick = {
                if (addr.isNotBlank()) {
                    vm.addNode(name, addr, port.toIntOrNull() ?: 5000, key.trim())
                    onDismiss()
                }
            }) { Text("添加") }
        },
        dismissButton = { TextButton(onClick = onDismiss) { Text("取消") } },
    )
}
