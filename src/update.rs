//! Auto-updater: one GitHub Releases lookup, a verified download, an in-place
//! swap of the running exe. Blocking; the UI runs it on a worker thread.
//! Design and the verification spike behind every API call: `docs/updater.md`.

use anyhow::{bail, Context, Result};
use std::path::Path;
use std::time::Duration;

pub const REPO: &str = "Lisood/PoEMercPricer";
pub const ASSET: &str = "poemercpricer-windows-x64.exe";
/// Optional gzip copy of `ASSET` (about 40% of the bytes). Verified against
/// the exe asset's own size and digest, so it adds no trust; a release without
/// it, or a bad download of it, falls back to the exe asset.
pub const COMPRESSED_ASSET: &str = "poemercpricer-windows-x64.exe.gz";
pub const RELEASES_URL: &str = "https://github.com/Lisood/PoEMercPricer/releases";
pub const CURRENT: &str = env!("CARGO_PKG_VERSION");

const JSON_TIMEOUT: Duration = Duration::from_secs(15);
const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(120);

#[derive(Clone, Debug, PartialEq)]
pub struct Release {
    pub version: semver::Version,
    pub page: String,
    pub download: String,
    /// `browser_download_url` of `COMPRESSED_ASSET` when the release has one.
    pub compressed: Option<String>,
    /// Size and digest of the exe asset; the compressed copy is checked
    /// against these after inflation.
    pub size: u64,
    pub sha256: String,
}

#[derive(Clone, Debug)]
pub enum UpdateState {
    Idle,
    Checking,
    UpToDate { checked: std::time::Instant },
    Available(Release),
    Downloading(Release),
    Ready { version: String },
    Failed { message: String },
}

/// One GET to `releases/latest`. `Ok(None)` means the running build is current.
pub fn check() -> Result<Option<Release>> {
    #[cfg(windows)]
    winrt()?;
    let url = format!("https://api.github.com/repos/{REPO}/releases/latest");
    let body = fetch(&url, JSON_TIMEOUT)?;
    parse_release(&body, CURRENT)
}

/// Download beside `exe`, verify size and sha256, keep the old exe as
/// `poemercpricer-previous.exe`, then swap. Nothing is changed on failure.
pub fn install(release: &Release, exe: &Path) -> Result<()> {
    #[cfg(windows)]
    winrt()?;
    // Try the gzip sibling first; any failure there (missing, network, corrupt,
    // mismatch) falls back to the exe asset.
    let compressed = release
        .compressed
        .as_deref()
        .and_then(|url| fetch(url, DOWNLOAD_TIMEOUT).ok())
        .and_then(|gz| inflate_verified(&gz, release).ok());
    let bytes = match compressed {
        Some(bytes) => bytes,
        None => {
            let bytes = fetch(&release.download, DOWNLOAD_TIMEOUT)?;
            verify(&bytes, release)?;
            bytes
        }
    };

    let dir = exe.parent().unwrap_or(Path::new("."));
    // Per-pid, so two running instances updating at once cannot tear each
    // other's temp file out from under a write or a self_replace.
    let temp = exe.with_extension(format!("exe.{}.update", std::process::id()));
    std::fs::write(&temp, &bytes).map_err(|e| write_error(dir, e))?;

    let finish = || -> Result<()> {
        // Another instance installed this same release while we were downloading;
        // copying the already-new exe over previous.exe would destroy its rollback.
        if sha256_hex(&std::fs::read(exe)?)? == release.sha256 {
            return Ok(());
        }
        std::fs::copy(exe, dir.join("poemercpricer-previous.exe"))
            .map_err(|e| write_error(dir, e))?;
        self_replace::self_replace(&temp).map_err(|e| write_error(dir, e))?;
        Ok(())
    };
    let result = finish();
    let _ = std::fs::remove_file(&temp);
    result
}

/// Size and sha256 of the exe asset must both match; nothing else is trusted.
pub fn verify(bytes: &[u8], release: &Release) -> Result<()> {
    if bytes.len() as u64 != release.size {
        bail!("checksum mismatch (expected {} bytes)", release.size);
    }
    if sha256_hex(bytes)? != release.sha256 {
        bail!("checksum mismatch");
    }
    Ok(())
}

