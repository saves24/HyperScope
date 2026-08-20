# HyperScope

**Lightweight self-hosted system monitoring — a Rust panel + collector with TLS/mTLS out of the box.**

[English](README.md) · [中文](README.zh-CN.md) · [Русский](README.ru-RU.md)

![HyperScope dashboard](docs/screenshot.jpeg)

HyperScope monitors all your machines from a single web dashboard. It is built from two
small static binaries — no Docker, no database, no JavaScript framework, no external
dependencies beyond systemd.

| Component | What it does | Runs on |
|---|---|---|
| **hyper-panel** | Web dashboard & aggregator. Polls every node, serves the UI, manages nodes via web or CLI. | your server (e.g. VPS) |
| **hyper-node** | Collector agent. Exposes CPU / memory / temperature / disk / network / processes / logs / ports over an authenticated API. | every monitored machine |

---

## Features

- **Real-time monitoring** — CPU, memory, temperature (incl. GPU: NVIDIA/AMD), disk, network, processes, listening ports, system logs per node
- **Docker management** — per-node container list (name / image / state / ports), start / stop / restart from the web UI
- **Disk I/O + connections** — disk read/write rate (MB/s) and live TCP connection count with real-time line charts
- **Secure by default**:
  - TLS 1.3 (rustls) with auto-generated self-signed certificates (10 years)
  - Certificate fingerprint pinning — TOFU on first connect, then fixed (no MITM)
  - **TLS 1.3 by default**, with optional mutual TLS — add the panel client certificate fingerprint to the node trust list (`hyper-node trust add SHA256:...`) to require mTLS
  - Per-node API keys; `key setup` prints `key|SHA256:fingerprint` so TLS + pinning enable themselves
  - HttpOnly cookies, argon2 password hashes, constant-time comparisons, unified 401 (no user enumeration), per-username login rate limit (5 fails / min lock)
- **Multi-user** — owner-based node isolation; admins manage users; normal users only see their own nodes
- **Keep-alive connection pool** — Example: 22 nodes polled in ~35 ms on a low-latency LAN with keep-alive connections
- **Remote reboot / shutdown** — from the web UI or CLI, over TLS, with a confirmation dialog
- **Protocol probing** — adding a node with a plain key against a TLS-only server is rejected with a clear reason
- **Node management via CLI or web** — add (single/batch/file), rename, ping, delete
- **i18n** — Chinese / English / Russian UI
- **Real-time trends** — traffic/rate curves, temperature history, TOP processes, disk I/O + TCP candles
- **History** — SQLite-persisted CPU/MEM/DISK/TEMP/NET/TCP (90d), 1h/24h/7d/30d aggregate views, CSV export, follows the selected node
- **Mobile UI** — single-column layout, touch-friendly targets

---

## Platform Support

| Feature | Linux | Windows |
|---|---|---|
| CPU / Memory / Disk | ✅ | ✅ |
| Network / TCP | ✅ | ✅ |
| Processes | ✅ | ✅ |
| Temperature | ✅ | ⚠️ (WMI, may be N/A) |
| GPU | NVIDIA / AMD | NVIDIA (nvidia-smi) |
| Wi-Fi (SSID/signal) | — | ✅ (netsh) |
| Event Log | system logs | Windows Event Log |
| Docker | docker.sock | Docker Desktop (CLI) |
| Listening ports | — | ✅ (netstat) |
| Reboot / Shutdown | ✅ | ✅ |
| Run as service | systemd | Windows Service |
| Config paths | /etc/hyper-node | C:\\ProgramData\\hyper-node |

## Quick Start

### 1. Install a collector on every machine you want to monitor

One command — the script detects your architecture and installs the binary + systemd service:

```bash
curl -fsSL https://raw.githubusercontent.com/saves24/HyperScope/main/install.sh | sudo bash -s node
```

Then set your API key and start the service:

