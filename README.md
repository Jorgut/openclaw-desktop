[English](README.md) | [中文](README.zh-CN.md)

# OpenClaw Desktop

A lightweight Tauri v2 desktop wrapper for [OpenClaw](https://docs.openclaw.ai) gateway. One click to launch — no manual SSH tunnels, no browser tabs.

## What it does

- **Auto-starts** `openclaw gateway run` as a child process
- **Waits** for gateway to be ready, then loads the Web UI in a native window
- **System tray** — close window hides to tray; right-click to Show/Hide/Quit
- **Proxy-aware** — reads system proxy settings (GNOME gsettings) so Telegram and other channels work
- **Clean lifecycle** — Quit kills the gateway child; orphan gateways are reused on next launch

## Prerequisites

- **Linux** (Ubuntu 22.04+ tested)
- **OpenClaw CLI** installed and configured (`~/.openclaw/openclaw.json` with `gateway` section)
- System dependencies for Tauri v2:
  ```bash
  sudo apt-get install -y \
    libwebkit2gtk-4.1-dev libgtk-3-dev librsvg2-dev \
    libayatana-appindicator3-dev libssl-dev pkg-config
  ```

## Install from .deb (easiest)

Download the `.deb` from [Releases](../../releases), then:

```bash
sudo dpkg -i "OpenClaw Desktop_0.1.0_amd64.deb"
```

Launch from the application menu (search "OpenClaw") or:

```bash
nohup openclaw-desktop > /dev/null 2>&1 &
```

## Build from source

### 1. Install Rust

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source "$HOME/.cargo/env"
```

### 2. Install Node.js dependencies

```bash
cd openclaw-desktop
npm install
```

### 3. Build

```bash
npx tauri build --bundles deb
```

The `.deb` will be at `src-tauri/target/release/bundle/deb/`.

For dev mode (faster iteration):

```bash
npx tauri dev
```

## Usage

### Launch

```bash
# From app menu: search "OpenClaw"
# Or from terminal (background):
nohup openclaw-desktop > /dev/null 2>&1 &
```

### Quit

Right-click the system tray icon → **Quit**

> Clicking the window close button only hides to tray. This is intentional — the gateway keeps running.

### Proxy / VPN

If you use a proxy to access Telegram API (common in some regions), configure it via:
- **GNOME Settings** → Network → Proxy (auto-detected by the app)
- Or set `HTTP_PROXY`/`HTTPS_PROXY` env vars before launching

## How it works

```
openclaw-desktop (Tauri)
  ├── Reads ~/.openclaw/openclaw.json → gateway port + auth token
  ├── Spawns `openclaw gateway run` as child process (with proxy env)
  ├── Polls health endpoint until ready
  ├── Loads local shell UI → redirects to http://127.0.0.1:{port}/#token={token}
  ├── System tray with Show/Hide/Status/Quit
  └── On Quit → kills gateway child process
```

## Project structure

```
openclaw-desktop/
├── package.json
├── ui/                          # Local shell (loading + error pages)
│   ├── index.html
│   ├── style.css
│   └── app.js
└── src-tauri/
    ├── Cargo.toml
    ├── tauri.conf.json
    ├── capabilities/default.json
    ├── icons/
    └── src/
        ├── main.rs              # Entry point
        ├── lib.rs               # App builder + setup + event loop
        ├── config.rs            # Reads ~/.openclaw/openclaw.json
        ├── gateway.rs           # Health check + auto-start + proxy
        ├── tray.rs              # System tray menu + background monitor
        └── commands.rs          # Tauri IPC commands
```

## Configuration

The app reads `~/.openclaw/openclaw.json`. The relevant section:

```json
{
  "gateway": {
    "port": 18789,
    "bind": "loopback",
    "auth": {
      "mode": "token",
      "token": "your-token-here"
    }
  }
}
```

This file is created by `openclaw` CLI during setup. No manual editing needed.

## Troubleshooting

| Problem | Solution |
|---------|----------|
| "Gateway Offline" on start | Check if `openclaw` CLI is installed: `which openclaw` |
| Telegram not responding | Ensure system proxy is configured (GNOME Settings → Network → Proxy) |
| Multiple instances | `pkill -f openclaw-desktop` then relaunch |
| Window disappeared | Click the system tray icon or right-click → Show |

## License

MIT
