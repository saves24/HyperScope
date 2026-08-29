package com.hyperscope.android.ui

import android.app.Application
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import com.hyperscope.android.data.NodeClient
import com.hyperscope.android.data.NativePing
import com.hyperscope.android.data.HsxCodec
import com.hyperscope.android.data.NodeConfig
import com.hyperscope.android.data.NotifItem
import com.hyperscope.android.data.NodeView
import com.hyperscope.android.data.DockerResponse
import com.hyperscope.android.data.TrendHistory
import com.hyperscope.android.data.SpeedHistory
import com.hyperscope.android.data.TrafficInfo
import com.hyperscope.android.data.IoInfo
import com.hyperscope.android.data.EventItem
import com.hyperscope.android.data.NodeRefreshResult
import com.hyperscope.android.data.SettingsStore
import com.hyperscope.android.data.AppLog
import com.hyperscope.android.R
import kotlinx.coroutines.Job
import kotlinx.coroutines.async
import kotlinx.coroutines.awaitAll
import kotlinx.coroutines.coroutineScope
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.isActive
import kotlinx.coroutines.launch
import kotlinx.coroutines.runBlocking
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import kotlinx.serialization.builtins.ListSerializer
import kotlinx.serialization.builtins.MapSerializer
import kotlinx.serialization.builtins.serializer
import kotlinx.serialization.decodeFromString
import kotlinx.serialization.encodeToString
import java.text.SimpleDateFormat
import java.util.Date
import java.util.HashMap
import java.util.Locale
import kotlin.math.roundToInt

class AppViewModel(app: Application) : AndroidViewModel(app) {
    private val store = SettingsStore(app)
    // Shared panel identity (imported from a .hsxc config) enables command
    // signing as "panel"; empty falls back to the local device key.
    private val client = NodeClient(
        runBlocking { store.identityKey.first() }
    )

    private val _nodes = MutableStateFlow<List<NodeView>>(emptyList())
    val nodes: StateFlow<List<NodeView>> = _nodes.asStateFlow()

    private val _selected = MutableStateFlow<String?>(null)
    val selected: StateFlow<String?> = _selected.asStateFlow()

    private val _sort = MutableStateFlow("default")
    val sort: StateFlow<String> = _sort.asStateFlow()

    // Custom node-card order (list of node names) set by the "Sort" picker.
    private val _nodeOrder = MutableStateFlow<List<String>>(emptyList())
    val nodeOrder: StateFlow<List<String>> = _nodeOrder.asStateFlow()

    // Recent cpu/mem samples per node for the trend chart (max TREND_POINTS).
    private val _trends = MutableStateFlow<Map<String, TrendHistory>>(emptyMap())
    val trends: StateFlow<Map<String, TrendHistory>> = _trends.asStateFlow()

    // Recent network speed samples per node (rx/tx) for the speed card.
    private val _speedHistories = MutableStateFlow<Map<String, SpeedHistory>>(emptyMap())
    val speedHistories: StateFlow<Map<String, SpeedHistory>> = _speedHistories.asStateFlow()

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

    private val _events = MutableStateFlow<List<EventItem>>(emptyList())
    val events: StateFlow<List<EventItem>> = _events.asStateFlow()

    // Tracks previously-active alert keys per node to avoid re-firing each cycle.
    private val activeAlerts = HashMap<String, List<String>>()

    // Bumped on every poll so each NodeView.seq differs and StateFlow always
    // notifies collectors (data-class equals would suppress identical results).
    private var refreshSeq = 0L

    private var refreshJob: Job? = null

    companion object {
        private const val TREND_POINTS = 24
    }

