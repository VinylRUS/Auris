//! Обработчики колбэков UI.

use std::sync::atomic::Ordering;
use std::sync::Arc;

use auris_api::Track;
use auris_core::{Quality, TokenStorage};
use slint::ComponentHandle;

use crate::app::App;
use crate::models;
use crate::MainWindow;

pub(crate) fn init_window(app: &Arc<App>, ui: &MainWindow) {
    {
        let weak = app.ui.clone();
        ui.on_window_minimize(move || {
            if let Some(ui) = weak.upgrade() {
                ui.window().set_minimized(true);
            }
        });
    }
    {
        let weak = app.ui.clone();
        ui.on_window_maximize(move || {
            if let Some(ui) = weak.upgrade() {
                let maximized = ui.window().is_maximized();
                ui.window().set_maximized(!maximized);
                ui.set_win_maximized(!maximized);
            }
        });
    }
    ui.on_window_close(move || {
        slint::quit_event_loop().ok();
    });
}

pub(crate) fn init_login(app: &Arc<App>, ui: &MainWindow) {
    {
        let app = Arc::clone(app);
        ui.on_login_start(move || app.start_login());
    }
    {
        let app = Arc::clone(app);
        ui.on_login_open_browser(move || {
            let url = app
                .ui
                .upgrade()
                .map(|ui| ui.get_login_url().to_string())
                .unwrap_or_default();
            if !url.is_empty() {
                let app = Arc::clone(&app);
                app.spawn(|_app| async move {
                    if let Err(e) = open::that(&url) {
                        log::warn!("open browser: {e}");
                    }
                });
            }
        });
    }
    {
        let app = Arc::clone(app);
        ui.on_login_paste_token(move |token| {
            let token = token.to_string();
            if token.trim().is_empty() {
                return;
            }
            let app = Arc::clone(&app);
            app.spawn(|app| async move {
                app.finish_login(token.trim().to_string()).await;
            });
        });
    }
    {
        let app = Arc::clone(app);
        ui.on_login_cancel(move || {
            // Прерываем опрос Device Flow и возвращаемся на стартовый экран.
            app.login_cancel.store(true, Ordering::Relaxed);
            if let Some(ui) = app.ui.upgrade() {
                ui.set_login_state(0);
            }
        });
    }
}

pub(crate) fn init_search(app: &Arc<App>, ui: &MainWindow) {
    {
        let app = Arc::clone(app);
        ui.on_search(move |text| {
            let text = text.to_string();
            if text.trim().is_empty() {
                return;
            }
            let app = Arc::clone(&app);
            app.spawn(|app| async move { app.run_search(&text).await });
        });
    }
    {
        let app = Arc::clone(app);
        ui.on_play_from_search(move |index| app.play_from_context(index as usize, 0));
    }
    {
        let app = Arc::clone(app);
        ui.on_play_best(move || {
            // Лучший результат: трек — играем, альбом/исполнителя/плейлист — открываем.
            let best = app.ui.upgrade().map(|ui| ui.get_search_best());
            if let Some(best) = best {
                match best.item_type {
                    2 => {
                        let app = Arc::clone(&app);
                        app.spawn(|app| async move { app.open_album(best.kind as i64).await });
                    }
                    1 => {
                        let app = Arc::clone(&app);
                        app.spawn(|app| async move { app.open_artist(best.kind as i64).await });
                    }
                    3 => {
                        let app = Arc::clone(&app);
                        let (uid, kind) = (best.owner_uid as i64, best.kind as i64);
                        app.spawn(|app| async move { app.open_playlist(uid, kind).await });
                    }
                    _ => app.play_from_context(0, 1),
                }
            }
        });
    }
    {
        let app = Arc::clone(app);
        ui.on_search_like_toggled(move |index| app.toggle_like_at(0, index as usize));
    }
    {
        let app = Arc::clone(app);
        ui.on_search_download(move |index| app.download_at(0, index as usize));
    }
}

