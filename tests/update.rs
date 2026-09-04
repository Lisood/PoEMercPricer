//! Updater parsing and hashing. Offline: nothing here touches the network.

use poemercpricer::update::{is_newer, parse_release, ASSET};

const FIXTURE: &[u8] = include_bytes!("fixtures/release-latest.json");

/// The fixture with the one line holding `needle` replaced by `replacement`.
fn edited(needle: &str, replacement: &str) -> Vec<u8> {
    let text = std::str::from_utf8(FIXTURE).unwrap();
    let line = text
        .lines()
        .find(|l| l.contains(needle))
        .unwrap_or_else(|| panic!("fixture has no line containing {needle}"));
    let json = text.replace(line, replacement);
    assert_ne!(json, text);
    json.into_bytes()
}

#[test]
fn parse_release_reads_the_windows_asset() {
    let release = parse_release(FIXTURE, "0.2.0").unwrap().expect("newer");
    assert_eq!(release.version, semver::Version::new(0, 3, 0));
    assert_eq!(
        release.page,
        "https://github.com/Lisood/PoEMercPricer/releases/tag/v0.3.0"
    );
    assert_eq!(
        release.download,
        format!("https://github.com/Lisood/PoEMercPricer/releases/download/v0.3.0/{ASSET}")
    );
    assert_eq!(release.size, 12_358_656);
    assert_eq!(release.sha256.len(), 64);
    assert!(!release.sha256.starts_with("sha256:"));
    assert!(release.sha256.chars().all(|c| c.is_ascii_hexdigit()));
}

#[test]
fn parse_release_is_none_when_not_newer() {
    assert!(parse_release(FIXTURE, "0.3.0").unwrap().is_none());
    assert!(parse_release(FIXTURE, "0.4.0").unwrap().is_none());
}

#[test]
fn is_newer_is_strict_and_ignores_prereleases() {
    assert!(is_newer("v0.3.0", "0.2.0").unwrap());
    assert!(!is_newer("v0.2.0", "0.2.0").unwrap());
    assert!(!is_newer("v0.2.0-rc1", "0.1.0").unwrap());
    assert!(is_newer("0.3.0", "0.2.0").unwrap());
    assert!(is_newer("v0.3.0", "v0.2.0").unwrap());
    assert!(is_newer("v1.0.0", "0.9.9").unwrap());
    assert!(is_newer("v0.3.0", "3.29").is_err());
    assert!(is_newer("3.29", "0.2.0").is_err());
}

#[test]
fn a_release_without_the_exe_is_an_error() {
    let json = edited(
        "\"name\": \"poemercpricer-windows-x64.exe\"",
        "      \"name\": \"poemercpricer-linux-x64\",",
    );
    let message = parse_release(&json, "0.2.0").unwrap_err().to_string();
    assert!(
        message.contains("Windows download"),
        "unexpected message: {message}"
    );
}

#[test]
fn a_release_without_a_digest_is_an_error() {
    let json = edited("\"digest\"", "");
    let message = parse_release(&json, "0.2.0").unwrap_err().to_string();
    assert!(
        message.contains("checksum"),
        "unexpected message: {message}"
    );
}

#[test]
fn a_release_with_an_implausibly_large_size_is_an_error() {
    let json = edited("\"size\": 12358656", "      \"size\": 999999999999,");
    let message = parse_release(&json, "0.2.0").unwrap_err().to_string();
    assert!(
        message.contains("implausibly large"),
        "unexpected message: {message}"
    );
}

