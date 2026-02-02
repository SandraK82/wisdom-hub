# Deployment Guide

## Overview

This guide covers deploying all three components of the Wisdom Network.

## Architecture Options

### Option 1: Single Machine (Development)

All components on one machine:

```
┌─────────────────────────────────────────────────┐
│                  localhost                       │
│  ┌───────────┐  ┌──────────────┐  ┌──────────┐ │
│  │wisdom-mcp │─▶│wisdom-gateway│─▶│wisdom-hub│ │
│  │  :stdio   │  │    :8080     │  │   :8081  │ │
│  └───────────┘  └──────────────┘  └──────────┘ │
└─────────────────────────────────────────────────┘
```

### Option 2: Local + Remote Hub (Recommended for Getting Started)

Gateway local, connecting to the public primary hub:

```
┌────────────────────────┐     ┌──────────────────────────────┐
│      Your Machine      │     │       Public Hub              │
│  ┌───────────┐         │     │  ┌────────────────────────┐  │
│  │wisdom-mcp │         │     │  │  hub1.wisdom.spawning.de│  │
│  └─────┬─────┘         │     │  │  (primary hub)         │  │
│        │               │     │  └────────────────────────┘  │
│  ┌─────▼─────┐         │     │              ▲               │
│  │  gateway  │─────────┼─────┼──────────────┘               │
│  └───────────┘  HTTPS  │     │                              │
└────────────────────────┘     └──────────────────────────────┘
```

**Public Primary Hub**: `https://hub1.wisdom.spawning.de`

This is the easiest way to get started - just point your gateway at the public hub.

### Option 3: Full Federation

Multiple hubs forming a network:

```
┌──────────────────┐     ┌──────────────────┐
│    Primary Hub   │◀───▶│  Secondary Hub   │
│   hub1.example   │     │   hub2.example   │
└────────┬─────────┘     └────────┬─────────┘
         │                        │
         └────────────────────────┘
                   ▲
         ┌─────────┴─────────┐
    ┌────┴────┐         ┌────┴────┐
    │Gateway A│         │Gateway B│
    └─────────┘         └─────────┘
```

## wisdom-hub Deployment

### Prerequisites

- Linux server (Ubuntu 22.04+ recommended)
- 2+ GB RAM
- 20+ GB disk (SSD recommended)
- Rust 1.75+ (for building)

### Building

```bash
# Clone and build
git clone https://github.com/SandraK82/wisdom-hub.git
cd wisdom-hub
cargo build --release

# Binary at ./target/release/wisdom-hub
```

### Configuration

Create `/etc/wisdom-hub/config.toml`:

```toml
[hub]
role = "primary"  # or "secondary"
hub_id = "your-unique-hub-id"
public_url = "https://hub.yourdomain.com"

[server]
host = "0.0.0.0"
http_port = 8080
grpc_port = 50051
workers = 4  # adjust to CPU cores

[database]
data_dir = "/var/lib/wisdom-hub/data"
compression = true
cache_size_mb = 512  # adjust to available RAM

[discovery]
enabled = true
# For secondary hubs:
# primary_hub_url = "https://primary-hub.example.com"

[trust]
max_depth = 5
damping_factor = 0.8
min_trust_threshold = 0.01

[resources]
warning_threshold = 60
critical_threshold = 80
check_interval_sec = 60

[metrics]
enabled = true
path = "/metrics"
```

### Systemd Service

Create `/etc/systemd/system/wisdom-hub.service`:

```ini
[Unit]
Description=Wisdom Hub Federation Server
After=network.target

[Service]
Type=simple
User=wisdom
Group=wisdom
ExecStart=/usr/local/bin/wisdom-hub
WorkingDirectory=/var/lib/wisdom-hub
Environment=RUST_LOG=info,wisdom_hub=debug
Restart=always
RestartSec=5

# Security hardening
NoNewPrivileges=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/var/lib/wisdom-hub

[Install]
WantedBy=multi-user.target
```

```bash
# Install and start
sudo cp target/release/wisdom-hub /usr/local/bin/
sudo useradd -r -s /bin/false wisdom
sudo mkdir -p /var/lib/wisdom-hub/data
sudo chown -R wisdom:wisdom /var/lib/wisdom-hub
sudo systemctl daemon-reload
sudo systemctl enable wisdom-hub
sudo systemctl start wisdom-hub
```

### Reverse Proxy (nginx)

```nginx
server {
    listen 443 ssl http2;
    server_name hub.yourdomain.com;

    ssl_certificate /etc/letsencrypt/live/hub.yourdomain.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/hub.yourdomain.com/privkey.pem;

    # REST API
    location / {
        proxy_pass http://127.0.0.1:8080;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }

    # gRPC (if needed)
    location /wisdom.hub.v1 {
        grpc_pass grpc://127.0.0.1:50051;
    }
}
```

