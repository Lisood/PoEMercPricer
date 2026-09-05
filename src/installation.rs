//! Per-user installer integration. Portable copies never register themselves.

use std::path::Path;

pub const APP_ID: &str = "PoEMercPricer";
pub const RUNNING_MUTEX: &str = "Local\\PoEMercPricer.Running";
pub const UNINSTALL_KEY: &str =
    "Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\PoEMercPricer_is1";

/// Keep the named object alive until process exit, including while self_replace
/// renames the original image. Setup and Uninstall both check this object.
#[cfg(windows)]
pub fn mark_running() -> anyhow::Result<()> {
    use windows::core::HSTRING;
    use windows::Win32::System::Threading::CreateMutexW;
    // Deliberately not closed: Windows releases the handle at process exit.
    unsafe { CreateMutexW(None, false, &HSTRING::from(RUNNING_MUTEX)) }?;
    Ok(())
}

#[cfg(not(windows))]
pub fn mark_running() -> anyhow::Result<()> {
    Ok(())
}

/// Windows Installed apps should report the version now on disk. A metadata
/// failure must not turn a successful binary replacement into a failed update.
pub fn record_update(exe: &Path, version: &str) {
    #[cfg(windows)]
    if let Err(error) = record_update_at(exe, version, UNINSTALL_KEY) {
        eprintln!("Could not refresh installation details: {error:#}");
    }
    #[cfg(not(windows))]
    let _ = (exe, version);
}

#[cfg(windows)]
fn record_update_at(exe: &Path, version: &str, key_path: &str) -> anyhow::Result<()> {
    use windows::core::{w, HSTRING, PCWSTR};
    use windows::Win32::Foundation::{ERROR_FILE_NOT_FOUND, ERROR_PATH_NOT_FOUND};
    use windows::Win32::System::Registry::{
        RegCloseKey, RegGetValueW, RegOpenKeyExW, RegSetValueExW, HKEY, HKEY_CURRENT_USER,
        KEY_QUERY_VALUE, KEY_SET_VALUE, KEY_WOW64_64KEY, REG_SZ, RRF_RT_REG_SZ,
    };

    semver::Version::parse(version)?;
    if exe.file_name().and_then(|n| n.to_str()) != Some("poemercpricer.exe") {
        return Ok(());
    }
    let mut key = HKEY::default();
    let status = unsafe {
        RegOpenKeyExW(
            HKEY_CURRENT_USER,
            &HSTRING::from(key_path),
            0,
            KEY_QUERY_VALUE | KEY_SET_VALUE | KEY_WOW64_64KEY,
            &mut key,
        )
    };
    if status == ERROR_FILE_NOT_FOUND || status == ERROR_PATH_NOT_FOUND {
        return Ok(());
    }
    status.ok()?;
    struct Key(HKEY);
    impl Drop for Key {
        fn drop(&mut self) {
            let _ = unsafe { RegCloseKey(self.0) };
        }
    }
    let key = Key(key);
    let mut location = vec![0u16; 32768];
    let mut len = (location.len() * 2) as u32;
    unsafe {
        RegGetValueW(
            key.0,
            PCWSTR::null(),
            w!("InstallLocation"),
            RRF_RT_REG_SZ,
            None,
            Some(location.as_mut_ptr().cast()),
            Some(&mut len),
        )
    }
    .ok()?;
    let end = location.iter().position(|c| *c == 0).unwrap_or(0);
    let location = String::from_utf16(&location[..end])?;
    // Canonicalization also handles drive casing, Unicode and junction aliases.
    if std::fs::canonicalize(Path::new(&location).join("poemercpricer.exe"))?
        != std::fs::canonicalize(exe)?
    {
        return Ok(());
    }
    let value: Vec<u8> = version
        .encode_utf16()
        .chain([0])
        .flat_map(u16::to_le_bytes)
        .collect();
    unsafe { RegSetValueExW(key.0, w!("DisplayVersion"), 0, REG_SZ, Some(&value)) }.ok()?;
    Ok(())
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    use std::process::Command;

    #[test]
    fn metadata_updates_only_the_registered_copy_and_never_creates_a_registration() {
        let unique = format!("PoEMercPricer.MetadataTest.{}", std::process::id());
        let key = format!("Software\\{unique}");
        let root = std::env::temp_dir().join(&unique);
        let installed = root.join("installed space \u{6587}");
        let portable = root.join("portable");
        std::fs::create_dir_all(&installed).unwrap();
        std::fs::create_dir_all(&portable).unwrap();
        let exe = installed.join("poemercpricer.exe");
        let other = portable.join("poemercpricer.exe");
        std::fs::write(&exe, b"installed").unwrap();
        std::fs::write(&other, b"portable").unwrap();
        let reg = |args: &[&str]| Command::new("reg.exe").args(args).output().unwrap();
        let full_key = format!("HKCU\\{key}");
        struct Cleanup(String, std::path::PathBuf);
        impl Drop for Cleanup {
            fn drop(&mut self) {
                let _ = Command::new("reg.exe")
                    .args(["delete", &self.0, "/f", "/reg:64"])
                    .output();
                let _ = std::fs::remove_dir_all(&self.1);
            }
        }
        let _cleanup = Cleanup(full_key.clone(), root);
        record_update_at(&exe, "1.2.3", &key).unwrap();
        assert!(!reg(&["query", &full_key, "/reg:64"]).status.success());
        assert!(reg(&[
            "add",
            &full_key,
            "/v",
            "InstallLocation",
            "/d",
            installed.to_str().unwrap(),
            "/f",
            "/reg:64"
        ])
        .status
        .success());
        assert!(reg(&[
            "add",
            &full_key,
            "/v",
            "DisplayVersion",
            "/d",
            "1.0.0",
            "/f",
            "/reg:64"
        ])
        .status
        .success());
        record_update_at(&other, "8.0.0", &key).unwrap();
        let version = || {
            String::from_utf8_lossy(
                &reg(&["query", &full_key, "/v", "DisplayVersion", "/reg:64"]).stdout,
            )
            .into_owned()
        };
        assert!(version().contains("1.0.0"));
        record_update_at(&exe, "1.2.3", &key).unwrap();
        assert!(version().contains("1.2.3"));
        assert!(record_update_at(&exe, "invalid", &key).is_err());
        assert!(version().contains("1.2.3"));
    }
}