#[cfg(windows)]
#[test]
fn sha256_matches_the_fips_180_vector() {
    assert_eq!(
        poemercpricer::update::sha256_hex(b"abc").unwrap(),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
}

#[test]
fn parse_release_records_the_gzip_sibling_but_sizes_the_exe() {
    use poemercpricer::update::COMPRESSED_ASSET;
    let release = parse_release(FIXTURE, "0.2.0").unwrap().expect("newer");
    assert_eq!(
        release.compressed.as_deref(),
        Some(
            format!(
                "https://github.com/Lisood/PoEMercPricer/releases/download/v0.3.0/{COMPRESSED_ASSET}"
            )
            .as_str()
        )
    );
    // Size and digest always describe the exe asset, never the gzip copy.
    assert_eq!(release.size, 12_358_656);
    assert!(release.sha256.starts_with("9f2c4b1e"));
}

#[test]
fn a_release_without_the_gzip_sibling_still_parses() {
    let json = edited(
        "\"name\": \"poemercpricer-windows-x64.exe.gz\"",
        "      \"name\": \"poemercpricer-windows-x64.exe.zst\",",
    );
    let release = parse_release(&json, "0.2.0").unwrap().expect("newer");
    assert!(release.compressed.is_none());
    assert_eq!(release.size, 12_358_656);
}

#[test]
fn gunzip_roundtrips_and_is_bounded() {
    use poemercpricer::update::gunzip;
    use std::io::Write;
    let payload: Vec<u8> = (0..100_000u32).map(|i| (i % 251) as u8).collect();
    let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::best());
    encoder.write_all(&payload).unwrap();
    let gz = encoder.finish().unwrap();
    assert!(gz.len() < payload.len());
    assert_eq!(gunzip(&gz, payload.len() as u64).unwrap(), payload);
    // A stream longer than the release claims stops at size + 1 instead of
    // inflating without bound; the size check then rejects it.
    assert_eq!(gunzip(&gz, 10).unwrap().len(), 11);
    assert!(gunzip(b"not gzip at all", 10).is_err());
}

#[cfg(windows)]
#[test]
fn inflate_verified_accepts_only_the_exact_exe_bytes() {
    use poemercpricer::update::{inflate_verified, sha256_hex, verify, Release};
    use std::io::Write;
    let payload: Vec<u8> = (0..50_000u32)
        .map(|i| (i.wrapping_mul(2654435761) >> 13) as u8)
        .collect();
    let gzip = |bytes: &[u8]| {
        let mut e = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        e.write_all(bytes).unwrap();
        e.finish().unwrap()
    };
    let release = Release {
        version: semver::Version::new(0, 3, 0),
        page: String::new(),
        download: String::new(),
        compressed: Some(String::new()),
        size: payload.len() as u64,
        sha256: sha256_hex(&payload).unwrap(),
    };
    assert_eq!(
        inflate_verified(&gzip(&payload), &release).unwrap(),
        payload
    );
    assert!(verify(&payload, &release).is_ok());

    let mut tampered = payload.clone();
    tampered[1234] ^= 0x01;
    let message = inflate_verified(&gzip(&tampered), &release)
        .unwrap_err()
        .to_string();
    assert!(
        message.contains("checksum"),
        "unexpected message: {message}"
    );

    let message = inflate_verified(&gzip(&payload[..payload.len() - 1]), &release)
        .unwrap_err()
        .to_string();
    assert!(
        message.contains("checksum"),
        "unexpected message: {message}"
    );

    let mut longer = payload.clone();
    longer.push(0);
    assert!(inflate_verified(&gzip(&longer), &release).is_err());
    assert!(inflate_verified(b"garbage", &release).is_err());
}

/// Regression for a STATUS_ACCESS_VIOLATION seen in CI: tying the process
/// MTA's lifetime to whichever thread initialized it means that once an updater
/// worker thread exits, the next WinRT call on a fresh thread faults.
/// `CoIncrementMTAUsage` in `winrt()` pins the MTA for the process; this test
/// fails without it under `--test-threads=1`.
#[cfg(windows)]
#[test]
fn winrt_survives_a_previous_updater_thread_exiting() {
    use poemercpricer::update::fetch;
    use std::time::Duration;

    // 127.0.0.1 needs no DNS lookup and nothing listens on port 9 (discard),
    // so the connection is refused almost immediately.
    let attempt = || {
        std::thread::spawn(|| fetch("http://127.0.0.1:9/", Duration::from_secs(5)))
            .join()
            .unwrap()
    };
    assert!(attempt().is_err(), "first thread's fetch should fail");
    assert!(attempt().is_err(), "second thread's fetch should fail too");
}
