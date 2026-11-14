use std::path::PathBuf;
use std::process::Stdio;
use std::sync::LazyLock;
use std::time::Duration;

use serde_json::{Value, json};
use tokio::process::Command;
use tokio::sync::Semaphore;

use crate::auth::app_data_dir;
use crate::types::{
    CollectionCard, MoodPill, PlaylistPage, Shelf, ShelfItem, Track, parse_duration_secs,
};

const SEARCH_URL: &str = "https://music.youtube.com/youtubei/v1/search?prettyPrint=false";
const BROWSE_URL: &str = "https://music.youtube.com/youtubei/v1/browse?prettyPrint=false";
const SONGS_PARAM: &str = "EgWKAQIIAWoMEAMQBBAJEAoQBRAR";
const PLAYLISTS_PARAM: &str = "EgWKAQIoAWoMEAMQBBAJEAoQBRAR";
const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36";

static THUMB_SEM: LazyLock<Semaphore> = LazyLock::new(|| Semaphore::new(6));

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

pub fn build_http_client() -> reqwest::Client {
    reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(30))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new())
}

fn client_context() -> Value {
    json!({
        "client": {
            "clientName": "WEB_REMIX",
            "clientVersion": "1.20241106.01.00",
            "hl": "en",
            "gl": "US"
        }
    })
}

async fn post_innertube(
    client: &reqwest::Client,
    url: &str,
    payload: Value,
) -> Result<Value, String> {
    let res = client
        .post(url)
        .header("Origin", "https://music.youtube.com")
        .header("Referer", "https://music.youtube.com/")
        .json(&payload)
        .send()
        .await
        .map_err(|e| format!("Network request failed: {e}"))?;

    if !res.status().is_success() {
        return Err(format!("YouTube Music returned HTTP {}", res.status()));
    }

    res.json()
        .await
        .map_err(|e| format!("Failed reading response JSON: {e}"))
}

pub async fn search_all(
    client: &reqwest::Client,
    query: &str,
) -> Result<(Vec<Track>, Vec<CollectionCard>), String> {
    let trimmed = query.trim();
    if trimmed.is_empty() {
        return Ok((Vec::new(), Vec::new()));
    }

    let songs_payload = json!({
        "context": client_context(),
        "query": trimmed,
        "params": SONGS_PARAM
    });
    let playlists_payload = json!({
        "context": client_context(),
        "query": trimmed,
        "params": PLAYLISTS_PARAM
    });

    let (songs_res, playlists_res) = tokio::join!(
        post_innertube(client, SEARCH_URL, songs_payload),
        post_innertube(client, SEARCH_URL, playlists_payload),
    );

    let tracks = parse_search_results(&songs_res?);
    let cards = playlists_res
        .map(|v| parse_search_playlists(&v))
        .unwrap_or_default();

    Ok((tracks, cards))
}

pub async fn fetch_browse_shelves(
    client: &reqwest::Client,
    browse_id: &str,
    params: Option<&str>,
) -> Result<Vec<Shelf>, String> {
    let mut payload = json!({
        "context": client_context(),
        "browseId": browse_id
    });
    if let Some(p) = params {
        payload["params"] = json!(p);
    }

    let body = post_innertube(client, BROWSE_URL, payload).await?;
    Ok(parse_browse_shelves(&body))
}

pub async fn fetch_playlist(
    client: &reqwest::Client,
    browse_id: &str,
    params: Option<&str>,
) -> Result<PlaylistPage, String> {
    let mut payload = json!({
        "context": client_context(),
        "browseId": browse_id
    });
    if let Some(p) = params {
        payload["params"] = json!(p);
    }

    let body = post_innertube(client, BROWSE_URL, payload).await?;
    Ok(parse_playlist_page(&body))
}

