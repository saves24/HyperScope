package com.hyperscope.android.ui

import android.app.Application
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import com.hyperscope.android.data.ApiClient
import com.hyperscope.android.data.EventsResponse
import com.hyperscope.android.data.HistoryResponse
import com.hyperscope.android.data.NodesResponse
import com.hyperscope.android.data.SettingsStore
import com.hyperscope.android.data.SystemResponse
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.launch

data class UiState(
    val baseUrl: String = "http://192.168.1.7:8088",
    val token: String = "",
    val user: String = "",
    val theme: String = "auto",
    val loggedIn: Boolean = false,
    val loading: Boolean = false,
    val error: String? = null,
)

class AppViewModel(app: Application) : AndroidViewModel(app) {
    private val store = SettingsStore(app)
    private val api = ApiClient()

    private val _ui = MutableStateFlow(UiState())
    val ui: StateFlow<UiState> = _ui.asStateFlow()

    private val _nodes = MutableStateFlow(NodesResponse())
    val nodes: StateFlow<NodesResponse> = _nodes.asStateFlow()

    private val _system = MutableStateFlow(SystemResponse())
    val system: StateFlow<SystemResponse> = _system.asStateFlow()

    private val _history = MutableStateFlow(HistoryResponse())
    val history: StateFlow<HistoryResponse> = _history.asStateFlow()

    private val _events = MutableStateFlow(EventsResponse())
    val events: StateFlow<EventsResponse> = _events.asStateFlow()

    init {
        viewModelScope.launch {
            val s = _ui.value.copy(
                baseUrl = store.baseUrl.first(),
                token = store.token.first(),
                user = store.user.first(),
                theme = store.theme.first(),
                loggedIn = store.token.first().isNotEmpty(),
            )
            _ui.value = s
        }
    }

    fun setBaseUrl(v: String) { _ui.value = _ui.value.copy(baseUrl = v) }
    fun setTheme(v: String) {
        _ui.value = _ui.value.copy(theme = v)
        viewModelScope.launch { store.setTheme(v) }
    }

    fun login(user: String, pass: String) {
        viewModelScope.launch {
            _ui.value = _ui.value.copy(loading = true, error = null)
            try {
                val token = api.login(_ui.value.baseUrl, user, pass)
                store.setToken(token); store.setUser(user)
                _ui.value = _ui.value.copy(token = token, user = user, loggedIn = true, loading = false)
                loadNodes()
            } catch (e: Exception) {
                _ui.value = _ui.value.copy(loading = false, error = e.message ?: "Login failed")
            }
        }
    }

    fun logout() {
        viewModelScope.launch { store.logout() }
        _ui.value = _ui.value.copy(token = "", user = "", loggedIn = false)
    }

    fun loadNodes() {
        viewModelScope.launch {
            try {
                _nodes.value = api.fetchNodes(_ui.value.baseUrl, _ui.value.token)
                if (_nodes.value.nodes.isNotEmpty()) {
                    loadSystem(_nodes.value.nodes.first().id)
                }
            } catch (e: Exception) {
                _ui.value = _ui.value.copy(error = e.message)
            }
        }
    }

    fun loadSystem(nodeId: String) {
        viewModelScope.launch {
            try { _system.value = api.fetchSystem(_ui.value.baseUrl, _ui.value.token, nodeId) }
            catch (_: Exception) {}
        }
    }

    fun loadHistory(nodeId: String, metric: String = "cpu", range: String = "1h") {
        viewModelScope.launch {
            try { _history.value = api.fetchHistory(_ui.value.baseUrl, _ui.value.token, nodeId, metric, range) }
            catch (_: Exception) {}
        }
    }

    fun loadEvents() {
        viewModelScope.launch {
            try { _events.value = api.fetchEvents(_ui.value.baseUrl, _ui.value.token) }
            catch (_: Exception) {}
        }
    }
}
