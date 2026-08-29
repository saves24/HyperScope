package com.hyperscope.android.ui

import com.hyperscope.android.data.AppLog
import com.hyperscope.android.data.SettingsStore
import com.hyperscope.android.R

/**
 * Local authentication: setup / login / logout / change-credentials.
 * Holds a reference to the owning ViewModel to update auth state and node
 * clearing. Split from AppViewModel so it stays a thin state holder.
 */
internal class AuthController(
    private val vm: AppViewModel,
    private val store: SettingsStore,
) {
    fun setup(userName: String, password: String, confirm: String) {
        try {
            if (userName.isBlank() || password.isBlank()) {
                vm.authError(vm.appString(R.string.auth_err_empty))
                return
            }
            if (password != confirm) {
                vm.authError(vm.appString(R.string.auth_err_mismatch))
                return
            }
            // Synchronous SharedPreferences write: never blocks, never stalls.
            store.setupCredentialsSync(userName, password)
            store.setLoggedInSync(true)
            AppLog.d("Auth", "setup ok: $userName")
            vm.authStateAuthed()
        } catch (e: Exception) {
            AppLog.e("Auth", "setup failed: ${e.message}")
            vm.authError(vm.appString(R.string.auth_err_empty))
        }
    }

    fun login(userName: String, password: String) {
        try {
            val ok = store.verifyLoginSync(userName, password)
            if (ok) {
                store.setLoggedInSync(true)
                vm.authStateAuthed()
                AppLog.d("Auth", "login ok")
            } else {
                AppLog.e("Auth", "login failed (wrong credentials)")
                vm.authError(vm.appString(R.string.auth_err_invalid))
            }
        } catch (e: Exception) {
            AppLog.e("Auth", "login error: ${e.message}")
            vm.authError(vm.appString(R.string.auth_err_invalid))
        }
    }

    fun logout() {
        store.setLoggedInSync(false)
        vm.clearAuth()
    }

    /** Change local account (username/password). Returns a result message for the UI. */
    fun changeCredentials(oldUser: String, oldPass: String, newUser: String, newPass: String, onResult: (Boolean, String) -> Unit) {
        try {
            if (oldUser.isBlank() || oldPass.isBlank() || newUser.isBlank() || newPass.isBlank()) {
                onResult(false, vm.appString(R.string.auth_err_empty))
                return
            }
            val ok = kotlinx.coroutines.runBlocking {
                store.changeCredentials(oldUser, oldPass, newUser, newPass)
            }
            if (ok) {
                AppLog.d("Auth", "credentials changed")
                onResult(true, vm.appString(R.string.change_ok))
            } else {
                AppLog.e("Auth", "change credentials failed (wrong old)")
                onResult(false, vm.appString(R.string.change_wrong_old))
            }
        } catch (e: Exception) {
            AppLog.e("Auth", "change error: ${e.message}")
            onResult(false, vm.appString(R.string.change_wrong_old))
        }
    }
}
