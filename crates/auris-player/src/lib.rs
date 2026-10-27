//! # auris-player
//!
//! Сервис воспроизведения на rodio + symphonia.
//!
//! Плеер живёт в отдельном потоке и управляется командами через
//! [`PlayerHandle`]; состояние (позиция, длительность, флаг паузы)
//! доступно без блокировок через [`PlayerSnapshot`] для поллинга из UI.
//!
//! Файл стримится в локальный кеш приложением (см. `auris-core`),
//! поэтому источник всегда seek-able — это упрощает перемотку и
//! даёт бесплатный офлайн-кеш.

#![warn(missing_docs)]

use std::fmt;
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use rodio::mixer::Mixer;
use rodio::{OutputStream, OutputStreamBuilder, Sink};

/// Команды плееру.
#[derive(Debug)]
pub enum Cmd {
    /// Играть файл с начала (кеш-путь).
    Play {
        /// Путь к локальному файлу.
        path: PathBuf,
        /// Идентификатор трека (для событий).
        track_id: i64,
        /// Длительность, с (из метаданных API).
        duration_sec: f64,
    },
    /// Пауза.
    Pause,
    /// Продолжить.
    Resume,
    /// Пауза/продолжить.
    Toggle,
    /// Полный стоп.
    Stop,
    /// Перемотка на долю [0..1] длительности.
    Seek(f64),
    /// Громкость [0..1].
    SetVolume(f32),
}

/// События плеера.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PlayerEvent {
    /// Дорожка закончилась естественным образом (`track_id`).
    Ended(i64),
}

/// Снимок состояния плеера для UI.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct PlayerSnapshot {
    /// Идентификатор текущего трека.
    pub track_id: Option<i64>,
    /// Играет ли сейчас.
    pub playing: bool,
    /// Позиция, с.
    pub position_sec: f64,
    /// Длительность, с (из API).
    pub duration_sec: f64,
    /// Громкость [0..1].
    pub volume: f32,
}

#[derive(Default)]
struct InnerState {
    snapshot: PlayerSnapshot,
    /// Была ли подана команда Play (чтобы не ловить «empty» на старте).
    started: bool,
}

struct Player {
    _stream: OutputStream,
    mixer: Mixer,
    sink: Option<Sink>,
    state: Arc<Mutex<InnerState>>,
    events: Sender<PlayerEvent>,
}

/// Управляющий хэндл плеера (клонируемый).
#[derive(Clone)]
pub struct PlayerHandle {
    tx: Sender<Cmd>,
    state: Arc<Mutex<InnerState>>,
}

impl PlayerHandle {
    /// Отправить команду плееру.
    pub fn send(&self, cmd: Cmd) {
        if self.tx.send(cmd).is_err() {
            log::warn!("player thread is gone, command dropped");
        }
    }

    /// Мгновенный снимок состояния.
    pub fn snapshot(&self) -> PlayerSnapshot {
        self.state.lock().map(|s| s.snapshot).unwrap_or_default()
    }
}

impl fmt::Debug for PlayerHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PlayerHandle")
            .field("snapshot", &self.snapshot())
            .finish()
    }
}

impl Player {
    fn handle_cmd(&mut self, cmd: Cmd) {
        match cmd {
            Cmd::Play {
                path,
                track_id,
                duration_sec,
            } => self.handle_play(path, track_id, duration_sec),
            cmd => self.handle_simple(cmd),
        }
    }

    fn handle_play(&mut self, path: PathBuf, track_id: i64, duration_sec: f64) {
        let started = match self.start_sink(&path) {
            Ok(sink) => {
                self.sink = Some(sink);
                true
            }
            Err(e) => {
                log::error!("cannot play {}: {e}", path.display());
                false
            }
        };
        let mut state = self.state.lock().expect("player state poisoned");
        state.started = started;
        state.snapshot.track_id = Some(track_id);
        state.snapshot.playing = started;
        state.snapshot.position_sec = 0.0;
        state.snapshot.duration_sec = duration_sec;
    }

