//! End-to-end `install()`. A loopback HTTP server plays GitHub, and a copy of
//! this test binary, run as a child process, replaces itself with the served
//! bytes, so `self_replace` never touches the running harness. Offline:
//! nothing leaves 127.0.0.1. Windows-only, like `install` itself.
#![cfg(windows)]

use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::PathBuf;
use std::process::Command;

use poemercpricer::update::{install, sha256_hex, Release};

/// Child entry point. The parent runs a copy of this binary as
/// `<copy> install_child --exact` with the release in env vars and reads the
/// outcome from the file named by `PMP_INSTALL_RESULT`. Without that variable
/// (a normal `cargo test`) it does nothing.
#[test]
fn install_child() {
    let Ok(result_path) = std::env::var("PMP_INSTALL_RESULT") else {
        return;
    };
    let var = |key: &str| std::env::var(key).unwrap();
    let release = Release {
        version: semver::Version::new(9, 9, 9),
        page: String::new(),
        download: var("PMP_INSTALL_DOWNLOAD"),
        compressed: std::env::var("PMP_INSTALL_COMPRESSED").ok(),
        size: var("PMP_INSTALL_SIZE").parse().unwrap(),
        sha256: var("PMP_INSTALL_SHA256"),
    };
    let exe = std::env::current_exe().unwrap();
    let outcome = match install(&release, &exe) {
        Ok(()) => "ok".to_string(),
        Err(e) => format!("err: {e:#}"),
    };
    std::fs::write(result_path, outcome).unwrap();
}

/// Minimal HTTP/1.1 server on a random loopback port: one thread, one
/// connection at a time, `Connection: close`. Unknown paths answer 404, which
/// is what GitHub does for a missing asset.
fn serve(routes: HashMap<&'static str, Vec<u8>>) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            let mut request = Vec::new();
            let mut buf = [0u8; 1024];
            while !request.windows(4).any(|w| w == b"\r\n\r\n") {
                match stream.read(&mut buf) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => request.extend_from_slice(&buf[..n]),
                }
            }
            let head = String::from_utf8_lossy(&request);
            let path = head.split_whitespace().nth(1).unwrap_or("/");
            let response = match routes.get(path) {
                Some(body) => {
                    let mut r = format!(
                        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                        body.len()
                    )
                    .into_bytes();
                    r.extend_from_slice(body);
                    r
                }
                None => b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                    .to_vec(),
            };
            let _ = stream.write_all(&response);
        }
    });
    base
}

fn gzip(bytes: &[u8]) -> Vec<u8> {
    let mut e = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    e.write_all(bytes).unwrap();
    e.finish().unwrap()
}

/// The "new exe": arbitrary bytes are fine, the child never runs them.
fn payload() -> Vec<u8> {
    (0..200_000u32)
        .map(|i| (i.wrapping_mul(2654435761) >> 11) as u8)
        .collect()
}

struct Outcome {
    dir: PathBuf,
    result: String,
    old_sha256: String,
}