    init {
        AppLog.d("App", "ViewModel init, loading nodes")
        viewModelScope.launch {
            val list = store.nodes.first()
            _nodes.value = list.map { NodeView(config = it) }
            _selected.value = list.firstOrNull()?.name
            _theme.value = store.theme.first()
            _lang.value = store.lang.first()
            _sort.value = store.sort.first()
            _nodeOrder.value = store.nodeOrder.first()
            // Restore persisted trend history (survives app restart).
            val savedTrends = store.trends.first()
            if (savedTrends.isNotBlank()) {
                runCatching {
                    val ser = MapSerializer(String.serializer(), TrendHistory.serializer())
                    HsxCodec.jsonExport.decodeFromString(ser, savedTrends)
                }.onSuccess { _trends.value = it }
            }
            // Restore the last successful node snapshot (offline cache): show
            // stale data immediately on cold start, then refresh in background.
            val savedSnapshot = store.snapshot.first()
            if (savedSnapshot.isNotBlank()) {
                runCatching {
                    HsxCodec.jsonExport.decodeFromString(
                        ListSerializer(NodeView.serializer()), savedSnapshot)
                }.onSuccess { cached ->
                    if (cached.isNotEmpty()) _nodes.value = cached
                }
            }
            val authed = store.loggedIn.first()
            _authState.value = if (authed) AuthState.Authed
            else if (store.hasAuth.first()) AuthState.Login
            else AuthState.Setup
            AppLog.d("App", "loaded ${list.size} nodes, auth=$authed")
            if (authed && list.isNotEmpty()) {
                // Cold start: no data yet — show "fetching" while the first poll
                // runs, never a "cannot connect" flash.
                _nodes.value = _nodes.value.map { it.copy(loading = true) }
                immediateRefresh = true // first tick polls immediately, no 5s wait
                startPolling()
            }
        }
    }

    // ---- authentication (local unlock) ----
    // Delegated to AuthController (keeps AppViewModel a thin state holder).

    internal fun authError(msg: String) {
        _authError.value = msg
        _authState.value = AuthState.Error
    }
    internal fun authStateAuthed() {
        _authState.value = AuthState.Authed
        _authError.value = null
    }
    internal fun clearAuth() {
        _authState.value = AuthState.Login
        _authError.value = null
        _nodes.value = emptyList()
        _selected.value = null
    }

    private val authController = AuthController(this, store)

    fun setup(userName: String, password: String, confirm: String) = authController.setup(userName, password, confirm)
    fun login(userName: String, password: String) = authController.login(userName, password)
    fun logout() = authController.logout()
    fun changeCredentials(oldUser: String, oldPass: String, newUser: String, newPass: String, onResult: (Boolean, String) -> Unit) =
        authController.changeCredentials(oldUser, oldPass, newUser, newPass, onResult)
    // Serializes refreshAll(): only one full poll runs at a time so an older
    // (cancelled/slower) refresh can never overwrite a newer result.
    private val refreshMutex = Mutex()

    // Rate-limit poll-failure logs: log the first failure, then at most once
    // per 30s while the problem persists, so a node that stays offline does
    // not spam the runtime log every 5s.
    private var lastPollErrorAt = 0L

    // Set by refreshNow() to request an immediate poll from the single worker;
    // consumed (reset) by the loop before each tick so a resume never spawns a
    // second concurrent refreshAll().
    @Volatile
    private var immediateRefresh = false

    private fun startPolling() {
        // A single long-lived poll loop; never cancel it to restart. If the loop
        // was killed by the OS while backgrounded, this recreates it. The body is
        // wrapped so a surprise exception in one refresh can never kill the loop
        // permanently (otherwise nodes would freeze on the last poll result and
        // only recover when the user taps a card to run a fresh refreshOne).
        if (refreshJob?.isActive == true) return
        refreshJob = viewModelScope.launch {
            while (isActive) {
                // Consume the "refresh now" signal: if set, skip the wait so
                // the next tick happens immediately.
                if (immediateRefresh) immediateRefresh = false
                else delay(3000)
                try {
                    refreshAll()
                } catch (e: kotlinx.coroutines.CancellationException) {
                    throw e // let normal cancellation propagate
                } catch (e: Exception) {
                    // swallow: keep the loop alive for the next tick, but log
                    // rate-limited so repeated failures stay debuggable.
                    val now = System.currentTimeMillis()
                    if (now - lastPollErrorAt > 30_000) {
                        lastPollErrorAt = now
                        AppLog.e("Poll", "poll failed: ${e.message ?: e.javaClass.simpleName}")
                    }
                }
            }
        }
    }

    /**
     * Called when the app returns to foreground. If auth/init hasn't finished
     * yet, the init block will start polling (which refreshes immediately);
     * otherwise refresh now with last-snapshot semantics.
     */
    fun onForeground() {
        if (authState.value == AuthState.Authed) refreshNow()
        // else: init is still loading — it marks nodes fetching and starts the
        // poll loop, which performs the first refresh itself.
    }

