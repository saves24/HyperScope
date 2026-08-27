package com.hyperscope.android.data

import android.content.Context
import androidx.datastore.preferences.core.edit
import androidx.datastore.preferences.core.stringPreferencesKey
import androidx.datastore.preferences.preferencesDataStore
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.map
import kotlinx.serialization.builtins.ListSerializer
import kotlinx.serialization.json.Json

private val Context.dataStore by preferencesDataStore(name = "settings")

/** Local node list + app prefs. The phone is the panel, so nodes live here. */
class SettingsStore(private val context: Context) {
    private val keyNodes = stringPreferencesKey("nodes")
    private val keyTheme = stringPreferencesKey("theme")
    private val json = Json { ignoreUnknownKeys = true }
    private val nodesSerializer = ListSerializer(NodeConfig.serializer())

    val nodes: Flow<List<NodeConfig>> = context.dataStore.data.map {
        val raw = it[keyNodes] ?: "[]"
        runCatching { json.decodeFromString(nodesSerializer, raw) }.getOrDefault(emptyList())
    }
    val theme: Flow<String> = context.dataStore.data.map { it[keyTheme] ?: "auto" }

    suspend fun saveNodes(list: List<NodeConfig>) {
        context.dataStore.edit { it[keyNodes] = json.encodeToString(nodesSerializer, list) }
    }
    suspend fun setTheme(v: String) = context.dataStore.edit { it[keyTheme] = v }
}
