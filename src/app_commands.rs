//! Tauri commands exposed to the main window UI (account list, settings,
//! about, account open/hide/remove). These are invoked from `ui/index.html`
//! via `window.__TAURI__.core.invoke(...)`.

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

use crate::api::page_ops::page_for;
use crate::config::AccountConfig;
use crate::window::{
    account_window_label, create_account_with_bridge, create_main_window,
};

/// Snapshot of one account as seen by the main UI.
#[derive(Clone, Serialize)]
pub struct AccountView {
    pub id: String,
    pub label: String,
    pub enabled: bool,
    pub window_visible: bool,
    pub session_state: String, // "detecting" | "scan_qr" | "logged_in"
}

/// API configuration mirror.
#[derive(Clone, Serialize, Deserialize)]
pub struct ApiConfigView {
    pub enabled: bool,
    pub port: u16,
    pub api_key: String,
    pub webhook_url: String,
    pub webhook_secret: String,
}

impl Default for ApiConfigView {
    fn default() -> Self {
        Self {
            enabled: false,
            port: crate::config::DEFAULT_API_PORT,
            api_key: String::new(),
            webhook_url: String::new(),
            webhook_secret: String::new(),
        }
    }
}

/// List all configured accounts with their current window/session state.
#[tauri::command]
pub async fn get_accounts(app: AppHandle) -> Vec<AccountView> {
    let settings = match crate::SHARED.get() {
        Some(shared) => shared.settings.clone(),
        None => return Vec::new(),
    };
    let accounts = settings.current_accounts();
    let mut views = Vec::new();
    for account in &accounts {
        let label = app.get_webview_window(&account_window_label(&account.id));
        let visible = label.map(|w| w.is_visible().unwrap_or(false)).unwrap_or(false);
        let session_state = if let Some(bridge) = page_for(Some(&account.id)) {
            match bridge.status_snapshot().await {
                Ok(snapshot) => {
                    let logged_in = snapshot
                        .get("composer_visible")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false)
                        || snapshot
                            .get("chat_list")
                            .and_then(|v| v.as_bool())
                            .unwrap_or(false);
                    let qrcode = snapshot
                        .get("qrcode_visible")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);
                    if logged_in {
                        "logged_in".to_string()
                    } else if qrcode {
                        "scan_qr".to_string()
                    } else {
                        "detecting".to_string()
                    }
                }
                Err(_) => "detecting".to_string(),
            }
        } else {
            "detecting".to_string()
        };
        views.push(AccountView {
            id: account.id.clone(),
            label: account.label.clone(),
            enabled: account.enabled,
            window_visible: visible,
            session_state,
        });
    }
    views
}

/// Show (focus) an account's WhatsApp Web window.
#[tauri::command]
pub fn open_account(app: AppHandle, id: String) -> Result<(), String> {
    let label = account_window_label(&id);
    if let Some(window) = app.get_webview_window(&label) {
        window.show().map_err(|e| e.to_string())?;
        window.set_focus().map_err(|e| e.to_string())?;
        return Ok(());
    }
    // Window doesn't exist yet: create it from config.
    let settings = crate::SHARED.get().map(|s| s.settings.clone()).ok_or("settings")?;
    let account = settings.account(&id).ok_or_else(|| format!("account '{id}' not found"))?;
    let window = create_account_with_bridge(&app, &account, true)
        .map_err(|e| e.to_string())?;
    window.set_focus().map_err(|e| e.to_string())?;
    Ok(())
}

/// Hide an account's WhatsApp Web window (keeps it running in background).
#[tauri::command]
pub fn close_account_window(app: AppHandle, id: String) -> Result<(), String> {
    let label = account_window_label(&id);
    if let Some(window) = app.get_webview_window(&label) {
        window.hide().map_err(|e| e.to_string())?;
        Ok(())
    } else {
        Err(format!("window for account '{id}' not found"))
    }
}

/// Add a new account (persisted in config.json) and create its hidden window.
#[tauri::command]
pub fn add_account(app: AppHandle, id: String, label: String) -> Result<(), String> {
    let settings = crate::SHARED.get().map(|s| s.settings.clone()).ok_or("settings")?;
    let mut accounts = settings.current_accounts();
    if accounts.iter().any(|a| a.id == id) {
        return Err(format!("account '{id}' already exists"));
    }
    accounts.push(AccountConfig {
        id: id.clone(),
        label: label.clone(),
        enabled: true,
    });
    settings.update_accounts(accounts).map_err(|e| e.to_string())?;
    let account = AccountConfig { id, label, enabled: true };
    // Create hidden so the user can open it from the list.
    let _window = create_account_with_bridge(&app, &account, false)
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// Remove an account from config and hide its window.
#[tauri::command]
pub fn remove_account(app: AppHandle, id: String) -> Result<(), String> {
    let settings = crate::SHARED.get().map(|s| s.settings.clone()).ok_or("settings")?;
    let before = settings.current_accounts().len();
    let accounts: Vec<AccountConfig> = settings
        .current_accounts()
        .into_iter()
        .filter(|a| a.id != id)
        .collect();
    if accounts.len() == before {
        return Err(format!("account '{id}' not found"));
    }
    settings.update_accounts(accounts).map_err(|e| e.to_string())?;
    // Hide the window if it exists.
    if let Some(window) = app.get_webview_window(&account_window_label(&id)) {
        let _ = window.hide();
    }
    Ok(())
}

/// Get current API configuration.
#[tauri::command]
pub fn get_api_config() -> ApiConfigView {
    let settings = crate::SHARED.get().map(|s| s.settings.clone()).unwrap();
    let api = settings.current();
    ApiConfigView {
        enabled: api.enabled,
        port: api.port,
        api_key: api.api_key,
        webhook_url: api.webhook_url,
        webhook_secret: api.webhook_secret,
    }
}

/// Save API configuration (persisted in config.json). (Re)starts server if enabled.
#[tauri::command]
pub fn save_api_config(config: ApiConfigView) -> Result<(), String> {
    let settings = crate::SHARED.get().map(|s| s.settings.clone()).ok_or("settings")?;
    let mut api = crate::config::ApiConfig {
        enabled: config.enabled,
        port: config.port,
        api_key: config.api_key,
        expose_network: false,
        webhook_url: config.webhook_url,
        webhook_secret: config.webhook_secret,
    };
    api = api.sanitized();
    settings.update_api(api.clone()).map_err(|e| e.to_string())?;
    if api.enabled {
        crate::api::server::start_loopback_server();
    } else {
        crate::api::server::shutdown_loopback_server();
    }
    Ok(())
}

/// Quit the application (from the UI).
#[tauri::command]
pub fn quit_app(app: AppHandle) {
    crate::api::server::shutdown_loopback_server();
    app.exit(0);
}

/// Ensure the main window exists (called from setup if needed).
#[tauri::command]
pub fn ensure_main_window(app: AppHandle) -> Result<(), String> {
    if app.get_webview_window("main").is_some() {
        return Ok(());
    }
    let _window = create_main_window(&app).map_err(|e| e.to_string())?;
    Ok(())
}
