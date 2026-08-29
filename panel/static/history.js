
function setHistRange(r) {
  histRange = r;
  document.querySelectorAll(".hist-range").forEach(b => b.classList.toggle("active", b.dataset.range === r));
  loadHistory();
}
function setHistMetric(m) {
  histMetric = m;
  document.querySelectorAll(".hist-metric").forEach(b => b.classList.toggle("active", b.dataset.metric === m));
  loadHistory();
}
function toggleHistCompare() {
  // Open a full compare dialog: left node and right node, each with its own
  // chart, side by side. Node pickers are in-page dropdowns (no native select).
  const list = nodes.map(n => typeof n === "string" ? n : n.name);
  if (list.length < 2) { showToast(t("hist-compare-need2")); return; }
  let left = list[activeNode] || list[0];
  let right = histCompareNode && list.includes(histCompareNode) ? histCompareNode : (list[0] !== left ? list[0] : list[1]);
  const mask = document.createElement("div");
  mask.className = "modal-mask";
  mask.style.cssText = "position:fixed;inset:0;z-index:200;display:flex;align-items:center;justify-content:center;background:rgba(0,0,0,.45)";
  mask.innerHTML = `<div class="modal" style="max-width:900px;width:95%;max-height:88vh;display:flex;flex-direction:column">
      <div class="modal-head">
          <span>${t("hist-compare")}</span>
          <button class="modal-close" onclick="window._cmpMask=null;this.closest('.modal-mask').remove()">✕</button>
      </div>
      <div class="modal-body" style="flex:1;overflow:auto;padding:16px">
          <div style="display:flex;gap:4px;font-size:11px;margin-bottom:12px;flex-wrap:wrap">
              <button class="hist-metric cmp-metric ${histMetric === "cpu" ? "active" : ""}" data-metric="cpu" onclick="setCmpMetric('cpu')">CPU</button>
              <button class="hist-metric cmp-metric ${histMetric === "mem" ? "active" : ""}" data-metric="mem" onclick="setCmpMetric('mem')">MEM</button>
              <button class="hist-metric cmp-metric ${histMetric === "disk" ? "active" : ""}" data-metric="disk" onclick="setCmpMetric('disk')">DISK</button>
              <button class="hist-metric cmp-metric ${histMetric === "temp" ? "active" : ""}" data-metric="temp" onclick="setCmpMetric('temp')">TEMP</button>
              <button class="hist-metric cmp-metric ${histMetric === "net_rx" ? "active" : ""}" data-metric="net_rx" onclick="setCmpMetric('net_rx')">NET</button>
          </div>
          <div style="display:grid;grid-template-columns:1fr 1fr;gap:16px">
              <div>
                  <span class="add-field" style="position:relative;display:block"><span>${t("hist-compare-left")}</span>
                      <div class="cmp-picker" id="cmpLeftPicker" onclick="event.stopPropagation();toggleCmpMenu('cmpLeftMenu')">
                          <span id="cmpLeftLabel"></span><span style="margin-left:auto">▾</span>
                      </div>
                      <div class="cmp-menu" id="cmpLeftMenu" onclick="event.stopPropagation()"></div>
                  </span>
                  <canvas id="cmpChartLeft" style="width:100%;height:200px;margin-top:8px"></canvas>
              </div>
              <div>
                  <span class="add-field" style="position:relative;display:block"><span>${t("hist-compare-right")}</span>
                      <div class="cmp-picker" id="cmpRightPicker" onclick="event.stopPropagation();toggleCmpMenu('cmpRightMenu')">
                          <span id="cmpRightLabel"></span><span style="margin-left:auto">▾</span>
                      </div>
                      <div class="cmp-menu" id="cmpRightMenu" onclick="event.stopPropagation()"></div>
                  </span>
                  <canvas id="cmpChartRight" style="width:100%;height:200px;margin-top:8px"></canvas>
              </div>
          </div>
      </div>
  </div>`;
  document.body.appendChild(mask);
  window._cmpMask = mask;
  // populate in-page menus
  const cmpState = { left, right };
  mask._cmpState = cmpState;
  for (const side of ["left", "right"]) {
      const menu = mask.querySelector("#cmp" + side[0].toUpperCase() + side.slice(1) + "Menu");
      menu.innerHTML = list.map(n =>
          `<div class="cmp-menu-item" onclick="event.stopPropagation();pickCmpNode('${side}', this)">${escapeHtml(n)}</div>`).join("");
      const label = mask.querySelector("#cmp" + side[0].toUpperCase() + side.slice(1) + "Label");
      label.textContent = (side === "left" ? left : right);
  }
  // close menu when clicking outside (bound once, not per open)
  if (!window._cmpCloseBound) {
      window._cmpCloseBound = true;
      document.addEventListener("click", closeCmpMenus);
  }
  histCompare = true;
  const cbtn = document.getElementById("histCompareBtn");
  if (cbtn) cbtn.classList.add("active");
  renderCompare();
}

