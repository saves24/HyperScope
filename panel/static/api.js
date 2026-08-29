


function loadPanelPort() {
  apiFetch("/api/settings")
      .then(r => r.json())
      .then(d => {
          document.getElementById("panelPort").value = d.panel_port || 8088;
      })
      .catch(() => {});
}
function savePanelPort() {
  const p = parseInt(document.getElementById("panelPort").value) || 0;
  if (p < 1 || p > 65535) {
      showToast(t("panel-port-invalid"));
      return;
  }
  apiFetch("/api/settings", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ panel_port: p })
  })
      .then(r => r.json())
      .then(d => {
          if (d.ok) {
              if (d.restarting) {
                  // record new port; panel will restart
                  setCookie("ts-pending-port", String(d.port), 1);
                  showToast(t("restarting"));
                  setTimeout(() => {
                      location.href = location.protocol + "//" + location.hostname + ":" + d.port;
                  }, 1500);
              } else {
                  // port unchanged or cannot auto-restart (not running under systemd)
                  showToast(t("saved") + (d.port != (parseInt(location.port) || 8088) ? " · " + t("restart-needed") : ""));
              }
          } else {
              showToast(d.error || t("op-err"));
          }
      })
      .catch(() => showToast(t("op-err")));
}
function addNode() {
  const addrInput = document.getElementById("addNodeAddr").value.trim();
  const key = document.getElementById("addNodeKey").value.trim();
  if (!addrInput || !key) {
      showToast(t("node-unauth"));
      return;
  }
  let addr = addrInput;
  let port = 5000;
  if (addrInput.includes(":")) {
      const parts = addrInput.split(":");
      addr = parts[0];
      if (parts[1]) port = parseInt(parts[1]) || 5000;
  }
  const name = addr;
  // key with cert fingerprint (key|SHA256:fp) forces TLS; else follow checkbox
  const keyHasFp = key.includes("|");
  const tls = keyHasFp ? true : document.getElementById("addNodeTls").checked;
  if (tls && !keyHasFp) {
      showToast(t("node-tls-key-hint") || "Plain key cannot enable TLS; run hyper-node key show to get the full key with cert fingerprint");
      return;
  }
  // Optional alert settings (webhook + per-metric thresholds)
  const webhook = (document.getElementById("addNodeWebhook")?.value || "").trim();
  const alertCpu = parseFloat(document.getElementById("addNodeAlertCpu")?.value) || null;
  const alertMem = parseFloat(document.getElementById("addNodeAlertMem")?.value) || null;
  const alertDisk = parseFloat(document.getElementById("addNodeAlertDisk")?.value) || null;
  const alertTemp = parseFloat(document.getElementById("addNodeAlertTemp")?.value) || null;
  const alertSettings = {};
  if (webhook) alertSettings.webhook = webhook;
  if (alertCpu) alertSettings.alert_cpu = alertCpu;
  if (alertMem) alertSettings.alert_mem = alertMem;
  if (alertDisk) alertSettings.alert_disk = alertDisk;
  if (alertTemp) alertSettings.alert_temp = alertTemp;
  apiFetch("/api/nodes", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ name, addr, port, key, tls, ...alertSettings })
  })
      .then(r => r.json())
      .then(d => {
          if (d.ok) {
              document.getElementById("nodeManagerModal").classList.remove("show");
              document.getElementById("addNodeAddr").value = "";
              document.getElementById("addNodeKey").value = "";
              loadNodeList();
              showToast(t("node-added"));
          } else {
              showToast(d.error || t("node-unauth"));
          }
      })
      .catch(() => showToast(t("node-unauth")));
}
function renameNode(i) {
  renameTargetIdx = i;
  const old = nodeNames[i] || nodes[i] || "";
  document.getElementById("renameTitle").textContent = t("rename-node") + " - " + old;
  document.getElementById("renameInput").value = old;
  document.getElementById("renameModal").classList.add("show");
  document.getElementById("renameInput").focus();
  document.getElementById("renameInput").select();
}
function rebootNode(i) {
  const name = nodeNames[i] || nodes[i] || "";
  showConfirm(t("reboot-node") + "?", name, () => {
      apiFetch(nodeApiUrl(i, "/reboot"), { method: "POST" })
          .then(r => r.json())
          .then(d => {
              if (d.ok) {
                  showToast(t("reboot-sent"));
              } else {
                  showToast(d.error || t("node-unauth"));
              }
          })
          .catch(() => showToast(t("node-unauth")));
  }, t("reboot-node"));
}
function shutdownNode(i) {
  const name = nodeNames[i] || nodes[i] || "";
  showConfirm(t("shutdown-node") + "?", name, () => {
      apiFetch(nodeApiUrl(i, "/shutdown"), { method: "POST" })
          .then(r => r.json())
          .then(d => {
              if (d.ok) {
                  showToast(t("shutdown-sent"));
              } else {
                  showToast(d.error || t("node-unauth"));
              }
          })
          .catch(() => showToast(t("node-unauth")));
  }, t("shutdown-node"));
}
function pingNode(i) {
  const name = nodeNames[i] || nodes[i] || "";
  const modal = document.getElementById("pingModal");
  document.getElementById("pingTitle").textContent = t("ping-node") + " - " + name;
  const out = document.getElementById("pingOutput");
  out.textContent = t("ping-running") + "...";
  const st = document.getElementById("pingStatus");
  if (st) { st.textContent = ""; st.className = "ping-status"; }
  modal.classList.add("show");
  apiFetch(nodeApiUrl(i, "/ping"), { method: "POST" })
      .then(r => r.json())
      .then(d => {
          if (d.output !== undefined) {
              out.textContent = d.output;
          } else if (d.error) {
              out.textContent = "ERROR: " + d.error;
          } else {
              out.textContent = JSON.stringify(d);
          }
          const status = document.getElementById("pingStatus");
          if (status) {
              if (d.ok === true) {
                  status.textContent = t("ping-ok") + " ✓";
                  status.className = "ping-status ping-ok";
              } else if (d.ok === false) {
                  status.textContent = t("ping-fail") + " ✗";
                  status.className = "ping-status ping-fail";
              } else {
                  status.textContent = "";
              }
          }
      })
      .catch(e => {
          out.textContent = "ERROR: " + e;
      });
}
function removeNode(i) {
  const name = nodes[i] || nodeNames[i] || "";
  showConfirm(t("node-removed") + "?", name, () => {
      const nid = nodeIds[i] || "";
      apiFetch("/api/node/id/" + encodeURIComponent(nid), { method: "DELETE" })
          .then(r => r.json())
          .then(d => {
              if (d.ok) {
                  showToast(t("node-removed"));
                  loadNodeList();
              } else {
                  showToast(d.error || t("node-unauth"));
              }
          })
          .catch(() => showToast(t("node-unauth")));
  });
}
function loadProcs() {
  const search = document.getElementById("procSearch").value.trim();
  const url = nodeApiUrl(activeNode, "/processes?sort=" + procSort + "&limit=" + procLimit + (search ? "&name=" + encodeURIComponent(search) : ""));
  apiFetch(url)
      .then(r => r.json())
      .then(d => {
          const body = document.getElementById("procBody");
          let html = "";
          // API may return a bare array (relay cache) or {processes:[...]}.
          const list = Array.isArray(d) ? d : (d.processes || []);
          list.forEach(p => {
              html += "<tr><td>" + p.pid + "</td><td class='proc-name'>" + escapeHtml(p.name) + "</td><td class='proc-cpu'>" + escapeHtml(p.cpu) + "%</td><td class='proc-rss'>" + escapeHtml(p.rss_mb) + " MB</td><td class='proc-state'>" + escapeHtml(p.state) + "</td></tr>";
          });
          body.innerHTML = html || "<tr><td colspan='5' style='text-align:center;color:var(--text-muted);'>--</td></tr>";
      })
      .catch(() => {});
}
function moreProcs() {
  procLimit += 20;
  loadProcs();
}
function loadDocker() {
  const body = document.getElementById("dockerBody");
  if (!body) return;
  body.innerHTML = "<div style='text-align:center;color:var(--text-muted);padding:30px'>" + t("docker-load") + "</div>";
  apiFetch(nodeApiUrl(activeNode, "/docker"))
      .then(r => r.json())
      .then(d => {
          const containers = d.containers || [];
          if (!containers.length) { body.innerHTML = "<div style='text-align:center;color:var(--text-muted);padding:30px'>" + t("docker-empty") + "</div>"; return; }
          body.innerHTML = `<table class="table docker-table">
              <thead><tr>
                  <th>${t("docker-name")}</th>
                  <th>${t("docker-image")}</th>
                  <th>${t("docker-state")}</th>
                  <th style="width:150px">${t("docker-title")}</th>
              </tr></thead>
              <tbody>` + containers.map(c => `
              <tr>
                  <td><b>${escapeHtml(c.name)}</b><br><small style="color:var(--text-muted)">${escapeHtml(c.short_id || "")}</small></td>
                  <td>${escapeHtml(c.image)}</td>
                  <td><span class="dot ${c.running ? "dot-green" : "dot-red"}"></span> ${c.running ? t("docker-running") : t("docker-exited")}<br><small style="color:var(--text-muted)">${escapeHtml(c.status)}</small></td>
                  <td>
                      <button class="btn-sm docker-act" data-name="${escapeHtml(c.name)}" data-action="logs">${t("docker-logs")}</button>
                      <button class="btn-sm docker-act" data-name="${escapeHtml(c.name)}" data-action="restart">${t("docker-restart")}</button>
                      ${c.running
                          ? `<button class="btn-sm docker-act" style="color:#dc2626" data-name="${escapeHtml(c.name)}" data-action="stop">${t("docker-stop")}</button>`
                          : `<button class="btn-sm docker-act" style="color:#16a34a" data-name="${escapeHtml(c.name)}" data-action="start">${t("docker-start")}</button>`}
                  </td>
              </tr>`).join("") + `</tbody></table>`;
          // Delegate docker actions: values come from data attributes so a
          // container name with quotes cannot break out of an inline handler.
          body.onclick = (ev) => {
              const btn = ev.target.closest(".docker-act");
              if (!btn) return;
              dockerAction(btn.dataset.name, btn.dataset.action, btn);
          };
      })
      .catch(() => {});
}
function dockerAction(name, action, btn) {
  if (!btn) return;
  const orig = btn.textContent;
  btn.disabled = true;
  btn.textContent = "...";
  apiFetch(nodeApiUrl(activeNode, "/docker/" + encodeURIComponent(name) + "/" + action), { method: "POST" })
      .then(r => r.json())
      .then(d => {
          btn.disabled = false;
          btn.textContent = orig;
          if (action === "logs") {
              // Show container logs in a dialog.
              const logs = d.result || d.error || "";
              showDockerLogs(name, logs);
              return;
          }
          if (d.ok || (d.error && !String(d.error).startsWith("HTTP"))) {
              if (d.ok) loadDocker();
              else alert(t("docker-op-fail") + ": " + d.error);
          } else {
              alert(t("docker-op-fail") + ": " + (d.error || ""));
          }
      })
      .catch(() => { btn.disabled = false; btn.textContent = orig; alert(t("docker-op-fail")); });
}
function showDockerLogs(name, logs) {
  const mask = document.createElement("div");
  mask.className = "modal-mask";
  mask.style.cssText = "position:fixed;inset:0;z-index:200;display:flex;align-items:center;justify-content:center;background:rgba(0,0,0,.45)";
  mask.innerHTML = `<div class="modal" style="max-width:720px;width:92%">
      <div class="modal-head"><span>${escapeHtml(name)} · ${t("docker-logs")}</span>
      <button class="modal-close" onclick="this.closest('.modal-mask').remove()">✕</button></div>
      <div class="modal-body" style="max-height:60vh;overflow:auto;white-space:pre-wrap;font-family:monospace;font-size:12px;line-height:1.5">${escapeHtml(logs || t("docker-logs-empty"))}</div>
  </div>`;
  mask.addEventListener("click", (e) => { if (e.target === mask) mask.remove(); });
  document.body.appendChild(mask);
}
function loadHistory() {
  if (!nodes.length) return;
  {
      const hurl = "/api/node/id/" + (nodeIds[activeNode] || "") + "/history?metric=" + histMetric + "&range=" + histRange;
      apiFetch(hurl)
          .then(r => { if (!r.ok) throw new Error("HTTP " + r.status + " " + hurl); return r.json(); })
          .then(d => {
              histData = d.points || [];
              if (histCompare && histCompareNode) {
                  // Node-vs-node comparison: fetch the second node's series.
                  const nid2 = nodeIds[nodes.findIndex(n => (typeof n === "string" ? n : n.name) === histCompareNode)] || "";
                  const curl = "/api/node/id/" + nid2 + "/history?metric=" + histMetric + "&range=" + histRange;
                  return apiFetch(curl).then(r => r.json()).then(c => { histCompareData = c.points || []; })
                      .catch(() => { histCompareData = []; });
              } else {
                  histCompareData = [];
              }
          })
          .then(() => drawHist())
          .catch(e => {
              // visible diagnostic instead of silent blank
              const canvas = document.getElementById("histChart");
              if (canvas) {
                  const ctx = canvas.getContext("2d");
                  const w = canvas.clientWidth || 300, h = 80;
                  ctx.clearRect(0, 0, w, h);
                  ctx.fillStyle = "#dc2626";
                  ctx.font = "12px sans-serif";
                  ctx.textAlign = "center";
                  ctx.fillText("history error: " + (e && e.message ? e.message : e), w / 2, h / 2);
              }
          });
  }
}
function exportHist() {
  const nid = nodeIds[activeNode] || "";
  const url = "/api/node/id/" + nid + "/history/export?metric=" + histMetric + "&range=" + histRange;
  apiFetch(url)
      .then(r => r.text())
      .then(csv => {
          const blob = new Blob([csv], { type: "text/csv" });
          const a = document.createElement("a");
          a.href = URL.createObjectURL(blob);
          a.download = "history_" + histMetric + "_" + histRange + ".csv";
          a.click();
          URL.revokeObjectURL(a.href);
      })
      .catch(() => {});
}