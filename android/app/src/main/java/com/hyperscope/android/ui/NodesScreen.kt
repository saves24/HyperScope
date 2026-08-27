package com.hyperscope.android.ui

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import com.hyperscope.android.data.NodeSummary

@Composable
fun NodesScreen(vm: AppViewModel) {
    val nodes by vm.nodes.collectAsStateWithLifecycle()
    LaunchedEffect(Unit) { vm.loadNodes() }

    LazyColumn(Modifier.fillMaxSize().padding(16.dp),
        verticalArrangement = Arrangement.spacedBy(10.dp)) {
        if (nodes.nodes.isEmpty()) {
            item { Text("暂无节点", Modifier.padding(20.dp),
                color = MaterialTheme.colorScheme.onBackground.copy(alpha = 0.6f)) }
        }
        items(nodes.nodes) { node -> NodeRow(node) { vm.loadSystem(node.id) } }
    }
}

@Composable
private fun NodeRow(node: NodeSummary, onClick: () -> Unit) {
    val online = node.online || node.status == "online"
    Row(
        Modifier.fillMaxWidth()
            .background(MaterialTheme.colorScheme.surface)
            .clickable { onClick() }
            .padding(14.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Box(Modifier.size(10.dp).background(
            if (online) Color(0xFF22C55E) else Color(0xFFDC2626), CircleShape))
        Column(Modifier.padding(start = 12.dp).weight(1f)) {
            Text(node.nodeName.ifBlank { node.name },
                style = MaterialTheme.typography.titleSmall)
            Text("${node.name}  v${node.version.ifBlank { "--" }}",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.6f))
        }
        Text(if (online) "在线" else "离线", style = MaterialTheme.typography.bodySmall,
            color = if (online) Color(0xFF16A34A) else Color(0xFFDC2626))
    }
}
