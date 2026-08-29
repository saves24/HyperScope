package com.hyperscope.android.ui

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.PaddingValues
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
import androidx.compose.foundation.lazy.grid.LazyGridState
import androidx.compose.foundation.lazy.grid.LazyVerticalGrid
import androidx.compose.foundation.lazy.grid.rememberLazyGridState
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Add
import androidx.compose.material.icons.filled.MoreVert
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import com.hyperscope.android.R
import com.hyperscope.android.data.NodeView
import com.hyperscope.android.data.TrendHistory
import com.hyperscope.android.data.SpeedHistory
import java.text.SimpleDateFormat
import java.util.Calendar
import java.util.Date
import java.util.Locale
import kotlin.math.roundToInt

@Composable
fun DashboardScreen(vm: AppViewModel, gridState: LazyGridState? = null) {
    // Use plain collectAsState(): collectAsStateWithLifecycle() pauses collection
    // while backgrounded and can fail to push the latest value on return to the
    // foreground (nodes appear offline until something triggers a recomposition).
    val nodes by vm.nodes.collectAsState()
    val selected by vm.selected.collectAsState()
    val sort by vm.sort.collectAsState()
    val nodeOrder by vm.nodeOrder.collectAsState()
    var showNodeManager by rememberSaveable { mutableStateOf(false) }
    var showOrderDialog by rememberSaveable { mutableStateOf(false) }
    // Derive the sorted list from the collected StateFlow so Compose subscribes
    // to node changes and recomposes on every update.
    val sorted = when {
        sort == "custom" && nodeOrder.isNotEmpty() ->
            nodes.sortedBy { nodeOrder.indexOf(it.config.name).let { i -> if (i < 0) Int.MAX_VALUE else i } }
        sort == "cpu" -> nodes.sortedByDescending { it.system?.cpu ?: 0.0 }
        sort == "mem" -> nodes.sortedByDescending { it.system?.mem_percent ?: 0.0 }
        else -> nodes
    }
    val active = sorted.firstOrNull { it.config.name == selected } ?: sorted.firstOrNull()

    Column(Modifier.fillMaxSize()) {
        GreetingBar(vm)
        NodeSectionHeader(
            sort = sort,
            onSort = vm::setSort,
            onManage = { showNodeManager = true },
            onCustomOrder = { showOrderDialog = true },
            onlineCount = nodes.count { it.online },
            totalCount = nodes.size,
        )
        NodeBar(nodes = sorted, selected = active?.config?.name, onSelect = vm::selectNode)

        when {
            sorted.isEmpty() -> EmptyState()
            active == null -> Text(stringResource(R.string.no_nodes), Modifier.padding(24.dp))
            !active.online -> OfflineState(active)
            else -> NodeDashboard(vm, active, gridState)
        }
    }

    if (showNodeManager) NodeManagerDialog(vm, nodes) { showNodeManager = false }
    if (showOrderDialog) NodeOrderDialog(vm, nodes) { showOrderDialog = false }
}

