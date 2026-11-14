use serde_json::Value;

const RELEASES_LATEST_URL: &str = "https://api.github.com/repos/Hum-fr/Hum/releases/latest";

#[derive(Debug, Clone)]
pub struct UpdateInfo {
    pub version: String,
    pub tag_name: String,
    pub changelog: String,
    pub html_url: String,
    pub exe_download_url: Option<String>,
}

/// Removes leftover `.old` binary created by a previous self-update on Windows.
pub fn cleanup_old_exe() {
    if let Ok(exe) = std::env::current_exe() {
        let mut old = exe.into_os_string();
        old.push(".old");
        let old_path = std::path::PathBuf::from(old);
        if old_path.exists() {
            let _ = std::fs::remove_file(old_path);
        }
    }
}

pub fn parse_semver(raw: &str) -> Option<(u64, u64, u64)> {
    let trimmed = raw.trim().trim_start_matches(['v', 'V']);
    let core = trimmed.split(['-', '+']).next()?.trim();
    let mut parts = core.split('.');
    let major = parts.next()?.parse::<u64>().ok()?;
    let minor = parts.next().unwrap_or("0").parse::<u64>().ok()?;
    let patch = parts.next().unwrap_or("0").parse::<u64>().ok()?;
    Some((major, minor, patch))
}

pub fn is_newer_version(latest: &str, current: &str) -> bool {
    match (parse_semver(latest), parse_semver(current)) {
        (Some(l), Some(c)) => l > c,
        _ => false,
    }
}

pub async fn check_for_updates(client: &reqwest::Client) -> Result<Option<UpdateInfo>, String> {
    let res = client
        .get(RELEASES_LATEST_URL)
        .header("Accept", "application/vnd.github+json")
        .header("User-Agent", concat!("hum/", env!("CARGO_PKG_VERSION")))
        .send()
        .await
        .map_err(|e| format!("Update check failed: {e}"))?;

    if !res.status().is_success() {
        return Err(format!("GitHub returned HTTP {}", res.status()));
    }

    let json: Value = res
        .json()
        .await
        .map_err(|e| format!("Invalid release JSON: {e}"))?;

    let tag_name = json
        .get("tag_name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| String::from("Release missing tag_name"))?
        .to_string();

    if !is_newer_version(&tag_name, env!("CARGO_PKG_VERSION")) {
        return Ok(None);
    }

    let version = tag_name.trim_start_matches(['v', 'V']).to_string();
    let changelog = json
        .get("body")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("New release available on GitHub.")
        .to_string();

    let html_url = json
        .get("html_url")
        .and_then(|v| v.as_str())
        .unwrap_or("https://github.com/Hum-fr/Hum/releases/latest")
        .to_string();

    let exe_download_url = json
        .get("assets")
        .and_then(|v| v.as_array())
        .and_then(|assets| {
            assets.iter().find_map(|asset| {
                let name = asset.get("name")?.as_str()?;
                if name.eq_ignore_ascii_case("hum.exe") {
                    asset
                        .get("browser_download_url")
                        .and_then(|u| u.as_str())
                        .map(str::to_string)
                } else {
                    None
                }
            })
        });

    Ok(Some(UpdateInfo {
        version,
        tag_name,
        changelog,
        html_url,
        exe_download_url,
    }))
}

pub async fn download_and_replace_exe(
    client: &reqwest::Client,
    download_url: &str,
) -> Result<(), String> {
    let res = client
        .get(download_url)
        .header("User-Agent", concat!("hum/", env!("CARGO_PKG_VERSION")))
        .send()
        .await
        .map_err(|e| format!("Download failed: {e}"))?;

    if !res.status().is_success() {
        return Err(format!("Download returned HTTP {}", res.status()));
    }

    let bytes = res
        .bytes()
        .await
        .map_err(|e| format!("Failed reading update binary: {e}"))?;

    #[cfg(windows)]
    if bytes.len() < 1024 || &bytes[..2] != b"MZ" {
        return Err(String::from("Downloaded file is not a valid Windows executable."));
    }

    let exe_path =
        std::env::current_exe().map_err(|e| format!("Failed locating current exe: {e}"))?;

    let mut new_os = exe_path.clone().into_os_string();
    new_os.push(".new");
    let new_path = std::path::PathBuf::from(new_os);

    let mut old_os = exe_path.clone().into_os_string();
    old_os.push(".old");
    let old_path = std::path::PathBuf::from(old_os);

    tokio::fs::write(&new_path, &bytes)
        .await
        .map_err(|e| format!("Could not write update file (try View release): {e}"))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = tokio::fs::set_permissions(&new_path, std::fs::Permissions::from_mode(0o755)).await;
    }

    let _ = std::fs::remove_file(&old_path);
    if let Err(e) = std::fs::rename(&exe_path, &old_path) {
        let _ = std::fs::remove_file(&new_path);
        return Err(format!("Cannot replace running exe in this folder: {e}"));
    }

    if let Err(e) = std::fs::rename(&new_path, &exe_path) {
        let _ = std::fs::rename(&old_path, &exe_path);
        return Err(format!("Failed installing new executable: {e}"));
    }

    std::process::Command::new(&exe_path)
        .spawn()
        .map_err(|e| format!("Updated, but failed to relaunch: {e}"))?;

    std::process::exit(0);
}

pub fn open_url(url: &str) {
    #[cfg(windows)]
    {
        let _ = std::process::Command::new("cmd")
            .args(["/C", "start", "", url])
            .spawn();
    }
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open").arg(url).spawn();
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let _ = std::process::Command::new("xdg-open").arg(url).spawn();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn semver_comparison_works() {
        assert!(is_newer_version("v0.1.2", "0.1.1"));
        assert!(is_newer_version("v0.2.0", "0.1.9"));
        assert!(is_newer_version("1.0.0", "0.9.9"));
        assert!(!is_newer_version("v0.1.1", "0.1.1"));
        assert!(!is_newer_version("v0.1.0", "0.1.1"));
    }
}
