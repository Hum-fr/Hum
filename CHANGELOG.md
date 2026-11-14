## What's New in v0.1.2

### Highlights
- **Instant Sub-Second Song Loading (No `yt-dlp` Wait)** — Streams audio directly in pure Rust using InnerTube player extraction (`visitorData` pre-warmed at startup) paired with 1 MiB byte-range HTTP/2 chunks to bypass single-connection throttling—cutting uncached track startup from ~5–10 seconds down to under a second (`yt-dlp` stays as an automatic fallback).
- **Live Search Preview Dropdown** — Shows real-time autocomplete queries, matching artists/albums, and directly playable tracks with thumbnails as you type in the search bar.
- **Auto-Update Popup with Changelog** — Automatically checks GitHub Releases on launch (and via **Settings → About**), displays release notes in a modal, and supports 1-click **Update & restart** for `hum.exe`.