    /**
     * Ensures the poll loop exists and requests an immediate refresh. Called on
     * foreground resume and manual refresh. Uses a signal flag consumed by the
     * single poll loop instead of spawning a second refreshAll(), so a resume
     * never triggers two consecutive full refreshes.
     */
    fun refreshNow() {
        // The OS often reaps TCP connections while the app is backgrounded;
        // drop the pool so the immediate refresh uses fresh sockets instead of
        // timing out on dead ones (the classic 5s "failed to connect").
        client.resetConnections()
        // Mark only nodes that have NEVER returned data as "fetching" so the UI
        // shows a friendly loading state for them; nodes that already have data
        // keep showing their last snapshot while the fresh poll completes in the
        // background (no blank/flashing "fetching" when returning to the app).
        val views = _nodes.value
        if (views.isNotEmpty()) {
            _nodes.value = views.map { if (it.system == null) it.copy(loading = true, error = null) else it }
        }
        // Request an immediate refresh from the single poll worker.
        immediateRefresh = true
        if (refreshJob?.isActive != true) startPolling()
    }

    private suspend fun refreshAll() = refreshMutex.withLock {
        val views = _nodes.value
        refreshSeq++
        val seq = refreshSeq
        // Refresh every node in parallel so one slow/offline node never blocks
        // the others. Each node keeps its previous state on cancellation and
        // reports offline only for real failures.
        val updated = coroutineScope {
            views.map { view ->
                async {
                    try {
                        // Fetch the six endpoints for this node in parallel
                        // (they are independent). Serial fetches made a cold
                        // start take N x round-trip; parallel makes it the
                        // slowest single endpoint.
                        val sysD = async { client.system(view.config) }
                        val disksD = async { client.disks(view.config).disks }
                        val procsD = async { client.processes(view.config).processes }
                        val dockD = async { runCatching { client.docker(view.config) }.getOrDefault(DockerResponse()) }
                        val traffD = async { runCatching { client.traffic(view.config) }.getOrDefault(TrafficInfo()) }
                        val ioD = async { runCatching { client.io(view.config) }.getOrDefault(IoInfo()) }
                        val sys = sysD.await()
                        val disks = disksD.await()
                        val procs = procsD.await()
                        val dock = dockD.await()
                        val traff = traffD.await()
                        val ioinfo = ioD.await()
                        view.copy(system = sys, disks = disks, processes = procs, docker = dock,
                            traffic = traff, io = ioinfo, online = true, error = null, seq = seq, loading = false)
                    } catch (e: kotlinx.coroutines.CancellationException) {
                        // A cancelled request (e.g. leaving the screen) must not be shown
                        // as a node failure nor abort the whole batch: keep the node's
                        // previous state so the UI never flips to offline on cancel.
                        view.copy(seq = seq, loading = false)
                    } catch (e: Exception) {
                        // Returning from background / cold start: sockets are being
                        // re-established and the first poll often fails briefly.
                        // - nodes with data keep showing their last snapshot
                        // - nodes that never succeeded stay in "fetching" state
                        //   (not a scary "cannot connect") until a later poll succeeds
                        if (view.system == null) {
                            view.copy(online = false, error = null, seq = seq, loading = true)
                        } else {
                            view.copy(online = true, error = null, seq = seq, loading = false)
                        }
                    }
                }
            }.awaitAll()
        }
        // Filter out nodes deleted while this refresh was in flight: the
        // snapshot we read at the start may predate a batch delete, and writing
        // it back would resurrect the removed nodes.
        val currentNames = _nodes.value.map { it.config.name }.toSet()
        val filtered = updated.filter { it.config.name in currentNames }
        _nodes.value = filtered
        // log node state transitions (bounded: only log when something changes)
        val events = _events.value.toMutableList()
        val time = SimpleDateFormat("MM-dd HH:mm", Locale.getDefault()).format(Date())
        for (v in filtered) {
            val prev = views.firstOrNull { it.config.name == v.config.name }
            if (prev != null && prev.online != v.online) {
                AppLog.d("Poll", "node ${v.config.name} -> ${if (v.online) "online" else "offline (${v.error ?: "no error"})"}")
                // Record an event only for real transitions (never mark first-load).
                events.add(
                    EventItem(
                        time = time,
                        node = v.config.name,
                        kind = if (v.online) "up" else "down",
                        msg = if (v.online) "Node online" else "Node offline",
                    )
                )
            } else if (prev != null && v.online && v.error == null && prev.error != null) {
                AppLog.d("Poll", "node ${v.config.name} recovered")
            }
        }
        if (events.size > 500) events.subList(0, events.size - 500).clear()
        _events.value = events
        recordTrends(updated)
        recordSpeed(updated)
        // Offline cache: persist the last successful snapshot (every 6th refresh
        // ≈ 18s) so cold start / weak network can show stale data immediately.
        if (refreshSeq % 6 == 0L) {
            viewModelScope.launch {
                runCatching {
                    val ser = ListSerializer(NodeView.serializer())
                    HsxCodec.jsonExport.encodeToString(ser, updated)
                }.onSuccess { store.setSnapshot(it) }
            }
        }
        try {
            alertEngine.detect(updated)
        } catch (e: kotlinx.coroutines.CancellationException) {
            throw e
        } catch (_: Exception) {
            // notifications are best-effort; never let them break polling
        }
    }

