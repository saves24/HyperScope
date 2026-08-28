package com.hyperscope.android.ui

import android.app.Application
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import com.hyperscope.android.data.NodeClient
import com.hyperscope.android.data.HsxCodec
import com.hyperscope.android.data.NodeConfig
import com.hyperscope.android.data.NotifItem
import com.hyperscope.android.data.NodeView
import com.hyperscope.android.data.SettingsStore
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.isActive
import kotlinx.coroutines.launch
import java.text.SimpleDateFormat
import java.util.Date
import java.util.HashMap
import java.util.Locale
import kotlin.math.roundToInt

class AppViewModel(app: Application) : AndroidViewModel(app) {
    private val store = SettingsStore(app)
    private val client = NodeClient()

    private val _nodes = MutableStateFlow<List<NodeView>>(emptyList())
    val nodes: StateFlow<List<NodeView>> = _nodes.asStateFlow()

    private val _selected = MutableStateFlow<String?>(null)
    val selected: StateFlow<String?> = _selected.asStateFlow()

    private val _sort = MutableStateFlow("default")
    val sort: StateFlow<String> = _sort.asStateFlow()

    private val _theme = MutableStateFlow("auto")
    val theme: StateFlow<String> = _theme.asStateFlow()

    private val _lang = MutableStateFlow("system")
    val lang: StateFlow<String> = _lang.asStateFlow()

    private val _authState = MutableStateFlow(AuthState.Loading)
    val authState: StateFlow<AuthState> = _authState.asStateFlow()

    private val _authError = MutableStateFlow<String?>(null)
    val authError: StateFlow<String?> = _authError.asStateFlow()

    private val _notifications = MutableStateFlow<List<NotifItem>>(emptyList())
    val notifications: StateFlow<List<NotifItem>> = _notifications.asStateFlow()

    // Tracks previously-active alert keys per node to avoid re-firing each cycle.
    private val activeAlerts = HashMap<String, List<String>>()

    private var refreshJob: Job? = null

    init {
        viewModelScope.launch {
            val list = store.nodes.first()
            _nodes.value = list.map { NodeView(config = it) }
            _selected.value = list.firstOrNull()?.name
            _theme.value = store.theme.first()
            _lang.value = store.lang.first()
            val authed = store.loggedIn.first()
            _authState.value = if (authed) AuthState.Authed
            else if (store.hasAuth.first()) AuthState.Login
            else AuthState.Setup
            if (authed && list.isNotEmpty()) startPolling()
        }
    }

    fun setup(userName: String, password: String, confirm: String) {
        viewModelScope.launch {
            if (userName.isBlank() || password.isBlank()) {
                _authError.value = "Username and password cannot be empty"
                _authState.value = AuthState.Error
                return@launch
            }
            if (password != confirm) {
                _authError.value = "Passwords do not match"
                _authState.value = AuthState.Error
                return@launch
            }
            store.setupCredentials(userName, password)
            store.setLoggedIn(true)
            _authState.value = AuthState.Authed
        }
    }

    fun login(userName: String, password: String) {
        viewModelScope.launch {
            val ok = store.verifyLogin(userName, password)
            if (ok) {
                store.setLoggedIn(true)
                _authState.value = AuthState.Authed
                _authError.value = null
            } else {
                _authError.value = "Wrong username or password"
                _authState.value = AuthState.Error
            }
        }
    }

    fun logout() {
        viewModelScope.launch { store.setLoggedIn(false) }
        _authState.value = AuthState.Login
        _authError.value = null
        _nodes.value = emptyList()
        _selected.value = null
    }

    private fun startPolling() {
        // Never cancel a running loop to restart it: that cancellation surfaces as
        // a CancellationException inside refreshAll() and wrongly marks nodes offline.
        if (refreshJob?.isActive == true) return
        refreshJob = viewModelScope.launch {
            while (isActive) {
                refreshAll()
                delay(5000)
            }
        }
    }

    /**
     * Force-restarts the poll loop and refreshes immediately. Used when returning
     * to the foreground, where the previous loop may have been killed by the OS
     * while backgrounded (stale offline state persists until a node is tapped).
     */
    fun restartPolling() {
        refreshJob?.cancel()
        startPolling()
        viewModelScope.launch { refreshAll() }
    }

    private suspend fun refreshAll() {
        val views = _nodes.value
        val updated = views.map { view ->
            try {
                val sys = client.system(view.config)
                val disks = client.disks(view.config).disks
                val procs = client.processes(view.config).processes
                view.copy(system = sys, disks = disks, processes = procs, online = true, error = null)
            } catch (e: kotlinx.coroutines.CancellationException) {
                // A cancelled request (e.g. leaving the screen) must not be shown
                // as a node failure nor abort the whole batch: keep the node's
                // previous state so the UI never flips to offline on cancel.
                view
            } catch (e: Exception) {
                view.copy(online = false, error = e.message)
            }
        }
        _nodes.value = updated
        detectNotifications(updated)
    }

