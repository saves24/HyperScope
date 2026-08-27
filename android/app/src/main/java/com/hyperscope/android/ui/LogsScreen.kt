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

// Shows the currently selected node's live system data (node-side "logs" view
// in this local-panel model = the live snapshot).
@Composable
fun LogsScreen(vm: AppViewModel) {
    val nodes by vm.nodes.collectAsStateWithLifecycle()
    val selected by vm.selected.collectAsStateWithLifecycle()
    val active = nodes.firstOrNull { it.config.name == selected }

    Column(Modifier.fillMaxSize().padding(16.dp)) {
        Text("节点状态", style = MaterialTheme.typography.titleMedium)
        if (active == null) {
            Text("请先在设置页添加节点", Modifier.padding(top = 24.dp),
                color = MaterialTheme.colorScheme.onBackground.copy(alpha = 0.6f))
            return@Column
        }
        Column(Modifier.fillMaxWidth().padding(top = 12.dp), verticalArrangement = Arrangement.spacedBy(6.dp)) {
            Text("节点：${active.config.name}", style = MaterialTheme.typography.bodyMedium)
            Text("地址：${active.config.addr}:${active.config.port}")
            Text("状态：${if (active.online) "在线" else "离线"}",
                color = if (active.online) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.error)
            active.error?.let { Text("错误：$it", color = MaterialTheme.colorScheme.error) }
        }
    }
}
