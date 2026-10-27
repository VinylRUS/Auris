//! Очередь воспроизведения, «волна» и продвижение по трекам.

use std::sync::Arc;

use auris_api::Track;

use crate::app::{App, WaveSession};
use crate::models;

impl App {
    /// Запускает воспроизведение индекса очереди (асинхронно).
    pub fn spawn_play(self: &Arc<Self>, index: usize) {
        let _app = Arc::clone(self);
        self.spawn(|app| async move {
            if let Err(e) = app.play_index(index).await {
                log::error!("playback: {e:#}");
            }
        });
    }

    /// Воспроизводит трек: кеш → прямая ссылка → файл → rodio.
    pub async fn play_index(self: &Arc<Self>, index: usize) -> anyhow::Result<()> {
        let (track, wave) = {
            let mut q = self.queue.lock().expect("queue");
            if index >= q.tracks.len() {
                anyhow::bail!("index {index} out of range");
            }
            q.index = Some(index);
            (q.tracks[index].clone(), q.wave.clone())
        };

        // Варианты загрузки + прямая ссылка.
        let client = self.api.lock().expect("api").clone();
        let infos = client
            .track_download_info(track.id)
            .await
            .map_err(|e| anyhow::anyhow!("download-info: {e}"))?;
        let Some(info) = self.downloads.pick_info(&infos) else {
            anyhow::bail!("трек недоступен для воспроизведения (нужна подписка?)");
        };
        let info_url = info.url.clone();
        let tag = format!("{}k", info.bitrate_in_kbps);
        let link = auris_api::sign::direct_link(&client.http(), &info_url).await?;
        let file = self.cache.fetch_audio(track.id, &tag, &link).await?;
        let path = match &file {
            auris_core::cache::CachedFile::Hit(p) | auris_core::cache::CachedFile::Fetched(p) => {
                p.clone()
            }
        };

        self.player.send(auris_player::Cmd::Play {
            path,
            track_id: track.id,
            duration_sec: track.duration_sec() as f64,
        });

        self.on_track_started(&track, wave.as_ref()).await;
        Ok(())
    }

    /// Обновляет UI/историю/фидбек после старта трека.
    async fn on_track_started(self: &Arc<Self>, track: &Track, wave: Option<&WaveSession>) {
        // История (guard не живёт дольше блока — await вне его)
        let recent: Vec<Track> = {
            let mut h = self.history.lock().expect("history");
            h.push(track);
            if let Err(e) = h.save() {
                log::warn!("history save: {e}");
            }
            h.recent(4)
        };
        {
            let mut cov = models::CoversSnapshot::default();
            self.fill_track_covers(&recent, &mut cov).await;
            self.ui_inv(move |ui| {
                let tiles: Vec<crate::TileItem> = recent
                    .iter()
                    .map(|t| models::history_tile(t, &cov))
                    .collect();
                ui.set_home_recent(slint::ModelRc::new(std::rc::Rc::new(
                    slint::VecModel::from(tiles),
                )));
            });
        }

        // Фидбек волны
        if let Some(w) = wave {
            self.wave_feedback(w, "trackStarted", Some(&track.composite_id()), None)
                .await;
        }

        // UI: текущий трек (обложку грузим в event loop по пути)
        let mut cov = models::CoversSnapshot::default();
        self.fill_track_covers(std::slice::from_ref(track), &mut cov)
            .await;
        let cover_path = cov.track_covers.get(&track.id).cloned();
        let title = track.title.clone();
        let artists = track.artists_display();
        let variant = models::cover_variant(track.id);
        self.ui_inv(move |ui| {
            let cover = cover_path
                .as_ref()
                .and_then(|p| slint::Image::load_from_path(std::path::Path::new(p)).ok())
                .unwrap_or_default();
            ui.set_cur_title(models::s(title));
            ui.set_cur_artists(models::s(artists));
            ui.set_cur_cover(cover);
            ui.set_cur_cover_variant(variant);
            ui.set_playing(true);
        });

        // Пересобрать модели с отметкой текущего трека
        self.rebuild_models();
        self.publish_media();
    }

