use std::collections::hash_map::RandomState;
use std::hash::{BuildHasher, Hasher};
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

use crate::types::{CollectionCard, Track, format_time, parse_iso_duration_secs};

const GOOGLE_AUTH_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const GOOGLE_TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
const GOOGLE_USERINFO_URL: &str = "https://www.googleapis.com/oauth2/v2/userinfo";
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
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at: u64,
    pub user_name: String,
    pub user_email: Option<String>,
}

impl GoogleSession {
    pub fn is_expired(&self) -> bool {
        now_secs() + 60 >= self.expires_at
    }
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

pub async fn authorize_with_google(
    client: &reqwest::Client,
    config: &GoogleOAuthConfig,
) -> Result<GoogleSession, String> {
    let client_id = config.client_id.trim();
    if client_id.is_empty() {
        return Err(String::from("Enter a Google OAuth Client ID first."));
    }

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .map_err(|e| format!("Could not open local callback listener: {e}"))?;
    let port = listener
        .local_addr()
        .map_err(|e| format!("Failed getting callback port: {e}"))?
        .port();

    let redirect_uri = format!("http://127.0.0.1:{port}/callback");
    let state = random_token(24);
    let verifier = random_token(48);
    let challenge = b64url(&sha256_bytes(verifier.as_bytes()));

    let scope = "openid profile email https://www.googleapis.com/auth/youtube.readonly";
    let auth_url = format!(
        "{GOOGLE_AUTH_URL}?client_id={}&redirect_uri={}&response_type=code&scope={}&state={}&code_challenge={}&code_challenge_method=S256&access_type=offline&prompt=consent",
        pct_encode(client_id),
        pct_encode(&redirect_uri),
        pct_encode(scope),
        pct_encode(&state),
        pct_encode(&challenge),
    );

    open_browser(&auth_url);

    let code = wait_for_callback(listener, &state).await?;

    let mut form = vec![
        ("client_id", client_id.to_string()),
        ("code", code),
        ("code_verifier", verifier),
        ("grant_type", "authorization_code".to_string()),
        ("redirect_uri", redirect_uri),
    ];
    if !config.client_secret.trim().is_empty() {
        form.push(("client_secret", config.client_secret.trim().to_string()));
    }

    let token_res = client
        .post(GOOGLE_TOKEN_URL)
        .form(&form)
        .send()
        .await
        .map_err(|e| format!("Google token request failed: {e}"))?;

    if !token_res.status().is_success() {
        let status = token_res.status();
        let body = token_res.text().await.unwrap_or_default();
        return Err(format!("Google token exchange failed ({status}): {body}"));
    }

    let token_json: Value = token_res
        .json()
        .await
        .map_err(|e| format!("Invalid Google token response: {e}"))?;

    let access_token = token_json
        .get("access_token")
        .and_then(|v| v.as_str())
        .ok_or_else(|| String::from("Google did not return an access_token"))?
        .to_string();

    let refresh_token = token_json
        .get("refresh_token")
        .and_then(|v| v.as_str())
        .map(str::to_string);

    let expires_in = token_json
        .get("expires_in")
        .and_then(|v| v.as_u64())
        .unwrap_or(3600);

    let (user_name, user_email) = fetch_userinfo(client, &access_token).await;

    let session = GoogleSession {
        access_token,
        refresh_token,
        expires_at: now_secs() + expires_in,
        user_name,
        user_email,
    };

    save_google_session(&session)?;
    Ok(session)
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

async fn wait_for_callback(listener: TcpListener, expected_state: &str) -> Result<String, String> {
    let accept = tokio::time::timeout(Duration::from_secs(180), listener.accept())
        .await
        .map_err(|_| String::from("Timed out waiting for Google sign-in in browser."))?
        .map_err(|e| format!("Callback error: {e}"))?;

    let (mut stream, _) = accept;
    let mut buf = [0u8; 4096];
    let n = stream
        .read(&mut buf)
        .await
        .map_err(|e| format!("Failed reading callback: {e}"))?;

    let req = String::from_utf8_lossy(&buf[..n]);
    let first_line = req.lines().next().unwrap_or("");
    let path = first_line.split_whitespace().nth(1).unwrap_or("");
    let query = path.split_once('?').map(|(_, q)| q).unwrap_or("");

    let mut code = None;
    let mut state = None;
    let mut err = None;

    for pair in query.split('&') {
        if let Some((k, v)) = pair.split_once('=') {
            let val = pct_decode(v);
            match k {
                "code" => code = Some(val),
                "state" => state = Some(val),
                "error" => err = Some(val),
                _ => {}
            }
        }
    }

    let (status_line, html) = if err.is_some() || code.is_none() {
        (
            "HTTP/1.1 400 Bad Request",
            "<html><body style='background:#0b0b0b;color:#fff;font-family:sans-serif;padding:48px'><h2>Sign-in cancelled or failed</h2><p>You can close this tab and return to hum.</p></body></html>",
        )
    } else {
        (
            "HTTP/1.1 200 OK",
            "<html><body style='background:#0b0b0b;color:#fff;font-family:sans-serif;padding:48px'><h2>Authorized with Google</h2><p>You can close this tab and return to hum.</p></body></html>",
        )
    };

    let response = format!(
        "{status_line}\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{html}",
        html.len()
    );
    let _ = stream.write_all(response.as_bytes()).await;

    if let Some(e) = err {
        return Err(format!("Google sign-in declined: {e}"));
    }
    if state.as_deref() != Some(expected_state) {
        return Err(String::from("OAuth state mismatch."));
    }

    code.ok_or_else(|| String::from("No authorization code returned by Google."))
}

async fn fetch_userinfo(client: &reqwest::Client, access_token: &str) -> (String, Option<String>) {
    let Ok(res) = client
        .get(GOOGLE_USERINFO_URL)
        .bearer_auth(access_token)
        .send()
        .await
    else {
        return (String::from("Google User"), None);
    };

    let Ok(json) = res.json::<Value>().await else {
        return (String::from("Google User"), None);
    };

    let name = json
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("Google User")
        .to_string();
    let email = json
        .get("email")
        .and_then(|v| v.as_str())
        .map(str::to_string);

    (name, email)
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

fn random_token(bytes_len: usize) -> String {
    let mut bytes = Vec::with_capacity(bytes_len);
    let rs = RandomState::new();
    while bytes.len() < bytes_len {
        let mut h = rs.build_hasher();
        h.write_u64(now_secs());
        h.write_usize(bytes.len());
        bytes.extend_from_slice(&h.finish().to_le_bytes());
    }
    bytes.truncate(bytes_len);
    b64url(&bytes)
}

fn b64url(input: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut out = String::with_capacity((input.len() * 4).div_ceil(3));
    for chunk in input.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = chunk.get(1).copied().unwrap_or(0) as u32;
        let b2 = chunk.get(2).copied().unwrap_or(0) as u32;
        let n = (b0 << 16) | (b1 << 8) | b2;
        out.push(TABLE[((n >> 18) & 63) as usize] as char);
        out.push(TABLE[((n >> 12) & 63) as usize] as char);
        if chunk.len() > 1 {
            out.push(TABLE[((n >> 6) & 63) as usize] as char);
        }
        if chunk.len() > 2 {
            out.push(TABLE[(n & 63) as usize] as char);
        }
    }
    out
}

fn pct_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~') {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

fn pct_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && i + 2 < bytes.len()
            && let Ok(hex) = std::str::from_utf8(&bytes[i + 1..i + 3])
            && let Ok(val) = u8::from_str_radix(hex, 16)
        {
            out.push(val);
            i += 3;
            continue;
        }
        if bytes[i] == b'+' {
            out.push(b' ');
        } else {
            out.push(bytes[i]);
        }
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn open_browser(url: &str) {
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

pub fn sha256_bytes(data: &[u8]) -> [u8; 32] {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];

    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];

    let bit_len = (data.len() as u64) * 8;
    let mut msg = data.to_vec();
    msg.push(0x80);
    while (msg.len() % 64) != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&bit_len.to_be_bytes());

    for chunk in msg.chunks_exact(64) {
        let mut w = [0u32; 64];
        for i in 0..16 {
            w[i] = u32::from_be_bytes([
                chunk[i * 4],
                chunk[i * 4 + 1],
                chunk[i * 4 + 2],
                chunk[i * 4 + 3],
            ]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }

        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut hh] = h;
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let temp1 = hh
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0.wrapping_add(maj);

            hh = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }

        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
        h[5] = h[5].wrapping_add(f);
        h[6] = h[6].wrapping_add(g);
        h[7] = h[7].wrapping_add(hh);
    }

    let mut out = [0u8; 32];
    for (i, word) in h.iter().enumerate() {
        out[i * 4..i * 4 + 4].copy_from_slice(&word.to_be_bytes());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha256_matches_standard_vector() {
        let digest = sha256_bytes(b"abc");
        let hex: String = digest.iter().map(|b| format!("{b:02x}")).collect();
        assert_eq!(
            hex,
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }
}
