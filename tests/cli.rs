use std::io::Write;
use std::process::{Command, Stdio};

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_poemercpricer"))
}

#[test]
fn cli_score_kineticist_jackpot_84() {
    let output = bin()
        .args([
            "score",
            "--family",
            "kineticist",
            "--skill",
            "Kinetic Blast of Clustering:returnT3,gmpT3,chainT2,edwaT3",
            "--skill",
            "Greater Kinetic Blast",
        ])
        .output()
        .expect("run score");
    assert!(
        output.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Score 84.0  [jackpot]  jackpot=true"),
        "{stdout}"
    );
    assert!(
        stdout.contains("  Kinetic Blast of Clustering: Return T3, Greater Multiple Projectiles T3, Chain T2, Elemental Damage with Attacks T3"),
        "{stdout}"
    );
}

#[test]
fn cli_rejects_unknown_flags_and_scan_without_a_path() {
    for args in [&["--bogus"][..], &["--scan"][..]] {
        let output = bin().args(args).output().expect("run");
        assert!(!output.status.success(), "{args:?} should fail");
        assert!(
            String::from_utf8_lossy(&output.stderr).contains("--help")
                || String::from_utf8_lossy(&output.stderr).contains("--scan needs"),
            "{args:?}"
        );
    }
}

#[test]
fn cli_clipboard_warrant() {
    let warrant = r#"
Item Class: Map Fragments
Rarity: Normal
Mercenary Warrant
--------
Build: Infamous Kineticist
Mercenary Level: 83
--------
Kinetic Blast of Clustering
Return (Tier: 3)
Greater Multiple Projectiles (Tier: 3)
Chain (Tier: 2)
Elemental Damage with Attacks (Tier: 3)
--------
Greater Kinetic Blast
--------
Haste
"#;
    let mut child = bin()
        .arg("clipboard")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn clipboard");
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(warrant.as_bytes())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Infamous Kineticist  Lvl 83"), "{stdout}");
    assert!(
        stdout.contains("Score 88.0  [jackpot]  jackpot=true"),
        "{stdout}"
    );
    // The market line follows the score line whether or not the snapshot
    // covers the build; its numbers come from the collector and are not asserted.
    assert!(
        stdout.contains("jackpot=true\nMarket: "),
        "market line missing after the score line: {stdout}"
    );
}

#[test]
fn cli_version_prints_cargo_version() {
    let output = bin().arg("--version").output().expect("version");
    assert!(
        output.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains(env!("CARGO_PKG_VERSION")), "{stdout}");
}

#[test]
fn cli_help_exits_zero() {
    let output = bin().arg("--help").output().expect("help");
    assert!(output.status.success());
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stderr),
        String::from_utf8_lossy(&output.stdout)
    );
    assert!(
        text.contains("Ctrl+Shift+M") || text.contains("overlay"),
        "{text}"
    );
}
