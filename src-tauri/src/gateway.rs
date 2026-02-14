use std::process::{Child, Command};
use std::sync::Mutex;
use std::time::Duration;

use crate::config;

static GATEWAY_PROCESS: Mutex<Option<Child>> = Mutex::new(None);

/// Kill the spawned gateway process on app exit.
pub fn shutdown() {
    if let Ok(mut guard) = GATEWAY_PROCESS.lock() {
        if let Some(ref mut child) = *guard {
            let _ = child.kill();
            let _ = child.wait();
            eprintln!("Gateway process stopped");
        }
    }
}

/// Start the gateway, killing any existing instance first.
/// Always starts fresh to guarantee proxy env vars are set correctly.
pub fn ensure_started() {
    let base_url = match config::load_config() {
        Ok(cfg) => cfg.gateway.base_url(),
        Err(e) => {
            eprintln!("Cannot read config to start gateway: {}", e);
            return;
        }
    };

    // Always kill existing gateway so we start fresh with correct proxy env.
    kill_existing_gateway();

    // Resolve openclaw binary from common locations
    let bin = find_openclaw_bin().unwrap_or_else(|| "openclaw".to_string());

    // Build a shell command that exports proxy vars THEN exec's the gateway.
    // This guarantees ALL descendant processes inherit proxy settings,
    // even if openclaw internally re-spawns with a clean env.
    //
    // For Node.js (undici/fetch), we also set GLOBAL_AGENT_HTTP_PROXY so
    // libraries like global-agent can intercept requests, and pass
    // --use-openssl-ca to ensure TLS works through the proxy.
    let proxy_exports = build_proxy_exports();
    let shell_cmd = format!("{}exec '{}' gateway run", proxy_exports, bin);

    eprintln!("Shell command: {}", shell_cmd);

    // Log to file so we can debug issues
    let log_path = dirs::home_dir()
        .map(|h| h.join(".openclaw/desktop-gateway.log"))
        .unwrap_or_else(|| std::path::PathBuf::from("/tmp/openclaw-gateway.log"));

    let log_file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path);

    let mut cmd = Command::new("bash");
    cmd.args(["-c", &shell_cmd])
        .stdout(std::process::Stdio::null());

    match log_file {
        Ok(file) => { cmd.stderr(std::process::Stdio::from(file)); }
        Err(_) => { cmd.stderr(std::process::Stdio::null()); }
    }

    match cmd.spawn()
    {
        Ok(child) => {
            eprintln!("Gateway spawned (pid {}) via {}", child.id(), bin);
            GATEWAY_PROCESS.lock().unwrap().replace(child);

            // Wait for gateway to become ready before UI starts checking
            wait_until_healthy(&base_url, 20);
        }
        Err(e) => eprintln!("Failed to start gateway: {}", e),
    }
}

/// Kill any existing openclaw-gateway process so we can start fresh.
fn kill_existing_gateway() {
    // Kill via pkill
    let _ = Command::new("pkill").args(["-9", "-f", "openclaw-gateway"]).status();
    let _ = Command::new("pkill").args(["-9", "-f", "openclaw gateway"]).status();
    // Also stop systemd service if running
    let _ = Command::new("systemctl")
        .args(["--user", "stop", "openclaw-gateway.service"])
        .status();
    // Wait for port to free up
    std::thread::sleep(Duration::from_secs(2));
}

/// Poll health endpoint until gateway is ready, up to `max_secs` seconds.
fn wait_until_healthy(base_url: &str, max_secs: u32) {
    for i in 0..(max_secs * 2) {
        std::thread::sleep(Duration::from_millis(500));
        if check_health(base_url) {
            eprintln!("Gateway ready after ~{}ms", (i + 1) * 500);
            return;
        }
    }
    eprintln!("Gateway not ready after {}s, UI will retry", max_secs);
}

