use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

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
            trade_league: "Covens".into(),
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
            Ok(text) => match serde_json::from_str::<Self>(&text) {
                Ok(mut cfg) => match retired_league_successor(&cfg.trade_league) {
                    Some(league) => {
                        cfg.trade_league = league;
                        let err = cfg
                            .save_to(path)
                            .err()
                            .map(|e| format!("Could not write {}: {e:#}", path.display()));
                        (cfg, err)
                    }
                    None => (cfg, None),
                },
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
        // Write a sibling and rename it over the target, so a crash mid-write
        // never leaves a truncated config behind. Each save gets its own
        // sibling, so concurrent saves never write into each other's file.
        static SAVES: AtomicU64 = AtomicU64::new(0);
        let json = serde_json::to_string_pretty(self)?;
        let (tmp, mut file) = create_temp_sibling(path, &SAVES)?;
        let written = file.write_all(json.as_bytes()).and_then(|()| {
            drop(file);
            fs::rename(&tmp, path)
        });
        if written.is_err() {
            let _ = fs::remove_file(&tmp);
        }
        Ok(written?)
    }
}

/// Exclusively create `<path>.<pid>.<n>.tmp`. A crash can leave a stale one
/// behind for a later process that reuses the pid, so taken names are skipped.
fn create_temp_sibling(path: &Path, saves: &AtomicU64) -> std::io::Result<(PathBuf, fs::File)> {
    let mut attempts = 0;
    loop {
        let mut tmp = path.as_os_str().to_owned();
        tmp.push(format!(
            ".{}.{}.tmp",
            std::process::id(),
            saves.fetch_add(1, Ordering::Relaxed)
        ));
        let tmp = PathBuf::from(tmp);
        match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&tmp)
        {
            Ok(file) => return Ok((tmp, file)),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists && attempts < 15 => {
                attempts += 1;
            }
            Err(e) => return Err(e),
        }
    }
}

/// The 3.29 Allflame leagues ended with 3.30. Only the two public leagues
/// v0.2.0 offered move to their Covens equivalents; any other name, such as a
/// private league, is the user's own and is kept verbatim.
fn retired_league_successor(league: &str) -> Option<String> {
    if league.eq_ignore_ascii_case("Allflame") {
        Some("Covens".into())
    } else if league.eq_ignore_ascii_case("Hardcore Allflame") {
        Some("Hardcore Covens".into())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stale_temp_files_from_a_reused_pid_are_skipped() {
        let dir = std::env::temp_dir().join(format!("pmp-config-stale-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.json");
        let stale = |n: u64| dir.join(format!("config.json.{}.{n}.tmp", std::process::id()));
        for n in 0..3 {
            fs::write(stale(n), "stale").unwrap();
        }

        let (tmp, file) = create_temp_sibling(&path, &AtomicU64::new(0)).unwrap();
        drop(file);
        assert_eq!(tmp, stale(3));
        assert_eq!(fs::read_to_string(stale(0)).unwrap(), "stale");

        // Bounded: sixteen taken names in a row give up instead of spinning.
        for n in 4..20 {
            fs::write(stale(n), "stale").unwrap();
        }
        let error = create_temp_sibling(&path, &AtomicU64::new(4)).unwrap_err();
        assert_eq!(error.kind(), std::io::ErrorKind::AlreadyExists);
        fs::remove_dir_all(&dir).unwrap();
    }
}
