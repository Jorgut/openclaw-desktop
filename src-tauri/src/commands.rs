use crate::config;
use crate::gateway;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct GatewayInfo {
    pub url: String,
    pub token: String,
    pub port: u16,
    pub full_url: String,
}

#[tauri::command]
pub fn get_gateway_info() -> Result<GatewayInfo, String> {
    let cfg = config::load_config()?;
    let gw = &cfg.gateway;

    Ok(GatewayInfo {
        url: gw.base_url(),
        token: gw.auth.token.clone(),
        port: gw.port,
        full_url: gw.full_url(),
    })
}

#[tauri::command]
pub fn check_gateway_status() -> Result<bool, String> {
    let cfg = config::load_config()?;
    Ok(gateway::check_health(&cfg.gateway.base_url()))
}

#[tauri::command]
pub fn get_gateway_url() -> Result<String, String> {
    let cfg = config::load_config()?;
    Ok(cfg.gateway.full_url())
}
