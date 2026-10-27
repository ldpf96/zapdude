//! ZapZap (Rust) — WhatsApp Web desktop client with a local automation API.
//!
//! Architecture:
#![windows_subsystem = "windows"]
//!   - One MAIN window (human interface): loads `ui/index.html`, shows the
//!     account list, API settings and About. Always the first visible window.
//!   - One WhatsApp Web window per account (`account-<id>`), isolated profile,
//!     created hidden at startup and shown on demand from the main UI.
//!   - Tray menu for quick account switching and "Exit ZapZap".
//!   - Loopback HTTP API for people and AI agents (opt-in via settings).

mod api;
mod app_commands;
mod config;
mod window;

use std::sync::OnceLock;

use tauri::Manager;

/// Global shared state used by the window and the API server threads.
pub static SHARED: OnceLock<SharedGlobals> = OnceLock::new();

#[derive(Clone, Debug)]
pub struct SharedGlobals {
    pub settings: config::SettingsState,
}

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            let data_dir = app
                .path()
                .app_data_dir()
                .unwrap_or_else(|_| std::env::temp_dir().join("zapdude"));
            let settings = config::SettingsState::load(&data_dir);
            let shared = SharedGlobals { settings };
            let _ = SHARED.set(shared.clone());

            // 1. Main window (human interface) — visible.
            let app_handle = app.handle().clone();
            window::create_main_window(&app_handle)?;

            // 2. One hidden WhatsApp Web window per enabled account.
            let accounts: Vec<_> = shared
                .settings
                .current_accounts()
                .into_iter()
                .filter(|account| account.enabled)
                .collect();
            for account in &accounts {
                let _ = window::create_account_with_bridge(&app_handle, account, false);
            }

            // 3. Tray (human interface for switching + exit).
            window::initialize_tray(app);

            // 4. Loopback API (only if enabled in config).
            crate::api::server::start_loopback_server();

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            app_commands::get_accounts,
            app_commands::open_account,
            app_commands::close_account_window,
            app_commands::add_account,
            app_commands::remove_account,
            app_commands::get_api_config,
            app_commands::save_api_config,
            app_commands::quit_app,
            app_commands::ensure_main_window,
        ])
        .on_window_event(|window, event| {
            // Never exit on window close: hide instead. The app only exits
            // through the tray "Exit ZapZap" or the UI "Quit" command.
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running ZapZap");
}
