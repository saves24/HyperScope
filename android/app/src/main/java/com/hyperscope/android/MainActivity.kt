package com.hyperscope.android

import android.content.Context
import android.os.Build
import android.os.Bundle
import android.util.Log
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.material3.Surface
import androidx.compose.runtime.getValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.LifecycleEventObserver
import androidx.lifecycle.ViewModelProvider
import androidx.lifecycle.viewmodel.compose.viewModel
import androidx.compose.runtime.collectAsState
import com.hyperscope.android.data.LocaleManager
import com.hyperscope.android.data.Notifier
import com.hyperscope.android.data.SettingsStore
import com.hyperscope.android.data.AppLog
import com.hyperscope.android.ui.AppViewModel
import com.hyperscope.android.ui.AuthScreen
import com.hyperscope.android.ui.AuthState
import com.hyperscope.android.ui.HyperScopeTheme
import com.hyperscope.android.ui.MainScreen
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.runBlocking

class MainActivity : ComponentActivity() {

    // Applies the stored language before any UI is inflated, and installs a
    // crash handler that writes to <files>/crash.log for offline diagnosis.
    override fun attachBaseContext(newBase: Context) {
        Thread.setDefaultUncaughtExceptionHandler { t, e ->
            try {
                val f = java.io.File(newBase.getExternalFilesDir(null) ?: newBase.filesDir, "crash.log")
                f.appendText("${System.currentTimeMillis()} $t\n${Log.getStackTraceString(e)}\n---\n")
            } catch (_: Exception) {}
        }
        val lang = runBlocking { SettingsStore(newBase).lang.first() }
        super.attachBaseContext(LocaleManager.applyLocale(newBase, lang))
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()
        AppLog.init(this)
        Notifier.ensureChannel(this)
        // Android 13+ requires POST_NOTIFICATIONS; ask once on first launch.
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
            if (checkSelfPermission(android.Manifest.permission.POST_NOTIFICATIONS)
                != android.content.pm.PackageManager.PERMISSION_GRANTED
            ) {
                requestPermissions(arrayOf(android.Manifest.permission.POST_NOTIFICATIONS), 1)
            }
        }
        AppLog.d("App", "MainActivity onCreate")
        // Refresh immediately when returning to foreground (background suspension
        // can drop poller connections; this re-arms the poll loop and re-fetches).
        lifecycle.addObserver(LifecycleEventObserver { _, event ->
            if (event == Lifecycle.Event.ON_RESUME) {
                val vm = ViewModelProvider(this)[AppViewModel::class.java]
                vm.onForeground()
            }
        })
        setContent {
            val vm: AppViewModel = viewModel()
            val auth by vm.authState.collectAsState()
            val theme by vm.theme.collectAsState()
            val darkTheme = when (theme) {
                "dark" -> true
                "light" -> false
                else -> isSystemInDarkTheme()
            }
            HyperScopeTheme(
                darkTheme = darkTheme,
                dynamicColor = theme == "auto",
            ) {
                Surface(Modifier.fillMaxSize(), color = Color.Transparent) {
                    when (auth) {
                        AuthState.Setup -> AuthScreen(vm, setup = true)
                        AuthState.Login, AuthState.Error -> AuthScreen(vm, setup = false)
                        else -> MainScreen(vm)
                    }
                }
            }
        }
    }
}
