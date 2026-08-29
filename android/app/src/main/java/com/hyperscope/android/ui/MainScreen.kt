package com.hyperscope.android.ui

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.safeDrawing
import androidx.compose.foundation.lazy.grid.LazyGridState
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Build
import androidx.compose.material.icons.filled.Home
import androidx.compose.material.icons.filled.Info
import androidx.compose.material.icons.filled.Settings
import androidx.compose.material3.Icon
import androidx.compose.material3.NavigationBar
import androidx.compose.material3.NavigationBarItem
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.runtime.snapshotFlow
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.res.stringResource
import com.hyperscope.android.R
import kotlinx.coroutines.flow.collect

// Nodes are selected from the top bar of the Dashboard, not the bottom nav.
enum class Tab(val labelRes: Int, val icon: ImageVector) {
    Dashboard(R.string.tab_dashboard, Icons.Filled.Home),
    Events(R.string.tab_events, Icons.Filled.Info),
    Control(R.string.tab_control, Icons.Filled.Build),
    Settings(R.string.tab_settings, Icons.Filled.Settings),
}

@Composable
fun MainScreen(vm: AppViewModel) {
    // rememberSaveable survives Activity recreation (e.g. language switch),
    // so the user stays on the current tab instead of bouncing to Dashboard.
    var tabName by rememberSaveable { mutableStateOf(Tab.Dashboard.name) }
    val tab = Tab.valueOf(tabName)
    // Save the dashboard grid scroll position across tab switches AND across
    // Activity recreation (language change), so it never jumps back to top.
    val savedIndex = rememberSaveable { mutableStateOf(0) }
    val savedOffset = rememberSaveable { mutableStateOf(0) }
    // remember() keeps the SAME LazyGridState instance across recompositions —
    // creating a new one each recomposition resets scrolling (cards become
    // undraggable). Saved values restore the position after recreation.
    val dashboardGridState = remember { LazyGridState(savedIndex.value, savedOffset.value) }
    // Persist the live scroll position whenever it changes.
    LaunchedEffect(dashboardGridState) {
        snapshotFlow { dashboardGridState.firstVisibleItemIndex to dashboardGridState.firstVisibleItemScrollOffset }
            .collect { (i, o) ->
                savedIndex.value = i
                savedOffset.value = o
            }
    }

    Scaffold(
        // Explicitly consume system bar insets (status bar top + nav bar bottom)
        // so edge-to-edge content never slides under them.
        contentWindowInsets = WindowInsets.safeDrawing,
        bottomBar = {
            NavigationBar {
                Tab.entries.forEach { t ->
                    NavigationBarItem(
                        selected = tab == t,
                        onClick = { tabName = t.name },
                        icon = { Icon(t.icon, contentDescription = null) },
                        label = { Text(stringResource(t.labelRes)) },
                    )
                }
            }
        },
    ) { pad ->
        Box(Modifier.fillMaxSize().padding(pad)) {
            when (tab) {
                Tab.Dashboard -> DashboardScreen(vm, dashboardGridState)
                Tab.Events -> EventsScreen(vm)
                Tab.Control -> ControlScreen(vm)
                Tab.Settings -> SettingsScreen(vm)
            }
        }
    }
}
