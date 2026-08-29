# HyperScope

Decentralized self-hosted infrastructure monitoring in Rust: a web panel plus per-machine control planes (hyper-relay) and on-demand collectors (hyper-node), with a Tailscale-inspired control-plane/data-plane split and full TLS (WSS).

[English](README.md) | [中文](README.zh-CN.md) | [Русский](README.ru-RU.md)

![HyperScope dashboard](docs/screenshot.jpeg)

## Overview

HyperScope monitors multiple Linux and Windows machines through a central panel. Its architecture is inspired by **Tailscale**: the **control plane is separated from the data plane** — the control plane (hyper-relay) does signaling only (waking the collector on demand), while each node collects data locally without depending on a central server for continuous forwarding. The project is a Cargo workspace containing a shared core, a web panel, and the `hyper-node` collector.

**Decentralized**: every monitored machine ships its own control plane (hyper-relay) and collector (hyper-node); the panel acts only as an observer — even if the panel is offline, every node keeps running and can be woken by itself or by a peer at any time.

```text
hyper-node (Linux or Windows collector, non-resident, no listening port)
        ^  woken on demand (local process)
        |
hyper-relay (per-machine control plane, the only resident service, signaling only, default :8686)
        ^  WSS/TLS control-plane connection
        |
hyper-panel (web aggregator and REST API, default :8088)
```

Workspace crates:

| Crate | Responsibility |
|---|---|
| `hyper-panel-core` | Shared domain models, protocol DTOs, persistence, polling and secure node networking |
| `hyper-scope` | `hyper-node` collector binary for monitored machines |
| `hyper-panel` | Axum web panel, REST API, authentication and node aggregation |

## Core Features

- **Decentralized architecture**: control plane separated from data plane (Tailscale-inspired) — the control plane does signaling only; the collector is woken on demand and never resident
- Real-time CPU, memory, temperature, disk, network, process, I/O, TCP and system-log monitoring
- Docker container listing and start, stop, restart and delete operations
- SQLite history with aggregate views and CSV export
- Node management through the web panel or CLI: add, import, rename, ping and delete
- Relay mode: the collector opens no listening port — hyper-relay (the only
  resident service) wakes it on demand for each poll
- Administrator account (admin password hashing via argon2, login rate limiting)
- TLS 1.3 (WSS), certificate fingerprint pinning, per-node API keys
- Linux systemd and Windows service deployment paths
- Chinese, English and Russian web UI
- **Alert detection** (CPU / memory / disk / temperature thresholds and non-running Docker containers) with a bell notification panel, persistent to disk and fully separate from the event log
- **Webhook alert delivery**: per-node notify channel (PushPlus / Server Chan / Telegram / custom webhook) plus configurable per-node thresholds. Alerts are delivered to mobile devices or messaging applications without manual supervision
- **Node grouping** with filter chips above the node list
- **Audit log**: admin actions (node deleted, user password changed) recorded into the event stream with actor and timestamp
- **Node manager dialog** in the web UI: single add, batch add, batch delete with checkboxes, and batch export to an encrypted `.hsxc` config file
- **Local encrypted config import/export** (`.hsxc`): AES-256-GCM + PBKDF2, fully compatible between the web panel and the Android app, decrypts locally and nothing leaves the device
- **Android client** (`android/`): a local dashboard that connects to each hyper-node directly or via its hyper-relay (no listening port required), with semi-circular speed gauges, CPU/memory trend charts (persisted across restarts), custom card ordering, health summary badge, node groups, .hsxc export, system notifications for alerts, Material You dynamic colors and a machine control tab (reboot / shutdown / view & stop processes / Docker start-stop-restart)

## Quick Start

### Linux collector and web panel

Install the collector on each monitored machine and the panel on a server:

```bash
curl -fsSL https://raw.githubusercontent.com/saves24/HyperScope/main/install.sh | sudo bash -s node
sudo hyper-node key setup
sudo systemctl enable --now hyper-relay
sudo hyper-node key show
```

```bash
curl -fsSL https://raw.githubusercontent.com/saves24/HyperScope/main/install.sh | sudo bash -s panel
sudo systemctl enable --now hyper-panel
```

Open `http://<server>:8088`, sign in with the initial `admin` / `admin` account, and change the password immediately:

```bash
sudo hyper-panel user passwd admin
```

Add the node address and the complete value printed by `hyper-node key show`, including its `|SHA256:...` fingerprint. TLS and fingerprint pinning are enabled automatically. The panel itself serves HTTP; place it behind an HTTPS reverse proxy or a private network before exposing it remotely.

**Trust controlling devices** (required for remote commands): relay commands are signed with the device Ed25519 key. On each node, add the panel/phone device to the trust list:

