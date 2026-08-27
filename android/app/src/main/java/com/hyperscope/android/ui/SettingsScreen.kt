package com.hyperscope.android.ui

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.RadioButton
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import com.hyperscope.android.R

private val themes = listOf(
    "auto" to R.string.theme_auto,
    "light" to R.string.theme_light,
    "dark" to R.string.theme_dark,
)

@Composable
fun SettingsScreen(vm: AppViewModel) {
    val nodes by vm.nodes.collectAsStateWithLifecycle()
    val theme by vm.theme.collectAsStateWithLifecycle()
    val lang by vm.lang.collectAsStateWithLifecycle()

    Column(Modifier.fillMaxSize().padding(16.dp).verticalScroll(rememberScrollState())) {
        Text(stringResource(R.string.settings), style = MaterialTheme.typography.titleLarge)
        Spacer(Modifier.height(16.dp))

        Card(colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface)) {
            Column(Modifier.padding(16.dp)) {
                Text(stringResource(R.string.language), style = MaterialTheme.typography.titleSmall)
                LanguagePicker(lang = lang, onSelect = vm::setLang)
            }
        }
        Spacer(Modifier.height(16.dp))

        Card(colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface)) {
            Column(Modifier.padding(16.dp)) {
                Text(stringResource(R.string.theme), style = MaterialTheme.typography.titleSmall)
                themes.forEach { (k, labelRes) ->
                    Row(verticalAlignment = Alignment.CenterVertically, modifier = Modifier.fillMaxWidth()) {
                        RadioButton(selected = theme == k, onClick = { vm.setTheme(k) })
                        Text(stringResource(labelRes))
                    }
                }
            }
        }
        Spacer(Modifier.height(16.dp))

        Text(stringResource(R.string.added_nodes), style = MaterialTheme.typography.titleSmall)
        Spacer(Modifier.height(8.dp))
        if (nodes.isEmpty()) {
            Text(stringResource(R.string.no_nodes_short),
                color = MaterialTheme.colorScheme.onBackground.copy(alpha = 0.6f))
        } else {
            nodes.forEach { n ->
                Card(colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface),
                    modifier = Modifier.fillMaxWidth().padding(bottom = 8.dp)) {
                    Row(Modifier.padding(14.dp), verticalAlignment = Alignment.CenterVertically) {
                        Column(Modifier.weight(1f)) {
                            Text(n.config.name, style = MaterialTheme.typography.bodyMedium)
                            Text("${n.config.addr}:${n.config.port}",
                                style = MaterialTheme.typography.bodySmall,
                                color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.6f))
                        }
                        Text(stringResource(R.string.delete), color = MaterialTheme.colorScheme.error,
                            modifier = Modifier.clickable { vm.removeNode(n.config.name) },
                            style = MaterialTheme.typography.bodyMedium)
                    }
                }
            }
        }

        Spacer(Modifier.height(20.dp))
        Button(onClick = { vm.logout() }, modifier = Modifier.fillMaxWidth()) {
            Text(stringResource(R.string.logout), color = MaterialTheme.colorScheme.error)
        }
    }
}