    fn handle_simple(&mut self, cmd: Cmd) {
        let mut state = self.state.lock().expect("player state poisoned");
        match cmd {
            Cmd::Pause => {
                if let Some(s) = &self.sink {
                    s.pause();
                    state.snapshot.playing = false;
                }
            }
            Cmd::Resume => {
                if let Some(s) = &self.sink {
                    s.play();
                    state.snapshot.playing = true;
                }
            }
            Cmd::Toggle => {
                if let Some(s) = &self.sink {
                    if state.snapshot.playing {
                        s.pause();
                        state.snapshot.playing = false;
                    } else {
                        s.play();
                        state.snapshot.playing = true;
                    }
                }
            }
            Cmd::Stop => {
                self.sink = None; // drop → stop
                state.snapshot = PlayerSnapshot {
                    volume: state.snapshot.volume,
                    ..Default::default()
                };
                state.started = false;
            }
            Cmd::Seek(frac) => {
                if let Some(s) = &self.sink {
                    let frac = frac.clamp(0.0, 1.0);
                    let target = state.snapshot.duration_sec * frac;
                    if let Err(e) = s.try_seek(Duration::from_secs_f64(target)) {
                        log::warn!("seek to {target:.1}s failed: {e:?}");
                    } else {
                        state.snapshot.position_sec = target;
                    }
                }
            }
            Cmd::SetVolume(v) => {
                let v = v.clamp(0.0, 1.0);
                if let Some(s) = &self.sink {
                    s.set_volume(v);
                }
                state.snapshot.volume = v;
            }
            // Play обрабатывается в handle_play и сюда попасть не может.
            Cmd::Play { .. } => unreachable!("Play is handled by handle_play"),
        }
    }

    fn start_sink(&mut self, path: &PathBuf) -> anyhow::Result<Sink> {
        let file = std::fs::File::open(path)?;
        let sink = Sink::connect_new(&self.mixer);
        sink.append(rodio::Decoder::new(std::io::BufReader::new(file))?);
        sink.set_volume(self.state.lock().map(|s| s.snapshot.volume).unwrap_or(1.0));
        Ok(sink)
    }

    fn tick(&mut self) {
        let Some(sink) = &self.sink else { return };
        let mut state = self.state.lock().expect("player state poisoned");
        if state.snapshot.playing {
            state.snapshot.position_sec = sink.get_pos().as_secs_f64();
        }
        // Завершение дорожки: буфер опустел во время воспроизведения.
        if state.started && state.snapshot.playing && sink.empty() {
            let id = state.snapshot.track_id.unwrap_or_default();
            state.snapshot.playing = false;
            state.snapshot.position_sec = state.snapshot.duration_sec;
            drop(state);
            let _ = self.events.send(PlayerEvent::Ended(id));
        }
    }
}

/// Запускает поток плеера; возвращает хэндл и приёмник событий.
///
/// Важно: `OutputStream` (cpal) на macOS не `Send`, поэтому аудио-поток
/// создаётся **внутри** потока плеера, а не переносится в него.
/// При недоступности аудиоустройства поток плеера завершается с ошибкой
/// в лог — хэндл остаётся валидным, команды игнорируются.
pub fn spawn() -> anyhow::Result<(PlayerHandle, Receiver<PlayerEvent>)> {
    let (cmd_tx, cmd_rx) = std::sync::mpsc::channel::<Cmd>();
    let (ev_tx, ev_rx) = std::sync::mpsc::channel::<PlayerEvent>();
    let state = Arc::new(Mutex::new(InnerState::default()));

    let thread_state = Arc::clone(&state);
    std::thread::Builder::new()
        .name("auris-player".into())
        .spawn(move || {
            let stream = match OutputStreamBuilder::open_default_stream() {
                Ok(s) => s,
                Err(e) => {
                    log::error!("аудиоустройство недоступна: {e}");
                    return;
                }
            };
            let mixer = stream.mixer().clone();
            let mut player = Player {
                _stream: stream,
                mixer,
                sink: None,
                state: thread_state,
                events: ev_tx,
            };
            loop {
                match cmd_rx.recv_timeout(Duration::from_millis(120)) {
                    Ok(cmd) => player.handle_cmd(cmd),
                    Err(_) => {} // таймаут или отключение — просто тикаем
                }
                player.tick();
            }
        })?;

    Ok((PlayerHandle { tx: cmd_tx, state }, ev_rx))
}
