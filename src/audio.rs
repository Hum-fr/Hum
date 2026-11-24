use std::io::Cursor;
use std::sync::{
    Arc, Mutex,
    mpsc::{self, RecvTimeoutError, Sender},
};
use std::thread;
use std::time::Duration;

use rodio::{Decoder, DeviceSinkBuilder, Player, Source};

use crate::dsp::{DspSource, SharedDspState};
use crate::types::{AudioSnapshot, DspConfig, PlaybackState, SpectrumFrame};

#[derive(Clone)]
struct SharedBytes(Arc<[u8]>);

impl AsRef<[u8]> for SharedBytes {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

enum Command {
    PlayBytes { seq: u64, bytes: Vec<u8> },
    TogglePause,
    Seek(Duration),
    SetVolume(f32),
    SetSpeed(f32),
    Stop,
}

#[derive(Clone)]
pub struct AudioEngine {
    tx: Sender<Command>,
    snapshot: Arc<Mutex<AudioSnapshot>>,
    dsp_state: Arc<SharedDspState>,
    spectrum: Arc<Mutex<SpectrumFrame>>,
}

impl AudioEngine {
    pub fn start(initial_volume: f32, initial_dsp: DspConfig) -> Self {
        let (tx, rx) = mpsc::channel();
        let snapshot = Arc::new(Mutex::new(AudioSnapshot::default()));
        let dsp_state = Arc::new(SharedDspState::new(initial_dsp));
        let spectrum = Arc::new(Mutex::new(SpectrumFrame::default()));
        let worker_snapshot = Arc::clone(&snapshot);
        let worker_dsp = Arc::clone(&dsp_state);
        let worker_spectrum = Arc::clone(&spectrum);

        thread::Builder::new()
            .name("hum-audio".into())
            .spawn(move || {
                audio_loop(
                    rx,
                    worker_snapshot,
                    worker_dsp,
                    worker_spectrum,
                    initial_volume.clamp(0.0, 1.0),
                    initial_dsp.speed.clamp(0.75, 1.50),
                )
            })
            .expect("failed to spawn audio thread");

        Self {
            tx,
            snapshot,
            dsp_state,
            spectrum,
        }
    }

    pub fn set_dsp(&self, cfg: DspConfig) {
        self.dsp_state.update(cfg);
        let _ = self.tx.send(Command::SetSpeed(cfg.speed.clamp(0.75, 1.50)));
    }

    pub fn spectrum(&self) -> SpectrumFrame {
        *self.spectrum.lock().unwrap()
    }

    pub fn play_bytes(&self, seq: u64, bytes: Vec<u8>) {
        {
            let mut guard = self.snapshot.lock().unwrap();
            guard.seq = seq;
            guard.state = PlaybackState::Loading;
            guard.position = Duration::ZERO;
            guard.finished_track = false;
            guard.error = None;
        }
        let _ = self.tx.send(Command::PlayBytes { seq, bytes });
    }

    pub fn toggle_pause(&self) {
        {
            let mut guard = self.snapshot.lock().unwrap();
            guard.state = match guard.state {
                PlaybackState::Playing => PlaybackState::Paused,
                PlaybackState::Paused => PlaybackState::Playing,
                other => other,
            };
        }
        let _ = self.tx.send(Command::TogglePause);
    }

    pub fn seek(&self, pos: Duration) {
        let _ = self.tx.send(Command::Seek(pos));
    }

    pub fn set_volume(&self, volume: f32) {
        let _ = self.tx.send(Command::SetVolume(volume.clamp(0.0, 1.0)));
    }

    pub fn stop(&self) {
        let _ = self.tx.send(Command::Stop);
    }

