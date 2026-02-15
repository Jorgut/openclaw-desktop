mod commands;
mod config;
mod gateway;
mod setup;
mod tray;

use tauri::Manager;

pub fn run() {
    let first_run = setup::is_first_run();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            commands::get_gateway_info,
            commands::check_gateway_status,
            commands::get_gateway_url,
            commands::start_gateway,
            setup::is_first_run,
            setup::check_prerequisites,
            setup::install_openclaw,
            setup::detect_proxy,
            setup::save_initial_config,
        ])
        .setup(move |app| {
            if first_run {
                // First run: load setup wizard, don't start gateway
                let win = app
                    .get_webview_window("main")
                    .expect("main window not found");
                let _ = win.eval("window.location.replace('setup.html')");
            } else {
                // Normal run: start gateway
                gateway::ensure_started();
            }

            // Create system tray (includes health monitor)
            if let Err(e) = tray::create_tray(app.handle()) {
                eprintln!("Failed to create tray: {}", e);
            }

            // Intercept window close → hide instead of quit
            let win = app
                .get_webview_window("main")
                .expect("main window not found");

            let win_clone = win.clone();
            win.on_window_event(move |event| {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let _ = win_clone.hide();
                }
            });

            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building OpenClaw Desktop")
        .run(|_app, event| {
            if let tauri::RunEvent::Exit = event {
                gateway::shutdown();
            }
        });
}