function toggleCmpMenu(menuId) {
  closeCmpMenus();
  const menu = document.getElementById(menuId);
  if (menu) menu.classList.toggle("show");
}
/** Switches the metric shown in the compare dialog (independent of history card). */
function setCmpMetric(m) {
  histMetric = m;
  document.querySelectorAll(".cmp-metric").forEach(b => b.classList.toggle("active", b.dataset.metric === m));
  renderCompare();
}
function closeCmpMenus() {
  document.querySelectorAll(".cmp-menu.show").forEach(m => m.classList.remove("show"));
}
function pickCmpNode(side, el) {
  const mask = window._cmpMask;
  if (!mask || !document.body.contains(mask)) return;
  const val = el.textContent;
  const state = mask._cmpState;
  state[side] = val;
  const label = mask.querySelector("#cmp" + side[0].toUpperCase() + side.slice(1) + "Label");
  if (label) label.textContent = val;
  closeCmpMenus();
  renderCompare();
}

/** Draws both node charts inside the compare dialog (independent of history card). */
function renderCompare() {
  const mask = window._cmpMask;
  if (!mask || !document.body.contains(mask)) return;
  const state = mask._cmpState;
  if (!state) return;
  const left = state.left;
  const right = state.right;
  histCompareNode = right;
  // fetch both series
  const nidL = nodeIds[nodes.findIndex(n => (typeof n === "string" ? n : n.name) === left)];
  const nidR = nodeIds[nodes.findIndex(n => (typeof n === "string" ? n : n.name) === right)];
  const urlL = "/api/node/id/" + (nidL || "") + "/history?metric=" + histMetric + "&range=" + histRange;
  const urlR = "/api/node/id/" + (nidR || "") + "/history?metric=" + histMetric + "&range=" + histRange;
  apiFetch(urlL).then(r => r.json()).then(d => {
      drawCompareChart(mask.querySelector("#cmpChartLeft"), d.points || [], left);
  }).catch(() => {});
  apiFetch(urlR).then(r => r.json()).then(d => {
      drawCompareChart(mask.querySelector("#cmpChartRight"), d.points || [], right);
  }).catch(() => {});
}

function drawCompareChart(canvas, points, name) {
  const ctx = canvas.getContext("2d");
  const w = canvas.clientWidth || canvas.width || 380, h = canvas.clientHeight || canvas.height || 200;
  canvas.width = w * 2;
  canvas.height = h * 2;
  ctx.scale(2, 2);
  ctx.clearRect(0, 0, w, h);
  const netMetric = histMetric === "net_rx" || histMetric === "net_tx";
  const values = points.map(p => netMetric ? (p.avg || 0) : p.avg);
  if (values.length < 2) {
      ctx.fillStyle = "#dc2626";
      ctx.font = "12px sans-serif";
      ctx.textAlign = "center";
      ctx.fillText(t("hist-compare-empty"), w / 2, h / 2);
      return;
  }
  const pad = 6, maxV = 100;
  const color = "#2563eb";
  // grid lines (25/50/75) — same visual language as the trend cards
  ctx.strokeStyle = "rgba(128,128,128,0.15)";
  ctx.lineWidth = 1;
  for (let g = 25; g < 100; g += 25) {
      const y = h - pad - (g / maxV) * (h - pad * 2);
      ctx.beginPath(); ctx.moveTo(pad, y); ctx.lineTo(w - pad, y); ctx.stroke();
  }
  // series line
  ctx.strokeStyle = color;
  ctx.lineWidth = 1.8;
  ctx.lineJoin = "round";
  ctx.beginPath();
  values.forEach((v, i) => {
      const x = pad + (i / (values.length - 1)) * (w - pad * 2);
      const y = h - pad - (Math.min(v, maxV) / maxV) * (h - pad * 2);
      if (i === 0) ctx.moveTo(x, y); else ctx.lineTo(x, y);
  });
  ctx.stroke();
  // gradient fill under the line
  const grad = ctx.createLinearGradient(0, 0, 0, h);
  grad.addColorStop(0, color + "44");
  grad.addColorStop(1, color + "00");
  ctx.lineTo(w - pad, h - pad);
  ctx.lineTo(pad, h - pad);
  ctx.closePath();
  ctx.fillStyle = grad;
  ctx.fill();
  // data points (sampled so the chart stays readable)
  const step = Math.max(1, Math.floor(values.length / 24));
  ctx.fillStyle = color;
  values.forEach((v, i) => {
      if (i % step !== 0) return;
      const x = pad + (i / (values.length - 1)) * (w - pad * 2);
      const y = h - pad - (Math.min(v, maxV) / maxV) * (h - pad * 2);
      ctx.beginPath(); ctx.arc(x, y, 1.5, 0, Math.PI * 2); ctx.fill();
  });
  // node name on the left; metric + current value below it (two lines so a
  // long node name or "RX 0.01 MB/s" never overlap).
  const metricNames = { cpu: "CPU", mem: "MEM", disk: "DISK", temp: "TEMP", net_rx: "RX", net_tx: "TX" };
  const metricLabel = metricNames[histMetric] || histMetric;
  const last = values[values.length - 1];
  const unit = netMetric ? " MB/s" : (histMetric === "temp" ? "°C" : "%");
  ctx.fillStyle = "rgba(0,0,0,0.75)";
  ctx.font = "11px sans-serif";
  ctx.textAlign = "left";
  ctx.fillText(name, pad + 2, pad + 10);
  ctx.fillStyle = color;
  ctx.font = "bold 12px sans-serif";
  ctx.textAlign = "left";
  ctx.fillText(metricLabel + " " + (netMetric ? last.toFixed(2) : Math.round(last)) + unit, pad + 2, pad + 24);
}