impl Outcome {
    fn exe(&self) -> Vec<u8> {
        std::fs::read(self.dir.join("poemercpricer.exe")).unwrap()
    }
    fn previous(&self) -> Option<Vec<u8>> {
        std::fs::read(self.dir.join("poemercpricer-previous.exe")).ok()
    }
    fn leftovers(&self) -> Vec<String> {
        std::fs::read_dir(&self.dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .filter(|n| n.contains(".update"))
            .collect()
    }
}

impl Drop for Outcome {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

/// Copy this test binary into a fresh folder and have it install `release`
/// over itself.
fn run_install(
    name: &str,
    download: &str,
    compressed: Option<&str>,
    size: u64,
    sha256: &str,
) -> Outcome {
    let dir = std::env::temp_dir().join(format!(
        "poemercpricer-install-{name}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let exe = dir.join("poemercpricer.exe");
    std::fs::copy(std::env::current_exe().unwrap(), &exe).unwrap();
    let old_sha256 = sha256_hex(&std::fs::read(&exe).unwrap()).unwrap();
    let result_path = dir.join("result.txt");

    let mut child = Command::new(&exe);
    child
        .args(["install_child", "--exact", "--test-threads=1"])
        .env("PMP_INSTALL_RESULT", &result_path)
        .env("PMP_INSTALL_DOWNLOAD", download)
        .env("PMP_INSTALL_SIZE", size.to_string())
        .env("PMP_INSTALL_SHA256", sha256)
        .env_remove("PMP_INSTALL_COMPRESSED");
    if let Some(url) = compressed {
        child.env("PMP_INSTALL_COMPRESSED", url);
    }
    let output = child.output().unwrap();
    assert!(
        output.status.success(),
        "child harness failed:\n{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let result = std::fs::read_to_string(&result_path).expect("child wrote its outcome");
    Outcome {
        dir,
        result,
        old_sha256,
    }
}

fn release_for(bytes: &[u8]) -> (u64, String) {
    (bytes.len() as u64, sha256_hex(bytes).unwrap())
}

fn assert_untouched(outcome: &Outcome) {
    assert_eq!(
        sha256_hex(&outcome.exe()).unwrap(),
        outcome.old_sha256,
        "exe was changed"
    );
    assert!(outcome.previous().is_none(), "previous.exe was written");
    assert!(
        outcome.leftovers().is_empty(),
        "temp file left: {:?}",
        outcome.leftovers()
    );
}

fn assert_installed(outcome: &Outcome, new: &[u8]) {
    assert_eq!(outcome.result, "ok");
    assert_eq!(outcome.exe(), new, "exe does not hold the served bytes");
    let previous = outcome.previous().expect("previous.exe kept as rollback");
    assert_eq!(
        sha256_hex(&previous).unwrap(),
        outcome.old_sha256,
        "previous.exe is not the old exe"
    );
    assert!(
        outcome.leftovers().is_empty(),
        "temp file left: {:?}",
        outcome.leftovers()
    );
}

#[test]
fn install_prefers_the_gzip_sibling() {
    let new = payload();
    let (size, sha) = release_for(&new);
    // Only the gzip route exists: succeeding proves it was used.
    let base = serve(HashMap::from([("/exe.gz", gzip(&new))]));
    let outcome = run_install(
        "gz",
        &format!("{base}/exe"),
        Some(&format!("{base}/exe.gz")),
        size,
        &sha,
    );
    assert_installed(&outcome, &new);
}

#[test]
fn install_falls_back_to_the_exe_when_the_gzip_is_missing_or_corrupt() {
    let new = payload();
    let (size, sha) = release_for(&new);
    let base = serve(HashMap::from([
        ("/exe", new.clone()),
        ("/corrupt.gz", gzip(&new[..new.len() - 1])),
    ]));
    let outcome = run_install(
        "gz404",
        &format!("{base}/exe"),
        Some(&format!("{base}/missing.gz")),
        size,
        &sha,
    );
    assert_installed(&outcome, &new);
    let outcome = run_install(
        "gzbad",
        &format!("{base}/exe"),
        Some(&format!("{base}/corrupt.gz")),
        size,
        &sha,
    );
    assert_installed(&outcome, &new);
}

#[test]
fn install_without_a_gzip_sibling_uses_the_exe() {
    let new = payload();
    let (size, sha) = release_for(&new);
    let base = serve(HashMap::from([("/exe", new.clone())]));
    let outcome = run_install("plain", &format!("{base}/exe"), None, size, &sha);
    assert_installed(&outcome, &new);
}

/// The second instance of the race: another copy already installed this release,
/// so `install` must skip the copy and the swap and leave that rollback alone.
#[test]
fn install_over_an_already_installed_release_keeps_the_rollback() {
    let new = std::fs::read(std::env::current_exe().unwrap()).unwrap();
    let (size, sha) = release_for(&new);
    let base = serve(HashMap::from([("/exe", new)]));
    let outcome = run_install("same", &format!("{base}/exe"), None, size, &sha);
    assert_eq!(outcome.result, "ok");
    assert_untouched(&outcome);
}

#[test]
fn install_refuses_a_download_that_does_not_match_the_digest() {
    let new = payload();
    let (size, _) = release_for(&new);
    let wrong = "0".repeat(64);
    let base = serve(HashMap::from([
        ("/exe", new.clone()),
        ("/exe.gz", gzip(&new)),
    ]));
    let outcome = run_install(
        "digest",
        &format!("{base}/exe"),
        Some(&format!("{base}/exe.gz")),
        size,
        &wrong,
    );
    assert!(
        outcome.result.contains("checksum mismatch"),
        "got {}",
        outcome.result
    );
    assert_untouched(&outcome);
}

#[test]
fn install_refuses_a_download_of_the_wrong_size() {
    let new = payload();
    let (size, sha) = release_for(&new);
    let base = serve(HashMap::from([("/exe", new.clone())]));
    let outcome = run_install("size", &format!("{base}/exe"), None, size + 1, &sha);
    assert!(
        outcome.result.contains("checksum mismatch"),
        "got {}",
        outcome.result
    );
    assert_untouched(&outcome);
}

#[test]
fn install_reports_a_missing_download() {
    let new = payload();
    let (size, sha) = release_for(&new);
    let base = serve(HashMap::new());
    let outcome = run_install("404", &format!("{base}/exe"), None, size, &sha);
    assert!(outcome.result.starts_with("err:"), "got {}", outcome.result);
    assert_untouched(&outcome);
}

#[test]
fn install_reports_an_unreachable_host() {
    let new = payload();
    let (size, sha) = release_for(&new);
    // Port 9 (discard) has no listener, so the connection is refused at once.
    let outcome = run_install("offline", "http://127.0.0.1:9/exe", None, size, &sha);
    assert!(
        outcome.result.contains("no internet connection"),
        "got {}",
        outcome.result
    );
    assert_untouched(&outcome);
}
