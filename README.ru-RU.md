# HyperScope

Децентрализованный self-hosted мониторинг инфраструктуры на Rust: веб-панель, на каждой машине — плоскость управления (hyper-relay) и агент по требованию (hyper-node), разделение плоскости управления и данных в духе Tailscale, полная поддержка TLS (WSS).

[English](README.md) · [中文](README.zh-CN.md) · [Русский](README.ru-RU.md)

![Панель HyperScope](docs/screenshot.jpeg)

## Обзор

HyperScope мониторит несколько Linux- и Windows-машин через единую центральную панель. Архитектура вдохновлена **Tailscale**: **плоскость управления отделена от плоскости данных** — плоскость управления (hyper-relay) выполняет только сигнализацию (пробуждает агента по требованию), а данные собираются каждым узлом локально, без зависимости от центрального сервера для непрерывной пересылки. Проект представляет собой Cargo workspace, включающий общую библиотеку core, веб-панель и агента `hyper-node`.

**Децентрализация**: на каждой отслеживаемой машине установлены собственная плоскость управления (hyper-relay) и агент (hyper-node); панель выступает только как наблюдатель — даже при офлайн-панели все узлы продолжают работать и могут быть пробуждены локально или одноранговым узлом в любой момент.

```text
hyper-node (агент Linux или Windows, не резидентный, без порта)
        ^  пробуждение по требованию (локальный процесс)
        |
hyper-relay (плоскость управления на каждой машине, единственная резидентная
        |   служба, только сигнализация, по умолчанию :8686)
        ^  WSS/TLS соединение плоскости управления
        |
hyper-panel (веб-агрегатор и REST API, по умолчанию :8088)
```

Crates в составе workspace:

| Crate | Назначение |
|---|---|
| `hyper-panel-core` | Общие доменные модели, DTO протокола, персистентность, опрос и защищённые сетевые взаимодействия |
| `hyper-scope` | Бинарный агент `hyper-node` для отслеживаемых машин |
| `hyper-panel` | Веб-панель Axum, REST API, аутентификация и агрегация узлов |

## Возможности

- **Децентрализованная архитектура**: плоскость управления отделена от плоскости данных (в духе Tailscale) — контрольная плоскость выполняет только сигнализацию; агент пробуждается по требованию и не является резидентным
- Мониторинг в реальном времени: CPU, память, температура, диск, сеть, процессы, I/O, TCP и системные журналы
- Список Docker-контейнеров и операции запуска, остановки, перезапуска и удаления
- История в SQLite с агрегированными представлениями и экспортом в CSV
- Управление узлами через веб-панель или CLI: добавление, импорт, переименование, ping и удаление
- Релейный режим: агент не открывает порт — hyper-relay (единственная резидентная служба) пробуждает его по требованию для каждого опроса
- Учётная запись администратора (хэширование пароля argon2, ограничение попыток входа)
- TLS 1.3 (WSS), пиннинг отпечатка сертификата, API-ключи на узел
- Развёртывание через systemd на Linux и службы Windows
- Интерфейс на китайском, английском и русском языках
- **Обнаружение оповещений** (пороги CPU / память / диск / температура и неработающие Docker-контейнеры) с панелью уведомлений, сохранение на диск и полное отделение от журнала событий
- **Доставка оповещений через webhook**: настраиваемый канал уведомлений на узел (PushPlus / Server Chan / Telegram / пользовательский webhook) и настраиваемые пороги. Оповещения доставляются на мобильные устройства или в мессенджеры без ручного контроля
- **Группировка узлов** с фильтрами над списком узлов
- **Журнал аудита**: административные действия (удаление узла, смена пароля пользователя) записываются в поток событий с указанием исполнителя и времени
- **Диалог управления узлами** в веб-интерфейсе: одиночное добавление, пакетное добавление, пакетное удаление с флажками и пакетный экспорт в зашифрованный файл `.hsxc`
- **Локальный импорт/экспорт зашифрованной конфигурации** (`.hsxc`): AES-256-GCM + PBKDF2, полностью совместим между веб-панелью и приложением Android, расшифровка выполняется локально, данные не покидают устройство
- **Клиент Android** (`android/`): локальная панель, подключается к каждому узлу напрямую или через его hyper-relay (порт не требуется), с полукруглыми измерителями скорости, графиками трендов CPU/память (сохраняются между перезапусками), настраиваемым порядком карточек, значком состояния, группами узлов, экспортом .hsxc, системными уведомлениями об оповещениях, динамической цветовой схемой Material You и вкладкой управления машиной (перезагрузка / выключение / просмотр и остановка процессов / запуск-остановка-перезапуск Docker)

