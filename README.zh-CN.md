# HyperScope

使用 Rust 构建的轻量级自托管基础设施监控平台：包含网页面板和支持 TLS/mTLS 的采集端。

[English](README.md) | [中文](README.zh-CN.md) | [Русский](README.ru-RU.md)

![HyperScope 面板](docs/screenshot.jpeg)

## 项目概览

HyperScope 从一个中心面板监控多台 Linux 和 Windows 机器。仓库是一个 Cargo workspace，包含共享核心库、网页面板，以及 `hyper-node` 采集端。

```text
hyper-node（Linux 或 Windows 采集端）
        | TLS/mTLS 或内网明文 + API key
        v
hyper-panel（网页聚合服务与 REST API，默认 :8088）
```

Workspace crate：

| Crate | 职责 |
|---|---|
| `hyper-panel-core` | 共享领域模型、协议 DTO、持久化、轮询和安全节点网络能力 |
| `hyper-scope` | 运行在被监控机器上的 `hyper-node` 采集端二进制 |
| `hyper-panel` | Axum 网页面板、REST API、认证和节点聚合 |

## 核心功能

- 实时监控 CPU、内存、温度、磁盘、网络、进程、I/O、TCP 和系统日志
- Docker 容器列表，以及启动、停止、重启和删除操作
- SQLite 历史数据、聚合视图和 CSV 导出
- 通过网页面板或 CLI 管理节点：添加、导入、重命名、ping 和删除
- 反向推送模式：优先使用 WebSocket，并在无法建立连接时回退到 HTTP
- 多用户访问、按归属隔离节点以及管理员控制
- TLS 1.3、证书指纹固定、每节点 API key 和可选的双向 TLS（mTLS）
- Linux systemd 和 Windows 服务部署路径
- 中文、英文和俄文网页界面

## 快速开始

### Linux 采集端和网页面板

在每台被监控机器上安装采集端，在一台服务器上安装面板：

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

打开 `http://<服务器>:8088`，使用初始账户 `admin` / `admin` 登录，并立即修改密码：

```bash
sudo hyper-panel user passwd admin
```

添加节点地址（`IP:5000`）以及 `hyper-node key show` 输出的完整内容，包括 `|SHA256:...` 指纹。面板会自动启用 TLS 和指纹固定。面板自身提供的是 HTTP；远程暴露前请放在 HTTPS 反向代理或私有网络之后。

### Windows 采集端

从项目 Release 下载 Windows 采集端，并在 PowerShell 中运行：

```powershell
.\hyper-node.exe key setup <你的key>
.\hyper-node.exe serve
```

配置保存在 `C:\ProgramData\hyper-node`。Windows 指标使用 `sysinfo`，温度在可用时使用 WMI，重启和关机使用 Windows 原生命令。

项目根目录的 `deploy/` 目录下提供了安装和卸载脚本：

- `deploy/install-windows.bat`：用于安装 Windows 采集端服务
- `deploy/uninstall-windows.bat`：用于卸载 Windows 采集端服务

#### 脚本安装（推荐）

脚本会把采集端注册为 Windows 服务（开机自启、无需登录）。**以管理员身份**打开 PowerShell，在仓库 `deploy/` 目录下执行：

```bat
:: 带指定 key 安装
deploy\install-windows.bat <你的API Key>

:: 或先不设 key（之后手动设置）
deploy\install-windows.bat
```

脚本会依次完成：