pub fn parse_browse_shelves(root: &Value) -> Vec<Shelf> {
    let mut shelves = Vec::new();
    let Some(sections) = root
        .pointer("/contents/singleColumnBrowseResultsRenderer/tabs/0/tabRenderer/content/sectionListRenderer/contents")
        .and_then(|v| v.as_array())
    else {
        return shelves;
    };

    for section in sections {
        if let Some(carousel) = section.get("musicCarouselShelfRenderer") {
            let title = carousel
                .pointer("/header/musicCarouselShelfBasicHeaderRenderer/title/runs/0/text")
                .and_then(|v| v.as_str())
                .unwrap_or("Recommended")
                .to_string();

            let mut items = Vec::new();
            if let Some(contents) = carousel.get("contents").and_then(|v| v.as_array()) {
                for raw in contents {
                    if let Some(item) = parse_shelf_item(raw) {
                        items.push(item);
                    }
                }
            }
            if !items.is_empty() {
                shelves.push(Shelf { title, items });
            }
        } else if let Some(grid) = section.get("gridRenderer") {
            let title = grid
                .pointer("/header/gridHeaderRenderer/title/runs/0/text")
                .and_then(|v| v.as_str())
                .unwrap_or("Moods & genres")
                .to_string();

            let mut items = Vec::new();
            if let Some(contents) = grid.get("items").and_then(|v| v.as_array()) {
                for raw in contents {
                    if let Some(item) = parse_shelf_item(raw) {
                        items.push(item);
                    }
                }
            }
            if !items.is_empty() {
                shelves.push(Shelf { title, items });
            }
        } else if let Some(shelf) = section.get("musicShelfRenderer") {
            let title = shelf
                .pointer("/title/runs/0/text")
                .and_then(|v| v.as_str())
                .unwrap_or("Tracks")
                .to_string();

            let mut items = Vec::new();
            if let Some(contents) = shelf.get("contents").and_then(|v| v.as_array()) {
                for raw in contents {
                    if let Some(track) = parse_song_item(raw) {
                        items.push(ShelfItem::Song(track));
                    }
                }
            }
            if !items.is_empty() {
                shelves.push(Shelf { title, items });
            }
        }
    }

    shelves
}

fn parse_shelf_item(raw: &Value) -> Option<ShelfItem> {
    if let Some(track) = parse_song_item(raw) {
        return Some(ShelfItem::Song(track));
    }

    if let Some(nav) = raw.get("musicNavigationButtonRenderer") {
        let title = runs_text(nav.pointer("/buttonText/runs"))?;
        let browse_id = nav
            .pointer("/clickCommand/browseEndpoint/browseId")
            .and_then(|v| v.as_str())?
            .to_string();
        let params = nav
            .pointer("/clickCommand/browseEndpoint/params")
            .and_then(|v| v.as_str())
            .map(str::to_string);
        let accent = nav
            .pointer("/solid/leftStripeColor")
            .and_then(|v| v.as_u64())
            .map(|v| (v & 0x00FF_FFFF) as u32)
            .unwrap_or(0xFF0000);

        return Some(ShelfItem::Mood(MoodPill {
            title,
            browse_id,
            params,
            accent,
        }));
    }

    let two_row = raw.get("musicTwoRowItemRenderer")?;
    let title = runs_text(two_row.pointer("/title/runs"))?;
    let subtitle = runs_text(two_row.pointer("/subtitle/runs")).unwrap_or_default();
    let thumb_url = pick_thumbnail(
        two_row
            .pointer("/thumbnailRenderer/musicThumbnailRenderer/thumbnail/thumbnails")
            .and_then(|v| v.as_array()),
        280,
    );

    if let Some(video_id) = two_row
        .pointer("/navigationEndpoint/watchEndpoint/videoId")
        .and_then(|v| v.as_str())
    {
        return Some(ShelfItem::Song(Track {
            id: video_id.to_string(),
            title,
            artist: if subtitle.is_empty() {
                String::from("YouTube Music")
            } else {
                subtitle
            },
            album: None,
            duration_label: String::from("--:--"),
            duration_secs: None,
            thumb_url,
        }));
    }

    let browse_id = two_row
        .pointer("/navigationEndpoint/browseEndpoint/browseId")
        .and_then(|v| v.as_str())?
        .to_string();
    let params = two_row
        .pointer("/navigationEndpoint/browseEndpoint/params")
        .and_then(|v| v.as_str())
        .map(str::to_string);

    Some(ShelfItem::Card(CollectionCard {
        browse_id,
        params,
        title,
        subtitle,
        thumb_url,
    }))
}

