//! Persistent settings for Zapdude.
//!
//! Holds the `api/*` keys plus the multi-account registry and UI prefs.
//! Everything lives in one JSON file inside the OS application-data
//! directory. Only keys the automation layer reads are persisted; labels
//! are kept separately from ids.

use std::path::{Path, PathBuf};
use std::sync::RwLock;

use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const DEFAULT_API_PORT: u16 = 29977;
pub const MIN_API_PORT: u16 = 1024;
pub const MAX_API_PORT: u16 = 65_535;

/// Automation API settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ApiConfig {
    pub enabled: bool,
    pub port: u16,
    pub api_key: String,
    pub expose_network: bool,
    pub webhook_url: String,
    pub webhook_secret: String,
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            port: DEFAULT_API_PORT,
            api_key: crate::api::protocol::generate_api_key(),
            expose_network: false,
            webhook_url: String::new(),
            webhook_secret: String::new(),
        }
    }
}

impl ApiConfig {
    pub fn sanitized(mut self) -> Self {
        if !api_port_ok(self.port) {
            self.port = DEFAULT_API_PORT;
        }
        {
            let key = self.api_key.trim();
            if key.len() >= 16 {
                self.api_key = key.to_string();
            } else {
                self.api_key = crate::api::protocol::generate_api_key();
            }
        }
        self.webhook_url = sanitize_url(&self.webhook_url);
        self.webhook_secret = self.webhook_secret.trim().to_string();
        self
    }
}

/// One WhatsApp account: stable id, human label, enabled flag.
///
/// `id` is the persisted identity (must survive label edits); `label` is
/// freely editable and shown in the tray and API outputs.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default, rename_all = "camelCase")]
pub struct AccountConfig {
    pub id: String,
    pub label: String,
    pub enabled: bool,
}

impl Default for AccountConfig {
    fn default() -> Self {
        Self {
            id: "default".to_string(),
            label: "default".to_string(),
            enabled: true,
        }
    }
}

fn sanitize_account_id(raw: &str) -> String {
    let cleaned: String = raw
        .trim()
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .take(48)
        .collect();
    cleaned
}

fn sanitize_accounts(raw: Vec<AccountConfig>) -> Vec<AccountConfig> {
    let mut seen = std::collections::HashSet::new();
    let mut result = Vec::new();
    for mut account in raw {
        account.id = sanitize_account_id(&account.id);
        if account.id.is_empty() || !seen.insert(account.id.clone()) {
            continue; // drop empty or duplicate ids
        }
        if account.label.trim().is_empty() {
            account.label = account.id.clone();
        } else {
            account.label = account.label.trim().to_string();
        }
        result.push(account);
    }
    // An EMPTY list is legal (the user removed every account); the shell
    // auto-creates a fresh account on the next open. The [default] fallback
    // only applies to legacy configs being parsed at load time.
    result
}

/// App-shell preferences (theme, autostart, minimized start).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default, rename_all = "camelCase")]
pub struct AppPrefs {
    /// Shell UI theme: "dark" (default) or "light".
    pub theme: String,
    /// Launch Zapdude when Windows starts (HKCU Run key).
    pub autostart: bool,
    /// Start hidden in the tray instead of showing the window.
    pub start_minimized: bool,
}

impl Default for AppPrefs {
    fn default() -> Self {
        Self {
            theme: "dark".to_string(),
            autostart: false,
            start_minimized: false,
        }
    }
}

impl AppPrefs {
    pub fn sanitized(mut self) -> Self {
        self.theme = if self.theme.eq_ignore_ascii_case("light") {
            "light".to_string()
        } else {
            "dark".to_string()
        };
        self
    }
}

/// Whole configuration file shape.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ConfigFile {
    pub api: ApiConfig,
    pub accounts: Vec<AccountConfig>,
    pub prefs: AppPrefs,
}

impl Default for ConfigFile {
    fn default() -> Self {
        Self {
            api: ApiConfig::default(),
            // VIRGIN installs start with NO accounts: the shell auto-opens
            // a fresh "Account 1" (account-1) with its QR. The legacy
            // "default" account name from other clients' configs is never
            // recreated.
            accounts: vec![],
            prefs: AppPrefs::default(),
        }
    }
}

fn sanitize_url(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty()
        || (!trimmed.starts_with("http://") && !trimmed.starts_with("https://"))
    {
        String::new()
    } else {
        trimmed.to_string()
    }
}

fn api_port_ok(value: u16) -> bool {
    (MIN_API_PORT..=MAX_API_PORT).contains(&value)
}

/// Thread-safe accessor to the persisted configuration.
#[derive(Clone, Debug)]
pub struct SettingsState {
    data_dir: PathBuf,
    inner: std::sync::Arc<RwLock<ConfigFile>>,
}

impl SettingsState {
    /// Load the settings from the given data directory (legacy-free, the
    /// Rust app is the only owner of the file).
    pub fn load(data_dir: &Path) -> Self {
        let path = data_dir.join("config.json");
        // A concurrent writer (installer race, second instance starting
        // while the first persists) can hand us a half-written JSON: retry
        // briefly before falling back to defaults, so one unlucky read
        // never nukes the user's accounts.
        let mut file = None;
        for _ in 0..4 {
            let text = std::fs::read_to_string(&path).ok();
            match text {
                Some(text) => {
                    // Windows editors often emit UTF-8 with BOM; strip it.
                    let clean = text.trim_start_matches('\u{feff}');
                    match serde_json::from_str::<Value>(clean) {
                        Ok(value) => {
                            file = serde_json::from_value::<ConfigFile>(value).ok();
                            break;
                        }
                        Err(_) => std::thread::sleep(std::time::Duration::from_millis(60)),
                    }
                }
                None => break, // no file yet: genuine first run
            }
        }
        let file = file.unwrap_or_default();
        let clean = ConfigFile {
            api: file.api.sanitized(),
            accounts: sanitize_accounts(file.accounts),
            prefs: file.prefs.sanitized(),
        };
        let state = Self {
            data_dir: data_dir.to_path_buf(),
            inner: std::sync::Arc::new(RwLock::new(clean)),
        };
        // Make sure the file exists with all currently valid fields.
        let _ = state.persist_now();
        state
    }

