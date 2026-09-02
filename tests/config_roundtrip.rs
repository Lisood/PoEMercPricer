use poemercpricer::config::AppConfig;
use poemercpricer::models::kineticist_jackpot_fixture;
use poemercpricer::parse::looks_like_warrant;

#[test]
fn default_config_serializes() {
    let cfg = AppConfig::default();
    assert_eq!(cfg.hotkey, "Ctrl+Shift+M");
    let json = serde_json::to_string_pretty(&cfg).unwrap();
    let back: AppConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(back.hotkey, cfg.hotkey);
    assert!(back.scan_clipboard_first);
    assert!(back.always_on_top);
    assert_eq!(back.trade_league, "Allflame");
}

#[test]
fn older_config_without_trade_league_uses_the_current_default() {
    let cfg: AppConfig = serde_json::from_str(r#"{"hotkey":"Ctrl+M"}"#).unwrap();
    assert_eq!(cfg.hotkey, "Ctrl+M");
    assert_eq!(cfg.trade_league, "Allflame");
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
