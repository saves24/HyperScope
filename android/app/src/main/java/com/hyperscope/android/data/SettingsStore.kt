package com.hyperscope.android.data

import android.content.Context
import androidx.datastore.preferences.core.edit
import androidx.datastore.preferences.core.stringPreferencesKey
import androidx.datastore.preferences.preferencesDataStore
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.map

private val Context.dataStore by preferencesDataStore(name = "settings")

/** Persisted user settings (server URL, auth token, theme). */
class SettingsStore(private val context: Context) {
    private val keyBase = stringPreferencesKey("base_url")
    private val keyToken = stringPreferencesKey("token")
    private val keyUser = stringPreferencesKey("user")
    private val keyTheme = stringPreferencesKey("theme")

    val baseUrl: Flow<String> = context.dataStore.data.map { it[keyBase] ?: "http://192.168.1.7:8088" }
    val token: Flow<String> = context.dataStore.data.map { it[keyToken] ?: "" }
    val user: Flow<String> = context.dataStore.data.map { it[keyUser] ?: "" }
    val theme: Flow<String> = context.dataStore.data.map { it[keyTheme] ?: "auto" }

    suspend fun setBaseUrl(v: String) = context.dataStore.edit { it[keyBase] = v }
    suspend fun setToken(v: String) = context.dataStore.edit { it[keyToken] = v }
    suspend fun setUser(v: String) = context.dataStore.edit { it[keyUser] = v }
    suspend fun setTheme(v: String) = context.dataStore.edit { it[keyTheme] = v }
    suspend fun logout() = context.dataStore.edit {
        it.remove(keyToken)
        it.remove(keyUser)
    }
}
