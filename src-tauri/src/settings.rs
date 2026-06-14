//! Persistent app settings: saved relays, this device's name, and which roles
//! (host / client) the user enabled. Stored as JSON in the app data dir so the
//! onboarding wizard only runs once.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// A saved relay endpoint the user can connect to.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relay {
    pub id: String,
    pub name: String,
    pub http_url: String,
    pub ws_url: String,
}

/// A host discovered on the local network and remembered by the user.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedLanDevice {
    pub id: String,
    pub name: String,
    pub address: String,
    pub port: u16,
    /// Advertised host identity (base64), if known — lets us pin the host.
    #[serde(default)]
    pub public_key: Option<String>,
}

/// Everything the app remembers between launches.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AppSettings {
    /// Onboarding finished — skip the wizard on next launch.
    #[serde(default)]
    pub setup_complete: bool,
    /// Friendly name for this machine (shown to support clients).
    #[serde(default)]
    pub device_name: String,
    /// Enabled roles: "host", "client", or both.
    #[serde(default)]
    pub roles: Vec<String>,
    /// Saved relays.
    #[serde(default)]
    pub relays: Vec<Relay>,
    /// Which saved relay is currently selected.
    #[serde(default)]
    pub active_relay_id: Option<String>,
    /// Hosts found on the local network and remembered by the user.
    #[serde(default)]
    pub lan_devices: Vec<SavedLanDevice>,
}

impl AppSettings {
    /// File the settings live in, inside `dir`.
    pub fn path(dir: &Path) -> PathBuf {
        dir.join("settings.json")
    }

    /// Load settings from `dir`, returning defaults if the file is absent.
    pub fn load(dir: &Path) -> Self {
        let path = Self::path(dir);
        match std::fs::read_to_string(&path) {
            Ok(body) => serde_json::from_str(&body).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    /// Persist settings as pretty JSON, creating the dir if needed.
    pub fn save(&self, dir: &Path) -> Result<(), String> {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        let body = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        std::fs::write(Self::path(dir), body).map_err(|e| e.to_string())
    }

    /// The currently selected relay, if any.
    pub fn active_relay(&self) -> Option<&Relay> {
        let id = self.active_relay_id.as_ref()?;
        self.relays.iter().find(|r| &r.id == id)
    }
}
