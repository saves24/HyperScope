# HyperScope

**轻量自托管系统监控 —— Rust 编写的面板 + 采集端，开箱即用 TLS/mTLS。**

[English](README.md) · [中文](README.zh-CN.md) · [Русский](README.ru-RU.md)

![HyperScope 面板](docs/screenshot.jpeg)

HyperScope 用一个网页面板监控你所有的机器。它由两个小体积静态二进制组成——无 Docker、无数据库、无前端框架、无外部依赖（只需 systemd）。

| 组件 | 功能 | 运行位置 |
|---|---|---|
| **hyper-panel** | 网页面板与聚合服务。轮询所有节点、提供界面、网页/CLI 管理节点。 | 你的服务器（如 VPS） |
| **hyper-node** | 采集端。通过带认证的 API 上报 CPU/内存/温度/磁盘/网络/进程/日志/端口。 | 每台被监控机器 |

---

## 特性

- **实时监控** — 每节点 CPU、内存、温度（含 GPU：NVIDIA/AMD）、磁盘、网络、进程、监听端口、系统日志
- **Docker 管理** — 每节点容器列表（名称/镜像/状态/端口），网页端一键启动/停止/重启
- **磁盘 IO + 连接数** — 磁盘读写速率（MB/s）+ 实时 TCP 连接数，带折线图
- **默认安全**：
  - TLS 1.3（rustls），自动生成自签证书（10 年）
  - 证书指纹固定 — 首次连接 TOFU，之后固定校验（防中间人）
  - **双向 TLS** — 节点通过信任列表校验面板客户端证书
  - 每节点 API key；`key setup` 输出 `密钥|SHA256:指纹`，TLS + 指纹自动生效
  - HttpOnly cookie、argon2 密码哈希、恒定时间比较、统一 401（防用户名枚举）、按用户名登录限流（5 次失败/分钟锁定）
- **多用户** — 基于归属的节点隔离；admin 管理用户；普通用户只能看到自己的节点
- **keep-alive 连接池** — 22 节点轮询仅 ~35ms
- **远程重启/关机** — 网页或 CLI，走 TLS，带确认弹窗
- **协议探测** — 明文 key 连 TLS-only 节点会被拒绝并给出明确原因
- **节点管理（CLI/网页）** — 添加（单个/批量/文件）、重命名、ping、日志、删除
- **国际化** — 中文/英文/俄文界面
- **实时趋势** — 流量速率曲线、温度历史、进程 TOP、磁盘 IO + TCP 连接
- **历史数据** — SQLite 持久化 CPU/内存/磁盘/温度/网络/TCP（90 天），1h/24h/7d/30d 聚合视图、CSV 导出、跟随当前选中节点
- **移动端 UI** — 单列布局、触控优化

---

## 平台支持

| 功能 | Linux | Windows |
|---|---|---|
| CPU / 内存 / 磁盘 | ✅ | ✅ |
| 网络 / TCP | ✅ | ✅ |
| 进程 | ✅ | ✅ |
| 温度 | ✅ | ⚠️（WMI，可能 N/A） |
| GPU | NVIDIA / AMD | NVIDIA（nvidia-smi） |
| Wi-Fi（SSID/信号） | — | ✅（netsh） |
| 事件日志 | 系统日志 | Windows 事件日志 |
| Docker | docker.sock | Docker Desktop（CLI） |
| 监听端口 | — | ✅（netstat） |
| 重启 / 关机 | ✅ | ✅ |
| 服务运行 | systemd | Windows 服务 |
| 配置路径 | /etc/hyper-node | C:\\ProgramData\\hyper-node |

## 快速上手

### 1. 在每台要监控的机器上安装采集端

一条命令——脚本自动识别架构并安装二进制 + systemd 服务：

```bash
curl -fsSL https://raw.githubusercontent.com/saves24/HyperScope/main/install.sh | sudo bash -s node
```

然后设置 key 并启动：

```bash
sudo hyper-node key setup          # 创建 API key（加密模式会自动生成 TLS 证书）
sudo systemctl start hyper-node   #（安装脚本已 enable）
sudo hyper-node key show          # 输出: <密钥>|SHA256:<指纹>
```

