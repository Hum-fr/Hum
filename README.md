# hum

A fast, native desktop client for YouTube Music built in pure Rust.

Most desktop wrappers load the entire `music.youtube.com` web app inside Electron or a browser view, burning 200–400 MB of RAM just to play audio. `hum` uses a native GPU-accelerated `iced` interface, queries YouTube Music directly, and decodes audio natively with `rodio`—keeping idle CPU near zero and memory usage around a few dozen megabytes.

---

## What's New in v0.1.1

- **Faster Audio Startup & Decoding** — Direct stream URL extraction paired with native HTTP/2 Rust downloads, magic-byte container sniffing (`m4a`/`mp3`/`ogg`/`flac`/`wav`) to skip redundant probes, configurable multi-track queue prefetching, and a local disk audio cache for instant replays.
- **Embedded `yt-dlp`** — GitHub Release builds now bundle `yt-dlp.exe` directly inside `hum.exe` (extracted automatically on first run to `%LOCALAPPDATA%\hum\`). No extra files or manual setup needed.
- **Google Profile Pictures** — Displays your circular Google account avatar in the top header, account modal, and Settings page.
- **Settings Page** — Dedicated **Settings** view in the sidebar for Account management, Audio engine status & one-click `yt-dlp` updates, Disk audio cache toggles & cleanup, Queue prefetch depth (`Off` / `1` / `2` tracks), persistent Volume, Listening history cleanup, and Keyboard shortcuts.
- **Zero-Config Google Sign-In** — Built-in WebView2 **"Sign in to YouTube Music"** popup syncs your personalized Home feed, YouTube Music playlists, and liked songs with no Google Cloud setup required.

---

## Features

- **YouTube Music Browse & Search**
  - **Home** feed with *Listen again* history, Quick Picks, and curated mixes/albums (personalized when signed in)
  - **Explore** page with Moods & Genres pills, New Releases, and trending shelves
  - **Search** across both individual **Songs** and **Playlists & Mixes**
- **Fast Native Audio Playback**
  - Direct stream extraction + native Rust HTTP/2 download with `yt-dlp` fallback
  - Zero-copy `rodio` + `symphonia` decoding with magic-byte MIME hints (`m4a`/AAC, `mp3`, `vorbis`, `flac`, `wav`)
  - Local disk LRU audio cache (up to 300 MB) and background queue prefetching (`0`–`2` tracks ahead)
  - Shuffle, Repeat (`Off` / `All` / `One`), volume slider, quick mute toggle, and accurate seeking
  - Dedicated background audio thread that sleeps completely when idle or paused
- **Queue & Local Library**
  - **Up Next** split view (large cover art + interactive queue with item removal & clear)
  - **Liked music**, **custom playlists**, and **Listen again** history stored in `%LOCALAPPDATA%\hum\`
- **Google Account Sync (WebView2)**
  - 1-click **Sign in** opens an embedded `"Sign in to YouTube Music"` window
  - Syncs your Google avatar, account name/email, personalized Home feed, YouTube Music playlists, and Liked Music
- **Settings**
  - Toggle disk audio cache, inspect cached size & clear cache, configure prefetch depth, update `yt-dlp`, clear listening history, and view saved data paths
- **Single-File Portable Releases**
  - Official releases embed `yt-dlp.exe` right inside `hum.exe`

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
├── ytmusic.rs       # InnerTube browse/search, direct stream downloader & disk cache
├── icons.rs         # Inline SVG vector icons
├── style.rs         # YouTube Music dark theme & widget styling
└── types.rs         # Domain types, settings & formatting helpers
```

---

## License

[MIT](LICENSE)
