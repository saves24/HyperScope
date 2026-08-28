package com.hyperscope.android.ui

import android.net.Uri
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
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
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.RadioButton
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import com.hyperscope.android.R
import com.hyperscope.android.data.HsxCodec
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

private val themes = listOf(
    "auto" to R.string.theme_auto,
    "light" to R.string.theme_light,
    "dark" to R.string.theme_dark,
)

@Composable
fun SettingsScreen(vm: AppViewModel) {
    val nodes by vm.nodes.collectAsState()
    val theme by vm.theme.collectAsState()
    val lang by vm.lang.collectAsState()
    val context = LocalContext.current
    val scope = rememberCoroutineScope()

    var showPassDialog by remember { mutableStateOf(false) }
    var pendingUri by remember { mutableStateOf<Uri?>(null) }
    var pass by remember { mutableStateOf("") }
    var resultMsg by remember { mutableStateOf<String?>(null) }

    val importLauncher = rememberLauncherForActivityResult(
        ActivityResultContracts.OpenDocument()
    ) { uri ->
        if (uri != null) {
            pendingUri = uri
            showPassDialog = true
        }
    }

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

        Card(colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface)) {
            Column(Modifier.padding(16.dp)) {
                Text(stringResource(R.string.import_config), style = MaterialTheme.typography.titleSmall)
                Spacer(Modifier.height(4.dp))
                Text(stringResource(R.string.import_config_hint),
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.6f))
                Spacer(Modifier.height(8.dp))
                OutlinedButton(onClick = { importLauncher.launch(arrayOf("application/octet-stream", "application/x-hsxc", "*/*")) }) {
                    Text(stringResource(R.string.import_config))
                }
                resultMsg?.let {
                    Spacer(Modifier.height(6.dp))
                    Text(it, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.primary)
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

    if (showPassDialog) {
        val uri = pendingUri
        AlertDialog(
            onDismissRequest = { showPassDialog = false },
            title = { Text(stringResource(R.string.import_pass_title)) },
            text = {
                Column {
                    OutlinedTextField(value = pass, onValueChange = { pass = it },
                        label = { Text(stringResource(R.string.import_pass)) },
                        singleLine = true)
                }
            },
            confirmButton = {
                TextButton(onClick = {
                    showPassDialog = false
                    val p = pass
                    pass = ""
                    if (uri != null && p.isNotEmpty()) {
                        scope.launch {
                            val bytes = withContext(Dispatchers.IO) {
                                runCatching {
                                    context.contentResolver.openInputStream(uri)?.use { it.readBytes() } ?: ByteArray(0)
                                }.getOrDefault(ByteArray(0))
                            }
                            if (bytes.isNotEmpty() && HsxCodec.isHsx(bytes)) {
                                vm.importNodes(bytes, p) { msg -> resultMsg = msg }
                            } else {
                                resultMsg = context.getString(R.string.import_not_hsx)
                            }
                        }
                    }
                }) { Text(stringResource(R.string.confirm)) }
            },
            dismissButton = {
                TextButton(onClick = { showPassDialog = false; pass = "" }) { Text(stringResource(R.string.cancel)) }
            }
        )
    }
}
