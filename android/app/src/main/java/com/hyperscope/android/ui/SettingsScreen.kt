package com.hyperscope.android.ui

import androidx.compose.foundation.clickable
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
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Add
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.RadioButton
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle

private val themes = listOf("auto" to "自动", "light" to "浅色", "dark" to "深色")

@Composable
fun SettingsScreen(vm: AppViewModel) {
    val nodes by vm.nodes.collectAsStateWithLifecycle()
    val theme by vm.theme.collectAsStateWithLifecycle()
    var name by rememberSaveable { mutableStateOf("") }
    var addr by rememberSaveable { mutableStateOf("") }
    var port by rememberSaveable { mutableStateOf("5000") }
    var key by rememberSaveable { mutableStateOf("") }

    Column(Modifier.fillMaxSize().padding(16.dp).verticalScroll(rememberScrollState())) {
        Text("设置", style = MaterialTheme.typography.titleLarge)
        Spacer(Modifier.height(16.dp))

        Card(colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface)) {
            Column(Modifier.padding(16.dp)) {
                Text("添加节点", style = MaterialTheme.typography.titleSmall)
                Spacer(Modifier.height(10.dp))
                OutlinedTextField(value = name, onValueChange = { name = it },
                    label = { Text("名称（可选）") }, modifier = Modifier.fillMaxWidth())
                Spacer(Modifier.height(8.dp))
                OutlinedTextField(value = addr, onValueChange = { addr = it },
                    label = { Text("节点地址，如 192.168.1.100") }, modifier = Modifier.fillMaxWidth())
                Spacer(Modifier.height(8.dp))
                OutlinedTextField(value = port, onValueChange = { port = it },
                    label = { Text("端口（默认 5000）") }, modifier = Modifier.fillMaxWidth())
                Spacer(Modifier.height(8.dp))
                OutlinedTextField(value = key, onValueChange = { key = it },
                    label = { Text("API Key（节点上运行 hyper-node key show 获取）") },
                    modifier = Modifier.fillMaxWidth())
                Spacer(Modifier.height(12.dp))
                Button(onClick = {
                    if (addr.isNotBlank()) {
                        vm.addNode(name, addr, port.toIntOrNull() ?: 5000, key.trim())
                        name = ""; addr = ""; key = ""
                    }
                }, modifier = Modifier.fillMaxWidth()) {
                    Icon(Icons.Filled.Add, null); Spacer(Modifier.width(6.dp)); Text("添加")
                }
            }
        }

        Spacer(Modifier.height(16.dp))
        Text("已添加节点", style = MaterialTheme.typography.titleSmall)
        Spacer(Modifier.height(8.dp))
        if (nodes.isEmpty()) {
            Text("暂无节点", color = MaterialTheme.colorScheme.onBackground.copy(alpha = 0.6f))
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
                        Text("删除", color = MaterialTheme.colorScheme.error,
                            modifier = Modifier.clickable { vm.removeNode(n.config.name) },
                            style = MaterialTheme.typography.bodyMedium)
                    }
                }
            }
        }

        Spacer(Modifier.height(20.dp))
        Card(colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface)) {
            Column(Modifier.padding(16.dp)) {
                Text("主题", style = MaterialTheme.typography.titleSmall)
                themes.forEach { (k, label) ->
                    Row(verticalAlignment = Alignment.CenterVertically,
                        modifier = Modifier.fillMaxWidth()) {
                        RadioButton(selected = theme == k, onClick = { vm.setTheme(k) })
                        Text(label)
                    }
                }
            }
        }
    }
}
