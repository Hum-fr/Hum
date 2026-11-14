use std::path::PathBuf;
use std::process::Stdio;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::process::Command;

use crate::types::{CollectionCard, ShelfItem, Track, format_time, parse_iso_duration_secs};
use crate::webview_login::{SIGNIN_FLAG, WebViewOutcome};
use crate::ytmusic::{self, YtmAuth};

const GOOGLE_TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
const YOUTUBE_API_BASE: &str = "https://www.googleapis.com/youtube/v3";

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GoogleOAuthConfig {
    pub client_id: String,
    pub client_secret: String,
}

impl GoogleOAuthConfig {
    pub fn load() -> Option<Self> {
        let env_id = std::env::var("HUM_GOOGLE_CLIENT_ID").ok();
        let compiled_id = option_env!("HUM_GOOGLE_CLIENT_ID").map(str::to_string);
        let client_id = env_id
            .filter(|s| !s.trim().is_empty())
            .or(compiled_id.filter(|s| !s.trim().is_empty()))?;

        let env_secret = std::env::var("HUM_GOOGLE_CLIENT_SECRET").ok();
        let compiled_secret = option_env!("HUM_GOOGLE_CLIENT_SECRET").map(str::to_string);
        let client_secret = env_secret
            .filter(|s| !s.trim().is_empty())
            .or(compiled_secret)
            .unwrap_or_default();

        Some(Self {
            client_id: client_id.trim().to_string(),
            client_secret: client_secret.trim().to_string(),
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoogleSession {
    #[serde(default)]
    pub access_token: String,
    #[serde(default)]
    pub refresh_token: Option<String>,
    #[serde(default)]
    pub expires_at: u64,
    pub user_name: String,
    pub user_email: Option<String>,
    #[serde(default)]
    pub avatar_url: Option<String>,
    #[serde(default)]
    pub cookie_header: Option<String>,
    #[serde(default)]
    pub sapisid: Option<String>,
}

impl GoogleSession {
    pub fn is_expired(&self) -> bool {
        now_secs() + 60 >= self.expires_at
    }

    pub fn ytm_auth(&self) -> Option<YtmAuth> {
        let cookie_header = self.cookie_header.as_deref()?.trim();
        let sapisid = self.sapisid.as_deref()?.trim();
        if cookie_header.is_empty() || sapisid.is_empty() {
            return None;
        }
        Some(YtmAuth {
            cookie_header: cookie_header.to_string(),
            sapisid: sapisid.to_string(),
        })
    }
}

/// Launches the embedded WebView2 sign-in window and returns an authenticated session.
pub async fn sign_in_with_webview(client: &reqwest::Client) -> Result<GoogleSession, String> {
    let dir = app_data_dir().ok_or_else(|| String::from("No local app data directory"))?;
    tokio::fs::create_dir_all(&dir)
        .await
        .map_err(|e| format!("Failed creating app dir: {e}"))?;

    let out_path = dir.join("ytm_signin_result.json");
    let _ = tokio::fs::remove_file(&out_path).await;

    let exe = std::env::current_exe()
        .map_err(|e| format!("Failed locating current executable: {e}"))?;

    let mut cmd = Command::new(exe);
    cmd.arg(SIGNIN_FLAG)
        .arg(&out_path)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    let mut child = cmd
        .spawn()
        .map_err(|e| format!("Failed launching sign-in window: {e}"))?;

    let status = tokio::time::timeout(Duration::from_secs(300), child.wait())
        .await
        .map_err(|_| {
            let _ = child.start_kill();
            String::from("Timed out waiting for Google sign-in.")
        })?
        .map_err(|e| format!("Sign-in window error: {e}"))?;

    let raw = tokio::fs::read_to_string(&out_path).await.ok();
    let _ = tokio::fs::remove_file(&out_path).await;

    let outcome = raw
        .as_deref()
        .and_then(|s| serde_json::from_str::<WebViewOutcome>(s).ok());

    let payload = match outcome {
        Some(WebViewOutcome::Ok(p)) => p,
        Some(WebViewOutcome::Err { message }) => return Err(message),
        None if !status.success() => return Err(String::from("Sign-in cancelled.")),
        None => return Err(String::from("Sign-in window closed without session cookies.")),
    };

    let ytm_auth = YtmAuth {
        cookie_header: payload.cookie_header.clone(),
        sapisid: payload.sapisid.clone(),
    };
    let (user_name, user_email, avatar_url) = ytmusic::fetch_account_info(client, &ytm_auth).await;

    let session = GoogleSession {
        access_token: String::new(),
        refresh_token: None,
        expires_at: 0,
        user_name,
        user_email,
        avatar_url,
        cookie_header: Some(payload.cookie_header),
        sapisid: Some(payload.sapisid),
    };

    save_google_session(&session)?;
    Ok(session)
}

pub async fn refresh_account_profile(
    client: &reqwest::Client,
    mut session: GoogleSession,
) -> Option<GoogleSession> {
    let auth = session.ytm_auth()?;
    let (name, email, avatar) = ytmusic::fetch_account_info(client, &auth).await;
    if name != "YouTube Music" {
        session.user_name = name;
    }
    if email.is_some() {
        session.user_email = email;
    }
    if avatar.is_some() {
        session.avatar_url = avatar;
    }
    let _ = save_google_session(&session);
    Some(session)
}

pub async fn fetch_ytm_playlists(
    client: &reqwest::Client,
    auth: &YtmAuth,
) -> Result<Vec<CollectionCard>, String> {
    let shelves = ytmusic::fetch_browse_shelves(
        client,
        "FEmusic_liked_playlists",
        None,
        Some(auth),
    )
    .await?;

    let mut cards = Vec::new();
    for shelf in shelves {
        for item in shelf.items {
            if let ShelfItem::Card(card) = item
                && !card.browse_id.is_empty()
                && !card.browse_id.starts_with("FEmusic_")
            {
                cards.push(card);
            }
        }
    }
    Ok(cards)
}

pub async fn fetch_ytm_liked_tracks(
    client: &reqwest::Client,
    auth: &YtmAuth,
) -> Result<Vec<Track>, String> {
    if let Ok(page) = ytmusic::fetch_playlist(client, "VLLM", None, Some(auth)).await
        && !page.tracks.is_empty()
    {
        return Ok(page.tracks);
    }

    let shelves = ytmusic::fetch_browse_shelves(
        client,
        "FEmusic_liked_videos",
        None,
        Some(auth),
    )
    .await?;

    let mut tracks = Vec::new();
    for shelf in shelves {
        for item in shelf.items {
            if let ShelfItem::Song(track) = item {
                tracks.push(track);
            }
        }
    }
    Ok(tracks)
}

pub fn sapisid_auth_header(sapisid: &str) -> String {
    let ts = now_secs();
    let msg = format!("{ts} {sapisid} https://music.youtube.com");
    let digest = sha1_bytes(msg.as_bytes());
    let hex: String = digest.iter().map(|b| format!("{b:02x}")).collect();
    format!("SAPISIDHASH {ts}_{hex}")
}

pub fn app_data_dir() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        let base = std::env::var_os("LOCALAPPDATA")?;
        Some(PathBuf::from(base).join("hum"))
    }
    #[cfg(not(windows))]
    {
        let home = std::env::var_os("HOME")?;
        Some(PathBuf::from(home).join(".local").join("share").join("hum"))
    }
}

pub fn load_google_session() -> Option<GoogleSession> {
    let path = app_data_dir()?.join("google_auth.json");
    let raw = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&raw).ok()
}

pub fn save_google_session(session: &GoogleSession) -> Result<(), String> {
    let dir = app_data_dir().ok_or_else(|| String::from("No local app data directory"))?;
    std::fs::create_dir_all(&dir).map_err(|e| format!("Failed creating app dir: {e}"))?;
    let json = serde_json::to_string_pretty(session).map_err(|e| e.to_string())?;
    std::fs::write(dir.join("google_auth.json"), json)
        .map_err(|e| format!("Failed saving Google session: {e}"))
}

pub fn clear_google_session() {
    if let Some(dir) = app_data_dir() {
        let _ = std::fs::remove_file(dir.join("google_auth.json"));
    }
}

pub async fn ensure_valid_token(
    client: &reqwest::Client,
    session: &mut GoogleSession,
) -> Result<String, String> {
    if !session.is_expired() {
        return Ok(session.access_token.clone());
    }

    let Some(refresh) = session.refresh_token.clone() else {
        return Err(String::from("Google session expired — please sign in again."));
    };
    let config = GoogleOAuthConfig::load()
        .ok_or_else(|| String::from("Missing Google OAuth Client ID for refresh."))?;

    let mut form = vec![
        ("client_id", config.client_id),
        ("refresh_token", refresh),
        ("grant_type", "refresh_token".to_string()),
    ];
    if !config.client_secret.is_empty() {
        form.push(("client_secret", config.client_secret));
    }

    let res = client
        .post(GOOGLE_TOKEN_URL)
        .form(&form)
        .send()
        .await
        .map_err(|e| format!("Token refresh failed: {e}"))?;

    if !res.status().is_success() {
        return Err(format!("Token refresh rejected ({})", res.status()));
    }

    let json: Value = res.json().await.map_err(|e| e.to_string())?;
    let access_token = json
        .get("access_token")
        .and_then(|v| v.as_str())
        .ok_or_else(|| String::from("Refresh returned no access_token"))?
        .to_string();
    let expires_in = json
        .get("expires_in")
        .and_then(|v| v.as_u64())
        .unwrap_or(3600);

    session.access_token = access_token.clone();
    session.expires_at = now_secs() + expires_in;
    let _ = save_google_session(session);
    Ok(access_token)
}

pub async fn fetch_google_playlists(
    client: &reqwest::Client,
    access_token: &str,
) -> Result<Vec<CollectionCard>, String> {
    let url = format!("{YOUTUBE_API_BASE}/playlists?part=snippet,contentDetails&mine=true&maxResults=25");
    let res = client
        .get(url)
        .bearer_auth(access_token)
        .send()
        .await
        .map_err(|e| format!("Failed loading Google playlists: {e}"))?;

    if !res.status().is_success() {
        return Err(format!("YouTube API returned HTTP {}", res.status()));
    }

    let json: Value = res.json().await.map_err(|e| e.to_string())?;
    let mut cards = Vec::new();
    if let Some(items) = json.get("items").and_then(|v| v.as_array()) {
        for item in items {
            let Some(id) = item.get("id").and_then(|v| v.as_str()) else {
                continue;
            };
            let title = item
                .pointer("/snippet/title")
                .and_then(|v| v.as_str())
                .unwrap_or("Playlist")
                .to_string();
            let count = item
                .pointer("/contentDetails/itemCount")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            let thumb_url = item
                .pointer("/snippet/thumbnails/medium/url")
                .or_else(|| item.pointer("/snippet/thumbnails/default/url"))
                .and_then(|v| v.as_str())
                .map(str::to_string);

            cards.push(CollectionCard {
                browse_id: format!("VL{id}"),
                params: None,
                title,
                subtitle: format!("{count} tracks"),
                thumb_url,
            });
        }
    }
    Ok(cards)
}

pub async fn fetch_google_liked_tracks(
    client: &reqwest::Client,
    access_token: &str,
) -> Result<Vec<Track>, String> {
    let url = format!("{YOUTUBE_API_BASE}/videos?part=snippet,contentDetails&myRating=like&maxResults=30");
    let res = client
        .get(url)
        .bearer_auth(access_token)
        .send()
        .await
        .map_err(|e| format!("Failed loading Google likes: {e}"))?;

    if !res.status().is_success() {
        return Err(format!("YouTube API returned HTTP {}", res.status()));
    }

    let json: Value = res.json().await.map_err(|e| e.to_string())?;
    let mut tracks = Vec::new();
    if let Some(items) = json.get("items").and_then(|v| v.as_array()) {
        for item in items {
            let Some(id) = item.get("id").and_then(|v| v.as_str()) else {
                continue;
            };
            let title = item
                .pointer("/snippet/title")
                .and_then(|v| v.as_str())
                .unwrap_or("Untitled")
                .to_string();
            let artist = item
                .pointer("/snippet/channelTitle")
                .and_then(|v| v.as_str())
                .unwrap_or("YouTube")
                .trim_end_matches(" - Topic")
                .to_string();
            let duration_secs = item
                .pointer("/contentDetails/duration")
                .and_then(|v| v.as_str())
                .and_then(parse_iso_duration_secs);
            let duration_label = duration_secs
                .map(format_time)
                .unwrap_or_else(|| String::from("--:--"));
            let thumb_url = item
                .pointer("/snippet/thumbnails/medium/url")
                .or_else(|| item.pointer("/snippet/thumbnails/default/url"))
                .and_then(|v| v.as_str())
                .map(str::to_string);

            tracks.push(Track {
                id: id.to_string(),
                title,
                artist,
                album: None,
                duration_label,
                duration_secs,
                thumb_url,
            });
        }
    }

    Ok(tracks)
}

pub fn load_liked_tracks() -> Vec<Track> {
    load_json_file("library.json")
}

pub fn save_liked_tracks(tracks: &[Track]) {
    save_json_file("library.json", tracks);
}

pub fn load_user_playlists() -> Vec<crate::types::UserPlaylist> {
    load_json_file("playlists.json")
}

pub fn save_user_playlists(playlists: &[crate::types::UserPlaylist]) {
    save_json_file("playlists.json", playlists);
}

pub fn load_history() -> Vec<Track> {
    load_json_file("history.json")
}

pub fn save_history(tracks: &[Track]) {
    save_json_file("history.json", tracks);
}

pub fn load_settings() -> crate::types::AppSettings {
    load_json_file("settings.json")
}

pub fn save_settings(settings: &crate::types::AppSettings) {
    save_json_file("settings.json", settings);
}

fn load_json_file<T: for<'de> Deserialize<'de> + Default>(name: &str) -> T {
    let Some(dir) = app_data_dir() else {
        return T::default();
    };
    let Ok(raw) = std::fs::read_to_string(dir.join(name)) else {
        return T::default();
    };
    serde_json::from_str(&raw).unwrap_or_default()
}

fn save_json_file<T: Serialize + ?Sized>(name: &str, data: &T) {
    let Some(dir) = app_data_dir() else {
        return;
    };
    if std::fs::create_dir_all(&dir).is_ok()
        && let Ok(json) = serde_json::to_string_pretty(data)
    {
        let _ = std::fs::write(dir.join(name), json);
    }
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

pub fn sha1_bytes(data: &[u8]) -> [u8; 20] {
    let mut h: [u32; 5] = [0x67452301, 0xEFCDAB89, 0x98BADCFE, 0x10325476, 0xC3D2E1F0];

    let bit_len = (data.len() as u64) * 8;
    let mut msg = data.to_vec();
    msg.push(0x80);
    while (msg.len() % 64) != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&bit_len.to_be_bytes());

    for chunk in msg.chunks_exact(64) {
        let mut w = [0u32; 80];
        for i in 0..16 {
            w[i] = u32::from_be_bytes([
                chunk[i * 4],
                chunk[i * 4 + 1],
                chunk[i * 4 + 2],
                chunk[i * 4 + 3],
            ]);
        }
        for i in 16..80 {
            w[i] = (w[i - 3] ^ w[i - 8] ^ w[i - 14] ^ w[i - 16]).rotate_left(1);
        }

        let [mut a, mut b, mut c, mut d, mut e] = h;
        for (i, &word) in w.iter().enumerate() {
            let (f, k) = match i {
                0..=19 => ((b & c) | ((!b) & d), 0x5A827999u32),
                20..=39 => (b ^ c ^ d, 0x6ED9EBA1u32),
                40..=59 => ((b & c) | (b & d) | (c & d), 0x8F1BBCDCu32),
                _ => (b ^ c ^ d, 0xCA62C1D6u32),
            };
            let temp = a
                .rotate_left(5)
                .wrapping_add(f)
                .wrapping_add(e)
                .wrapping_add(k)
                .wrapping_add(word);
            e = d;
            d = c;
            c = b.rotate_left(30);
            b = a;
            a = temp;
        }

        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
    }

    let mut out = [0u8; 20];
    for (i, word) in h.iter().enumerate() {
        out[i * 4..i * 4 + 4].copy_from_slice(&word.to_be_bytes());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha1_matches_standard_vector() {
        let digest = sha1_bytes(b"abc");
        let hex: String = digest.iter().map(|b| format!("{b:02x}")).collect();
        assert_eq!(hex, "a9993e364706816aba3e25717850c26c9cd0d89d");
    }
}
