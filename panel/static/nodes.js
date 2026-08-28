


function toggleSettingsPanel() {
  document.getElementById("settingsPanel").classList.toggle("show");
}
function toggleSettingsGroup(el) {
  if (el.tagName !== "DIV") return;
  const body = el.querySelector(".settings-body, .settings-options");
  if (!body) return;
  el.classList.toggle("collapsed");
  const caret = el.querySelector(".group-caret");
  if (caret) caret.textContent = el.classList.contains("collapsed") ? "▸" : "▾";
}
function initSettingsGroups() {
  // admin check: backend /api/me provides is_admin
  apiFetch("/api/me")
      .then(r => r.json())
      .then(d => {
          const isAdmin = d && d.is_admin;
          document.querySelectorAll(".settings-group[data-admin-only]").forEach(g => {
              g.style.display = isAdmin ? "" : "none";
          });
      })
      .catch(() => {});
  // all groups expanded by default (click to collapse)
}
function checkPendingPort() {
  const pending = getCookie("ts-pending-port");
  if (!pending) return;
  const cur = location.port || (location.protocol === "https:" ? "443" : "80");
  if (String(pending) !== cur) {
      // probe new port; redirect once reachable
      const target = location.protocol + "//" + location.hostname + ":" + pending;
      fetch(target + "/api/status", { method: "GET" })
          .then(r => {
              if (r.ok) {
                  delCookie("ts-pending-port");
                  location.href = target;
              } else {
                  delCookie("ts-pending-port");
              }
          })
          .catch(() => {
              delCookie("ts-pending-port");
          });
  } else {
      delCookie("ts-pending-port");
  }
}
function selectTheme(mode) {
  applyTheme(mode);
}
function selectLang(lang) {
  setCookie("ts-lang", lang, 365);
  applyLang();
}
function loadBg() {
  const bg = localStorage.getItem("ts-bg");
  const opacity = getCookie("ts-bg-opacity");
  const layer = document.getElementById("bgLayer");
  const overlay = document.getElementById("bgOverlay");
  if (bg) layer.style.backgroundImage = "url(" + bg + ")";
  else layer.style.backgroundImage = "none";
  const op = opacity ? parseInt(opacity) : 65;
  document.documentElement.style.setProperty("--bg-opacity", (op / 100).toFixed(2));
  const slider = document.getElementById("bgOpacity");
  if (slider) slider.value = op;
  const label = document.getElementById("bgOpacityLabel");
  if (label) label.textContent = op + "%";
}
function handleBgFile(input) {
  const file = input.files[0];
  if (!file) return;
  const nameEl = document.getElementById("bgFileName");
  if (nameEl) nameEl.textContent = t("bg-file-selected") + ": " + file.name;
  const reader = new FileReader();
  reader.onload = function (e) {
      const img = new Image();
      img.onload = function () {
          const maxSize = 1920;
          let w = img.width, h = img.height;
          if (w > maxSize || h > maxSize) {
              const ratio = Math.min(maxSize / w, maxSize / h);
              w = Math.round(w * ratio);
              h = Math.round(h * ratio);
          }
          const canvas = document.createElement("canvas");
          canvas.width = w;
          canvas.height = h;
          const ctx = canvas.getContext("2d");
          ctx.drawImage(img, 0, 0, w, h);
          const dataUrl = canvas.toDataURL("image/jpeg", 0.8);
          try {
              localStorage.setItem("ts-bg", dataUrl);
              loadBg();
              document.getElementById("settingsPanel").classList.remove("show");
              showToast(t("bg-set"));
          } catch (err) {
              showToast(t("bg-too-large"));
          }
      };
      img.src = e.target.result;
  };
  reader.readAsDataURL(file);
  input.value = "";
}
function toggleCardTrans(checkbox) {
  document.body.classList.toggle("card-trans", checkbox.checked);
  setCookie("ts-card-trans", checkbox.checked ? "1" : "0", 365);
}
function loadCardTrans() {
  const on = getCookie("ts-card-trans") === "1";
  document.body.classList.toggle("card-trans", on);
  const cb = document.getElementById("cardTrans");
  if (cb) cb.checked = on;
}
function setBgOpacity(val) {
  const op = parseInt(val);
  document.documentElement.style.setProperty("--bg-opacity", (op / 100).toFixed(2));
  document.getElementById("bgOpacityLabel").textContent = op + "%";
  setCookie("ts-bg-opacity", op, 365);
}
function removeBg() {
  localStorage.removeItem("ts-bg");
  loadBg();
  document.getElementById("settingsPanel").classList.remove("show");
}
function selectNode(i) {
  if (i === activeNode) return;
  activeNode = i;
  localStorage.setItem("ts-active-node", nodes[i] || "");
  document.querySelectorAll(".node-card").forEach((c, idx) => c.classList.toggle("active", idx === i));
  refreshOverview();
  loadHistory(); // history card follows the selected node
}
function onNodeGridScroll() {
  // close node menu on scroll (fixed positioning breaks; keep open 200ms after open to avoid snap miscloses)
  const now = Date.now();
  document.querySelectorAll(".node-menu.show").forEach(m => {
      const openedAt = parseInt(m.dataset.openedAt || "0", 10);
      if (now - openedAt > 200) m.classList.remove("show");
  });
  clearTimeout(nodeScrollTimer);
  nodeScrollTimer = setTimeout(() => {
      const grid = document.getElementById("nodeGrid");
      const cards = grid.querySelectorAll(".node-card");
      if (!cards.length) return;
      const center = grid.scrollLeft + grid.clientWidth / 2;
      let best = 0;
      cards.forEach((c, i) => {
          const mid = c.offsetLeft + c.offsetWidth / 2;
          if (Math.abs(mid - center) < Math.abs(cards[best].offsetLeft + cards[best].offsetWidth / 2 - center)) {
              best = i;
          }
      });
      if (best !== activeNode) selectNode(best);
      // alignment via scroll-snap (proximity); no forced scroll to avoid loops
  }, 150);
}
function openAddNodeModal() {
  document.getElementById("addNodeModal").classList.add("show");
}
function openNodeManager() {
  nmTab("add");
  document.getElementById("nodeManagerModal").classList.add("show");
}
function nmTab(tab) {
  document.querySelectorAll(".nm-tab").forEach(b => b.classList.toggle("active", b.dataset.nmtab === tab));
  ["add","batch","del","export"].forEach(t => {
      const el = document.getElementById("nm" + t[0].toUpperCase() + t.slice(1));
      if (el) el.style.display = t === tab ? "" : "none";
  });
  if (tab === "del") loadNmDelList();
}
function batchAddNodes() {
  const text = (document.getElementById("batchNodes").value || "").trim();
  if (!text) { showToast(t("node-batch-empty")); return; }
  const lines = text.split(/\n+/).map(l => l.trim()).filter(Boolean);
  const results = { ok: 0, fail: 0 };
  let done = 0;
  lines.forEach(line => {
    const parts = line.split(",").map(s => s.trim());
    let addr = parts[0] || "";
    let port = 5000;
    if (addr.includes(":")) { const p = addr.split(":"); addr = p[0]; port = parseInt(p[1]) || 5000; }
    const key = parts[1] || "";
    const name = parts[2] || addr;
    const tls = key.includes("|");
    if (!addr || !key) { results.fail++; done++; maybeDone(); return; }
    apiFetch("/api/nodes", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ name, addr, port, key, tls })
    })
      .then(r => r.json())
      .then(d => { d && d.ok ? results.ok++ : results.fail++; })
      .catch(() => { results.fail++; })
      .finally(() => { done++; maybeDone(); });
  });
  function maybeDone() {
    if (done >= lines.length) {
      showToast(t("node-batch-result").replace("{ok}", results.ok).replace("{fail}", results.fail));
      document.getElementById("batchNodes").value = "";
      loadNodeList();
    }
  }
}
function loadNmDelList() {
  const box = document.getElementById("nmDelList");
  apiFetch("/api/nodes")
      .then(r => r.json())
      .then(d => {
          const list = d.nodes || [];
          if (!list.length) { box.innerHTML = "<div class='empty-hint' style='padding:20px;text-align:center;color:var(--text-muted)'>--</div>"; return; }
          box.innerHTML = list.map((n, i) => `
              <label class="nm-del-item">
                  <input type="checkbox" class="nm-del-check" data-idx="${i}" data-id="${escapeHtml(n.id || "")}">
                  <span class="node-list-dot ${n.status === "online" || n.status === "ok" ? "ok" : "bad"}"></span>
                  <b>${escapeHtml(displayName(n))}</b>
                  ${n.node_name && n.node_name !== displayName(n) ? `<span class="node-list-host">${escapeHtml(n.node_name)}</span>` : ""}
                  <span style="margin-left:auto;color:var(--text-muted);font-size:11px;">v${escapeHtml(n.version || "--")}</span>
              </label>`).join("");
      })
      .catch(() => {});
}
function batchDelNodes() {
  const checks = document.querySelectorAll(".nm-del-check:checked");
  if (!checks.length) { showToast(t("node-batch-none")); return; }
  const ids = [...checks].map(c => c.dataset.id);
  showConfirm(t("node-batch-del-confirm") + " (" + ids.length + ")", "", () => {
    let done = 0; let removed = 0;
    ids.forEach(id => {
      apiFetch("/api/node/id/" + encodeURIComponent(id), { method: "DELETE" })
        .then(r => r.json())
        .then(d => { if (d && d.ok) removed++; })
        .catch(() => {})
        .finally(() => { done++; if (done >= ids.length) { loadNmDelList(); loadNodeList(); showToast(t("node-removed") + " x" + removed); } });
    });
  }, t("confirm-del"));
}
function openNodeListModal() {
  apiFetch("/api/nodes")
      .then(r => r.json())
      .then(d => {
          const list = d.nodes || [];
          const body = document.getElementById("nodeListBody");
          if (!list.length) {
              body.innerHTML = "<div style='text-align:center;color:var(--text-muted);padding:30px;'>--</div>";
          } else {
              body.innerHTML = list.map((n, i) => `
                  <div class="node-list-item ${i === activeNode ? "active" : ""}" onclick="selectNodeFromList(${i})">
                      <span class="node-list-dot ${n.status === "online" || n.status === "ok" ? "ok" : "bad"}"></span>
                      <b>${escapeHtml(displayName(n))}</b>
                      ${n.node_name && n.node_name !== displayName(n) ? `<span class="node-list-host">${escapeHtml(n.node_name)}</span>` : ""}
                      ${n.owner ? `<span class="node-owner-tag">@${escapeHtml(n.owner)}</span>` : ""}
                      <span class="node-list-ver">v${escapeHtml(n.version || "--")}</span>
                  </div>`).join("");
          }
          document.getElementById("nodeListModal").classList.add("show");
      })
      .catch(() => {});
}
function selectNodeFromList(i) {
  selectNode(i);
  document.getElementById("nodeListModal").classList.remove("show");
  // scroll horizontal area to the matching card
  const cards = document.querySelectorAll("#nodeGrid .node-card");
  if (cards[i]) cards[i].scrollIntoView({ behavior: "smooth", inline: "start", block: "nearest" });
}
function onAddKeyInput() {
  const v = document.getElementById("addNodeKey").value.trim();
  const tlsBox = document.getElementById("addNodeTls");
  if (!tlsBox) return;
  if (v.includes("|")) {
      tlsBox.checked = true;
      tlsBox.disabled = true;
  } else {
      tlsBox.disabled = false;
  }
}
function renameConfirm() {
  const i = renameTargetIdx;
  const newName = document.getElementById("renameInput").value.trim();
  if (!newName) return;
  const old = nodeNames[i] || nodes[i] || "";
  if (newName === old) { closeRenameModal(); return; }
  apiFetch(nodeApiUrl(i, "/name"), {
      method: "PUT",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ name: newName })
  })
      .then(r => r.json())
      .then(d => {
          if (d.ok) {
              showToast(t("node-renamed"));
              closeRenameModal();
              loadNodeList();
          } else {
              showToast(d.error || t("node-unauth"));
          }
      })
      .catch(() => showToast(t("node-unauth")));
}
function closeRenameModal() {
  document.getElementById("renameModal").classList.remove("show");
}
function closePingModal() {
  document.getElementById("pingModal").classList.remove("show");
}
function loadNodeList() {
  apiFetch("/api/nodes")
      .then(r => r.json())
      .then(d => {
          const list = d.nodes || [];
          // re-render when node count/name changes (compare display names to avoid pure-IP false positives)
          if (list.length !== nodes.length || list.some((n, i) => displayName(n) !== nodeNames[i])) {
              nodes = list.map(n => n.name);              nodeIds = list.map(n => n.id || "");
              nodeNames = list.map(n => displayName(n));
              nodeHostnames = list.map(n => (n.node_name && n.node_name !== displayName(n)) ? n.node_name : "");
              nodeOwners = list.map(n => n.owner || "");
              nodeTls = list.map(n => ({ tls: !!n.tls, verified: n.cert_verified !== false }));
              // selection: restore last → first online → first overall
              const saved = localStorage.getItem("ts-active-node");
              let target = -1;
              if (saved) {
                  const idx = nodes.indexOf(saved);
                  if (idx >= 0) target = idx;
              }
              if (target < 0) {
                  target = list.findIndex(n => n.status === "online" || n.status === "ok");
              }
              if (target < 0) target = 0;
              activeNode = Math.min(target, nodes.length - 1);
              localStorage.setItem("ts-active-node", nodes[activeNode] || "");
              renderNodes();
              refreshOverview();
              loadHistory(); // ensure history card draws after node list is ready
          }
      })
      .catch(() => {});
}
function renderNodes() {
  const grid = document.getElementById("nodeGrid");
  // remove menus attached to body before rebuilding cards (avoid id conflicts)
  document.querySelectorAll("body > .node-menu").forEach(m => m.remove());
  if (!nodes.length) {
      grid.innerHTML = "<div class='empty-hint' style='grid-column:1/-1;text-align:center;color:var(--text-muted);padding:30px;'>" + t("nodes") + " --</div>";
      return;
  }
  grid.innerHTML = nodes.map((name, i) => `
      <div class="node-card ${i === activeNode ? "active" : ""}" id="nodeCard_${i}" onclick="selectNode(${i})">
          <div class="node-top">
              <span class="node-name" id="nodeName_${i}">${escapeHtml(nodeNames[i] || name)}</span>
              ${nodeTls[i] && nodeTls[i].tls ? (nodeTls[i].verified ? `<span class="node-tls-tag" title="TLS encrypted">TLS</span>` : `<span class="node-tls-tag unverified" title="TLS unverified certificate">TLS?</span>`) : ""}
              ${nodeOwners[i] ? `<span class="node-owner-tag" title="${escapeHtml(nodeOwners[i])}">@${escapeHtml(nodeOwners[i])}</span>` : ""}
              <div class="btn-wrap">
                  <button class="node-menu-btn" onclick="event.stopPropagation();toggleNodeMenu(${i})">
                      <svg viewBox="0 0 24 24" fill="currentColor"><path d="M12 8a2 2 0 1 0 0-4 2 2 0 0 0 0 4zm0 2a2 2 0 1 0 0 4 2 2 0 0 0 0-4zm0 6a2 2 0 1 0 0 4 2 2 0 0 0 0-4z"/></svg>
                  </button>
                  <div class="menu node-menu" id="nodeMenu_${i}">
                      <div class="menu-item" onclick="closeMenu(${i});renameNode(${i})" data-i18n="rename-node">重命名</div>
                      <div class="menu-item" onclick="closeMenu(${i});pingNode(${i})" data-i18n="ping-node">Ping</div>
                      <div class="menu-item" onclick="closeMenu(${i});openProcModal(${i})" data-i18n="proc-btn">进程</div>
                      <div class="menu-item" onclick="closeMenu(${i});openDiskModal(${i})" data-i18n="disk-btn">磁盘</div>
<div class="menu-item" onclick="closeMenu(${i});openDockerModal(${i})" data-i18n="docker-title">Docker</div>
                      <div class="menu-item" style="color:#f59e0b;" onclick="closeMenu(${i});rebootNode(${i})" data-i18n="reboot-node">重启</div>
                      <div class="menu-item" style="color:#dc2626;" onclick="closeMenu(${i});shutdownNode(${i})" data-i18n="shutdown-node">关机</div>
                      <div class="menu-item" style="color:#dc2626;" onclick="closeMenu(${i});removeNode(${i})" data-i18n="remove-node">删除节点</div>
                  </div>
              </div>
          </div>
          <div class="node-hostname" id="nodeHostname_${i}">${nodeHostnames[i] ? escapeHtml(nodeHostnames[i]) : ""}</div>
          <div class="node-gauges">
              <div class="gauge-wrap">
                  <div class="gauge-svg">
                      <svg viewBox="0 0 120 70">
                          <path class="gauge-track" d="M15 60 A45 45 0 0 1 105 60"/>
                          <path class="gauge-fill" id="gaugeCpuFill_${i}" d="M15 60 A45 45 0 0 1 105 60"/>
                          <line class="gauge-needle" id="gaugeCpuNeedle_${i}" x1="60" y1="60" x2="60" y2="22"/>
                          <circle class="gauge-center" cx="60" cy="60" r="4"/>
                      </svg>
                  </div>
                  <div class="gauge-label"><span data-i18n="cpu-usage">CPU</span> <b id="gaugeCpuVal_${i}">--%</b></div>
              </div>
              <div class="gauge-wrap">
                  <div class="gauge-svg">
                      <svg viewBox="0 0 120 70">
                          <path class="gauge-track" d="M15 60 A45 45 0 0 1 105 60"/>
                          <path class="gauge-fill" id="gaugeMemFill_${i}" d="M15 60 A45 45 0 0 1 105 60"/>
                          <line class="gauge-needle" id="gaugeMemNeedle_${i}" x1="60" y1="60" x2="60" y2="22"/>
                          <circle class="gauge-center" cx="60" cy="60" r="4"/>
                      </svg>
                  </div>
                  <div class="gauge-label"><span data-i18n="memory">内存</span> <b id="gaugeMemVal_${i}">--%</b></div>
              </div>
          </div>
          <div class="node-info">
              <div class="node-info-item"><span data-i18n="cpu-temp">CPU 温度</span>: <b id="nodeCpuTemp_${i}">--</b></div>
              <div class="node-info-item"><span data-i18n="gpu-temp">GPU 温度</span>: <b id="nodeGpuTemp_${i}">--</b></div>
              <div class="node-info-item"><span data-i18n="cpu-cores">核心</span>: <b id="nodeCores_${i}">--</b></div>
              <div class="node-info-item"><span data-i18n="processes">进程数</span>: <b id="nodeProcs_${i}">--</b></div>
              <div class="node-info-item node-load-item"><span data-i18n="loadavg">负载</span>: <b id="nodeLoad_${i}">--</b></div>
              <div class="node-info-item"><span data-i18n="uptime">运行时间</span>: <b id="nodeUptime_${i}">--</b></div>
              <div class="node-info-item"><span data-i18n="updated">更新于</span>: <b id="nodeUpdated_${i}">--</b></div>
          </div>
      </div>
  `).join("");
  applyLang();
  // horizontal swipe to switch nodes
  const gridEl = document.getElementById("nodeGrid");
  gridEl.removeEventListener("scroll", onNodeGridScroll);
  gridEl.addEventListener("scroll", onNodeGridScroll, { passive: true });
}
function toggleNodeMenu(i) {
  // close all menus; skip reopen if clicking the open one
  const menu = document.getElementById("nodeMenu_" + i);
  const wasOpen = menu.classList.contains("show");
  document.querySelectorAll(".node-menu").forEach(m => m.classList.remove("show"));
  if (wasOpen) return;
  // find button inside card (menu may be moved to body; parentElement unreliable)
  const btn = document.querySelector("#nodeCard_" + i + " .node-menu-btn");
  if (!btn) return;
  const rect = btn.getBoundingClientRect();
  // don't open only when button fully off-screen
  if (rect.right <= 0 || rect.left >= window.innerWidth || rect.bottom <= 0 || rect.top >= window.innerHeight) {
      return;
  }
  // attach menu to body to avoid card container clipping
  document.body.appendChild(menu);
  menu.classList.add("show");
  menu.dataset.openedAt = String(Date.now());
  menu.style.zIndex = "9999";
  // dynamic position: fixed relative to viewport, below button
  const mw = 130;
  let left = rect.right - mw;
  // clamp within viewport
  if (left + mw > window.innerWidth - 8) left = window.innerWidth - mw - 8;
  if (left < 8) left = 8;
  let top = rect.bottom + 8;
  if (top + menu.offsetHeight > window.innerHeight - 8) top = rect.top - menu.offsetHeight - 8;
  menu.style.position = "fixed";
  menu.style.top = top + "px";
  menu.style.left = left + "px";
  menu.style.right = "auto";
  menu.style.bottom = "auto";
  menu.style.transform = "none";
}
function closeMenu(i) {
  document.getElementById("nodeMenu_" + i).classList.remove("show");
}
function openModal(id) {
  document.getElementById(id).classList.add("show");
}
function closeModal(id, event) {
  if (event && event.target !== event.currentTarget) return;
  document.getElementById(id).classList.remove("show");
  if (id === "procModal") clearInterval(procTimer);
}
function openProcModal(i) {
  activeNode = i;
  openModal("procModal");
  loadProcs();
  clearInterval(procTimer);
  procTimer = setInterval(loadProcs, 5000);
}
function openDiskModal(i) {
  activeNode = i;
  openModal("diskModal");
  apiFetch(nodeApiUrl(activeNode, "/disks"))
      .then(r => r.json())
      .then(d => {
          const body = document.getElementById("diskBody");
          const disks = d.disks || [];
          if (!disks.length) { body.innerHTML = "--"; return; }
          let html = "<table class='disk-table'><thead><tr><th></th><th>" + t("disk-used") + "</th><th>" + t("disk-free") + "</th><th>" + t("disk-total") + "</th></tr></thead><tbody>";
          disks.forEach(disk => {
              const pct = Math.min(100, disk.percent);
              const color = disk.percent > 90 ? "#dc2626" : disk.percent > 75 ? "#f59e0b" : "#2563eb";
              const inodePct = disk.inode_pct || 0;
              html += "<tr><td class='disk-mount'><b>" + escapeHtml(disk.mount) + "</b> <span class='disk-fstype'>" + escapeHtml(disk.fs_type || "") + "</span>"
                  + "<div class='bar'><div class='bar-fill' style='width:" + pct + "%;background:" + color + "'></div></div>"
                  + "<span class='disk-pct' style='color:" + color + "'>" + disk.percent + "%</span>"
                  + "</td>"
                  + "<td>" + disk.used_gb + " GB</td>"
                  + "<td>" + (disk.free_gb || "--") + " GB</td>"
                  + "<td>" + disk.total_gb + " GB</td></tr>";
              // inode row
              html += "<tr class='disk-inode'><td colspan='4'>" + t("disk-inode") + ": "
                  + "<span class='disk-inode-val'>" + disk.inodes_used + " / " + disk.inodes_total + "</span> ("
                  + "<span style='color:" + (inodePct > 80 ? "#dc2626" : "#2563eb") + "'>" + inodePct + "%</span>)</td></tr>";
          });
          html += "</tbody></table>";
          body.innerHTML = html;
      })
      .catch(() => {});
}
function openDockerModal(i) {
  activeNode = i;
  openModal("dockerModal");
  loadDocker();
}

