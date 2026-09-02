use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    pub hotkey: String,
    pub assume_projectile_speed: bool,
    pub dump_debug: bool,
    pub poe_window_title: String,
    pub overlay_seconds: f32,
    pub scan_clipboard_first: bool,
    pub always_on_top: bool,
    pub hide_on_escape: bool,
    pub trade_league: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            hotkey: "Ctrl+Shift+M".into(),
            assume_projectile_speed: false,
            dump_debug: false,
            poe_window_title: "Path of Exile".into(),
            overlay_seconds: 12.0,
            scan_clipboard_first: true,
            always_on_top: true,
            hide_on_escape: true,
            trade_league: "Allflame".into(),
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

    pub fn load() -> Self {
        let path = Self::path();
        if let Ok(text) = fs::read_to_string(&path) {
            if let Ok(cfg) = serde_json::from_str::<AppConfig>(&text) {
                return cfg;
            }
        }
        let cfg = Self::default();
        let _ = cfg.save();
        cfg
    }

    pub fn save(&self) -> Result<PathBuf> {
        let dir = Self::dir();
        fs::create_dir_all(&dir)?;
        let path = Self::path();
        fs::write(&path, serde_json::to_string_pretty(self)?)?;
        Ok(path)
    }
}
