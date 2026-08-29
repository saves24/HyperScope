package com.hyperscope.android.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import com.hyperscope.android.R
import com.hyperscope.android.data.EventItem

/**
 * Event history: node up/down transitions and alert triggers. Mirrors the
 * panel's events page, but recorded locally by the app.
 */
@Composable
fun EventsScreen(vm: AppViewModel) {
    val events by vm.events.collectAsState()

    Column(Modifier.fillMaxSize().padding(16.dp)) {
        Row(verticalAlignment = Alignment.CenterVertically) {
            Text(stringResource(R.string.events), style = MaterialTheme.typography.titleLarge)
            Spacer(Modifier.weight(1f))
            if (events.isNotEmpty()) {
                OutlinedButton(onClick = { vm.clearEvents() }) {
                    Text(stringResource(R.string.notif_clear))
                }
            }
        }
        Spacer(Modifier.height(12.dp))
        if (events.isEmpty()) {
            Text(stringResource(R.string.events_empty),
                color = MaterialTheme.colorScheme.onBackground.copy(alpha = 0.6f))
        } else {
            LazyColumn(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                items(events.reversed()) { ev -> EventRow(ev) }
            }
        }
    }
}

@Composable
private fun EventRow(ev: EventItem) {
    val color = when (ev.kind) {
        "up", "online" -> Color(0xFF22C55E)
        "down", "offline" -> Color(0xFFDC2626)
        "admin_action" -> Color(0xFF8B5CF6)
        else -> Color(0xFFF59E0B)
    }
    Card(colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface),
        modifier = Modifier.fillMaxWidth()) {
        Row(Modifier.padding(12.dp), verticalAlignment = Alignment.CenterVertically) {
            androidx.compose.foundation.Canvas(Modifier.width(10.dp).height(10.dp)) {
                drawCircle(color)
            }
            Spacer(Modifier.width(10.dp))
            Column(Modifier.weight(1f)) {
                Text(ev.msg, style = MaterialTheme.typography.bodyMedium)
                Text("${ev.node} · ${ev.time}", style = MaterialTheme.typography.labelSmall,
                    color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.6f))
            }
        }
    }
}
