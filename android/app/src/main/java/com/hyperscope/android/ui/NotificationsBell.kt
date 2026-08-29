package com.hyperscope.android.ui

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Notifications
import androidx.compose.material3.BadgedBox
import androidx.compose.material3.Badge
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import androidx.compose.ui.window.Dialog
import com.hyperscope.android.R
import com.hyperscope.android.data.NotifItem

/** Bell button with a red unread-count badge; opens the alert notification popup. */
@Composable
fun NotificationsBell(vm: AppViewModel) {
    val notifications by vm.notifications.collectAsState()
    var show by remember { mutableStateOf(false) }

    Box(contentAlignment = Alignment.TopEnd) {
        BadgedBox(
            badge = {
                if (notifications.isNotEmpty()) {
                    Badge(containerColor = Color(0xFFEF4444)) {
                        Text(
                            if (notifications.size > 99) "99+" else notifications.size.toString(),
                            style = MaterialTheme.typography.labelSmall,
                            color = Color.White,
                        )
                    }
                }
            },
        ) {
            Box(
                Modifier
                    .size(34.dp)
                    .background(
                        MaterialTheme.colorScheme.surface.copy(alpha = 0.6f),
                        RoundedCornerShape(10.dp),
                    )
                    .clickable { show = true },
                contentAlignment = Alignment.Center,
            ) {
                Icon(
                    Icons.Filled.Notifications,
                    contentDescription = stringResource(R.string.notifications),
                    tint = MaterialTheme.colorScheme.onSurface,
                    modifier = Modifier.size(20.dp),
                )
            }
        }
    }

    if (show) {
        Dialog(onDismissRequest = { show = false }) {
            NotificationPanel(vm, notifications) { show = false }
        }
    }
}

@Composable
private fun NotificationPanel(vm: AppViewModel, items: List<NotifItem>, onDismiss: () -> Unit) {
    Surface(
        shape = RoundedCornerShape(14.dp),
        color = MaterialTheme.colorScheme.surface,
        shadowElevation = 8.dp,
        modifier = Modifier.fillMaxWidth(),
    ) {
        Column(Modifier.padding(12.dp)) {
            Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
                Text(stringResource(R.string.notifications), style = MaterialTheme.typography.titleSmall)
                Spacer(Modifier.weight(1f))
                if (items.isNotEmpty()) {
                    TextButton(onClick = {
                        vm.clearNotifications()
                        onDismiss()
                    }) {
                        Text(stringResource(R.string.notif_clear),
                            color = MaterialTheme.colorScheme.error,
                            style = MaterialTheme.typography.bodySmall)
                    }
                }
            }
            if (items.isEmpty()) {
                Text(stringResource(R.string.notif_empty),
                    Modifier.padding(vertical = 20.dp).fillMaxWidth(),
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.6f))
            } else {
                LazyColumn(Modifier.heightIn(max = 320.dp)) {
                    items(items.asReversed()) { n -> NotificationRow(n) }
                }
            }
        }
    }
}

@Composable
private fun NotificationRow(n: NotifItem) {
    val color = Color(0xFFF97316)
    Column(Modifier.fillMaxWidth().padding(vertical = 6.dp)) {
        Row(Modifier.fillMaxWidth()) {
            Text(n.node, style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurface)
            Spacer(Modifier.weight(1f))
            Text(n.time, style = MaterialTheme.typography.labelSmall,
                color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.5f))
        }
        Text(n.msg, style = MaterialTheme.typography.bodySmall, color = color)
    }
}
