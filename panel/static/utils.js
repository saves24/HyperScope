// ===== Login =====
// token stored by backend Set-Cookie (HttpOnly, browser-managed, 1-day expiry)
// JS never touches token; requests carry cookie automatically; prefs use plain cookies


// fetch wrapper: browser sends cookie; show login on 401
async function apiFetch(url, opts) {
  opts = opts || {};
  opts.headers = opts.headers || {};
  const r = await fetch(url, opts);
  if (r.status === 401 && !url.includes("/api/login")) {
      showLogin();
      throw { unauthorized: true };
  }
  return r;
}

function enterLoginMode() {
   const title = document.getElementById("loginTitle");
   const btn = document.getElementById("loginBtn");
   title.textContent = t("login-title");
   btn.textContent = t("login-btn");
   btn.onclick = doLogin;
   document.getElementById("tabLogin").classList.add("active");
   document.getElementById("tabSetup").classList.remove("active");
}

function showLogin(closeable) {
   const mask = document.getElementById("loginMask");
   const firstShow = !mask.classList.contains("show");
   mask.classList.add("show");
   document.getElementById("loginError").classList.remove("show");
   // show close button when closeable=true; hide on forced login
   document.getElementById("loginClose").style.display = closeable ? "block" : "none";
   if (firstShow) document.getElementById("loginUser").focus();
}
function hideLogin() {
   document.getElementById("loginMask").classList.remove("show");
}
function doLogin() {
   const user = document.getElementById("loginUser").value.trim();
   const pass = document.getElementById("loginPass").value;
   fetch("/api/login", {
       method: "POST",
       headers: { "Content-Type": "application/json" },
       body: JSON.stringify({ user, pass })
   })
       .then(r => r.json())
       .then(d => {
           if (d.ok) {
               // cookie set by backend, entering on refresh
               hideLogin();
               location.reload();
           } else {
               showLoginError(t("login-error"));
           }
       })
       .catch(() => showLoginError(t("login-error")));
}

// error message auto-hides after 10s (enough time to read and re-type)
let loginErrorTimer = null;
function showLoginError(msg) {
   const el = document.getElementById("loginError");
   el.textContent = msg;
   el.classList.add("show");
   clearTimeout(loginErrorTimer);
   loginErrorTimer = setTimeout(() => el.classList.remove("show"), 10000);
}

// toggle password visibility
function togglePass(inputId, eyeId) {
   const input = document.getElementById(inputId);
   const eye = document.getElementById(eyeId);
   if (input.type === "password") {
       input.type = "text";
       eye.classList.add("off");
   } else {
       input.type = "password";
       eye.classList.remove("off");
   }
}
// clear error on input
["loginUser", "loginPass"].forEach(id => {
   document.getElementById(id).addEventListener("input", () => {
       document.getElementById("loginError").classList.remove("show");
   });
});
document.addEventListener("keydown", (e) => {
   if (e.key === "Enter" && document.getElementById("loginMask").classList.contains("show")) {
       doLogin();
   }
});
// probe login state on load (admin account created via CLI: hyper-panel setup / user passwd)
function checkAuthStatus() {
   fetch("/api/settings")
       .then(r => {
           if (r.status === 401) {
               enterLoginMode();
               showLogin();
           }
       })
       .catch(() => {});
}
checkAuthStatus();

// ===== Top menu (hamburger button) =====
function toggleHeaderMenu(event) {
   if (event) event.stopPropagation();
   // close settings panel when opening menu (avoid overlap)
   document.getElementById("settingsPanel").classList.remove("show");
   document.getElementById("headerMenu").classList.toggle("show");
}

function closeHeaderMenu() {
   document.getElementById("headerMenu").classList.remove("show");
}

function logout() {
   // token in HttpOnly cookie; ask backend to clear it
   fetch("/api/logout", { method: "POST" })
       .catch(() => {})
       .finally(() => {
           closeHeaderMenu();
           location.reload();
       });
}

