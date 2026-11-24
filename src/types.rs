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
pub enum SearchSuggestion {
    Query(String),
    Track(Track),
    Browse(CollectionCard),
}

#[derive(Debug, Clone)]
pub struct LyricLine {
    pub time_ms: Option<u64>,
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct TrackLyrics {
    pub lines: Vec<LyricLine>,
    pub synced: bool,
    pub source: String,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum VisualizerMode {
    #[default]
    Bars,
    Wave,
    Halo,
    Off,
}

impl VisualizerMode {
    pub fn next(self) -> Self {
        match self {
            Self::Bars => Self::Wave,
            Self::Wave => Self::Halo,
            Self::Halo => Self::Off,
            Self::Off => Self::Bars,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Bars => "Bars",
            Self::Wave => "Wave",
            Self::Halo => "Halo",
            Self::Off => "Off",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum EqPreset {
    #[default]
    Flat,
    BassBoost,
    LateNight,
    Vocal,
    Electronic,
    LoFi,
    Custom,
}

impl EqPreset {
    pub fn gains(self) -> [f32; 5] {
        match self {
            Self::Flat | Self::Custom => [0.0, 0.0, 0.0, 0.0, 0.0],
            Self::BassBoost => [6.5, 3.5, 0.0, 1.0, 1.5],
            Self::LateNight => [-2.5, 1.0, 2.5, 1.5, -2.0],
            Self::Vocal => [-1.5, 0.5, 4.5, 3.5, 1.0],
            Self::Electronic => [5.0, 2.0, -1.0, 3.0, 4.5],
            Self::LoFi => [3.5, 2.5, -1.0, -3.5, -6.0],
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Flat => "Flat",
            Self::BassBoost => "Bass Boost",
            Self::LateNight => "Late Night",
            Self::Vocal => "Vocal",
            Self::Electronic => "Electronic",
            Self::LoFi => "Lo-Fi",
            Self::Custom => "Custom",
        }
    }

    pub fn all_presets() -> &'static [EqPreset] {
        &[
            Self::Flat,
            Self::BassBoost,
            Self::LateNight,
            Self::Vocal,
            Self::Electronic,
            Self::LoFi,
        ]
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(default)]
pub struct DspConfig {
    pub eq_gains: [f32; 5],
    pub reverb_mix: f32,
    pub speed: f32,
}

impl Default for DspConfig {
    fn default() -> Self {
        Self {
            eq_gains: [0.0; 5],
            reverb_mix: 0.0,
            speed: 1.0,
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct SpectrumFrame {
    pub bands: [f32; 32],
    pub peaks: [f32; 32],
    pub bass: f32,
    pub rms: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppSettings {
    pub default_volume: f32,
    pub disk_audio_cache: bool,
    pub prefetch_ahead: usize,
    pub eq_preset: EqPreset,
    pub eq_gains: [f32; 5],
    pub reverb_mix: f32,
    pub playback_speed: f32,
    pub visualizer: VisualizerMode,
    pub auto_radio: bool,
    pub ambient_glow: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            default_volume: 0.85,
            disk_audio_cache: true,
            prefetch_ahead: 2,
            eq_preset: EqPreset::Flat,
            eq_gains: [0.0; 5],
            reverb_mix: 0.0,
            playback_speed: 1.0,
            visualizer: VisualizerMode::Bars,
            auto_radio: true,
            ambient_glow: true,
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
