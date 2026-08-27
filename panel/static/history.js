
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
}