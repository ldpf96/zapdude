//! ZapZap (Rust) — WhatsApp Web desktop client with a local automation API.
//!
//! Shell architecture (like the original ZapZap):
#![windows_subsystem = "windows"]
//!   - ONE window ("main") hosts every webview:
//!       * `rail`        — the narrow account rail (avatars, "+", settings)
//!       * `welcome`     — onboarding shown when no account is active
//!       * `account-<id>` — one WhatsApp Web webview per account, switched
//!         in place like browser tabs, each with its own isolated profile.
//!   - A separate "settings" window holds API config, AI agent info and About.
//!   - A tray menu mirrors the rail for quick account switching.
//!   - A loopback HTTP API lets people and AI agents control the app.

mod api;
mod app_commands;
mod config;
mod window;

use std::sync::OnceLock;

use tauri::Manager;

/// Global shared state used by the window and the API server threads.
pub static SHARED: OnceLock<SharedGlobals> = OnceLock::new();

/// Global AppHandle so HTTP endpoints can show/hide account webviews.
pub static APP_HANDLE: OnceLock<AppHandleBox> = OnceLock::new();

pub type AppHandleBox = tauri::AppHandle<tauri::Wry>;

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
                .unwrap_or_else(|_| std::env::temp_dir().join("app.zapzap.desktop"));
            let settings = config::SettingsState::load(&data_dir);
            eprintln!(
                "zapdude: settings loaded, {} accounts, api.enabled={}",
                settings.current_accounts().len(),
                settings.current().enabled
            );
            let _ = SHARED.set(SharedGlobals {
                settings: settings.clone(),
            });
            let _ = APP_HANDLE.set(app.handle().clone());

            // 1. The shell: rail + welcome screen (created hidden, shown when
            //    the webviews are ready so there is no white flash).
            let app_handle = app.handle().clone();
            window::create_shell(&app_handle)?;

            // 2. One hidden WhatsApp Web webview per enabled account so
            //    notifications keep working in the background.
            for account in settings
                .current_accounts()
                .into_iter()
                .filter(|account| account.enabled)
            {
                if let Err(err) = window::create_account_webview_hidden(&app_handle, &account) {
                    eprintln!(
                        "zapdude: could not create webview for '{}': {}",
                        account.id, err
                    );
                }
            }

            // 3. Tray icon with per-account entries.
            window::initialize_tray(app);

            // 4. Loopback API for agents (the UI uses IPC, not HTTP).
            crate::api::server::start_loopback_server();

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            app_commands::get_accounts,
            app_commands::add_account,
            app_commands::open_account,
            app_commands::hide_account,
            app_commands::remove_account,
            app_commands::open_settings,
            app_commands::get_api_config,
            app_commands::save_api_config,
            app_commands::quit_app,
        ])
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                // The shell hides to tray; auxiliary windows (settings) close.
                if window.label() == "main" {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running ZapZap");
}
