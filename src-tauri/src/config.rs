use serde::Deserialize;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize)]
pub struct OpenClawConfig {
    pub gateway: GatewayConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GatewayConfig {
    pub port: u16,
    #[serde(default = "default_bind")]
    pub bind: String,
    #[serde(default)]
    pub auth: AuthConfig,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct AuthConfig {
    #[serde(default)]
    pub mode: String,
    #[serde(default)]
    pub token: String,
}

fn default_bind() -> String {
    "loopback".to_string()
}

fn config_path() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".openclaw").join("openclaw.json"))
}

pub fn load_config() -> Result<OpenClawConfig, String> {
    let path = config_path().ok_or("Could not determine home directory")?;

    if !path.exists() {
        return Err(format!("Config not found: {}", path.display()));
    }

    let content = fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;

    let config: OpenClawConfig = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse config: {}", e))?;

    Ok(config)
}

impl GatewayConfig {
    pub fn base_url(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }

    pub fn full_url(&self) -> String {
        if self.auth.token.is_empty() {
            self.base_url()
        } else {
            format!("{}/#token={}", self.base_url(), self.auth.token)
        }
    }
}