// close user menu on outside click
document.addEventListener("click", (e) => {
   const menu = document.getElementById("headerMenu");
   const btn = document.getElementById("headerMenuBtn");
   if (menu.classList.contains("show") && !menu.contains(e.target) && !btn.contains(e.target)) {
       menu.classList.remove("show");
   }
});
// ===== i18n (Chinese / English / Russian) =====
const I18N = {
  "zh": {
      "theme-auto": "自动", "theme-system": "跟随系统", "theme-light": "白天", "theme-dark": "夜间",
      "lang-label": "中文",
      "bg": "背景", "bg-choose": "📁 选择图片", "bg-opacity-label": "透明度",
      "bg-card-trans": "卡片透明", "bg-remove": "移除背景",
      "theme": "主题", "language": "语言", "settings-menu": "设置", "logout": "退出登录",
      "notif-title": "通知", "notif-empty": "暂无通知", "notif-clear": "清空",
      "panel-port": "面板端口", "panel-port-current": "当前", "panel-port-hint": "修改后需重启面板生效", "panel-port-invalid": "端口无效 (1-65535)", "save": "保存", "saved": "已保存", "restarting": "正在重启面板并跳转新端口...", "restart-needed": "请手动重启面板生效",
      "title": "HyperScope", "docker-overview": "Docker", "history-title": "历史趋势", "hist-export": "导出", "sort-default": "默认", "sort-cpu": "CPU", "sort-mem": "内存", "docker-overview": "Docker",
      "docker-title": "Docker", "docker-load": "加载容器中...", "docker-empty": "无容器", "docker-name": "名称", "docker-image": "镜像", "docker-state": "状态", "docker-running": "运行中", "docker-exited": "已停止", "docker-restart": "重启", "docker-stop": "停止", "docker-start": "启动", "docker-op-fail": "操作失败", "docker-overview": "Docker", "kline-title": "IO/连接 K线", "kline-read": "读", "kline-write": "写",
      "login-title": "登录", "login-user": "用户名", "login-pass": "密码", "login-btn": "登录", "login-error": "用户名或密码错误",
      "op-err": "操作失败",
      "nodes": "节点", "add-node": "添加节点", "add-node-title": "添加节点", "node-list": "节点列表", "node-manage": "节点管理", "node-batch-add": "批量添加", "node-batch-del": "批量删除", "node-batch-hint": "每行一个节点：地址[:端口],key[,名称]", "node-batch-empty": "请输入要添加的节点", "node-batch-result": "批量添加完成：成功 {ok}，失败 {fail}", "node-batch-none": "请先勾选要删除的节点", "node-batch-del-confirm": "确认删除所选节点", "node-import": "本地导入", "node-import-choose": "选择 .hsxc 文件", "node-import-hint": "从 .hsxc 配置文件本地导入", "node-import-pass": "口令", "node-import-nofile": "请先选择 .hsxc 文件", "node-import-pass-need": "请输入口令", "node-import-empty": "配置中没有节点", "node-import-invalid": "不是有效的 .hsxc 文件", "node-import-badpass": "口令错误或文件已损坏", "node-export": "批量导出", "node-export-hint": "设置一个口令，将当前所有节点加密导出为 .hsxc 文件", "node-export-pass": "口令", "node-export-pass-need": "请设置导出口令", "node-export-empty": "没有可导出的节点", "node-export-ok": "已导出 .hsxc 配置文件", "node-addr-label": "地址", "node-key-label": "API Key",
      "add-node-key-hint": "在节点运行 hyper-node key show 获取", "node-tls-label": "TLS 加密连接", "node-tls-hint": "HTTPS + 证书指纹校验",
      "node-alert-advanced": "告警设置（可选）", "node-webhook-label": "Webhook URL（告警推送，如 ntfy/Bark/Server 酱）", "node-alert-cpu": "CPU ≥", "node-alert-mem": "内存 ≥", "node-alert-disk": "磁盘 ≥", "node-alert-temp": "温度 ≥", "node-alert-edit": "告警", "node-alert-title": "告警设置", "node-alert-save": "保存告警设置", "node-alert-saved": "告警设置已保存", "node-alert-hint": "告警阈值触发时推送 webhook（填 URL 即启用推送）", "node-group-edit": "分组", "node-group-saved": "分组已保存", "node-group-hint": "分组用于列表筛选，如 office / hk / lab", "group-all": "全部", "node-group-manage": "分组设置", "node-group-save": "保存分组", "node-notify-label": "通知方式", "notify-pushplus": "PushPlus（微信）", "notify-serverchan": "Server 酱（微信）", "notify-telegram": "Telegram", "notify-webhook": "自定义 Webhook", "notify-pushplus-token": "PushPlus Token（pushplus.plus 扫码注册获取）", "notify-serverchan-key": "Server 酱 SendKey（sct.ftqq.com 获取）", "notify-telegram-bot": "Bot Token（@BotFather 创建）", "notify-telegram-chat": "Chat ID（@userinfobot 获取）",
      "cancel": "取消", "confirm": "确定", "confirm-del": "删除",
      "add-node-confirm": "添加", "node-offline": "离线", "node-unauth": "Key 无效", "remove-node": "删除节点", "rename-node": "重命名", "ping-node": "Ping", "node-renamed": "节点已重命名", "ping-ok": "可达", "ping-fail": "不可达", "ping-running": "测试中", "close": "关闭", "node-tls-key-hint": "明文 key 不能启用 TLS, 请使用 hyper-node key show 获取带证书指纹的完整 key", "reboot-node": "重启", "shutdown-node": "关机", "reboot-sent": "重启指令已发送", "shutdown-sent": "关机指令已发送",
      "cpu-usage": "CPU 占用", "memory": "内存", "cpu-temp": "CPU 温度", "gpu-temp": "GPU 温度",
      "processes": "进程数", "disk": "磁盘", "uptime": "运行时间", "updated": "更新于", "loadavg": "负载",
      "load-1m": "1分钟", "load-5m": "5分钟", "load-15m": "15分钟",
      "cpu-cores": "核心", "proc-btn": "进程", "disk-btn": "磁盘", "proc-title": "进程列表", "disk-used": "已用", "disk-free": "可用", "disk-total": "总量", "disk-inode": "Inode",
      "disk-title": "磁盘详情", "greet-morning": "早上好", "greet-afternoon": "下午好", "greet-evening": "晚上好", "greet-night": "夜深了",
      "overview-title": "系统总览", "disks-title": "磁盘总览", "net-title": "网络接口", "trend-title": "实时趋势", "events-title": "事件记录", "events-export": "导出 CSV", "events-clear": "清空日志", "events-empty": "暂无事件", "events-clear-confirm": "确认清空所有事件？",
      "ov-kernel": "内核", "ov-version": "版本", "ov-load": "负载", "ov-uptime": "运行时间", "ov-procs": "进程数",
      "ev-online": "恢复在线", "ev-offline": "掉线", "ev-unauthorized": "Key 认证失败", "ev-info": "信息", "ev-alert": "告警", "ev-admin": "审计", "ev-msg-online": "节点恢复在线", "ev-msg-offline": "节点掉线", "ev-msg-unauthorized": "认证失败",
      "net-rx": "↓", "net-tx": "↑", "speed-title": "实时速率", "temp-title": "温度监控", "top-title": "进程 TOP", "nodes-stats": "节点统计", "speed-trend-title": "速率趋势", "sp-iface": "接口",
      "proc-name": "进程名", "proc-cpu": "CPU", "proc-mem": "内存", "proc-state": "状态",
      "bg-file-selected": "已选择", "bg-set": "背景已设置", "bg-too-large": "图片过大，存储失败。请选择更小的图片。", "node-added": "节点已添加", "node-removed": "节点已删除"
  },
  "en": {
      "theme-auto": "Auto", "docker-empty": "No containers", "docker-exited": "Stopped", "docker-image": "Image", "docker-load": "Loading containers...", "docker-name": "Name", "docker-op-fail": "Operation failed", "docker-overview": "Docker", "docker-restart": "Restart", "docker-running": "Running", "docker-start": "Start", "docker-state": "State", "docker-stop": "Stop", "docker-title": "Docker", "kline-read": "Read", "kline-write": "Write", "kline-title": "IO/Candles", "history-title": "History", "hist-export": "Export", "sort-default": "Default", "sort-cpu": "CPU", "sort-mem": "Memory", "theme-system": "System", "theme-light": "Light", "theme-dark": "Dark",
      "lang-label": "English",
      "bg": "Background", "bg-choose": "📁 Choose Image", "bg-opacity-label": "Opacity",
      "bg-card-trans": "Transparent Cards", "bg-remove": "Remove",
      "theme": "Theme", "language": "Language", "settings-menu": "Settings", "logout": "Sign Out",
      "notif-title": "Notifications", "notif-empty": "No notifications", "notif-clear": "Clear",
      "panel-port": "Panel Port", "panel-port-current": "Current", "panel-port-hint": "Takes effect after panel restart", "panel-port-invalid": "Invalid port (1-65535)", "save": "Save", "saved": "Saved", "restarting": "Restarting panel, redirecting to new port...", "restart-needed": "Restart the panel manually",
      "title": "HyperScope",
      "login-title": "Sign In", "login-user": "Username", "login-pass": "Password", "login-btn": "Sign In", "login-error": "Invalid username or password",
      "op-err": "Operation failed",
      "nodes": "Nodes", "add-node": "Add Node", "add-node-title": "Add Node", "node-list": "Node List", "node-manage": "Node Manager", "node-batch-add": "Batch Add", "node-batch-del": "Batch Delete", "node-batch-hint": "One node per line: addr[:port],key[,name]", "node-batch-empty": "Enter nodes to add", "node-batch-result": "Batch add done: ok {ok}, fail {fail}", "node-batch-none": "Select nodes to delete first", "node-batch-del-confirm": "Delete selected nodes", "node-import": "Local Import", "node-import-choose": "Choose .hsxc file", "node-import-hint": "Import from a local .hsxc config file", "node-import-pass": "Passphrase", "node-import-nofile": "Select a .hsxc file first", "node-import-pass-need": "Enter the passphrase", "node-import-empty": "No nodes in config", "node-import-invalid": "Not a valid .hsxc file", "node-import-badpass": "Wrong passphrase or corrupted file", "node-export": "Batch Export", "node-export-hint": "Set a passphrase and export all nodes encrypted as .hsxc", "node-export-pass": "Passphrase", "node-export-pass-need": "Set an export passphrase", "node-export-empty": "No nodes to export", "node-export-ok": ".hsxc config exported", "node-addr-label": "Address", "node-key-label": "API Key",
      "add-node-key-hint": "Run hyper-node key show on the node", "node-tls-label": "TLS encrypted", "node-tls-hint": "HTTPS + cert fingerprint verification",
      "node-alert-advanced": "Alert settings (optional)", "node-webhook-label": "Webhook URL (alert push, e.g. ntfy/Bark/Server Chan)", "node-alert-cpu": "CPU ≥", "node-alert-mem": "Memory ≥", "node-alert-disk": "Disk ≥", "node-alert-temp": "Temp ≥", "node-alert-edit": "Alerts", "node-alert-title": "Alert Settings", "node-alert-save": "Save Alert Settings", "node-alert-saved": "Alert settings saved", "node-alert-hint": "Push webhook when thresholds are hit (set URL to enable)", "node-group-edit": "Group", "node-group-saved": "Group saved", "node-group-hint": "Group for list filtering, e.g. office / hk / lab", "group-all": "All", "node-group-manage": "Group Settings", "node-group-save": "Save Groups", "node-notify-label": "Notify via", "notify-pushplus": "PushPlus (WeChat)", "notify-serverchan": "Server Chan (WeChat)", "notify-telegram": "Telegram", "notify-webhook": "Custom Webhook", "notify-pushplus-token": "PushPlus Token (get at pushplus.plus)", "notify-serverchan-key": "Server Chan SendKey (get at sct.ftqq.com)", "notify-telegram-bot": "Bot Token (create with @BotFather)", "notify-telegram-chat": "Chat ID (get from @userinfobot)",
      "cancel": "Cancel", "confirm": "OK", "confirm-del": "Delete",
      "add-node-confirm": "Add", "node-offline": "Offline", "node-unauth": "Invalid Key", "remove-node": "Remove Node", "rename-node": "Rename", "ping-node": "Ping", "node-renamed": "Node renamed", "ping-ok": "Reachable", "ping-fail": "Unreachable", "ping-running": "Testing", "close": "Close", "node-tls-key-hint": "Plain key cannot enable TLS. Use hyper-node key show to get the full key with certificate fingerprint", "reboot-node": "Reboot", "shutdown-node": "Shutdown", "reboot-sent": "Reboot command sent", "shutdown-sent": "Shutdown command sent",
      "cpu-usage": "CPU Usage", "memory": "Memory", "cpu-temp": "CPU Temp", "gpu-temp": "GPU Temp",
      "processes": "Processes", "disk": "Disk", "uptime": "Uptime", "updated": "Updated", "loadavg": "Load",
      "load-1m": "1m", "load-5m": "5m", "load-15m": "15m",
      "cpu-cores": "Cores", "proc-btn": "Processes", "disk-btn": "Disk", "proc-title": "Process List", "disk-used": "Used", "disk-free": "Free", "disk-total": "Total", "disk-inode": "Inodes",
      "disk-title": "Disk Details", "greet-morning": "Good morning", "greet-afternoon": "Good afternoon", "greet-evening": "Good evening", "greet-night": "Late night",
      "overview-title": "Overview", "disks-title": "Disks", "net-title": "Network", "trend-title": "Live Trends", "events-title": "Events",
      "events-export": "Export CSV",
      "events-clear": "Clear Log",
      "events-empty": "No events",
      "events-clear-confirm": "Clear all events?",
      "ov-kernel": "Kernel", "ov-version": "Version", "ov-load": "Load", "ov-uptime": "Uptime", "ov-procs": "Processes",
      "ev-online": "back online", "ev-offline": "offline", "ev-unauthorized": "key unauthorized", "ev-info": "info", "ev-alert": "alert", "ev-admin": "audit", "ev-msg-online": "Node back online", "ev-msg-offline": "Node went offline", "ev-msg-unauthorized": "Authentication failed",
      "net-rx": "↓", "net-tx": "↑", "speed-title": "Live Speed", "temp-title": "Temperatures", "top-title": "Top Processes", "nodes-stats": "Node Stats", "speed-trend-title": "Speed Trend", "sp-iface": "Iface",
      "proc-name": "Process", "proc-cpu": "CPU", "proc-mem": "Memory", "proc-state": "State",
      "bg-file-selected": "Selected", "bg-set": "Background set", "bg-too-large": "Image too large. Please choose a smaller one.", "node-added": "Node added", "node-removed": "Node removed"
  },
  "ru": {
      "theme-auto": "Авто", "docker-empty": "Нет контейнеров", "docker-exited": "Остановлен", "docker-image": "Образ", "docker-load": "Загрузка контейнеров...", "docker-name": "Имя", "docker-op-fail": "Ошибка операции", "docker-overview": "Docker", "docker-restart": "Перезапуск", "docker-running": "Запущен", "docker-start": "Старт", "docker-state": "Статус", "docker-stop": "Стоп", "docker-title": "Docker", "kline-read": "Чт", "kline-write": "Зап", "kline-title": "IO/Свечи", "history-title": "История", "hist-export": "Экспорт", "sort-default": "По умолчанию", "sort-cpu": "CPU", "sort-mem": "Память", "theme-system": "Как в системе", "theme-light": "Светлая", "theme-dark": "Тёмная",
      "lang-label": "Русский",
      "bg": "Фон", "bg-choose": "📁 Выбрать изображение", "bg-opacity-label": "Прозрачность",
      "bg-card-trans": "Прозрачные карточки", "bg-remove": "Удалить",
      "theme": "Тема", "language": "Язык", "settings-menu": "Настройки", "logout": "Выйти",
      "notif-title": "Уведомления", "notif-empty": "Нет уведомлений", "notif-clear": "Очистить",
      "panel-port": "Порт панели", "panel-port-current": "Текущий", "panel-port-hint": "Вступит в силу после перезапуска", "panel-port-invalid": "Неверный порт (1-65535)", "save": "Сохранить", "saved": "Сохранено", "restarting": "Перезапуск панели, переход на новый порт...", "restart-needed": "Перезапустите панель вручную",
      "title": "HyperScope",
      "login-title": "Вход", "login-user": "Имя", "login-pass": "Пароль", "login-btn": "Войти", "login-error": "Неверное имя или пароль",
      "op-err": "Ошибка операции",
      "nodes": "Узлы", "add-node": "Добавить узел", "add-node-title": "Добавить узел", "node-list": "Список узлов", "node-manage": "Управление узлами", "node-batch-add": "Массовое добавление", "node-batch-del": "Массовое удаление", "node-batch-hint": "Один узел в строке: адрес[:порт],ключ[,имя]", "node-batch-empty": "Введите узлы для добавления", "node-batch-result": "Массовое добавление: успешно {ok}, ошибок {fail}", "node-batch-none": "Сначала выберите узлы для удаления", "node-batch-del-confirm": "Удалить выбранные узлы", "node-import": "Локальный импорт", "node-import-choose": "Выбрать файл .hsxc", "node-import-hint": "Импорт из локального файла конфигурации .hsxc", "node-import-pass": "Парольная фраза", "node-import-nofile": "Сначала выберите файл .hsxc", "node-import-pass-need": "Введите парольную фразу", "node-import-empty": "В конфигурации нет узлов", "node-import-invalid": "Недействительный файл .hsxc", "node-import-badpass": "Неверная фраза или повреждённый файл", "node-export": "Массовый экспорт", "node-export-hint": "Задайте парольную фразу и экспортируйте все узлы в зашифрованный .hsxc", "node-export-pass": "Парольная фраза", "node-export-pass-need": "Задайте парольную фразу для экспорта", "node-export-empty": "Нет узлов для экспорта", "node-export-ok": "Конфиг .hsxc экспортирован", "node-addr-label": "Адрес", "node-key-label": "API Key",
      "add-node-key-hint": "Запустите hyper-node key show на узле", "node-tls-label": "Шифрование TLS", "node-tls-hint": "HTTPS + проверка отпечатка сертификата",
      "node-alert-advanced": "Настройки оповещений (опционально)", "node-webhook-label": "Webhook URL (push оповещений, напр. ntfy/Bark/Server Chan)", "node-alert-cpu": "CPU ≥", "node-alert-mem": "Память ≥", "node-alert-disk": "Диск ≥", "node-alert-temp": "Темп ≥", "node-alert-edit": "Оповещ.", "node-alert-title": "Настройки оповещений", "node-alert-save": "Сохранить", "node-alert-saved": "Оповещения сохранены", "node-alert-hint": "Отправлять webhook при превышении порогов (укажите URL)", "node-group-edit": "Группа", "node-group-saved": "Группа сохранена", "node-group-hint": "Группа для фильтрации списка, напр. office / hk / lab", "group-all": "Все", "node-group-manage": "Настройки групп", "node-group-save": "Сохранить группы", "node-notify-label": "Способ уведомления", "notify-pushplus": "PushPlus (WeChat)", "notify-serverchan": "Server Chan (WeChat)", "notify-telegram": "Telegram", "notify-webhook": "Свой Webhook", "notify-pushplus-token": "PushPlus Token (на pushplus.plus)", "notify-serverchan-key": "SendKey Server Chan (на sct.ftqq.com)", "notify-telegram-bot": "Bot Token (создать у @BotFather)", "notify-telegram-chat": "Chat ID (узнать у @userinfobot)",
      "cancel": "Отмена", "confirm": "ОК", "confirm-del": "Удалить",
      "add-node-confirm": "Добавить", "node-offline": "Офлайн", "node-unauth": "Неверный ключ", "remove-node": "Удалить узел", "rename-node": "Переименовать", "ping-node": "Ping", "node-renamed": "Узел переименован", "ping-ok": "Доступен", "ping-fail": "Недоступен", "ping-running": "Тестирование", "close": "Закрыть", "node-tls-key-hint": "Обычный ключ не может включить TLS. Используйте hyper-node key show для получения полного ключа с отпечатком сертификата", "reboot-node": "Перезагрузка", "shutdown-node": "Выключение", "reboot-sent": "Команда перезагрузки отправлена", "shutdown-sent": "Команда выключения отправлена",
      "cpu-usage": "Загрузка CPU", "memory": "Память", "cpu-temp": "Темп. CPU", "gpu-temp": "Темп. GPU",
      "processes": "Процессы", "disk": "Диск", "uptime": "Время работы", "updated": "Обновлено", "loadavg": "Нагрузка",
      "load-1m": "1м", "load-5m": "5м", "load-15m": "15м",
      "cpu-cores": "Ядра", "proc-btn": "Процессы", "disk-btn": "Диск", "proc-title": "Список процессов", "disk-used": "Использовано", "disk-free": "Свободно", "disk-total": "Всего", "disk-inode": "Иноды",
      "disk-title": "Диски", "greet-morning": "Доброе утро", "greet-afternoon": "Добрый день", "greet-evening": "Добрый вечер", "greet-night": "Уже поздно",
      "overview-title": "Обзор", "disks-title": "Диски", "net-title": "Сеть", "trend-title": "Графики", "events-title": "События", "events-export": "Экспорт CSV", "events-clear": "Очистить лог", "events-empty": "Нет событий", "events-clear-confirm": "Очистить все события?",
      "ov-kernel": "Ядро", "ov-version": "Версия", "ov-load": "Нагрузка", "ov-uptime": "Время работы", "ov-procs": "Процессы",
      "ev-online": "снова в сети", "ev-offline": "офлайн", "ev-unauthorized": "неверный ключ", "ev-info": "инфо", "ev-alert": "оповещение", "ev-admin": "аудит", "ev-msg-online": "Узел снова в сети", "ev-msg-offline": "Узел отключился", "ev-msg-unauthorized": "Ошибка аутентификации",
      "net-rx": "↓", "net-tx": "↑", "speed-title": "Скорость", "temp-title": "Температура", "top-title": "Топ процессов", "nodes-stats": "Статистика узлов", "speed-trend-title": "График скорости", "sp-iface": "Интерфейс",
      "proc-name": "Процесс", "proc-cpu": "CPU", "proc-mem": "Память", "proc-state": "Состояние",
      "bg-file-selected": "Выбрано", "bg-set": "Фон установлен", "bg-too-large": "Изображение слишком большое. Выберите меньшее.", "node-added": "Узел добавлен", "node-removed": "Узел удалён"
  }
};