    pub fn data_dir(&self) -> PathBuf {
        self.data_dir.clone()
    }

    pub fn current(&self) -> ApiConfig {
        self.read(|file| file.api.clone())
            .unwrap_or_else(|_| ApiConfig::default())
    }

    pub fn current_accounts(&self) -> Vec<AccountConfig> {
        self.read(|file| file.accounts.clone()).unwrap_or_default()
    }

    pub fn current_prefs(&self) -> AppPrefs {
        self.read(|file| file.prefs.clone())
            .unwrap_or_else(|_| AppPrefs::default())
    }

    /// Replace the app preferences (sanitized) and persist to disk.
    pub fn update_prefs(&self, prefs: AppPrefs) -> Result<(), String> {
        self.write(|file| {
            file.prefs = prefs.sanitized();
            self.persist_locked(file)
        })
    }

    pub fn account(&self, id: &str) -> Option<AccountConfig> {
        self.read(|file| {
            file.accounts
                .iter()
                .find(|account| account.id == id)
                .cloned()
        })
        .ok()
        .flatten()
    }

    /// First enabled account, or the first account when none is enabled.
    pub fn primary_account(&self) -> Option<AccountConfig> {
        let accounts = self.current_accounts();
        accounts
            .iter()
            .find(|account| account.enabled)
            .or_else(|| accounts.first())
            .cloned()
    }

    /// Replace the whole API config after sanitizing it and persist to disk.
    pub fn update_api(&self, change: ApiConfig) -> Result<(), String> {
        self.write(|file| {
            let mut clean = change.sanitized();
            // Keep the existing key when the caller passes an empty one.
            if clean.api_key.is_empty() {
                clean.api_key = file.api.api_key.clone();
            }
            file.api = clean;
            self.persist_locked(file)
        })
    }

    /// Replace the account registry (sanitized) and persist to disk.
    pub fn update_accounts(&self, accounts: Vec<AccountConfig>) -> Result<(), String> {
        self.write(|file| {
            file.accounts = sanitize_accounts(accounts);
            self.persist_locked(file)
        })
    }

    pub fn api_key(&self) -> String {
        self.current().api_key
    }

    fn persist_now(&self) -> Result<(), String> {
        self.write(|file| self.persist_locked(file))
    }

    fn read<R>(&self, extract: impl Fn(&ConfigFile) -> R) -> Result<R, String>
    where
        R: Default,
    {
        let file = self.inner.read().map_err(|_| "settings needed".to_string())?;
        Ok(extract(&file))
    }

    fn write<R>(
        &self,
        apply: impl FnOnce(&mut ConfigFile) -> Result<R, String>,
    ) -> Result<R, String> {
        let mut file = self
            .inner
            .write()
            .map_err(|_| "settings lock poisoned".to_string())?;
        apply(&mut file)
    }

    fn persist_locked(&self, file: &ConfigFile) -> Result<(), String> {
        if !self.data_dir.exists() {
            let _ = std::fs::create_dir_all(&self.data_dir);
        }
        let path = self.data_dir.join("config.json");
        serde_json::to_writer_pretty(
            std::io::BufWriter::new(
                std::fs::File::create(path).map_err(|err| err.to_string())?,
            ),
            file,
        )
        .map_err(|err| err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_has_disabled_api_and_no_accounts() {
        // Virgin installs: API closed, ZERO accounts (the shell auto-creates
        // a fresh "Account 1" on the next open (never a legacy "default").
        let config = ConfigFile::default();
        assert!(!config.api.enabled);
        assert_eq!(config.api.port, DEFAULT_API_PORT);
        assert_eq!(config.accounts.len(), 0);
    }

    #[test]
    fn sanitize_rejects_non_url_webhook() {
        let bad = ApiConfig {
            webhook_url: "javascript:alert(1)".into(),
            ..ApiConfig::default()
        };
        let clean = bad.sanitized();
        assert_eq!(clean.webhook_url, String::new());
    }

    #[test]
    fn sanitize_accounts_dedupes_and_repairs() {
        let raw = vec![
            AccountConfig {
                id: "personal!".into(),
                label: String::new(),
                enabled: true,
            },
            AccountConfig {
                id: "work".into(),
                label: "Work".into(),
                enabled: false,
            },
            AccountConfig {
                id: String::new(),
                label: "dropped".into(),
                enabled: true,
            },
        ];
        let clean = sanitize_accounts(raw);
        assert_eq!(
            clean
                .iter()
                .map(|account| account.id.clone())
                .collect::<Vec<_>>(),
            vec!["personal".to_string(), "work".to_string()]
        );
        assert_eq!(clean[0].label, "personal");
        assert_eq!(clean[1].label, "Work");
    }

    #[test]
    fn sanitize_accounts_allows_empty_registry() {
        // The user may remove every account; the shell auto-creates a fresh
        // one on the next open instead of resurrecting a zombie default.
        let clean = sanitize_accounts(vec![]);
        assert_eq!(clean.len(), 0);
    }
}
