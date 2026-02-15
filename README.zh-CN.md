# OpenClaw Desktop

轻量级 Tauri v2 桌面客户端，包装 [OpenClaw](https://docs.openclaw.ai) Gateway。安装 `.deb`，打开应用，跟着引导走完设置，无需碰终端。

## 功能

- **首次启动引导向导** — 引导新用户完成 Node.js 检测、OpenClaw CLI 安装、模型供应商配置、频道设置和代理检测
- **自动启动** Gateway — 以子进程方式运行 `openclaw gateway run`
- **等待就绪** — Gateway 启动后自动轮询，就绪后直接加载 Web UI
- **系统托盘** — 关闭窗口仅隐藏到托盘，右键菜单可 Show / Hide / Quit
- **代理感知** — 自动读取系统代理设置（环境变量 / GNOME gsettings），Telegram 等频道可在代理后正常工作
- **干净的生命周期** — Quit 时自动关闭 Gateway 子进程；下次启动若发现残留 Gateway 会自动清理

## 快速上手（普通用户）

### 1. 安装

从 [Releases](../../releases) 下载 `.deb` 安装包，然后：

```bash
sudo dpkg -i "OpenClaw Desktop_0.1.0_amd64.deb"
```

### 2. 启动

从应用菜单启动（搜索 "OpenClaw"），或者：

```bash
nohup openclaw-desktop > /dev/null 2>&1 &
```

### 3. 跟着引导向导走

首次启动时，应用会引导你完成以下步骤：

1. **环境检测** — 检测 Node.js 和 OpenClaw CLI。如果缺少 OpenClaw CLI，点击"一键安装"按钮即可通过 npm 自动安装。
2. **模型配置** — 选择模型供应商（推荐 MiniMax，有免费额度）并输入 API Key。
3. **频道配置**（可选） — 添加 Telegram Bot Token 和/或 Discord Bot Token。
4. **代理检测** — 自动检测系统代理。如果你在中国大陆，可能需要配置代理才能让 Telegram/Discord 正常工作。
5. **确认并启动** — 检查配置摘要，保存，开始使用。

引导完成后，Gateway 自动启动并加载 Web UI。之后再次启动时会跳过引导，直接进入主界面。

> **注意：** 需要 Node.js（v18+），但应用不会自动安装它（需要 sudo 权限）。如果检测到 Node.js 缺失，引导页会显示安装命令。

## 从源码构建

### 前置条件

- **Linux**（已测试 Ubuntu 22.04+）
- **Rust** 工具链
- Tauri v2 系统依赖：
  ```bash
  sudo apt-get install -y \
    libwebkit2gtk-4.1-dev libgtk-3-dev librsvg2-dev \
    libayatana-appindicator3-dev libssl-dev pkg-config
  ```

### 构建步骤

```bash
# 1. 安装 Rust（如果还没有）
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source "$HOME/.cargo/env"

# 2. 安装 Node.js 依赖
cd openclaw-desktop
npm install

# 3. 构建 .deb
npx tauri build --bundles deb
```

`.deb` 包会生成在 `src-tauri/target/release/bundle/deb/` 目录下。

开发模式（快速迭代）：

```bash
npx tauri dev
```

## 使用方法

### 退出

右键系统托盘图标 → **Quit**

> 点击窗口关闭按钮只会隐藏到托盘，不会退出。这是设计行为 — Gateway 会保持运行。

### 代理 / VPN

如果你需要代理才能访问 Telegram API（中国大陆常见），请通过以下方式配置：
- **GNOME 设置** → 网络 → 代理（应用会自动检测）
- 或在启动前设置 `HTTP_PROXY` / `HTTPS_PROXY` 环境变量

## 工作原理

```
openclaw-desktop (Tauri)
  ├── 首次启动？
  │   ├── 是 → 显示引导向导（setup.html）
  │   │         ├── 检测 Node.js / npm / openclaw CLI
  │   │         ├── 缺少 openclaw CLI 则安装
  │   │         ├── 配置模型供应商 + API Key
  │   │         ├── 配置频道（可选）
  │   │         ├── 检测 / 配置代理
  │   │         └── 保存配置 → 启动 Gateway → 加载 Web UI
  │   └── 否 → 正常启动
  │             ├── 读取 ~/.openclaw/openclaw.json → 端口 + Token
  │             ├── 启动 `openclaw gateway run`（携带代理环境变量）
  │             ├── 轮询 /health 直到就绪
  │             └── 加载 http://127.0.0.1:{port}/#token={token}
  ├── 系统托盘：Show / Hide / 状态显示 / Quit
  └── 退出时终止 Gateway 子进程
```

## 项目结构

```
openclaw-desktop/
├── package.json
├── ui/                          # 前端（加载页、错误页、引导向导）
│   ├── index.html               # 主加载/错误页面
│   ├── setup.html               # 首次启动引导向导
│   ├── setup.js                 # 引导向导逻辑
│   ├── app.js                   # 主页面逻辑
│   └── style.css                # 共享样式（深色主题）
└── src-tauri/
    ├── Cargo.toml
    ├── tauri.conf.json
    ├── capabilities/default.json
    ├── icons/
    └── src/
        ├── main.rs              # 入口
        ├── lib.rs               # App 构建器 + 首次运行判断 + 事件循环
        ├── config.rs            # 读取 ~/.openclaw/openclaw.json
        ├── gateway.rs           # 健康检查 + 自动启动 + 代理注入
        ├── setup.rs             # 引导向导后端（环境检测、安装、配置生成）
        ├── tray.rs              # 系统托盘菜单 + 后台状态监控
        └── commands.rs          # Tauri IPC 命令
```

## 配置

应用读取（首次运行时自动创建）`~/.openclaw/openclaw.json`：

```json
{
  "gateway": {
    "port": 18789,
    "bind": "loopback",
    "auth": {
      "mode": "token",
      "token": "自动生成"
    }
  },
  "providers": {
    "minimax": {
      "apiKey": "你的 API Key"
    }
  },
  "defaultProvider": "minimax",
  "defaultModel": "MiniMax-M1"
}
```

首次运行时由引导向导自动生成，无需手动编辑。

## 常见问题

| 问题 | 解决方法 |
|------|----------|
| 引导页提示"Node.js 未安装" | 安装 Node.js v18+：`curl -fsSL https://deb.nodesource.com/setup_22.x \| sudo -E bash - && sudo apt-get install -y nodejs` |
| "一键安装"按钮失败 | 检查网络连接；如果在代理后面，先配置系统代理 |
| 设置完成后显示 "Gateway Offline" | 查看日志：`cat ~/.openclaw/desktop-gateway.log` |
| Telegram 没有回复 | 确认系统代理已配置（GNOME 设置 → 网络 → 代理） |
| 出现多个实例 | `pkill -f openclaw-desktop` 然后重新启动 |
| 窗口消失了 | 点击系统托盘图标，或右键 → Show |
| 想重新运行引导向导 | 删除 `~/.openclaw/openclaw.json` 后重新启动 |

## 许可证

MIT