pub(crate) fn init_home(app: &Arc<App>, ui: &MainWindow) {
    {
        let app = Arc::clone(app);
        ui.on_play_wave(move || app.start_wave(vec!["user:onyourwave".into()], "Моя волна"));
    }
    {
        let app = Arc::clone(app);
        ui.on_chip_clicked(move |chip| {
            let app = Arc::clone(&app);
            app.spawn(|app| async move {
                match chip {
                    1 => app.load_landing("new").await,
                    2 => app.load_landing("chart").await,
                    _ => app.load_home().await,
                }
            });
        });
    }
    {
        let app = Arc::clone(app);
        ui.on_open_tile(move |tile| {
            let app = Arc::clone(&app);
            let item_type = tile.item_type;
            let owner_uid = tile.owner_uid;
            let kind = tile.kind;
            app.spawn(|app| async move {
                match item_type {
                    0 => app.open_playlist(owner_uid as i64, kind as i64).await,
                    1 => {
                        // История: играем последние треки
                        let tracks: Vec<Track> = app.history.lock().expect("history").recent(20);
                        if !tracks.is_empty() {
                            {
                                let mut q = app.queue.lock().expect("queue");
                                q.tracks = tracks;
                                q.index = None;
                                q.source = "История".into();
                                q.wave = None;
                            }
                            app.ui_inv(|ui| ui.set_wave_active(false));
                            app.spawn_play(0);
                        }
                    }
                    2 => app.open_likes().await,
                    3 => app.open_album(kind as i64).await,
                    _ => app.open_artist(kind as i64).await,
                }
            });
        });
    }
}

pub(crate) fn init_player(app: &Arc<App>, ui: &MainWindow) {
    {
        let app = Arc::clone(app);
        ui.on_player_toggle(move || {
            app.player.send(auris_player::Cmd::Toggle);
            app.publish_media();
        });
    }
    {
        let app = Arc::clone(app);
        ui.on_player_next(move || app.advance(true));
    }
    {
        let app = Arc::clone(app);
        ui.on_player_prev(move || app.prev_track());
    }
    {
        let app = Arc::clone(app);
        ui.on_player_seek(move |frac| {
            app.player.send(auris_player::Cmd::Seek(frac as f64));
            app.publish_media();
        });
    }
    {
        let app = Arc::clone(app);
        ui.on_player_set_volume(move |vol| {
            app.player.send(auris_player::Cmd::SetVolume(vol));
            let mut cfg = app.config.lock().expect("config");
            cfg.volume = vol;
            let _ = cfg.save();
            app.publish_media();
        });
    }
    {
        let app = Arc::clone(app);
        ui.on_player_toggle_like(move || {
            let track = {
                let q = app.queue.lock().expect("queue");
                q.index.and_then(|i| q.tracks.get(i).cloned())
            };
            if let Some(t) = track {
                let app = Arc::clone(&app);
                app.spawn(|app| async move { app.toggle_like(t).await });
            }
        });
    }
    {
        let app = Arc::clone(app);
        ui.on_player_toggle_shuffle(move || {
            let on = {
                let mut q = app.queue.lock().expect("queue");
                q.shuffle = !q.shuffle;
                q.shuffle
            };
            app.ui_inv(move |ui| ui.set_shuffle(on));
            app.publish_media();
        });
    }
    {
        let app = Arc::clone(app);
        ui.on_player_cycle_repeat(move || {
            let r = {
                let mut q = app.queue.lock().expect("queue");
                q.repeat = (q.repeat + 1) % 3;
                q.repeat
            };
            app.ui_inv(move |ui| ui.set_repeat(r as i32));
            app.publish_media();
        });
    }
}