@Composable
private fun GreetingBar(vm: AppViewModel) {
    val now = Calendar.getInstance()
    val hour = now.get(Calendar.HOUR_OF_DAY)
    val greeting = when {
        hour < 6 -> stringResource(R.string.greet_night)
        hour < 12 -> stringResource(R.string.greet_morning)
        hour < 18 -> stringResource(R.string.greet_afternoon)
        else -> stringResource(R.string.greet_evening)
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
private fun NodeSectionHeader(sort: String, onSort: (String) -> Unit, onManage: () -> Unit, onCustomOrder: () -> Unit, onlineCount: Int = 0, totalCount: Int = 0) {
    Row(Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 4.dp),
        verticalAlignment = Alignment.CenterVertically) {
        Text(stringResource(R.string.nodes), style = MaterialTheme.typography.titleMedium)
        Spacer(Modifier.width(8.dp))
        if (totalCount > 0) {
            // Health summary: online/total with color coding (all ok = green,
            // some offline = amber, all offline = red).
            val color = when {
                onlineCount == 0 -> Color(0xFFDC2626)
                onlineCount < totalCount -> Color(0xFFF59E0B)
                else -> Color(0xFF22C55E)
            }
            Text("$onlineCount/$totalCount",
                color = color,
                style = MaterialTheme.typography.labelMedium,
                modifier = Modifier
                    .background(color.copy(alpha = 0.12f), RoundedCornerShape(6.dp))
                    .padding(horizontal = 6.dp, vertical = 2.dp))
        }
        Spacer(Modifier.weight(1f))
        TextButton(onClick = onManage) {
            Icon(Icons.Filled.Add, null); Spacer(Modifier.width(4.dp)); Text(stringResource(R.string.node_manage))
        }
    }
    Row(Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()).padding(horizontal = 16.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(8.dp)) {
        SortChip("default", R.string.sort_default, sort, onSort)
        SortChip("cpu", R.string.sort_cpu, sort, onSort)
        SortChip("mem", R.string.sort_mem, sort, onSort)
        // Custom order picker — always neutral (only Default/CPU/Mem highlight).
        Card(
            shape = RoundedCornerShape(8.dp),
            colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface),
            onClick = onCustomOrder,
        ) {
            Row(Modifier.padding(start = 2.dp, end = 10.dp, top = 6.dp, bottom = 6.dp),
                verticalAlignment = Alignment.CenterVertically) {
                Icon(Icons.Filled.MoreVert, null, Modifier.size(14.dp), tint = MaterialTheme.colorScheme.onSurface)
                Spacer(Modifier.width(2.dp))
                Text(stringResource(R.string.sort_order),
                    color = MaterialTheme.colorScheme.onSurface,
                    style = MaterialTheme.typography.labelMedium)
            }
        }
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
                    if (n.config.group.isNotBlank()) {
                        Spacer(Modifier.width(6.dp))
                        Text(n.config.group,
                            color = if (isSel) MaterialTheme.colorScheme.onPrimary.copy(alpha = 0.8f)
                                    else MaterialTheme.colorScheme.primary,
                            style = MaterialTheme.typography.labelSmall)
                    }
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
        // First poll has not completed yet (or we just returned from background):
        // show "fetching" instead of a scary connection error.
        val fetching = view.loading
        if (fetching) {
            CircularProgressIndicator(modifier = Modifier.size(36.dp))
            Spacer(Modifier.height(14.dp))
        }
        Text(
            if (fetching) stringResource(R.string.node_fetching) + " ${view.config.name}"
            else stringResource(R.string.node_offline) + " ${view.config.name}",
            style = MaterialTheme.typography.titleMedium,
        )
        Spacer(Modifier.height(8.dp))
        Text(
            if (fetching) stringResource(R.string.node_fetching_hint)
            else view.error ?: stringResource(R.string.node_offline_hint),
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onBackground.copy(alpha = 0.6f),
        )
    }
}

