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
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import com.hyperscope.android.R

// Shows the currently selected node's connectivity and snapshot status.
@Composable
fun LogsScreen(vm: AppViewModel) {
    val nodes by vm.nodes.collectAsStateWithLifecycle()
    val selected by vm.selected.collectAsStateWithLifecycle()
    val active = nodes.firstOrNull { it.config.name == selected }

    Column(Modifier.fillMaxSize().padding(16.dp)) {
        Text(stringResource(R.string.node_status), style = MaterialTheme.typography.titleMedium)
        if (active == null) {
            Text(stringResource(R.string.no_node_add_first), Modifier.padding(top = 24.dp),
                color = MaterialTheme.colorScheme.onBackground.copy(alpha = 0.6f))
        } else {
            Column(Modifier.fillMaxWidth().padding(top = 12.dp), verticalArrangement = Arrangement.spacedBy(6.dp)) {
                Text(stringResource(R.string.nodes) + ": ${active.config.name}", style = MaterialTheme.typography.bodyMedium)
                Text(stringResource(R.string.address) + ": ${active.config.addr}:${active.config.port}")
                Text(
                    stringResource(R.string.status) + ": " + stringResource(if (active.online) R.string.online else R.string.offline),
                    color = if (active.online) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.error,
                )
                active.error?.let {
                    Text(stringResource(R.string.error) + ": $it", color = MaterialTheme.colorScheme.error)
                }
            }
        }
    }
}
