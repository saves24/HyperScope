


// ===== Theme switching =====


// ===== Settings panel (theme / language / background) =====


// sub-menu: click group title to expand/collapse

// init: hide data-admin-only groups for non-admin + default expand state

// ===== User management (admin only) =====


// panel port: load current value

// panel port: save (shared panel.json with CLI; auto-restart + redirect on change)


// on load: if a pending port was recorded and is live, auto-redirect


document.addEventListener("click", (e) => {
  const panel = document.getElementById("settingsPanel");
  if (panel.classList.contains("show") && !panel.contains(e.target)) {
      panel.classList.remove("show");
  }
});
// Close settings panel on scroll (mobile: page swipes should dismiss it)
window.addEventListener("scroll", () => {
  const panel = document.getElementById("settingsPanel");
  if (panel.classList.contains("show")) {
      panel.classList.remove("show");
  }
}, { passive: true });
applyLang();
applyTheme(getCookie("ts-theme") || "auto");
loadPanelPort();
initSettingsGroups();
checkPendingPort();
// ===== Background image =====


loadBg();
loadCardTrans();
setInterval(() => {
  const mode = getCookie("ts-theme") || "auto";
  if (mode === "auto" || mode === "system") {
      // follow system theme (sync on OS dark/light change)
      const shouldDark = window.matchMedia("(prefers-color-scheme: dark)").matches;
      const isDark = document.documentElement.getAttribute("data-theme") === "dark";
      if (shouldDark !== isDark) applyTheme(mode);
  }
}, 60000);

// display name: hostname for pure-IP configs, else config name (custom name wins)

// ===== Node management (driven by panel aggregator, nodes.json config) =====
let nodes = [];
let nodeNames = []; // node display names (from /api/nodes; config name preferred)
let nodeHostnames = []; // node hostname note (agent-reported, shown when differs from config name)
let nodeOwners = []; // node owner (only populated for admin view)
let nodeIds = [];   // node stable ids (preferred for data APIs)
// Build node API URL by stable id (fallback to legacy idx for robustness)

let nodeTls = [];    // node TLS state {tls, verified}
let nodeAlerts = []; // node alert settings {webhook, alert_cpu, alert_mem, alert_disk, alert_temp}
let nodeAlertActive = []; // currently raised alert keys per node (badge)
let nodeGroups = []; // node group/label (for list filtering)
let activeGroup = ""; // current group filter ("" = all)
let activeNode = 0; // currently selected node (cards show its data)
// select node: update activeNode + highlight + refresh card area

// select nearest card when horizontal scroll stops
let nodeScrollTimer = null;

// refresh overview cards (on node selection change)


// node list modal: list all nodes, click to select


// key input: auto-check + lock TLS toggle when cert fingerprint present


// rename node
let renameTargetIdx = -1;


// reboot node (confirm dialog)

// shutdown node (confirm dialog)

// ping node: modal shows progress and result


// ===== Confirm dialog =====
let confirmCallback = null;


document.addEventListener("click", (e) => {
  document.querySelectorAll(".node-menu").forEach(m => {
      if (m.classList.contains("show") && !m.contains(e.target) && !e.target.closest(".node-menu-btn")) {
          m.classList.remove("show");
      }
  });
});
// ===== Gauge =====
const GAUGE_CIRCUMFERENCE = Math.PI * 45;

let failCount = []; // consecutive poll failures per node
let sortMode = "default"; // node card sort: default | cpu | mem
let nodeStats = {}; // per-node latest {cpu, mem} for sorting

// node data polling


setInterval(loadNodeList, 10000);
setInterval(updateAllNodes, 5000);
loadNodeList();
renderNodes();
setTimeout(updateAllNodes, 500);
// ===== Overview section =====
// system overview (selected node)

// docker overview card: running/total + progress bar

// load display: mini bar (relative to cores) + i18n time labels


// disk overview

// network interface overview

// real-time trends
const TREND_POINTS = 60;
let trendCpu = new Array(TREND_POINTS).fill(0);
let trendMem = new Array(TREND_POINTS).fill(0);
let trendDisk = new Array(TREND_POINTS).fill(0);


// event history

setInterval(updateOverview, 5000);
  setInterval(updateKline, 5000);
  setInterval(loadHistory, 60000);
setInterval(updateDiskOverview, 10000);
setInterval(updateNetOverview, 5000);
setInterval(updateTrend, 5000);
setInterval(updateEvents, 5000);
setTimeout(() => { updateOverview(); updateKline(); loadHistory(); updateDiskOverview(); updateNetOverview(); updateTrend(); updateEvents(); }, 1000);
// live rate / temperature / top processes / node stats
// live rate (main API) + rate trend curve
let trendNetRx = new Array(TREND_POINTS).fill(0);
let trendNetTx = new Array(TREND_POINTS).fill(0);


// temperature monitor + curve
let trendTemp = new Array(TREND_POINTS).fill(0);


// top 5 processes (by CPU)

// node stats (all nodes)

setInterval(updateSpeed, 2000);
setInterval(updateTemp, 5000);
setInterval(updateTopProcs, 10000);
setInterval(updateNodeStats, 10000);
setTimeout(() => { updateSpeed(); updateTemp(); updateTopProcs(); updateNodeStats(); }, 1500);
// ===== Modals (per-node actions) =====


// ===== Process modal =====
let procSort = "mem";
let procLimit = 20;
let procTimer = null;


function toggleProcSort() {
  procSort = procSort === "mem" ? "cpu" : "mem";
  const btn = document.getElementById("procSortBtn");
  btn.textContent = procSort === "mem" ? t("sort-mem") : t("sort-cpu");
  loadProcs();
}

// ===== Disk modal =====


// ===== Docker container modal =====
// resolve current node index (activeNode may be card idx or config idx)


// ===== Historical trend (SQLite persisted) =====
let histRange = "24h";

// Multi-node compare mode + CSV export


let histMetric = "cpu";
let histData = [];
let histCompare = false; // show previous-period overlay
let histCompareData = []; // previous-period points
let histCompareNode = null; // second node name for node-vs-node compare


// ===== Disk I/O + connection trend (line) =====
let klineRead = new Array(30).fill(0);
let klineWrite = new Array(30).fill(0);
const KLINE_POINTS = 30;


setInterval(updateGreetClock, 1000);
updateGreetClock();

// Export events as CSV (client-side generation)


// Clear the event log (admin)