1. 从最新 Release 下载 `hyper-node-windows-amd64.exe` 到 `C:\ProgramData\hyper-node\`
2. 设置 API Key（传入参数时），并把 key 文件权限收紧为仅 `SYSTEM`/`Administrators` 可读
3. 开放入站防火墙规则 TCP `5000`（仅专用/域网络）
4. 注册并启动 `hyper-node` 服务（自动启动）

安装后验证并获取 Key：

```powershell
sc query hyper-node                          # 服务状态（应为 RUNNING）
C:\ProgramData\hyper-node\hyper-node.exe key show   # 复制完整值（含 |SHA256:... 指纹）
```

然后在面板添加节点：地址 `IP:5000` + 完整 Key。

卸载（同样以管理员身份）：

```bat
deploy\uninstall-windows.bat
```

注意事项：

- 脚本下载使用 `Invoke-WebRequest`；若被 PowerShell 拦截，先允许 TLS 1.2：`[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12`
- 服务无需登录用户即可运行。如需前台运行（如调试），用 `hyper-node.exe serve`
- 若需纯出站（不开放入站端口），见下方"反向推送模式"
- Windows 指标使用 `sysinfo`，温度在可用时使用 WMI，重启和关机使用 Windows 原生命令。

### 反向推送模式

节点可以主动出站连接面板，而不必监听面板请求：

```bash
hyper-node connect http://<面板地址>:8088 <节点名> <节点key>
```

节点优先使用 WebSocket，并在不可用时回退到 `POST /api/push`。添加节点时使用 `hyper-panel node add ... --push`，或在 API 中设置 `"push": true`。

## 技术栈

- Rust 2021 与 resolver 2 的 Cargo workspace
- 面板和采集端服务使用 Axum、Tokio
- 使用 reqwest、rustls、tokio-tungstenite 实现认证 HTTP/TLS/WebSocket 传输
- 使用 SQLite 保存历史数据
- 使用 serde/serde_json 定义协议 DTO
- 使用 sysinfo 及 Linux、Windows 平台集成采集指标

## 平台支持

| 能力 | Linux | Windows |
|---|---|---|
| CPU、内存、磁盘、网络、进程 | 支持 | 支持 |
| 磁盘 I/O 和 TCP 连接 | 支持 | 支持 |
| 温度 | 原生传感器 | WMI（可用时） |
| GPU 温度 | NVIDIA / AMD 集成 | 通过 `nvidia-smi` 支持 NVIDIA |
| Wi-Fi SSID 和信号 | 未提供 | `netsh` |
| 日志 | 系统日志 | Windows 事件日志 |
| Docker | Docker socket/CLI | Docker Desktop CLI |
| 监听端口 | 取决于平台 | `netstat` |
| 重启和关机 | 支持 | 支持 |
| 服务部署 | systemd | Windows 服务 |

## 文档

- [贡献指南](CONTRIBUTING.md)
- [变更日志](CHANGELOG.md)
- [节点配置示例](nodes.example.json)

## 安全说明

采集端默认使用 TLS 1.3，并自动生成自签证书。面板首次连接时会固定证书指纹（TOFU），每个节点还要求自己的 API key。将面板客户端证书指纹加入节点信任列表即可启用双向 TLS：

```bash
hyper-node trust add SHA256:<面板证书指纹>
```

不要在不可信网络中使用明文模式。远程使用前请修改默认面板密码，并通过防火墙或私有网络保护节点和面板端口。

## 许可证

HyperScope 使用 [MIT License](LICENSE) 发布。

## 命令

### 网页面板 CLI（`hyper-panel`）

```text
hyper-panel node add <地址> <key>              添加节点（默认端口 5000；支持批量：{地址 key}{地址 key}...；--tls 启用加密连接）
hyper-panel node link [--tls|--plain] <地址> <key>  连接节点（--tls 加密 / --plain 明文测试；默认：key 包含指纹时自动 TLS）
hyper-panel node add -f <文件>                  从文件批量导入节点（每行 "地址[:端口] key"）
hyper-panel node rename <名称> <新名称>          重命名节点
hyper-panel node ping <名称>                    测试节点可达性
hyper-panel node del <名称>                     从配置中删除节点
hyper-panel node list                           列出所有已配置节点
hyper-panel node show <名称>                    显示节点详情（包括连接状态）
hyper-panel setup [--user <用户名>]             重置管理员账户（覆盖所有用户，默认 admin，交互式密码）
hyper-panel user add <用户名>                   添加用户（交互式密码）
hyper-panel user del <用户名>                   删除用户
hyper-panel user passwd <用户名>                修改用户密码（交互式）
hyper-panel user rename <旧名> <新名>           重命名用户
hyper-panel user list                           列出所有用户
hyper-panel port [N]                            查看/设置面板端口（默认 8088，重启生效）
hyper-panel log show [N]                        查看面板日志（最后 N 行，默认 50）
hyper-panel log system [N]                      查看主机 systemd 服务日志（journalctl -u hyper-panel，默认 50）
hyper-panel log retention <天数>                设置日志保留天数（默认 7）
hyper-panel serve [--port N]                    启动聚合服务（默认 8088）
hyper-panel help                                显示帮助
```

### 采集端 CLI（`hyper-node`）

```text
hyper-node key setup [KEY] [--plain]            设置 API key。未指定 KEY 时自动生成随机 key。
                                                 默认生成证书绑定 key（key 包含证书指纹，适用于 TLS 节点）；
                                                 --plain 生成传统明文 key（适用于非 TLS 节点）
hyper-node key show                             显示当前 API key（含证书指纹格式）
hyper-node cert gen                             生成/续期 TLS 证书（自签名，写入 /etc/hyper-node/）
hyper-node cert show                            显示当前证书 SHA256 指纹
hyper-node mode [tls|plain]                     查看或设置连接模式（tls=加密 / plain=明文，重启后生效）
hyper-node serve [--port N] [--no-tls]          启动采集服务，默认 HTTPS（缺失时自动生成证书）
                                                 默认监听 0.0.0.0:5000；--no-tls 降级为明文
hyper-node log retention N                      设置日志保留天数（默认 7，自动清理）
hyper-node log show                             显示日志保留配置
hyper-node trust add <指纹>                     信任面板客户端证书指纹（mTLS）
hyper-node trust list                           列出所有已信任的证书指纹
hyper-node trust clear                          清除所有已信任的证书指纹
hyper-node help                                 显示帮助
```
