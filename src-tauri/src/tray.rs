use tauri::{
    image::Image,
    menu::{MenuBuilder, MenuItem, MenuItemBuilder},
    tray::TrayIconBuilder,
    AppHandle, Emitter, Manager,
};

use crate::config;
use crate::gateway;

pub fn create_tray(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let show = MenuItemBuilder::with_id("show", "Show Window").build(app)?;
    let hide = MenuItemBuilder::with_id("hide", "Hide Window").build(app)?;
    let status = MenuItemBuilder::with_id("status", "Status: Checking...")
        .enabled(false)
        .build(app)?;
    let quit = MenuItemBuilder::with_id("quit", "Quit").build(app)?;

    let menu = MenuBuilder::new(app)
        .item(&show)
        .item(&hide)
        .separator()
        .item(&status)
        .separator()
        .item(&quit)
        .build()?;

    let icon = Image::from_bytes(include_bytes!("../icons/32x32.png"))?;

    let _tray = TrayIconBuilder::new()
        .icon(icon)
        .menu(&menu)
        .tooltip("OpenClaw Desktop")
        .on_menu_event(move |app, event| match event.id().as_ref() {
            "show" => {
                if let Some(win) = app.get_webview_window("main") {
                    let _ = win.unminimize();
                    let _ = win.show();
                    let _ = win.set_focus();
                }
            }
            "hide" => {
                if let Some(win) = app.get_webview_window("main") {
                    let _ = win.hide();
                }
            }
            "quit" => {
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let tauri::tray::TrayIconEvent::Click { .. } = event {
                let app = tray.app_handle();
                if let Some(win) = app.get_webview_window("main") {
                    let _ = win.unminimize();
                    let _ = win.show();
                    let _ = win.set_focus();
                }
            }
        })
        .build(app)?;

    // Start health monitor with the status menu item so it can update the label
    start_health_monitor(app.clone(), status);

    Ok(())
}

fn start_health_monitor(app: AppHandle, status_item: MenuItem<tauri::Wry>) {
    std::thread::spawn(move || {
        // Initial check after 5 seconds (faster first update)
        std::thread::sleep(std::time::Duration::from_secs(5));

        loop {
            let status_text = match config::load_config() {
                Ok(cfg) => {
                    if gateway::check_health(&cfg.gateway.base_url()) {
                        "Status: Online"
                    } else {
                        "Status: Offline"
                    }
                }
                Err(_) => "Status: No Config",
            };

            if let Err(e) = status_item.set_text(status_text) {
                eprintln!("Failed to update tray status: {}", e);
            }

            // Also emit event for frontend
            let _ = app.emit_to("main", "gateway-status", status_text);

            std::thread::sleep(std::time::Duration::from_secs(15));
        }
    });
}