@Composable
private fun NodeDashboard(vm: AppViewModel, view: NodeView, gridState: LazyGridState? = null) {
    val sys = view.system
    // Fixed item height keeps every card the same size and aligned per row,
    // regardless of how many KV rows each card contains. The system card has 6
    // rows, so KV rows stay compact to fit within the same height.
    val cellMod = Modifier.fillMaxWidth().height(160.dp)
    val trend = vm.trends.collectAsState().value[view.config.name] ?: TrendHistory()
    // Use the hoisted state (preserved across tab switches) or a local one.
    val localGridState = rememberLazyGridState()
    val gridState = gridState ?: localGridState
    LazyVerticalGrid(
        columns = GridCells.Fixed(2),
        modifier = Modifier.fillMaxSize().padding(horizontal = 12.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp),
        horizontalArrangement = Arrangement.spacedBy(12.dp),
        // Extra bottom padding so the last row (trend card) can scroll fully
        // above the bottom navigation bar instead of being clipped.
        contentPadding = PaddingValues(bottom = 16.dp),
        state = gridState,
    ) {
        // CPU + memory first as a pair: both are 160dp gauges, so they sit
        // side by side on the same row (system card then follows).
        item(key = "cpu") {
            GaugeCard("CPU", sys?.cpu ?: 0.0, "${(sys?.cpu ?: 0.0).roundToInt()}%", cellMod)
        }
        item(key = "mem") {
            GaugeCard(stringResource(R.string.mem), sys?.mem_percent ?: 0.0,
                "${sys?.mem_percent?.roundToInt() ?: 0}%",
                cellMod,
                subLabel = "${formatMem(sys?.mem_used)} / ${formatMem(sys?.mem_total)}")
        }
        item(key = "system") {
            GridCard(stringResource(R.string.system), cellMod) {
                KV(stringResource(R.string.host), sys?.node_name ?: "--")
                KV(stringResource(R.string.version), sys?.version ?: "--")
                KV(stringResource(R.string.kernel), sys?.kernel ?: "--")
                KV(stringResource(R.string.load), sys?.loadavg ?: "--")
                KV(stringResource(R.string.uptime), sys?.uptime ?: "--")
                KV(stringResource(R.string.procs), sys?.processes?.toString() ?: "--")
            }
        }

        item(key = "net") {
            // Speed card: current rx/tx plus a mini trend line (web-panel style).
            val t = view.traffic
            val speed = vm.speedHistories.collectAsState().value[view.config.name] ?: SpeedHistory()
            GridCard(stringResource(R.string.net), cellMod) {
                KV(stringResource(R.string.net_rx), t?.speed_rx_str ?: "--")
                KV(stringResource(R.string.net_tx), t?.speed_tx_str ?: "--")
                KV(stringResource(R.string.host), t?.iface?.takeIf { it.isNotBlank() } ?: "--")
                SpeedMiniChart(speed.rx, speed.tx)
            }
        }

        item(key = "trend") {
            TrendCard(stringResource(R.string.trend), trend, cellMod)
        }
        item(key = "disk") {
            GridCard(stringResource(R.string.disk), cellMod) {
                KV(stringResource(R.string.used_total), "${sys?.disk_used ?: "--"} / ${sys?.disk_total ?: "--"}")
                KV(stringResource(R.string.usage), "${sys?.disk_percent ?: 0.0}%")
                view.disks.take(2).forEach { KV("  ${it.name}", "${it.used} / ${it.total}") }
            }
        }
        item(key = "temp") {
            GridCard(stringResource(R.string.temp), cellMod) {
                KV("CPU", sys?.cpu_temp ?: "N/A")
                KV("GPU", sys?.gpu_temp ?: "N/A")
            }
        }
        item(key = "io") {
            GridCard(stringResource(R.string.io), cellMod) {
                val i = view.io
                KV(stringResource(R.string.io_read), "${"%.2f".format(i?.disk_read_mbs ?: 0.0)} MB/s")
                KV(stringResource(R.string.io_write), "${"%.2f".format(i?.disk_write_mbs ?: 0.0)} MB/s")
                KV(stringResource(R.string.io_tcp), "${i?.tcp_conns ?: 0}")
            }
        }
        item(key = "procs") {
            GridCard(stringResource(R.string.top_procs), cellMod) {
                view.processes.take(5).forEach { KV(" ${it.pid}", "${it.name} · ${it.cpu}%") }
            }
        }
    }
}

@Composable
private fun GridCard(title: String, modifier: Modifier = Modifier, content: @Composable () -> Unit) {
    Card(modifier = modifier, shape = RoundedCornerShape(14.dp),
        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface),
        elevation = CardDefaults.cardElevation(defaultElevation = LocalCardElevation.current)) {
        Column(Modifier.padding(12.dp)) {
            Text(title, style = MaterialTheme.typography.labelMedium,
                color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.7f))
            Spacer(Modifier.height(6.dp))
            content()
        }
    }
}

@Composable
private fun KV(label: String, value: String) {
    Row(Modifier.fillMaxWidth().padding(vertical = 1.dp)) {
        Text(label, modifier = Modifier.weight(1f), style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.6f))
        Text(value, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurface)
    }
}

/** Mini rx/tx speed trend lines inside the speed card (web-panel style). */
@Composable
private fun SpeedMiniChart(rx: List<Double>, tx: List<Double>) {
    if (rx.size < 2 && tx.size < 2) return
    Canvas(modifier = Modifier.fillMaxWidth().height(40.dp).padding(top = 4.dp)) {
        val padX = 2.dp.toPx()
        val chartW = size.width - padX * 2
        val chartH = size.height
        fun drawSeries(values: List<Double>, color: Color) {
            if (values.size < 2) return
            val max = (values.maxOrNull() ?: 1.0).coerceAtLeast(1.0)
            val stepX = chartW / (values.size - 1)
            val points = values.mapIndexed { i, v ->
                val y = chartH - ((v / max).toFloat() * chartH).coerceIn(0f, chartH)
                Offset(padX + i * stepX, y)
            }
            for (i in 0 until points.size - 1) {
                drawLine(color, points[i], points[i + 1], strokeWidth = 1.5.dp.toPx())
            }
        }
        drawSeries(rx, Color(0xFF22C55E))
        drawSeries(tx, Color(0xFF2563EB))
    }
}

/** Formats a memory size string (GB value from hyper-node) with a unit: G for >=1, M below. */
private fun formatMem(v: String?): String {
    val gb = v?.toDoubleOrNull() ?: return "--"
    return if (gb >= 1.0) "${"%.1f".format(gb)}G" else "${(gb * 1024).roundToInt()}M"
}

