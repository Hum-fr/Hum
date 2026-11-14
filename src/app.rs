use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use iced::keyboard::{self, Key, key::Named};
use iced::widget::{
    Id, button, column, container, image, operation, row, scrollable, slider, space, stack, text,
    text_input,
};
use iced::{
    Alignment, Background, Color, ContentFit, Element, Length, Padding, Subscription, Task, Theme,
};

use crate::audio::AudioEngine;
use crate::auth::{self, GoogleSession};
use crate::icons;
use crate::style;
use crate::types::{
    AppSettings, CollectionCard, MoodPill, PlaybackState, PlaylistPage, RepeatMode,
    SearchSuggestion, Shelf, ShelfItem, Track, UserPlaylist, format_time, truncate_text,
};
use crate::updater::{self, UpdateInfo};
use crate::ytmusic::{self, YtmAuth};

const MAX_CACHED_THUMBS: usize = 400;
const MAX_HISTORY: usize = 40;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    Home,
    Explore,
    Search,
    Library,
    UpNext,
    Settings,
    Playlist,
    UserPlaylist(usize),
}

#[derive(Debug, Clone)]
pub enum Message {
    QueryChanged(String),
    SubmitSearch,
    SearchFinished(Result<(Vec<Track>, Vec<CollectionCard>), String>),
    HomeLoaded(Result<Vec<Shelf>, String>),
    ExploreLoaded(Result<Vec<Shelf>, String>),
    GoogleLibraryLoaded {
        playlists: Result<Vec<CollectionCard>, String>,
        liked: Result<Vec<Track>, String>,
    },
    OpenPage(Page),
    OpenBrowse {
        browse_id: String,
        params: Option<String>,
        as_shelves: bool,
    },
    BrowseShelvesLoaded(Result<Vec<Shelf>, String>),
    PlaylistLoaded(Result<PlaylistPage, String>),
    PlayTrackList {
        tracks: Vec<Track>,
        index: usize,
    },
    QueueTrack(Track),
    RemoveFromQueue(usize),
    ClearQueue,
    AudioReady {
        seq: u64,
        track: Track,
        result: Result<Vec<u8>, String>,
    },
    PrefetchReady {
        video_id: String,
        result: Result<Vec<u8>, String>,
    },
    ThumbLoaded {
        url: String,
        bytes: Vec<u8>,
    },
    ThumbFailed(String),
    ToggleLike(Track),
    TogglePause,
    ToggleShuffle,
    CycleRepeat,
    ToggleMute,
    NextTrack,
    PrevTrack,
    SeekPreview(f32),
    SeekCommit,
    VolumeChanged(f32),
    InstallYtDlp,
    YtDlpInstalled(Result<String, String>),

    // Google / Account
    OpenSignIn,
    CloseSignIn,
    StartGoogleAuth,
    GoogleAuthFinished(Result<GoogleSession, String>),
    AccountProfileRefreshed(Option<GoogleSession>),
    RefreshLibrary,
    SignOut,

    // Settings
    ToggleDiskCache,
    SetPrefetchAhead(usize),
    ClearAudioCache,
    ClearHistory,

    // Custom playlists
    OpenNewPlaylist,
    NewPlaylistNameChanged(String),
    CreatePlaylist,
    DeleteUserPlaylist(usize),
    OpenAddToPlaylist(Track),
    CloseModals,
    AddTrackToUserPlaylist {
        playlist_idx: usize,
        track: Track,
    },
    RemoveTrackFromUserPlaylist {
        playlist_idx: usize,
        track_idx: usize,
    },

    // Live search suggestions
    SuggestionsLoaded {
        seq: u64,
        result: Result<Vec<SearchSuggestion>, String>,
    },
    PickSuggestionQuery(String),
    PickSuggestionTrack(Track),
    PickSuggestionBrowse(CollectionCard),
    DismissSuggestions,

    // App updates
    CheckForUpdates,
    UpdateChecked {
        manual: bool,
        result: Result<Option<UpdateInfo>, String>,
    },
    StartUpdate,
    UpdateFinished(Result<(), String>),
    OpenUpdatePage,
    DismissUpdateModal,

    KeyEvent(keyboard::Event),
    Tick,
}

pub struct Hum {
    client: reqwest::Client,
    audio: AudioEngine,
    search_id: Id,

    google_session: Option<GoogleSession>,
    show_sign_in: bool,
    authorizing: bool,
    auth_error: Option<String>,
    google_playlists: Vec<CollectionCard>,

    show_new_playlist: bool,
    new_playlist_name: String,
    add_to_playlist_track: Option<Track>,

    page: Page,
    prev_page: Page,
    query: String,
    loading_page: bool,
    installing_helper: bool,
    has_yt_dlp: bool,
    settings: AppSettings,
    cache_stats: (usize, u64),

    home_shelves: Vec<Shelf>,
    explore_shelves: Vec<Shelf>,
    browse_shelves: Vec<Shelf>,
    search_tracks: Vec<Track>,
    search_cards: Vec<CollectionCard>,
    playlist_page: Option<PlaylistPage>,

    liked_tracks: Vec<Track>,
    liked_ids: HashSet<String>,
    user_playlists: Vec<UserPlaylist>,
    history: Vec<Track>,

    queue: Vec<Track>,
    current_index: Option<usize>,
    now_playing: Option<Track>,
    playback: PlaybackState,
    position: Duration,
    audio_duration: Option<Duration>,
    seeking_to: Option<f32>,
    volume: f32,
    muted: bool,
    shuffle: bool,
    repeat: RepeatMode,
    play_seq: u64,
    status_note: Option<String>,
    suggestions: Vec<SearchSuggestion>,
    show_suggestions: bool,
    suggestion_seq: Arc<AtomicU64>,
    available_update: Option<UpdateInfo>,
    show_update_modal: bool,
    checking_update: bool,
    updating_app: bool,
    update_error: Option<String>,

    thumbs: HashMap<String, image::Handle>,
    thumb_order: VecDeque<String>,
    thumb_pending: HashSet<String>,
    prefetched_audio: HashMap<String, Vec<u8>>,
    prefetch_order: VecDeque<String>,
    prefetching: HashSet<String>,
}

impl Hum {
    pub fn boot() -> (Self, Task<Message>) {
        let client = ytmusic::build_http_client();
        let google_session = auth::load_google_session();
        let ytm_auth = google_session.as_ref().and_then(GoogleSession::ytm_auth);
        let settings = auth::load_settings();
        let liked_tracks = auth::load_liked_tracks();
        let liked_ids = liked_tracks.iter().map(|t| t.id.clone()).collect();
        let user_playlists = auth::load_user_playlists();
        let history = auth::load_history();
        let has_yt_dlp = ytmusic::find_yt_dlp().is_some();
        let cache_stats = ytmusic::cache_stats();

        let mut app = Self {
            client: client.clone(),
            audio: AudioEngine::start(settings.default_volume),
            search_id: Id::new("search_bar"),
            google_session: google_session.clone(),
            show_sign_in: false,
            authorizing: false,
            auth_error: None,
            google_playlists: Vec::new(),
            show_new_playlist: false,
            new_playlist_name: String::new(),
            add_to_playlist_track: None,
            page: Page::Home,
            prev_page: Page::Home,
            query: String::new(),
            loading_page: true,
            installing_helper: false,
            has_yt_dlp,
            settings: settings.clone(),
            cache_stats,
            home_shelves: Vec::new(),
            explore_shelves: Vec::new(),
            browse_shelves: Vec::new(),
            search_tracks: Vec::new(),
            search_cards: Vec::new(),
            playlist_page: None,
            liked_tracks,
            liked_ids,
            user_playlists,
            history,
            queue: Vec::new(),
            current_index: None,
            now_playing: None,
            playback: PlaybackState::Stopped,
            position: Duration::ZERO,
            audio_duration: None,
            seeking_to: None,
            volume: settings.default_volume,
            muted: settings.default_volume <= 0.001,
            shuffle: false,
            repeat: RepeatMode::Off,
            play_seq: 0,
            status_note: None,
            suggestions: Vec::new(),
            show_suggestions: false,
            suggestion_seq: Arc::new(AtomicU64::new(0)),
            available_update: None,
            show_update_modal: false,
            checking_update: false,
            updating_app: false,
            update_error: None,
            thumbs: HashMap::new(),
            thumb_order: VecDeque::new(),
            thumb_pending: HashSet::new(),
            prefetched_audio: HashMap::new(),
            prefetch_order: VecDeque::new(),
            prefetching: HashSet::new(),
        };

        let home_client = client.clone();
        let mut tasks = vec![Task::perform(
            async move {
                ytmusic::fetch_browse_shelves(&home_client, "FEmusic_home", None, ytm_auth.as_ref())
                    .await
            },
            Message::HomeLoaded,
        )];

        if let Some(session) = &google_session {
            tasks.push(app.sync_google_library());
            if session.avatar_url.is_none() && session.ytm_auth().is_some() {
                let profile_client = client.clone();
                let profile_session = session.clone();
                tasks.push(Task::perform(
                    async move { auth::refresh_account_profile(&profile_client, profile_session).await },
                    Message::AccountProfileRefreshed,
                ));
            }
        }

        let mut boot_thumbs: Vec<String> = app
            .history
            .iter()
            .filter_map(|t| t.thumb_url.clone())
            .collect();
        if let Some(avatar) = google_session.as_ref().and_then(|s| s.avatar_url.clone()) {
            boot_thumbs.push(avatar);
        }
        tasks.push(app.fetch_thumbs(boot_thumbs));

        let warm_client = client.clone();
        tasks.push(Task::perform(
            async move { ytmusic::warm_visitor_data(&warm_client).await },
            |()| Message::Tick,
        ));

        let update_client = client.clone();
        tasks.push(Task::perform(
            async move { updater::check_for_updates(&update_client).await },
            |result| Message::UpdateChecked {
                manual: false,
                result,
            },
        ));

        (app, Task::batch(tasks))
    }

    pub fn title(&self) -> String {
        match &self.now_playing {
            Some(track) if self.playback == PlaybackState::Playing => {
                format!("{} — {} | hum", track.title, track.artist)
            }
            _ => String::from("hum — Music"),
        }
    }

