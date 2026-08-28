package com.hyperscope.android.ui

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Home
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
import androidx.compose.ui.res.stringResource
import com.hyperscope.android.R

// Nodes are selected from the top bar of the Dashboard, not the bottom nav.
enum class Tab(val labelRes: Int, val icon: ImageVector) {
    Dashboard(R.string.tab_dashboard, Icons.Filled.Home),
    History(R.string.tab_history, Icons.Filled.List),
    Logs(R.string.tab_logs, Icons.Filled.Info),
    Settings(R.string.tab_settings, Icons.Filled.Settings),
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
                        icon = { Icon(t.icon, contentDescription = null) },
                        label = { Text(stringResource(t.labelRes)) },
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