    /** Append cpu/mem samples to the per-node trend history (kept bounded). */
    private fun recordTrends(views: List<NodeView>) {
        val cur = _trends.value.toMutableMap()
        for (view in views) {
            if (view.system == null) continue
            val t = cur[view.config.name] ?: TrendHistory()
            cur[view.config.name] = TrendHistory(
                cpu = (t.cpu + view.system.cpu).takeLast(TREND_POINTS),
                mem = (t.mem + view.system.mem_percent).takeLast(TREND_POINTS),
            )
        }
        _trends.value = cur
        // Persist every 6th refresh (≈18s) so history survives restarts without
        // hammering DataStore on every poll tick.
        if (refreshSeq % 6 == 0L) {
            viewModelScope.launch {
                runCatching {
                    val ser = MapSerializer(String.serializer(), TrendHistory.serializer())
                    HsxCodec.jsonExport.encodeToString(ser, cur)
                }.onSuccess { store.setTrends(it) }
            }
        }
    }

    /** Append rx/tx speed samples per node (bounded, like trends). */
    private fun recordSpeed(views: List<NodeView>) {
        val cur = _speedHistories.value.toMutableMap()
        for (view in views) {
            val t = view.traffic ?: continue
            val h = cur[view.config.name] ?: SpeedHistory()
            cur[view.config.name] = SpeedHistory(
                rx = (h.rx + t.speed_rx).takeLast(TREND_POINTS),
                tx = (h.tx + t.speed_tx).takeLast(TREND_POINTS),
            )
        }
        _speedHistories.value = cur
    }

    // ---- alert detection (delegated to AlertEngine) ----

    internal fun addNotification(time: String, node: String, msg: String) {
        _notifications.value = _notifications.value + NotifItem(time = time, node = node, kind = "alert", msg = msg)
    }
    internal fun trimNotificationsAndEvents() {
        val list = _notifications.value
        if (list.size > 100) _notifications.value = list.takeLast(100)
        val evs = _events.value
        if (evs.size > 500) _events.value = evs.takeLast(500)
    }
    internal fun application(): android.app.Application = getApplication()

    private val alertEngine = AlertEngine(this, client)

    fun clearNotifications() { _notifications.value = emptyList() }

    fun clearEvents() { _events.value = emptyList() }

    // ---- node list management ----
    // Delegated to NodeRepository (keeps AppViewModel a thin state holder).

    internal fun appendNode(view: NodeView) { _nodes.value = _nodes.value + view }
    internal fun removeNodeByName(name: String) {
        _nodes.value = _nodes.value.filter { it.config.name != name }
        if (_selected.value == name) _selected.value = _nodes.value.firstOrNull()?.config?.name
    }
    internal fun removeNodesByName(names: Set<String>) {
        _nodes.value = _nodes.value.filter { it.config.name !in names }
        if (_selected.value in names) _selected.value = _nodes.value.firstOrNull()?.config?.name
    }
    internal fun addEvent(time: String, node: String, kind: String, msg: String) {
        _events.value = _events.value + EventItem(time = time, node = node, kind = kind, msg = msg)
    }

    private val nodeRepository = NodeRepository(this, store)

    fun batchAddNodes(text: String) = nodeRepository.batchAddNodes(text)
    fun removeNode(name: String) = nodeRepository.removeNode(name)
    fun removeNodes(names: Set<String>) = nodeRepository.removeNodes(names)