pub(crate) fn init_queue(app: &Arc<App>, ui: &MainWindow) {
    {
        let app = Arc::clone(app);
        ui.on_queue_play_index(move |index| {
            let len = app.queue.lock().expect("queue").tracks.len();
            if (index as usize) < len {
                app.spawn_play(index as usize);
            }
        });
    }
    {
        let app = Arc::clone(app);
        ui.on_queue_remove_index(move |index| {
            {
                let mut q = app.queue.lock().expect("queue");
                let i = index as usize;
                if i < q.tracks.len() {
                    q.tracks.remove(i);
                    match q.index {
                        Some(cur) if cur > i => q.index = Some(cur - 1),
                        Some(cur) if cur == i => q.index = None,
                        _ => {}
                    }
                }
            }
            app.rebuild_models();
        });
    }
    {
        let app = Arc::clone(app);
        ui.on_playlist_play_all(move || {
            let tracks = app.ui_cache.lock().expect("cache").playlist_tracks.clone();
            if tracks.is_empty() {
                return;
            }
            {
                let mut q = app.queue.lock().expect("queue");
                q.tracks = tracks;
                q.index = None;
                q.source = "Плейлист".into();
                q.wave = None;
            }
            app.ui_inv(|ui| ui.set_wave_active(false));
            app.spawn_play(0);
        });
    }
    {
        let app = Arc::clone(app);
        ui.on_playlist_play_index(move |index| {
            let tracks = app.ui_cache.lock().expect("cache").playlist_tracks.clone();
            if (index as usize) < tracks.len() {
                {
                    let mut q = app.queue.lock().expect("queue");
                    q.tracks = tracks;
                    q.index = None;
                    q.source = "Плейлист".into();
                    q.wave = None;
                }
                app.ui_inv(|ui| ui.set_wave_active(false));
                app.spawn_play(index as usize);
            }
        });
    }
    {
        let app = Arc::clone(app);
        ui.on_playlist_like_toggled(move |index| app.toggle_like_at(2, index as usize));
    }
    {
        let app = Arc::clone(app);
        ui.on_playlist_download(move |index| app.download_at(2, index as usize));
    }
}

pub(crate) fn init_wave(app: &Arc<App>, ui: &MainWindow) {
    {
        let app = Arc::clone(app);
        ui.on_wave_start(move || app.start_wave(vec!["user:onyourwave".into()], "Моя волна"));
    }
    {
        let app = Arc::clone(app);
        ui.on_wave_next(move || app.advance(true));
    }
    {
        let app = Arc::clone(app);
        ui.on_wave_dislike(move || {
            let wave = app.queue.lock().expect("queue").wave.clone();
            let track = {
                let q = app.queue.lock().expect("queue");
                q.index.and_then(|i| q.tracks.get(i).cloned())
            };
            if let (Some(w), Some(t)) = (wave, track) {
                let app = Arc::clone(&app);
                app.spawn(|app| async move {
                    app.wave_feedback(&w, "dislike", Some(&t.composite_id()), None)
                        .await;
                });
            }
            app.advance(true);
        });
    }
    {
        let app = Arc::clone(app);
        ui.on_wave_play_index(move |index| {
            let len = app.queue.lock().expect("queue").tracks.len();
            if (index as usize) < len {
                app.spawn_play(index as usize);
            }
        });
    }
    {
        let app = Arc::clone(app);
        ui.on_wave_like_toggled(move |_index| {
            let track = {
                let q = app.queue.lock().expect("queue");
                q.index.and_then(|i| q.tracks.get(i).cloned())
            };
            if let Some(t) = track {
                let app = Arc::clone(&app);
                app.spawn(|app| async move { app.toggle_like(t).await });
            }
        });
    }
    {
        let app = Arc::clone(app);
        ui.on_radio_start(move |seed, name| {
            app.start_wave(vec![seed.to_string()], &name);
        });
    }
}

