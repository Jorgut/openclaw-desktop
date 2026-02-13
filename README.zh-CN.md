# OpenClaw Desktop

轻量级 Tauri v2 桌面客户端，包装 [OpenClaw](https://docs.openclaw.ai) Gateway。一键启动，无需手动 SSH 端口转发、无需浏览器标签页。

## 功能

- **自动启动** Gateway — 以子进程方式运行 `openclaw gateway run`
- **等待就绪** — Gateway 启动后自动轮询，就绪后直接加载 Web UI
- **系统托盘** — 关闭窗口仅隐藏到托盘，右键菜单可 Show / Hide / Quit
- **代理感知** — 自动读取系统代理设置（GNOME gsettings），Telegram 等频道正常工作
- **干净的生命周期** — Quit 时自动关闭 Gateway 子进程；下次启动若发现残留 Gateway 会自动复用

## 前置条件

- **Linux**（已测试 Ubuntu 22.04+）
- **OpenClaw CLI** 已安装并配置好（`~/.openclaw/openclaw.json` 中有 `gateway` 配置段）
- Tauri v2 系统依赖：
  ```bash
  sudo apt-get install -y \
    libwebkit2gtk-4.1-dev libgtk-3-dev librsvg2-dev \
    libayatana-appindicator3-dev libssl-dev pkg-config
  ```

## 安装（最简单）

从 [Releases](../../releases) 下载 `.deb` 安装包，然后：

```bash
sudo dpkg -i "OpenClaw Desktop_0.1.0_amd64.deb"
```

从应用菜单启动（搜索 "OpenClaw"），或者：

```bash
nohup openclaw-desktop > /dev/null 2>&1 &
```

## 从源码构建

### 1. 安装 Rust

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source "$HOME/.cargo/env"
```

### 2. 安装 Node.js 依赖

```bash
cd openclaw-desktop
npm install
```

### 3. 构建

```bash
npx tauri build --bundles deb
```

`.deb` 包会生成在 `src-tauri/target/release/bundle/deb/` 目录下。

开发模式（快速迭代）：

```bash
npx tauri dev
```

## 使用方法

### 启动

```bash
# 从应用菜单：搜索 "OpenClaw"
# 或从终端后台启动：
nohup openclaw-desktop > /dev/null 2>&1 &
```

### 退出

右键系统托盘图标 → **Quit**

> 点击窗口关闭按钮只会隐藏到托盘，不会退出。这是设计行为 — Gateway 会保持运行。

### 代理 / VPN

如果你需要代理才能访问 Telegram API（某些地区常见），请通过以下方式配置：
- **GNOME 设置** → 网络 → 代理（应用会自动检测）
- 或在启动前设置 `HTTP_PROXY` / `HTTPS_PROXY` 环境变量

## 工作原理

```
openclaw-desktop (Tauri)
  ├── 读取 ~/.openclaw/openclaw.json → Gateway 端口 + 认证 Token
  ├── 以子进程方式启动 `openclaw gateway run`（携带代理环境变量）
  ├── 轮询健康检查接口直到就绪
  ├── 加载本地 Shell UI → 自动跳转到 http://127.0.0.1:{port}/#token={token}
  ├── 系统托盘：Show / Hide / 状态显示 / Quit
  └── 退出时终止 Gateway 子进程
```

## 项目结构

```
openclaw-desktop/
├── package.json
├── ui/                          # 本地 Shell（加载页 + 错误页）
│   ├── index.html
│   ├── style.css
│   └── app.js
└── src-tauri/
    ├── Cargo.toml
    ├── tauri.conf.json
    ├── capabilities/default.json
    ├── icons/
    └── src/
        ├── main.rs              # 入口
        ├── lib.rs               # App 构建器 + setup + 事件循环
        ├── config.rs            # 读取 ~/.openclaw/openclaw.json
        ├── gateway.rs           # 健康检查 + 自动启动 + 代理注入
        ├── tray.rs              # 系统托盘菜单 + 后台状态监控
        └── commands.rs          # Tauri IPC 命令
```

## 配置

应用读取 `~/.openclaw/openclaw.json`，相关配置段：

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

此文件由 `openclaw` CLI 在初始化时自动生成，无需手动编辑。

## 常见问题

| 问题 | 解决方法 |
|------|----------|
| 启动时显示 "Gateway Offline" | 检查 OpenClaw CLI 是否已安装：`which openclaw` |
| Telegram 没有回复 | 确认系统代理已配置（GNOME 设置 → 网络 → 代理） |
| 出现多个实例 | `pkill -f openclaw-desktop` 然后重新启动 |
| 窗口消失了 | 点击系统托盘图标，或右键 → Show |

## 许可证

MIT