fn find_openclaw_bin() -> Option<String> {
    let candidates = [
        dirs::home_dir().map(|h| h.join(".npm-global/bin/openclaw")),
        dirs::home_dir().map(|h| h.join(".local/bin/openclaw")),
    ];

    for candidate in candidates.into_iter().flatten() {
        if candidate.exists() {
            return Some(candidate.to_string_lossy().into_owned());
        }
    }

    // Fall back to PATH lookup
    Command::new("which")
        .arg("openclaw")
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
}

/// Build shell export lines for proxy env vars.
/// Checks current process env first, falls back to GNOME gsettings.
fn build_proxy_exports() -> String {
    let mut exports = Vec::new();
    let no_proxy = "localhost,127.0.0.1,192.168.0.0/16,10.0.0.0/8,172.16.0.0/12,::1";

    // Try current process env first (e.g. launched from terminal)
    if let Ok(proxy) = std::env::var("HTTP_PROXY").or_else(|_| std::env::var("http_proxy")) {
        eprintln!("Proxy from env: {}", proxy);
        exports.push(format!("export HTTP_PROXY='{}' http_proxy='{}' HTTPS_PROXY='{}' https_proxy='{}';",
            proxy, proxy, proxy, proxy));
        // Also set GLOBAL_AGENT vars for Node.js global-agent compatibility
        exports.push(format!("export GLOBAL_AGENT_HTTP_PROXY='{}' GLOBAL_AGENT_HTTPS_PROXY='{}';",
            proxy, proxy));
        if let Ok(all) = std::env::var("ALL_PROXY").or_else(|_| std::env::var("all_proxy")) {
            exports.push(format!("export ALL_PROXY='{}' all_proxy='{}';", all, all));
        }
        exports.push(format!("export NO_PROXY='{}' no_proxy='{}';", no_proxy, no_proxy));
        return exports.join(" ");
    }

    // Fall back to GNOME gsettings
    let mode = gsettings_get("org.gnome.system.proxy", "mode");
    if mode.as_deref() != Some("manual") {
        return String::new();
    }

    let host = gsettings_get("org.gnome.system.proxy.http", "host");
    let port = gsettings_get("org.gnome.system.proxy.http", "port");

    if let (Some(h), Some(p)) = (host, port) {
        if !h.is_empty() && p != "0" {
            let proxy = format!("http://{}:{}/", h, p);
            eprintln!("Proxy from gsettings: {}", proxy);
            exports.push(format!("export HTTP_PROXY='{}' http_proxy='{}' HTTPS_PROXY='{}' https_proxy='{}';",
                proxy, proxy, proxy, proxy));
            // Also set GLOBAL_AGENT vars for Node.js global-agent compatibility
            exports.push(format!("export GLOBAL_AGENT_HTTP_PROXY='{}' GLOBAL_AGENT_HTTPS_PROXY='{}';",
                proxy, proxy));
        }
    }

    let socks_host = gsettings_get("org.gnome.system.proxy.socks", "host");
    let socks_port = gsettings_get("org.gnome.system.proxy.socks", "port");

    if let (Some(h), Some(p)) = (socks_host, socks_port) {
        if !h.is_empty() && p != "0" {
            let socks = format!("socks://{}:{}/", h, p);
            exports.push(format!("export ALL_PROXY='{}' all_proxy='{}';", socks, socks));
        }
    }

    exports.push(format!("export NO_PROXY='{}' no_proxy='{}';", no_proxy, no_proxy));
    exports.join(" ")
}

fn gsettings_get(schema: &str, key: &str) -> Option<String> {
    Command::new("gsettings")
        .args(["get", schema, key])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().trim_matches('\'').to_string())
}

pub fn check_health(base_url: &str) -> bool {
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(3))
        .build();

    let client = match client {
        Ok(c) => c,
        Err(_) => return false,
    };

    let url = format!("{}/health", base_url);

    match client.get(&url).send() {
        Ok(resp) => resp.status().is_success(),
        Err(_) => {
            // Fall back: try fetching the root page
            match client.get(base_url).send() {
                Ok(resp) => resp.status().is_success(),
                Err(_) => false,
            }
        }
    }
}
