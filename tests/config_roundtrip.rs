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
    assert_eq!(back.trade_league, "Allflame");
    assert_eq!(back.theme, "standard");
}

#[test]
fn older_config_without_trade_league_uses_the_current_default() {
    let cfg: AppConfig = serde_json::from_str(r#"{"hotkey":"Ctrl+M"}"#).unwrap();
    assert_eq!(cfg.hotkey, "Ctrl+M");
    assert_eq!(cfg.trade_league, "Allflame");
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