## Быстрый старт

### Агент Linux и веб-панель

Установите агент на каждой отслеживаемой машине, а панель — на сервере:

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

Откройте `http://<сервер>:8088`, войдите с учётной записью `admin` / `admin` и немедленно смените пароль:

```bash
sudo hyper-panel user passwd admin
```

Добавьте адрес узла и полное значение, выводимое командой `hyper-node key show`, включая отпечаток `|SHA256:...`. TLS и пиннинг отпечатка включаются автоматически. Панель обслуживает HTTP; перед удалённым доступом разместите её за HTTPS reverse proxy или в частной сети.

**Доверьте управляющие устройства** (обязательно для удалённых команд): команды реле подписаны ключом устройства Ed25519. На каждом узле добавьте устройство панели/телефона в список доверия:

```bash
# на узле, с открытым ключом устройства (показывается панелью/телефоном):
sudo hyper-node device add <идентификатор> <открытый-ключ> admin
sudo hyper-node device list
```

### Агент Windows

Используйте скрипт установки (рекомендуется). Скрипт регистрирует службу hyper-relay (автозапуск при загрузке, без входа в систему); агент пробуждается ею по требованию. Запустите PowerShell **от имени администратора** и скачайте и выполните bat-файл:

```powershell
# скачайте установщик, затем запустите с указанным ключом
Invoke-WebRequest -Uri 'https://raw.githubusercontent.com/saves24/HyperScope/main/deploy/install-windows.bat' -OutFile install-windows.bat
.\install-windows.bat <ваш-api-ключ>

# или пропустите настройку ключа (задайте его вручную позже)
.\install-windows.bat
```

Конфигурация хранится в `C:\ProgramData\hyper-node`. Windows-метрики используют `sysinfo`; температура — WMI при наличии; перезагрузка/выключение — нативные команды Windows.

Что делает скрипт:

