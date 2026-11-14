use std::io::Cursor;
use std::sync::{
    Arc, Mutex,
    mpsc::{self, RecvTimeoutError, Sender},
};
use std::thread;
use std::time::Duration;

use rodio::{Decoder, DeviceSinkBuilder, Player, Source};

use crate::types::{AudioSnapshot, PlaybackState};

enum Command {
    PlayBytes { seq: u64, bytes: Vec<u8> },
    TogglePause,
    Seek(Duration),
    SetVolume(f32),
    Stop,
}

#[derive(Clone)]
pub struct AudioEngine {
    tx: Sender<Command>,
    snapshot: Arc<Mutex<AudioSnapshot>>,
}

impl AudioEngine {
    pub fn start() -> Self {
        let (tx, rx) = mpsc::channel();
        let snapshot = Arc::new(Mutex::new(AudioSnapshot::default()));
        let worker_snapshot = Arc::clone(&snapshot);

        thread::Builder::new()
            .name("hum-audio".into())
            .spawn(move || audio_loop(rx, worker_snapshot))
            .expect("failed to spawn audio thread");

        Self { tx, snapshot }
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

fn audio_loop(rx: mpsc::Receiver<Command>, snapshot: Arc<Mutex<AudioSnapshot>>) {
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
    let mut volume = 0.85_f32;
    player.set_volume(volume);

    loop {
        // Block completely when idle or paused so background CPU stays near zero.
        let msg = if has_track && !player.is_paused() {
            match rx.recv_timeout(Duration::from_millis(200)) {
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
                    match Decoder::new(Cursor::new(bytes)) {
                        Ok(source) => {
                            let decoded_duration = source.total_duration();
                            player.set_volume(volume);
                            player.append(source);
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
                Command::Stop => {
                    player.clear();
                    has_track = false;
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
