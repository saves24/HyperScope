package com.hyperscope.android.ui

import androidx.lifecycle.viewModelScope
import com.hyperscope.android.data.AppLog
import com.hyperscope.android.data.ContainerInfo
import com.hyperscope.android.data.EventItem
import com.hyperscope.android.data.NodeView
import com.hyperscope.android.data.Notifier
import com.hyperscope.android.data.WebhookSender
import com.hyperscope.android.data.NodeClient
import kotlinx.coroutines.launch
import java.text.SimpleDateFormat
import java.util.Date
import java.util.HashMap
import java.util.Locale

/**
 * Alert detection + delivery (in-app list, event stream, webhook, system
 * notification). Holds a reference to the owning ViewModel to mutate
 * notifications/events state and to reach the node client. Split from
 * AppViewModel so it stays a thin state holder.
 */
internal class AlertEngine(
    private val vm: AppViewModel,
    private val client: NodeClient,
) {
    // Tracks previously-active alert keys per node to avoid re-firing each cycle.
    private val activeAlerts = HashMap<String, List<String>>()

    suspend fun detect(views: List<NodeView>) {
        var added = false
        val time = SimpleDateFormat("MM-dd HH:mm", Locale.getDefault()).format(Date())
        for (view in views) {
            if (!view.online) continue
            val id = view.config.name

            // Resource alerts only (no online/offline notifications)
            val sys = view.system ?: continue
            val cfg = view.config
            // Docker containers not running
            var dockContainers: List<ContainerInfo> = emptyList()
            try {
                dockContainers = client.docker(view.config).containers
            } catch (_: kotlinx.coroutines.CancellationException) {
                // ignore cancellation here; detectNotifications is best-effort
            } catch (_: Exception) {}
            val keys = detectAlertKeys(sys, cfg, dockContainers)

            val prev = activeAlerts[id] ?: emptyList()
            for (key in keys) {
                if (!prev.contains(key)) {
                    val msg = alertMessage(key)
                    vm.addNotification(time, view.config.name, msg)
                    vm.addEvent(time, view.config.name, "alert", msg)
                    // Best-effort webhook push (node-level config).
                    if (cfg.webhook.isNotBlank()) {
                        vm.viewModelScope.launch {
                            try {
                                WebhookSender.send(cfg.webhook, view.config.name, key, msg)
                            } catch (e: Exception) {
                                AppLog.e("Alert", "webhook failed: ${e.message}")
                            }
                        }
                    }
                    // System notification so alerts are visible with the app closed.
                    runCatching {
                        Notifier.postAlert(vm.application(), view.config.name, msg)
                    }
                    added = true
                }
            }
            activeAlerts[id] = keys
        }
        if (added) {
            vm.trimNotificationsAndEvents()
        }
    }
}
