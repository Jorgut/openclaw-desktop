[English](README.md) | [中文](README.zh-CN.md)

# OpenClaw Desktop

A lightweight Tauri v2 desktop wrapper for [OpenClaw](https://docs.openclaw.ai) gateway. Install the `.deb`, open the app, and follow the guided setup — no terminal needed.

## Features

- **First-run setup wizard** — guides new users through Node.js check, OpenClaw CLI installation, model provider configuration, channel setup, and proxy detection
- **Auto-starts** `openclaw gateway run` as a child process
- **Waits** for gateway to be ready, then loads the Web UI in a native window
- **System tray** — close window hides to tray; right-click to Show/Hide/Quit
- **Proxy-aware** — reads system proxy settings (env vars / GNOME gsettings) so Telegram and other channels work behind a proxy
- **Clean lifecycle** — Quit kills the gateway child; orphan gateways are cleaned up on next launch

## Quick Start (for end users)

### 1. Install

Download the `.deb` from [Releases](../../releases), then:

```bash
sudo dpkg -i "OpenClaw Desktop_0.1.0_amd64.deb"
```

### 2. Launch

Search "OpenClaw" in the application menu, or:

```bash
nohup openclaw-desktop > /dev/null 2>&1 &
```

### 3. Follow the Setup Wizard

On first launch, the app will guide you through:

1. **Environment check** — detects Node.js and OpenClaw CLI. If OpenClaw CLI is missing, click "Install" to install it automatically via npm.
2. **Model configuration** — choose a provider (MiniMax recommended, free tier available) and enter your API Key.
3. **Channel configuration** (optional) — add Telegram Bot Token and/or Discord Bot Token.
4. **Proxy detection** — auto-detects system proxy. If you're in mainland China, you may need to configure a proxy for Telegram/Discord to work.
5. **Confirm & launch** — review your settings, save, and start.

After the wizard, the gateway starts automatically and the Web UI loads. On subsequent launches, the wizard is skipped.

> **Note:** Node.js (v18+) is required but the app does NOT install it for you (it needs sudo). If Node.js is missing, the wizard will show you the install command.

## Build from Source

### Prerequisites

- **Linux** (Ubuntu 22.04+ tested)
- **Rust** toolchain
- System dependencies for Tauri v2:
  ```bash
  sudo apt-get install -y \
    libwebkit2gtk-4.1-dev libgtk-3-dev librsvg2-dev \
    libayatana-appindicator3-dev libssl-dev pkg-config
  ```

### Steps

```bash
# 1. Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source "$HOME/.cargo/env"

# 2. Install Node.js dependencies
cd openclaw-desktop
npm install

# 3. Build .deb
npx tauri build --bundles deb
```

The `.deb` will be at `src-tauri/target/release/bundle/deb/`.

For dev mode (faster iteration):

```bash
npx tauri dev
```

## Usage

### Quit

Right-click the system tray icon → **Quit**

> Clicking the window close button only hides to tray. This is intentional — the gateway keeps running.

### Proxy / VPN

If you need a proxy to access Telegram API (common in mainland China), configure it via:
- **GNOME Settings** → Network → Proxy (auto-detected by the app)
- Or set `HTTP_PROXY`/`HTTPS_PROXY` env vars before launching

## How it Works

```
openclaw-desktop (Tauri)
  ├── First launch?
  │   ├── Yes → Show setup wizard (setup.html)
  │   │         ├── Check Node.js / npm / openclaw CLI
  │   │         ├── Install openclaw CLI if missing
  │   │         ├── Configure provider + API key
  │   │         ├── Configure channels (optional)
  │   │         ├── Detect / configure proxy
  │   │         └── Save config → start gateway → load Web UI
  │   └── No → Normal startup
  │             ├── Read ~/.openclaw/openclaw.json → port + token
  │             ├── Spawn `openclaw gateway run` (with proxy env)
  │             ├── Poll /health until ready
  │             └── Load http://127.0.0.1:{port}/#token={token}
  ├── System tray: Show / Hide / Status / Quit
  └── On Quit → kill gateway child process
```

## Project Structure

```
openclaw-desktop/
├── package.json
├── ui/                          # Frontend (loading, error, setup wizard)
│   ├── index.html               # Main loading/error page
│   ├── setup.html               # First-run setup wizard
│   ├── setup.js                 # Setup wizard logic
│   ├── app.js                   # Main page logic
│   └── style.css                # Shared styles (dark theme)
└── src-tauri/
    ├── Cargo.toml
    ├── tauri.conf.json
    ├── capabilities/default.json
    ├── icons/
    └── src/
        ├── main.rs              # Entry point
        ├── lib.rs               # App builder + first-run check + event loop
        ├── config.rs            # Reads ~/.openclaw/openclaw.json
        ├── gateway.rs           # Health check + auto-start + proxy injection
        ├── setup.rs             # Setup wizard backend (prereq check, install, config)
        ├── tray.rs              # System tray menu + background health monitor
        └── commands.rs          # Tauri IPC commands
```

## Configuration

The app reads (and on first run, creates) `~/.openclaw/openclaw.json`:

```json
{
  "gateway": {
    "port": 18789,
    "bind": "loopback",
    "auth": {
      "mode": "token",
      "token": "auto-generated"
    }
  },
  "providers": {
    "minimax": {
      "apiKey": "your-api-key"
    }
  },
  "defaultProvider": "minimax",
  "defaultModel": "MiniMax-M1"
}
```

On first run this file is generated automatically by the setup wizard. No manual editing needed.

## Troubleshooting

| Problem | Solution |
|---------|----------|
| Setup wizard says "Node.js not found" | Install Node.js v18+: `curl -fsSL https://deb.nodesource.com/setup_22.x \| sudo -E bash - && sudo apt-get install -y nodejs` |
| "Install OpenClaw" button fails | Check your network connection; if behind a proxy, configure system proxy first |
| "Gateway Offline" after setup | Check logs: `cat ~/.openclaw/desktop-gateway.log` |
| Telegram not responding | Ensure system proxy is configured (GNOME Settings → Network → Proxy) |
| Multiple instances | `pkill -f openclaw-desktop` then relaunch |
| Window disappeared | Click the system tray icon or right-click → Show |
| Want to re-run setup wizard | Delete `~/.openclaw/openclaw.json` and relaunch |

## License

MIT