    /**
     * Exports all nodes as an encrypted .hsxc ByteArray. Returns null when the
     * passphrase is blank or there are no nodes (caller shows the message).
     */
    fun addNode(
        name: String, addr: String, port: Int, key: String,
        group: String = "", webhook: String = "",
        alertCpu: Double? = null, alertMem: Double? = null,
        alertDisk: Double? = null, alertTemp: Double? = null,
    ) {
        viewModelScope.launch {
            val cfg = buildNodeConfig(name, addr, port, key, group, webhook, alertCpu, alertMem, alertDisk, alertTemp)
            AppLog.d("Node", "add node ${cfg.name} (${cfg.addr}:${cfg.port} tls=${cfg.tls} push=${cfg.push})")
            val list = _nodes.value.map { it.config } + cfg
            store.saveNodes(list)
            _nodes.value = _nodes.value + NodeView(config = cfg)
            if (_selected.value == null) _selected.value = cfg.name
            refreshNow()
        }
    }

    /**
     * Updates node config locally (rename / group / alert thresholds). All
     * editable fields live in the on-device node list, so no panel API needed.
     */
    fun updateNodeConfig(
        oldName: String,
        newName: String? = null,
        group: String? = null,
        webhook: String? = null,
        alertCpu: Double? = null, alertMem: Double? = null,
        alertDisk: Double? = null, alertTemp: Double? = null,
    ) {
        viewModelScope.launch {
            val updated = _nodes.value.map { v ->
                if (v.config.name != oldName) v
                else {
                    val c = v.config
                    v.copy(config = c.copy(
                        name = newName ?: c.name,
                        group = group ?: c.group,
                        webhook = webhook ?: c.webhook,
                        alertCpu = alertCpu ?: c.alertCpu,
                        alertMem = alertMem ?: c.alertMem,
                        alertDisk = alertDisk ?: c.alertDisk,
                        alertTemp = alertTemp ?: c.alertTemp,
                    ))
                }
            }
            _nodes.value = updated
            store.saveNodes(updated.map { it.config })
            if (_selected.value == oldName && newName != null) _selected.value = newName
            refreshNow()
        }
    }


