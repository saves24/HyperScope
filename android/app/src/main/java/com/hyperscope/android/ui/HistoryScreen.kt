package com.hyperscope.android.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import com.hyperscope.android.R
import java.text.SimpleDateFormat
import java.util.Date
import java.util.Locale

// Snapshot view for the selected node (hyper-node has no persisted history
// endpoint in this version, so this shows live snapshot fields).
@Composable
fun HistoryScreen(vm: AppViewModel) {
    val nodes by vm.nodes.collectAsState()
    val selected by vm.selected.collectAsState()
    val active = nodes.firstOrNull { it.config.name == selected }

    Column(Modifier.fillMaxSize().padding(16.dp)) {
        Text(stringResource(R.string.history_data), style = MaterialTheme.typography.titleMedium)
        val sys = active?.system
        if (sys == null) {
            Text(stringResource(R.string.history_empty), Modifier.padding(top = 24.dp),
                color = MaterialTheme.colorScheme.onBackground.copy(alpha = 0.6f))
        } else {
            val now = SimpleDateFormat("MM-dd HH:mm:ss", Locale.getDefault()).format(Date())
            Column(Modifier.fillMaxWidth().padding(top = 12.dp), verticalArrangement = Arrangement.spacedBy(6.dp)) {
                Text("${active.config.name} @ $now", style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onBackground.copy(alpha = 0.6f))
                Text("CPU: ${sys.cpu}%  ·  MEM: ${sys.mem_used} / ${sys.mem_total}  ·  ${sys.mem_percent}%")
                Text("DISK: ${sys.disk_used} / ${sys.disk_total}  ·  ${sys.disk_percent}%")
                Text("TEMP: CPU ${sys.cpu_temp}  GPU ${sys.gpu_temp}")
                Text("LOAD: ${sys.loadavg}  ·  UP: ${sys.uptime}")
            }
        }
    }
}
