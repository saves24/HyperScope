package com.hyperscope.android.ui

import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.layout.Arrangement
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
import com.hyperscope.android.data.AppLog
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

/** Compact theme radio (auto / light / dark). */
@Composable
private fun ThemeOption(value: String, labelRes: Int, current: String, onSelect: (String) -> Unit) {
    Column(horizontalAlignment = Alignment.CenterHorizontally, modifier = Modifier.padding(horizontal = 4.dp)) {
        RadioButton(selected = current == value, onClick = { onSelect(value) })
        Text(stringResource(labelRes), style = MaterialTheme.typography.labelSmall,
            color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.8f))
    }
}

@Composable
fun SettingsScreen(vm: AppViewModel) {
    val theme by vm.theme.collectAsState()
    val lang by vm.lang.collectAsState()
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    var showLogDialog by remember { mutableStateOf(false) }
    var showChangeDialog by remember { mutableStateOf(false) }
    var logMsg by remember { mutableStateOf<String?>(null) }

    val exportLogLauncher = rememberLauncherForActivityResult(
        ActivityResultContracts.CreateDocument("text/plain")
    ) { uri ->
        if (uri != null) {
            scope.launch {
                withContext(Dispatchers.IO) {
                    runCatching {
                        context.contentResolver.openOutputStream(uri)?.use { out ->
                            out.write(AppLog.dump().toByteArray(Charsets.UTF_8))
                        }
                    }
                }
                logMsg = context.getString(R.string.log_exported)
            }
        }
    }

    Column(Modifier.fillMaxSize().padding(16.dp).verticalScroll(rememberScrollState())) {
        Text(stringResource(R.string.settings), style = MaterialTheme.typography.titleLarge)
        Spacer(Modifier.height(16.dp))

        // Row: language + auto theme together.
        Card(colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface)) {
            Row(Modifier.fillMaxWidth().padding(16.dp), verticalAlignment = Alignment.CenterVertically) {
                Column(Modifier.weight(1f)) {
                    Text(stringResource(R.string.language), style = MaterialTheme.typography.titleSmall)
                    LanguagePicker(lang = lang, onSelect = vm::setLang)
                }
                Column(Modifier.weight(1f), horizontalAlignment = Alignment.CenterHorizontally) {
                    Text(stringResource(R.string.theme), style = MaterialTheme.typography.titleSmall)
                    Row(verticalAlignment = Alignment.CenterVertically) {
                        ThemeOption("auto", R.string.theme_auto, theme, vm::setTheme)
                        ThemeOption("light", R.string.theme_light, theme, vm::setTheme)
                        ThemeOption("dark", R.string.theme_dark, theme, vm::setTheme)
                    }
                }
            }
        }
        Spacer(Modifier.height(16.dp))

        // Runtime log: view in-app state and export to local file.
        Card(colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface)) {
            Column(Modifier.padding(16.dp)) {
                Text(stringResource(R.string.runtime_log), style = MaterialTheme.typography.titleSmall)
                Spacer(Modifier.height(4.dp))
                Text(stringResource(R.string.runtime_log_hint),
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.6f))
                Spacer(Modifier.height(8.dp))
                Row(horizontalArrangement = Arrangement.spacedBy(12.dp)) {
                    OutlinedButton(onClick = { showLogDialog = true }, modifier = Modifier.weight(1f)) {
                        Text(stringResource(R.string.view_log))
                    }
                    OutlinedButton(onClick = { exportLogLauncher.launch("hsapp-log.txt") }, modifier = Modifier.weight(1f)) {
                        Text(stringResource(R.string.export_log))
                    }
                }
                logMsg?.let {
                    Spacer(Modifier.height(6.dp))
                    Text(it, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.primary)
                }
            }
        }

        Spacer(Modifier.height(20.dp))
        // Device public key: needed to trust this device on a node so that
        // signed relay commands are accepted (hyper-node device add).
        Column(Modifier.padding(16.dp)) {
            Text(stringResource(R.string.device_public_key), style = MaterialTheme.typography.titleSmall)
            Spacer(Modifier.height(4.dp))
            Text(
                DeviceIdentity.publicKeyB64(),
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                modifier = Modifier.fillMaxWidth(),
            )
            Spacer(Modifier.height(6.dp))
            Text(
                stringResource(R.string.device_public_key_hint),
                style = MaterialTheme.typography.labelSmall,
                color = MaterialTheme.colorScheme.onBackground.copy(alpha = 0.6f),
            )
            Spacer(Modifier.height(8.dp))
            OutlinedButton(onClick = {
                runCatching {
                    val clipboard = context.getSystemService(android.content.Context.CLIPBOARD_SERVICE) as android.content.ClipboardManager
                    clipboard.setPrimaryClip(android.content.ClipData.newPlainText("device_pubkey", DeviceIdentity.publicKeyB64()))
                    android.widget.Toast.makeText(context, R.string.copied, android.widget.Toast.LENGTH_SHORT).show()
                }
            }) {
                Text(stringResource(R.string.copy))
            }
        }
        Spacer(Modifier.height(6.dp))
        // Change local account (username/password).
        OutlinedButton(onClick = { showChangeDialog = true }, modifier = Modifier.fillMaxWidth()) {
            Text(stringResource(R.string.change_account), color = MaterialTheme.colorScheme.onSurface)
        }
        Spacer(Modifier.height(10.dp))
        // Sign out: neutral outlined style (no purple fill / red text clash).
        OutlinedButton(onClick = { vm.logout() }, modifier = Modifier.fillMaxWidth()) {
            Text(stringResource(R.string.logout), color = MaterialTheme.colorScheme.onSurface)
        }

        Spacer(Modifier.height(24.dp))
        // Author + repo info (tap to open the GitHub repository in a browser).
        Column(Modifier.fillMaxWidth(), horizontalAlignment = Alignment.CenterHorizontally) {
            Text("HyperScope v1.0.0", style = MaterialTheme.typography.labelMedium,
                color = MaterialTheme.colorScheme.onBackground.copy(alpha = 0.6f))
            Spacer(Modifier.height(2.dp))
            Text(stringResource(R.string.about_author), style = MaterialTheme.typography.labelSmall,
                color = MaterialTheme.colorScheme.onBackground.copy(alpha = 0.5f))
            Spacer(Modifier.height(6.dp))
            // "View source" button → opens the GitHub repo in the browser.
            OutlinedButton(onClick = {
                runCatching {
                    val intent = android.content.Intent(
                        android.content.Intent.ACTION_VIEW,
                        android.net.Uri.parse("https://github.com/saves24/HyperScope"),
                    )
                    context.startActivity(intent)
                }
            }, modifier = Modifier.fillMaxWidth()) {
                Text(stringResource(R.string.view_source), color = MaterialTheme.colorScheme.onSurface)
            }
        }
        Spacer(Modifier.height(12.dp))
    }

    if (showChangeDialog) {
        ChangeAccountDialog(vm) { showChangeDialog = false }
    }

    if (showLogDialog) {
        val dump = AppLog.dump()
        AlertDialog(
            onDismissRequest = { showLogDialog = false },
            title = { Text(stringResource(R.string.runtime_log)) },
            text = {
                if (dump.isBlank()) {
                    Text(stringResource(R.string.runtime_log_hint))
                } else {
                    Text(dump, style = MaterialTheme.typography.bodySmall,
                        modifier = Modifier.verticalScroll(rememberScrollState()).height(300.dp))
                }
            },
            confirmButton = { TextButton(onClick = { showLogDialog = false }) { Text(stringResource(R.string.done)) } },
        )
    }
}

