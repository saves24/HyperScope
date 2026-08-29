package com.hyperscope.android.ui

import android.net.Uri
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import com.hyperscope.android.R
import com.hyperscope.android.data.HsxCodec
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

/**
 * Reusable encrypted-config import UI used by the node manager. Picks a file,
 * asks for the passphrase, decrypts locally and merges nodes via the ViewModel.
 */
@Composable
fun ImportConfigSection(vm: AppViewModel) {
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

    OutlinedButton(onClick = {
        importLauncher.launch(arrayOf("application/octet-stream", "application/x-hsxc", "*/*"))
    }, modifier = Modifier.fillMaxWidth()) {
        Text(stringResource(R.string.import_config))
    }
    resultMsg?.let {
        Spacer(Modifier.height(6.dp))
        Text(it, style = androidx.compose.material3.MaterialTheme.typography.bodySmall,
            color = androidx.compose.material3.MaterialTheme.colorScheme.primary)
    }

    if (showPassDialog) {
        val uri = pendingUri
        var passErr by remember { mutableStateOf<String?>(null) }
        AlertDialog(
            onDismissRequest = { showPassDialog = false; passErr = null },
            title = { Text(stringResource(R.string.import_pass_title)) },
            text = {
                Column {
                    OutlinedTextField(value = pass, onValueChange = { pass = it },
                        label = { Text(stringResource(R.string.import_pass)) },
                        singleLine = true,
                        isError = passErr != null)
                    passErr?.let {
                        Spacer(Modifier.height(6.dp))
                        Text(it, color = MaterialTheme.colorScheme.error,
                            style = MaterialTheme.typography.bodySmall)
                    }
                }
            },
            confirmButton = {
                TextButton(onClick = {
                    val p = pass
                    if (p.isEmpty()) { passErr = context.getString(R.string.import_pass_need); return@TextButton }
                    if (uri != null) {
                        scope.launch {
                            val bytes = withContext(Dispatchers.IO) {
                                runCatching {
                                    context.contentResolver.openInputStream(uri)?.use { it.readBytes() } ?: ByteArray(0)
                                }.getOrDefault(ByteArray(0))
                            }
                            if (bytes.isNotEmpty() && HsxCodec.isHsx(bytes)) {
                                vm.importNodes(bytes, p) { msg ->
                                    if (msg == context.getString(R.string.import_ok)) {
                                        // Success: close the dialog and reset.
                                        showPassDialog = false; passErr = null; pass = ""
                                    } else {
                                        // Wrong passphrase / failure: keep the
                                        // dialog open and show the error inside.
                                        passErr = msg
                                    }
                                    resultMsg = msg
                                }
                            } else {
                                passErr = context.getString(R.string.import_not_hsx)
                            }
                        }
                    }
                }) { Text(stringResource(R.string.confirm)) }
            },
            dismissButton = {
                TextButton(onClick = { showPassDialog = false; passErr = null; pass = "" }) { Text(stringResource(R.string.cancel)) }
            }
        )
    }
}
