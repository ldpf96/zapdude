//! Persistent settings for ZapZap (Rust).
//!
//! Mirrors the `api/*` keys of the original Python fork plus the
//! multi-account registry. Everything lives in one JSON file inside the OS
//! application-data directory. Only keys the automation layer reads are
//! persisted; labels are kept separately from ids.

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
    if result.is_empty() {
        result.push(AccountConfig::default());
    }
    result
}

/// Whole configuration file shape.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ConfigFile {
    pub api: ApiConfig,
    pub accounts: Vec<AccountConfig>,
}

impl Default for ConfigFile {
    fn default() -> Self {
        Self {
            api: ApiConfig::default(),
            accounts: vec![AccountConfig::default()],
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
        let file = std::fs::read_to_string(&path)
            .ok()
            .map(|text| {
                // Windows editors often emit UTF-8 with BOM; strip it.
                let clean = text.trim_start_matches('\u{feff}');
                serde_json::from_str::<Value>(clean)
            })
            .and_then(|parsed| parsed.ok())
            .and_then(|value| serde_json::from_value::<ConfigFile>(value).ok())
            .unwrap_or_default();
        let clean = ConfigFile {
            api: file.api.sanitized(),
            accounts: sanitize_accounts(file.accounts),
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
            // TEMP DEBUG: catch unexpected account-list rewrites (a stale
            // empty list would wipe user accounts). Remove after diagnosis.
            if file.accounts.len() <= 1 {
                eprintln!(
                    "zapdude: DEBUG update_accounts -> {} account(s): {:?}",
                    file.accounts.len(),
                    file.accounts.iter().map(|a| a.id.clone()).collect::<Vec<_>>()
                );
                eprintln!("{}", std::backtrace::Backtrace::force_capture());
            }
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
    fn default_config_has_disabled_api_and_one_account() {
        let config = ConfigFile::default();
        assert!(!config.api.enabled);
        assert_eq!(config.api.port, DEFAULT_API_PORT);
        assert_eq!(config.accounts.len(), 1);
        assert_eq!(config.accounts[0].id, "default");
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
    fn sanitize_accounts_falls_back_to_default() {
        let clean = sanitize_accounts(vec![]);
        assert_eq!(clean.len(), 1);
        assert_eq!(clean[0].id, "default");
    }
}
