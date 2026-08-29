package com.hyperscope.android.ui

import android.app.Application
import androidx.lifecycle.viewModelScope
import com.hyperscope.android.data.AppLog
import com.hyperscope.android.data.NodeClient
import com.hyperscope.android.data.NodeView
import kotlinx.coroutines.launch

/**
 * Machine control operations (reboot / shutdown / kill process / docker) for
 * the control tab. Holds a reference to the owning ViewModel to read the node
 * list and to trigger a refresh after docker actions. Split from
 * AppViewModel so it stays a thin state holder.
 */
internal class NodeController(
    private val vm: AppViewModel,
    private val client: NodeClient,
) {
    /**
     * Resolve a node by name, run a control action inside a coroutine, and
     * route failures to the UI callback. Shared by all control operations so
     * they don't repeat the lookup + try/catch + log boilerplate.
     */
    fun withNode(name: String, onResult: (String) -> Unit, block: suspend (NodeView) -> Unit) {
        vm.viewModelScope.launch {
            val node = vm.nodes.value.firstOrNull { it.config.name == name }
            if (node == null) {
                onResult(vm.appString(com.hyperscope.android.R.string.op_node_not_found))
                return@launch
            }
            try {
                block(node)
            } catch (e: Exception) {
                AppLog.e("Ctl", "control on ${node.config.name} failed: ${e.message}")
                onResult(e.message ?: vm.appString(com.hyperscope.android.R.string.op_node_not_found))
            }
        }
    }

    /** Send reboot to a node; result message via callback. */
    fun rebootNode(name: String, onResult: (String) -> Unit) = withNode(name, onResult) { node ->
        AppLog.d("Ctl", "reboot ${node.config.name}")
        client.reboot(node.config)
        onResult(vm.appString(com.hyperscope.android.R.string.op_reboot_sent))
    }

    fun shutdownNode(name: String, onResult: (String) -> Unit) = withNode(name, onResult) { node ->
        AppLog.d("Ctl", "shutdown ${node.config.name}")
        client.shutdown(node.config)
        onResult(vm.appString(com.hyperscope.android.R.string.op_shutdown_sent))
    }

    fun killProcessOnNode(name: String, pid: Int, onResult: (String) -> Unit) = withNode(name, onResult) { node ->
        AppLog.d("Ctl", "kill pid=$pid on ${node.config.name}")
        client.killProcess(node.config, pid)
        onResult(vm.appString(com.hyperscope.android.R.string.op_kill_ok, pid))
    }

    fun dockerActionOnNode(name: String, container: String, action: String, onResult: (String) -> Unit) = withNode(name, onResult) { node ->
        AppLog.d("Ctl", "docker $action $container on ${node.config.name}")
        client.dockerAction(node.config, container, action)
        onResult(vm.appString(com.hyperscope.android.R.string.op_docker_ok, container, action))
        vm.refreshOnePublic(name)
    }
}