pub fn parse_playlist_page(root: &Value) -> PlaylistPage {
    let header_two = root.pointer("/contents/twoColumnBrowseResultsRenderer/tabs/0/tabRenderer/content/sectionListRenderer/contents/0/musicResponsiveHeaderRenderer");
    let header_single = root.pointer("/header/musicDetailHeaderRenderer");
    let header = header_two.or(header_single);

    let title = header
        .and_then(|h| runs_text(h.pointer("/title/runs")))
        .unwrap_or_else(|| String::from("Playlist"));

    let subtitle = header
        .and_then(|h| runs_text(h.pointer("/subtitle/runs")))
        .unwrap_or_default();

    let thumb_url = header.and_then(|h| {
        pick_thumbnail(
            h.pointer("/thumbnail/musicThumbnailRenderer/thumbnail/thumbnails")
                .or_else(|| h.pointer("/thumbnail/croppedSquareThumbnailRenderer/thumbnail/thumbnails"))
                .and_then(|v| v.as_array()),
            320,
        )
    });

    let mut tracks = Vec::new();
    let section_paths = [
        "/contents/twoColumnBrowseResultsRenderer/secondaryContents/sectionListRenderer/contents",
        "/contents/singleColumnBrowseResultsRenderer/tabs/0/tabRenderer/content/sectionListRenderer/contents",
    ];

    for path in section_paths {
        if let Some(sections) = root.pointer(path).and_then(|v| v.as_array()) {
            for sec in sections {
                let items = sec
                    .pointer("/musicPlaylistShelfRenderer/contents")
                    .or_else(|| sec.pointer("/musicShelfRenderer/contents"))
                    .and_then(|v| v.as_array());
                if let Some(items) = items {
                    for item in items {
                        if let Some(track) = parse_song_item(item) {
                            tracks.push(track);
                        }
                    }
                }
            }
        }
    }

    PlaylistPage {
        title,
        subtitle,
        thumb_url,
        tracks,
    }
}

pub fn parse_search_results(root: &Value) -> Vec<Track> {
    let mut tracks = Vec::new();
    let Some(sections) = root
        .pointer("/contents/tabbedSearchResultsRenderer/tabs/0/tabRenderer/content/sectionListRenderer/contents")
        .and_then(|v| v.as_array())
    else {
        return tracks;
    };

    for section in sections {
        let Some(items) = section
            .pointer("/musicShelfRenderer/contents")
            .and_then(|v| v.as_array())
        else {
            continue;
        };

        for item in items {
            if let Some(track) = parse_song_item(item) {
                tracks.push(track);
            }
        }
    }

    tracks
}

fn parse_search_playlists(root: &Value) -> Vec<CollectionCard> {
    let mut cards = Vec::new();
    let Some(sections) = root
        .pointer("/contents/tabbedSearchResultsRenderer/tabs/0/tabRenderer/content/sectionListRenderer/contents")
        .and_then(|v| v.as_array())
    else {
        return cards;
    };

    for section in sections {
        let Some(items) = section
            .pointer("/musicShelfRenderer/contents")
            .and_then(|v| v.as_array())
        else {
            continue;
        };

        for item in items.iter().take(12) {
            let Some(renderer) = item.get("musicResponsiveListItemRenderer") else {
                continue;
            };
            let Some(browse_id) = renderer
                .pointer("/navigationEndpoint/browseEndpoint/browseId")
                .and_then(|v| v.as_str())
            else {
                continue;
            };
            let Some(title) = runs_text(
                renderer.pointer("/flexColumns/0/musicResponsiveListItemFlexColumnRenderer/text/runs"),
            ) else {
                continue;
            };
            let subtitle = runs_text(
                renderer.pointer("/flexColumns/1/musicResponsiveListItemFlexColumnRenderer/text/runs"),
            )
            .unwrap_or_default();
            let thumb_url = pick_thumbnail(
                renderer
                    .pointer("/thumbnail/musicThumbnailRenderer/thumbnail/thumbnails")
                    .and_then(|v| v.as_array()),
                226,
            );
            cards.push(CollectionCard {
                browse_id: browse_id.to_string(),
                params: None,
                title,
                subtitle,
                thumb_url,
            });
        }
    }

    cards
}

