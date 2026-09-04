use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    pub hotkey: String,
    pub assume_projectile_speed: bool,
    pub dump_debug: bool,
    pub poe_window_title: String,
    pub scan_clipboard_first: bool,
    pub always_on_top: bool,
    pub hide_on_escape: bool,
    pub trade_league: String,
    /// One `mercenary` stat group per skill. Anonymous searches reject more
    /// than one group as too complex; a pathofexile.com login lifts the cap.
    pub trade_every_skill: bool,
    /// Check GitHub Releases for a newer version at startup and every 6 hours.
    pub check_updates: bool,
    /// After a check finds a newer release, download and verify it without
    /// waiting for a click; the user still chooses when to restart.
    pub install_updates_automatically: bool,
    /// One of `"standard"`, `"dark"`, `"light"`, or `"system"` (follows the
    /// Windows app mode). Unknown values fall back to `"standard"`.
    pub theme: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            hotkey: "Ctrl+Shift+M".into(),
            assume_projectile_speed: false,
            dump_debug: false,
            poe_window_title: "Path of Exile".into(),
            scan_clipboard_first: true,
            always_on_top: true,
            hide_on_escape: true,
            trade_league: "Allflame".into(),
            trade_every_skill: false,
            check_updates: true,
            install_updates_automatically: true,
            theme: "standard".into(),
        }
    }
}

impl AppConfig {
    pub fn dir() -> PathBuf {
        directories::ProjectDirs::from("com", "PoEMercPricer", "PoEMercPricer")
            .map(|d| d.config_dir().to_path_buf())
            .unwrap_or_else(|| PathBuf::from(".").join("config"))
    }

    pub fn path() -> PathBuf {
        Self::dir().join("config.json")
    }

    /// Load the config, writing defaults only when the file does not exist.
    /// An unreadable or unparsable file is left untouched; the returned
    /// message explains why defaults are in use.
    pub fn load() -> (Self, Option<String>) {
        Self::load_from(&Self::path())
    }

    pub fn load_from(path: &Path) -> (Self, Option<String>) {
        match fs::read_to_string(path) {
            Ok(text) => match serde_json::from_str(&text) {
                Ok(cfg) => (cfg, None),
                Err(e) => (
                    Self::default(),
                    Some(format!(
                        "{} is not valid; using defaults, file left untouched: {e}",
                        path.display()
                    )),
                ),
            },
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                let cfg = Self::default();
                let err = cfg
                    .save_to(path)
                    .err()
                    .map(|e| format!("Could not write {}: {e:#}", path.display()));
                (cfg, err)
            }
            Err(e) => (
                Self::default(),
                Some(format!("Could not read {}: {e}", path.display())),
            ),
        }
    }

    pub fn save(&self) -> Result<()> {
        self.save_to(&Self::path())
    }

    pub fn save_to(&self, path: &Path) -> Result<()> {
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }
        fs::write(path, serde_json::to_string_pretty(self)?)?;
        Ok(())
    }
}
