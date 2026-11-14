use std::time::Duration;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Track {
    pub id: String,
    pub title: String,
    pub artist: String,
    pub album: Option<String>,
    pub duration_label: String,
    pub duration_secs: Option<u64>,
    pub thumb_url: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CollectionCard {
    pub browse_id: String,
    pub params: Option<String>,
    pub title: String,
    pub subtitle: String,
    pub thumb_url: Option<String>,
}

#[derive(Debug, Clone)]
pub struct MoodPill {
    pub title: String,
    pub browse_id: String,
    pub params: Option<String>,
    pub accent: u32,
}

#[derive(Debug, Clone)]
pub enum ShelfItem {
    Song(Track),
    Card(CollectionCard),
    Mood(MoodPill),
}

#[derive(Debug, Clone)]
pub struct Shelf {
    pub title: String,
    pub items: Vec<ShelfItem>,
}

#[derive(Debug, Clone)]
pub struct PlaylistPage {
    pub title: String,
    pub subtitle: String,
    pub thumb_url: Option<String>,
    pub tracks: Vec<Track>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPlaylist {
    pub id: String,
    pub name: String,
    pub tracks: Vec<Track>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppSettings {
    pub default_volume: f32,
    pub disk_audio_cache: bool,
    pub prefetch_ahead: usize,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            default_volume: 0.85,
            disk_audio_cache: true,
            prefetch_ahead: 2,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaybackState {
    Stopped,
    Loading,
    Playing,
    Paused,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepeatMode {
    Off,
    All,
    One,
}

impl RepeatMode {
    pub fn next(self) -> Self {
        match self {
            Self::Off => Self::All,
            Self::All => Self::One,
            Self::One => Self::Off,
        }
    }
}

#[derive(Debug, Clone)]
pub struct AudioSnapshot {
    pub seq: u64,
    pub state: PlaybackState,
    pub position: Duration,
    pub duration: Option<Duration>,
    pub finished_track: bool,
    pub error: Option<String>,
}

impl Default for AudioSnapshot {
    fn default() -> Self {
        Self {
            seq: 0,
            state: PlaybackState::Stopped,
            position: Duration::ZERO,
            duration: None,
            finished_track: false,
            error: None,
        }
    }
}

pub fn format_time(secs: u64) -> String {
    let hours = secs / 3600;
    let mins = (secs % 3600) / 60;
    let rem = secs % 60;

    if hours > 0 {
        format!("{hours}:{mins:02}:{rem:02}")
    } else {
        format!("{mins}:{rem:02}")
    }
}

pub fn parse_duration_secs(text: &str) -> Option<u64> {
    let parts: Vec<u64> = text
        .trim()
        .split(':')
        .map(|p| p.parse::<u64>().ok())
        .collect::<Option<Vec<_>>>()?;

    match parts.as_slice() {
        [m, s] if *s < 60 => Some(m * 60 + s),
        [h, m, s] if *m < 60 && *s < 60 => Some(h * 3600 + m * 60 + s),
        _ => None,
    }
}

pub fn parse_iso_duration_secs(iso: &str) -> Option<u64> {
    let s = iso.strip_prefix("PT")?;
    let mut num = 0u64;
    let mut total = 0u64;
    for ch in s.chars() {
        if let Some(d) = ch.to_digit(10) {
            num = num * 10 + u64::from(d);
        } else {
            match ch {
                'H' => total += num * 3600,
                'M' => total += num * 60,
                'S' => total += num,
                _ => return None,
            }
            num = 0;
        }
    }
    Some(total)
}

pub fn truncate_text(s: &str, max_chars: usize) -> String {
    let count = s.chars().count();
    if count <= max_chars {
        return s.to_string();
    }
    let keep = max_chars.saturating_sub(1);
    let mut out: String = s.chars().take(keep).collect();
    out.push('…');
    out
}