```bash
# on the node, with the device's public key (shown by the panel/phone):
sudo hyper-node device add <device-id> <device-pubkey> admin
sudo hyper-node device list
```

### Windows collector

Use the installer script (recommended):

```powershell
Invoke-WebRequest -Uri 'https://raw.githubusercontent.com/saves24/HyperScope/main/deploy/install-windows.bat' -OutFile install-windows.bat
.\install-windows.bat
```

Configuration is stored in `C:\ProgramData\hyper-node`. Windows metrics use `sysinfo`; temperature uses WMI when available, and reboot/shutdown use the native Windows command.

The `deploy/` directory provides installation and uninstallation scripts for the Windows collector:

- `deploy/install-windows.bat`: install the collector + hyper-relay (registers the hyper-relay service)
- `deploy/uninstall-windows.bat`: uninstall the hyper-relay service and remove files

#### Script install (recommended)

The script registers the hyper-relay service (auto-start at boot, no logon required); the collector is woken by it on demand. Run PowerShell **as Administrator** and download + run the batch file in one go:

```powershell
# download the installer, then run it with a specific key
Invoke-WebRequest -Uri 'https://raw.githubusercontent.com/saves24/HyperScope/main/deploy/install-windows.bat' -OutFile install-windows.bat
.\install-windows.bat <your-api-key>

# or let it skip key setup (set the key manually afterwards)
.\install-windows.bat
```

What the script does:

1. Downloads `hyper-node-windows-amd64.exe` and `hyper-relay-windows-amd64.exe` from the latest release into `C:\ProgramData\hyper-node\`
2. Sets the API key (when a key argument is given) and protects the key file so only `SYSTEM`/`Administrators` can read it
3. Registers and starts the `hyper-relay` service (auto start). The collector is not registered as a service — the relay wakes it on demand

After install, verify and get the key:

```powershell
sc query hyper-relay          # service state (should be RUNNING)
C:\ProgramData\hyper-node\hyper-node.exe key show   # copy the full value incl. |SHA256:... fingerprint
```

Then add the node in the panel: the node address + the full key.

Also trust the controlling device (panel/phone) so remote commands are accepted:

```powershell
C:\ProgramData\hyper-node\hyper-node.exe device add <device-id> <device-pubkey> admin
C:\ProgramData\hyper-node\hyper-node.exe device list
```

To uninstall (also as Administrator):

```powershell
Invoke-WebRequest -Uri 'https://raw.githubusercontent.com/saves24/HyperScope/main/deploy/uninstall-windows.bat' -OutFile uninstall-windows.bat
.\uninstall-windows.bat
```

Notes:

- The script uses `Invoke-WebRequest` for the download of the agent binary; if PowerShell blocks it, allow TLS 1.2 first: `[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12`
- The service runs without a logged-on user. To run in the foreground instead (e.g. for testing), use `hyper-node.exe relay`
- Windows metrics use `sysinfo`; temperature uses WMI when available, and reboot/shutdown use the native Windows command.

## Tech Stack

- Rust 2021 and Cargo workspace with resolver 2
- Axum and Tokio for the panel and collector services
- reqwest, rustls and tokio-tungstenite for authenticated HTTP/TLS/WebSocket transport
- SQLite for retained history data
- serde/serde_json for protocol DTOs
- sysinfo plus platform-specific Linux and Windows integrations for metrics

## Platform Support

| Capability | Linux | Windows |
|---|---|---|
| CPU, memory, disk, network, processes | Yes | Yes |
| Disk I/O and TCP connections | Yes | Yes |
| Temperature | Native sensors | WMI when available |
| GPU temperature | NVIDIA / AMD integrations | NVIDIA via `nvidia-smi` |
| Wi-Fi SSID and signal | Not provided | `netsh` |
| Logs | system logs | Windows Event Log |
| Docker | Docker socket/CLI | Docker Desktop CLI |
| Listening ports | Platform dependent | `netstat` |
| Reboot and shutdown | Yes | Yes |
| Service deployment | systemd | Windows Service |

## Documentation

- [Example node configuration](nodes.example.json)

## P2P Relay Protocol (hyper-relay)

Nodes run without listening on any port by using the `hyper-relay` agent:
the relay (installed on the same machine as the node) is the only persistent
service and exposes a single port; the collector is woken on demand via a
local process (`hyper-node collect` / `hyper-node control`). End-to-end
Ed25519 signatures keep commands trustworthy even if the relay or the web
panel is compromised.

- **Install**: `install.sh` / `install-windows.bat` install both `hyper-relay`
  (system service) and `hyper-node` (on-demand) on the same machine.
- **Collect**: the relay spawns `hyper-node collect` on the same machine on
  demand for each poll; the collector is not a resident service.
- **Data path**: Android/web panel talk to the collector only through the relay
  (control plane) — no direct connections; the relay wakes the local collector
  and returns a fresh snapshot.
- **Commands**: signed with the device key; high-risk actions (SSH / system
  update / package install) require a second admin confirmation.
- **TLS (WSS)**: `hyper-relay serve --tls-cert <pem> --tls-key <pem>` serves
  encrypted wss:// connections — recommended for public nodes (self-signed
  certs are accepted by the clients); LAN nodes can enable it too with no
  perceptible latency (AES hardware acceleration).
- **Certificate management**: certificates are **machine-agnostic** — generate
  once on any machine and share it across nodes (`hyper-node cert import
  <cert.pem> <key.pem>`), or generate independently per machine
  (`hyper-node cert gen`). Shared certs suit home/LAN setups (simpler
  management); independent certs are recommended for public/multi-tenant
  environments (audit, isolation, revocation).
- **Account model**: trusted-device keys are stored only on the node
  (`/etc/hyper-node/trusted.toml`); the web panel holds no keys.

## Security Notes

The panel is designed for private LAN use — do not expose it to the public internet. Keep it behind your home network / VPN and access it from a trusted device.

The collector uses TLS 1.3 by default with an automatically generated self-signed certificate. The panel pins the certificate fingerprint on first connection (TOFU), and each node also requires its own API key. Mutual TLS can be enforced by adding the panel client certificate fingerprint to the node trust list:

```bash
hyper-node trust add SHA256:<panel-certificate-fingerprint>
```

Do not use plaintext mode on an untrusted network. Change the default panel password before remote use and keep node and panel ports behind an appropriate firewall or private network.

## Commands

### Web panel CLI (`hyper-panel`)

```text
hyper-panel node add <address> <key>              add node (default port 8686; batch: {addr key}{addr key}...; --tls enable encrypted connection)
hyper-panel node link [--tls|--plain] <address> <key>  connect node (--tls encrypted / --plain plaintext test; default: auto TLS when key has fingerprint)
hyper-panel node add -f <file>                    batch import nodes from file (one "address[:port] key" per line)
hyper-panel node rename <name> <new-name>         rename node
hyper-panel node ping <name>                      ping test node reachability
hyper-panel node del <name>                       remove node from config
hyper-panel node list                             list all configured nodes
hyper-panel node show <name>                      show node details (including connectivity)
hyper-panel setup [--user <username>]             create/reset the admin account (default admin, interactive password)
hyper-panel user passwd <username>                change the admin password (interactive)
hyper-panel port [N]                              view/set panel port (default 8088, takes effect on restart)
hyper-panel log show [N]                          view panel log (last N lines, default 50)
hyper-panel log system [N]                        view host systemd service log (journalctl -u hyper-panel, default 50)
hyper-panel log retention <days>                  set log retention days (default 7)
hyper-panel serve [--port N]                      start aggregator service (default 8088)
hyper-panel help                                  show this help
```

### Collector CLI (`hyper-node`)

```text
hyper-node key setup [KEY] [--plain]              set API key. Generates random key when KEY is not given.
                                                  default generates certificate-bound key (key includes cert fingerprint, for TLS nodes);
                                                  --plain generates legacy plaintext key (for non-TLS nodes)
hyper-node key show                               show current API key (with certificate fingerprint format)
hyper-node cert gen                               generate/renew TLS certificate (self-signed, written to /etc/hyper-node/)
hyper-node cert import <cert.pem> <key.pem>       import a shared certificate
hyper-node cert show                              show current certificate SHA256 fingerprint
hyper-node identity init                          generate the Ed25519 identity key (prints public key)
hyper-node identity show                          show the identity public key
hyper-node identity sign <msg>                    sign a message with the identity key
hyper-node device list                            list trusted devices
hyper-node device add <id> <pubkey> <role>        trust a device (owner|admin|viewer)
hyper-node device remove <id>                     remove a trusted device
hyper-node relay | serve                          run the collector in relay mode (no listening port; metrics
                                                  are served on demand through hyper-relay)
hyper-node log retention N                        set log retention days (default 7, auto cleanup)
hyper-node log show                               show log retention config
hyper-node trust add <fingerprint>                trust a panel client certificate fingerprint (mTLS)
hyper-node trust list                             list all trusted certificate fingerprints
hyper-node trust clear                            clear all trusted certificate fingerprints
hyper-node help                                   show this help
```

## License

HyperScope is released under the [MIT License](LICENSE).