    pub fn subscription(&self) -> Subscription<Message> {
        let keys = keyboard::listen().map(Message::KeyEvent);
        match self.playback {
            PlaybackState::Playing | PlaybackState::Loading => Subscription::batch([
                keys,
                iced::time::every(Duration::from_millis(250)).map(|_| Message::Tick),
            ]),
            PlaybackState::Paused | PlaybackState::Stopped => keys,
        }
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::QueryChanged(q) => {
                self.query = q;
                let seq = self.suggestion_seq.fetch_add(1, Ordering::SeqCst) + 1;
                let trimmed = self.query.trim().to_string();
                if trimmed.chars().count() < 2 {
                    self.suggestions.clear();
                    self.show_suggestions = false;
                    return Task::none();
                }

                let client = self.client.clone();
                let auth = self.current_ytm_auth();
                let seq_guard = self.suggestion_seq.clone();
                Task::perform(
                    async move {
                        tokio::time::sleep(Duration::from_millis(150)).await;
                        if seq_guard.load(Ordering::SeqCst) != seq {
                            return Ok(Vec::new());
                        }
                        ytmusic::fetch_search_suggestions(&client, &trimmed, auth.as_ref()).await
                    },
                    move |result| Message::SuggestionsLoaded { seq, result },
                )
            }
            Message::SubmitSearch => {
                let query = self.query.trim().to_string();
                self.run_search(query)
            }
            Message::SearchFinished(Ok((tracks, cards))) => {
                self.loading_page = false;
                let mut urls: Vec<String> = tracks.iter().filter_map(|t| t.thumb_url.clone()).collect();
                urls.extend(cards.iter().filter_map(|c| c.thumb_url.clone()));
                self.search_tracks = tracks;
                self.search_cards = cards;
                self.fetch_thumbs(urls)
            }
            Message::SearchFinished(Err(err)) => {
                self.loading_page = false;
                self.status_note = Some(err);
                Task::none()
            }
            Message::HomeLoaded(Ok(shelves)) => {
                self.loading_page = false;
                let urls = collect_shelf_thumbs(&shelves);
                self.home_shelves = shelves;
                self.fetch_thumbs(urls)
            }
            Message::HomeLoaded(Err(err)) => {
                self.loading_page = false;
                self.status_note = Some(err);
                Task::none()
            }
            Message::ExploreLoaded(Ok(shelves)) => {
                self.loading_page = false;
                let urls = collect_shelf_thumbs(&shelves);
                self.explore_shelves = shelves;
                self.fetch_thumbs(urls)
            }
            Message::ExploreLoaded(Err(err)) => {
                self.loading_page = false;
                self.status_note = Some(err);
                Task::none()
            }
            Message::GoogleLibraryLoaded { playlists, liked } => {
                let mut urls = Vec::new();
                if let Ok(cards) = playlists {
                    urls.extend(cards.iter().filter_map(|c| c.thumb_url.clone()));
                    self.google_playlists = cards;
                }
                if let Ok(tracks) = liked {
                    for track in tracks {
                        if let Some(u) = &track.thumb_url {
                            urls.push(u.clone());
                        }
                        if self.liked_ids.insert(track.id.clone()) {
                            self.liked_tracks.push(track);
                        }
                    }
                    auth::save_liked_tracks(&self.liked_tracks);
                }
                self.fetch_thumbs(urls)
            }
            Message::OpenPage(page) => {
                self.hide_suggestions();
                if !matches!(self.page, Page::Playlist | Page::UpNext) {
                    self.prev_page = self.page;
                }
                self.page = page;
                match page {
                    Page::Home if self.home_shelves.is_empty() => {
                        self.loading_page = true;
                        let client = self.client.clone();
                        let auth = self.current_ytm_auth();
                        Task::perform(
                            async move {
                                ytmusic::fetch_browse_shelves(
                                    &client,
                                    "FEmusic_home",
                                    None,
                                    auth.as_ref(),
                                )
                                .await
                            },
                            Message::HomeLoaded,
                        )
                    }
                    Page::Explore if self.explore_shelves.is_empty() => {
                        self.loading_page = true;
                        let client = self.client.clone();
                        let auth = self.current_ytm_auth();
                        Task::perform(
                            async move {
                                ytmusic::fetch_browse_shelves(
                                    &client,
                                    "FEmusic_explore",
                                    None,
                                    auth.as_ref(),
                                )
                                .await
                            },
                            Message::ExploreLoaded,
                        )
                    }
                    Page::Settings => {
                        self.cache_stats = ytmusic::cache_stats();
                        Task::none()
                    }
                    _ => Task::none(),
                }
            }
            Message::OpenBrowse {
                browse_id,
                params,
                as_shelves,
            } => self.open_browse(browse_id, params, as_shelves),
            Message::BrowseShelvesLoaded(Ok(shelves)) => {
                self.loading_page = false;
                let urls = collect_shelf_thumbs(&shelves);
                self.browse_shelves = shelves;
                self.fetch_thumbs(urls)
            }
            Message::BrowseShelvesLoaded(Err(err)) => {
                self.loading_page = false;
                self.status_note = Some(err);
                Task::none()
            }
            Message::PlaylistLoaded(Ok(page)) => {
                self.loading_page = false;
                let mut urls: Vec<String> =
                    page.tracks.iter().filter_map(|t| t.thumb_url.clone()).collect();
                if let Some(u) = &page.thumb_url {
                    urls.push(u.clone());
                }
                self.playlist_page = Some(page);
                self.fetch_thumbs(urls)
            }
            Message::PlaylistLoaded(Err(err)) => {
                self.loading_page = false;
                self.status_note = Some(err);
                Task::none()
            }
            Message::PlayTrackList { mut tracks, index } => {
                if index >= tracks.len() {
                    return Task::none();
                }
                if self.shuffle && tracks.len() > 2 {
                    let chosen = tracks.remove(index);
                    shuffle_slice(&mut tracks);
                    tracks.insert(0, chosen);
                    self.queue = tracks;
                    return self.start_track(0);
                }
                self.queue = tracks;
                self.start_track(index)
            }
            Message::QueueTrack(track) => {
                self.queue.push(track.clone());
                self.status_note = Some(format!("Added \"{}\" to queue", truncate_text(&track.title, 28)));
                Task::none()
            }
            Message::RemoveFromQueue(idx) => {
                if idx < self.queue.len() {
                    self.queue.remove(idx);
                    if let Some(cur) = self.current_index {
                        if idx < cur {
                            self.current_index = Some(cur - 1);
                        } else if idx == cur {
                            self.current_index = None;
                        }
                    }
                }
                Task::none()
            }
            Message::ClearQueue => {
                if let Some(cur) = self.now_playing.clone() {
                    self.queue = vec![cur];
                    self.current_index = Some(0);
                } else {
                    self.queue.clear();
                    self.current_index = None;
                }
                Task::none()
            }
            Message::AudioReady { seq, track, result } => {
                if seq != self.play_seq {
                    return Task::none();
                }

                match result {
                    Ok(bytes) => {
                        self.audio_duration = track.duration_secs.map(Duration::from_secs);
                        self.now_playing = Some(track);
                        self.playback = PlaybackState::Loading;
                        self.position = Duration::ZERO;
                        self.status_note = None;
                        self.audio.play_bytes(seq, bytes);
                        return self.prefetch_next_track();
                    }
                    Err(err) => {
                        self.playback = PlaybackState::Stopped;
                        self.status_note = Some(err);
                    }
                }
                Task::none()
            }
            Message::PrefetchReady { video_id, result } => {
                self.prefetching.remove(&video_id);
                if let Ok(bytes) = result
                    && !self.prefetched_audio.contains_key(&video_id)
                {
                    if self.prefetch_order.len() >= 3
                        && let Some(oldest) = self.prefetch_order.pop_front()
                    {
                        self.prefetched_audio.remove(&oldest);
                    }
                    self.prefetched_audio.insert(video_id.clone(), bytes);
                    self.prefetch_order.push_back(video_id);
                }
                Task::none()
            }
            Message::ThumbLoaded { url, bytes } => {
                self.thumb_pending.remove(&url);
                if !self.thumbs.contains_key(&url) {
                    if self.thumb_order.len() >= MAX_CACHED_THUMBS
                        && let Some(oldest) = self.thumb_order.pop_front()
                    {
                        self.thumbs.remove(&oldest);
                    }
                    self.thumbs.insert(url.clone(), image::Handle::from_bytes(bytes));
                    self.thumb_order.push_back(url);
                }
                Task::none()
            }
            Message::ThumbFailed(url) => {
                self.thumb_pending.remove(&url);
                Task::none()
            }
            Message::ToggleLike(track) => {
                if self.liked_ids.contains(&track.id) {
                    self.liked_ids.remove(&track.id);
                    self.liked_tracks.retain(|t| t.id != track.id);
                } else {
                    self.liked_ids.insert(track.id.clone());
                    self.liked_tracks.insert(0, track);
                }
                auth::save_liked_tracks(&self.liked_tracks);
                Task::none()
            }
            Message::TogglePause => {
                if self.now_playing.is_some() {
                    self.audio.toggle_pause();
                    self.playback = match self.playback {
                        PlaybackState::Playing => PlaybackState::Paused,
                        PlaybackState::Paused => PlaybackState::Playing,
                        other => other,
                    };
                }
                Task::none()
            }
            Message::ToggleShuffle => {
                self.shuffle = !self.shuffle;
                if self.shuffle && self.queue.len() > 2
                    && let Some(cur) = self.current_index
                    && cur + 1 < self.queue.len()
                {
                    shuffle_slice(&mut self.queue[cur + 1..]);
                }
                Task::none()
            }
            Message::CycleRepeat => {
                self.repeat = self.repeat.next();
                Task::none()
            }
            Message::ToggleMute => {
                self.muted = !self.muted;
                self.audio.set_volume(if self.muted { 0.0 } else { self.volume });
                Task::none()
            }
            Message::NextTrack => self.advance_next(true),
            Message::PrevTrack => {
                if self.position.as_secs() > 3 {
                    self.audio.seek(Duration::ZERO);
                    self.position = Duration::ZERO;
                    return Task::none();
                }
                if let Some(idx) = self.current_index
                    && idx > 0
                {
                    return self.start_track(idx - 1);
                }
                Task::none()
            }
            Message::SeekPreview(secs) => {
                self.seeking_to = Some(secs);
                Task::none()
            }
            Message::SeekCommit => {
                if let Some(secs) = self.seeking_to.take() {
                    let pos = Duration::from_secs_f32(secs.max(0.0));
                    self.audio.seek(pos);
                    self.position = pos;
                }
                Task::none()
            }
            Message::VolumeChanged(vol) => {
                self.volume = vol;
                self.muted = vol <= 0.001;
                self.audio.set_volume(vol);
                self.settings.default_volume = vol;
                auth::save_settings(&self.settings);
                Task::none()
            }
            Message::InstallYtDlp => {
                if self.installing_helper {
                    return Task::none();
                }
                self.installing_helper = true;
                self.status_note = Some("Downloading yt-dlp...".into());
                let client = self.client.clone();
                Task::perform(
                    async move {
                        ytmusic::install_yt_dlp(&client)
                            .await
                            .map(|p| p.display().to_string())
                    },
                    Message::YtDlpInstalled,
                )
            }
            Message::YtDlpInstalled(Ok(_)) => {
                self.installing_helper = false;
                self.has_yt_dlp = true;
                self.status_note = Some("Audio helper ready.".into());
                Task::none()
            }
            Message::YtDlpInstalled(Err(err)) => {
                self.installing_helper = false;
                self.status_note = Some(format!("Setup failed: {err}"));
                Task::none()
            }
            Message::OpenSignIn => {
                self.show_sign_in = true;
                self.auth_error = None;
                if self.google_session.is_none() && !self.authorizing {
                    return self.start_sign_in();
                }
                Task::none()
            }
            Message::CloseSignIn => {
                self.show_sign_in = false;
                self.auth_error = None;
                Task::none()
            }
            Message::StartGoogleAuth => self.start_sign_in(),
            Message::GoogleAuthFinished(Ok(session)) => {
                self.authorizing = false;
                self.show_sign_in = false;
                self.status_note = Some(format!("Signed in as {}", session.user_name));
                let avatar = session.avatar_url.clone().into_iter().collect();
                self.google_session = Some(session);
                Task::batch([
                    self.sync_google_library(),
                    self.reload_home(),
                    self.fetch_thumbs(avatar),
                ])
            }
            Message::GoogleAuthFinished(Err(err)) => {
                self.authorizing = false;
                self.auth_error = Some(err);
                Task::none()
            }
            Message::AccountProfileRefreshed(Some(session)) => {
                let avatar = session.avatar_url.clone().into_iter().collect();
                self.google_session = Some(session);
                self.fetch_thumbs(avatar)
            }
            Message::AccountProfileRefreshed(None) => Task::none(),
            Message::RefreshLibrary => {
                self.status_note = Some("Refreshing library...".into());
                Task::batch([self.sync_google_library(), self.reload_home()])
            }
            Message::SignOut => {
                auth::clear_google_session();
                self.google_session = None;
                self.google_playlists.clear();
                self.show_sign_in = false;
                self.status_note = Some("Signed out.".into());
                self.reload_home()
            }
            Message::ToggleDiskCache => {
                self.settings.disk_audio_cache = !self.settings.disk_audio_cache;
                auth::save_settings(&self.settings);
                Task::none()
            }
            Message::SetPrefetchAhead(count) => {
                self.settings.prefetch_ahead = count.min(2);
                auth::save_settings(&self.settings);
                Task::none()
            }
            Message::ClearAudioCache => {
                match ytmusic::clear_audio_cache() {
                    Ok(()) => {
                        self.cache_stats = (0, 0);
                        self.prefetched_audio.clear();
                        self.prefetch_order.clear();
                        self.status_note = Some("Audio cache cleared.".into());
                    }
                    Err(e) => self.status_note = Some(e),
                }
                Task::none()
            }
            Message::ClearHistory => {
                self.history.clear();
                auth::save_history(&self.history);
                self.status_note = Some("Listening history cleared.".into());
                Task::none()
            }
            Message::OpenNewPlaylist => {
                self.show_new_playlist = true;
                self.new_playlist_name.clear();
                Task::none()
            }
            Message::NewPlaylistNameChanged(s) => {
                self.new_playlist_name = s;
                Task::none()
            }
            Message::CreatePlaylist => {
                let name = self.new_playlist_name.trim().to_string();
                if !name.is_empty() {
                    let id = format!("local_{}", self.user_playlists.len() + 1);
                    let mut tracks = Vec::new();
                    if let Some(pending) = self.add_to_playlist_track.take() {
                        tracks.push(pending);
                    }
                    self.user_playlists.push(UserPlaylist { id, name, tracks });
                    auth::save_user_playlists(&self.user_playlists);
                    self.page = Page::UserPlaylist(self.user_playlists.len() - 1);
                }
                self.show_new_playlist = false;
                self.new_playlist_name.clear();
                Task::none()
            }
            Message::DeleteUserPlaylist(idx) => {
                if idx < self.user_playlists.len() {
                    self.user_playlists.remove(idx);
                    auth::save_user_playlists(&self.user_playlists);
                    self.page = Page::Library;
                }
                Task::none()
            }
            Message::OpenAddToPlaylist(track) => {
                self.add_to_playlist_track = Some(track);
                Task::none()
            }
            Message::CloseModals => {
                self.show_sign_in = false;
                self.show_new_playlist = false;
                self.add_to_playlist_track = None;
                self.show_update_modal = false;
                self.hide_suggestions();
                Task::none()
            }
            Message::AddTrackToUserPlaylist {
                playlist_idx,
                track,
            } => {
                if let Some(pl) = self.user_playlists.get_mut(playlist_idx)
                    && !pl.tracks.iter().any(|t| t.id == track.id)
                {
                    let pl_name = pl.name.clone();
                    pl.tracks.push(track.clone());
                    auth::save_user_playlists(&self.user_playlists);
                    self.status_note = Some(format!(
                        "Added to {}",
                        truncate_text(&pl_name, 22)
                    ));
                }
                self.add_to_playlist_track = None;
                Task::none()
            }
            Message::RemoveTrackFromUserPlaylist {
                playlist_idx,
                track_idx,
            } => {
                if let Some(pl) = self.user_playlists.get_mut(playlist_idx)
                    && track_idx < pl.tracks.len()
                {
                    pl.tracks.remove(track_idx);
                    auth::save_user_playlists(&self.user_playlists);
                }
                Task::none()
            }
            Message::KeyEvent(event) => {
                if let keyboard::Event::KeyPressed { key, modifiers, .. } = event {
                    match key {
                        Key::Named(Named::Escape) => {
                            return self.update(Message::CloseModals);
                        }
                        Key::Named(Named::Space)
                            if !self.show_sign_in
                                && !self.show_new_playlist
                                && !self.show_update_modal
                                && !self.show_suggestions
                                && self.add_to_playlist_track.is_none() =>
                        {
                            return self.update(Message::TogglePause);
                        }
                        Key::Character(ref ch)
                            if modifiers.command() && (ch == "f" || ch == "k") =>
                        {
                            return operation::focus(self.search_id.clone());
                        }
                        Key::Named(Named::ArrowRight) if modifiers.command() => {
                            return self.update(Message::NextTrack);
                        }
                        Key::Named(Named::ArrowLeft) if modifiers.command() => {
                            return self.update(Message::PrevTrack);
                        }
                        _ => {}
                    }
                }
                Task::none()
            }
            Message::SuggestionsLoaded { seq, result } => {
                if seq != self.suggestion_seq.load(Ordering::SeqCst)
                    || self.query.trim().chars().count() < 2
                {
                    return Task::none();
                }
                match result {
                    Ok(items) => {
                        let mut urls = Vec::new();
                        for item in &items {
                            match item {
                                SearchSuggestion::Track(t) => {
                                    if let Some(u) = &t.thumb_url {
                                        urls.push(u.clone());
                                    }
                                }
                                SearchSuggestion::Browse(c) => {
                                    if let Some(u) = &c.thumb_url {
                                        urls.push(u.clone());
                                    }
                                }
                                SearchSuggestion::Query(_) => {}
                            }
                        }
                        self.show_suggestions = !items.is_empty();
                        self.suggestions = items;
                        self.fetch_thumbs(urls)
                    }
                    Err(_) => Task::none(),
                }
            }
            Message::PickSuggestionQuery(query) => {
                self.query = query.clone();
                self.run_search(query)
            }
            Message::PickSuggestionTrack(track) => {
                self.hide_suggestions();
                self.queue = vec![track];
                self.start_track(0)
            }
            Message::PickSuggestionBrowse(card) => {
                let as_shelves = card.browse_id.starts_with("UC");
                self.open_browse(card.browse_id, card.params, as_shelves)
            }
            Message::DismissSuggestions => {
                self.hide_suggestions();
                Task::none()
            }
            Message::CheckForUpdates => {
                if self.checking_update {
                    return Task::none();
                }
                self.checking_update = true;
                self.update_error = None;
                let client = self.client.clone();
                Task::perform(
                    async move { updater::check_for_updates(&client).await },
                    |result| Message::UpdateChecked {
                        manual: true,
                        result,
                    },
                )
            }
            Message::UpdateChecked { manual, result } => {
                self.checking_update = false;
                match result {
                    Ok(Some(info)) => {
                        self.available_update = Some(info);
                        self.show_update_modal = true;
                        self.update_error = None;
                    }
                    Ok(None) => {
                        self.available_update = None;
                        if manual {
                            self.status_note = Some(format!(
                                "Up to date (v{})",
                                env!("CARGO_PKG_VERSION")
                            ));
                        }
                    }
                    Err(err) => {
                        if manual {
                            self.status_note = Some(err);
                        }
                    }
                }
                Task::none()
            }
            Message::StartUpdate => {
                let Some(url) = self
                    .available_update
                    .as_ref()
                    .and_then(|u| u.exe_download_url.clone())
                else {
                    return Task::none();
                };
                if self.updating_app {
                    return Task::none();
                }
                self.updating_app = true;
                self.update_error = None;
                let client = self.client.clone();
                Task::perform(
                    async move { updater::download_and_replace_exe(&client, &url).await },
                    Message::UpdateFinished,
                )
            }
            Message::UpdateFinished(Ok(())) => {
                std::process::exit(0);
            }
            Message::UpdateFinished(Err(err)) => {
                self.updating_app = false;
                self.update_error = Some(err);
                Task::none()
            }
            Message::OpenUpdatePage => {
                if let Some(info) = &self.available_update {
                    updater::open_url(&info.html_url);
                }
                Task::none()
            }
            Message::DismissUpdateModal => {
                self.show_update_modal = false;
                Task::none()
            }
            Message::Tick => {
                let snap = self.audio.take_snapshot();
                if snap.seq != self.play_seq {
                    return Task::none();
                }
                if let Some(err) = snap.error {
                    self.status_note = Some(err);
                }

                self.playback = snap.state;
                if let Some(dur) = snap.duration {
                    self.audio_duration = Some(dur);
                    if let Some(track) = &mut self.now_playing
                        && track.duration_secs.is_none()
                    {
                        let secs = dur.as_secs();
                        track.duration_secs = Some(secs);
                        track.duration_label = format_time(secs);
                    }
                }
                if self.seeking_to.is_none() {
                    self.position = snap.position;
                }

                if snap.finished_track {
                    return self.advance_next(false);
                }

                Task::none()
            }
        }
    }

    fn hide_suggestions(&mut self) {
        self.suggestion_seq.fetch_add(1, Ordering::SeqCst);
        self.show_suggestions = false;
        self.suggestions.clear();
    }

    fn run_search(&mut self, query: String) -> Task<Message> {
        self.hide_suggestions();
        let query = query.trim().to_string();
        if query.is_empty() {
            return Task::none();
        }
        self.loading_page = true;
        self.page = Page::Search;
        self.status_note = None;

        let client = self.client.clone();
        let auth = self.current_ytm_auth();
        Task::perform(
            async move { ytmusic::search_all(&client, &query, auth.as_ref()).await },
            Message::SearchFinished,
        )
    }

    fn open_browse(
        &mut self,
        browse_id: String,
        params: Option<String>,
        as_shelves: bool,
    ) -> Task<Message> {
        self.hide_suggestions();
        if !matches!(self.page, Page::Playlist) {
            self.prev_page = self.page;
        }
        self.loading_page = true;
        self.page = Page::Playlist;
        self.playlist_page = None;
        self.browse_shelves.clear();

        let client = self.client.clone();
        let auth = self.current_ytm_auth();
        if as_shelves {
            Task::perform(
                async move {
                    ytmusic::fetch_browse_shelves(&client, &browse_id, params.as_deref(), auth.as_ref())
                        .await
                },
                Message::BrowseShelvesLoaded,
            )
        } else {
            Task::perform(
                async move {
                    ytmusic::fetch_playlist(&client, &browse_id, params.as_deref(), auth.as_ref())
                        .await
                },
                Message::PlaylistLoaded,
            )
        }
    }

    fn advance_next(&mut self, manual: bool) -> Task<Message> {
        let Some(idx) = self.current_index else {
            return Task::none();
        };

        if !manual && self.repeat == RepeatMode::One {
            return self.start_track(idx);
        }

        if idx + 1 < self.queue.len() {
            return self.start_track(idx + 1);
        }

        if self.repeat == RepeatMode::All && !self.queue.is_empty() {
            return self.start_track(0);
        }

        self.playback = PlaybackState::Stopped;
        self.position = Duration::ZERO;
        Task::none()
    }

    fn current_ytm_auth(&self) -> Option<YtmAuth> {
        self.google_session.as_ref().and_then(GoogleSession::ytm_auth)
    }

    fn start_sign_in(&mut self) -> Task<Message> {
        if self.authorizing {
            return Task::none();
        }
        self.authorizing = true;
        self.auth_error = None;
        let client = self.client.clone();
        Task::perform(
            async move { auth::sign_in_with_webview(&client).await },
            Message::GoogleAuthFinished,
        )
    }

    fn reload_home(&mut self) -> Task<Message> {
        let client = self.client.clone();
        let auth = self.current_ytm_auth();
        Task::perform(
            async move {
                ytmusic::fetch_browse_shelves(&client, "FEmusic_home", None, auth.as_ref()).await
            },
            Message::HomeLoaded,
        )
    }

    fn sync_google_library(&mut self) -> Task<Message> {
        let Some(mut session) = self.google_session.clone() else {
            return Task::none();
        };
        let client = self.client.clone();
        Task::perform(
            async move {
                if let Some(ytm_auth) = session.ytm_auth() {
                    return tokio::join!(
                        auth::fetch_ytm_playlists(&client, &ytm_auth),
                        auth::fetch_ytm_liked_tracks(&client, &ytm_auth),
                    );
                }
                let token = match auth::ensure_valid_token(&client, &mut session).await {
                    Ok(t) => t,
                    Err(e) => {
                        return (Err(e.clone()), Err(e));
                    }
                };
                tokio::join!(
                    auth::fetch_google_playlists(&client, &token),
                    auth::fetch_google_liked_tracks(&client, &token),
                )
            },
            |(playlists, liked)| Message::GoogleLibraryLoaded { playlists, liked },
        )
    }

    fn fetch_thumbs(&mut self, urls: Vec<String>) -> Task<Message> {
        let mut tasks = Vec::new();
        for url in urls {
            if self.thumbs.contains_key(&url) || !self.thumb_pending.insert(url.clone()) {
                continue;
            }
            let client = self.client.clone();
            tasks.push(Task::perform(
                async move {
                    let res = ytmusic::download_thumb_bytes(&client, &url).await;
                    (url, res)
                },
                |(url, res)| match res {
                    Ok(bytes) => Message::ThumbLoaded { url, bytes },
                    Err(_) => Message::ThumbFailed(url),
                },
            ));
        }
        Task::batch(tasks)
    }

    fn start_track(&mut self, idx: usize) -> Task<Message> {
        let Some(track) = self.queue.get(idx).cloned() else {
            return Task::none();
        };

        self.audio.stop();
        self.play_seq += 1;
        let seq = self.play_seq;

        self.current_index = Some(idx);
        self.now_playing = Some(track.clone());
        self.playback = PlaybackState::Loading;
        self.position = Duration::ZERO;
        self.audio_duration = track.duration_secs.map(Duration::from_secs);
        self.seeking_to = None;
        self.status_note = None;

        // Record in Recently Played
        self.history.retain(|t| t.id != track.id);
        self.history.insert(0, track.clone());
        self.history.truncate(MAX_HISTORY);
        auth::save_history(&self.history);

        if let Some(bytes) = self.prefetched_audio.remove(&track.id) {
            self.now_playing = Some(track);
            self.playback = PlaybackState::Loading;
            self.audio.play_bytes(seq, bytes);
            return self.prefetch_next_track();
        }

        let client = self.client.clone();
        let video_id = track.id.clone();
        let use_cache = self.settings.disk_audio_cache;
        Task::perform(
            async move { ytmusic::fetch_track_audio(&client, &video_id, use_cache).await },
            move |result| Message::AudioReady { seq, track, result },
        )
    }

    fn prefetch_next_track(&mut self) -> Task<Message> {
        self.cache_stats = ytmusic::cache_stats();
        let ahead = self.settings.prefetch_ahead.min(2);
        if ahead == 0 {
            return Task::none();
        }
        let Some(idx) = self.current_index else {
            return Task::none();
        };

        let mut tasks = Vec::new();
        for offset in 1..=ahead {
            let next_idx = if idx + offset < self.queue.len() {
                idx + offset
            } else if self.repeat == RepeatMode::All && !self.queue.is_empty() {
                (idx + offset) % self.queue.len()
            } else {
                continue;
            };
            let next_id = self.queue[next_idx].id.clone();
            if self.prefetched_audio.contains_key(&next_id)
                || !self.prefetching.insert(next_id.clone())
            {
                continue;
            }
            let client = self.client.clone();
            let use_cache = self.settings.disk_audio_cache;
            tasks.push(Task::perform(
                async move {
                    let res = ytmusic::fetch_track_audio(&client, &next_id, use_cache).await;
                    (next_id, res)
                },
                |(video_id, result)| Message::PrefetchReady { video_id, result },
            ));
        }

        Task::batch(tasks)
    }

    pub fn view(&self) -> Element<'_, Message> {
        let shell = column![
            row![self.view_sidebar(), self.view_main_column()]
                .width(Length::Fill)
                .height(Length::Fill),
            self.view_player_bar(),
        ]
        .width(Length::Fill)
        .height(Length::Fill);

        let base: Element<'_, Message> =
            if self.show_suggestions && !self.suggestions.is_empty() {
                let dropdown = container(self.view_search_suggestions())
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .padding(Padding {
                        top: 56.0,
                        right: 24.0,
                        bottom: 24.0,
                        left: 240.0,
                    });
                stack![shell, dropdown].into()
            } else {
                shell.into()
            };

        if self.show_update_modal
            && let Some(info) = &self.available_update
        {
            stack![base, self.view_update_modal(info)].into()
        } else if self.show_sign_in {
            stack![base, self.view_sign_in_modal()].into()
        } else if self.show_new_playlist {
            stack![base, self.view_new_playlist_modal()].into()
        } else if let Some(track) = &self.add_to_playlist_track {
            stack![base, self.view_add_to_playlist_modal(track)].into()
        } else {
            base
        }
    }

    fn view_sidebar(&self) -> Element<'_, Message> {
        let brand = row![
            container(icons::play(13.0, style::TEXT_PRIMARY))
                .width(26)
                .height(26)
                .center_x(26)
                .center_y(26)
                .style(style::brand_circle),
            text("Music").size(18).color(style::TEXT_PRIMARY),
        ]
        .spacing(10)
        .align_y(Alignment::Center)
        .padding([16, 18]);

        let nav = column![
            self.nav_button(
                "Home",
                Page::Home,
                icons::home(18.0, icon_color(self.page == Page::Home)).into()
            ),
            self.nav_button(
                "Explore",
                Page::Explore,
                icons::explore(18.0, icon_color(self.page == Page::Explore)).into()
            ),
            self.nav_button(
                "Library",
                Page::Library,
                icons::library(18.0, icon_color(self.page == Page::Library)).into()
            ),
            self.nav_button(
                "Up Next",
                Page::UpNext,
                icons::queue(18.0, icon_color(self.page == Page::UpNext)).into()
            ),
        ]
        .spacing(4)
        .padding([4, 10]);

        let divider = container(space::horizontal().width(Length::Fill))
            .height(1)
            .style(style::divider);

        let new_pl_btn = button(
            row![
                icons::plus(15.0, style::TEXT_PRIMARY),
                text("New playlist").size(13),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
        )
        .on_press(Message::OpenNewPlaylist)
        .padding([8, 12])
        .width(Length::Fill)
        .style(style::pill_button(false));

        let mut playlists_col = column![
            button(
                column![
                    text("Liked music").size(13).color(style::TEXT_PRIMARY),
                    text(format!("{} songs", self.liked_tracks.len()))
                        .size(11)
                        .color(style::TEXT_MUTED),
                ]
                .spacing(2),
            )
            .on_press(Message::OpenPage(Page::Library))
            .padding([8, 10])
            .width(Length::Fill)
            .style(style::nav_item(self.page == Page::Library)),
        ]
        .spacing(4);

        for (i, pl) in self.user_playlists.iter().enumerate() {
            let active = self.page == Page::UserPlaylist(i);
            playlists_col = playlists_col.push(
                button(
                    column![
                        text(truncate_text(&pl.name, 20))
                            .size(13)
                            .color(style::TEXT_PRIMARY),
                        text(format!("{} songs", pl.tracks.len()))
                            .size(11)
                            .color(style::TEXT_MUTED),
                    ]
                    .spacing(2),
                )
                .on_press(Message::OpenPage(Page::UserPlaylist(i)))
                .padding([8, 10])
                .width(Length::Fill)
                .style(style::nav_item(active)),
            );
        }

        let account_label = match &self.google_session {
            Some(s) => format!("Google: {}", truncate_text(&s.user_name, 16)),
            None => String::from("Guest mode"),
        };

        let settings_btn = container(
            self.nav_button(
                "Settings",
                Page::Settings,
                icons::settings(18.0, icon_color(self.page == Page::Settings)).into(),
            ),
        )
        .padding([4, 10]);

        container(
            column![
                brand,
                nav,
                container(divider).padding([12, 16]),
                container(new_pl_btn).padding([0, 12]),
                scrollable(playlists_col.padding([8, 10]))
                    .width(Length::Fill)
                    .height(Length::Fill),
                settings_btn,
                container(text(account_label).size(11).color(style::TEXT_MUTED))
                    .padding([6, 18]),
            ]
            .width(216)
            .height(Length::Fill),
        )
        .style(style::sidebar)
        .into()
    }

    fn view_main_column(&self) -> Element<'_, Message> {
        let header = self.view_header();
        let body = match self.page {
            Page::Home => self.view_home(),
            Page::Explore => self.view_shelves(&self.explore_shelves, "Loading Explore..."),
            Page::Search => self.view_search_results(),
            Page::Library => self.view_library(),
            Page::UpNext => self.view_up_next(),
            Page::Settings => self.view_settings(),
            Page::Playlist => self.view_playlist_or_browse(),
            Page::UserPlaylist(idx) => self.view_user_playlist(idx),
        };

        column![header, body]
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn view_header(&self) -> Element<'_, Message> {
        let search_box = container(
            row![
                icons::search(16.0, style::TEXT_MUTED),
                text_input("Search songs, albums, artists  (Ctrl+F)", &self.query)
                    .id(self.search_id.clone())
                    .on_input(Message::QueryChanged)
                    .on_submit(Message::SubmitSearch)
                    .padding([8, 8])
                    .size(14)
                    .style(style::search_input)
                    .width(Length::Fill),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
        )
        .max_width(460)
        .width(Length::Fill);

        let mut right_actions = row![].spacing(10).align_y(Alignment::Center);

        if let Some(note) = &self.status_note {
            right_actions = right_actions.push(
                text(truncate_text(note, 38))
                    .size(12)
                    .color(style::TEXT_SECONDARY),
            );
        }

        if !self.has_yt_dlp {
            let label = if self.installing_helper {
                "Installing..."
            } else {
                "Setup audio"
            };
            right_actions = right_actions.push(
                button(text(label).size(12))
                    .on_press(Message::InstallYtDlp)
                    .padding([7, 14])
                    .style(style::pill_button(false)),
            );
        }

        if let Some(session) = &self.google_session {
            right_actions = right_actions.push(
                button(
                    row![
                        self.render_avatar(session.avatar_url.as_deref(), 20.0, style::TEXT_PRIMARY),
                        text(truncate_text(&session.user_name, 16)).size(12),
                    ]
                    .spacing(8)
                    .align_y(Alignment::Center),
                )
                .on_press(Message::OpenSignIn)
                .padding([5, 12])
                .style(style::pill_button(false)),
            );
        } else {
            right_actions = right_actions.push(
                button(
                    row![
                        icons::user(14.0, style::BG_CANVAS),
                        text("Sign in").size(12),
                    ]
                    .spacing(6)
                    .align_y(Alignment::Center),
                )
                .on_press(Message::OpenSignIn)
                .padding([7, 14])
                .style(style::pill_button(true)),
            );
        }

        container(
            row![search_box, space::horizontal().width(Length::Fill), right_actions]
                .spacing(16)
                .align_y(Alignment::Center),
        )
        .padding([12, 24])
        .width(Length::Fill)
        .style(style::header_bar)
        .into()
    }

    fn view_home(&self) -> Element<'_, Message> {
        if self.loading_page && self.home_shelves.is_empty() && self.history.is_empty() {
            return container(
                text("Loading YouTube Music...")
                    .size(14)
                    .color(style::TEXT_SECONDARY),
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into();
        }

        let mut col = column![].spacing(28).padding([20, 26]);

        if !self.history.is_empty() {
            let mut recent_row = row![].spacing(14);
            let history_slice: Vec<Track> = self.history.iter().take(12).cloned().collect();
            for (i, track) in history_slice.iter().enumerate() {
                let thumb = self.render_thumb(track.thumb_url.as_deref(), 144.0, 8.0);
                recent_row = recent_row.push(
                    button(
                        column![
                            thumb,
                            text(truncate_text(&track.title, 20))
                                .size(13)
                                .color(style::TEXT_PRIMARY),
                            text(truncate_text(&track.artist, 22))
                                .size(12)
                                .color(style::TEXT_SECONDARY),
                        ]
                        .spacing(5)
                        .width(144),
                    )
                    .on_press(Message::PlayTrackList {
                        tracks: history_slice.clone(),
                        index: i,
                    })
                    .padding(8)
                    .style(style::card_button),
                );
            }

            col = col.push(
                column![
                    text("Listen again").size(20).color(style::TEXT_PRIMARY),
                    scrollable(recent_row).horizontal(),
                ]
                .spacing(12),
            );
        }

        for shelf in &self.home_shelves {
            col = col.push(self.view_shelf(shelf));
        }

        scrollable(col)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn view_shelves<'a>(
        &'a self,
        shelves: &'a [Shelf],
        loading_label: &'a str,
    ) -> Element<'a, Message> {
        if self.loading_page && shelves.is_empty() {
            return container(text(loading_label).size(14).color(style::TEXT_SECONDARY))
                .width(Length::Fill)
                .height(Length::Fill)
                .center_x(Length::Fill)
                .center_y(Length::Fill)
                .into();
        }

        if shelves.is_empty() {
            return container(
                text("Nothing loaded yet. Try searching above or check your connection.")
                    .size(14)
                    .color(style::TEXT_MUTED),
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into();
        }

        let mut col = column![].spacing(28).padding([20, 26]);
        for shelf in shelves {
            col = col.push(self.view_shelf(shelf));
        }

        scrollable(col)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn view_shelf<'a>(&'a self, shelf: &'a Shelf) -> Element<'a, Message> {
        let mut cards_row = row![].spacing(14);
        let mut moods_row = row![].spacing(10);
        let mut song_refs: Vec<&Track> = Vec::new();

        for item in &shelf.items {
            match item {
                ShelfItem::Card(card) => {
                    cards_row = cards_row.push(self.view_collection_card(card));
                }
                ShelfItem::Mood(mood) => {
                    moods_row = moods_row.push(self.view_mood_pill(mood));
                }
                ShelfItem::Song(track) => {
                    song_refs.push(track);
                }
            }
        }

        let mut section =
            column![text(&shelf.title).size(20).color(style::TEXT_PRIMARY)].spacing(12);

        if !song_refs.is_empty() {
            let queue_copy: Vec<Track> = song_refs.iter().map(|t| (*t).clone()).collect();
            let mut rows = column![].spacing(4);
            for (i, track) in song_refs.iter().enumerate() {
                let is_active =
                    self.now_playing.as_ref().map(|t| t.id.as_str()) == Some(track.id.as_str());
                rows = rows.push(self.view_track_row(
                    track,
                    is_active,
                    Message::PlayTrackList {
                        tracks: queue_copy.clone(),
                        index: i,
                    },
                    None,
                ));
            }
            section = section.push(rows);
        } else if shelf.items.iter().any(|i| matches!(i, ShelfItem::Mood(_))) {
            section = section.push(scrollable(moods_row).horizontal());
        } else {
            section = section.push(scrollable(cards_row).horizontal());
        }

        section.into()
    }

    fn view_mood_pill<'a>(&'a self, mood: &'a MoodPill) -> Element<'a, Message> {
        button(text(truncate_text(&mood.title, 22)).size(13))
            .on_press(Message::OpenBrowse {
                browse_id: mood.browse_id.clone(),
                params: mood.params.clone(),
                as_shelves: true,
            })
            .padding([12, 18])
            .style(style::mood_pill(mood.accent))
            .into()
    }

    fn view_collection_card<'a>(&'a self, card: &'a CollectionCard) -> Element<'a, Message> {
        let cover = self.render_thumb(card.thumb_url.as_deref(), 156.0, 8.0);

        let content = column![
            cover,
            text(truncate_text(&card.title, 22))
                .size(13)
                .color(style::TEXT_PRIMARY),
            text(truncate_text(&card.subtitle, 24))
                .size(12)
                .color(style::TEXT_SECONDARY),
        ]
        .spacing(5)
        .width(156);

        button(content)
            .on_press(Message::OpenBrowse {
                browse_id: card.browse_id.clone(),
                params: card.params.clone(),
                as_shelves: false,
            })
            .padding(8)
            .style(style::card_button)
            .into()
    }

    fn view_search_results(&self) -> Element<'_, Message> {
        if self.loading_page {
            return container(text("Searching...").size(14).color(style::TEXT_SECONDARY))
                .width(Length::Fill)
                .height(Length::Fill)
                .center_x(Length::Fill)
                .center_y(Length::Fill)
                .into();
        }

        if self.search_tracks.is_empty() && self.search_cards.is_empty() {
            return container(
                column![
                    text("No results").size(16).color(style::TEXT_PRIMARY),
                    text("Try searching another song, artist, or playlist.")
                        .size(13)
                        .color(style::TEXT_MUTED),
                ]
                .spacing(6)
                .align_x(Alignment::Center),
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into();
        }

        let mut col = column![].spacing(24).padding([20, 26]);

        if !self.search_cards.is_empty() {
            let mut cards = row![].spacing(14);
            for card in &self.search_cards {
                cards = cards.push(self.view_collection_card(card));
            }
            col = col.push(
                column![
                    text("Playlists & mixes").size(20).color(style::TEXT_PRIMARY),
                    scrollable(cards).horizontal(),
                ]
                .spacing(12),
            );
        }

        if !self.search_tracks.is_empty() {
            let current_id = self.now_playing.as_ref().map(|t| t.id.as_str());
            let mut rows = column![text("Songs").size(20).color(style::TEXT_PRIMARY)].spacing(6);
            for (i, track) in self.search_tracks.iter().enumerate() {
                let is_active = current_id == Some(track.id.as_str());
                rows = rows.push(self.view_track_row(
                    track,
                    is_active,
                    Message::PlayTrackList {
                        tracks: self.search_tracks.clone(),
                        index: i,
                    },
                    None,
                ));
            }
            col = col.push(rows);
        }

        scrollable(col)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn view_library(&self) -> Element<'_, Message> {
        let mut col = column![].spacing(24).padding([20, 26]);

        if !self.google_playlists.is_empty() {
            let mut cards = row![].spacing(14);
            for card in &self.google_playlists {
                cards = cards.push(self.view_collection_card(card));
            }
            col = col.push(
                column![
                    text("Your Google playlists").size(20).color(style::TEXT_PRIMARY),
                    scrollable(cards).horizontal(),
                ]
                .spacing(12),
            );
        }

        if !self.user_playlists.is_empty() {
            let mut pl_row = row![].spacing(12);
            for (i, pl) in self.user_playlists.iter().enumerate() {
                let thumb_url = pl.tracks.first().and_then(|t| t.thumb_url.as_deref());
                pl_row = pl_row.push(
                    button(
                        column![
                            self.render_thumb(thumb_url, 144.0, 8.0),
                            text(truncate_text(&pl.name, 20))
                                .size(13)
                                .color(style::TEXT_PRIMARY),
                            text(format!("{} songs", pl.tracks.len()))
                                .size(12)
                                .color(style::TEXT_SECONDARY),
                        ]
                        .spacing(5)
                        .width(144),
                    )
                    .on_press(Message::OpenPage(Page::UserPlaylist(i)))
                    .padding(8)
                    .style(style::card_button),
                );
            }
            col = col.push(
                column![
                    text("Your playlists").size(20).color(style::TEXT_PRIMARY),
                    scrollable(pl_row).horizontal(),
                ]
                .spacing(12),
            );
        }

        col = col.push(
            row![
                text("Liked music").size(20).color(style::TEXT_PRIMARY),
                space::horizontal().width(Length::Fill),
                if !self.liked_tracks.is_empty() {
                    Element::from(
                        button(text("Play all").size(12))
                            .on_press(Message::PlayTrackList {
                                tracks: self.liked_tracks.clone(),
                                index: 0,
                            })
                            .padding([6, 14])
                            .style(style::pill_button(true)),
                    )
                } else {
                    space::horizontal().width(0).into()
                },
            ]
            .align_y(Alignment::Center),
        );

        if self.liked_tracks.is_empty() {
            col = col.push(
                text("Tap the heart icon on any song to save it here, or authorize with Google to sync your likes.")
                    .size(13)
                    .color(style::TEXT_MUTED),
            );
        } else {
            let mut rows = column![].spacing(4);
            for (i, track) in self.liked_tracks.iter().enumerate() {
                let is_active =
                    self.now_playing.as_ref().map(|t| t.id.as_str()) == Some(track.id.as_str());
                rows = rows.push(self.view_track_row(
                    track,
                    is_active,
                    Message::PlayTrackList {
                        tracks: self.liked_tracks.clone(),
                        index: i,
                    },
                    None,
                ));
            }
            col = col.push(rows);
        }

        scrollable(col)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn view_user_playlist(&self, idx: usize) -> Element<'_, Message> {
        let Some(pl) = self.user_playlists.get(idx) else {
            return container(text("Playlist not found").size(14).color(style::TEXT_MUTED))
                .width(Length::Fill)
                .height(Length::Fill)
                .center_x(Length::Fill)
                .center_y(Length::Fill)
                .into();
        };

        let thumb_url = pl.tracks.first().and_then(|t| t.thumb_url.as_deref());
        let header = row![
            self.render_thumb(thumb_url, 150.0, 8.0),
            column![
                text(&pl.name).size(26).color(style::TEXT_PRIMARY),
                text(format!("{} songs", pl.tracks.len()))
                    .size(13)
                    .color(style::TEXT_SECONDARY),
                space::vertical().height(6),
                row![
                    button(
                        row![
                            icons::play(15.0, style::BG_CANVAS),
                            text("Play all").size(13),
                        ]
                        .spacing(6)
                        .align_y(Alignment::Center),
                    )
                    .on_press(Message::PlayTrackList {
                        tracks: pl.tracks.clone(),
                        index: 0,
                    })
                    .padding([8, 18])
                    .style(style::pill_button(true)),
                    button(text("Delete playlist").size(12))
                        .on_press(Message::DeleteUserPlaylist(idx))
                        .padding([8, 14])
                        .style(style::pill_button(false)),
                ]
                .spacing(10),
            ]
            .spacing(8),
        ]
        .spacing(22)
        .align_y(Alignment::Center);

        let mut rows = column![].spacing(4);
        if pl.tracks.is_empty() {
            rows = rows.push(
                text("No songs in this playlist yet — click + on any song to add it.")
                    .size(13)
                    .color(style::TEXT_MUTED),
            );
        } else {
            for (i, track) in pl.tracks.iter().enumerate() {
                let is_active =
                    self.now_playing.as_ref().map(|t| t.id.as_str()) == Some(track.id.as_str());
                rows = rows.push(self.view_track_row(
                    track,
                    is_active,
                    Message::PlayTrackList {
                        tracks: pl.tracks.clone(),
                        index: i,
                    },
                    Some(Message::RemoveTrackFromUserPlaylist {
                        playlist_idx: idx,
                        track_idx: i,
                    }),
                ));
            }
        }

        scrollable(column![header, rows].spacing(20).padding([20, 26]))
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn view_playlist_or_browse(&self) -> Element<'_, Message> {
        if self.loading_page {
            return container(text("Loading...").size(14).color(style::TEXT_SECONDARY))
                .width(Length::Fill)
                .height(Length::Fill)
                .center_x(Length::Fill)
                .center_y(Length::Fill)
                .into();
        }

        let back_btn = button(
            row![
                icons::back(16.0, style::TEXT_PRIMARY),
                text("Back").size(13),
            ]
            .spacing(6)
            .align_y(Alignment::Center),
        )
        .on_press(Message::OpenPage(self.prev_page))
        .padding([6, 12])
        .style(style::pill_button(false));

        if !self.browse_shelves.is_empty() {
            let mut col = column![back_btn].spacing(24).padding([18, 26]);
            for shelf in &self.browse_shelves {
                col = col.push(self.view_shelf(shelf));
            }
            return scrollable(col)
                .width(Length::Fill)
                .height(Length::Fill)
                .into();
        }

        let Some(page) = &self.playlist_page else {
            return container(
                column![
                    back_btn,
                    text("Could not load playlist.").size(14).color(style::TEXT_MUTED),
                ]
                .spacing(16),
            )
            .padding(24)
            .width(Length::Fill)
            .height(Length::Fill)
            .into();
        };

        let header = row![
            self.render_thumb(page.thumb_url.as_deref(), 160.0, 8.0),
            column![
                text(truncate_text(&page.title, 44))
                    .size(24)
                    .color(style::TEXT_PRIMARY),
                text(truncate_text(&page.subtitle, 60))
                    .size(13)
                    .color(style::TEXT_SECONDARY),
                space::vertical().height(8),
                button(
                    row![
                        icons::play(15.0, style::BG_CANVAS),
                        text("Play all").size(13),
                    ]
                    .spacing(6)
                    .align_y(Alignment::Center),
                )
                .on_press(Message::PlayTrackList {
                    tracks: page.tracks.clone(),
                    index: 0,
                })
                .padding([9, 18])
                .style(style::pill_button(true)),
            ]
            .spacing(8),
        ]
        .spacing(22)
        .align_y(Alignment::Center);

        let mut rows = column![].spacing(4);
        for (i, track) in page.tracks.iter().enumerate() {
            let is_active =
                self.now_playing.as_ref().map(|t| t.id.as_str()) == Some(track.id.as_str());
            rows = rows.push(self.view_track_row(
                track,
                is_active,
                Message::PlayTrackList {
                    tracks: page.tracks.clone(),
                    index: i,
                },
                None,
            ));
        }

        scrollable(
            column![back_btn, header, rows]
                .spacing(20)
                .padding([18, 26]),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }

    fn view_up_next(&self) -> Element<'_, Message> {
        let left_panel: Element<'_, Message> = if let Some(track) = &self.now_playing {
            let liked = self.liked_ids.contains(&track.id);
            container(
                column![
                    self.render_thumb(track.thumb_url.as_deref(), 290.0, 10.0),
                    text(truncate_text(&track.title, 36))
                        .size(20)
                        .color(style::TEXT_PRIMARY),
                    text(truncate_text(&track.artist, 40))
                        .size(14)
                        .color(style::TEXT_SECONDARY),
                    row![
                        button(icons::heart(
                            18.0,
                            if liked { style::YT_RED } else { style::TEXT_SECONDARY },
                            liked
                        ))
                        .on_press(Message::ToggleLike(track.clone()))
                        .padding(8)
                        .style(style::icon_button),
                        button(icons::plus(18.0, style::TEXT_SECONDARY))
                            .on_press(Message::OpenAddToPlaylist(track.clone()))
                            .padding(8)
                            .style(style::icon_button),
                    ]
                    .spacing(8),
                ]
                .spacing(10)
                .align_x(Alignment::Center),
            )
            .width(Length::FillPortion(5))
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into()
        } else {
            container(
                text("Pick a song to start listening")
                    .size(15)
                    .color(style::TEXT_MUTED),
            )
            .width(Length::FillPortion(5))
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into()
        };

        let mut queue_rows = column![
            row![
                text(format!("Up next ({})", self.queue.len()))
                    .size(16)
                    .color(style::TEXT_PRIMARY),
                space::horizontal().width(Length::Fill),
                button(text("Clear").size(12))
                    .on_press(Message::ClearQueue)
                    .padding([5, 12])
                    .style(style::pill_button(false)),
            ]
            .align_y(Alignment::Center)
        ]
        .spacing(8);

        for (i, track) in self.queue.iter().enumerate() {
            let is_active = self.current_index == Some(i);
            queue_rows = queue_rows.push(self.view_track_row(
                track,
                is_active,
                Message::PlayTrackList {
                    tracks: self.queue.clone(),
                    index: i,
                },
                Some(Message::RemoveFromQueue(i)),
            ));
        }

        let right_panel = scrollable(queue_rows.padding([20, 20]))
            .width(Length::FillPortion(5))
            .height(Length::Fill);

        row![left_panel, right_panel]
            .spacing(16)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn view_settings(&self) -> Element<'_, Message> {
        let account_card = if let Some(session) = &self.google_session {
            let email_line = session
                .user_email
                .as_deref()
                .unwrap_or("Signed in to YouTube Music");
            container(
                row![
                    self.render_avatar(session.avatar_url.as_deref(), 44.0, style::TEXT_PRIMARY),
                    column![
                        text(&session.user_name).size(15).color(style::TEXT_PRIMARY),
                        text(email_line).size(12).color(style::TEXT_MUTED),
                    ]
                    .spacing(3)
                    .width(Length::Fill),
                    button(text("Refresh").size(12))
                        .on_press(Message::RefreshLibrary)
                        .padding([7, 14])
                        .style(style::pill_button(false)),
                    button(text("Sign out").size(12))
                        .on_press(Message::SignOut)
                        .padding([7, 14])
                        .style(style::pill_button(true)),
                ]
                .spacing(14)
                .align_y(Alignment::Center),
            )
            .padding(18)
            .width(Length::Fill)
            .style(style::modal_card)
        } else {
            container(
                row![
                    self.render_avatar(None, 40.0, style::TEXT_MUTED),
                    column![
                        text("Not signed in").size(15).color(style::TEXT_PRIMARY),
                        text("Sign in with Google to sync your YouTube Music feed, playlists, and likes.")
                            .size(12)
                            .color(style::TEXT_MUTED),
                    ]
                    .spacing(3)
                    .width(Length::Fill),
                    button(text("Sign in").size(12))
                        .on_press(Message::OpenSignIn)
                        .padding([7, 16])
                        .style(style::pill_button(true)),
                ]
                .spacing(14)
                .align_y(Alignment::Center),
            )
            .padding(18)
            .width(Length::Fill)
            .style(style::modal_card)
        };

        let engine_status = if ytmusic::has_embedded_yt_dlp() {
            "Embedded in hum.exe (ready)"
        } else if self.has_yt_dlp {
            "Ready"
        } else {
            "Not installed"
        };
        let update_label = if self.installing_helper {
            "Updating..."
        } else if self.has_yt_dlp {
            "Update yt-dlp"
        } else {
            "Install yt-dlp"
        };

        let (cache_count, cache_bytes) = self.cache_stats;
        let cache_mb = (cache_bytes as f64) / (1024.0 * 1024.0);
        let cache_summary = if self.settings.disk_audio_cache {
            format!("Enabled  •  {cache_count} tracks ({cache_mb:.1} MB cached)")
        } else {
            String::from("Disabled  •  streams are not saved to disk")
        };

        let prefetch_pills = row![
            button(text("Off").size(12))
                .on_press(Message::SetPrefetchAhead(0))
                .padding([6, 12])
                .style(style::pill_button(self.settings.prefetch_ahead == 0)),
            button(text("1 track").size(12))
                .on_press(Message::SetPrefetchAhead(1))
                .padding([6, 12])
                .style(style::pill_button(self.settings.prefetch_ahead == 1)),
            button(text("2 tracks").size(12))
                .on_press(Message::SetPrefetchAhead(2))
                .padding([6, 12])
                .style(style::pill_button(self.settings.prefetch_ahead >= 2)),
        ]
        .spacing(8);

        let playback_card = container(
            column![
                text("Audio & Playback").size(15).color(style::TEXT_PRIMARY),
                row![
                    column![
                        text("Audio engine (yt-dlp)").size(13).color(style::TEXT_PRIMARY),
                        text(engine_status).size(12).color(style::TEXT_MUTED),
                    ]
                    .spacing(2)
                    .width(Length::Fill),
                    button(text(update_label).size(12))
                        .on_press(Message::InstallYtDlp)
                        .padding([7, 14])
                        .style(style::pill_button(false)),
                ]
                .align_y(Alignment::Center),
                row![
                    column![
                        text("Disk audio cache").size(13).color(style::TEXT_PRIMARY),
                        text(cache_summary).size(12).color(style::TEXT_MUTED),
                    ]
                    .spacing(2)
                    .width(Length::Fill),
                    button(text(if self.settings.disk_audio_cache { "Turn off" } else { "Turn on" }).size(12))
                        .on_press(Message::ToggleDiskCache)
                        .padding([7, 14])
                        .style(style::pill_button(self.settings.disk_audio_cache)),
                    button(text("Clear cache").size(12))
                        .on_press(Message::ClearAudioCache)
                        .padding([7, 14])
                        .style(style::pill_button(false)),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
                row![
                    column![
                        text("Prefetch next tracks in queue").size(13).color(style::TEXT_PRIMARY),
                        text("Preloads upcoming tracks in the background for instant skipping.")
                            .size(12)
                            .color(style::TEXT_MUTED),
                    ]
                    .spacing(2)
                    .width(Length::Fill),
                    prefetch_pills,
                ]
                .align_y(Alignment::Center),
                row![
                    text(format!("Volume: {}%", (self.volume * 100.0).round() as u32))
                        .size(13)
                        .color(style::TEXT_PRIMARY)
                        .width(Length::Fill),
                    slider(0.0..=1.0, self.volume, Message::VolumeChanged)
                        .step(0.01_f32)
                        .width(180)
                        .style(style::volume_slider),
                ]
                .align_y(Alignment::Center),
            ]
            .spacing(16),
        )
        .padding(18)
        .width(Length::Fill)
        .style(style::modal_card);

        let data_dir = auth::app_data_dir()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| String::from("%LOCALAPPDATA%\\hum"));

        let data_card = container(
            column![
                text("Library & Data").size(15).color(style::TEXT_PRIMARY),
                row![
                    column![
                        text("Listening history").size(13).color(style::TEXT_PRIMARY),
                        text(format!("{} recently played tracks", self.history.len()))
                            .size(12)
                            .color(style::TEXT_MUTED),
                    ]
                    .spacing(2)
                    .width(Length::Fill),
                    button(text("Clear history").size(12))
                        .on_press(Message::ClearHistory)
                        .padding([7, 14])
                        .style(style::pill_button(false)),
                ]
                .align_y(Alignment::Center),
                column![
                    text("App data folder").size(13).color(style::TEXT_PRIMARY),
                    text(data_dir).size(12).color(style::TEXT_MUTED),
                ]
                .spacing(2),
            ]
            .spacing(14),
        )
        .padding(18)
        .width(Length::Fill)
        .style(style::modal_card);

        let shortcuts_card = container(
            column![
                text("Keyboard Shortcuts").size(15).color(style::TEXT_PRIMARY),
                row![text("Space").size(12).color(style::TEXT_PRIMARY).width(140), text("Play / Pause").size(12).color(style::TEXT_MUTED)],
                row![text("Ctrl+F / Ctrl+K").size(12).color(style::TEXT_PRIMARY).width(140), text("Focus search bar").size(12).color(style::TEXT_MUTED)],
                row![text("Ctrl+Right").size(12).color(style::TEXT_PRIMARY).width(140), text("Next track").size(12).color(style::TEXT_MUTED)],
                row![text("Ctrl+Left").size(12).color(style::TEXT_PRIMARY).width(140), text("Previous track / restart").size(12).color(style::TEXT_MUTED)],
                row![text("Esc").size(12).color(style::TEXT_PRIMARY).width(140), text("Close open modal").size(12).color(style::TEXT_MUTED)],
            ]
            .spacing(8),
        )
        .padding(18)
        .width(Length::Fill)
        .style(style::modal_card);

        let check_label = if self.checking_update {
            "Checking..."
        } else {
            "Check for updates"
        };
        let mut about_actions = row![
            button(text(check_label).size(12))
                .on_press_maybe((!self.checking_update).then_some(Message::CheckForUpdates))
                .padding([7, 14])
                .style(style::pill_button(false)),
        ]
        .spacing(10)
        .align_y(Alignment::Center);

        if let Some(info) = &self.available_update {
            about_actions = about_actions.push(
                button(text(format!("Update v{} ready", info.version)).size(12))
                    .on_press(Message::OpenUpdatePage)
                    .padding([7, 14])
                    .style(style::pill_button(true)),
            );
        }

        let about_card = container(
            column![
                text("About").size(15).color(style::TEXT_PRIMARY),
                text(format!(
                    "hum v{}  •  Native YouTube Music desktop client (MIT License)",
                    env!("CARGO_PKG_VERSION")
                ))
                .size(12)
                .color(style::TEXT_MUTED),
                text("github.com/Hum-fr/Hum  •  hum-fr.github.io")
                    .size(12)
                    .color(style::TEXT_MUTED),
                space::vertical().height(4),
                about_actions,
            ]
            .spacing(6),
        )
        .padding(18)
        .width(Length::Fill)
        .style(style::modal_card);

        let content = column![
            text("Settings").size(26).color(style::TEXT_PRIMARY),
            account_card,
            playback_card,
            data_card,
            shortcuts_card,
            about_card,
        ]
        .spacing(16)
        .max_width(820)
        .padding([24, 28]);

        scrollable(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn view_track_row<'a>(
        &'a self,
        track: &'a Track,
        is_active: bool,
        on_play: Message,
        on_remove: Option<Message>,
    ) -> Element<'a, Message> {
        let thumb = self.render_thumb(track.thumb_url.as_deref(), 44.0, 4.0);
        let subline_raw = match &track.album {
            Some(alb) => format!("{}  •  {}", track.artist, alb),
            None => track.artist.clone(),
        };

        let title_color = if is_active {
            style::YT_RED
        } else {
            style::TEXT_PRIMARY
        };

        let liked = self.liked_ids.contains(&track.id);
        let like_btn = button(icons::heart(
            15.0,
            if liked { style::YT_RED } else { style::TEXT_MUTED },
            liked,
        ))
        .on_press(Message::ToggleLike(track.clone()))
        .padding(6)
        .style(style::icon_button);

        let queue_btn = button(icons::list_plus(15.0, style::TEXT_MUTED))
            .on_press(Message::QueueTrack(track.clone()))
            .padding(6)
            .style(style::icon_button);

        let add_btn = button(icons::plus(15.0, style::TEXT_MUTED))
            .on_press(Message::OpenAddToPlaylist(track.clone()))
            .padding(6)
            .style(style::icon_button);

        let main_btn = button(
            row![
                thumb,
                column![
                    text(truncate_text(&track.title, 48))
                        .size(14)
                        .color(title_color),
                    text(truncate_text(&subline_raw, 56))
                        .size(12)
                        .color(style::TEXT_SECONDARY),
                ]
                .spacing(2)
                .width(Length::Fill),
                text(&track.duration_label).size(12).color(style::TEXT_MUTED),
            ]
            .spacing(12)
            .align_y(Alignment::Center),
        )
        .on_press(on_play)
        .padding([7, 10])
        .width(Length::Fill)
        .style(style::track_row(is_active));

        let mut row_out = row![main_btn, queue_btn, add_btn, like_btn]
            .spacing(4)
            .align_y(Alignment::Center);

        if let Some(rm_msg) = on_remove {
            row_out = row_out.push(
                button(icons::trash(14.0, style::TEXT_MUTED))
                    .on_press(rm_msg)
                    .padding(6)
                    .style(style::icon_button),
            );
        }

        row_out.into()
    }

    fn view_player_bar(&self) -> Element<'_, Message> {
        let total_secs = self
            .audio_duration
            .map(|d| d.as_secs())
            .or_else(|| self.now_playing.as_ref().and_then(|t| t.duration_secs))
            .unwrap_or(1)
            .max(1) as f32;

        let current_secs = self
            .seeking_to
            .unwrap_or_else(|| self.position.as_secs_f32())
            .min(total_secs);

        let progress = slider(0.0..=total_secs, current_secs, Message::SeekPreview)
            .on_release(Message::SeekCommit)
            .step(1.0_f32)
            .style(style::yt_progress_slider)
            .width(Length::Fill);

        let play_icon = match self.playback {
            PlaybackState::Playing => icons::pause(18.0, style::BG_CANVAS),
            PlaybackState::Loading => icons::note(16.0, style::BG_CANVAS),
            PlaybackState::Paused | PlaybackState::Stopped => icons::play(18.0, style::BG_CANVAS),
        };

        let left_controls = row![
            button(icons::prev(18.0, style::TEXT_PRIMARY))
                .on_press(Message::PrevTrack)
                .padding(8)
                .style(style::icon_button),
            button(play_icon)
                .on_press(Message::TogglePause)
                .width(38)
                .height(38)
                .padding(10)
                .style(style::play_circle),
            button(icons::next(18.0, style::TEXT_PRIMARY))
                .on_press(Message::NextTrack)
                .padding(8)
                .style(style::icon_button),
            text(format!(
                "{} / {}",
                format_time(current_secs as u64),
                format_time(total_secs as u64)
            ))
            .size(12)
            .color(style::TEXT_SECONDARY),
        ]
        .spacing(8)
        .align_y(Alignment::Center)
        .width(Length::FillPortion(3));

        let center_track: Element<'_, Message> = if let Some(track) = &self.now_playing {
            let liked = self.liked_ids.contains(&track.id);
            row![
                self.render_thumb(track.thumb_url.as_deref(), 46.0, 4.0),
                column![
                    text(truncate_text(&track.title, 34))
                        .size(14)
                        .color(style::TEXT_PRIMARY),
                    text(truncate_text(&track.artist, 34))
                        .size(12)
                        .color(style::TEXT_SECONDARY),
                ]
                .spacing(2),
                button(icons::heart(
                    17.0,
                    if liked { style::YT_RED } else { style::TEXT_SECONDARY },
                    liked,
                ))
                .on_press(Message::ToggleLike(track.clone()))
                .padding(8)
                .style(style::icon_button),
            ]
            .spacing(12)
            .align_y(Alignment::Center)
            .width(Length::FillPortion(4))
            .into()
        } else {
            row![text("Nothing playing").size(13).color(style::TEXT_MUTED)]
                .align_y(Alignment::Center)
                .width(Length::FillPortion(4))
                .into()
        };

        let vol_icon = if self.muted || self.volume <= 0.001 {
            icons::volume_mute(17.0, style::TEXT_SECONDARY)
        } else {
            icons::volume(17.0, style::TEXT_SECONDARY)
        };

        let repeat_color = match self.repeat {
            RepeatMode::Off => style::TEXT_MUTED,
            RepeatMode::All | RepeatMode::One => style::TEXT_PRIMARY,
        };

        let right_controls = row![
            space::horizontal().width(Length::Fill),
            button(icons::shuffle(
                16.0,
                if self.shuffle {
                    style::TEXT_PRIMARY
                } else {
                    style::TEXT_MUTED
                }
            ))
            .on_press(Message::ToggleShuffle)
            .padding(7)
            .style(style::icon_button),
            button(icons::repeat(16.0, repeat_color, self.repeat == RepeatMode::One))
                .on_press(Message::CycleRepeat)
                .padding(7)
                .style(style::icon_button),
            button(vol_icon)
                .on_press(Message::ToggleMute)
                .padding(7)
                .style(style::icon_button),
            slider(
                0.0..=1.0,
                if self.muted { 0.0 } else { self.volume },
                Message::VolumeChanged
            )
            .step(0.02_f32)
            .style(style::volume_slider)
            .width(86),
            button(icons::queue(
                18.0,
                if self.page == Page::UpNext {
                    style::TEXT_PRIMARY
                } else {
                    style::TEXT_SECONDARY
                }
            ))
            .on_press(Message::OpenPage(if self.page == Page::UpNext {
                self.prev_page
            } else {
                Page::UpNext
            }))
            .padding(8)
            .style(style::icon_button),
        ]
        .spacing(6)
        .align_y(Alignment::Center)
        .width(Length::FillPortion(3));

        container(
            column![
                progress,
                container(
                    row![left_controls, center_track, right_controls]
                        .spacing(12)
                        .align_y(Alignment::Center),
                )
                .padding([10, 18])
                .width(Length::Fill),
            ]
            .width(Length::Fill),
        )
        .width(Length::Fill)
        .style(style::player_bar)
        .into()
    }

    fn view_sign_in_modal(&self) -> Element<'_, Message> {
        let mut card = column![
            text("Sign in to YouTube Music")
                .size(19)
                .color(style::TEXT_PRIMARY),
            text("Sign in with Google to sync your YouTube Music home feed, playlists, and liked songs.")
                .size(13)
                .color(style::TEXT_SECONDARY),
        ]
        .spacing(12);

        if let Some(session) = &self.google_session {
            card = card.push(
                row![
                    self.render_avatar(session.avatar_url.as_deref(), 40.0, style::TEXT_PRIMARY),
                    column![
                        text(&session.user_name).size(14).color(style::TEXT_PRIMARY),
                        text(
                            session
                                .user_email
                                .as_deref()
                                .unwrap_or("YouTube Music account"),
                        )
                        .size(12)
                        .color(style::TEXT_MUTED),
                    ]
                    .spacing(2),
                ]
                .spacing(12)
                .align_y(Alignment::Center),
            );
            card = card.push(
                row![
                    space::horizontal().width(Length::Fill),
                    button(text("Close").size(13))
                        .on_press(Message::CloseSignIn)
                        .padding([8, 16])
                        .style(style::pill_button(false)),
                    button(text("Sign out").size(13))
                        .on_press(Message::SignOut)
                        .padding([8, 16])
                        .style(style::pill_button(true)),
                ]
                .spacing(10),
            );
        } else {
            if let Some(err) = &self.auth_error {
                card = card.push(text(err).size(12).color(style::YT_RED));
            }

            if self.authorizing {
                card = card.push(
                    text("Finish signing in in the window that opened.")
                        .size(13)
                        .color(style::TEXT_MUTED),
                );
                card = card.push(
                    row![
                        space::horizontal().width(Length::Fill),
                        button(text("Close").size(13))
                            .on_press(Message::CloseSignIn)
                            .padding([8, 16])
                            .style(style::pill_button(false)),
                    ]
                    .spacing(10),
                );
            } else {
                card = card.push(
                    row![
                        space::horizontal().width(Length::Fill),
                        button(text("Cancel").size(13))
                            .on_press(Message::CloseSignIn)
                            .padding([8, 16])
                            .style(style::pill_button(false)),
                        button(text("Sign in with Google").size(13))
                            .on_press(Message::StartGoogleAuth)
                            .padding([8, 18])
                            .style(style::pill_button(true)),
                    ]
                    .spacing(10),
                );
            }
        }

        let modal = container(card)
            .padding(24)
            .max_width(480)
            .style(style::modal_card);

        container(modal)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .style(|_: &Theme| container::Style {
                background: Some(Background::Color(Color::from_rgba(0.0, 0.0, 0.0, 0.72))),
                ..Default::default()
            })
            .into()
    }

    fn view_new_playlist_modal(&self) -> Element<'_, Message> {
        let card = column![
            text("New playlist").size(18).color(style::TEXT_PRIMARY),
            text_input("Playlist name...", &self.new_playlist_name)
                .on_input(Message::NewPlaylistNameChanged)
                .on_submit(Message::CreatePlaylist)
                .padding([10, 12])
                .size(13)
                .style(style::plain_input)
                .width(Length::Fill),
            row![
                space::horizontal().width(Length::Fill),
                button(text("Cancel").size(13))
                    .on_press(Message::CloseModals)
                    .padding([8, 16])
                    .style(style::pill_button(false)),
                button(text("Create").size(13))
                    .on_press(Message::CreatePlaylist)
                    .padding([8, 18])
                    .style(style::pill_button(true)),
            ]
            .spacing(10),
        ]
        .spacing(14);

        let modal = container(card)
            .padding(24)
            .max_width(400)
            .style(style::modal_card);

        container(modal)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .style(|_: &Theme| container::Style {
                background: Some(Background::Color(Color::from_rgba(0.0, 0.0, 0.0, 0.72))),
                ..Default::default()
            })
            .into()
    }

    fn view_add_to_playlist_modal<'a>(&'a self, track: &'a Track) -> Element<'a, Message> {
        let mut col = column![
            text("Add to playlist").size(18).color(style::TEXT_PRIMARY),
            text(truncate_text(&track.title, 42))
                .size(13)
                .color(style::TEXT_SECONDARY),
        ]
        .spacing(10);

        if self.user_playlists.is_empty() {
            col = col.push(
                text("No custom playlists yet.")
                    .size(13)
                    .color(style::TEXT_MUTED),
            );
        } else {
            for (i, pl) in self.user_playlists.iter().enumerate() {
                col = col.push(
                    button(
                        row![
                            text(&pl.name).size(14).color(style::TEXT_PRIMARY),
                            space::horizontal().width(Length::Fill),
                            text(format!("{} songs", pl.tracks.len()))
                                .size(12)
                                .color(style::TEXT_MUTED),
                        ]
                        .align_y(Alignment::Center),
                    )
                    .on_press(Message::AddTrackToUserPlaylist {
                        playlist_idx: i,
                        track: track.clone(),
                    })
                    .padding([10, 12])
                    .width(Length::Fill)
                    .style(style::track_row(false)),
                );
            }
        }

        col = col.push(
            row![
                button(text("+ New playlist").size(13))
                    .on_press(Message::OpenNewPlaylist)
                    .padding([8, 14])
                    .style(style::pill_button(false)),
                space::horizontal().width(Length::Fill),
                button(text("Close").size(13))
                    .on_press(Message::CloseModals)
                    .padding([8, 16])
                    .style(style::pill_button(true)),
            ]
            .spacing(10),
        );

        let modal = container(col)
            .padding(24)
            .max_width(420)
            .style(style::modal_card);

        container(modal)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .style(|_: &Theme| container::Style {
                background: Some(Background::Color(Color::from_rgba(0.0, 0.0, 0.0, 0.72))),
                ..Default::default()
            })
            .into()
    }

    fn render_thumb(&self, url: Option<&str>, size: f32, radius: f32) -> Element<'_, Message> {
        if let Some(u) = url
            && let Some(handle) = self.thumbs.get(u)
        {
            return image(handle.clone())
                .width(size)
                .height(size)
                .content_fit(ContentFit::Cover)
                .border_radius(radius)
                .into();
        }

        container(icons::note(size * 0.36, style::TEXT_MUTED))
            .width(size)
            .height(size)
            .center_x(size)
            .center_y(size)
            .style(style::thumb_box)
            .into()
    }

    fn render_avatar(
        &self,
        url: Option<&str>,
        size: f32,
        fallback_color: Color,
    ) -> Element<'_, Message> {
        if let Some(u) = url
            && let Some(handle) = self.thumbs.get(u)
        {
            return image(handle.clone())
                .width(size)
                .height(size)
                .content_fit(ContentFit::Cover)
                .border_radius(size / 2.0)
                .into();
        }

        icons::user(size * 0.72, fallback_color).into()
    }

    fn view_search_suggestions(&self) -> Element<'_, Message> {
        let mut list = column![].spacing(2);

        for item in &self.suggestions {
            let btn = match item {
                SearchSuggestion::Query(q) => button(
                    row![
                        icons::search(14.0, style::TEXT_MUTED),
                        text(truncate_text(q, 52))
                            .size(13)
                            .color(style::TEXT_PRIMARY),
                    ]
                    .spacing(10)
                    .align_y(Alignment::Center),
                )
                .on_press(Message::PickSuggestionQuery(q.clone())),
                SearchSuggestion::Track(t) => button(
                    row![
                        self.render_thumb(t.thumb_url.as_deref(), 36.0, 4.0),
                        column![
                            text(truncate_text(&t.title, 42))
                                .size(13)
                                .color(style::TEXT_PRIMARY),
                            text(format!("Song  •  {}", truncate_text(&t.artist, 36)))
                                .size(11)
                                .color(style::TEXT_MUTED),
                        ]
                        .spacing(2),
                        space::horizontal().width(Length::Fill),
                        icons::play(13.0, style::TEXT_SECONDARY),
                    ]
                    .spacing(10)
                    .align_y(Alignment::Center),
                )
                .on_press(Message::PickSuggestionTrack(t.clone())),
                SearchSuggestion::Browse(c) => button(
                    row![
                        self.render_thumb(c.thumb_url.as_deref(), 36.0, 4.0),
                        column![
                            text(truncate_text(&c.title, 42))
                                .size(13)
                                .color(style::TEXT_PRIMARY),
                            text(truncate_text(&c.subtitle, 44))
                                .size(11)
                                .color(style::TEXT_MUTED),
                        ]
                        .spacing(2),
                    ]
                    .spacing(10)
                    .align_y(Alignment::Center),
                )
                .on_press(Message::PickSuggestionBrowse(c.clone())),
            };

            list = list.push(
                btn.padding([8, 10])
                    .width(Length::Fill)
                    .style(style::track_row(false)),
            );
        }

        let footer = row![
            space::horizontal().width(Length::Fill),
            button(text("Close").size(11).color(style::TEXT_MUTED))
                .on_press(Message::DismissSuggestions)
                .padding([4, 8])
                .style(style::track_row(false)),
        ];

        container(
            column![
                container(scrollable(list).width(Length::Fill)).max_height(360),
                footer,
            ]
            .spacing(4),
        )
        .width(460)
        .padding(8)
        .style(style::modal_card)
        .into()
    }

    fn view_update_modal<'a>(&'a self, info: &'a UpdateInfo) -> Element<'a, Message> {
        let notes = if info.changelog.trim().is_empty() {
            "New release available on GitHub."
        } else {
            info.changelog.as_str()
        };

        let mut col = column![
            text(format!("Update available  —  {}", info.tag_name))
                .size(18)
                .color(style::TEXT_PRIMARY),
            text(format!(
                "You are running v{}. What's new:",
                env!("CARGO_PKG_VERSION")
            ))
            .size(13)
            .color(style::TEXT_SECONDARY),
            container(
                scrollable(
                    container(text(notes).size(13).color(style::TEXT_PRIMARY))
                        .padding(12)
                        .width(Length::Fill),
                )
                .width(Length::Fill),
            )
            .max_height(240)
            .width(Length::Fill)
            .style(style::modal_card),
        ]
        .spacing(12);

        if let Some(err) = &self.update_error {
            col = col.push(text(err).size(12).color(style::YT_RED));
        }

        let mut actions = row![
            button(text("View release").size(13))
                .on_press(Message::OpenUpdatePage)
                .padding([8, 14])
                .style(style::pill_button(false)),
            space::horizontal().width(Length::Fill),
            button(text("Later").size(13))
                .on_press(Message::DismissUpdateModal)
                .padding([8, 16])
                .style(style::pill_button(false)),
        ]
        .spacing(10)
        .align_y(Alignment::Center);

        if info.exe_download_url.is_some() {
            let update_label = if self.updating_app {
                "Updating..."
            } else {
                "Update & restart"
            };
            actions = actions.push(
                button(text(update_label).size(13))
                    .on_press_maybe((!self.updating_app).then_some(Message::StartUpdate))
                    .padding([8, 18])
                    .style(style::pill_button(true)),
            );
        }

        col = col.push(actions);

        let modal = container(col)
            .padding(24)
            .max_width(480)
            .style(style::modal_card);

        container(modal)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .style(|_: &Theme| container::Style {
                background: Some(Background::Color(Color::from_rgba(0.0, 0.0, 0.0, 0.72))),
                ..Default::default()
            })
            .into()
    }

    fn nav_button<'a>(
        &self,
        label: &'static str,
        target: Page,
        ico: Element<'a, Message>,
    ) -> Element<'a, Message> {
        let active = self.page == target;
        button(
            row![ico, text(label).size(14)]
                .spacing(14)
                .align_y(Alignment::Center),
        )
        .on_press(Message::OpenPage(target))
        .padding([10, 14])
        .width(Length::Fill)
        .style(style::nav_item(active))
        .into()
    }
}

fn icon_color(active: bool) -> Color {
    if active {
        style::TEXT_PRIMARY
    } else {
        style::TEXT_SECONDARY
    }
}

fn collect_shelf_thumbs(shelves: &[Shelf]) -> Vec<String> {
    let mut urls = Vec::new();
    for shelf in shelves {
        for item in &shelf.items {
            let url = match item {
                ShelfItem::Song(t) => t.thumb_url.as_ref(),
                ShelfItem::Card(c) => c.thumb_url.as_ref(),
                ShelfItem::Mood(_) => None,
            };
            if let Some(u) = url {
                urls.push(u.clone());
            }
        }
    }
    urls
}

fn shuffle_slice<T>(items: &mut [T]) {
    use std::collections::hash_map::RandomState;
    use std::hash::{BuildHasher, Hasher};

    let rs = RandomState::new();
    for i in (1..items.len()).rev() {
        let mut h = rs.build_hasher();
        h.write_usize(i);
        let j = (h.finish() as usize) % (i + 1);
        items.swap(i, j);
    }
}
