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
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle

@Composable
fun LoginScreen(vm: AppViewModel) {
    val ui by vm.ui.collectAsStateWithLifecycle()
    var user by remember { mutableStateOf("") }
    var pass by remember { mutableStateOf("") }

    Column(
        Modifier.fillMaxSize().padding(24.dp),
        verticalArrangement = Arrangement.Center,
        horizontalAlignment = Alignment.CenterHorizontally,
    ) {
        Text("HyperScope", style = MaterialTheme.typography.headlineMedium)
        Spacer(Modifier.height(24.dp))
        OutlinedTextField(
            value = ui.baseUrl, onValueChange = vm::setBaseUrl,
            label = { Text("服务器地址") }, modifier = Modifier.fillMaxWidth(),
        )
        Spacer(Modifier.height(12.dp))
        OutlinedTextField(
            value = user, onValueChange = { user = it },
            label = { Text("用户名") }, modifier = Modifier.fillMaxWidth(),
        )
        Spacer(Modifier.height(12.dp))
        OutlinedTextField(
            value = pass, onValueChange = { pass = it },
            label = { Text("密码") }, modifier = Modifier.fillMaxWidth(),
        )
        if (ui.error != null) {
            Spacer(Modifier.height(10.dp))
            Text(ui.error!!, color = MaterialTheme.colorScheme.error,
                style = MaterialTheme.typography.bodySmall)
        }
        Spacer(Modifier.height(20.dp))
        Button(onClick = { vm.login(user, pass) }, enabled = !ui.loading,
            modifier = Modifier.fillMaxWidth()) {
            Text(if (ui.loading) "登录中..." else "登录")
        }
    }
}

@Composable
fun DashboardScreen(vm: AppViewModel) {
    val system by vm.system.collectAsStateWithLifecycle()
    Column(Modifier.fillMaxSize().padding(16.dp).verticalScroll(rememberScrollState())) {
        StatCard("系统总览") {
            StatRow("主机名", system.hostname ?: "--")
            StatRow("版本", system.version ?: "--")
            StatRow("内核", system.kernel ?: "--")
            StatRow("负载", system.load ?: "--")
            StatRow("运行时间", system.uptime ?: "--")
            StatRow("进程数", system.procs?.toString() ?: "--")
        }
        Spacer(Modifier.height(14.dp))
        StatCard("CPU / 内存") {
            StatRow("CPU", "${system.cpu ?: 0.0}%")
            val mem = system.memory
            StatRow("内存", mem?.let { "${it.used / 1048576} / ${it.total / 1048576} MB" } ?: "--")
            StatRow("内存使用率", mem?.percent?.let { "$it%" } ?: "--")
        }
        Spacer(Modifier.height(14.dp))
        StatCard("温度") {
            StatRow("CPU 温度", system.temp?.cpu?.let { "$it°C" } ?: "--")
            StatRow("GPU 温度", system.temp?.gpu?.let { "$it°C" } ?: "--")
        }
    }
}
