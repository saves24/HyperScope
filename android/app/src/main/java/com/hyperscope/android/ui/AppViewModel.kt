package com.hyperscope.android.ui

import android.app.Application
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import com.hyperscope.android.data.NodeClient
import com.hyperscope.android.data.NodeConfig
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

    private val _authState = MutableStateFlow(AuthState.Loading)
    val authState: StateFlow<AuthState> = _authState.asStateFlow()

    private val _authError = MutableStateFlow<String?>(null)
    val authError: StateFlow<String?> = _authError.asStateFlow()

    private var refreshJob: Job? = null

    init {
        viewModelScope.launch {
            val list = store.nodes.first()
            _nodes.value = list.map { NodeView(config = it) }
            _selected.value = list.firstOrNull()?.name
            _theme.value = store.theme.first()
            _authState.value = if (store.hasAuth.first()) AuthState.Login else AuthState.Setup
            if (list.isNotEmpty()) startPolling()
        }
    }

    fun setup(userName: String, password: String, confirm: String) {
        viewModelScope.launch {
            if (userName.isBlank() || password.isBlank()) {
                _authError.value = "用户名和密码不能为空"
                _authState.value = AuthState.Error
                return@launch
            }
            if (password != confirm) {
                _authError.value = "两次输入的密码不一致"
                _authState.value = AuthState.Error
                return@launch
            }
            store.setupCredentials(userName, password)
            _authState.value = AuthState.Authed
        }
    }

    fun login(userName: String, password: String) {
        viewModelScope.launch {
            val ok = store.verifyLogin(userName, password)
            if (ok) {
                _authState.value = AuthState.Authed
                _authError.value = null
            } else {
                _authError.value = "用户名或密码错误"
                _authState.value = AuthState.Error
            }
        }
    }

    fun logout() {
        viewModelScope.launch { store.setupCredentials("", "") }
        _authState.value = AuthState.Setup
        _authError.value = null
        _nodes.value = emptyList()
        _selected.value = null
    }

    private fun startPolling() {
        refreshJob?.cancel()
        refreshJob = viewModelScope.launch {
            while (isActive) {
                refreshAll()
                delay(5000)
            }
        }
    }

    private suspend fun refreshAll() {
        val views = _nodes.value
        val updated = views.map { view ->
            try {
                val sys = client.system(view.config)
                val disks = client.disks(view.config).disks
                val procs = client.processes(view.config).processes
                view.copy(system = sys, disks = disks, processes = procs, online = true, error = null)
            } catch (e: Exception) {
                view.copy(online = false, error = e.message)
            }
        }
        _nodes.value = updated
    }

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

    fun manualRefresh() { viewModelScope.launch { refreshAll() } }
}

enum class AuthState { Loading, Setup, Login, Authed, Error }