pub(crate) fn init_settings(app: &Arc<App>, ui: &MainWindow) {
    {
        let app = Arc::clone(app);
        ui.on_settings_set_theme(move |preset| {
            let preset = preset.to_string();
            {
                let mut cfg = app.config.lock().expect("config");
                cfg.theme.preset = preset.clone();
                let _ = cfg.save();
            }
            let theme = {
                let cfg = app.config.lock().expect("config").clone();
                crate::bridge::theme_from_config(&cfg.theme.preset, &cfg.theme.accent_override)
            };
            app.ui_inv(move |ui| {
                crate::bridge::apply_theme(ui, &theme);
                ui.set_settings_preset(preset.clone().into());
            });
        });
    }
    {
        let app = Arc::clone(app);
        ui.on_settings_set_accent(move |accent| {
            let accent = accent.to_string();
            let override_hex = if accent.is_empty() {
                None
            } else {
                Some(accent.clone())
            };
            {
                let mut cfg = app.config.lock().expect("config");
                cfg.theme.accent_override = override_hex.clone();
                let _ = cfg.save();
            }
            let theme = {
                let cfg = app.config.lock().expect("config").clone();
                crate::bridge::theme_from_config(&cfg.theme.preset, &override_hex)
            };
            app.ui_inv(move |ui| {
                crate::bridge::apply_theme(ui, &theme);
                ui.set_settings_accent(accent.clone().into());
            });
        });
    }
    {
        let app = Arc::clone(app);
        ui.on_settings_pick_dir(move || {
            let app = Arc::clone(&app);
            app.spawn(|app| async move {
                let picked = rfd::AsyncFileDialog::new()
                    .set_title("Каталог загрузок")
                    .pick_folder()
                    .await;
                if let Some(folder) = picked {
                    let path =
                        std::path::PathBuf::from(folder.path().to_string_lossy().to_string());
                    {
                        let mut cfg = app.config.lock().expect("config");
                        cfg.downloads.dir = path.clone();
                        let _ = cfg.save();
                    }
                    let quality = app.config.lock().expect("config").downloads.quality;
                    app.downloads.configure(path.clone(), quality);
                    let label = path.to_string_lossy().to_string();
                    app.ui_inv(move |ui| {
                        ui.set_downloads_dir(label.into());
                    });
                }
            });
        });
    }
    {
        let app = Arc::clone(app);
        ui.on_settings_set_quality(move |q| {
            let quality = match q {
                0 => Quality::Low,
                1 => Quality::Normal,
                _ => Quality::High,
            };
            let dir = {
                let mut cfg = app.config.lock().expect("config");
                cfg.downloads.quality = quality;
                let _ = cfg.save();
                cfg.downloads.dir.clone()
            };
            app.downloads.configure(dir, quality);
            app.ui_inv(move |ui| ui.set_settings_quality(q));
        });
    }
    {
        let app = Arc::clone(app);
        ui.on_settings_clear_cache(move || {
            let app = Arc::clone(&app);
            app.spawn(|app| async move {
                if let Err(e) = app.cache.clear() {
                    log::error!("clear cache: {e}");
                }
                let size = crate::app::format_bytes(app.cache.size_bytes());
                app.ui_inv(move |ui| ui.set_settings_cache_size(size.into()));
            });
        });
    }
    {
        let app = Arc::clone(app);
        ui.on_settings_logout(move || {
            let app = Arc::clone(&app);
            app.spawn(|app| async move {
                app.login_cancel.store(true, Ordering::Relaxed);
                let mut storage = TokenStorage::default();
                let _ = storage.purge();
                app.api.lock().expect("api").set_token(None);
                app.ui_inv(|ui| {
                    ui.set_logged_in(false);
                    ui.set_login_state(0);
                    ui.set_page(0);
                });
            });
        });
    }
}

// ----------------------------------------------------------------------
// Логика, привязанная к контекстам списков
// ----------------------------------------------------------------------

impl App {
    /// Выполняет поиск и обновляет модели.
    async fn run_search(self: &Arc<Self>, text: &str) {
        self.ui_inv(|ui| {
            ui.set_searching(true);
            ui.set_has_results(false);
        });
        let client = self.api.lock().expect("api").clone();
        let result = client.search_tracks(text, 0).await;
        match result {
            Ok(search) => {
                let mut cov = models::CoversSnapshot::default();
                let tracks = search
                    .tracks
                    .as_ref()
                    .map(|b| b.results.clone())
                    .unwrap_or_default();
                self.fill_track_covers(&tracks, &mut cov).await;
                // Обложка лучшего результата-альбома.
                if let Some(al) = search.best.as_ref().and_then(|b| b.as_album()) {
                    if let Some(url) = al.cover_url("300x300") {
                        let path = self.cache.cover(&url).await;
                        cov.album_covers.insert(al.id, path);
                    }
                }
                {
                    let mut cache = self.ui_cache.lock().expect("cache");
                    cache.search = tracks.clone();
                    cache.covers.track_covers.extend(cov.track_covers.clone());
                }
                let best_src = search.best.clone();
                let total = search.tracks.as_ref().map(|b| b.total).unwrap_or(0) as i32;
                self.ui_inv(move |ui| {
                    let best = models::best_item(&best_src, &cov);
                    let has_best = best.is_some();
                    ui.set_searching(false);
                    ui.set_has_results(true);
                    ui.set_has_best(has_best);
                    if let Some(b) = &best {
                        ui.set_search_best(b.clone());
                    }
                    ui.set_search_total(total);
                });
                self.rebuild_models();
            }
            Err(e) => {
                log::error!("search: {e}");
                self.ui_inv(|ui| {
                    ui.set_searching(false);
                    ui.set_has_results(false);
                });
            }
        }
    }

