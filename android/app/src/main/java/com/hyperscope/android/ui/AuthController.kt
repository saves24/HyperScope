package com.hyperscope.android.ui

import androidx.lifecycle.viewModelScope
import com.hyperscope.android.data.AppLog
import com.hyperscope.android.data.SettingsStore
import com.hyperscope.android.R
import kotlinx.coroutines.launch

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
        vm.viewModelScope.launch {
            if (userName.isBlank() || password.isBlank()) {
                vm.authError(vm.appString(R.string.auth_err_empty))
                return@launch
            }
            if (password != confirm) {
                vm.authError(vm.appString(R.string.auth_err_mismatch))
                return@launch
            }
            store.setupCredentials(userName, password)
            store.setLoggedIn(true)
            vm.authStateAuthed()
        }
    }

    fun login(userName: String, password: String) {
        vm.viewModelScope.launch {
            val ok = store.verifyLogin(userName, password)
            if (ok) {
                store.setLoggedIn(true)
                vm.authStateAuthed()
                AppLog.d("Auth", "login ok")
            } else {
                AppLog.e("Auth", "login failed (wrong credentials)")
                vm.authError(vm.appString(R.string.auth_err_invalid))
            }
        }
    }

    fun logout() {
        vm.viewModelScope.launch { store.setLoggedIn(false) }
        vm.clearAuth()
    }

    /** Change local account (username/password). Returns a result message for the UI. */
    fun changeCredentials(oldUser: String, oldPass: String, newUser: String, newPass: String, onResult: (Boolean, String) -> Unit) {
        vm.viewModelScope.launch {
            if (oldUser.isBlank() || oldPass.isBlank() || newUser.isBlank() || newPass.isBlank()) {
                onResult(false, vm.appString(R.string.auth_err_empty))
                return@launch
            }
            val ok = store.changeCredentials(oldUser, oldPass, newUser, newPass)
            if (ok) {
                AppLog.d("Auth", "credentials changed")
                onResult(true, vm.appString(R.string.change_ok))
            } else {
                AppLog.e("Auth", "change credentials failed (wrong old)")
                onResult(false, vm.appString(R.string.change_wrong_old))
            }
        }
    }
}