// ===== Node config export / import (.hsxc, AES-256-GCM + PBKDF2, done locally) =====
const HSX_ITER = 200000;
// ==== .hsxc codec backed by node-forge (works over plain HTTP, no WebCrypto needed) ====
// File layout: "HSX1" + salt(16) + iv(12) + AES-256-GCM ciphertext.
function hsxDeriveKey(pass, salt) {
  return forge.pkcs5.pbkdf2(pass, salt, HSX_ITER, 32); // 32 bytes = AES-256
}
function hsxEncrypt(pass, payloadObj) {
  const salt = forge.random.getBytesSync(16);
  const iv = forge.random.getBytesSync(12);
  const key = hsxDeriveKey(pass, salt);
  const cipher = forge.cipher.createCipher("AES-GCM", key);
  cipher.start({ iv: iv, tagLength: 128 });
  cipher.update(forge.util.createBuffer(JSON.stringify(payloadObj)));
  cipher.finish();
  const tag = cipher.mode.tag.getBytes();
  // File layout: HSX1(4) + salt(16) + iv(12) + ciphertext + tag(16)
  const bin = "HSX1" + salt + iv + cipher.output.getBytes() + tag;
  const out = new Uint8Array(bin.length);
  for (let i = 0; i < bin.length; i++) out[i] = bin.charCodeAt(i) & 0xff;
  return out;
}
function hsxDecrypt(pass, fileBytes) {
  const data = new Uint8Array(fileBytes);
  const bytes = String.fromCharCode.apply(null, data);
  if (bytes.length < 32 || bytes.slice(0, 4) !== "HSX1") {
      throw new Error(t("node-import-invalid"));
  }
  const salt = bytes.slice(4, 20);
  const iv = bytes.slice(20, 32);
  const ct = bytes.slice(32, bytes.length - 16);
  const tag = bytes.slice(bytes.length - 16);
  const key = hsxDeriveKey(pass, salt);
  const decipher = forge.cipher.createDecipher("AES-GCM", key);
  decipher.start({ iv: iv, tagLength: 128, tag: forge.util.createBuffer(tag) });
  decipher.update(forge.util.createBuffer(ct));
  try {
      const ok = decipher.finish();
      if (!ok) throw new Error(t("node-import-badpass"));
      return JSON.parse(decipher.output.toString());
  } catch (e) {
      throw new Error(t("node-import-badpass"));
  }
}

