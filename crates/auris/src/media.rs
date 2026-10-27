//! Системные медиа-контролы (SMTC в Windows, MPRIS в Linux) — опционально.
//!
//! Модуль пуст, если фича `media-controls` отключена.

#![cfg(feature = "media-controls")]

use std::sync::Arc;
use std::time::Duration;

use auris_player::Cmd;

use crate::app::App;

/// Создаёт контролы, привязанные к приложению.
///
/// Возвращает `None`, если платформа недоступна (нет D-Bus и т.п.) —
/// приложение продолжает работать без интеграции с ОС.
pub fn attach(app: &Arc<App>) -> Option<playwire::MediaControls> {
    let config = playwire::PlayerConfig::new("Auris")
        .desktop_entry("auris")
        .track_id_prefix("/com/github/vinylrus/auris/track");

    let app = Arc::clone(app);
    match playwire::MediaControls::new(config, move |event| match event {
        playwire::Event::Play => app.player.send(Cmd::Resume),
        playwire::Event::Pause => app.player.send(Cmd::Pause),
        playwire::Event::PlayPause => app.player.send(Cmd::Toggle),
        playwire::Event::Stop => app.player.send(Cmd::Stop),
        playwire::Event::Next => app.advance(true),
        playwire::Event::Previous => app.prev_track(),
        playwire::Event::SeekTo(pos) => {
            let dur = app.player.snapshot().duration_sec;
            if dur > 0.5 {
                app.player.send(Cmd::Seek(pos.as_secs_f64() / dur));
            }
        }
        _ => {}
    }) {
        Ok(controls) => Some(controls),
        Err(e) => {
            log::warn!("media controls недоступны: {e}");
            None
        }
    }
}

impl App {
    /// Публикует снимок состояния в SMTC/MPRIS.
    pub fn publish_media(self: &Arc<Self>) {
        let Ok(mut guard) = self.media.lock() else {
            return;
        };
        let Some(controls) = guard.as_mut() else {
            return;
        };

        let snapshot = self.player.snapshot();
        let (track, repeat, shuffle) = {
            let q = self.queue.lock().expect("queue");
            let track = q.index.and_then(|i| q.tracks.get(i)).cloned();
            (track, q.repeat, q.shuffle)
        };

        let state = playwire::PlaybackState {
            track: track.as_ref().map(|t| playwire::Track {
                id: format!("ym-{}", t.id),
                title: t.title.clone(),
                artists: vec![t.artists_display()],
                album: t.album_title().to_string(),
                artwork_url: String::new(),
                url: String::new(),
            }),
            playing: snapshot.playing,
            position: Duration::from_secs_f64(snapshot.position_sec),
            duration: if snapshot.duration_sec > 0.0 {
                Some(Duration::from_secs_f64(snapshot.duration_sec))
            } else {
                None
            },
            volume: snapshot.volume as f64,
            repeat: match repeat {
                1 => playwire::Repeat::All,
                2 => playwire::Repeat::One,
                _ => playwire::Repeat::Off,
            },
            shuffle,
            capabilities: playwire::Capabilities::default(),
        };
        if let Err(e) = controls.set_state(&state) {
            log::debug!("media publish: {e}");
        }
    }
}
