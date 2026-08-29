package com.hyperscope.android.data

import android.content.Context
import java.text.SimpleDateFormat
import java.util.Date
import java.util.Locale
import java.util.concurrent.ConcurrentLinkedQueue

/**
 * Lightweight in-app runtime log for debugging. Keeps a bounded ring buffer in
 * memory and mirrors every entry to a file in the app's files dir so it can be
 * exported on demand (Settings -> Runtime log -> Export). Debug builds use this
 * to capture app state: polling results, node errors, control actions, auth.
 */
object AppLog {
    private const val MAX_ENTRIES = 1000
    private val buffer = ConcurrentLinkedQueue<String>()
    private val fmt = SimpleDateFormat("MM-dd HH:mm:ss.SSS", Locale.US)

    @Volatile
    private var file: java.io.File? = null

    /** Must be called once from the Application/Activity context to enable file mirroring. */
    fun init(context: Context) {
        if (file == null) {
            file = java.io.File(context.filesDir, "hsapp.log")
        }
    }

    fun d(tag: String, msg: String) {
        val line = "${fmt.format(Date())} D/$tag: $msg"
        buffer.add(line)
        while (buffer.size > MAX_ENTRIES) buffer.poll()
        writeFile(line)
    }

    fun e(tag: String, msg: String) {
        val line = "${fmt.format(Date())} E/$tag: $msg"
        buffer.add(line)
        while (buffer.size > MAX_ENTRIES) buffer.poll()
        writeFile(line)
    }

    /** Latest log text (bounded), oldest first. */
    fun dump(): String = buffer.joinToString("\n")

    /** Path of the mirrored log file (null until init is called). */
    fun filePath(): String? = file?.absolutePath

    /** Clears the in-memory buffer and truncates the file. */
    fun clear() {
        buffer.clear()
        try { file?.writeText("") } catch (_: Exception) {}
    }

    private fun writeFile(line: String) {
        val f = file ?: return
        try {
            f.appendText("$line\n")
            // keep the file from growing forever (~5MB cap)
            if (f.length() > 5 * 1024 * 1024) {
                f.writeText(f.readText().takeLast(1024 * 1024))
            }
        } catch (_: Exception) {}
    }
}