    /**
     * Exports all nodes as an encrypted .hsxc ByteArray. Returns null when the
     * passphrase is blank or there are no nodes (caller shows the message).
     */
    fun exportNodes(passphrase: String, onResult: (ByteArray?, String) -> Unit) {
        viewModelScope.launch {
            val list = _nodes.value.map { it.config }
            if (list.isEmpty()) {
                onResult(null, getApplication<Application>().getString(R.string.export_empty))
                return@launch
            }
            if (passphrase.isBlank()) {
                onResult(null, getApplication<Application>().getString(R.string.export_pass_needed))
                return@launch
            }
            val encrypted = exportHsx(list, passphrase)
            onResult(encrypted, "")
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
                    onResult(getApplication<Application>().getString(R.string.import_empty))
                    return@launch
                }
                val cfgs = new.map { n ->
                    // key may be "plain" or "key|SHA256:FINGERPRINT" (TLS mode).
                    // Split the fingerprint out and pin it when present.
                    val (keyPart, fpPart) = splitKeyAndFingerprint(n.key)
                    NodeConfig(
                        name = n.name,
                        addr = n.addr,
                        port = n.port.takeIf { p -> p > 0 } ?: 8686,
                        key = keyPart,
                        tls = n.tls || fpPart.isNotBlank(),
                        certFp = fpPart,
                    )
                }
                AppLog.d("Node", "import ${cfgs.size} nodes (total ${_nodes.value.size} -> ${_nodes.value.size + cfgs.size})")
                val full = _nodes.value.map { it.config } + cfgs
                store.saveNodes(full)
                _nodes.value = _nodes.value + cfgs.map { NodeView(config = it) }
                // Save the embedded panel identity (if any) so commands are
                // signed as "panel" and accepted by nodes that trust it.
                if (payload.identity_key.isNotBlank()) store.setIdentityKey(payload.identity_key)
                if (_selected.value == null) _selected.value = cfgs.first().name
                refreshNow()
                onResult(getApplication<Application>().getString(R.string.import_ok, cfgs.size))
            } catch (e: IllegalArgumentException) {
                onResult(e.message ?: getApplication<Application>().getString(R.string.import_failed))
            }
        }
    }


    fun selectNode(name: String) {
        _selected.value = name
        // Refresh the tapped node immediately so the user sees live data instead
        // of waiting up to the next 5s poll.
        viewModelScope.launch { refreshOne(name) }
    }

    private suspend fun refreshOne(name: String) = refreshMutex.withLock {
        val view = _nodes.value.firstOrNull { it.config.name == name } ?: return@withLock
        refreshSeq++
        val seq = refreshSeq
        val updatedView = try {
            // Parallel fetch like refreshAll: one node's six endpoints are
            // independent, so await them concurrently.
            val (sys, disks, procs, dock) = coroutineScope {
                val sysD = async { client.system(view.config) }
                val disksD = async { client.disks(view.config).disks }
                val procsD = async { client.processes(view.config).processes }
                val dockD = async { runCatching { client.docker(view.config) }.getOrDefault(DockerResponse()) }
                val s = sysD.await()
                val d = disksD.await()
                val p = procsD.await()
                val c = dockD.await()
                // Return as a small holder instead of nested Pairs.
                NodeRefreshResult(s, d, p, c)
            }
            view.copy(system = sys, disks = disks, processes = procs, docker = dock, online = true, error = null, seq = seq, loading = false)
        } catch (e: kotlinx.coroutines.CancellationException) {
            view.copy(seq = seq, loading = false)
        } catch (e: Exception) {
            view.copy(online = false, error = e.message, seq = seq, loading = false)
        }
        _nodes.value = _nodes.value.map { if (it.config.name == name) updatedView else it }
    }

    fun setSort(s: String) {
        _sort.value = s
        // Picking a built-in sort clears any custom order.
        if (s != "custom") _nodeOrder.value = emptyList()
        viewModelScope.launch {
            store.setSort(s)
            if (s != "custom") store.setNodeOrder(emptyList())
        }
    }

    /** Set the custom card order (list of node names). */
    fun setNodeOrder(order: List<String>) {
        _nodeOrder.value = order
        _sort.value = "custom"
        viewModelScope.launch { store.setNodeOrder(order) }
    }

    // ---- machine control (control tab) ----
    // Delegated to NodeController (keeps AppViewModel a thin state holder).

    /** String resource helper (used by NodeController callbacks). */
    internal fun appString(resId: Int, vararg args: Any): String =
        getApplication<Application>().getString(resId, *args)

    /** Public refresh-one used by NodeController after docker actions. */
    internal fun refreshOnePublic(name: String) {
        viewModelScope.launch { refreshOne(name) }
    }

    private val nodeController = NodeController(this, client)

    fun rebootNode(name: String, onResult: (String) -> Unit) = nodeController.rebootNode(name, onResult)
    fun shutdownNode(name: String, onResult: (String) -> Unit) = nodeController.shutdownNode(name, onResult)
    fun killProcessOnNode(name: String, pid: Int, onResult: (String) -> Unit) = nodeController.killProcessOnNode(name, pid, onResult)
    fun dockerActionOnNode(name: String, container: String, action: String, onResult: (String) -> Unit) = nodeController.dockerActionOnNode(name, container, action, onResult)

    /** Ping a node: real ICMP probe via the bundled native ping binary. */
    fun pingNode(name: String, onResult: (Pair<Boolean, String>) -> Unit) {
        viewModelScope.launch {
            val view = _nodes.value.firstOrNull { it.config.name == name }
            if (view == null) { onResult(false to "node not found"); return@launch }
            onResult(NativePing.ping(getApplication<Application>(), view.config.addr))
        }
    }

    fun setTheme(v: String) = updateSetting(v, { _theme.value = it }, { store.setTheme(v) })
    fun setLang(v: String) {
        _lang.value = v
        // Write synchronously: LanguagePicker calls Activity.recreate() right
        // after this, and attachBaseContext reads the persisted value to build
        // the new locale. An async write would race recreate() and the UI would
        // come back in the old language.
        runBlocking { store.setLang(v) }
    }
    /** Sets a setting state and persists it via the store in a coroutine. */
    private fun <T> updateSetting(v: T, apply: (T) -> Unit, persist: suspend () -> Unit) {
        apply(v)
        viewModelScope.launch { persist() }
    }
}

enum class AuthState { Loading, Setup, Login, Authed, Error }