> 采集端监听 `0.0.0.0:5000`，**默认 TLS 加密模式**（首次启动自动生成自签证书）。
> 内网可切明文：`hyper-node mode plain`。

### 2. 在服务器上安装面板

```bash
curl -fsSL https://raw.githubusercontent.com/saves24/HyperScope/main/install.sh | sudo bash -s panel
sudo systemctl start hyper-panel
```

### 3. 登录并添加节点

打开 `http://<服务器>:8088`，用默认管理员登录：

> **默认管理员：`admin` / `admin` —— 请立即修改！**
> `sudo hyper-panel user passwd admin`

添加节点：粘贴节点地址（`IP:5000`）和 `hyper-node key show` 输出的**完整 key**（含 `|SHA256:...` 部分）——面板自动启用 TLS + 指纹固定，零额外步骤。

---

### Windows 采集端

从 Release 下载 `hyper-node-windows-amd64.exe`，在 PowerShell 中运行：

```powershell
# 首次设置 key，然后启动
.\hyper-node.exe key setup <你的key>
.\hyper-node.exe serve
```

配置位于 `C:\ProgramData\hyper-node`（key、mode、证书）。指标通过 sysinfo 采集（CPU/内存/磁盘/网络/进程），温度走 WMI，重启/关机通过 `shutdown /r|/s`。


### 反向推送模式（无需监听端口）

节点可以不开放任何入站端口——它主动**出站**连接面板并推送指标。

```bash
hyper-node connect http://<面板地址>:8089 <节点名> <节点key>
```

- 主通道：WebSocket 长连接（实时，5 秒）
- 备用：WebSocket 不可用时 HTTP POST 到 `/api/push`
- 添加节点时用 `--push` 标志（`hyper-panel node add ... --push`）或 API 的 `"push": true`
- 节点状态/数据从面板缓存读取

## 安装脚本

安装脚本支持两种角色：

```bash
# 安装采集端（hyper-node）—— 每台被监控机器执行
curl -fsSL https://raw.githubusercontent.com/saves24/HyperScope/main/install.sh | sudo bash -s node

# 安装面板（hyper-panel）—— 监控服务器执行
curl -fsSL https://raw.githubusercontent.com/saves24/HyperScope/main/install.sh | sudo bash -s panel
```

脚本会：
- 自动识别架构（amd64 / arm64），从 GitHub Releases 下载对应二进制
- 安装到 `/usr/local/bin/`
- 创建配置目录（`/etc/hyper-node`、`/etc/hyper-panel`）
- 安装并 enable systemd 服务（开机自启、崩溃自动重启）

手动安装（不用脚本时）：

```bash
# 采集端
sudo cp hyper-node /usr/local/bin/
sudo cp hyper-node.service /etc/systemd/system/
sudo systemctl enable --now hyper-node

# 面板
sudo cp hyper-panel /usr/local/bin/
sudo cp hyper-panel.service /etc/systemd/system/
sudo systemctl enable --now hyper-panel
```

---

## 命令参考

### 面板 CLI（hyper-panel）

```text
hyper-panel node list                           查看节点
hyper-panel node add <地址> <key>               添加节点（默认端口 5000）
hyper-panel node add {地址1 key1}{地址2 key2}   批量添加
hyper-panel node add -f <文件>                  从文件导入（每行 "地址[:端口] key"）
hyper-panel node link --tls <地址> <key>        TLS 连接
hyper-panel node link --plain <地址> <key>      明文连接（测试）
hyper-panel node rename <名称> <新名>            重命名
hyper-panel node ping <名称>                    ping 测试
hyper-panel node show <名称>                    节点详情
hyper-panel node del <名称>                     删除节点
hyper-panel user add <名称>                     添加用户（交互输入密码）
hyper-panel user passwd <名称>                  修改密码
hyper-panel user rename <旧名> <新名>            重命名用户
hyper-panel user del <名称>                     删除用户
hyper-panel port [N]                            查看/修改面板端口
hyper-panel log show | log system [N]           面板日志 / 宿主机服务日志
hyper-panel setup                               重置管理员
```

