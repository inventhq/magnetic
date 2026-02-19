#!/bin/bash
set -euo pipefail

# Magnetic Cloud — Node Setup Script
# Run on a fresh Ubuntu 22.04 Civo instance to set up:
#   1. magnetic-v8-server --platform (port 3003)
#   2. magnetic-control-plane (port 4000)
#   3. Caddy with Cloudflare DNS challenge (port 443/80)
#
# Usage: ssh ubuntu@<ip> 'bash -s' < deploy/setup-node.sh
# Or:    scp deploy/setup-node.sh ubuntu@<ip>: && ssh ubuntu@<ip> 'sudo bash setup-node.sh'

echo "=== Magnetic Cloud Node Setup ==="

# ── 1. System deps ──────────────────────────────────────────────────
apt-get update -qq
apt-get install -y -qq curl jq tar

# ── 2. Create directories ──────────────────────────────────────────
mkdir -p /var/lib/magnetic/apps
mkdir -p /etc/magnetic
mkdir -p /etc/caddy

# ── 3. Install Caddy with Cloudflare DNS plugin ────────────────────
echo "[setup] Installing Caddy with cloudflare DNS plugin..."
CADDY_URL="https://caddyserver.com/api/download?os=linux&arch=amd64&p=github.com/caddy-dns/cloudflare"
curl -fsSL "$CADDY_URL" -o /usr/local/bin/caddy
chmod +x /usr/local/bin/caddy
echo "[setup] Caddy installed: $(/usr/local/bin/caddy version)"

# ── 4. Download magnetic binaries ──────────────────────────────────
# TODO: Replace with actual GitHub Release URLs once binaries are published
ARCH=$(uname -m)
case "$ARCH" in
  x86_64)  TARGET="x86_64-unknown-linux-gnu" ;;
  aarch64) TARGET="aarch64-unknown-linux-gnu" ;;
  *)       echo "Unsupported arch: $ARCH"; exit 1 ;;
esac

RELEASE_BASE="https://github.com/nicholasgriffintn/magnetic/releases/latest/download"
echo "[setup] Downloading magnetic-v8-server ($TARGET)..."
curl -fsSL "$RELEASE_BASE/magnetic-v8-server-${TARGET}" -o /usr/local/bin/magnetic-v8-server || echo "[WARN] Binary not found — you'll need to upload it manually"
chmod +x /usr/local/bin/magnetic-v8-server 2>/dev/null || true

echo "[setup] Downloading magnetic-control-plane ($TARGET)..."
curl -fsSL "$RELEASE_BASE/magnetic-control-plane-${TARGET}" -o /usr/local/bin/magnetic-control-plane || echo "[WARN] Binary not found — you'll need to upload it manually"
chmod +x /usr/local/bin/magnetic-control-plane 2>/dev/null || true

# ── 5. Write Caddyfile ─────────────────────────────────────────────
cat > /etc/caddy/Caddyfile << 'CADDYEOF'
{
	email admin@fujs.dev
	admin localhost:2019
}

api.fujs.dev {
	reverse_proxy localhost:4000
}

fujs.dev {
	reverse_proxy localhost:4000
}

*.fujs.dev {
	tls {
		dns cloudflare {env.CF_API_TOKEN}
	}
	reverse_proxy localhost:3003
}
CADDYEOF

# ── 6. Write systemd services ──────────────────────────────────────

cat > /etc/systemd/system/magnetic-platform.service << 'EOF'
[Unit]
Description=Magnetic Platform Server (multi-tenant V8 hosting)
After=network.target

[Service]
Type=simple
ExecStart=/usr/local/bin/magnetic-v8-server --platform --port 3003 --data-dir /var/lib/magnetic/apps --park-idle 300
Restart=always
RestartSec=2
LimitNOFILE=65535
Environment=RUST_LOG=info
NoNewPrivileges=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/var/lib/magnetic

[Install]
WantedBy=multi-user.target
EOF

cat > /etc/systemd/system/magnetic-control-plane.service << 'EOF'
[Unit]
Description=Magnetic Cloud Control Plane
After=network.target

[Service]
Type=simple
ExecStart=/usr/local/bin/magnetic-control-plane
Restart=always
RestartSec=2
LimitNOFILE=65535
EnvironmentFile=/etc/magnetic/control-plane.env
NoNewPrivileges=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/var/lib/magnetic

[Install]
WantedBy=multi-user.target
EOF

cat > /etc/systemd/system/caddy.service << 'EOF'
[Unit]
Description=Caddy web server
After=network.target

[Service]
Type=simple
ExecStart=/usr/local/bin/caddy run --config /etc/caddy/Caddyfile
Restart=always
RestartSec=2
LimitNOFILE=65535
EnvironmentFile=/etc/magnetic/control-plane.env
AmbientCapabilities=CAP_NET_BIND_SERVICE

[Install]
WantedBy=multi-user.target
EOF

# ── 7. Write env file (placeholder — fill in real values) ──────────
if [ ! -f /etc/magnetic/control-plane.env ]; then
cat > /etc/magnetic/control-plane.env << 'EOF'
TURSO_URL=libsql://magnetic-control-vish.aws-us-west-2.turso.io
TURSO_TOKEN=SET_ME
CIVO_API_KEY=SET_ME
CF_API_TOKEN=SET_ME
MAGNETIC_DOMAIN=fujs.dev
PORT=4000
EOF
echo "[setup] Created /etc/magnetic/control-plane.env — fill in secrets!"
else
echo "[setup] /etc/magnetic/control-plane.env already exists, skipping"
fi

# ── 8. Enable and start services ───────────────────────────────────
systemctl daemon-reload
systemctl enable magnetic-platform magnetic-control-plane caddy

echo ""
echo "=== Setup complete ==="
echo ""
echo "Next steps:"
echo "  1. Edit /etc/magnetic/control-plane.env with real secrets"
echo "  2. Upload binaries if GitHub Release download failed:"
echo "     scp target/release/magnetic-v8-server ubuntu@<ip>:/usr/local/bin/"
echo "     scp target/release/magnetic-control-plane ubuntu@<ip>:/usr/local/bin/"
echo "  3. Start services:"
echo "     systemctl start magnetic-platform"
echo "     systemctl start magnetic-control-plane"
echo "     systemctl start caddy"
echo "  4. Verify: curl https://api.fujs.dev/health"
echo ""
