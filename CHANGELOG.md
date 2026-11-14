## What's New in v0.1.1

### Highlights
- **Single-file `hum.exe` with embedded `yt-dlp`** — Release builds now bundle `yt-dlp.exe` directly inside `hum.exe` (extracted automatically to `%LOCALAPPDATA%\hum\yt-dlp.exe` on first launch), so you only need to download and run one file.
- **Faster Audio Decoding & Track Switching** — Extracts direct stream URLs and downloads audio over native HTTP/2 in Rust (`yt-dlp -o -` fallback kept automatically), uses magic-byte container sniffing + zero-copy `rodio` decoding to skip redundant probing, adds a 300 MB disk audio cache for instant replays, and supports configurable multi-track queue prefetching.
- **Settings Page** — Added a full **Settings** tab in the sidebar for:
  - Account status, circular profile avatar, manual refresh, and sign out
  - Audio engine (`yt-dlp`) status & 1-click update button
  - Disk audio cache toggle, cached track/size readout, and **Clear cache**
  - Queue prefetch depth selector (`Off` / `1 track` / `2 tracks`)
  - Persistent volume preference
  - Listening history count & **Clear history**
  - Keyboard shortcuts & About info
- **Google Profile Pictures** — Displays your circular Google profile photo in the top header bar, account modal, and Settings page.
- **Built-in WebView2 YouTube Music Sign-In** — 1-click **Sign in** pops an embedded `"Sign in to YouTube Music"` window to sync your personalized Home feed, YouTube Music playlists, and Liked Music without needing any Google Cloud OAuth setup.
