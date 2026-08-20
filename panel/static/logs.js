
function updateEvents() {
  apiFetch("/api/events")
      .then(r => r.json())
      .then(d => {
          const body = document.getElementById("eventsBody");
          if (!body) return;
          const evs = d.events || [];
          if (!evs.length) { body.innerHTML = "<div class='empty-hint'>--</div>"; return; }
          const colorMap = { online: "#10b981", offline: "#dc2626", unauthorized: "#f59e0b", info: "#64748b" };
          const labelMap = { online: t("ev-online"), offline: t("ev-offline"), unauthorized: t("ev-unauthorized"), info: t("ev-info") };
          const msgMap = { "node online": t("ev-msg-online"), "node offline": t("ev-msg-offline"), "auth failed": t("ev-msg-unauthorized") };
          // event node name mapping: config name → real hostname
          const evName = (name) => {
              const idx = nodes.indexOf(name);
              return idx >= 0 ? (nodeNames[idx] || name) : name;
          };
          body.innerHTML = evs.slice().reverse().map(ev => `
              <div class="ov-event-row">
                  <span class="ov-event-time">${ev.time}</span>
                  <span class="ov-event-node">${escapeHtml(evName(ev.node))}</span>
                    <span class="ov-event-kind" style="color:${colorMap[ev.kind] || "#64748b"}">${escapeHtml(labelMap[ev.kind] || ev.kind)}</span>
                    <span class="ov-event-msg">${escapeHtml(msgMap[ev.msg] || ev.msg)}</span>
              </div>`).join("");
      })
      .catch(() => {});
}
function exportEventsCSV() {
    fetch('/api/events', { headers: { 'Accept': 'application/json' } })
        .then(r => r.json())
        .then(data => {
            const events = data.events || [];
            if (!events.length) { showConfirm(t('events-empty') || 'No events', '', null, 'OK'); return; }
            const rows = [['time', 'node', 'kind', 'message']];
            for (const e of events) {
                rows.push([e.time || '', e.node || '', e.kind || '', e.message || '']);
            }
            const csv = rows.map(r => r.map(c => '"' + String(c).replace(/"/g, '""') + '"').join(',')).join('\n');
            const blob = new Blob(['\ufeff' + csv], { type: 'text/csv;charset=utf-8' });
            const a = document.createElement('a');
            a.href = URL.createObjectURL(blob);
            a.download = 'hyper-events-' + new Date().toISOString().slice(0, 10) + '.csv';
            a.click();
            URL.revokeObjectURL(a.href);
        })
        .catch(() => showConfirm(t('events-export-fail') || 'Export failed', '', null, 'OK'));
}
function clearEvents() {
    showConfirm(t('events-clear-confirm') || 'Clear all events?', '', () => {
        fetch('/api/events/clear', { method: 'DELETE' })
            .then(r => r.json())
            .then(d => { if (d.ok) { updateEvents(); } else { showConfirm('Failed: ' + (d.error || ''), '', null, 'OK'); } })
            .catch(() => showConfirm('Clear failed', '', null, 'OK'));
    }, t('events-clear') || 'Clear');
}
function updateGreetClock() {
  const now = new Date();
  const h = now.getHours();
  const key = h >= 5 && h < 12 ? "greet-morning" : h >= 12 && h < 18 ? "greet-afternoon" : h >= 18 && h < 23 ? "greet-evening" : "greet-night";
  const greetEl = document.getElementById("greetText");
  if (greetEl) greetEl.textContent = t(key);
  const timeEl = document.getElementById("clockTime");
  if (timeEl) timeEl.textContent = String(now.getHours()).padStart(2, "0") + ":" + String(now.getMinutes()).padStart(2, "0");
  const dateEl = document.getElementById("clockDate");
  if (dateEl) dateEl.textContent = now.toLocaleDateString(currentLang() === "zh" ? "zh-CN" : currentLang() === "ru" ? "ru-RU" : "en-US", { year: "numeric", month: "long", day: "numeric", weekday: "long" });
}