function setCookie(name, value, days) {
  let expires = "";
  if (days) {
      const d = new Date();
      d.setTime(d.getTime() + days * 86400000);
      expires = "; expires=" + d.toUTCString();
  }
  document.cookie = name + "=" + encodeURIComponent(value) + expires + "; path=/";
}
function getCookie(name) {
  const m = document.cookie.match(new RegExp("(?:^|; )" + name + "=([^;]*)"));
  return m ? decodeURIComponent(m[1]) : "";
}
function delCookie(name) {
  document.cookie = name + "=; expires=Thu, 01 Jan 1970 00:00:00 GMT; path=/";
}
function detectBrowserLang() {
  const navLang = (navigator.language || navigator.userLanguage || "en").toLowerCase();
  if (navLang.startsWith("zh")) return "zh";
  if (navLang.startsWith("ru")) return "ru";
  return "en";
}
function currentLang() {
  const saved = getCookie("ts-lang");
  if (saved === "zh" || saved === "en" || saved === "ru") return saved;
  return detectBrowserLang();
}
function t(key) {
  const lang = currentLang();
  return (I18N[lang] && I18N[lang][key]) || (I18N["zh"][key]) || key;
}
function applyLang() {
  const lang = currentLang();
  document.querySelectorAll("[data-i18n]").forEach(el => {
      el.textContent = t(el.getAttribute("data-i18n"));
  });
  document.querySelectorAll("[data-i18n-placeholder]").forEach(el => {
      el.setAttribute("placeholder", t(el.getAttribute("data-i18n-placeholder")));
  });
  const langLabel = document.getElementById("langLabel");
  if (langLabel) langLabel.textContent = t("lang-label");
  document.documentElement.lang = lang === "zh" ? "zh-CN" : lang === "ru" ? "ru" : "en";
  if (typeof updateGreetClock === "function") updateGreetClock();
}
function resolveTheme(mode) {
  // auto / system both follow OS theme (prefers-color-scheme)
  if (mode === "auto" || mode === "system") return window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
  return mode;
}
function applyTheme(mode) {
  const resolved = resolveTheme(mode);
  document.documentElement.setAttribute("data-theme", resolved);
  setCookie("ts-theme", mode, 365);
  // highlight active theme option
  document.querySelectorAll(".settings-opt[data-theme]").forEach(b => {
      b.classList.toggle("active", b.getAttribute("data-theme") === mode);
  });
}
function showToast(msg, durationMs = 2000) {
  const toast = document.getElementById("toast");
  toast.innerHTML = msg;
  toast.style.display = "block";
  clearTimeout(toast._timer);
  toast._timer = setTimeout(() => { toast.style.display = "none"; }, durationMs);
}
function displayName(n) {
  const name = n.name || n.node_name || "";
  const isIp = /^\d{1,3}(\.\d{1,3}){3}$/.test(name);
  return (isIp && n.node_name) ? n.node_name : name;
}
function nodeApiUrl(i, path) {
  return "/api/node/id/" + (nodeIds[i] || "") + path;
}
function formatLoad(loadavg, cores, showBar) {
  if (!loadavg) return "--";
  const parts = String(loadavg).trim().split(/\s+/).map(parseFloat);
  if (!parts.length || parts.some(isNaN)) return String(loadavg);
  const n = cores > 0 ? cores : 4;
  const labels = [t("load-1m"), t("load-5m"), t("load-15m")];
  return parts.map((v, i) => {
      const pct = Math.round(v / n * 100);
      const color = pct > 200 ? "#dc2626" : pct > 100 ? "#f59e0b" : pct > 50 ? "#eab308" : "#10b981";
      const bar = showBar === false ? "" : `<span class="load-bar"><span class="load-fill" style="width:${Math.min(100, pct)}%;background:${color}"></span></span>`;
      return `<span class="load-item">
          ${bar}
          <span class="load-pct" style="color:${color};font-weight:700;">${pct}%</span>
          <span class="load-time">${labels[i]}</span>
      </span>`;
  }).join("");
}
function setText(id, v) {
  const el = document.getElementById(id);
  if (el) el.textContent = v;
}
function fmtTotal(bytes) {
  if (!bytes) return "0 B";
  if (bytes >= 1024 * 1024 * 1024) return (bytes / 1024 / 1024 / 1024).toFixed(1) + " GB";
  if (bytes >= 1024 * 1024) return (bytes / 1024 / 1024).toFixed(1) + " MB";
  if (bytes >= 1024) return (bytes / 1024).toFixed(1) + " KB";
  return bytes + " B";
}
function escapeHtml(s) {
  return String(s).replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/"/g, "&quot;").replace(/'/g, "&#39;");
}
function showConfirm(title, msg, onOk, confirmText) {
  confirmCallback = onOk;
  // Pure-info dialog (no callback): hide the Cancel button (both buttons just close it).
  // Each showConfirm call resets it, so no restore timer is needed.
  const cancelBtn = document.getElementById("confirmCancelBtn");
  if (cancelBtn) cancelBtn.style.display = onOk === null ? "none" : "";
  document.getElementById("confirmTitle").textContent = title;
  document.getElementById("confirmMsg").textContent = msg || "";
  const okBtn = document.getElementById("confirmOkBtn");
  if (okBtn) {
      okBtn.textContent = confirmText || t("confirm-del") || "Delete";
      // custom button colors: orange for reboot, red otherwise
      okBtn.style.background = confirmText === t("reboot-node") ? "#f59e0b" : "#dc2626";
      okBtn.style.borderColor = okBtn.style.background;
  }
  document.getElementById("confirmModal").classList.add("show");
}
function confirmOk() {
  document.getElementById("confirmModal").classList.remove("show");
  const cb = confirmCallback;
  confirmCallback = null;
  if (cb) cb();
}
function closeConfirm(event) {
  if (event && event.target !== event.currentTarget) return;
  document.getElementById("confirmModal").classList.remove("show");
  confirmCallback = null;
}