    /// Продвижение очереди: следующий трек по режиму повтора/перемешивания.
    pub fn advance(self: &Arc<Self>, manual: bool) {
        let (index, len, repeat, shuffle, wave, ended) = {
            let q = self.queue.lock().expect("queue");
            let idx = match q.index {
                Some(i) => i,
                None => return,
            };
            (
                idx,
                q.tracks.len(),
                q.repeat,
                q.shuffle,
                q.wave.clone(),
                q.tracks.get(idx).cloned(),
            )
        };

        // Фидбек за завершённый трек
        if let (Some(t), Some(w)) = (&ended, &wave) {
            let event = if manual { "skip" } else { "trackFinished" };
            let _app = Arc::clone(self);
            let t = t.clone();
            let w = w.clone();
            self.spawn(|app| async move {
                app.wave_feedback(&w, event, Some(&t.composite_id()), None)
                    .await;
            });
        }

        let next: Option<usize> = if repeat == 2 && !manual {
            Some(index) // повтор одного трека
        } else if shuffle && len > 1 {
            let nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.subsec_nanos() as usize)
                .unwrap_or(0);
            let mut candidate = (nanos + index + 1) % len;
            if candidate == index {
                candidate = (candidate + 1) % len;
            }
            Some(candidate)
        } else if index + 1 < len {
            Some(index + 1)
        } else if repeat == 1 && len > 0 {
            Some(0)
        } else {
            None
        };