fn parse_song_item(item: &Value) -> Option<Track> {
    let renderer = item.get("musicResponsiveListItemRenderer")?;

    let video_id = renderer
        .pointer("/playlistItemData/videoId")
        .or_else(|| {
            renderer.pointer(
                "/overlay/musicItemThumbnailOverlayRenderer/content/musicPlayButtonRenderer/playNavigationEndpoint/watchEndpoint/videoId",
            )
        })
        .or_else(|| {
            renderer.pointer(
                "/flexColumns/0/musicResponsiveListItemFlexColumnRenderer/text/runs/0/navigationEndpoint/watchEndpoint/videoId",
            )
        })
        .and_then(|v| v.as_str())?
        .to_string();

    let title = renderer
        .pointer("/flexColumns/0/musicResponsiveListItemFlexColumnRenderer/text/runs/0/text")
        .and_then(|v| v.as_str())?
        .to_string();

    let mut artist_parts = Vec::new();
    let mut album: Option<String> = None;
    let mut duration_label = String::from("--:--");
    let mut duration_secs = None;

    if let Some(runs) = renderer
        .pointer("/flexColumns/1/musicResponsiveListItemFlexColumnRenderer/text/runs")
        .and_then(|v| v.as_array())
    {
        let mut segments: Vec<String> = Vec::new();
        let mut current = String::new();

        for run in runs {
            let text = run.get("text").and_then(|t| t.as_str()).unwrap_or("");
            if text.trim() == "•" {
                let seg = current.trim().to_string();
                if !seg.is_empty() {
                    segments.push(seg);
                }
                current.clear();
            } else {
                current.push_str(text);
            }
        }
        let tail = current.trim().to_string();
        if !tail.is_empty() {
            segments.push(tail);
        }

        if matches!(segments.first().map(String::as_str), Some("Song" | "Video")) {
            segments.remove(0);
        }

        if let Some(last) = segments.last()
            && let Some(secs) = parse_duration_secs(last)
        {
            duration_label = last.clone();
            duration_secs = Some(secs);
            segments.pop();
        }

        match segments.as_slice() {
            [artist] => artist_parts.push(artist.clone()),
            [artist, alb, ..] => {
                artist_parts.push(artist.clone());
                album = Some(alb.clone());
            }
            [] => {}
        }
    }

    if duration_secs.is_none()
        && let Some(fixed_text) = renderer
            .pointer("/fixedColumns/0/musicResponsiveListItemFixedColumnRenderer/text/runs/0/text")
            .and_then(|v| v.as_str())
        && let Some(secs) = parse_duration_secs(fixed_text)
    {
        duration_label = fixed_text.trim().to_string();
        duration_secs = Some(secs);
    }

    let artist = if artist_parts.is_empty() {
        String::from("Unknown artist")
    } else {
        artist_parts.join(", ")
    };

    let thumb_url = pick_thumbnail(
        renderer
            .pointer("/thumbnail/musicThumbnailRenderer/thumbnail/thumbnails")
            .and_then(|v| v.as_array()),
        226,
    );

    Some(Track {
        id: video_id,
        title,
        artist,
        album,
        duration_label,
        duration_secs,
        thumb_url,
    })
}