### Health Checks

```bash
# HTTP health
curl https://hub.yourdomain.com/health

# Metrics
curl https://hub.yourdomain.com/metrics
```

## wisdom-gateway Deployment

### Building

```bash
git clone https://github.com/SandraK82/wisdom-gateway.git
cd wisdom-gateway
go build -o bin/gateway ./cmd/gateway
```

### Running

```bash
# As user service
./bin/gateway -addr :8080 -db ~/.wisdom/gateway.db -hub https://hub.example.com
```

### macOS LaunchAgent

Create `~/Library/LaunchAgents/com.wisdom.gateway.plist`:

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.wisdom.gateway</string>
    <key>ProgramArguments</key>
    <array>
        <string>/usr/local/bin/wisdom-gateway</string>
        <string>-addr</string>
        <string>:8080</string>
        <string>-db</string>
        <string>/Users/YOU/.wisdom/gateway.db</string>
        <string>-hub</string>
        <string>https://hub.example.com</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <true/>
</dict>
</plist>
```

```bash
launchctl load ~/Library/LaunchAgents/com.wisdom.gateway.plist
```

### Linux Systemd (User)

Create `~/.config/systemd/user/wisdom-gateway.service`:

```ini
[Unit]
Description=Wisdom Gateway
After=network.target

[Service]
ExecStart=/usr/local/bin/wisdom-gateway -addr :8080 -db %h/.wisdom/gateway.db -hub https://hub.example.com
Restart=always

[Install]
WantedBy=default.target
```

```bash
systemctl --user daemon-reload
systemctl --user enable wisdom-gateway
systemctl --user start wisdom-gateway
```

## wisdom-mcp Setup

### Installation

```bash
git clone https://github.com/SandraK82/wisdom-mcp.git
cd wisdom-mcp
npm install
npm run build
```

### Claude Desktop Configuration

Edit `~/Library/Application Support/Claude/claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "wisdom": {
      "command": "node",
      "args": ["/path/to/wisdom-mcp/dist/index.js"],
      "env": {
        "WISDOM_GATEWAY_URL": "http://localhost:8080"
      }
    }
  }
}
```

### First-Time Setup

1. Start Claude Desktop
2. The wisdom tools should appear
3. First use will prompt for agent setup (generates keys automatically)

## Monitoring

### Prometheus Scrape Config

```yaml
scrape_configs:
  - job_name: 'wisdom-hub'
    static_configs:
      - targets: ['hub.example.com:8080']
    metrics_path: '/metrics'
    scheme: 'https'
```

### Key Metrics

| Metric | Description |
|--------|-------------|
| `wisdom_hub_entities_total` | Total entities by type |
| `wisdom_hub_requests_total` | API requests by endpoint |
| `wisdom_hub_disk_usage_percent` | Current disk usage |
| `wisdom_hub_resource_level` | 0=normal, 1=warning, 2=critical |

### Alerting Rules

```yaml
groups:
  - name: wisdom-hub
    rules:
      - alert: WisdomHubCritical
        expr: wisdom_hub_resource_level == 2
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "Wisdom Hub at critical capacity"

      - alert: WisdomHubDown
        expr: up{job="wisdom-hub"} == 0
        for: 1m
        labels:
          severity: critical
        annotations:
          summary: "Wisdom Hub is down"
```

## Backup & Recovery

### Hub Data Backup

```bash
# Stop hub (optional, RocksDB supports hot backup)
systemctl stop wisdom-hub

# Backup data directory
tar -czf wisdom-hub-backup-$(date +%Y%m%d).tar.gz /var/lib/wisdom-hub/data

# Restart
systemctl start wisdom-hub
```

### Gateway Backup

```bash
# SQLite backup
sqlite3 ~/.wisdom/gateway.db ".backup ~/.wisdom/gateway-backup.db"
```

## Troubleshooting

### Hub won't start

```bash
# Check logs
journalctl -u wisdom-hub -f

# Common issues:
# - Port already in use: check with `lsof -i :8080`
# - Permission denied: check data directory ownership
# - Config error: validate TOML syntax
```

### Gateway can't connect to hub

```bash
# Test connectivity
curl https://hub.example.com/health

# Check DNS resolution
dig hub.example.com

# Check TLS
openssl s_client -connect hub.example.com:443
```

### MCP tools not appearing

1. Check Claude Desktop logs
2. Verify path in config is correct
3. Try running manually: `node /path/to/wisdom-mcp/dist/index.js`
