# hum

A fast, native desktop client for YouTube Music built in pure Rust.

Most desktop wrappers load the entire `music.youtube.com` web app inside Electron or a browser view, burning 200–400 MB of RAM just to play audio. `hum` uses a native GPU-accelerated `iced` interface, queries YouTube Music directly, and decodes audio natively with `rodio`—keeping idle CPU near zero and memory usage around a few dozen megabytes.

---

## What's New in v0.2.0

- **Sound Lab Audio DSP (`Ctrl+E`)** — Real-time 5-band Biquad Equalizer with 6 built-in presets + vertical sliders, Stereo Studio Reverb, Playback Speed (`0.75x`–`1.50x`), 1-click **Slowed + Reverb** & **Nightcore**, and a soft-fading **Sleep Timer**.
- **GPU Audio Visualizers & Reactive Ambient Glow** — Live 512-point FFT spectrum analyzer rendering **Bars**, **Wave**, and **Halo** modes on GPU canvas alongside a bass-breathing multi-orb obsidian ambient mesh and Player Bar **MiniMeter**.
- **Zen Cinema Stage (`Ctrl+Shift+Z`)** — Hides chrome and sidebars for pure glowing cover art, visualizer motion, and large synced lyrics.
- **Endless Auto-Radio & Queue Reorder** — Automatically discovers and appends matching tracks at the end of a queue (or hit **Radio** on any song), with `↑`/`↓` queue track reordering.
- **Spotlight Command Palette (`Ctrl+K`) & Refreshed Home** — Instant command & preset runner anywhere in the app, plus Home Vibe chips, a **Jump Back In** hero card, and live toast feedback.

---

## Features

- **YouTube Music Browse & Search**
  - **Home** feed with **Jump Back In** hero card, **Quick Vibe** mood chips, *Listen again* history, 3-column Quick Picks, and personalized shelves
  - **Explore** page with Moods & Genres pills, New Releases, and trending shelves
  - **Live Search Preview** — Debounced autocomplete dropdown as you type, showing suggested queries, artists/albums, and directly playable tracks
  - **Search** hero with **Top result** frosted card beside top **Songs** and **Playlists & Mixes**
- **Sound Lab & Real-Time Audio DSP**
  - Pure-Rust 5-band parametric Biquad EQ (`60Hz`, `230Hz`, `910Hz`, `3.6kHz`, `12kHz`) + soft limiter
  - Stereo Schroeder Studio Reverb + Playback Speed (`0.75x`–`1.50x`) + 1-click **Slowed + Reverb** and **Nightcore**
  - **Sleep Timer** (`15m` / `30m` / `45m` / `60m`) with smooth fade-out before pausing
- **GPU Canvas Visualizers & Zen Stage**
  - Live 32-band FFT spectrum visualizer (**Bars**, **Wave**, **Halo**, **Off**) + Player Bar **MiniMeter**
  - Reactive album-art ambient glow mesh & **Zen Cinema** mode
- **Fast Native Audio Playback & Endless Auto-Radio**
  - Direct stream extraction + native Rust HTTP/2 chunked streaming with embedded `yt-dlp` fallback
  - **Endless Auto-Radio** & 1-click Radio seeding from any track, plus queue reordering
  - Zero-copy `rodio` + `symphonia` decoding, 300 MB disk LRU cache, and background queue prefetching
  - Dedicated background audio & adaptive tick loop keeping idle CPU near zero
- **Queue, Live Lyrics & Local Library**
  - Full-bleed **Now Playing** stage with synced LRCLIB / YouTube Music active-line lyrics, auto-scroll, and click-to-seek
  - **Liked music**, **custom playlists**, and **Listen again** history stored in `%LOCALAPPDATA%\hum\`
- **Google Account Sync (WebView2)**
  - 1-click **Sign in** opens an embedded `"Sign in to YouTube Music"` window to sync avatar, Home feed, playlists, and likes
- **Single-File Portable Releases & Auto-Updater**
  - Official releases embed `yt-dlp.exe` right inside `hum.exe` with startup & Settings update detection and 1-click self-update

---

## Keyboard Shortcuts

| Shortcut | Action |
| --- | --- |
| `Space` | Play / Pause |
| `Ctrl + K` | Open Spotlight Command Palette |
| `Ctrl + F` | Focus search bar |
| `Ctrl + E` | Toggle Sound Lab (EQ & FX) |
| `Ctrl + L` | Toggle Live Lyrics |
| `Ctrl + Shift + S` | Toggle Slowed + Reverb |
| `Ctrl + Shift + V` | Cycle GPU audio visualizer |
| `Ctrl + Shift + Z` | Toggle Zen Cinema Mode |
| `Ctrl + Left / Right` | Previous / Next track |
| `Shift + Left / Right` | Seek `-10s` / `+10s` |
| `Esc` | Exit Zen Mode or close modal |

---

## Download & Run

1. Download **`hum.exe`** (or `hum-windows-x86_64.zip`) from [**Releases**](https://github.com/Hum-fr/Hum/releases).
2. Double-click `hum.exe` to run—audio and Google sign-in work right out of the box.

All user data (Google session, liked music, playlists, history, settings, audio cache) is stored in `%LOCALAPPDATA%\hum\`, so updating `hum.exe` keeps your library intact.

---

## Building from Source

### Prerequisites

- [Rust toolchain](https://rust-lang.org/tools/install) (`1.88+`)
- Windows MSVC C++ Build Tools (selected automatically during `rustup` installation)

### Run

```bash
cargo run --release
```

To embed `yt-dlp.exe` into your own local build, set `HUM_EMBED_YT_DLP` (or place `yt-dlp.exe` in `vendor/`):

```powershell
$env:HUM_EMBED_YT_DLP = "C:\path\to\yt-dlp.exe"
cargo build --release
```

Otherwise, click **Setup audio** in the top-right bar (or **Install yt-dlp** in **Settings**) on first launch to download `yt-dlp.exe` into `%LOCALAPPDATA%\hum\`.

### Tests

```bash
# Offline unit tests
cargo test

# Live YouTube Music search & audio decode smoke tests
cargo test -- --ignored --nocapture
```

---

## Project Layout

```text
build.rs             # Optional compile-time yt-dlp embedding
src/
├── main.rs          # App entry point & --ytm-signin subprocess dispatch
├── app.rs           # Application state, views, Settings, modals, shortcuts
├── audio.rs         # Dedicated rodio playback thread with fast container hinting
├── auth.rs          # WebView2 YouTube Music session, library sync & local JSON storage
├── webview_login.rs # Embedded WebView2 "Sign in to YouTube Music" popup window
├── ytmusic.rs       # InnerTube browse/search, live suggestions, stream downloader & disk cache
├── updater.rs       # GitHub Releases update checker & 1-click Windows self-updater
├── icons.rs         # Inline SVG vector icons
├── style.rs         # YouTube Music dark theme & widget styling
└── types.rs         # Domain types, settings & formatting helpers
```

---

## License

[MIT](LICENSE)