/// Inflate a downloaded `COMPRESSED_ASSET` and verify it like the exe asset.
pub fn inflate_verified(gz: &[u8], release: &Release) -> Result<Vec<u8>> {
    let bytes = gunzip(gz, release.size)?;
    verify(&bytes, release)?;
    Ok(bytes)
}

/// gunzip, reading at most `expected_len + 1` bytes so a corrupt or hostile
/// stream cannot inflate without bound; the caller's size check then fails.
pub fn gunzip(gz: &[u8], expected_len: u64) -> Result<Vec<u8>> {
    use std::io::Read;
    let mut out = Vec::with_capacity(expected_len as usize);
    flate2::read::GzDecoder::new(gz)
        .take(expected_len + 1)
        .read_to_end(&mut out)
        .context("inflating the compressed download")?;
    Ok(out)
}

/// Parse a `releases/latest` body. `Ok(None)` when the tag is not newer.
pub fn parse_release(json: &[u8], current: &str) -> Result<Option<Release>> {
    #[derive(serde::Deserialize)]
    struct Latest {
        tag_name: String,
        html_url: String,
        assets: Vec<Asset>,
    }
    #[derive(serde::Deserialize)]
    struct Asset {
        name: String,
        browser_download_url: String,
        size: u64,
        #[serde(default)]
        digest: Option<String>,
    }

    let latest: Latest =
        serde_json::from_slice(json).context("GitHub sent a release listing I cannot read")?;
    if !is_newer(&latest.tag_name, current)? {
        return Ok(None);
    }
    let asset = latest
        .assets
        .iter()
        .find(|a| a.name == ASSET)
        .context("the release has no Windows download")?;
    if asset.size > 100_000_000 {
        bail!(
            "the release download is implausibly large ({} bytes)",
            asset.size
        );
    }
    let sha256 = asset
        .digest
        .as_deref()
        .and_then(|d| d.strip_prefix("sha256:"))
        .filter(|hex| hex.len() == 64 && hex.chars().all(|c| c.is_ascii_hexdigit()))
        .context("the release has no usable sha256 checksum")?
        .to_lowercase();
    Ok(Some(Release {
        version: parse_tag(&latest.tag_name)?,
        page: latest.html_url,
        download: asset.browser_download_url.clone(),
        compressed: latest
            .assets
            .iter()
            .find(|a| a.name == COMPRESSED_ASSET)
            .map(|a| a.browser_download_url.clone()),
        size: asset.size,
        sha256,
    }))
}

/// Strict `>`. Accepts `v0.3.0` or `0.3.0`; prerelease tags are never newer.
pub fn is_newer(tag: &str, current: &str) -> Result<bool> {
    let candidate = parse_tag(tag)?;
    Ok(candidate.pre.is_empty() && candidate > parse_tag(current)?)
}

fn parse_tag(tag: &str) -> Result<semver::Version> {
    semver::Version::parse(tag.strip_prefix('v').unwrap_or(tag))
        .with_context(|| format!("release tag {tag:?} is not a version"))
}

fn write_error(dir: &Path, error: std::io::Error) -> anyhow::Error {
    if error.kind() == std::io::ErrorKind::PermissionDenied {
        anyhow::anyhow!(
            "permission denied writing {}; move poemercpricer.exe to a folder you can write to",
            dir.display()
        )
    } else {
        anyhow::Error::new(error).context(format!("writing to {}", dir.display()))
    }
}

/// Lower-case hex SHA-256, via the Win32 CNG one-shot call. Needs no
/// COM/WinRT apartment, unlike the WinRT `HashAlgorithmProvider`, so it is
/// safe to call from any thread at any point in its life.
pub fn sha256_hex(bytes: &[u8]) -> Result<String> {
    #[cfg(windows)]
    {
        use std::fmt::Write;
        use windows::Win32::Security::Cryptography::{BCryptHash, BCRYPT_SHA256_ALG_HANDLE};

        let mut hash = [0u8; 32];
        unsafe { BCryptHash(BCRYPT_SHA256_ALG_HANDLE, None, bytes, &mut hash) }
            .ok()
            .context("hashing the download")?;
        Ok(hash
            .iter()
            .fold(String::with_capacity(64), |mut hex, byte| {
                let _ = write!(hex, "{byte:02x}");
                hex
            }))
    }
    #[cfg(not(windows))]
    {
        let _ = bytes;
        bail!("sha256 needs Windows")
    }
}