        match next {
            Some(i) => self.spawn_play(i),
            None => {
                // Очередь кончилась: для волны подтягиваем новую партию.
                if wave.is_some() {
                    let _app = Arc::clone(self);
                    self.spawn(|app| async move {
                        if let Err(e) = app.wave_fetch_more().await {
                            log::error!("wave fetch more: {e:#}");
                            app.ui_inv(|ui| ui.set_playing(false));
                        }
                    });
                } else {
                    self.ui_inv(|ui| ui.set_playing(false));
                }
            }
        }
    }

    /// Запуск «Мои волны» (или станции по сиду).
    pub fn start_wave(self: &Arc<Self>, seeds: Vec<String>, label: &str) {
        let _app = Arc::clone(self);
        let label = label.to_string();
        self.spawn(|app| async move {
            let client = app.api.lock().expect("api").clone();
            let session = client
                .rotor_session_new(&seeds)
                .await
                .map_err(|e| anyhow::anyhow!("rotor session: {e}"))
                .map_err(|e| {
                    log::error!("{e:#}");
                    e
                })
                .ok();
            let Some(session) = session else { return };
            let tracks = session.tracks();
            let wave = WaveSession {
                id: session.radio_session_id.clone().unwrap_or_default(),
                batch_id: session.batch_id.clone(),
                label: label.clone(),
            };
            if wave.id.is_empty() || tracks.is_empty() {
                log::error!("пустая сессия волны");
                return;
            }
            {
                let mut q = app.queue.lock().expect("queue");
                q.tracks = tracks;
                q.index = None;
                q.source = label.clone();
                q.wave = Some(wave.clone());
            }
            let label2 = label.clone();
            app.ui_inv(move |ui| {
                ui.set_wave_active(true);
                ui.set_queue_source(models::s(label2));
                ui.set_page(4);
            });
            app.wave_feedback(&wave, "radioStarted", None, None).await;
            if let Err(e) = app.play_index(0).await {
                log::error!("wave play: {e:#}");
            }
        });
    }

    /// Подтягивает следующую партию треков волны.
    pub async fn wave_fetch_more(self: &Arc<Self>) -> anyhow::Result<()> {
        let wave = {
            let q = self.queue.lock().expect("queue");
            q.wave
                .clone()
                .ok_or_else(|| anyhow::anyhow!("нет активной волны"))?
        };
        let client = self.api.lock().expect("api").clone();
        let batch = client
            .rotor_session_tracks(&wave.id)
            .await
            .map_err(|e| anyhow::anyhow!("rotor tracks: {e}"))?;
        if batch.unknown_session.unwrap_or(false) {
            anyhow::bail!("сессия волны истекла — перезапустите волну");
        }
        let more = batch.tracks();
        if more.is_empty() {
            anyhow::bail!("волна не вернула треков");
        }
        let start_index = {
            let mut q = self.queue.lock().expect("queue");
            let start = q.tracks.len();
            q.tracks.extend(more);
            if let Some(w) = q.wave.as_mut() {
                w.batch_id = batch.batch_id.clone();
            }
            start
        };
        self.spawn_play(start_index);
        Ok(())
    }

    /// Отправляет событие волны (fire-and-forget, ошибки только в лог).
    pub async fn wave_feedback(
        self: &Arc<Self>,
        wave: &WaveSession,
        event: &str,
        track_id: Option<&str>,
        total_played: Option<f64>,
    ) {
        let client = self.api.lock().expect("api").clone();
        if let Err(e) = client
            .rotor_session_feedback(
                &wave.id,
                event,
                track_id,
                total_played,
                wave.batch_id.as_deref(),
            )
            .await
        {
            log::debug!("wave feedback {event}: {e}");
        }
    }

    /// Пересобирает все видимые модели с текущим состоянием.
    pub fn rebuild_models(self: &Arc<Self>) {
        let snapshot = self.player.snapshot();
        let current_id = snapshot.track_id;
        let playing = snapshot.playing;

        let (queue_tracks, wave, source, search, playlist, downloads, cov) = {
            let q = self.queue.lock().expect("queue");
            let c = self.ui_cache.lock().expect("cache");
            (
                q.tracks.clone(),
                q.wave.is_some(),
                q.source.clone(),
                c.search.clone(),
                c.playlist_tracks.clone(),
                c.downloads.clone(),
                c.covers.clone(),
            )
        };
        let states = self.download_states();

        // Волна: ближайшие треки после текущего
        let upcoming: Vec<Track> = if wave {
            let idx = self.queue.lock().expect("queue").index.unwrap_or(0);
            queue_tracks
                .iter()
                .skip(idx.saturating_add(1))
                .take(12)
                .cloned()
                .collect()
        } else {
            Vec::new()
        };

        let liked = current_id
            .map(|id| cov.likes.contains(&id))
            .unwrap_or(false);
        self.ui_inv(move |ui| {
            let make = |tracks: &[Track]| {
                slint::ModelRc::new(std::rc::Rc::new(slint::VecModel::from(
                    models::track_items(tracks, &cov, current_id, playing, &states),
                )))
            };
            ui.set_queue_items(make(&queue_tracks));
            ui.set_wave_upcoming(make(&upcoming));
            ui.set_search_tracks(make(&search));
            ui.set_playlist_tracks(make(&playlist));
            ui.set_downloads_items(make(&downloads));
            ui.set_cur_liked(liked);
            if !source.is_empty() {
                ui.set_queue_source(models::s(source));
            }
        });
    }

    /// Состояния загрузок в формате UI (id → код).
    pub fn download_states(&self) -> std::collections::HashMap<i64, i32> {
        self.downloads
            .states()
            .into_iter()
            .map(|(id, state)| {
                let code = match state {
                    auris_core::downloads::DownloadState::Queued => 1,
                    auris_core::downloads::DownloadState::Progress(_) => 1,
                    auris_core::downloads::DownloadState::Done => 2,
                    auris_core::downloads::DownloadState::Failed => 3,
                };
                (id, code)
            })
            .collect()
    }

    /// Загружает обложки треков и пополняет общий кеш (путями).
    pub async fn fill_track_covers(
        self: &Arc<Self>,
        tracks: &[Track],
        cov: &mut models::CoversSnapshot,
    ) {
        for t in tracks {
            if t.id == 0 || cov.track_covers.contains_key(&t.id) {
                continue;
            }
            if let Some(url) = t.cover_url("200x200") {
                let path = self.cache.cover(&url).await;
                cov.track_covers.insert(t.id, path);
            }
        }
        let mut cache = self.ui_cache.lock().expect("cache");
        for (k, v) in &cov.track_covers {
            cache.covers.track_covers.insert(*k, v.clone());
        }
    }

    /// Тикающий опрос плеера (вызывается таймером).
    pub fn on_tick(self: &Arc<Self>) {
        let snapshot = self.player.snapshot();
        self.ui_inv(move |ui| {
            ui.set_position_sec(snapshot.position_sec as f32);
            ui.set_duration_sec(snapshot.duration_sec as f32);
            ui.set_playing(snapshot.playing);
        });

        // События плеера (единственный вариант — Ended)
        let rx = self.player_events.lock().expect("events");
        while let Ok(_ev) = rx.try_recv() {
            self.advance(false);
        }
    }

    /// Предыдущий трек (или в начало, если трек идёт > 3 с).
    pub fn prev_track(self: &Arc<Self>) {
        let snap = self.player.snapshot();
        if snap.position_sec > 3.0 {
            self.player.send(auris_player::Cmd::Seek(0.0));
            return;
        }
        let idx = self.queue.lock().expect("queue").index.unwrap_or(0);
        if idx > 0 {
            self.spawn_play(idx - 1);
        } else {
            self.player.send(auris_player::Cmd::Seek(0.0));
        }
    }
}
