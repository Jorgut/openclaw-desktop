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

/// Start the gateway if not already running.
/// If a previous gateway is listening (e.g. orphan from crash), reuse it.
pub fn ensure_started() {
    let base_url = match config::load_config() {
        Ok(cfg) => cfg.gateway.base_url(),
        Err(e) => {
            eprintln!("Cannot read config to start gateway: {}", e);
            return;
        }
    };

    if check_health(&base_url) {
        eprintln!("Gateway already running, reusing");
        return;
    }

    // Resolve openclaw binary from common locations
    let bin = find_openclaw_bin().unwrap_or_else(|| "openclaw".to_string());

    // Use `gateway run` (foreground child process) instead of `gateway start` (systemd).
    // This keeps Telegram and other plugins working the same as terminal usage.
    let mut cmd = Command::new(&bin);
    cmd.args(["gateway", "run"])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());

    // Inject proxy into our own process env so all descendants inherit it.
    inject_proxy_into_process_env();

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

/// Inject proxy env vars into the current process so ALL descendant processes
/// inherit them — including grandchildren spawned by `openclaw gateway run`.
/// Reads from the current env first, falls back to GNOME gsettings.
fn inject_proxy_into_process_env() {
    if std::env::var("HTTP_PROXY").is_ok() || std::env::var("http_proxy").is_ok() {
        // Already present (e.g. launched from a terminal with proxy).
        // Re-set them so they appear in both upper and lower-case forms.
        if let Ok(v) = std::env::var("HTTP_PROXY").or_else(|_| std::env::var("http_proxy")) {
            set_proxy_vars(&v);
        }
        return;
    }

    // Read from GNOME gsettings
    let mode = gsettings_get("org.gnome.system.proxy", "mode");
    if mode.as_deref() != Some("manual") {
        return;
    }

    let host = gsettings_get("org.gnome.system.proxy.http", "host");
    let port = gsettings_get("org.gnome.system.proxy.http", "port");

    if let (Some(h), Some(p)) = (host, port) {
        if !h.is_empty() && p != "0" {
            let proxy = format!("http://{}:{}/", h, p);
            eprintln!("Proxy from gsettings: {}", proxy);
            set_proxy_vars(&proxy);
        }
    }

    let socks_host = gsettings_get("org.gnome.system.proxy.socks", "host");
    let socks_port = gsettings_get("org.gnome.system.proxy.socks", "port");

    if let (Some(h), Some(p)) = (socks_host, socks_port) {
        if !h.is_empty() && p != "0" {
            let socks = format!("socks://{}:{}/", h, p);
            std::env::set_var("ALL_PROXY", &socks);
            std::env::set_var("all_proxy", &socks);
        }
    }

    std::env::set_var("NO_PROXY", "localhost,127.0.0.1,192.168.0.0/16,10.0.0.0/8,172.16.0.0/12,::1");
    std::env::set_var("no_proxy", "localhost,127.0.0.1,192.168.0.0/16,10.0.0.0/8,172.16.0.0/12,::1");
}

fn set_proxy_vars(proxy: &str) {
    std::env::set_var("HTTP_PROXY", proxy);
    std::env::set_var("http_proxy", proxy);
    std::env::set_var("HTTPS_PROXY", proxy);
    std::env::set_var("https_proxy", proxy);
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