/// The updater runs on worker threads that never pump messages, so use the MTA.
/// A thread already initialized in another mode keeps working. `RoInitialize`
/// alone ties the process MTA's lifetime to threads: when the last WinRT thread
/// exits, its apartment reference goes with it, and once the count hits zero
/// the MTA tears down mid-process, dangling the activation factories `windows`
/// caches in statics and faulting the next WinRT call from a fresh thread.
/// `CoIncrementMTAUsage` pins the MTA for the process with a cookie that is
/// deliberately never released.
#[cfg(windows)]
fn winrt() -> Result<()> {
    use std::cell::Cell;
    use std::sync::Once;
    use windows::Win32::Foundation::RPC_E_CHANGED_MODE;
    use windows::Win32::System::Com::CoIncrementMTAUsage;
    use windows::Win32::System::WinRT::{RoInitialize, RO_INIT_MULTITHREADED};

    static MTA: Once = Once::new();
    MTA.call_once(|| unsafe {
        let _ = CoIncrementMTAUsage();
    });

    thread_local! {
        static INITIALIZED: Cell<bool> = const { Cell::new(false) };
    }
    INITIALIZED.with(|initialized| {
        if initialized.get() {
            return Ok(());
        }
        match unsafe { RoInitialize(RO_INIT_MULTITHREADED) } {
            Ok(()) => {}
            Err(error) if error.code() == RPC_E_CHANGED_MODE => {}
            Err(error) => return Err(error).context("initializing WinRT for the updater"),
        }
        initialized.set(true);
        Ok(())
    })
}

/// One blocking GET through the OS HTTP stack: system proxy, OS trust store,
/// redirects followed. Maps GitHub's 403 and 404 to the reasons the UI shows.
#[cfg(windows)]
pub fn fetch(url: &str, timeout: Duration) -> Result<Vec<u8>> {
    use windows::core::HSTRING;
    use windows::Foundation::{AsyncStatus, Uri};
    use windows::Storage::Streams::DataReader;
    use windows::Web::Http::HttpClient;

    fn offline(error: windows::core::Error) -> anyhow::Error {
        anyhow::anyhow!("no internet connection ({error})")
    }

    winrt()?;
    let client = HttpClient::new().map_err(offline)?;
    let headers = client.DefaultRequestHeaders()?;
    headers
        .UserAgent()?
        .TryParseAdd(&HSTRING::from(format!("PoEMercPricer/{CURRENT}")))?;
    headers
        .Accept()?
        .TryParseAdd(&HSTRING::from("application/vnd.github+json"))?;

    let op = client
        .GetAsync(&Uri::CreateUri(&HSTRING::from(url))?)
        .map_err(offline)?;
    let started = std::time::Instant::now();
    while op.Status().map_err(offline)? == AsyncStatus::Started {
        if started.elapsed() > timeout {
            let _ = op.Cancel();
            bail!(
                "no internet connection (timed out after {}s)",
                timeout.as_secs()
            );
        }
        std::thread::sleep(Duration::from_millis(50));
    }

    let response = op.GetResults().map_err(offline)?;
    if !response.IsSuccessStatusCode().map_err(offline)? {
        match response.StatusCode().map_err(offline)?.0 {
            403 | 429 => bail!("GitHub rate limit, try again in an hour"),
            404 => bail!("no release published yet"),
            code => bail!("GitHub returned HTTP {code}"),
        }
    }

    let buffer = response
        .Content()
        .map_err(offline)?
        .ReadAsBufferAsync()
        .map_err(offline)?
        .get()
        .map_err(offline)?;
    let mut bytes = vec![0u8; buffer.Length()? as usize];
    DataReader::FromBuffer(&buffer)?.ReadBytes(&mut bytes)?;
    Ok(bytes)
}

/// See the Windows implementation; this crate only ships on Windows.
#[cfg(not(windows))]
pub fn fetch(url: &str, timeout: Duration) -> Result<Vec<u8>> {
    let _ = (url, timeout);
    bail!("updates need Windows")
}