function stopHistCompare() {
  histCompare = false;
  histCompareNode = null;
  histCompareData = [];
  const cbtn = document.getElementById("histCompareBtn");
  if (cbtn) cbtn.classList.remove("active");
  loadHistory();
}
function drawHist() {
  const canvas = document.getElementById("histChart");
  if (!canvas) return;
  const ctx = canvas.getContext("2d");
  const w = canvas.clientWidth || 300, h = 80;
  canvas.width = w * 2;
  canvas.height = h * 2;
  ctx.scale(2, 2);
  ctx.clearRect(0, 0, w, h);
  if (!histData.length) {
      ctx.fillStyle = "rgba(128,128,128,0.4)";
      ctx.font = "12px sans-serif";
      ctx.textAlign = "center";
      ctx.fillText(histMetric === "temp" ? "-- no temp sensor --" : "-- no data --", w / 2, h / 2);
      return;
  }
  const n = Math.max(1, histData.length - 1);
  // net metrics are MB/s, others are percent/temp
  const netMetric = histMetric === "net_rx" || histMetric === "net_tx";
  const values = histData.map(p => netMetric ? (p.avg || 0) : p.avg);
  const maxV = Math.max(1, ...values) * 1.1;
  const minV = Math.min(0, ...values) * 0.9;
  const range = (maxV - minV) || 1;
  const pad = 4;
  // grid
  ctx.strokeStyle = "rgba(128,128,128,0.15)";
  ctx.lineWidth = 1;
  for (let g = 1; g < 4; g++) {
      const y = pad + (h - pad * 2) * g / 4;
      ctx.beginPath(); ctx.moveTo(0, y); ctx.lineTo(w, y); ctx.stroke();
  }
  // avg line
  ctx.strokeStyle = "#2563eb";
  ctx.lineWidth = 1.5;
  ctx.beginPath();
  histData.forEach((p, i) => {
      const x = pad + (i / n) * (w - pad * 2);
      const y = h - pad - ((p.avg - minV) / range) * (h - pad * 2);
      if (i === 0) ctx.moveTo(x, y); else ctx.lineTo(x, y);
  });
  ctx.stroke();
  // max/min area (subtle)
  ctx.strokeStyle = "rgba(37,99,235,0.2)";
  ctx.lineWidth = 1;
  ctx.beginPath();
  histData.forEach((p, i) => {
      const x = pad + (i / n) * (w - pad * 2);
      const y = h - pad - ((p.max - minV) / range) * (h - pad * 2);
      if (i === 0) ctx.moveTo(x, y); else ctx.lineTo(x, y);
  });
  ctx.stroke();
  ctx.strokeStyle = "rgba(37,99,235,0.2)";
  ctx.beginPath();
  histData.forEach((p, i) => {
      const x = pad + (i / n) * (w - pad * 2);
      const y = h - pad - ((p.min - minV) / range) * (h - pad * 2);
      if (i === 0) ctx.moveTo(x, y); else ctx.lineTo(x, y);
  });
  ctx.stroke();
  // comparison window (second node) as a dashed orange line
  if (histCompare) {
      if (histCompareData && histCompareData.length > 1) {
          ctx.strokeStyle = "rgba(245,158,11,0.85)";
          ctx.lineWidth = 2;
          ctx.setLineDash([5, 3]);
          ctx.beginPath();
          const cn = histCompareData.length - 1;
          histCompareData.forEach((p, i) => {
              const x = pad + (i / cn) * (w - pad * 2);
              const y = h - pad - (((p.avg || 0) - minV) / range) * (h - pad * 2);
              if (i === 0) ctx.moveTo(x, y); else ctx.lineTo(x, y);
          });
          ctx.stroke();
          ctx.setLineDash([]);
          // legend: node names at the top of the chart
          ctx.font = "10px sans-serif";
          ctx.textAlign = "left";
          const leftName = (typeof nodes[activeNode] === "string" ? nodes[activeNode] : (nodes[activeNode] && nodes[activeNode].name)) || "";
          ctx.fillStyle = "#2563eb";
          ctx.fillText("— " + leftName, pad, pad + 10);
          ctx.fillStyle = "rgba(245,158,11,0.9)";
          ctx.fillText("— " + (histCompareNode || ""), pad, pad + 22);
      } else {
          // Comparison enabled but the second node has no data yet.
          ctx.fillStyle = "rgba(245,158,11,0.7)";
          ctx.font = "11px sans-serif";
          ctx.textAlign = "center";
          ctx.fillText(t("hist-compare-empty"), w / 2, h - 6);
      }
  }
}