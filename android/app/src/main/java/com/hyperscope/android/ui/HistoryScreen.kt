package com.hyperscope.android.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.FilterChip
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import com.hyperscope.android.data.HistoryPoint
import java.text.SimpleDateFormat
import java.util.Date
import java.util.Locale

private val metrics = listOf("cpu", "mem", "disk", "temp", "net_rx")
private val ranges = listOf("1h", "24h", "7d", "30d")

@Composable
fun HistoryScreen(vm: AppViewModel) {
    val nodes by vm.nodes.collectAsStateWithLifecycle()
    val history by vm.history.collectAsStateWithLifecycle()
    var metric by remember { mutableStateOf("cpu") }
    var range by remember { mutableStateOf("1h") }

    LaunchedEffect(Unit) { if (nodes.nodes.isNotEmpty()) vm.loadHistory(nodes.nodes.first().id, metric, range) }

    Column(Modifier.fillMaxSize().padding(16.dp)) {
        Text("历史趋势", style = MaterialTheme.typography.titleMedium)
        Row(Modifier.fillMaxWidth().padding(top = 8.dp), horizontalArrangement = Arrangement.spacedBy(6.dp)) {
            metrics.forEach { m ->
                FilterChip(selected = metric == m, onClick = {
                    metric = m
                    if (nodes.nodes.isNotEmpty()) vm.loadHistory(nodes.nodes.first().id, m, range)
                }, label = { Text(m.uppercase()) })
            }
        }
        Row(Modifier.fillMaxWidth().padding(top = 8.dp), horizontalArrangement = Arrangement.spacedBy(6.dp)) {
            ranges.forEach { r ->
                FilterChip(selected = range == r, onClick = {
                    range = r
                    if (nodes.nodes.isNotEmpty()) vm.loadHistory(nodes.nodes.first().id, metric, r)
                }, label = { Text(r) })
            }
        }
        if (history.points.isEmpty()) {
            Text("暂无数据", Modifier.padding(top = 30.dp),
                color = MaterialTheme.colorScheme.onBackground.copy(alpha = 0.6f))
        } else {
            LazyColumn(Modifier.fillMaxWidth().padding(top = 12.dp),
                verticalArrangement = Arrangement.spacedBy(4.dp)) {
                items(history.points.takeLast(60)) { p -> HistoryRow(p) }
            }
        }
    }
}

@Composable
private fun HistoryRow(p: HistoryPoint) {
    val fmt = SimpleDateFormat("MM-dd HH:mm", Locale.getDefault())
    Row(Modifier.fillMaxWidth()) {
        Text(fmt.format(Date(p.ts * 1000)), style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onBackground.copy(alpha = 0.6f), modifier = Modifier.weight(1f))
        Text("${p.value}", style = MaterialTheme.typography.bodySmall)
    }
}
