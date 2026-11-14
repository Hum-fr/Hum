# hum

A fast, low-overhead desktop client for YouTube Music built in pure Rust.

Most desktop wrappers load the entire `music.youtube.com` web app inside Electron or a browser view, burning 200–400 MB of RAM just to play audio. `hum` uses a native GPU-accelerated `iced` interface, queries YouTube Music directly, and decodes audio natively with `rodio`—keeping idle CPU near zero and memory usage around a few dozen megabytes.

---

## Features

- **YouTube Music Browse & Search**
  - **Home** feed with *Listen again* history, Quick Picks, and curated mixes/albums
  - **Explore** page with Moods & Genres pills, New Releases, and trending shelves
  - **Search** across both individual **Songs** and **Playlists & Mixes**
- **Native Audio Playback**
  - Buffered native playback via `rodio` + `symphonia` (`m4a`/AAC, `mp3`, `vorbis`)
  - Accurate container-decoded track durations and instant seeking
  - Shuffle, Repeat (`Off` / `All` / `One`), volume slider, and quick mute toggle
  - Dedicated background audio thread that sleeps completely when idle or paused
- **Queue & Local Library**
  - **Up Next** split view (large cover art + interactive queue with item removal & clear)
  - **Liked music** and **custom playlists** (create, delete, add/remove tracks) stored locally in `%LOCALAPPDATA%\hum\`
  - **Listen again** history tracking
- **Google Authorization (OAuth 2.0)**
  - Browser-based **Authorize with Google** (OAuth 2.0 PKCE + localhost callback)
  - Syncs your Google YouTube playlists and liked videos via the YouTube Data API v3
- **One-Click Audio Setup**
  - Bundles or downloads `yt-dlp` in one click from the top bar when missing

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

1. Download the latest `hum-windows-x86_64.zip` from **Releases**.
2. Extract the archive (it includes both `hum.exe` and `yt-dlp.exe`).
3. Run `hum.exe`.

---

## Building from Source

### Prerequisites

- [Rust toolchain](https://rust-lang.org/tools/install) (`1.88+`)
- Windows MSVC C++ Build Tools (selected automatically during `rustup` installation)

### Run

```bash
cargo run --release
```

If `yt-dlp` is not already in your `PATH`, click **Setup audio** in the top-right bar on first launch to download `yt-dlp.exe` into `%LOCALAPPDATA%\hum\`.

### Tests

```bash
# Offline unit tests
cargo test

# Live YouTube Music search & audio decode smoke tests
cargo test -- --ignored --nocapture
```

---

## Google OAuth Setup (Optional)

You can browse, search, play music, like songs, and manage local playlists without signing in.

To enable 1-click **Continue with Google** in your own builds / GitHub Releases:

1. Open [Google Cloud Console](https://console.cloud.google.com/) and create (or select) a project.
2. Enable the **YouTube Data API v3**.
3. Configure the **OAuth consent screen** (External, add test users while in Testing or publish the app).
4. Go to **Credentials → Create Credentials → OAuth client ID** and choose **Desktop app**.
5. Set `HUM_GOOGLE_CLIENT_ID` and `HUM_GOOGLE_CLIENT_SECRET` when building (or add them as GitHub repository secrets for the Release workflow):
   ```powershell
   $env:HUM_GOOGLE_CLIENT_ID = "your-client-id.apps.googleusercontent.com"
   $env:HUM_GOOGLE_CLIENT_SECRET = "your-client-secret"
   cargo build --release
   ```

Once baked into the release binary, end users never see any OAuth IDs—they just click **Sign in → Continue with Google**.

---

## Project Layout

```text
src/
├── main.rs      # Window config and app entry point
├── app.rs       # Application state, views, modals, keyboard shortcuts
├── audio.rs     # Dedicated rodio playback thread and snapshot tracking
├── auth.rs      # Google OAuth 2.0 PKCE + YouTube API sync + local JSON storage
├── ytmusic.rs   # YouTube Music InnerTube browse/search & yt-dlp audio loader
├── icons.rs     # Inline SVG vector icons
├── style.rs     # YouTube Music dark theme & widget styling
└── types.rs     # Shared domain types and formatting helpers
```

---

## License

[MIT](LICENSE)
