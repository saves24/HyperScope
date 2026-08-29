package com.hyperscope.android.data

import android.content.Context
import androidx.datastore.preferences.core.booleanPreferencesKey
import androidx.datastore.preferences.core.edit
import androidx.datastore.preferences.core.intPreferencesKey
import androidx.datastore.preferences.core.stringPreferencesKey
import androidx.datastore.preferences.preferencesDataStore
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.flow.map
import kotlinx.serialization.builtins.ListSerializer
import kotlinx.serialization.json.Json

private val Context.dataStore by preferencesDataStore(name = "settings")

/** Local node list + app prefs. The phone is the panel, so nodes live here. */
class SettingsStore(private val context: Context) {
    private val keyNodes = stringPreferencesKey("nodes")
    private val keyTheme = stringPreferencesKey("theme")
    private val keyUser = stringPreferencesKey("user")
    private val keyPassHash = stringPreferencesKey("pass_hash")
    private val keyLoggedIn = booleanPreferencesKey("logged_in")
    private val keyLang = stringPreferencesKey("lang")
    private val keySort = stringPreferencesKey("sort")
    private val keyNodeOrder = stringPreferencesKey("node_order")
    private val keyTrends = stringPreferencesKey("trends")
    private val keySnapshot = stringPreferencesKey("snapshot")
    private val keyIdentityKey = stringPreferencesKey("identity_key")
    private val json = Json { ignoreUnknownKeys = true }
    private val nodesSerializer = ListSerializer(NodeConfig.serializer())

    val nodes: Flow<List<NodeConfig>> = context.dataStore.data.map {
        val raw = it[keyNodes] ?: "[]"
        runCatching { json.decodeFromString(nodesSerializer, raw) }.getOrDefault(emptyList())
    }
    val theme: Flow<String> = context.dataStore.data.map { it[keyTheme] ?: "auto" }
    val user: Flow<String> = context.dataStore.data.map { it[keyUser] ?: "" }
    val lang: Flow<String> = context.dataStore.data.map { it[keyLang] ?: "system" }
    val sort: Flow<String> = context.dataStore.data.map { it[keySort] ?: "default" }
    val nodeOrder: Flow<List<String>> = context.dataStore.data.map { raw ->
        raw[keyNodeOrder]?.takeIf { s -> s.isNotBlank() }
            ?.split("\u0001") ?: emptyList()
    }
    val trends: Flow<String> = context.dataStore.data.map { prefs -> prefs[keyTrends] ?: "" }
    /** Last successful node snapshot (offline cache: show stale data when weak/no network). */
    val snapshot: Flow<String> = context.dataStore.data.map { prefs -> prefs[keySnapshot] ?: "" }
    /** Shared panel identity private key imported from a .hsxc config (base64). */
    val identityKey: Flow<String> = context.dataStore.data.map { prefs -> prefs[keyIdentityKey] ?: "" }

    /** true once a username+password has been configured. */
    val hasAuth: Flow<Boolean> = context.dataStore.data.map { !it[keyUser].isNullOrEmpty() }

    /** true if the user has an active (non-logged-out) session. */
    val loggedIn: Flow<Boolean> = context.dataStore.data.map { it[keyLoggedIn] == true }

    /**
     * Auth state lives in SharedPreferences (synchronous, atomic), not
     * DataStore. DataStore reads/writes can stall after an interrupted write
     * (the app gets force-stopped mid-edit), which froze startup/login. Auth is
     * the one thing that must never block the main thread.
     */
    private val authPrefs get() = context.getSharedPreferences("hs_auth", Context.MODE_PRIVATE)

    /** true once a username+password has been configured. */
    fun hasAuthNow(): Boolean = !authPrefs.getString("user", "").isNullOrEmpty()
    fun loggedInNow(): Boolean = authPrefs.getBoolean("logged_in", false)

    /** Create or verify local login credentials (hash stored, plaintext never kept). */
    fun setupCredentialsSync(userName: String, password: String) {
        authPrefs.edit()
            .putString("user", userName)
            .putString("pass_hash", sha256(password))
            .putBoolean("logged_in", false)
            .commit()
    }
    fun setLoggedInSync(v: Boolean) {
        authPrefs.edit().putBoolean("logged_in", v).commit()
    }
    fun verifyLoginSync(userName: String, password: String): Boolean {
        val u = authPrefs.getString("user", null) ?: return false
        val h = authPrefs.getString("pass_hash", null) ?: return false
        return u == userName && h == sha256(password)
    }

    suspend fun saveNodes(list: List<NodeConfig>) {
        context.dataStore.edit { it[keyNodes] = json.encodeToString(nodesSerializer, list) }
    }
    suspend fun setIdentityKey(key: String) = context.dataStore.edit { it[keyIdentityKey] = key }
    suspend fun setTheme(v: String) = context.dataStore.edit { it[keyTheme] = v }
    suspend fun setLang(v: String) = context.dataStore.edit { it[keyLang] = v }
    suspend fun setSort(v: String) = context.dataStore.edit { it[keySort] = v }
    suspend fun setNodeOrder(order: List<String>) =
        context.dataStore.edit { it[keyNodeOrder] = order.joinToString("\u0001") }
    suspend fun setTrends(v: String) = context.dataStore.edit { it[keyTrends] = v }
    suspend fun setSnapshot(v: String) = context.dataStore.edit { it[keySnapshot] = v }

    /** Create or verify local login credentials (hash stored, plaintext never kept). */
    suspend fun setupCredentials(userName: String, password: String) {
        context.dataStore.edit {
            it[keyUser] = userName
            it[keyPassHash] = sha256(password)
            it[keyLoggedIn] = false
        }
    }
    suspend fun setLoggedIn(v: Boolean) = context.dataStore.edit { it[keyLoggedIn] = v }
    suspend fun verifyLogin(userName: String, password: String): Boolean {
        val stored = context.dataStore.data.first()
        val u = stored[keyUser] ?: return false
        val h = stored[keyPassHash] ?: return false
        return u == userName && h == sha256(password)
    }

    /**
     * Change the local account (username and/or password). Requires the current
     * credentials to match before replacing them. Returns false on wrong old
     * credentials.
     */
    suspend fun changeCredentials(oldUser: String, oldPass: String, newUser: String, newPass: String): Boolean {
        val u = authPrefs.getString("user", null) ?: return false
        val h = authPrefs.getString("pass_hash", null) ?: return false
        if (u != oldUser || h != sha256(oldPass)) return false
        authPrefs.edit()
            .putString("user", newUser)
            .putString("pass_hash", sha256(newPass))
            .commit()
        return true
    }

    private fun sha256(s: String): String {
        val md = java.security.MessageDigest.getInstance("SHA-256")
        return md.digest(s.toByteArray()).joinToString("") { "%02x".format(it) }
    }
}