    /// Играет трек из контекста: 0 — поиск, 1 — лучший результат.
    fn play_from_context(self: &Arc<Self>, index: usize, context: u8) {
        let tracks: Vec<Track> = {
            let cache = self.ui_cache.lock().expect("cache");
            if context == 1 {
                // Лучший результат + остальные
                let mut all = Vec::new();
                let best = self.ui.upgrade().map(|ui| ui.get_search_best());
                if let Some(best) = best {
                    if best.item_type == 0 && best.track_id != 0 {
                        if let Some(t) = cache.search.iter().find(|t| t.id as i32 == best.track_id)
                        {
                            all.push(t.clone());
                        }
                    }
                }
                all.extend(cache.search.iter().cloned());
                all
            } else {
                cache.search.clone()
            }
        };
        if tracks.is_empty() {
            return;
        }
        {
            let mut q = self.queue.lock().expect("queue");
            q.tracks = tracks;
            q.index = None;
            q.source = "Поиск".into();
            q.wave = None;
        }
        self.ui_inv(|ui| ui.set_wave_active(false));
        let idx = if context == 1 { 0 } else { index };
        let len = self.queue.lock().expect("queue").tracks.len();
        self.spawn_play(idx.min(len.saturating_sub(1)));
    }

    /// Лайк трека из контекста (0 — поиск, 2 — плейлист).
    fn toggle_like_at(self: &Arc<Self>, context: u8, index: usize) {
        let track = {
            let cache = self.ui_cache.lock().expect("cache");
            let list = if context == 0 {
                &cache.search
            } else {
                &cache.playlist_tracks
            };
            list.get(index).cloned()
        };
        if let Some(t) = track {
            let _app = Arc::clone(self);
            self.spawn(|app| async move { app.toggle_like(t).await });
        }
    }

    /// Переключает лайк трека (API + кеш + UI).
    async fn toggle_like(self: &Arc<Self>, track: Track) {
        let uid = *self.uid.lock().expect("uid");
        let liked = self
            .ui_cache
            .lock()
            .expect("cache")
            .covers
            .likes
            .contains(&track.id);
        let client = self.api.lock().expect("api").clone();
        let result = if liked {
            client.likes_tracks_remove(uid, &[track.id]).await
        } else {
            client.likes_tracks_add(uid, &[track.id]).await
        };
        if let Err(e) = result {
            log::error!("like: {e}");
            return;
        }
        {
            let mut cache = self.ui_cache.lock().expect("cache");
            if liked {
                cache.covers.likes.remove(&track.id);
            } else {
                cache.covers.likes.insert(track.id);
            }
        }
        self.rebuild_models();
    }

    /// Скачать трек из контекста (0 — поиск, 2 — плейлист).
    fn download_at(self: &Arc<Self>, context: u8, index: usize) {
        let track = {
            let cache = self.ui_cache.lock().expect("cache");
            let list = if context == 0 {
                &cache.search
            } else {
                &cache.playlist_tracks
            };
            list.get(index).cloned()
        };
        if let Some(t) = track {
            self.enqueue_download(t);
        }
    }

