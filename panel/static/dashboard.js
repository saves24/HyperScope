
function refreshOverview() {
  updateOverview();
  updateDiskOverview();
  updateNetOverview();
  updateSpeed();
  updateTemp();
  updateTopProcs();
  updateTrend();
}
function setGauge(percent, fillId, needleId, valId) {
  if (typeof percent !== "number" || isNaN(percent)) return; // keep last shown value on bad data
  const pct = Math.max(0, Math.min(100, percent));
  const dash = (pct / 100) * GAUGE_CIRCUMFERENCE;
  const fill = document.getElementById(fillId);
  const needle = document.getElementById(needleId);
  const val = document.getElementById(valId);
  if (fill) {
      fill.style.strokeDasharray = dash + " " + GAUGE_CIRCUMFERENCE;
      fill.style.stroke = pct > 85 ? "#dc2626" : pct > 70 ? "#f59e0b" : "#2563eb";
  }
  if (needle) {
      const angle = -90 + (pct / 100) * 180;
      needle.setAttribute("transform", "rotate(" + angle + " 60 60)");
      needle.style.stroke = pct > 85 ? "#dc2626" : pct > 70 ? "#f59e0b" : "#2563eb";
  }
  if (val) val.textContent = pct + "%";
}
function setSortMode(m) {
  sortMode = m;
  document.querySelectorAll(".sort-btn").forEach(b => b.classList.toggle("active", b.dataset.sort === m));
  // update dropdown label (merged sort control)
  const lbl = document.getElementById("sortBtnLabel");
  if (lbl) {
      const i18nKey = m === "cpu" ? "sort-cpu" : m === "mem" ? "sort-mem" : "sort-default";
      lbl.textContent = t(i18nKey);
  }
  // reorder existing cards in place (no DOM rebuild, no data flash)
  const grid = document.getElementById("nodeGrid");
  const order = nodes.map((_, i) => i);
  if (m === "cpu" || m === "mem") {
      order.sort((a, b) => (nodeStats[b] && nodeStats[b][m] || 0) - (nodeStats[a] && nodeStats[a][m] || 0));
  }
  order.forEach(idx => {
      const card = document.getElementById("nodeCard_" + idx);
      if (card) grid.appendChild(card);
  });
}
// Merged sort dropdown (default/CPU/memory in one control)
function toggleSortDropdown(e) {
  if (e) e.stopPropagation();
  document.getElementById("sortDropdown").classList.toggle("show");
}
function pickSort(m) {
  setSortMode(m);
  document.getElementById("sortDropdown").classList.remove("show");
}
document.addEventListener("click", () => {
  const dd = document.getElementById("sortDropdown");
  if (dd) dd.classList.remove("show");
});
function updateNode(i) {
  if (i >= nodes.length) return;
  apiFetch(nodeApiUrl(i, "/system"))
      .then(r => r.json())
      .then(d => {
          if (d.status === "offline" || d.status === "unauthorized") throw { unauthorized: d.status === "unauthorized" };
          if (d.error) throw { keep: true }; // 503: no cached data yet, keep last shown values
          if (document.getElementById("nodeName_" + i)) document.getElementById("nodeName_" + i).textContent = nodeNames[i] || d.node_name || nodes[i];
          if (document.getElementById("nodeHostname_" + i) && d.node_name && d.node_name !== (nodeNames[i] || "")) document.getElementById("nodeHostname_" + i).textContent = d.node_name;
          if (document.getElementById("nodeCpuTemp_" + i)) document.getElementById("nodeCpuTemp_" + i).textContent = d.cpu_temp;
          if (document.getElementById("nodeGpuTemp_" + i)) document.getElementById("nodeGpuTemp_" + i).textContent = d.gpu_temp;
          if (document.getElementById("nodeCores_" + i)) document.getElementById("nodeCores_" + i).textContent = d.cpu_cores;
          if (document.getElementById("nodeProcs_" + i)) document.getElementById("nodeProcs_" + i).textContent = d.processes;
          if (document.getElementById("nodeLoad_" + i)) document.getElementById("nodeLoad_" + i).innerHTML = formatLoad(d.loadavg, d.cpu_cores, false);
          if (document.getElementById("nodeUptime_" + i)) document.getElementById("nodeUptime_" + i).textContent = d.uptime;
          if (document.getElementById("nodeUpdated_" + i)) {
              const t = new Date();
              document.getElementById("nodeUpdated_" + i).textContent = String(t.getHours()).padStart(2, "0") + ":" + String(t.getMinutes()).padStart(2, "0");
          }
          setGauge(d.cpu, "gaugeCpuFill_" + i, "gaugeCpuNeedle_" + i, "gaugeCpuVal_" + i);
          setGauge(d.mem_percent, "gaugeMemFill_" + i, "gaugeMemNeedle_" + i, "gaugeMemVal_" + i);
          failCount[i] = 0;
          nodeStats[i] = { cpu: d.cpu, mem: d.mem_percent };
      })
      .catch(err => {
          if (err && err.keep) return; // 503: transient, keep last values
          // transient network blips: keep last values; only show offline after 3 consecutive failures
          failCount[i] = (failCount[i] || 0) + 1;
          if (failCount[i] < 3) return;
          failCount[i] = 0;
          const nameEl = document.getElementById("nodeName_" + i);
          if (nameEl) nameEl.textContent = (err && err.unauthorized ? "🔒 " : "⚠ ") + (nodeNames[i] || nodes[i]) + " (" + (err && err.unauthorized ? t("node-unauth") : t("node-offline")) + ")";
          setGauge(0, "gaugeCpuFill_" + i, "gaugeCpuNeedle_" + i, "gaugeCpuVal_" + i);
          setGauge(0, "gaugeMemFill_" + i, "gaugeMemNeedle_" + i, "gaugeMemVal_" + i);
      });
}
function updateAllNodes() {
  nodes.forEach((_, i) => updateNode(i));
}
function updateOverview() {
  if (!nodes.length) return;
  apiFetch(nodeApiUrl(activeNode, "/system"))
      .then(r => r.json())
      .then(d => {
          // show data if present (offline keeps last known data); backend returns 503 {error} when empty
          if (d.error) {
              setText("ovKernel", "--");
              setText("ovVersion", "--");
              document.getElementById("ovLoad").innerHTML = "--";
              setText("ovUptime", "--");
              setText("ovProcs", "--");
              return;
          }
          setText("ovKernel", d.kernel || "--");
          setText("ovVersion", "v" + (d.version || "--"));
          document.getElementById("ovLoad").innerHTML = formatLoad(d.loadavg, d.cpu_cores);
          setText("ovUptime", d.uptime || "--");
          setText("ovProcs", d.processes || "--");
          updateDockerOverview();
      })
      .catch(() => {});
}
function updateDockerOverview() {
  const el = document.getElementById("dockerOverview");
  if (!el) return;
  apiFetch(nodeApiUrl(activeNode, "/docker"))
      .then(r => r.json())
      .then(d => {
          const containers = d.containers || [];
          if (!containers.length) { el.innerHTML = "<div style='color:var(--text-muted);font-size:12px'>" + t("docker-empty") + "</div>"; return; }
          const running = containers.filter(c => c.running).length;
          const total = containers.length;
          const pct = Math.round(running / total * 100);
          el.innerHTML = `<div style="font-size:20px;font-weight:600">${running}<span style="font-size:12px;color:var(--text-muted)"> / ${total}</span></div>
              <div style="font-size:11px;color:var(--text-muted);margin:2px 0 6px">${t("docker-running")}</div>
              <div style="height:4px;background:var(--border,#ddd);border-radius:2px;overflow:hidden">
                  <div style="height:100%;width:${pct}%;background:#2563eb;border-radius:2px"></div>
              </div>`;
      })
      .catch(() => {});
}
function updateDiskOverview() {
  if (!nodes.length) return;
  apiFetch(nodeApiUrl(activeNode, "/disks"))
      .then(r => r.json())
      .then(d => {
          const body = document.getElementById("diskOverview");
          if (!body) return;
          // API may return a bare array (relay cache) or {disks:[...]}.
          const disks = Array.isArray(d) ? d : (d.disks || []);
          if (!disks.length) { body.innerHTML = "<div class='empty-hint'>--</div>"; return; }
          body.innerHTML = disks.map(x => `
              <div class="ov-disk-row">
                  <span class="ov-disk-name">${escapeHtml(x.mount)}</span>
                  <span class="ov-disk-bar"><span class="ov-disk-fill" style="width:${x.percent}%;background:${x.percent > 90 ? "#dc2626" : x.percent > 75 ? "#f59e0b" : "#2563eb"}"></span></span>
                  <span class="ov-disk-val">${x.used_gb} / ${x.total_gb} GB (${x.percent}%)</span>
              </div>`).join("");
      })
      .catch(() => {});
}
function updateNetOverview() {
  if (!nodes.length) return;
  apiFetch(nodeApiUrl(activeNode, "/traffic"))
      .then(r => r.json())
      .then(d => {
          const body = document.getElementById("netOverview");
          if (!body) return;
          if (!d.ifaces || !d.ifaces.length) { body.innerHTML = "<div class='empty-hint'>--</div>"; return; }
          body.innerHTML = d.ifaces.map(x => `
              <div class="ov-net-row">
                  <span class="ov-net-name">${escapeHtml(x.name)}${x.name === d.iface ? " ●" : ""}</span>
                  <span class="ov-net-total">${t("net-rx")} ${fmtTotal(x.total_rx)} ${t("net-tx")} ${fmtTotal(x.total_tx)}</span>
              </div>`).join("");
      })
      .catch(() => {});
}
function updateTrend() {
  if (!nodes.length) return;
  apiFetch(nodeApiUrl(activeNode, "/system"))
      .then(r => r.json())
      .then(d => {
          if (d.status === "offline" || d.status === "unauthorized") return;
          trendCpu.push(parseFloat(d.cpu) || 0);
          trendMem.push(parseFloat(d.mem_percent) || 0);
          trendDisk.push(parseFloat(d.disk_percent) || 0);
          trendCpu.shift();
          trendMem.shift();
          trendDisk.shift();
          drawTrend("trendCpu", trendCpu, "#2563eb");
          drawTrend("trendMem", trendMem, "#eab308");
          drawTrend("trendDisk", trendDisk, "#10b981");
      })
      .catch(() => {});
}
function drawTrend(canvasId, data, color) {
  const canvas = document.getElementById(canvasId);
  if (!canvas) return;
  const ctx = canvas.getContext("2d");
  const w = canvas.getBoundingClientRect().width || canvas.clientWidth || 300, h = canvas.clientHeight || 60;
  canvas.width = w * 2;
  canvas.height = h * 2;
  ctx.scale(2, 2);
  ctx.clearRect(0, 0, w, h);
  if (data.length < 2) return;
  const pad = 4, maxV = 100;
  ctx.strokeStyle = color;
  ctx.lineWidth = 1.5;
  ctx.lineJoin = "round";
  ctx.beginPath();
  data.forEach((v, i) => {
      const x = pad + (i / (data.length - 1)) * (w - pad * 2);
      const y = h - pad - (Math.min(v, maxV) / maxV) * (h - pad * 2);
      if (i === 0) ctx.moveTo(x, y);
      else ctx.lineTo(x, y);
  });
  ctx.stroke();
  const grad = ctx.createLinearGradient(0, 0, 0, h);
  grad.addColorStop(0, color + "44");
  grad.addColorStop(1, color + "00");
  ctx.lineTo(w - pad, h - pad);
  ctx.lineTo(pad, h - pad);
  ctx.closePath();
  ctx.fillStyle = grad;
  ctx.fill();
}
function updateSpeed() {
  if (!nodes.length) return;
  apiFetch(nodeApiUrl(activeNode, "/traffic"))
      .then(r => r.json())
      .then(d => {
          if (d.error || d.status === "offline" || d.status === "unauthorized") {
              setText("speedRx", "--");
              setText("speedTx", "--");
              setText("speedIface", t("sp-iface") + ": --");
              return;
          }
          setText("speedRx", d.speed_rx_str || "--");
          setText("speedTx", d.speed_tx_str || "--");
          setText("speedIface", t("sp-iface") + ": " + (d.iface || "--"));
          const rx = parseFloat(d.speed_rx) || 0;
          const tx = parseFloat(d.speed_tx) || 0;
          trendNetRx.push(rx);
          trendNetTx.push(tx);
          trendNetRx.shift();
          trendNetTx.shift();
          drawNetTrend();
      })
      .catch(() => {});
}
function drawNetTrend() {
  const canvas = document.getElementById("trendNet");
  if (!canvas) return;
  const ctx = canvas.getContext("2d");
  const w = canvas.getBoundingClientRect().width || canvas.clientWidth || 300, h = canvas.clientHeight || 70;
  canvas.width = w * 2;
  canvas.height = h * 2;
  ctx.scale(2, 2);
  ctx.clearRect(0, 0, w, h);
  if (trendNetRx.length < 2) return;
  // auto-scale max rate
  const maxV = Math.max(1024, ...trendNetRx, ...trendNetTx) * 1.1;
  const pad = 4;
  drawLine(ctx, trendNetRx, w, h, pad, maxV, "#2563eb");
  drawLine(ctx, trendNetTx, w, h, pad, maxV, "#10b981");
}
function drawLine(ctx, data, w, h, pad, maxV, color) {
  ctx.strokeStyle = color;
  ctx.lineWidth = 1.5;
  ctx.lineJoin = "round";
  ctx.beginPath();
  data.forEach((v, i) => {
      const x = pad + (i / (data.length - 1)) * (w - pad * 2);
      const y = h - pad - (Math.min(v, maxV) / maxV) * (h - pad * 2);
      if (i === 0) ctx.moveTo(x, y);
      else ctx.lineTo(x, y);
  });
  ctx.stroke();
}
function updateTemp() {
  if (!nodes.length) return;
  apiFetch(nodeApiUrl(activeNode, "/system"))
      .then(r => r.json())
      .then(d => {
          if (d.error) {
              setText("tempCpu", "--");
              setText("tempGpu", "--");
              return;
          }
          setText("tempCpu", d.cpu_temp || "--");
          setText("tempGpu", d.gpu_temp || "--");
          const raw = parseFloat(d.cpu_temp_raw);
          if (!isNaN(raw)) {
              trendTemp.push(raw);
              trendTemp.shift();
              drawTempTrend();
          }
      })
      .catch(() => {});
}
function drawTempTrend() {
  const canvas = document.getElementById("trendTemp");
  if (!canvas) return;
  const ctx = canvas.getContext("2d");
  const w = canvas.getBoundingClientRect().width || canvas.clientWidth || 300, h = canvas.clientHeight || 50;
  canvas.width = w * 2;
  canvas.height = h * 2;
  ctx.scale(2, 2);
  ctx.clearRect(0, 0, w, h);
  if (trendTemp.length < 2) return;
  // temperature range auto-scale 40-90C
  const minT = 40, maxT = 90;
  const pad = 4;
  ctx.strokeStyle = "#ef4444";
  ctx.lineWidth = 1.5;
  ctx.lineJoin = "round";
  ctx.beginPath();
  trendTemp.forEach((v, i) => {
      const x = pad + (i / (trendTemp.length - 1)) * (w - pad * 2);
      const y = h - pad - (Math.min(Math.max(v, minT), maxT) - minT) / (maxT - minT) * (h - pad * 2);
      if (i === 0) ctx.moveTo(x, y);
      else ctx.lineTo(x, y);
  });
  ctx.stroke();
  const grad = ctx.createLinearGradient(0, 0, 0, h);
  grad.addColorStop(0, "#ef444444");
  grad.addColorStop(1, "#ef444400");
  ctx.lineTo(w - pad, h - pad);
  ctx.lineTo(pad, h - pad);
  ctx.closePath();
  ctx.fillStyle = grad;
  ctx.fill();
}
function updateTopProcs() {
  if (!nodes.length) return;
  apiFetch(nodeApiUrl(activeNode, "/processes?sort=cpu&limit=5"))
      .then(r => r.json())
      .then(d => {
          const body = document.getElementById("topProcs");
          if (!body) return;
          if (d.error || d.status === "offline" || d.status === "unauthorized") { body.innerHTML = "<div class='empty-hint'>--</div>"; return; }
          // API may return a bare array (relay cache) or {processes:[...]}.
          const procs = Array.isArray(d) ? d : (d.processes || []);
          if (!procs.length) { body.innerHTML = "<div class='empty-hint'>--</div>"; return; }
          body.innerHTML = procs.map(p => `
              <div class="ov-proc-row">
                  <span class="ov-proc-name" title="${escapeHtml(p.name)}">${escapeHtml(p.name)}</span>
                  <span class="ov-proc-cpu">${p.cpu}%</span>
                  <span class="ov-proc-mem">${p.rss_mb} MB</span>
              </div>`).join("");
      })
      .catch(() => {});
}
function updateNodeStats() {
  apiFetch("/api/nodes")
      .then(r => r.json())
      .then(d => {
          const body = document.getElementById("nodeStats");
          if (!body) return;
          const list = d.nodes || [];
          if (!list.length) { body.innerHTML = "<div class='empty-hint'>--</div>"; return; }
          body.innerHTML = list.map(n => `
              <div class="ov-node-row">
                  <span class="ov-node-name">${escapeHtml(n.node_name || n.name)}</span>
                  <span class="ov-node-ver">${escapeHtml(n.version ? "v" + n.version : "--")}</span>
                  <span class="ov-node-status ${n.online ? "ok" : "bad"}">${n.online ? "●" : "○"}</span>
              </div>`).join("");
      })
      .catch(() => {});
}
function updateKline() {
  if (!nodes.length) return;
  apiFetch(nodeApiUrl(activeNode, "/io"))
      .then(r => r.json())
      .then(d => {
          if (d.error) return;
          const el = document.getElementById("klineConns");
          if (el) el.textContent = d.tcp_conns || 0;
          klineRead.push(d.disk_read_mbs || 0);
          klineWrite.push(d.disk_write_mbs || 0);
          if (klineRead.length > KLINE_POINTS) klineRead.shift();
          if (klineWrite.length > KLINE_POINTS) klineWrite.shift();
          drawKline();
      })
      .catch(() => {});
}
function drawKline() {
  const canvas = document.getElementById("klineDisk");
  if (!canvas) return;
  const ctx = canvas.getContext("2d");
  const w = canvas.clientWidth || 300, h = canvas.clientHeight || 90;
  canvas.width = w * 2;
  canvas.height = h * 2;
  ctx.scale(2, 2);
  ctx.clearRect(0, 0, w, h);
  if (klineRead.length < 2) return;
  // auto-scale max rate (max of read/write)
  const maxV = Math.max(1, ...klineRead, ...klineWrite) * 1.2;
  const pad = 4;
  // grid lines
  ctx.strokeStyle = "rgba(128,128,128,0.15)";
  ctx.lineWidth = 1;
  for (let g = 1; g < 4; g++) {
      const y = pad + (h - pad * 2) * g / 4;
      ctx.beginPath(); ctx.moveTo(0, y); ctx.lineTo(w, y); ctx.stroke();
  }
  drawLine(ctx, klineRead, w, h, pad, maxV, "#ef4444");
  drawLine(ctx, klineWrite, w, h, pad, maxV, "#22c55e");
}