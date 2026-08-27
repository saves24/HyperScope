package com.hyperscope.android.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import java.text.SimpleDateFormat
import java.util.Date
import java.util.Locale

// History is fetched directly from the selected node (hyper-node has no
// persisted history endpoint in this version, so this shows the node's
// reported snapshot fields with timestamps as a placeholder).
@Composable
fun HistoryScreen(vm: AppViewModel) {
    val nodes by vm.nodes.collectAsStateWithLifecycle()
    val selected by vm.selected.collectAsStateWithLifecycle()
    val active = nodes.firstOrNull { it.config.name == selected }

    Column(Modifier.fillMaxSize().padding(16.dp)) {
        Text("历史数据", style = MaterialTheme.typography.titleMedium)
        if (active?.system == null) {
            Text("暂无数据", Modifier.padding(top = 24.dp),
                color = MaterialTheme.colorScheme.onBackground.copy(alpha = 0.6f))
            return@Column
        }
        val sys = active.system
        val now = SimpleDateFormat("MM-dd HH:mm:ss", Locale.getDefault()).format(Date())
        Column(Modifier.fillMaxWidth().padding(top = 12.dp), verticalArrangement = Arrangement.spacedBy(6.dp)) {
            Text("当前快照（${active.config.name} @ $now）", style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onBackground.copy(alpha = 0.6f))
            Text("CPU: ${sys.cpu}%  ·  内存: ${sys.mem_used} / ${sys.mem_total}  ·  ${sys.mem_percent}%")
            Text("磁盘: ${sys.disk_used} / ${sys.disk_total}  ·  ${sys.disk_percent}%")
            Text("温度: CPU ${sys.cpu_temp}  GPU ${sys.gpu_temp}")
            Text("负载: ${sys.loadavg}  ·  运行: ${sys.uptime}")
        }
    }
}
