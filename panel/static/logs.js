
function updateEvents() {
  apiFetch("/api/events")
      .then(r => r.json())
      .then(d => {
          const body = document.getElementById("eventsBody");
          if (!body) return;
          const evs = d.events || [];
          if (!evs.length) { body.innerHTML = "<div class='empty-hint'>--</div>"; return; }
          const colorMap = { online: "#10b981", offline: "#dc2626", unauthorized: "#f59e0b", info: "#64748b", alert: "#f97316", admin_action: "#8b5cf6" };
          const labelMap = { online: t("ev-online"), offline: t("ev-offline"), unauthorized: t("ev-unauthorized"), info: t("ev-info"), alert: t("ev-alert"), admin_action: t("ev-admin") };
          const msgMap = { "node online": t("ev-msg-online"), "node offline": t("ev-msg-offline"), "auth failed": t("ev-msg-unauthorized") };
          // event node name mapping: config name → real hostname
          const evName = (name) => {
              const idx = nodes.indexOf(name);
              return idx >= 0 ? (nodeNames[idx] || name) : name;
          };
          body.innerHTML = evs.slice().reverse().map(ev => `
              <div class="ov-event-row">
                  <span class="ov-event-time">${ev.time}</span>
                  <span class="ov-event-node">${ev.kind === "admin_action" ? escapeHtml(ev.actor || "") : escapeHtml(evName(ev.node))}</span>
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

// ===== Notification bell =====
function toggleNotifications(event) {
  event.stopPropagation();
  const panel = document.getElementById("notifPanel");
  const headerMenu = document.getElementById("headerMenu");
  if (headerMenu) headerMenu.classList.remove("show");
  if (!panel) return;
  if (panel.classList.contains("show")) {
    panel.classList.remove("show");
    return;
  }
  panel.classList.add("show");
  loadNotifications();
}
function loadNotifications() {
  const body = document.getElementById("notifBody");
  const countEl = document.getElementById("notifCount");
  const clearBtn = document.getElementById("notifClear");
  apiFetch("/api/notifications")
      .then(r => r.json())
      .then(d => {
          const notifs = d.notifications || [];
          if (countEl) countEl.textContent = notifs.length ? String(notifs.length) : "";
          if (clearBtn) clearBtn.disabled = notifs.length === 0;
          if (!body) return;
          if (!notifs.length) {
              body.innerHTML = "<div class='notif-empty'>" + t("notif-empty") + "</div>";
              return;
          }
          const evName = (name) => {
              const idx = (typeof nodes !== "undefined" ? nodes : []).indexOf(name);
              return idx >= 0 ? ((typeof nodeNames !== "undefined" ? nodeNames[idx] : "") || name) : name;
          };
          body.innerHTML = notifs.slice().reverse().slice(0, 20).map(ev => `
              <div class="notif-item">
                  <div class="notif-item-top">
                      <span class="notif-node">${escapeHtml(evName(ev.node))}</span>
                      <span class="notif-kind" style="color:#f97316">${escapeHtml(t("ev-alert"))}</span>
                      <span class="notif-time">${escapeHtml(ev.time || "")}</span>
                  </div>
                  <div class="notif-msg">${escapeHtml(ev.msg || "")}</div>
              </div>`).join("");
      })
      .catch(() => {});
}
// Close notification panel on outside click / scroll (mobile friendly)
document.addEventListener("click", (e) => {
  const panel = document.getElementById("notifPanel");
  if (panel && panel.classList.contains("show") && !panel.contains(e.target) && e.target.id !== "bellBtn") {
      panel.classList.remove("show");
  }
});
window.addEventListener("scroll", () => {
  const panel = document.getElementById("notifPanel");
  if (panel && panel.classList.contains("show")) panel.classList.remove("show");
}, { passive: true });
// Refresh badge on page load
loadNotifications();

// Clear all notifications (admin only endpoint)
function clearNotifications(event) {
  if (event) event.stopPropagation();
  fetch('/api/notifications', { method: 'DELETE' })
      .then(r => r.json())
      .then(d => {
          if (d.ok) {
              loadNotifications();
              const body = document.getElementById("notifBody");
              if (body) body.innerHTML = "<div class='notif-empty'>" + t("notif-empty") + "</div>";
          } else {
              const countEl = document.getElementById("notifCount");
              if (countEl) countEl.textContent = "";
          }
      })
      .catch(() => {});
}