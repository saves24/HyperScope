package com.hyperscope.android.data

import android.app.NotificationChannel
import android.app.NotificationManager
import android.content.Context
import android.os.Build
import androidx.core.app.NotificationCompat

/**
 * Posts system notifications for alerts so the user sees them even when the
 * app is backgrounded. Uses a single channel; Android 13+ requires the
 * POST_NOTIFICATIONS runtime permission (requested on first launch).
 */
object Notifier {
    const val CHANNEL_ID = "alerts"
    const val SILENT_CHANNEL_ID = "alerts_silent"
    private const val NOTIF_ID = 1001

    fun ensureChannel(context: Context) {
        val nm = context.getSystemService(Context.NOTIFICATION_SERVICE) as NotificationManager
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            val channel = NotificationChannel(
                CHANNEL_ID, "HyperScope alerts", NotificationManager.IMPORTANCE_HIGH
            ).apply {
                description = "Node alert notifications (CPU / memory / disk / temp / docker)"
            }
            nm.createNotificationChannel(channel)
            // A second, lower-importance channel so users can mute alert sound
            // independently in system settings without disabling alerts.
            val silent = NotificationChannel(
                SILENT_CHANNEL_ID, "HyperScope alerts (silent)", NotificationManager.IMPORTANCE_LOW
            ).apply {
                description = "Node alert notifications without sound"
            }
            nm.createNotificationChannel(silent)
        }
    }

    fun postAlert(context: Context, node: String, message: String) {
        post(context, node, message, CHANNEL_ID)
    }

    /** Posts to the silent channel (used when the user muted alerts in-app). */
    fun postAlertSilent(context: Context, node: String, message: String) {
        post(context, node, message, SILENT_CHANNEL_ID)
    }

    /** Clears the active alert notification and the launcher badge. */
    fun clear(context: Context) {
        val nm = context.getSystemService(Context.NOTIFICATION_SERVICE) as NotificationManager
        nm.cancel(NOTIF_ID)
        setLauncherBadge(context, 0)
    }

    /** Launcher badge count (Android 8+ shows it on the icon). */
    private fun setLauncherBadge(context: Context, count: Int) {
        try {
            val intent = context.packageManager.getLaunchIntentForPackage(context.packageName)
                ?: return
            intent.putExtra("badge_count", count)
            val notif = NotificationCompat.Builder(context, CHANNEL_ID)
                .setSmallIcon(android.R.drawable.stat_notify_error)
                .setNumber(count)
                .setContentTitle("")
                .setContentText("")
                .build()
            val nm = context.getSystemService(Context.NOTIFICATION_SERVICE) as NotificationManager
            nm.notify(NOTIF_ID + 1, notif)
        } catch (_: Exception) {}
    }

    private fun post(context: Context, node: String, message: String, channelId: String) {
        val nm = context.getSystemService(Context.NOTIFICATION_SERVICE) as NotificationManager
        val notif = NotificationCompat.Builder(context, channelId)
            .setSmallIcon(android.R.drawable.stat_notify_error)
            .setContentTitle("⚠️ $node")
            .setContentText(message)
            .setStyle(NotificationCompat.BigTextStyle().bigText(message))
            .setAutoCancel(true)
            .build()
        try {
            nm.notify(NOTIF_ID, notif)
            setLauncherBadge(context, 1)
        } catch (_: SecurityException) {
            // POST_NOTIFICATIONS not granted yet — silently skip.
        }
    }
}