1. Скачивает `hyper-node-windows-amd64.exe` и `hyper-relay-windows-amd64.exe` из последнего релиза в `C:\ProgramData\hyper-node\`
2. Устанавливает API-ключ (при передаче аргумента) и защищает файл ключа так, чтобы его могли читать только `SYSTEM`/`Administrators`
3. Регистрирует и запускает службу `hyper-relay` (автозапуск). Агент не регистрируется как служба — реле пробуждает его по требованию

После установки проверьте и получите ключ:

```powershell
sc query hyper-relay          # состояние службы (должно быть RUNNING)
C:\ProgramData\hyper-node\hyper-node.exe key show   # скопируйте полное значение, включая |SHA256:... отпечаток
```

Затем добавьте узел в панели: адрес узла + полный ключ.

Также доверьте управляющее устройство (панель/телефон), чтобы удалённые команды принимались:

```powershell
C:\ProgramData\hyper-node\hyper-node.exe device add <идентификатор> <открытый-ключ> admin
C:\ProgramData\hyper-node\hyper-node.exe device list
```

Для удаления (также от имени администратора):

```powershell
Invoke-WebRequest -Uri 'https://raw.githubusercontent.com/saves24/HyperScope/main/deploy/uninstall-windows.bat' -OutFile uninstall-windows.bat
.\uninstall-windows.bat
```

Примечания:

- Скрипт использует `Invoke-WebRequest` для скачивания бинарника агента; если PowerShell блокирует его, сначала разрешите TLS 1.2: `[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12`
- Служба работает без вошедшего пользователя. Для работы в фоновом режиме (например, при отладке) используйте `hyper-node.exe relay`
- Windows-метрики используют `sysinfo`; температура — WMI при наличии; перезагрузка/выключение — нативные команды Windows.

## Технологический стек

- Rust 2021 и Cargo workspace с resolver 2
- Axum и Tokio для панели и служб агента
- reqwest, rustls и tokio-tungstenite для аутентифицированного HTTP/TLS/WebSocket транспорта
- SQLite для хранения истории
- serde/serde_json для DTO протокола
- sysinfo и платформенные интеграции Linux/Windows для метрик

## Поддержка платформ

| Возможность | Linux | Windows |
|---|---|---|
| CPU, память, диск, сеть, процессы | Да | Да |
| Дисковый I/O и TCP-соединения | Да | Да |
| Температура | Нативные датчики | WMI при наличии |
| Температура GPU | NVIDIA / AMD интеграции | NVIDIA через `nvidia-smi` |
| Wi-Fi SSID и сигнал | Не предоставляется | `netsh` |
| Журналы | Системные журналы | Журнал событий Windows |
| Docker | Docker socket/CLI | Docker Desktop CLI |
| Слушающие порты | Зависит от платформы | `netstat` |
| Перезагрузка и выключение | Да | Да |
| Развёртывание службы | systemd | Служба Windows |

## Документация

- [Пример конфигурации узла](nodes.example.json)

## P2P-релейный протокол (hyper-relay)

Узлы работают без прослушивания портов с помощью агента `hyper-relay`: реле (установленное на той же машине, что и узел) — единственная резидентная служба и открывает один порт; агент пробуждается по требованию через локальный процесс (`hyper-node collect` / `hyper-node control`). Сквозные подписи Ed25519 обеспечивают доверие к командам даже при компрометации реле или веб-панели.

- **Установка**: `install.sh` / `install-windows.bat` устанавливают и `hyper-relay` (системная служба), и `hyper-node` (по требованию) на одной машине.
- **Сбор**: реле пробуждает `hyper-node collect` на той же машине по требованию для каждого опроса; агент не является резидентной службой.
- **Путь данных**: Android/веб-панель взаимодействуют с агентом только через реле (плоскость управления) — без прямых соединений; реле пробуждает локальный агент и возвращает свежий снимок.
- **Команды**: подписаны ключом устройства; действия с высоким риском (SSH / обновление системы / установка пакетов) требуют второго подтверждения администратора.
- **TLS (WSS)**: `hyper-relay serve --tls-cert <pem> --tls-key <pem>` обслуживает зашифрованные wss:// соединения — рекомендуется для публичных узлов (самоподписанные сертификаты принимаются клиентами); на локальных узлах задержка незаметна (аппаратное ускорение AES).
- **Управление сертификатами**: сертификаты **не привязаны к машине** — сгенерируйте один раз на любой машине и используйте на других узлах (`hyper-node cert import <cert.pem> <key.pem>`), либо генерируйте независимо на каждой машине (`hyper-node cert gen`). Общие сертификаты подходят для дома/локальной сети (проще управление); независимые рекомендуются для публичных/мультитенантных сред (аудит, изоляция, отзыв).
- **Модель учётных записей**: ключи доверенных устройств хранятся только на узле (`/etc/hyper-node/trusted.toml`); веб-панель не хранит ключи.

## Замечания по безопасности

Панель предназначена для **частной локальной сети** и не должна открываться в публичный интернет. Размещайте её в домашней сети / VPN и получайте доступ с доверенного устройства.

Агент по умолчанию использует TLS 1.3 с автоматически сгенерированным самоподписанным сертификатом. Панель закрепляет отпечаток сертификата при первом подключении (TOFU), каждый узел также требует собственный API-ключ. Взаимный TLS можно включить, добавив отпечаток клиентского сертификата панели в список доверия узла:

```bash
hyper-node trust add SHA256:<отпечаток-сертификата-панели>
```

Не используйте открытый режим в ненадёжной сети. Перед удалённым использованием смените пароль панели по умолчанию и защитите порты узлов и панели брандмауэром или частной сетью.

## Команды

### CLI веб-панели (`hyper-panel`)

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

### CLI агента (`hyper-node`)

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

## Лицензия

HyperScope распространяется под лицензией [MIT](LICENSE).
