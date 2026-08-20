


function loadUsers() {
  apiFetch("/api/users")
      .then(r => {
          if (r.status === 403) {
              // non-admin: hide user management group
              const groups = document.querySelectorAll(".settings-group");
              groups.forEach(g => {
                  if (g.querySelector("[data-i18n='users-title']")) g.style.display = "none";
              });
              return null;
          }
          return r.json();
      })
      .then(d => {
          if (!d) return;
          const list = document.getElementById("userList");
          if (!list) return;
          if (!d.users || !d.users.length) { list.innerHTML = "<div class='empty-hint'>--</div>"; return; }
          list.innerHTML = d.users.map(u => `
              <div class="user-item">
                  <b>${escapeHtml(u.name)}</b>${u.is_admin ? " <span class='user-admin'>admin</span>" : ""}
                  <div class="user-actions">
                      <button onclick="userPasswd('${escapeHtml(u.name)}')" data-i18n="users-passwd">改密</button>
                      <button onclick="userRename('${escapeHtml(u.name)}')" data-i18n="users-rename">改名</button>
                      ${u.is_admin ? "" : `<button style="color:#dc2626" onclick="delUser('${escapeHtml(u.name)}')" data-i18n="users-del">删除</button>`}
                  </div>
              </div>`).join("");
          applyLang();
      })
      .catch(() => {});
}
function addUser() {
  const user = document.getElementById("newUserName").value.trim();
  const pass = document.getElementById("newUserPass").value;
  if (!user || !pass) { showToast(t("users-err-empty")); return; }
  apiFetch("/api/users", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ user, pass })
  })
      .then(r => r.json())
      .then(d => {
          if (d.ok) {
              showToast(t("node-added"));
              document.getElementById("newUserName").value = "";
              document.getElementById("newUserPass").value = "";
              loadUsers();
          } else {
              showToast(d.error || t("op-err"));
          }
      })
      .catch(() => {});
}
function delUser(name) {
  if (!confirm(t("confirm-del") + ": " + name)) return;
  apiFetch("/api/users/" + encodeURIComponent(name), { method: "DELETE" })
      .then(r => r.json())
      .then(d => {
          if (d.ok) { showToast(t("node-removed")); loadUsers(); }
          else showToast(d.error || t("op-err"));
      })
      .catch(() => {});
}
function userPasswd(name) {
  const np = prompt(t("users-passwd-prompt") + " (" + name + ")");
  if (np === null) return;
  apiFetch("/api/users/" + encodeURIComponent(name), {
      method: "PUT",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ pass: np })
  })
      .then(r => r.json())
      .then(d => {
          if (d.ok) { showToast(t("saved")); loadUsers(); }
          else showToast(d.error || t("op-err"));
      })
      .catch(() => {});
}
function userRename(name) {
  const nn = prompt(t("users-rename-prompt") + " (" + name + ")");
  if (nn === null) return;
  apiFetch("/api/users/" + encodeURIComponent(name), {
      method: "PUT",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ new_name: nn })
  })
      .then(r => r.json())
      .then(d => {
          if (d.ok) { showToast(t("saved")); loadUsers(); }
          else showToast(d.error || t("op-err"));
      })
      .catch(() => {});
}
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
  apiFetch("/api/nodes", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ name, addr, port, key, tls })
  })
      .then(r => r.json())
      .then(d => {
          if (d.ok) {
              document.getElementById("addNodeModal").classList.remove("show");
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
          if (d.ok) {
              status.textContent = t("ping-ok") + " ✓";
              status.className = "ping-status ping-ok";
          } else if (d.ok === false) {
              status.textContent = t("ping-fail") + " ✗";
              status.className = "ping-status ping-fail";
          } else {
              status.textContent = "";
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
          (d.processes || []).forEach(p => {
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
                      <button class="btn-sm" onclick="dockerAction('${escapeHtml(c.name)}','restart',this)">${t("docker-restart")}</button>
                      ${c.running
                          ? `<button class="btn-sm" style="color:#dc2626" onclick="dockerAction('${escapeHtml(c.name)}','stop',this)">${t("docker-stop")}</button>`
                          : `<button class="btn-sm" style="color:#16a34a" onclick="dockerAction('${escapeHtml(c.name)}','start',this)">${t("docker-start")}</button>`}
                  </td>
              </tr>`).join("") + `</tbody></table>`;
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
          if (d.ok || (d.error && !String(d.error).startsWith("HTTP"))) {
              if (d.ok) loadDocker();
              else alert(t("docker-op-fail") + ": " + d.error);
          } else {
              alert(t("docker-op-fail") + ": " + (d.error || ""));
          }
      })
      .catch(() => { btn.disabled = false; btn.textContent = orig; alert(t("docker-op-fail")); });
}
function loadHistory() {
  if (!nodes.length) return;
  {
      const hurl = "/api/node/id/" + (nodeIds[activeNode] || "") + "/history?metric=" + histMetric + "&range=" + histRange;
      apiFetch(hurl)
          .then(r => { if (!r.ok) throw new Error("HTTP " + r.status + " " + hurl); return r.json(); })
          .then(d => {
              histData = d.points || [];
              drawHist();
          })
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