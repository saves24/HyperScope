#!/bin/bash
# HyperScope installer
# Usage:
#   bash install.sh node    # install hyper-node + hyper-relay (monitored machine)
#   bash install.sh panel   # install hyper-panel (panel server)
# or default node
set -e

ROLE="${1:-node}"
ARCH=$(uname -m)
case "$ARCH" in
    x86_64)  BIN_SUFFIX="amd64" ;;
    aarch64|arm64) BIN_SUFFIX="arm64" ;;
    *)
        echo "Error: unsupported architecture $ARCH (only x86_64 / aarch64)"
        exit 1
        ;;
esac

# ---- helpers -------------------------------------------------------------
download() {
    # $1 = url, $2 = dest. Retries once; fails loudly instead of silently.
    echo "==> Downloading $2"
    if ! curl -fL --connect-timeout 15 --max-time 300 -o "$2" "$1"; then
        echo "Error: failed to download $1 (check network / proxy)." >&2
        exit 1
    fi
    chmod +x "$2"
}

service_exists() {
    systemctl cat "$1" >/dev/null 2>&1
}

# ---- node role ------------------------------------------------------------
if [ "$ROLE" = "node" ]; then
    echo "==> HyperScope node installer (arch=$ARCH)"

    # Pre-flight: stop the relay service so binaries can be replaced.
    if service_exists hyper-relay; then
        echo "==> Stopping existing hyper-relay service..."
        systemctl stop hyper-relay 2>/dev/null || true
    fi

    # Only download when the binary is missing or differs from the release.
    LATEST_NODE_URL="https://github.com/saves24/HyperScope/releases/latest/download/hyper-node-linux-${BIN_SUFFIX}"
    LATEST_RELAY_URL="https://github.com/saves24/HyperScope/releases/latest/download/hyper-relay-linux-${BIN_SUFFIX}"

    if [ -x /usr/local/bin/hyper-node ]; then
        echo "==> hyper-node already installed ($(hyper-node --version 2>/dev/null | head -1 || echo unknown)); replacing with latest"
    fi
    download "$LATEST_NODE_URL" /usr/local/bin/hyper-node

    if [ -x /usr/local/bin/hyper-relay ]; then
        echo "==> hyper-relay already installed; replacing with latest"
    fi
    download "$LATEST_RELAY_URL" /usr/local/bin/hyper-relay

    echo "==> Creating config directories"
    mkdir -p /etc/hyper-node
    mkdir -p /var/log/hyper-node

    # systemd service (only hyper-relay is a persistent service).
    # When installed via `curl | bash` the repo deploy/ dir is absent, so
    # fetch the unit from GitHub; when run from a checkout use the local file.
    RELAY_UNIT="/etc/systemd/system/hyper-relay.service"
    if [ -f "$(dirname "$0")/deploy/hyper-relay.service" ]; then
        echo "==> Installing hyper-relay systemd service (from checkout)"
        cp "$(dirname "$0")/deploy/hyper-relay.service" "$RELAY_UNIT"
    else
        echo "==> Installing hyper-relay systemd service (from GitHub)"
        curl -fsSL --connect-timeout 15 --max-time 60 \
            "https://raw.githubusercontent.com/saves24/HyperScope/main/deploy/hyper-relay.service" \
            -o "$RELAY_UNIT"
    fi

    systemctl daemon-reload
    systemctl enable hyper-relay 2>/dev/null || true

    echo ""
    echo "==> hyper-node + hyper-relay installed!"
    echo "Next steps:"
    echo "  1. Set key:        sudo hyper-node key setup        (or: sudo hyper-node key setup <your-key>)"
    echo "  2. Start service:  sudo systemctl start hyper-relay"
    echo "  3. Show key:       sudo hyper-node key show"
    echo "  4. Add node in panel: node address + key (all nodes run in relay mode)"
    echo "  5. Trust devices (REQUIRED for remote commands): relay commands are"
    echo "     signed; add each controlling device (panel/phone) to the trust list:"
    echo "       sudo hyper-node device add <device-id> <device-pubkey> admin"
    echo "     (get the device pubkey from the panel/phone; see hyper-node device list)"
    echo ""
    echo "Certificate options (relay serves WSS/TLS):"
    echo "  1) Shared cert:    certificates are machine-agnostic — generate once on any"
    echo "                     machine, then copy it to others:"
    echo "                     sudo hyper-node cert import <cert.pem> <key.pem>"
    echo "  2) Independent:    each machine generates its own:"
    echo "                     sudo hyper-node cert gen"
    echo "  Check:             sudo hyper-node cert show"
    exit 0
fi

# ---- panel role ------------------------------------------------------------
if [ "$ROLE" = "panel" ]; then
    echo "==> HyperScope panel installer (arch=$ARCH)"

    if [ -x /usr/local/bin/hyper-panel ]; then
        echo "==> hyper-panel already installed; replacing with latest"
    fi
    download "https://github.com/saves24/HyperScope/releases/latest/download/hyper-panel-linux-${BIN_SUFFIX}" /usr/local/bin/hyper-panel

    echo "==> Creating config directories"
    mkdir -p /etc/hyper-panel
    mkdir -p /var/log/hyper-panel
    if [ ! -f /etc/hyper-panel/nodes.json ]; then
        echo "[]" > /etc/hyper-panel/nodes.json
    fi

    echo "==> Creating dedicated system user (non-root runtime)"
    if ! id hyperscope >/dev/null 2>&1; then
        useradd -r -s /usr/sbin/nologin -M hyperscope
    fi
    chown -R hyperscope:hyperscope /etc/hyper-panel /var/log/hyper-panel
    chmod 750 /etc/hyper-panel /var/log/hyper-panel

    if [ -f "$(dirname "$0")/deploy/hyper-panel.service" ]; then
        echo "==> Installing systemd service (from checkout)"
        cp "$(dirname "$0")/deploy/hyper-panel.service" /etc/systemd/system/hyper-panel.service
    else
        echo "==> Installing systemd service (from GitHub)"
        curl -fsSL --connect-timeout 15 --max-time 60 \
            "https://raw.githubusercontent.com/saves24/HyperScope/main/deploy/hyper-panel.service" \
            -o /etc/systemd/system/hyper-panel.service
    fi

    systemctl daemon-reload
    systemctl enable hyper-panel 2>/dev/null || true

    echo ""
    echo "==> hyper-panel installed!"
    echo "Next steps:"
    echo "  1. Start service:  sudo systemctl start hyper-panel"
    echo "  2. Add node:       hyper-panel node add <name> <address> <key>"
    echo "  3. View nodes:     hyper-panel node list"
    echo "  4. Open panel:     http://<this-host-ip>:8088"
    exit 0
fi

echo "Error: unknown role $ROLE (choose node / panel)"
exit 1