    /// Ставит трек в загрузку и на страницу «Скачанное».
    fn enqueue_download(self: &Arc<Self>, track: Track) {
        if !self
            .ui
            .upgrade()
            .map(|ui| ui.get_has_subscription())
            .unwrap_or(false)
        {
            log::warn!("скачивание требует подписки");
            return;
        }
        {
            let mut cache = self.ui_cache.lock().expect("cache");
            if !cache.downloads.iter().any(|t| t.id == track.id) {
                cache.downloads.push(track.clone());
            }
        }
        let track2 = track.clone();
        let _app = Arc::clone(self);
        self.spawn(|app| async move {
            app.downloads.enqueue(track2, &app.rt);
        });
        self.rebuild_models();
    }

    /// Новинки/чарт → плитки «Собрано для вас».
    async fn load_landing(self: &Arc<Self>, kind: &str) {
        let client = self.api.lock().expect("api").clone();
        if kind == "chart" {
            let tracks = match client.landing_chart().await {
                Ok(t) => t,
                Err(e) => {
                    log::error!("chart: {e}");
                    return;
                }
            };
            let mut cov = models::CoversSnapshot::default();
            self.fill_track_covers(&tracks, &mut cov).await;
            self.ui_inv(move |ui| {
                let tiles: Vec<crate::TileItem> = tracks
                    .iter()
                    .map(|t| crate::TileItem {
                        title: models::s(t.title.clone()),
                        subtitle: models::s(t.artists_display()),
                        cover: cov
                            .track_covers
                            .get(&t.id)
                            .and_then(|p| {
                                slint::Image::load_from_path(std::path::Path::new(p)).ok()
                            })
                            .unwrap_or_default(),
                        cover_variant: models::cover_variant(t.id),
                        item_type: 1,
                        owner_uid: 0,
                        kind: t.id as i32,
                    })
                    .collect();
                ui.set_home_generated(slint::ModelRc::new(std::rc::Rc::new(
                    slint::VecModel::from(tiles),
                )));
            });
        } else {
            let albums = match client.landing_new_releases().await {
                Ok(a) => a,
                Err(e) => {
                    log::error!("new releases: {e}");
                    return;
                }
            };
            // Обложки альбомов (путями — Image создаётся в event loop).
            let mut cov = models::CoversSnapshot::default();
            for a in &albums {
                if let Some(url) = a.cover_url("300x300") {
                    let path = self.cache.cover(&url).await;
                    cov.album_covers.insert(a.id, path);
                }
            }
            self.ui_inv(move |ui| {
                let tiles: Vec<crate::TileItem> =
                    albums.iter().map(|a| models::album_tile(a, &cov)).collect();
                ui.set_home_generated(slint::ModelRc::new(std::rc::Rc::new(
                    slint::VecModel::from(tiles),
                )));
            });
        }
    }

    /// Открывает плейлист на странице библиотеки.
    async fn open_playlist(self: &Arc<Self>, owner_uid: i64, kind: i64) {
        let client = self.api.lock().expect("api").clone();
        let playlist = client.user_playlist(owner_uid, kind).await;
        match playlist {
            Ok(pl) => {
                let mut cov = models::CoversSnapshot::default();
                self.fill_track_covers(&pl.tracks, &mut cov).await;
                if let Some(url) = pl.cover_url("300x300") {
                    let path = self.cache.cover(&url).await;
                    cov.playlist_covers.insert((pl.uid, pl.kind), path);
                }
                let title = pl.title.clone();
                let subtitle = format!(
                    "{} · {} треков",
                    pl.owner_display(),
                    pl.track_count.unwrap_or(pl.tracks.len() as i64)
                );
                {
                    let mut cache = self.ui_cache.lock().expect("cache");
                    cache.playlist_tracks = pl.tracks.clone();
                    cache.covers.track_covers.extend(cov.track_covers.clone());
                }
                self.ui_inv(move |ui| {
                    ui.set_playlist_title(models::s(title));
                    ui.set_playlist_subtitle(models::s(subtitle));
                    ui.set_playlist_open(true);
                    ui.set_page(2);
                });
                self.rebuild_models();
            }
            Err(e) => log::error!("open playlist: {e}"),
        }
    }

