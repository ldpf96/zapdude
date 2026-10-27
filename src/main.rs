//! ZapZap (Rust) — WhatsApp Web desktop client with a local automation API.
//!
//! Loads WhatsApp Web in WebView2 windows (one per account, isolated
//! profiles) with a Chrome-aligned identity so voice/video calls and screen
//! sharing entry points stay available, and exposes a loopback HTTP API for
//! people and AI agents.

mod api;
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
            window::create_account_windows(app)?;
            Ok(())
        })
        .on_window_event(|window, event| {
            // Account windows keep running hidden so notifications from
            // every account stay alive; closing any of them just hides it.
            // The app is exited through the tray menu ("Exit ZapZap").
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                if window.label().starts_with("account-") {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running ZapZap");
}
