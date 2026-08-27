package com.hyperscope.android

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.material3.Surface
import androidx.compose.runtime.getValue
import androidx.compose.ui.Modifier
import androidx.lifecycle.viewmodel.compose.viewModel
import androidx.compose.runtime.collectAsState
import com.hyperscope.android.ui.AppViewModel
import com.hyperscope.android.ui.HyperScopeTheme
import com.hyperscope.android.ui.LoginScreen
import com.hyperscope.android.ui.MainScreen

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()
        setContent {
            val vm: AppViewModel = viewModel()
            val ui by vm.ui.collectAsState()
            HyperScopeTheme {
                Surface(Modifier.fillMaxSize()) {
                    if (ui.loggedIn) MainScreen(vm) else LoginScreen(vm)
                }
            }
        }
    }
}
