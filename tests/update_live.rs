//! Live updater tests. These reach GitHub, so every one is `#[ignore]`.
//! Run with `cargo test --test update_live -- --ignored`.
//!
//! `install()` is never called here: `self_replace` would overwrite the running
//! test harness.

use std::time::Duration;

use poemercpricer::update::{check, fetch, sha256_hex};

const JSON: Duration = Duration::from_secs(15);
const DOWNLOAD: Duration = Duration::from_secs(120);

/// network; run with `cargo test --test update_live -- --ignored`
#[test]
#[ignore]
fn live_check_against_real_repo() {
    match check() {
        Ok(Some(release)) => println!("check(): update available, {}", release.version),
        Ok(None) => println!("check(): already current"),
        Err(error) => {
            let text = format!("{error:#}");
            println!("check(): {text}");
            assert!(
                text.contains("no release published yet"),
                "unexpected failure: {text}"
            );
        }
    }
}

/// network; run with `cargo test --test update_live -- --ignored`
#[test]
#[ignore]
fn live_download_matches_github_digest() {
    let body = fetch("https://api.github.com/repos/cli/cli/releases/latest", JSON)
        .expect("cli/cli releases/latest");
    let release: serde_json::Value = serde_json::from_slice(&body).expect("release json");

    let asset = release["assets"]
        .as_array()
        .expect("assets array")
        .iter()
        .filter(|a| a["digest"].is_string())
        .min_by_key(|a| a["size"].as_u64().unwrap_or(u64::MAX))
        .expect("an asset with a digest");
    let url = asset["browser_download_url"].as_str().unwrap();
    let size = asset["size"].as_u64().unwrap();
    let digest = asset["digest"].as_str().unwrap();
    println!("downloading {url} ({size} bytes, {digest})");

    let bytes = fetch(url, DOWNLOAD).expect("asset download");
    assert_eq!(bytes.len() as u64, size, "downloaded size");
    assert_eq!(
        format!("sha256:{}", sha256_hex(&bytes).unwrap()),
        digest,
        "sha256 of the download"
    );
}

/// The published `.exe.gz` (made by .NET GZipStream in release.yml) must
/// inflate with flate2 to exactly the bytes GitHub digested for the exe; that
/// is the path every installed copy takes first.
/// network; run with `cargo test --test update_live -- --ignored`
#[test]
#[ignore]
fn live_gzip_asset_inflates_to_the_exe_digest() {
    use poemercpricer::update::{inflate_verified, parse_release, REPO};
    let body = fetch(
        &format!("https://api.github.com/repos/{REPO}/releases/latest"),
        JSON,
    )
    .expect("releases/latest");
    // "0.0.0" makes every published tag count as newer.
    let release = parse_release(&body, "0.0.0")
        .expect("release json")
        .expect("a published release");
    let gz_url = release.compressed.as_deref().expect("the .exe.gz sibling");
    println!("downloading {gz_url} for {}", release.version);
    let gz = fetch(gz_url, DOWNLOAD).expect("gzip download");
    assert!((gz.len() as u64) < release.size, "gzip is not smaller");
    let bytes = inflate_verified(&gz, &release).expect("inflates to the digested exe");
    assert_eq!(bytes.len() as u64, release.size);
}

/// network; run with `cargo test --test update_live -- --ignored`
#[test]
#[ignore]
fn live_404_is_mapped() {
    // The status-code mapping lives in `fetch`, so assert on its message. A
    // repo that does not exist answers 404 the same way a private one does.
    let error = fetch(
        "https://api.github.com/repos/Lisood/PoEMercPricer-does-not-exist/releases/latest",
        JSON,
    )
    .expect_err("a private or missing repo answers 404");
    let text = format!("{error:#}");
    println!("fetch(): {text}");
    assert!(
        text.contains("no release published yet"),
        "unexpected failure: {text}"
    );
}