```bash
sudo hyper-node key setup          # create the API key (+ TLS certificate for encrypted mode)
sudo systemctl start hyper-node   # (the installer already enabled it)
sudo hyper-node key show          # output: <key>|SHA256:<fingerprint>
```

> The collector listens on `0.0.0.0:5000`. It runs in **TLS mode by default** —
> a self-signed certificate is generated on first start. For internal networks you can
> switch to plaintext with `hyper-node mode plain`.

### 2. Install the panel on your server

```bash
curl -fsSL https://raw.githubusercontent.com/saves24/HyperScope/main/install.sh | sudo bash -s panel
sudo systemctl start hyper-panel
```

### 3. Log in and add nodes

Open `http://<your-server>:8088` and log in with the default admin account:

> **Default admin: `admin` / `admin` — change it immediately!**
> `sudo hyper-panel user passwd admin`

Add a node: paste the node's address (`IP:5000`) and the **full key from `hyper-node key show`**
(including the `|SHA256:...` part). The panel automatically enables TLS + fingerprint pinning —
zero extra steps.

---

### Windows collector

Download `hyper-node-windows-amd64.exe` from Releases and run in PowerShell:

```powershell
# one-time key setup, then start
.\hyper-node.exe key setup <your-key>
.\hyper-node.exe serve
```

Config lives in `C:\ProgramData\hyper-node` (key, mode, cert). CPU/memory/disk/network/process metrics via sysinfo, temperature via WMI, reboot/shutdown via `shutdown /r|/s`.


### Reverse push mode (no listening port)

A node can run without opening any inbound port: it connects **out** to the panel and pushes metrics.

```bash
hyper-node connect http://<panel-host>:8089 <node-name> <node-key>
```

- Primary: WebSocket long connection (real-time, 5s)
- Fallback: HTTP POST to `/api/push` if the WebSocket is unavailable
- Register the node with `--push` flag (`hyper-panel node add ... --push`) or `"push": true` in the API
- Node status/data is served from the panel cache

## Installation Script

The installer supports two roles:

```bash
# Install the collector (hyper-node) — run on every monitored machine
curl -fsSL https://raw.githubusercontent.com/saves24/HyperScope/main/install.sh | sudo bash -s node

# Install the panel (hyper-panel) — run on the monitoring server
curl -fsSL https://raw.githubusercontent.com/saves24/HyperScope/main/install.sh | sudo bash -s panel
```

What it does:
- Detects architecture (`amd64` / `arm64`) and downloads the matching binary from GitHub Releases
- Installs the binary to `/usr/local/bin/`
- Creates config directories (`/etc/hyper-node`, `/etc/hyper-panel`)
- Installs and enables the systemd service (starts on boot, restarts on failure)

Manual install (alternative to the script):

```bash
# collector
sudo cp hyper-node /usr/local/bin/
sudo cp hyper-node.service /etc/systemd/system/
sudo systemctl enable --now hyper-node

# panel
sudo cp hyper-panel /usr/local/bin/
sudo cp hyper-panel.service /etc/systemd/system/
sudo systemctl enable --now hyper-panel
```

---

## Usage

### Panel CLI (`hyper-panel`)

```text
hyper-panel node list                           list nodes
hyper-panel node add <addr> <key>               add node (port defaults to 5000)
hyper-panel node add {addr1 key1}{addr2 key2}   batch add
hyper-panel node add -f <file>                  import from file ("addr[:port] key" per line)
hyper-panel node link --tls <addr> <key>        connect via TLS
hyper-panel node link --plain <addr> <key>      connect via plaintext (testing)
hyper-panel node rename <name> <new>            rename node
hyper-panel node ping <name>                    ping a node
hyper-panel node show <name>                    node details
hyper-panel node del <name>                     remove node
hyper-panel user add <name>                     add user (interactive password)
hyper-panel user passwd <name>                  change password
hyper-panel user rename <old> <new>             rename user
hyper-panel user del <name>                     delete user
hyper-panel port [N]                            view / change panel port
hyper-panel log show | log system [N]           panel logs / host service logs
hyper-panel setup                               reset admin account
```

