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
        }
    }

    fun postAlert(context: Context, node: String, message: String) {
        val nm = context.getSystemService(Context.NOTIFICATION_SERVICE) as NotificationManager
        val notif = NotificationCompat.Builder(context, CHANNEL_ID)
            .setSmallIcon(android.R.drawable.stat_notify_error)
            .setContentTitle("⚠️ $node")
            .setContentText(message)
            .setStyle(NotificationCompat.BigTextStyle().bigText(message))
            .setAutoCancel(true)
            .build()
        try {
            nm.notify(NOTIF_ID, notif)
        } catch (_: SecurityException) {
            // POST_NOTIFICATIONS not granted yet — silently skip.
        }
    }
}