fn runs_text(runs: Option<&Value>) -> Option<String> {
    let arr = runs?.as_array()?;
    let joined: String = arr
        .iter()
        .filter_map(|r| r.get("text").and_then(|t| t.as_str()))
        .collect();
    let trimmed = joined.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn pick_thumbnail(thumbs: Option<&Vec<Value>>, target_width: u64) -> Option<String> {
    let thumbs = thumbs?;
    let best = thumbs
        .iter()
        .filter_map(|t| {
            let url = t.get("url")?.as_str()?;
            let width = t.get("width").and_then(|w| w.as_u64()).unwrap_or(0);
            Some((url, width))
        })
        .min_by_key(|(_, w)| w.abs_diff(target_width))?;

    Some(best.0.to_string())
}

pub fn find_yt_dlp() -> Option<PathBuf> {
    if let Ok(exe) = std::env::current_exe()
        && let Some(dir) = exe.parent()
    {
        let sibling = dir.join(if cfg!(windows) { "yt-dlp.exe" } else { "yt-dlp" });
        if sibling.is_file() {
            return Some(sibling);
        }
    }

    if let Some(local_bin) = local_yt_dlp_path()
        && local_bin.is_file()
    {
        return Some(local_bin);
    }

    let path_var = std::env::var_os("PATH")?;
    let bin_name = if cfg!(windows) { "yt-dlp.exe" } else { "yt-dlp" };
    for dir in std::env::split_paths(&path_var) {
        let candidate = dir.join(bin_name);
        if candidate.is_file() {
            return Some(candidate);
        }
    }

    None
}

fn local_yt_dlp_path() -> Option<PathBuf> {
    let bin_name = if cfg!(windows) { "yt-dlp.exe" } else { "yt-dlp" };
    Some(app_data_dir()?.join(bin_name))
}

pub async fn install_yt_dlp(client: &reqwest::Client) -> Result<PathBuf, String> {
    let dest = local_yt_dlp_path()
        .ok_or_else(|| String::from("Could not determine local app data directory"))?;
    let parent = dest
        .parent()
        .ok_or_else(|| String::from("Invalid destination path"))?;

    tokio::fs::create_dir_all(parent)
        .await
        .map_err(|e| format!("Failed creating folder: {e}"))?;

    let download_url = if cfg!(windows) {
        "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp.exe"
    } else {
        "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp"
    };

    let bytes = download_bytes(client, download_url).await?;
    let tmp = dest.with_extension("tmp");
    tokio::fs::write(&tmp, &bytes)
        .await
        .map_err(|e| format!("Failed writing yt-dlp: {e}"))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(0o755);
        tokio::fs::set_permissions(&tmp, perms)
            .await
            .map_err(|e| format!("Failed setting permissions: {e}"))?;
    }

    tokio::fs::rename(&tmp, &dest)
        .await
        .map_err(|e| format!("Failed finalizing yt-dlp: {e}"))?;

    Ok(dest)
}

pub async fn fetch_track_audio(video_id: &str) -> Result<Vec<u8>, String> {
    let yt_dlp = find_yt_dlp().ok_or_else(|| {
        String::from("yt-dlp not found. Click Setup audio in the top bar.")
    })?;

    match run_yt_dlp_audio(&yt_dlp, video_id, None).await {
        Ok(bytes) => Ok(bytes),
        Err(first_err) => {
            // Retry with mobile player clients if default extraction hits a snag.
            run_yt_dlp_audio(&yt_dlp, video_id, Some("youtube:player_client=android_vr,ios,web"))
                .await
                .map_err(|_| first_err)
        }
    }
}

async fn run_yt_dlp_audio(
    yt_dlp: &PathBuf,
    video_id: &str,
    extractor_args: Option<&str>,
) -> Result<Vec<u8>, String> {
    let watch_url = format!("https://music.youtube.com/watch?v={video_id}");
    let mut cmd = Command::new(yt_dlp);
    cmd.args([
        "-f",
        "140/bestaudio[ext=m4a][protocol^=https]/bestaudio[ext=mp4][protocol^=https]/bestaudio[ext=mp3]/bestaudio[protocol^=https]",
        "--no-playlist",
        "--no-warnings",
    ]);

    if let Some(args) = extractor_args {
        cmd.args(["--extractor-args", args]);
    }

    cmd.args(["-o", "-", &watch_url])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);

    let out = cmd
        .output()
        .await
        .map_err(|e| format!("Failed to start yt-dlp: {e}"))?;

    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        let first_line = err.lines().next().unwrap_or("yt-dlp failed");
        return Err(first_line.to_string());
    }

    if out.stdout.is_empty() {
        return Err(String::from("yt-dlp returned empty audio data"));
    }

    Ok(out.stdout)
}

