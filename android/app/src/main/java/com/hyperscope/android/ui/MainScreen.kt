package com.hyperscope.android.ui

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Dashboard
import androidx.compose.material.icons.filled.Info
import androidx.compose.material.icons.filled.List
import androidx.compose.material.icons.filled.Settings
import androidx.compose.material3.Icon
import androidx.compose.material3.NavigationBar
import androidx.compose.material3.NavigationBarItem
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.vector.ImageVector

// Nodes are selected from the top bar of the Dashboard, not the bottom nav.
enum class Tab(val label: String, val icon: ImageVector) {
    Dashboard("概览", Icons.Filled.Dashboard),
    History("历史", Icons.Filled.List),
    Logs("日志", Icons.Filled.Info),
    Settings("设置", Icons.Filled.Settings),
}

@Composable
fun MainScreen(vm: AppViewModel) {
    var tab by remember { mutableStateOf(Tab.Dashboard) }

    Scaffold(
        bottomBar = {
            NavigationBar {
                Tab.entries.forEach { t ->
                    NavigationBarItem(
                        selected = tab == t,
                        onClick = { tab = t },
                        icon = { Icon(t.icon, contentDescription = t.label) },
                        label = { Text(t.label) },
                    )
                }
            }
        },
    ) { pad ->
        Box(Modifier.fillMaxSize().padding(pad)) {
            when (tab) {
                Tab.Dashboard -> DashboardScreen(vm)
                Tab.History -> HistoryScreen(vm)
                Tab.Logs -> LogsScreen(vm)
                Tab.Settings -> SettingsScreen(vm)
            }
        }
    }
}