/** Dialog to change the local account username/password (old credentials required). */
@Composable
private fun ChangeAccountDialog(vm: AppViewModel, onDismiss: () -> Unit) {
    val context = LocalContext.current
    var oldUser by remember { mutableStateOf("") }
    var oldPass by remember { mutableStateOf("") }
    var newUser by remember { mutableStateOf("") }
    var newPass by remember { mutableStateOf("") }
    var confirmPass by remember { mutableStateOf("") }
    var msg by remember { mutableStateOf<String?>(null) }
    var done by remember { mutableStateOf(false) }

    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text(stringResource(R.string.change_account)) },
        text = {
            Column(Modifier.verticalScroll(rememberScrollState())) {
                OutlinedTextField(value = oldUser, onValueChange = { oldUser = it },
                    label = { Text(stringResource(R.string.change_old_user)) }, modifier = Modifier.fillMaxWidth())
                Spacer(Modifier.height(8.dp))
                OutlinedTextField(value = oldPass, onValueChange = { oldPass = it },
                    label = { Text(stringResource(R.string.change_old_pass)) }, modifier = Modifier.fillMaxWidth(),
                    visualTransformation = androidx.compose.ui.text.input.PasswordVisualTransformation())
                Spacer(Modifier.height(8.dp))
                OutlinedTextField(value = newUser, onValueChange = { newUser = it },
                    label = { Text(stringResource(R.string.change_new_user)) }, modifier = Modifier.fillMaxWidth())
                Spacer(Modifier.height(8.dp))
                OutlinedTextField(value = newPass, onValueChange = { newPass = it },
                    label = { Text(stringResource(R.string.change_new_pass)) }, modifier = Modifier.fillMaxWidth(),
                    visualTransformation = androidx.compose.ui.text.input.PasswordVisualTransformation())
                Spacer(Modifier.height(8.dp))
                OutlinedTextField(value = confirmPass, onValueChange = { confirmPass = it },
                    label = { Text(stringResource(R.string.change_confirm_pass)) }, modifier = Modifier.fillMaxWidth(),
                    visualTransformation = androidx.compose.ui.text.input.PasswordVisualTransformation())
                msg?.let {
                    Spacer(Modifier.height(8.dp))
                    Text(it, color = if (done) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.error,
                        style = MaterialTheme.typography.bodySmall)
                }
            }
        },
        confirmButton = {
            TextButton(onClick = {
                if (newPass != confirmPass) {
                    msg = context.getString(R.string.change_mismatch)
                    return@TextButton
                }
                vm.changeCredentials(oldUser, oldPass, newUser, newPass) { ok, m ->
                    msg = m
                    done = ok
                    if (ok) onDismiss()
                }
            }) { Text(stringResource(R.string.confirm)) }
        },
        dismissButton = { TextButton(onClick = onDismiss) { Text(stringResource(R.string.cancel)) } },
    )
}
