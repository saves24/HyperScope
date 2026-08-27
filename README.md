# HyperScope

Lightweight self-hosted infrastructure monitoring in Rust: a web panel and a collector agent with TLS/mTLS support.

[English](README.md) | [中文](README.zh-CN.md) | [Русский](README.ru-RU.md)

![HyperScope dashboard](docs/screenshot.jpeg)

## Overview

HyperScope monitors multiple Linux and Windows machines from a central panel. The repository is a Cargo workspace with a shared core, a web panel, and the `hyper-node` collector.

```text
hyper-node (Linux or Windows collector)
        | TLS/mTLS or internal plaintext + API key
        v
hyper-panel (web aggregator and REST API, default :8088)
```

Workspace crates:

| Crate | Responsibility |
|---|---|
| `hyper-panel-core` | Shared domain models, protocol DTOs, persistence, polling and secure node networking |
| `hyper-scope` | `hyper-node` collector binary for monitored machines |
| `hyper-panel` | Axum web panel, REST API, authentication and node aggregation |

## Core Features

- Real-time CPU, memory, temperature, disk, network, process, I/O, TCP and system-log monitoring
- Docker container listing and start, stop, restart and delete operations
- SQLite history with aggregate views and CSV export
- Node management through the web panel or CLI: add, import, rename, ping and delete
- Reverse push mode over WebSocket with HTTP fallback for nodes that cannot accept inbound connections
- Multi-user access with owner-based node isolation and administrator controls
- TLS 1.3, certificate fingerprint pinning, per-node API keys and optional mutual TLS
- Linux systemd and Windows service deployment paths
- Chinese, English and Russian web UI

## Quick Start

### Linux collector and web panel

Install the collector on each monitored machine and the panel on a server:

```bash
curl -fsSL https://raw.githubusercontent.com/saves24/HyperScope/main/install.sh | sudo bash -s node
sudo hyper-node key setup
sudo systemctl enable --now hyper-node
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

Add the node address (`IP:5000`) and the complete value printed by `hyper-node key show`, including its `|SHA256:...` fingerprint. TLS and fingerprint pinning are enabled automatically. The panel itself serves HTTP; place it behind an HTTPS reverse proxy or a private network before exposing it remotely.

### Windows collector

Download the Windows collector from the project releases and run it in PowerShell:

```powershell
.\hyper-node.exe key setup <your-key>
.\hyper-node.exe serve
```

Configuration is stored in `C:\ProgramData\hyper-node`. Windows metrics use `sysinfo`; temperature uses WMI when available, and reboot/shutdown use the native Windows command.

The `deploy/` directory provides installation and uninstallation scripts for the Windows agent:

- `deploy/install-windows.bat`: install the Windows agent as a service
- `deploy/uninstall-windows.bat`: uninstall the Windows agent service

### Reverse push mode

Nodes can connect outbound to the panel instead of listening for panel requests:

```bash
hyper-node connect http://<panel-host>:8088 <node-name> <node-key>
```

The node uses a WebSocket connection first and falls back to `POST /api/push`. Register it with `hyper-panel node add ... --push` or the API's `"push": true` field.

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

- [Contributing guide](CONTRIBUTING.md)
- [Changelog](CHANGELOG.md)
- [Example node configuration](nodes.example.json)

## Security Notes

The collector uses TLS 1.3 by default with an automatically generated self-signed certificate. The panel pins the certificate fingerprint on first connection (TOFU), and each node also requires its own API key. Mutual TLS can be enforced by adding the panel client certificate fingerprint to the node trust list:

```bash
hyper-node trust add SHA256:<panel-certificate-fingerprint>
```

Do not use plaintext mode on an untrusted network. Change the default panel password before remote use and keep node and panel ports behind an appropriate firewall or private network.

## License

HyperScope is released under the [MIT License](LICENSE).

## Commands

### Web panel CLI (`hyper-panel`)

```text
hyper-panel node add <address> <key>              add node (default port 5000; batch: {addr key}{addr key}...; --tls enable encrypted connection)
hyper-panel node link [--tls|--plain] <address> <key>  connect node (--tls encrypted / --plain plaintext test; default: auto TLS when key has fingerprint)
hyper-panel node add -f <file>                    batch import nodes from file (one "address[:port] key" per line)
hyper-panel node rename <name> <new-name>         rename node
hyper-panel node ping <name>                      ping test node reachability
hyper-panel node del <name>                       remove node from config
hyper-panel node list                             list all configured nodes
hyper-panel node show <name>                      show node details (including connectivity)
hyper-panel setup [--user <username>]             reset admin account (overwrites all users, default admin, interactive password)
hyper-panel user add <username>                   add user (interactive password)
hyper-panel user del <username>                   delete user
hyper-panel user passwd <username>                change user password (interactive)
hyper-panel user rename <old> <new>               rename user
hyper-panel user list                             list all users
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
hyper-node cert show                              show current certificate SHA256 fingerprint
hyper-node mode [tls|plain]                       view or set connection mode (tls=encrypted / plain=plaintext, takes effect after restart)
hyper-node serve [--port N] [--no-tls]            start collector service, default HTTPS (auto-generates cert if missing)
                                                  default listen 0.0.0.0:5000; --no-tls downgrades to plaintext
hyper-node log retention N                        set log retention days (default 7, auto cleanup)
hyper-node log show                               show log retention config
hyper-node trust add <fingerprint>                trust a panel client certificate fingerprint (mTLS)
hyper-node trust list                             list all trusted certificate fingerprints
hyper-node trust clear                            clear all trusted certificate fingerprints
hyper-node help                                   show this help
```
