package com.hyperscope.android.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import com.hyperscope.android.data.EventItem
import java.text.SimpleDateFormat
import java.util.Date
import java.util.Locale

@Composable
fun LogsScreen(vm: AppViewModel) {
    val events by vm.events.collectAsStateWithLifecycle()
    LaunchedEffect(Unit) { vm.loadEvents() }

    Column(Modifier.fillMaxSize().padding(16.dp)) {
        Text("事件记录", style = MaterialTheme.typography.titleMedium)
        if (events.events.isEmpty()) {
            Text("暂无记录", Modifier.padding(top = 24.dp),
                color = MaterialTheme.colorScheme.onBackground.copy(alpha = 0.6f))
        } else {
            LazyColumn(Modifier.fillMaxWidth().padding(top = 12.dp),
                verticalArrangement = Arrangement.spacedBy(8.dp)) {
                items(events.events) { e -> EventRow(e) }
            }
        }
    }
}

@Composable
private fun EventRow(e: EventItem) {
    val fmt = SimpleDateFormat("MM-dd HH:mm:ss", Locale.getDefault())
    Column(Modifier.fillMaxWidth().padding(vertical = 2.dp)) {
        Text(fmt.format(Date(e.ts * 1000)), style = MaterialTheme.typography.labelSmall,
            color = MaterialTheme.colorScheme.onBackground.copy(alpha = 0.5f))
        Text(e.msg, style = MaterialTheme.typography.bodySmall)
    }
}