    private suspend fun detectNotifications(views: List<NodeView>) {
        var added = false
        val time = SimpleDateFormat("MM-dd HH:mm", Locale.getDefault()).format(Date())
        for (view in views) {
            if (!view.online) continue
            val id = view.config.name

            // Resource alerts only (no online/offline notifications)
            val sys = view.system ?: continue
            val keys = ArrayList<String>()
            if (sys.cpu >= 90.0) keys.add("cpu:${sys.cpu.roundToInt()}")
            if (sys.mem_percent >= 90.0) keys.add("mem:${sys.mem_percent.roundToInt()}")
            if (sys.disk_percent >= 90.0) keys.add("disk:${sys.disk_percent.roundToInt()}")
            sys.cpu_temp_raw?.let { if (it >= 85.0) keys.add("temp:${it.roundToInt()}") }
            // Docker containers not running
            try {
                val dock = client.docker(view.config).containers
                dock.filter { !it.running }.forEach { keys.add("docker:${it.name}") }
            } catch (_: kotlinx.coroutines.CancellationException) {
                // ignore cancellation here; detectNotifications is best-effort
            } catch (_: Exception) {}

            val prev = activeAlerts[id] ?: emptyList()
            for (key in keys) {
                if (!prev.contains(key)) {
                    _notifications.value = _notifications.value + NotifItem(
                        time = time, node = view.config.name, kind = "alert", msg = alertMessage(key),
                    )
                    added = true
                }
            }
            activeAlerts[id] = keys
        }
        if (added) {
            // keep the list bounded
            val list = _notifications.value
            if (list.size > 100) _notifications.value = list.takeLast(100)
        }
    }

    private fun alertMessage(key: String): String {
        return when {
            key.startsWith("cpu:") -> "CPU high: ${key.removePrefix("cpu:")}%"
            key.startsWith("mem:") -> "Memory high: ${key.removePrefix("mem:")}%"
            key.startsWith("disk:") -> "Disk high: ${key.removePrefix("disk:")}%"
            key.startsWith("temp:") -> "Temperature high: ${key.removePrefix("temp:")}°C"
            key.startsWith("docker:") -> "Container not running: ${key.removePrefix("docker:")}"
            else -> key
        }
    }

    fun clearNotifications() { _notifications.value = emptyList() }

    fun addNode(name: String, addr: String, port: Int, key: String) {
        viewModelScope.launch {
            val cfg = NodeConfig(name = name.ifBlank { addr }, addr = addr, port = port, key = key)
            val list = _nodes.value.map { it.config } + cfg
            store.saveNodes(list)
            _nodes.value = _nodes.value + NodeView(config = cfg)
            if (_selected.value == null) _selected.value = cfg.name
            startPolling()
        }
    }

    /**
     * Imports nodes from an encrypted .hsxc file (decrypted fully on-device).
     * New nodes are appended; a node whose name already exists is skipped
     * (dedup by name, mirroring the panel's unique-name rule).
     */
    fun importNodes(fileBytes: ByteArray, passphrase: String, onResult: (String) -> Unit) {
        viewModelScope.launch {
            try {
                val payload = HsxCodec.decrypt(fileBytes, passphrase)
                val existing = _nodes.value.map { it.config.name }.toHashSet()
                val new = payload.nodes.filter { !existing.contains(it.name) && it.name.isNotBlank() }
                if (new.isEmpty()) {
                    onResult("No new nodes to import (already present or empty)")
                    return@launch
                }
                val cfgs = new.map { NodeConfig(name = it.name, addr = it.addr, port = it.port.takeIf { p -> p > 0 } ?: 5000, key = it.key) }
                val full = _nodes.value.map { it.config } + cfgs
                store.saveNodes(full)
                _nodes.value = _nodes.value + cfgs.map { NodeView(config = it) }
                if (_selected.value == null) _selected.value = cfgs.first().name
                startPolling()
                onResult("Imported ${cfgs.size} node(s) from config")
            } catch (e: IllegalArgumentException) {
                onResult(e.message ?: "Import failed")
            }
        }
    }

    fun removeNode(name: String) {
        viewModelScope.launch {
            val list = _nodes.value.map { it.config }.filter { it.name != name }
            store.saveNodes(list)
            _nodes.value = _nodes.value.filter { it.config.name != name }
            if (_selected.value == name) {
                _selected.value = _nodes.value.firstOrNull()?.config?.name
            }
        }
    }

    fun selectNode(name: String) { _selected.value = name }

    fun setSort(s: String) { _sort.value = s }

    fun sortedViews(): List<NodeView> {
        val views = _nodes.value
        return when (_sort.value) {
            "cpu" -> views.sortedByDescending { it.system?.cpu ?: 0.0 }
            "mem" -> views.sortedByDescending { it.system?.mem_percent ?: 0.0 }
            else -> views
        }
    }

    fun setTheme(v: String) {
        _theme.value = v
        viewModelScope.launch { store.setTheme(v) }
    }

    fun setLang(v: String) {
        _lang.value = v
        viewModelScope.launch { store.setLang(v) }
    }

    /** Immediate refresh + ensure the poll loop is running (e.g. on return to foreground). */
    fun manualRefresh() {
        viewModelScope.launch {
            startPolling()
            refreshAll()
        }
    }
}

enum class AuthState { Loading, Setup, Login, Authed, Error }