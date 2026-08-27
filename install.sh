#!/bin/bash
# System monitoring installer
# Usage:
#   bash install.sh node    # install hyper-node (monitored machine)
#   bash install.sh panel   # install hyper-panel (panel server, runs on host)
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

if [ "$ROLE" = "node" ]; then
        echo "==> Installing hyper-node ($ARCH)"
    curl -fL -o /usr/local/bin/hyper-node "https://github.com/saves24/HyperScope/releases/latest/download/hyper-node-linux-${BIN_SUFFIX}"
    chmod +x /usr/local/bin/hyper-node

        echo "==> Creating config directory"
    mkdir -p /etc/hyper-node
    mkdir -p /var/log/hyper-node

        echo "==> Installing systemd service"
    cp "$(dirname "$0")/deploy/hyper-node.service" /etc/systemd/system/hyper-node.service

    systemctl daemon-reload
    systemctl enable hyper-node

    echo ""
        echo "==> hyper-node installed!"
        echo "Next steps:"
        echo "  1. Set key:        hyper-node key setup        (or specify: hyper-node key setup <your-key>)"
        echo "  2. Start service:  systemctl start hyper-node"
        echo "  3. Show key:       hyper-node key show"
        echo "  4. Add node in panel: address:5000 + key"

elif [ "$ROLE" = "panel" ]; then
        echo "==> Installing hyper-panel ($ARCH)"
    curl -fL -o /usr/local/bin/hyper-panel "https://github.com/saves24/HyperScope/releases/latest/download/hyper-panel-linux-${BIN_SUFFIX}"
    chmod +x /usr/local/bin/hyper-panel

        echo "==> Creating config directory"
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

        echo "==> Installing systemd service"
    cp "$(dirname "$0")/deploy/hyper-panel.service" /etc/systemd/system/hyper-panel.service

    systemctl daemon-reload
    systemctl enable hyper-panel

    echo ""
        echo "==> hyper-panel installed!"
        echo "Next steps:"
        echo "  1. Start service:  systemctl start hyper-panel"
        echo "  2. Add node:       hyper-panel add node <name> <address> <port> <key>"
        echo "  3. View nodes:     hyper-panel nodes"
        echo "  4. View logs:      hyper-panel log show | log system | log retention"
        echo "  5. Open panel:     http://<this-host-ip>:8088"

else
        echo "Error: unknown role $ROLE (choose node / panel)"
    exit 1
fi