### 采集端 CLI（hyper-node）

```text
hyper-node key setup [KEY]                      设置 API key（--plain 生成纯明文 key）
hyper-node key show                             查看 key（含证书指纹）
hyper-node cert gen | cert show                 管理自签 TLS 证书
hyper-node trust add <SHA256:指纹>               信任面板客户端证书（mTLS）
hyper-node trust list | trust clear             管理信任列表
hyper-node mode [tls|plain]                     查看/切换连接模式（重启生效）
hyper-node serve [--port N] [--no-tls]          启动采集端（默认 HTTPS）
hyper-node log retention <天数> | log show       管理本地日志
```

---

## 架构

```
┌────────────────────────────────────────────────┐
│ 被监控机器                                     │
│  hyper-node（采集端，端口 5000）                │
│  ├─ 系统数据：CPU/内存/温度/磁盘/网络/进程/      │
│  │   日志/端口                                 │
│  ├─ Bearer key 认证                            │
│  ├─ TLS：自签证书 + mTLS 信任列表               │
│  └─ axum API（默认 HTTPS）                     │
└───────────────┬────────────────────────────────┘
                │ HTTPS/mTLS + Bearer key（内网可明文）
┌───────────────▼────────────────────────────────┐
│ 服务器（VPS……）                         │
│  hyper-panel（聚合服务，端口 8088）              │
│  ├─ poller：快照 → 并发拉取 → 回写               │
│  │   （22 节点约 35ms）                         │
│  ├─ keep-alive 连接池                          │
│  ├─ TLS 客户端：TOFU 指纹固定 + 客户端证书（mTLS）│
│  ├─ 多用户认证（cookie token、节点隔离）         │
│  ├─ 节点管理：增删改/重命名/ping/日志/重启/关机   │
│  └─ 添加节点时协议探测                          │
└────────────────────────────────────────────────┘
```

> **面板 HTTP 本身无 TLS 加密** — 请勿直接暴露到公网。
> 远程访问请使用 HTTPS 反向代理（Caddy/Nginx）或 Tailscale 等私有网络。

## 安全模型

| 层级 | 机制 |
|---|---|
| 传输 | TLS 1.3（rustls），自动自签证书（10 年） |
| 服务端身份 | 证书指纹固定（TOFU 后固定） |
| 客户端身份 | 可选双向 TLS（mTLS）— 执行 `hyper-node trust add` 将面板证书指纹加入节点信任列表后强制 mTLS |
| 应用层 | 每节点 Bearer API key |
| 进程隔离 | hyper-panel 以专用非 root 系统用户（hyperscope）运行；hyper-node 以 root 运行 |

节点证书指纹编码进 key（`密钥|SHA256:指纹`）——面板粘贴完整 key 即自动启用 TLS + 指纹固定。

## 配置路径

- 面板：`/etc/hyper-panel/` — `nodes.json`、`auth.json`、`panel.json`、客户端证书；日志在 `/var/log/hyper-panel/`
- 采集端：`/etc/hyper-node/` — `key`、`cert.pem`、`key.pem`、`trust.json`、`mode`；日志在 `/var/log/hyper-node/`

## 注意事项

- 节点默认监听 `5000` 端口。公网请用 TLS 模式（默认）或 Tailscale——明文模式可被嗅探。
- 首次登录后必须修改默认管理员密码（`admin`/`admin`）。
- 如需第二块面板接入双向 TLS：在每台节点上添加该面板客户端证书指纹 `hyper-node trust add <SHA256:指纹>`（面板证书自动生成于 `/etc/hyper-panel/client-cert.pem`）。

## 项目历程

HyperScope 源自一个约 200 行的小型 Python Docker 服务 `temp-api`，最初通过 HTTP API 提供天气信息，供终端使用和网页嵌入。项目随后在 AI 辅助开发下持续扩展并大幅重构，最终成为现在的 Rust 版 HyperScope 平台。

HyperScope 的开发大量借助 AI 辅助。架构、功能方向、测试、审查、调试以及最终工程决策均由项目作者抉择。

## 许可

MIT