    /// Открывает «Мне нравится» как плейлист.
    async fn open_likes(self: &Arc<Self>) {
        let uid = *self.uid.lock().expect("uid");
        let client = self.api.lock().expect("api").clone();
        let likes = client.users_likes_tracks(uid).await;
        match likes {
            Ok(likes) => {
                let mut cov = models::CoversSnapshot::default();
                self.fill_track_covers(&likes.tracks, &mut cov).await;
                {
                    let mut cache = self.ui_cache.lock().expect("cache");
                    cache.playlist_tracks = likes.tracks.clone();
                    cache.covers.track_covers.extend(cov.track_covers.clone());
                }
                let count = likes.tracks.len();
                self.ui_inv(move |ui| {
                    ui.set_playlist_title("Мне нравится".into());
                    ui.set_playlist_subtitle(format!("{count} треков").into());
                    ui.set_playlist_open(true);
                    ui.set_page(2);
                });
                self.rebuild_models();
            }
            Err(e) => log::error!("likes: {e}"),
        }
    }

    /// Открывает альбом на странице библиотеки (`GET /albums/{id}/with-tracks`).
    async fn open_album(self: &Arc<Self>, album_id: i64) {
        let client = self.api.lock().expect("api").clone();
        let album = match client.album_with_tracks(album_id).await {
            Ok(a) => a,
            Err(e) => {
                log::error!("open album {album_id}: {e}");
                return;
            }
        };
        let tracks = album.all_tracks();
        if tracks.is_empty() {
            log::warn!("альбом {album_id} без треков");
            return;
        }
        let mut cov = models::CoversSnapshot::default();
        self.fill_track_covers(&tracks, &mut cov).await;
        if let Some(url) = album.album.cover_url("400x400") {
            let path = self.cache.cover(&url).await;
            cov.album_covers.insert(album.album.id, path);
        }
        let title = album.album.title.clone();
        let artist_line = album
            .album
            .artists
            .iter()
            .map(|a| a.name.clone())
            .collect::<Vec<_>>()
            .join(", ");
        let subtitle = format!(
            "{}{} · {} треков",
            artist_line,
            album
                .album
                .year
                .map(|y| format!(" · {y}"))
                .unwrap_or_default(),
            tracks.len()
        );
        {
            let mut cache = self.ui_cache.lock().expect("cache");
            cache.playlist_tracks = tracks.clone();
            cache.covers.track_covers.extend(cov.track_covers.clone());
        }
        self.ui_inv(move |ui| {
            ui.set_playlist_title(models::s(title));
            ui.set_playlist_subtitle(models::s(subtitle));
            ui.set_playlist_open(true);
            ui.set_page(2);
        });
        self.rebuild_models();
    }

    /// Открывает карточку исполнителя с популярными треками
    /// (`GET /artists/{id}/brief-info`).
    async fn open_artist(self: &Arc<Self>, artist_id: i64) {
        let client = self.api.lock().expect("api").clone();
        let info = match client.artists_brief_info(artist_id).await {
            Ok(i) => i,
            Err(e) => {
                log::error!("open artist {artist_id}: {e}");
                return;
            }
        };
        let tracks = info.popular_tracks;
        if tracks.is_empty() {
            log::warn!("у исполнителя {} нет треков", info.artist.name);
            return;
        }
        let mut cov = models::CoversSnapshot::default();
        self.fill_track_covers(&tracks, &mut cov).await;
        if let Some(url) = info.artist.cover.as_ref().and_then(|c| c.url("400x400")) {
            let path = self.cache.cover(&url).await;
            cov.artist_covers.insert(info.artist.id, path);
        }
        let title = info.artist.name.clone();
        let count = tracks.len();
        let subtitle = format!("Популярные треки · {count}");
        {
            let mut cache = self.ui_cache.lock().expect("cache");
            cache.playlist_tracks = tracks.clone();
            cache.covers.track_covers.extend(cov.track_covers.clone());
        }
        self.ui_inv(move |ui| {
            ui.set_playlist_title(models::s(title));
            ui.set_playlist_subtitle(models::s(subtitle));
            ui.set_playlist_open(true);
            ui.set_page(2);
        });
        self.rebuild_models();
    }
}