### Collector CLI (`hyper-node`)

```text
hyper-node key setup [KEY]                      set API key (--plain for legacy plain key)
hyper-node key show                             show key (with certificate fingerprint)
hyper-node cert gen | cert show                 manage self-signed TLS certificate
hyper-node trust add <SHA256:fp>                trust a panel client certificate (mTLS)
hyper-node trust list | trust clear             manage trust list
hyper-node mode [tls|plain]                     view / switch connection mode (restart to apply)
hyper-node serve [--port N] [--no-tls]          start collector (HTTPS by default)
hyper-node log retention <days> | log show      manage local logs
```

---

## Architecture

```
┌────────────────────────────────────────────────┐
│ Monitored machines                            │
│  hyper-node (collector, port 5000)            │
│  ├─ system stats: CPU / mem / temp / disk /   │
│  │   net / processes / io / docker            │
│  ├─ Bearer key auth                           │
│  ├─ TLS: self-signed cert + mTLS trust list   │
│  └─ axum API (HTTPS by default)               │
└───────────────┬────────────────────────────────┘
                │ HTTPS/mTLS + Bearer key (or plaintext on internal nets)
┌───────────────▼────────────────────────────────┐
│ Server (VPS, ...)                │
│  hyper-panel (aggregator, port 8088)           │
│  ├─ poller: snapshot → concurrent fetch →      │
│  │   write-back (22 nodes in ~35 ms)           │
│  ├─ keep-alive connection pool                 │
│  ├─ TLS client: TOFU fingerprint pinning +     │
│  │   client certificate for mTLS               │
│  ├─ multi-user auth (cookie token, isolation)  │
│  ├─ node mgmt: CRUD / rename / ping / reboot / │
│  │   shutdown / docker control                 │
│  └─ protocol probing on add                    │
└────────────────────────────────────────────────┘
```

> **Panel HTTP is not TLS-encrypted by itself** — do not expose it directly to the public internet.
> For remote access use an HTTPS reverse proxy (Caddy/Nginx) or a private overlay like Tailscale.

## Security Model

| Layer | Mechanism |
|---|---|
| Transport | TLS 1.3 (rustls), auto self-signed cert (10 years) |
| Server identity | Certificate fingerprint pinning (TOFU then fixed) |
| Client identity | Optional mutual TLS — add panel cert fingerprint to node trust list (`hyper-node trust add`) to require mTLS |
| Application | Per-node Bearer API key |
| Process isolation | hyper-panel runs as a dedicated non-root system user (hyperscope); hyper-node as root |

The node's certificate fingerprint is encoded into the key (`key|SHA256:fingerprint`),
so pasting the full key into the panel automatically enables TLS + pinning.

## Configuration Paths

- Panel: `/etc/hyper-panel/` — `nodes.json`, `auth.json`, `panel.json`, client cert; logs in `/var/log/hyper-panel/`
- Collector: `/etc/hyper-node/` — `key`, `cert.pem`, `key.pem`, `trust.json`, `mode`; logs in `/var/log/hyper-node/`

## Notes

- Nodes expose port `5000` by default. On public networks use TLS mode (default) or Tailscale —
  plaintext mode can be sniffed.
- Change the default admin password (`admin`/`admin`) immediately after first login.
- For mutual TLS with a *second* panel, add that panel's client cert fingerprint on each node:
  `hyper-node trust add <SHA256:fp>` (the panel cert auto-generates at `/etc/hyper-panel/client-cert.pem`).

## History

HyperScope originated from a small ~200-line Python Docker service named `temp-api`, initially designed to expose weather information through an HTTP API for terminal usage and web embedding. The project was subsequently expanded and substantially redesigned with AI-assisted development, eventually becoming the current Rust-based HyperScope platform.

HyperScope is developed with extensive AI assistance. The architecture, feature direction, testing, review, debugging, and final engineering decisions are decided by the project author.

## License

MIT
