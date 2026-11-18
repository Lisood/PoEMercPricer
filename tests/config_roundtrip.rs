use poemercpricer::config::AppConfig;
use poemercpricer::models::kineticist_jackpot_fixture;
use poemercpricer::parse::looks_like_warrant;

#[test]
fn default_config_serializes() {
    let cfg = AppConfig::default();
    assert_eq!(cfg.hotkey, "Ctrl+Shift+M");
    assert_eq!(cfg.theme, "standard");
    let json = serde_json::to_string_pretty(&cfg).unwrap();
    let back: AppConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(back.hotkey, cfg.hotkey);
    assert!(back.scan_clipboard_first);
    assert!(back.always_on_top);
    assert_eq!(back.trade_league, "Covens");
    assert_eq!(back.theme, "standard");
}

#[test]
fn older_config_without_trade_league_uses_the_current_default() {
    let cfg: AppConfig = serde_json::from_str(r#"{"hotkey":"Ctrl+M"}"#).unwrap();
    assert_eq!(cfg.hotkey, "Ctrl+M");
    assert_eq!(cfg.trade_league, "Covens");
    assert_eq!(cfg.theme, "standard");
}

#[test]
fn theme_round_trips_and_unknown_values_are_kept_verbatim() {
    let cfg: AppConfig = serde_json::from_str(r#"{"hotkey":"Ctrl+M","theme":"light"}"#).unwrap();
    assert_eq!(cfg.theme, "light");
    let json = serde_json::to_string_pretty(&cfg).unwrap();
    let back: AppConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(back.theme, "light");
}

#[test]
fn older_config_without_update_keys_defaults_both_checks_on() {
    let cfg: AppConfig = serde_json::from_str(r#"{"hotkey":"Ctrl+M"}"#).unwrap();
    assert_eq!(cfg.hotkey, "Ctrl+M");
    assert!(cfg.check_updates);
    assert!(cfg.install_updates_automatically);
}

#[test]
fn corrupt_config_is_left_untouched_and_reported() {
    let dir = std::env::temp_dir().join(format!("pmp-config-{}", std::process::id()));
    let path = dir.join("config.json");
    std::fs::create_dir_all(&dir).unwrap();
    let corrupt = b"{\"hotkey\": \"Ctrl+M\",}";
    std::fs::write(&path, corrupt).unwrap();

    let (cfg, error) = AppConfig::load_from(&path);

    assert_eq!(std::fs::read(&path).unwrap(), corrupt);
    assert_eq!(cfg.hotkey, "Ctrl+Shift+M");
    assert!(
        error.as_deref().is_some_and(|e| e.contains("untouched")),
        "{error:?}"
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn missing_config_is_created_with_defaults() {
    let dir = std::env::temp_dir().join(format!("pmp-config-new-{}", std::process::id()));
    let path = dir.join("nested").join("config.json");
    let _ = std::fs::remove_dir_all(&dir);

    let (cfg, error) = AppConfig::load_from(&path);

    assert_eq!(error, None);
    assert_eq!(cfg.hotkey, "Ctrl+Shift+M");
    let back: AppConfig = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(back.hotkey, cfg.hotkey);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn looks_like_warrant_detects_game_copy() {
    assert!(looks_like_warrant(
        "Item Class: Map Fragments\nMercenary Warrant\n--------\nBuild: Manyshot"
    ));
    assert!(looks_like_warrant(
        "Build: Infamous Kineticist\nMercenary Level: 83"
    ));
    assert!(!looks_like_warrant("hello world"));
    assert!(!looks_like_warrant(""));
}

#[test]
fn kineticist_fixture_is_jackpot() {
    let (merc, result) = kineticist_jackpot_fixture();
    assert_eq!(merc.family, "kineticist");
    assert!(result.jackpot);
    assert!(result.score >= 84.0);
}

#[test]
fn retired_allflame_league_is_migrated_to_covens_and_saved() {
    let dir = std::env::temp_dir().join(format!("pmp-config-league-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("config.json");
    for (saved, expected) in [
        ("Allflame", "Covens"),
        ("Hardcore Allflame", "Hardcore Covens"),
        ("Ruthless Allflame", "Ruthless Covens"),
        ("HC Ruthless Allflame", "HC Ruthless Covens"),
        ("hardcore ALLFLAME", "hardcore Covens"),
        ("Standard", "Standard"),
        ("My Private League (PL123)", "My Private League (PL123)"),
        ("Allflamers (PL123)", "Allflamers (PL123)"),
        ("Hardcore-Allflame (PL9)", "Hardcore-Covens (PL9)"),
    ] {
        let original = format!(r#"{{"hotkey":"Ctrl+M","trade_league":"{saved}"}}"#);
        std::fs::write(&path, &original).unwrap();

        let (cfg, error) = AppConfig::load_from(&path);

        assert_eq!(error, None);
        assert_eq!(cfg.trade_league, expected);
        assert_eq!(cfg.hotkey, "Ctrl+M");
        let on_disk = std::fs::read_to_string(&path).unwrap();
        if saved == expected {
            assert_eq!(on_disk, original, "an unaffected file is not rewritten");
        } else {
            let back: AppConfig = serde_json::from_str(&on_disk).unwrap();
            assert_eq!(back.trade_league, expected);
            assert_eq!(back.hotkey, "Ctrl+M");
        }
        assert!(
            !dir.join("config.json.tmp").exists(),
            "atomic save left its temp file"
        );
    }
    std::fs::remove_dir_all(&dir).unwrap();
}
