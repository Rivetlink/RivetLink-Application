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
    /// True only for an mDNS advertisement from the boot-time physical
    /// GDM/GNOME console broker. Older remembered LAN hosts remain regular
    /// hosts by default.
    #[serde(default)]
    pub physical_console: bool,
}

/// A client this host trusts: its identity public key plus a name the owner
/// gave it. A trusted client may connect without the session code.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustedKey {
    pub id: String,
    pub name: String,
    /// The client's Ed25519 identity public key (base64).
    pub public_key: String,
}

/// A relay host identity accepted during an explicit first connection.  The
/// relay is routing infrastructure, not a host identity authority after this
/// point: a changed key must be deliberately removed and paired again.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustedRelayHost {
    pub relay_id: String,
    pub device_id: String,
    pub public_key: String,
}

/// Everything the app remembers between launches.
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    /// Pinned identity keys for relay hosts, scoped to their relay endpoint.
    #[serde(default)]
    pub trusted_relay_hosts: Vec<TrustedRelayHost>,
    /// Clients allowed to connect to this host without the session code.
    #[serde(default)]
    pub trusted_keys: Vec<TrustedKey>,
    /// Remote wheel/trackpad sensitivity: "slow", "normal", or "fast".
    #[serde(default = "default_scroll_speed")]
    pub scroll_speed: String,
}

fn default_scroll_speed() -> String {
    "slow".to_string()
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            setup_complete: false,
            device_name: String::new(),
            roles: Vec::new(),
            relays: Vec::new(),
            active_relay_id: None,
            lan_devices: Vec::new(),
            trusted_relay_hosts: Vec::new(),
            trusted_keys: Vec::new(),
            scroll_speed: default_scroll_speed(),
        }
    }
}

impl AppSettings {
    /// File the settings live in, inside `dir`.
    pub fn path(dir: &Path) -> PathBuf {
        dir.join("settings.json")
    }

    /// Load settings from `dir`, returning defaults if the file is absent.
    pub fn load(dir: &Path) -> Self {
        let path = Self::path(dir);
        let mut settings = match std::fs::read_to_string(&path) {
            Ok(body) => serde_json::from_str(&body).unwrap_or_default(),
            Err(_) => Self::default(),
        };
        if !matches!(settings.scroll_speed.as_str(), "slow" | "normal" | "fast") {
            settings.scroll_speed = default_scroll_speed();
        }
        settings
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_scroll_speed_defaults_to_slow() {
        let settings: AppSettings = serde_json::from_str("{}").unwrap();
        assert_eq!(settings.scroll_speed, "slow");
        assert_eq!(AppSettings::default().scroll_speed, "slow");
    }

    #[test]
    fn scroll_speed_round_trips() {
        let settings = AppSettings {
            scroll_speed: "normal".to_string(),
            ..AppSettings::default()
        };
        let encoded = serde_json::to_string(&settings).unwrap();
        let decoded: AppSettings = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded.scroll_speed, "normal");
    }
}
