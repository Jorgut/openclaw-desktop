use serde::Serialize;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[derive(Debug, Clone, Serialize)]
pub struct PrereqStatus {
    pub node_installed: bool,
    pub node_version: String,
    pub npm_installed: bool,
    pub openclaw_installed: bool,
    pub openclaw_version: String,
    pub config_exists: bool,
    pub proxy_detected: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProxyInfo {
    pub detected: bool,
    pub http: String,
    pub socks: String,
    pub source: String,
}

fn openclaw_config_path() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".openclaw").join("openclaw.json"))
}

fn run_command_output(cmd: &str, args: &[&str]) -> Option<String> {
    Command::new(cmd)
        .args(args)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
}

fn gsettings_get(schema: &str, key: &str) -> Option<String> {
    run_command_output("gsettings", &["get", schema, key])
        .map(|s| s.trim_matches('\'').to_string())
}

#[tauri::command]
pub fn is_first_run() -> bool {
    match openclaw_config_path() {
        Some(path) => !path.exists(),
        None => true,
    }
}

#[tauri::command]
pub fn check_prerequisites() -> PrereqStatus {
    let node_version = run_command_output("node", &["--version"]).unwrap_or_default();
    let node_installed = !node_version.is_empty();

    let npm_version = run_command_output("npm", &["--version"]).unwrap_or_default();
    let npm_installed = !npm_version.is_empty();

    let openclaw_version = run_command_output("openclaw", &["--version"]).unwrap_or_default();
    let openclaw_installed = !openclaw_version.is_empty();

    let config_exists = openclaw_config_path()
        .map(|p| p.exists())
        .unwrap_or(false);

    let proxy = detect_proxy_inner();
    let proxy_detected = if proxy.detected {
        Some(proxy.http.clone())
    } else {
        None
    };

    PrereqStatus {
        node_installed,
        node_version,
        npm_installed,
        openclaw_installed,
        openclaw_version,
        config_exists,
        proxy_detected,
    }
}

#[tauri::command]
pub async fn install_openclaw() -> Result<String, String> {
    let output = tauri::async_runtime::spawn_blocking(|| {
        Command::new("npm")
            .args(["install", "-g", "openclaw"])
            .output()
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
    .map_err(|e| format!("Failed to run npm: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if output.status.success() {
        Ok(format!("{}\n{}", stdout, stderr).trim().to_string())
    } else {
        Err(format!(
            "npm install failed (exit code {}):\n{}\n{}",
            output.status.code().unwrap_or(-1),
            stdout,
            stderr
        ))
    }
}

fn detect_proxy_inner() -> ProxyInfo {
    // Check environment variables first
    if let Ok(proxy) = std::env::var("HTTP_PROXY").or_else(|_| std::env::var("http_proxy")) {
        let socks = std::env::var("ALL_PROXY")
            .or_else(|_| std::env::var("all_proxy"))
            .unwrap_or_default();
        return ProxyInfo {
            detected: true,
            http: proxy,
            socks,
            source: "env".to_string(),
        };
    }

    // Fall back to GNOME gsettings
    let mode = gsettings_get("org.gnome.system.proxy", "mode");
    if mode.as_deref() == Some("manual") {
        let host = gsettings_get("org.gnome.system.proxy.http", "host").unwrap_or_default();
        let port = gsettings_get("org.gnome.system.proxy.http", "port").unwrap_or_default();

        let socks_host =
            gsettings_get("org.gnome.system.proxy.socks", "host").unwrap_or_default();
        let socks_port =
            gsettings_get("org.gnome.system.proxy.socks", "port").unwrap_or_default();

        let http = if !host.is_empty() && port != "0" {
            format!("http://{}:{}", host, port)
        } else {
            String::new()
        };

        let socks = if !socks_host.is_empty() && socks_port != "0" {
            format!("socks://{}:{}", socks_host, socks_port)
        } else {
            String::new()
        };

        if !http.is_empty() || !socks.is_empty() {
            return ProxyInfo {
                detected: true,
                http,
                socks,
                source: "gsettings".to_string(),
            };
        }
    }

    ProxyInfo {
        detected: false,
        http: String::new(),
        socks: String::new(),
        source: "none".to_string(),
    }
}

#[tauri::command]
pub fn detect_proxy() -> ProxyInfo {
    detect_proxy_inner()
}

#[tauri::command]
pub fn save_initial_config(
    provider: String,
    api_key: String,
    model: String,
    telegram_token: Option<String>,
    discord_token: Option<String>,
    proxy_url: Option<String>,
) -> Result<(), String> {
    let home = dirs::home_dir().ok_or("Could not determine home directory")?;
    let openclaw_dir = home.join(".openclaw");

    // Create ~/.openclaw/ directory if it doesn't exist
    fs::create_dir_all(&openclaw_dir)
        .map_err(|e| format!("Failed to create ~/.openclaw: {}", e))?;

    let config_path = openclaw_dir.join("openclaw.json");

    // Build the config JSON
    let mut config = serde_json::json!({
        "gateway": {
            "port": 18789,
            "bind": "loopback",
            "auth": {
                "mode": "token",
                "token": generate_token()
            }
        },
        "providers": {
            provider.clone(): {
                "apiKey": api_key
            }
        },
        "defaultProvider": provider,
        "defaultModel": model
    });

    // Add proxy config if provided
    if let Some(ref proxy) = proxy_url {
        if !proxy.is_empty() {
            config["proxy"] = serde_json::json!({
                "url": proxy
            });
        }
    }

    let config_str = serde_json::to_string_pretty(&config)
        .map_err(|e| format!("Failed to serialize config: {}", e))?;

    fs::write(&config_path, &config_str)
        .map_err(|e| format!("Failed to write config: {}", e))?;

    // Add channels via openclaw CLI if tokens provided
    if let Some(ref token) = telegram_token {
        if !token.is_empty() {
            add_channel("telegram", token);
        }
    }

    if let Some(ref token) = discord_token {
        if !token.is_empty() {
            add_channel("discord", token);
        }
    }

    Ok(())
}

fn add_channel(channel_type: &str, token: &str) {
    let result = Command::new("openclaw")
        .args(["channels", "add", channel_type, "--token", token])
        .output();

    match result {
        Ok(output) => {
            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                eprintln!("Failed to add {} channel: {}", channel_type, stderr);
            }
        }
        Err(e) => eprintln!("Failed to run openclaw channels add: {}", e),
    }
}

fn generate_token() -> String {
    use std::collections::hash_map::RandomState;
    use std::hash::{BuildHasher, Hasher};

    let mut result = String::with_capacity(32);
    let chars = b"abcdefghijklmnopqrstuvwxyz0123456789";

    for _ in 0..32 {
        let s = RandomState::new();
        let mut hasher = s.build_hasher();
        hasher.write_u8(0);
        let idx = (hasher.finish() as usize) % chars.len();
        result.push(chars[idx] as char);
    }

    result
}
