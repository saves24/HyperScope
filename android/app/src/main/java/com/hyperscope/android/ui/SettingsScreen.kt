package com.hyperscope.android.ui

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
import androidx.compose.material3.Button
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.RadioButton
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import kotlinx.coroutines.launch
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import androidx.compose.runtime.rememberCoroutineScope

private val themes = listOf("auto" to "自动", "light" to "浅色", "dark" to "深色")

@Composable
fun SettingsScreen(vm: AppViewModel) {
    val ui by vm.ui.collectAsStateWithLifecycle()
    val scope = rememberCoroutineScope()
    var baseUrl by rememberSaveable { mutableStateOf(ui.baseUrl) }

    Column(Modifier.fillMaxSize().padding(20.dp).verticalScroll(rememberScrollState())) {
        Text("设置", style = MaterialTheme.typography.titleLarge)
        Spacer(Modifier.height(20.dp))

        StatCard("服务器") {
            OutlinedTextField(value = baseUrl, onValueChange = { baseUrl = it },
                label = { Text("服务器地址") }, modifier = Modifier.fillMaxWidth())
            Button(onClick = { scope.launch { vm.setBaseUrl(baseUrl) } },
                modifier = Modifier.fillMaxWidth().padding(top = 8.dp)) {
                Text("保存地址")
            }
        }
        Spacer(Modifier.height(14.dp))

        StatCard("主题") {
            themes.forEach { (k, label) ->
                Row(verticalAlignment = Alignment.CenterVertically, modifier = Modifier.fillMaxWidth()) {
                    RadioButton(selected = ui.theme == k, onClick = { vm.setTheme(k) })
                    Text(label)
                }
            }
        }
        Spacer(Modifier.height(14.dp))

        StatCard("账户") {
            Text("当前用户：${ui.user.ifBlank { "--" }}")
            Button(onClick = { vm.logout() }, modifier = Modifier.fillMaxWidth().padding(top = 8.dp)) {
                Text("退出登录")
            }
        }
    }
}