    pub fn take_snapshot(&self) -> AudioSnapshot {
        let mut guard = self.snapshot.lock().unwrap();
        let out = guard.clone();
        guard.finished_track = false;
        out
    }
}

fn sniff_audio_format(bytes: &[u8]) -> (Option<&'static str>, Option<&'static str>) {
    if bytes.len() >= 8 && &bytes[4..8] == b"ftyp" {
        return (Some("m4a"), Some("audio/mp4"));
    }
    if bytes.starts_with(b"OggS") {
        return (Some("ogg"), Some("audio/ogg"));
    }
    if bytes.starts_with(b"ID3")
        || (bytes.len() >= 2 && bytes[0] == 0xFF && (bytes[1] & 0xE0) == 0xE0)
    {
        return (Some("mp3"), Some("audio/mpeg"));
    }
    if bytes.starts_with(b"fLaC") {
        return (Some("flac"), Some("audio/flac"));
    }
    if bytes.starts_with(b"RIFF") {
        return (Some("wav"), Some("audio/wav"));
    }
    (None, None)
}

fn build_fast_decoder(bytes: Vec<u8>) -> Result<Decoder<Cursor<SharedBytes>>, rodio::decoder::DecoderError> {
    let byte_len = bytes.len() as u64;
    let (hint, mime) = sniff_audio_format(&bytes);
    let shared = SharedBytes(Arc::from(bytes));

    if hint.is_some() || mime.is_some() {
        let mut builder = Decoder::builder()
            .with_data(Cursor::new(shared.clone()))
            .with_byte_len(byte_len)
            .with_seekable(true);
        if let Some(h) = hint {
            builder = builder.with_hint(h);
        }
        if let Some(m) = mime {
            builder = builder.with_mime_type(m);
        }
        if let Ok(dec) = builder.build() {
            return Ok(dec);
        }
    }

    Decoder::builder()
        .with_data(Cursor::new(shared))
        .with_byte_len(byte_len)
        .with_seekable(true)
        .build()
}

fn audio_loop(
    rx: mpsc::Receiver<Command>,
    snapshot: Arc<Mutex<AudioSnapshot>>,
    dsp_state: Arc<SharedDspState>,
    spectrum: Arc<Mutex<SpectrumFrame>>,
    initial_volume: f32,
    initial_speed: f32,
) {
    let sink_handle = match DeviceSinkBuilder::open_default_sink() {
        Ok(handle) => handle,
        Err(err) => {
            let mut guard = snapshot.lock().unwrap();
            guard.error = Some(format!("Audio device error: {err}"));
            return;
        }
    };

    let player = Player::connect_new(&sink_handle.mixer());
    let mut has_track = false;
    let mut current_seq: u64 = 0;
    let mut volume = initial_volume;
    let mut speed = initial_speed;
    player.set_volume(volume);
    player.set_speed(speed);

    loop {
        // Block completely when idle or paused so background CPU stays near zero.
        let msg = if has_track && !player.is_paused() {
            match rx.recv_timeout(Duration::from_millis(50)) {
                Ok(cmd) => Some(cmd),
                Err(RecvTimeoutError::Timeout) => None,
                Err(RecvTimeoutError::Disconnected) => break,
            }
        } else {
            match rx.recv() {
                Ok(cmd) => Some(cmd),
                Err(_) => break,
            }
        };

        if let Some(cmd) = msg {
            match cmd {
                Command::PlayBytes { seq, bytes } => {
                    current_seq = seq;
                    player.clear();
                    *spectrum.lock().unwrap() = SpectrumFrame::default();
                    match build_fast_decoder(bytes) {
                        Ok(source) => {
                            let decoded_duration = source.total_duration();
                            let processed =
                                DspSource::new(source, Arc::clone(&dsp_state), Arc::clone(&spectrum));
                            player.set_volume(volume);
                            player.set_speed(speed);
                            player.append(processed);
                            player.play();
                            has_track = true;
                            let mut guard = snapshot.lock().unwrap();
                            guard.seq = seq;
                            guard.state = PlaybackState::Playing;
                            guard.position = Duration::ZERO;
                            guard.duration = decoded_duration;
                            guard.finished_track = false;
                            guard.error = None;
                        }
                        Err(err) => {
                            has_track = false;
                            let mut guard = snapshot.lock().unwrap();
                            guard.seq = seq;
                            guard.state = PlaybackState::Stopped;
                            guard.position = Duration::ZERO;
                            guard.duration = None;
                            guard.error = Some(format!("Decode error: {err}"));
                        }
                    }
                }
                Command::TogglePause => {
                    if has_track {
                        if player.is_paused() {
                            player.play();
                        } else {
                            player.pause();
                        }
                    }
                }
                Command::Seek(pos) => {
                    if has_track {
                        let _ = player.try_seek(pos);
                    }
                }
                Command::SetVolume(vol) => {
                    volume = vol;
                    player.set_volume(vol);
                }
                Command::SetSpeed(s) => {
                    speed = s.clamp(0.75, 1.50);
                    player.set_speed(speed);
                }
                Command::Stop => {
                    player.clear();
                    has_track = false;
                    *spectrum.lock().unwrap() = SpectrumFrame::default();
                    let mut guard = snapshot.lock().unwrap();
                    guard.state = PlaybackState::Stopped;
                    guard.position = Duration::ZERO;
                    guard.duration = None;
                    guard.finished_track = false;
                }
            }
        }

        if has_track {
            let mut guard = snapshot.lock().unwrap();
            if guard.seq == current_seq {
                if player.empty() {
                    has_track = false;
                    *spectrum.lock().unwrap() = SpectrumFrame::default();
                    guard.state = PlaybackState::Stopped;
                    guard.position = Duration::ZERO;
                    guard.finished_track = true;
                } else {
                    guard.state = if player.is_paused() {
                        PlaybackState::Paused
                    } else {
                        PlaybackState::Playing
                    };
                    guard.position = player.get_pos();
                }
            }
        }
    }
}
