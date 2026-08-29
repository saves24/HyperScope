package com.hyperscope.android.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.statusBarsPadding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Button
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import com.hyperscope.android.R

/** First-run credential setup (no account yet) or login (account exists). */
@Composable
fun AuthScreen(vm: AppViewModel, setup: Boolean) {
    val authError by vm.authError.collectAsStateWithLifecycle()
    val lang by vm.lang.collectAsStateWithLifecycle()
    var user by rememberSaveable { mutableStateOf("") }
    var pass by rememberSaveable { mutableStateOf("") }
    var confirm by rememberSaveable { mutableStateOf("") }

    // Language picker is anchored in the top bar (outside the scroll/center
    // container) so its dropdown stays correctly aligned under the button.
    // statusBarsPadding keeps it clear of the system status bar (edge-to-edge).
    Box(Modifier.fillMaxSize()) {
        Row(Modifier.fillMaxWidth().statusBarsPadding().padding(12.dp)) {
            Spacer(Modifier.weight(1f))
            LanguagePicker(lang = lang, onSelect = vm::setLang)
        }
        Column(
            Modifier.fillMaxSize().padding(horizontal = 24.dp)
                .imePadding()
                .verticalScroll(rememberScrollState()),
            verticalArrangement = Arrangement.Center,
            horizontalAlignment = Alignment.CenterHorizontally,
        ) {
            Text("HyperScope", style = MaterialTheme.typography.headlineMedium)
            Text(if (setup) stringResource(R.string.auth_setup_title) else stringResource(R.string.auth_login_title),
                style = MaterialTheme.typography.bodyMedium,
                color = MaterialTheme.colorScheme.onBackground.copy(alpha = 0.6f))
            Spacer(Modifier.height(28.dp))
            OutlinedTextField(value = user, onValueChange = { user = it },
                label = { Text(stringResource(R.string.auth_username)) }, modifier = Modifier.fillMaxWidth())
            Spacer(Modifier.height(12.dp))
            OutlinedTextField(value = pass, onValueChange = { pass = it },
                label = { Text(stringResource(R.string.auth_password)) }, modifier = Modifier.fillMaxWidth())
            if (setup) {
                Spacer(Modifier.height(12.dp))
                OutlinedTextField(value = confirm, onValueChange = { confirm = it },
                    label = { Text(stringResource(R.string.auth_confirm)) }, modifier = Modifier.fillMaxWidth())
            }
            authError?.let {
                Spacer(Modifier.height(10.dp))
                Text(it, color = MaterialTheme.colorScheme.error, style = MaterialTheme.typography.bodySmall)
            }
            Spacer(Modifier.height(20.dp))
            Button(onClick = {
                if (setup) vm.setup(user, pass, confirm) else vm.login(user, pass)
            }, modifier = Modifier.fillMaxWidth()) {
                Text(if (setup) stringResource(R.string.auth_setup_action) else stringResource(R.string.auth_login_action))
            }
        }
    }
}
