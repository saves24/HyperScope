package com.hyperscope.android.ui

import androidx.lifecycle.viewModelScope
import com.hyperscope.android.data.AppLog
import com.hyperscope.android.data.NodeConfig
import com.hyperscope.android.data.NodeView
import com.hyperscope.android.data.SettingsStore
import com.hyperscope.android.R
import kotlinx.coroutines.launch
import java.text.SimpleDateFormat
import java.util.Date
import java.util.Locale

/**
 * Node list management: batch add, single remove, batch remove. Holds a
 * reference to the owning ViewModel to mutate the node StateFlow and to
 * trigger refresh/event recording. Split from AppViewModel so it stays a
 * thin state holder.
 */
internal class NodeRepository(
    private val vm: AppViewModel,
    private val store: SettingsStore,
) {
    /**
     * Batch add nodes from pasted text, one per line: "addr[:port],key[,name]".
     * Skips malformed lines and duplicates.
     */
    fun batchAddNodes(text: String) {
        vm.viewModelScope.launch {
            val existing = vm.nodes.value.map { it.config.addr to it.config.port }.toMutableSet()
            var added = 0
            for (raw in text.lines()) {
                val trimmed = raw.trim()
                if (trimmed.isEmpty()) continue
                val parsed = parseBatchLine(trimmed)
                if (parsed == null) {
                    AppLog.e("Node", "batch add: skipped malformed line: $trimmed")
                    continue
                }
                if ((parsed.host to parsed.port) in existing) continue
                val (key, fp) = splitKeyAndFingerprint(parsed.key)
                val cfg = NodeConfig(name = parsed.name, addr = parsed.host, port = parsed.port, key = key,
                    tls = fp.isNotBlank(), certFp = fp)
                vm.appendNode(NodeView(config = cfg))
                existing.add(parsed.host to parsed.port)
                added++
            }
            store.saveNodes(vm.nodes.value.map { it.config })
            AppLog.d("Node", "batch add: $added node(s)")
            if (added > 0) vm.refreshNow()
        }
    }

    fun removeNode(name: String) {
        vm.viewModelScope.launch {
            AppLog.d("Node", "remove node $name")
            val list = vm.nodes.value.map { it.config }.filter { it.name != name }
            store.saveNodes(list)
            vm.removeNodeByName(name)
        }
    }

    /**
     * Remove multiple nodes in a single coroutine. Batch delete MUST NOT loop
     * removeNode(): each call launches its own coroutine and reads the same
     * stale _nodes snapshot, so concurrent removes overwrite each other and the
     * node reappears until a second delete.
     */
    fun removeNodes(names: Set<String>) {
        if (names.isEmpty()) return
        vm.viewModelScope.launch {
            AppLog.d("Node", "remove ${names.size} nodes: $names")
            val time = SimpleDateFormat("MM-dd HH:mm", Locale.getDefault()).format(Date())
            val list = vm.nodes.value.map { it.config }.filter { it.name !in names }
            store.saveNodes(list)
            vm.removeNodesByName(names)
            // Audit trail: who deleted what (local action log).
            names.forEach { n ->
                vm.addEvent(time, n, "admin_action", "deleted node $n (local)")
            }
        }
    }
}