pub async fn download_thumb_bytes(client: &reqwest::Client, url: &str) -> Result<Vec<u8>, String> {
    let _permit = THUMB_SEM
        .acquire()
        .await
        .map_err(|_| String::from("Semaphore closed"))?;
    download_bytes(client, url).await
}

pub async fn download_bytes(client: &reqwest::Client, url: &str) -> Result<Vec<u8>, String> {
    let res = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("Download failed: {e}"))?;

    if !res.status().is_success() {
        return Err(format!("Server returned HTTP {}", res.status()));
    }

    res.bytes()
        .await
        .map(|b| b.to_vec())
        .map_err(|e| format!("Failed reading bytes: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_song_item_metadata() {
        let sample = json!({
            "contents": {
                "tabbedSearchResultsRenderer": {
                    "tabs": [{
                        "tabRenderer": {
                            "content": {
                                "sectionListRenderer": {
                                    "contents": [{
                                        "musicShelfRenderer": {
                                            "contents": [{
                                                "musicResponsiveListItemRenderer": {
                                                    "playlistItemData": { "videoId": "abc123xyz" },
                                                    "flexColumns": [
                                                        {
                                                            "musicResponsiveListItemFlexColumnRenderer": {
                                                                "text": { "runs": [{ "text": "Midnight City" }] }
                                                            }
                                                        },
                                                        {
                                                            "musicResponsiveListItemFlexColumnRenderer": {
                                                                "text": {
                                                                    "runs": [
                                                                        { "text": "Song" },
                                                                        { "text": " • " },
                                                                        { "text": "M83" },
                                                                        { "text": " • " },
                                                                        { "text": "Hurry Up, We're Dreaming" },
                                                                        { "text": " • " },
                                                                        { "text": "4:03" }
                                                                    ]
                                                                }
                                                            }
                                                        }
                                                    ],
                                                    "thumbnail": {
                                                        "musicThumbnailRenderer": {
                                                            "thumbnail": {
                                                                "thumbnails": [
                                                                    { "url": "https://example.com/60.jpg", "width": 60 },
                                                                    { "url": "https://example.com/226.jpg", "width": 226 }
                                                                ]
                                                            }
                                                        }
                                                    }
                                                }
                                            }]
                                        }
                                    }]
                                }
                            }
                        }
                    }]
                }
            }
        });

        let tracks = parse_search_results(&sample);
        assert_eq!(tracks.len(), 1);
        assert_eq!(tracks[0].id, "abc123xyz");
        assert_eq!(tracks[0].title, "Midnight City");
        assert_eq!(tracks[0].artist, "M83");
        assert_eq!(tracks[0].album.as_deref(), Some("Hurry Up, We're Dreaming"));
        assert_eq!(tracks[0].duration_secs, Some(243));
        assert_eq!(tracks[0].thumb_url.as_deref(), Some("https://example.com/226.jpg"));
    }

    #[tokio::test]
    #[ignore = "hits live music.youtube.com"]
    async fn live_search_smoke() {
        let client = build_http_client();
        let (tracks, _) = search_all(&client, "daft punk instant crush")
            .await
            .unwrap();
        assert!(!tracks.is_empty(), "expected non-empty live search results");
        println!(
            "First hit: {} - {} ({})",
            tracks[0].title, tracks[0].artist, tracks[0].id
        );
    }

    #[tokio::test]
    #[ignore = "hits live yt-dlp and decodes audio"]
    async fn live_playback_smoke() {
        for id in ["khnokW3Mw24", "5At0aMBsSSE"] {
            let bytes = fetch_track_audio(id)
                .await
                .expect("fetch_track_audio failed");
            println!("{id}: Downloaded bytes: {}", bytes.len());
            let decoder =
                rodio::Decoder::new(std::io::Cursor::new(bytes)).expect("rodio Decoder failed");
            println!(
                "{id}: Decoded OK! sample_rate={:?}, duration={:?}",
                rodio::Source::sample_rate(&decoder),
                rodio::Source::total_duration(&decoder)
            );
        }
    }
}
