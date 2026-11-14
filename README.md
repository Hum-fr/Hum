# hum

A fast, native desktop client for YouTube Music built in pure Rust.

Most desktop wrappers load the entire `music.youtube.com` web app inside Electron or a browser view, burning 200–400 MB of RAM just to play audio. `hum` uses a native GPU-accelerated `iced` interface, queries YouTube Music directly, and decodes audio natively with `rodio`—keeping idle CPU near zero and memory usage around a few dozen megabytes.

---

## What's New in v0.1.5

- **Dynamic Ambient Obsidian UI Redesign** — Real-time palette extraction from playing album art drives ambient background gradients and dynamic player accents, paired with a custom `hum` soundwave brand mark, frosted glass surfaces, collapsible sidebar, and real `<` / `>` Back & Forward page history.
- **3-Column Quick Picks & Search Hero** — Home Quick Picks now render in a crisp 3-column grid, and Search leads with a **Top result** glass hero card alongside top **Songs**.
- **Full-Bleed Now Playing & Balanced Player Bar** — Dedicated full-bleed Lyrics / Up Next stage with large cover artwork and spacious clickable synced lyrics, plus a balanced 3-section player bar.
- **Sub-Second Native Song Loading & Auto-Updater** — Pure-Rust InnerTube + byte-range HTTP/2 streaming, debounced live search preview dropdown, LRCLIB + YTM synced lyrics, and 1-click self-updater.

---

## Features

- **YouTube Music Browse & Search**
  - **Home** feed with *Listen again* history, Quick Picks, and curated mixes/albums (personalized when signed in)
  - **Explore** page with Moods & Genres pills, New Releases, and trending shelves
  - **Live Search Preview** — Debounced autocomplete dropdown as you type, showing suggested queries, artists/albums, and directly playable tracks
  - **Search** across both individual **Songs** and **Playlists & Mixes**
- **Fast Native Audio Playback**
  - Direct stream extraction + native Rust HTTP/2 download with `yt-dlp` fallback
  - Zero-copy `rodio` + `symphonia` decoding with magic-byte MIME hints (`m4a`/AAC, `mp3`, `vorbis`, `flac`, `wav`)
  - Local disk LRU audio cache (up to 300 MB) and background queue prefetching (`0`–`2` tracks ahead)
  - Shuffle, Repeat (`Off` / `All` / `One`), volume slider, quick mute toggle, and accurate seeking
  - Dedicated background audio thread that sleeps completely when idle or paused
- **Queue, Live Lyrics & Local Library**
  - **Up Next** split view with **Queue** & **Lyrics** tabs, synced active-line highlighting, auto-scroll, and click-to-seek
  - **Liked music**, **custom playlists**, and **Listen again** history stored in `%LOCALAPPDATA%\hum\`
- **Google Account Sync (WebView2)**
  - 1-click **Sign in** opens an embedded `"Sign in to YouTube Music"` window
  - Syncs your Google avatar, account name/email, personalized Home feed, YouTube Music playlists, and Liked Music
- **Settings**
  - Toggle disk audio cache, inspect cached size & clear cache, configure prefetch depth, update `yt-dlp`, clear listening history, and view saved data paths
- **Single-File Portable Releases & Auto-Updater**
  - Official releases embed `yt-dlp.exe` right inside `hum.exe`
  - Checks GitHub Releases on startup (or manually in **Settings → About**), shows an update popup with release notes, and performs 1-click in-place `hum.exe` updates & restart

---

## Keyboard Shortcuts

| Shortcut | Action |
| --- | --- |
| `Space` | Play / Pause |
| `Ctrl + F` or `Ctrl + K` | Focus search bar |
| `Ctrl + Right` | Next track |
| `Ctrl + Left` | Previous track / restart current |
| `Esc` | Close open modal |

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