function exportNodes() {
  const pass = document.getElementById("exportPass").value;
  if (!pass) { showToast(t("node-export-pass-need")); return; }
  apiFetch("/api/nodes")
      .then(r => r.json())
      .then(d => {
          const list = d.nodes || [];
          if (!list.length) { showToast(t("node-export-empty")); return; }
          const payload = { nodes: list.map(n => ({ name: n.name, addr: n.addr || n.address, port: n.port, key: n.key, tls: !!n.tls })) };
          const out = hsxEncrypt(pass, payload);
          const blob = new Blob([out], { type: "application/octet-stream" });
          const a = document.createElement("a");
          a.href = URL.createObjectURL(blob);
          a.download = "hyper-nodes-" + new Date().toISOString().slice(0,10) + ".hsxc";
          a.click();
          URL.revokeObjectURL(a.href);
          showToast(t("node-export-ok"));
          document.getElementById("exportPass").value = "";
      })
      .catch(() => showToast(t("op-err")));
}

function onImportFileSelected(input) {
  input._file = input.files && input.files[0];
  const label = document.querySelector("label.nm-file span");
  if (label && input._file) label.textContent = input._file.name;
}

function importFromFile() {
  const input = document.getElementById("importFile");
  const file = input && input._file;
  const pass = document.getElementById("importPass").value;
  if (!file) { showToast(t("node-import-nofile")); return; }
  if (!pass) { showToast(t("node-import-pass-need")); return; }
  const reader = new FileReader();
  reader.onload = async () => {
      try {
          const payload = hsxDecrypt(pass, reader.result);
          const nodes = payload.nodes || [];
          if (!nodes.length) { showToast(t("node-import-empty")); return; }
          const results = { ok: 0, fail: 0 };
          let done = 0;
          nodes.forEach(n => {
              const key = n.key || "";
              apiFetch("/api/nodes", {
                  method: "POST",
                  headers: { "Content-Type": "application/json" },
                  body: JSON.stringify({ name: n.name || n.addr, addr: n.addr, port: n.port || 5000, key, tls: key.includes("|") ? true : !!n.tls })
              })
                .then(r => r.json())
                .then(d => { d && d.ok ? results.ok++ : results.fail++; })
                .catch(() => { results.fail++; })
                .finally(() => { done++; if (done >= nodes.length) { finishImport(results); } });
          });
          function finishImport(res) {
              showToast(t("node-batch-result").replace("{ok}", res.ok).replace("{fail}", res.fail));
              document.getElementById("importPass").value = "";
              input.value = ""; input._file = null;
              loadNodeList();
          }
      } catch (e) {
          showToast(e.message || t("op-err"));
      }
  };
  reader.readAsArrayBuffer(file);
}