//! Оркестратор приложения: состояние, колбэки UI, вход, настройки.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::Receiver;
use std::sync::{Arc, Mutex};

use auris_api::{auth, Client, Playlist, Track};
use auris_core::cache::Cache;
use auris_core::downloads::DownloadManager;
use auris_core::token::StorageKind;
use auris_core::{Config, Quality, TokenStorage};
use auris_player::{PlayerEvent, PlayerHandle};
use slint::ComponentHandle;
use slint::Weak;

use crate::models::{self, LyricsUi, SharedCache};
use crate::MainWindow;

/// Состояние очереди.
pub struct Queue {
    /// Треки.
    pub tracks: Vec<Track>,
    /// Текущий индекс.
    pub index: Option<usize>,
    /// Подпись источника («Поиск: …», «Моя волна»…).
    pub source: String,
    /// Активная сессия волны.
    pub wave: Option<WaveSession>,
    /// Перемешивание.
    pub shuffle: bool,
    /// Повтор: 0 выкл, 1 все, 2 один.
    pub repeat: u8,
}

/// Сессия радио.
#[derive(Clone)]
pub struct WaveSession {
    /// Идентификатор сессии.
    pub id: String,
    /// Идентификатор партии.
    pub batch_id: Option<String>,
    /// Название станции (для будущих фич: отображение в UI плеера ОС).
    #[allow(dead_code)]
    pub label: String,
}

/// Приложение: общее состояние и сервисы.
pub struct App {
    /// Хэндл tokio.
    pub rt: tokio::runtime::Handle,
    /// API-клиент (токен обновляется на месте).
    pub api: Arc<Mutex<Client>>,
    /// Плеер.
    pub player: PlayerHandle,
    /// События плеера (читает таймер).
    pub player_events: Arc<Mutex<Receiver<PlayerEvent>>>,
    /// Кеш.
    pub cache: Cache,
    /// Настройки.
    pub config: Arc<Mutex<Config>>,
    /// Менеджер загрузок.
    pub downloads: DownloadManager,
    /// Очередь.
    pub queue: Arc<Mutex<Queue>>,
    /// Кеш UI.
    pub ui_cache: SharedCache,
    /// История.
    pub history: Arc<Mutex<auris_core::history::History>>,
    /// Отмена логина.
    pub login_cancel: Arc<AtomicBool>,
    /// Отмена скачивания обновления.
    pub update_cancel: Arc<AtomicBool>,
    /// UID аккаунта.
    pub uid: Arc<Mutex<i64>>,
    /// Поколение запроса подсказок (для дебаунса).
    pub suggest_gen: AtomicU64,
    /// Слабая ссылка на окно.
    pub ui: Weak<MainWindow>,
    /// Где хранится токен.
    pub storage_kind: Arc<Mutex<Option<StorageKind>>>,
    #[cfg(feature = "media-controls")]
    /// Системные медиа-контролы (SMTC/MPRIS).
    pub media: Mutex<Option<playwire::MediaControls>>,
}

impl App {
    /// Создаёт приложение.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        rt: tokio::runtime::Handle,
        api: Arc<Mutex<Client>>,
        player: PlayerHandle,
        player_events: Receiver<PlayerEvent>,
        cache: Cache,
        config: Config,
        tokens: TokenStorage,
        ui: Weak<MainWindow>,
    ) -> Self {
        let downloads = DownloadManager::new(
            Arc::clone(&api),
            config.downloads.dir.clone(),
            config.downloads.quality,
        );
        let history = Arc::new(Mutex::new(auris_core::history::History::load()));
        Self {
            rt,
            api,
            player: player.clone(),
            player_events: Arc::new(Mutex::new(player_events)),
            cache,
            config: Arc::new(Mutex::new(config)),
            downloads,
            queue: Arc::new(Mutex::new(Queue {
                tracks: Vec::new(),
                index: None,
                source: String::new(),
                wave: None,
                shuffle: false,
                repeat: 0,
            })),
            ui_cache: Arc::new(Mutex::new(models::UiCache::default())),
            history,
            login_cancel: Arc::new(AtomicBool::new(false)),
            update_cancel: Arc::new(AtomicBool::new(false)),
            suggest_gen: AtomicU64::new(0),
            uid: Arc::new(Mutex::new(0)),
            ui,
            storage_kind: Arc::new(Mutex::new(tokens.kind)),
            #[cfg(feature = "media-controls")]
            media: Mutex::new(None),
        }
    }

    /// Запустить async-задачу, передав ей клон `Arc<App>`
    /// (избавляет от конфликтов заимствования при захвате в future).
    pub fn spawn<F, Fut>(self: &Arc<Self>, f: F)
    where
        F: FnOnce(Arc<App>) -> Fut,
        Fut: std::future::Future<Output = ()> + Send + 'static,
    {
        let app = Arc::clone(self);
        self.rt.spawn(f(app));
    }

    /// Обновить UI из потока задач.
    pub fn ui_inv<F: FnOnce(&MainWindow) + Send + 'static>(&self, f: F) {
        let weak = self.ui.clone();
        let _ = slint::invoke_from_event_loop(move || {
            if let Some(ui) = weak.upgrade() {
                f(&ui);
            }
        });
    }

    /// Финализация: сохранить настройки.
    pub fn shutdown(&self) {
        if let Some(ui) = self.ui.upgrade() {
            let size = ui.window().size();
            let mut cfg = self.config.lock().expect("config");
            cfg.window.width = size.width as f32;
            cfg.window.height = size.height as f32;
            if let Err(e) = cfg.save() {
                log::warn!("config save: {e}");
            }
        }
    }

    // ------------------------------------------------------------------
    // Загрузка
    // ------------------------------------------------------------------

    /// Первичная загрузка: если есть токен — тянем аккаунт и данные.
    pub fn bootstrap(self: &Arc<Self>) {
        let _app = Arc::clone(self);
        self.spawn(|app| async move {
            let authorized = app.api.lock().expect("api").is_authorized();
            if !authorized {
                app.ui_inv(|ui| ui.set_logged_in(false));
                return;
            }
            match app.load_account().await {
                Ok(()) => {
                    app.ui_inv(|ui| ui.set_logged_in(true));
                    app.load_home().await;
                    app.load_radio().await;
                }
                Err(e) => {
                    log::error!("account: {e:#}");
                    app.ui_inv(|ui| {
                        ui.set_logged_in(false);
                        ui.set_login_state(2);
                        ui.set_login_error("Токен недействителен — войдите заново".into());
                    });
                }
            }
        });
    }

    /// Статус аккаунта → UI.
    pub async fn load_account(self: &Arc<Self>) -> anyhow::Result<()> {
        let client = self.api.lock().expect("api").clone();
        let status = client
            .account_status()
            .await
            .map_err(|e| anyhow::anyhow!("{e}"))?;

        let name = status.display_name();
        let initial = name
            .chars()
            .next()
            .map(|c| c.to_uppercase().to_string())
            .unwrap_or_default();
        let login = status
            .account
            .as_ref()
            .map(|a| a.login.clone())
            .unwrap_or_default();
        let has_sub = status.has_subscription();
        *self.uid.lock().expect("uid") = status.uid();

        let storage_kind = self.storage_kind.clone();
        self.ui_inv(move |ui| {
            ui.set_user_name(models::s(name));
            ui.set_user_initial(models::s(initial));
            ui.set_user_login(models::s(login));
            ui.set_has_subscription(has_sub);
            ui.set_user_plan(if has_sub {
                "Подписка активна".into()
            } else {
                "Без подписки".into()
            });
            let kind = match &*storage_kind.lock().unwrap() {
                Some(StorageKind::Keyring) => "keyring",
                Some(StorageKind::File) => "файл",
                None => "",
            };
            ui.set_settings_storage(kind.into());
        });
        Ok(())
    }

    /// Подсказки поиска с дебаунсом 350 мс (`GET /search/suggest`).
    pub async fn suggest(self: &Arc<Self>, part: &str) {
        let part = part.trim().to_string();
        let gen = self.suggest_gen.fetch_add(1, Ordering::Relaxed) + 1;
        if part.len() < 2 {
            self.clear_suggestions();
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(350)).await;
        // Пользователь продолжил печатать — этот запрос устарел.
        if self.suggest_gen.load(Ordering::Relaxed) != gen {
            return;
        }
        let client = self.api.lock().expect("api").clone();
        match client.search_suggest(&part).await {
            Ok(items) if !items.is_empty() => {
                if self.suggest_gen.load(Ordering::Relaxed) != gen {
                    return;
                }
                let top: Vec<slint::SharedString> =
                    items.iter().take(8).map(|s| models::s(s.clone())).collect();
                self.ui_inv(move |ui| {
                    ui.set_search_suggestions(slint::ModelRc::new(std::rc::Rc::new(
                        slint::VecModel::from(top),
                    )));
                });
            }
            _ => self.clear_suggestions(),
        }
    }

    /// Скрывает попап подсказок.
    pub fn clear_suggestions(self: &Arc<Self>) {
        self.ui_inv(move |ui| {
            ui.set_search_suggestions(slint::ModelRc::new(std::rc::Rc::new(
                slint::VecModel::from(Vec::<slint::SharedString>::new()),
            )));
        });
    }

    /// Домашняя страница: лента, плейлисты, лайки, история.
    pub async fn load_home(self: &Arc<Self>) {
        if let Err(e) = self.load_home_inner().await {
            log::error!("home: {e:#}");
        }
    }

    /// Радио: станции из дашборда ротатора (`GET /rotor/stations/dashboard`).
    pub async fn load_radio(self: &Arc<Self>) {
        let client = self.api.lock().expect("api").clone();
        let dash = match client.rotor_stations_dashboard().await {
            Ok(d) => d,
            Err(e) => {
                log::warn!("radio dashboard: {e}");
                return;
            }
        };
        let mut seen = std::collections::HashSet::new();
        let items: Vec<(String, String)> = dash
            .dashboard
            .stations
            .iter()
            .filter_map(|s| {
                let st = s.station.as_ref()?;
                (!st.name.is_empty()).then_some((st.name.clone(), s.id.seed()))
            })
            .filter(|(name, seed)| name != "" && seen.insert(seed.clone()))
            .collect();
        if items.is_empty() {
            return;
        }
        self.ui_inv(move |ui| {
            let model: Vec<crate::StationItem> = items
                .iter()
                .enumerate()
                .map(|(i, (name, seed))| crate::StationItem {
                    label: models::s(name.clone()),
                    seed: models::s(seed.clone()),
                    variant: (i % 8) as i32 + 1,
                })
                .collect();
            ui.set_radio_stations(slint::ModelRc::new(std::rc::Rc::new(
                slint::VecModel::from(model),
            )));
        });
    }

    async fn load_home_inner(self: &Arc<Self>) -> anyhow::Result<()> {
        let uid = *self.uid.lock().expect("uid");
        let client = self.api.lock().expect("api").clone();
        let feed = client
            .feed()
            .await
            .map_err(|e| anyhow::anyhow!("feed: {e}"))?;
        let playlists = client
            .users_playlists_list(uid)
            .await
            .map_err(|e| anyhow::anyhow!("playlists: {e}"))?;
        let (likes, likes_albums, likes_artists) = tokio::join!(
            client.users_likes_tracks(uid),
            client.users_likes_albums(uid),
            client.users_likes_artists(uid)
        );
        let likes = likes.map_err(|e| anyhow::anyhow!("likes: {e}"))?;
        // Лайки альбомов/исполнителей не критичны — при ошибке просто пустые.
        let likes_albums = likes_albums.unwrap_or_default();
        let likes_artists = likes_artists.unwrap_or_default();

        // Лайки: множество id
        {
            let mut cache = self.ui_cache.lock().expect("cache");
            cache.covers.likes = likes.tracks.iter().map(|t| t.id).collect();
        }

        // Обложки (путями — Image создаётся в event loop)
        let mut cov = models::CoversSnapshot::default();
        self.fill_track_covers(&likes.tracks, &mut cov).await;
        for pl in &playlists {
            if let Some(url) = pl.cover_url("300x300") {
                let path = self.cache.cover(&url).await;
                cov.playlist_covers.insert((pl.uid, pl.kind), path);
            }
        }
        for la in &likes_albums {
            if let Some(url) = la.album.cover_url("300x300") {
                let path = self.cache.cover(&url).await;
                cov.album_covers.insert(la.album.id, path);
            }
        }
        for la in &likes_artists {
            if let Some(url) = la.artist.cover.as_ref().and_then(|c| c.url("300x300")) {
                let path = self.cache.cover(&url).await;
                cov.artist_covers.insert(la.artist.id, path);
            }
        }
        {
            let mut cache = self.ui_cache.lock().expect("cache");
            cache.covers.playlist_covers = cov.playlist_covers.clone();
            cache.covers.track_covers.extend(cov.track_covers.clone());
        }

        // Плитки собираем в event loop (Image не Send)
        let generated_src: Vec<Playlist> = feed
            .generated_playlists
            .iter()
            .filter_map(|g| g.playlist.clone())
            .take(4)
            .collect();
        let likes_count = likes.tracks.len();
        let recent: Vec<Track> = self.history.lock().expect("history").recent(4);
        let albums_src: Vec<auris_api::Album> =
            likes_albums.iter().map(|la| la.album.clone()).collect();
        let artists_src: Vec<auris_api::Artist> =
            likes_artists.iter().map(|la| la.artist.clone()).collect();
        let greeting = greeting_now();
        self.ui_inv(move |ui| {
            let generated: Vec<crate::TileItem> = generated_src
                .iter()
                .map(|pl| models::playlist_tile(pl, &cov))
                .collect();
            let mut lib: Vec<crate::TileItem> = vec![models::likes_tile(likes_count)];
            lib.extend(playlists.iter().map(|pl| models::playlist_tile(pl, &cov)));
            let history_tiles: Vec<crate::TileItem> = recent
                .iter()
                .map(|t| models::history_tile(t, &cov))
                .collect();

            ui.set_greeting(models::s(greeting));
            ui.set_home_generated(slint::ModelRc::new(std::rc::Rc::new(
                slint::VecModel::from(generated),
            )));
            ui.set_home_playlists(slint::ModelRc::new(std::rc::Rc::new(
                slint::VecModel::from(lib.clone()),
            )));
            ui.set_library_playlists(slint::ModelRc::new(std::rc::Rc::new(
                slint::VecModel::from(lib),
            )));
            let albums: Vec<crate::TileItem> = albums_src
                .iter()
                .map(|a| models::album_tile(a, &cov))
                .collect();
            let artists: Vec<crate::TileItem> = artists_src
                .iter()
                .map(|a| models::artist_tile(a, &cov))
                .collect();
            ui.set_library_albums(slint::ModelRc::new(std::rc::Rc::new(
                slint::VecModel::from(albums),
            )));
            ui.set_library_artists(slint::ModelRc::new(std::rc::Rc::new(
                slint::VecModel::from(artists),
            )));
            ui.set_home_recent(slint::ModelRc::new(std::rc::Rc::new(
                slint::VecModel::from(history_tiles),
            )));
        });
        Ok(())
    }

    // ------------------------------------------------------------------
    // Текст песни
    // ------------------------------------------------------------------

    /// Переключает панель текста песни (кнопка в плеере).
    /// Владелец состояния — Rust: Slint только вызывает этот колбэк.
    pub fn toggle_lyrics_panel(self: &Arc<Self>) {
        let open = self
            .ui
            .upgrade()
            .map(|ui| !ui.get_lyrics_open())
            .unwrap_or(false);
        self.ui_inv(move |ui| {
            ui.set_lyrics_open(open);
            if open {
                ui.set_queue_open(false);
            }
        });
        if open {
            self.load_lyrics();
        }
    }

    /// Загружает текст текущего трека: кеш → `GET /tracks/{id}/supplement`.
    pub fn load_lyrics(self: &Arc<Self>) {
        let track = {
            let q = self.queue.lock().expect("queue");
            q.index.and_then(|i| q.tracks.get(i).cloned())
        };
        let Some(track) = track else {
            self.ui_inv(|ui| {
                ui.set_lyrics_state(0);
                ui.set_lyrics_text("".into());
            });
            return;
        };
        // Кеш (включая отрицательный результат — «текста нет»).
        let cached = self
            .ui_cache
            .lock()
            .expect("cache")
            .lyrics
            .get(&track.id)
            .cloned();
        if let Some(cached) = cached {
            self.ui_inv(move |ui| ui.set_cached_lyrics(cached));
            return;
        }
        self.ui_inv(|ui| {
            ui.set_lyrics_state(1);
            ui.set_lyrics_text("".into());
        });
        let _app = Arc::clone(self);
        self.spawn(|app| async move {
            let client = app.api.lock().expect("api").clone();
            match client.track_supplement(track.id).await {
                Ok(sup) => {
                    let text = sup
                        .lyrics
                        .map(|l| l.text)
                        .unwrap_or_default()
                        .replace("\r\n", "\n");
                    let text = if text.trim().is_empty() {
                        None
                    } else {
                        Some(text.trim().to_string())
                    };
                    app.ui_cache
                        .lock()
                        .expect("cache")
                        .lyrics
                        .insert(track.id, text.clone());
                    app.ui_inv(move |ui| ui.set_cached_lyrics(text));
                }
                Err(e) => {
                    // Ошибку не кешируем — при следующем открытии повторим.
                    log::warn!("lyrics {}: {e}", track.id);
                    app.ui_inv(|ui| ui.set_lyrics_state(3));
                }
            }
        });
    }

    // ------------------------------------------------------------------
    // Вход
    // ------------------------------------------------------------------

    /// Запускает Device Flow.
    pub fn start_login(self: &Arc<Self>) {
        self.login_cancel.store(false, Ordering::Relaxed);
        let _app = Arc::clone(self);
        self.spawn(|app| async move {
            app.ui_inv(|ui| {
                ui.set_login_state(1);
                ui.set_login_ttl(-1);
            });
            let oauth = auris_api::OAuthClient::new();
            let device_id = random_device_id();
            let code = match auth::request_device_code(&oauth, &device_id, "Auris").await {
                Ok(c) => c,
                Err(e) => {
                    let msg =
                        format!("Не удалось получить код входа ({e}). Проверьте интернет и повторите попытку.");
                    app.ui_inv(move |ui| {
                        ui.set_login_state(2);
                        ui.set_login_error(msg.into());
                    });
                    return;
                }
            };

            // QR-код страницы подтверждения (грузим в event loop)
            let qr_path = app.cache.root.join("login-qr.png");
            let qr_ok = crate::qr::write_qr_png(&code.verification_url, &qr_path).is_ok();
            if !qr_ok {
                log::warn!("qr generation failed");
            }

            let user_code = code.user_code.clone();
            let url = code.verification_url.clone();
            app.ui_inv(move |ui| {
                ui.set_login_user_code(models::s(user_code));
                ui.set_login_url(models::s(url));
                let qr = if qr_ok {
                    slint::Image::load_from_path(&qr_path).unwrap_or_default()
                } else {
                    slint::Image::default()
                };
                ui.set_login_qr_image(qr);
            });

            let interval = std::time::Duration::from_secs(code.interval.max(1));
            let deadline =
                std::time::Instant::now() + std::time::Duration::from_secs(code.expires_in);
            let mut next_poll = std::time::Instant::now() + interval;
            let mut interval = interval;
            let mut fails = 0u32;
            // Сколько подряд неудачных опросов терпим, прежде чем сдаться
            // (сетевые заминки не должны убивать вход: у человека есть
            // целых `expires_in` секунд, чтобы достать телефон).
            // 12 попыток × интервал 5 с ≈ минута без связи.
            const MAX_FAILS: u32 = 12;
            loop {
                if app.login_cancel.load(Ordering::Relaxed) {
                    return;
                }
                let now = std::time::Instant::now();
                if now > deadline {
                    app.ui_inv(|ui| {
                        ui.set_login_state(2);
                        ui.set_login_error("Код истёк — попробуйте снова".into());
                    });
                    return;
                }

                // Обратный отсчёт: сколько секунд код ещё жив.
                let ttl = (deadline - now).as_secs() as i32;
                app.ui_inv(move |ui| ui.set_login_ttl(ttl));

                if now >= next_poll {
                    match auth::poll_token(&oauth, &code.device_code).await {
                        Ok(auth::PollOutcome::Token(token)) => {
                            app.finish_login(token.access_token).await;
                            return;
                        }
                        Ok(auth::PollOutcome::Pending) => {
                            fails = 0;
                        }
                        Ok(auth::PollOutcome::SlowDown) => {
                            // RFC 8628: увеличиваем интервал на 5 c.
                            fails = 0;
                            interval = (interval + std::time::Duration::from_secs(5))
                                .min(std::time::Duration::from_secs(30));
                        }
                        Ok(auth::PollOutcome::Expired) => {
                            app.ui_inv(|ui| {
                                ui.set_login_state(2);
                                ui.set_login_error("Код истёк — попробуйте снова".into());
                            });
                            return;
                        }
                        Ok(auth::PollOutcome::Denied) => {
                            app.ui_inv(|ui| {
                                ui.set_login_state(2);
                                ui.set_login_error(
                                    "Вход отклонён на странице подтверждения".into(),
                                );
                            });
                            return;
                        }
                        Err(e) => {
                            fails += 1;
                            if fails >= MAX_FAILS {
                                let msg =
                                    format!("Не удалось связаться с Яндекс ID ({e}). Проверьте интернет и повторите попытку.");
                                app.ui_inv(move |ui| {
                                    ui.set_login_state(2);
                                    ui.set_login_error(msg.into());
                                });
                                return;
                            }
                            log::warn!("token poll failed ({fails}/{MAX_FAILS}): {e}");
                        }
                    }
                    next_poll = std::time::Instant::now() + interval;
                }

                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            }
        });
    }

    /// Сохраняет токен и догружает аккаунт.
    pub async fn finish_login(self: &Arc<Self>, token: String) {
        {
            let mut storage = TokenStorage::default();
            if let Err(e) = storage.store(&token) {
                log::error!("token store: {e}");
            }
            *self.storage_kind.lock().expect("storage") = storage.kind;
        }
        self.api.lock().expect("api").set_token(Some(token));
        match self.load_account().await {
            Ok(()) => {
                self.ui_inv(|ui| {
                    ui.set_logged_in(true);
                    ui.set_login_state(0);
                    ui.set_login_ttl(-1);
                });
                self.load_home().await;
            }
            Err(e) => {
                log::error!("after login: {e:#}");
                let msg = format!("{e:#}");
                self.ui_inv(|ui| {
                    ui.set_login_state(2);
                    ui.set_login_error(msg.into());
                });
            }
        }
    }

    // ------------------------------------------------------------------
    // Инициализация
    // ------------------------------------------------------------------

    /// Регистрирует все колбэки UI и запускает таймер.
    pub fn init(app: &Arc<Self>, ui: &MainWindow) {
        crate::callbacks::init_window(app, ui);
        crate::callbacks::init_login(app, ui);
        crate::callbacks::init_search(app, ui);
        crate::callbacks::init_home(app, ui);
        crate::callbacks::init_player(app, ui);
        crate::callbacks::init_queue(app, ui);
        crate::callbacks::init_wave(app, ui);
        crate::callbacks::init_settings(app, ui);
        crate::update::init(app, ui);

        // Медиа-контролы ОС (SMTC/MPRIS)
        #[cfg(feature = "media-controls")]
        {
            *app.media.lock().expect("media") = crate::media::attach(app);
        }
        // Настройки: начальные значения
        let cfg = app.config.lock().expect("config").clone();
        let theme = crate::bridge::theme_from_config(&cfg.theme.preset, &cfg.theme.accent_override);
        crate::bridge::apply_theme(ui, &theme);
        ui.set_settings_preset(cfg.theme.preset.clone().into());
        ui.set_settings_accent(cfg.theme.accent_override.clone().unwrap_or_default().into());
        ui.set_settings_quality(match cfg.downloads.quality {
            Quality::Low => 0,
            Quality::Normal => 1,
            Quality::High => 2,
        });
        ui.set_downloads_dir(cfg.downloads.dir.to_string_lossy().to_string().into());
        ui.set_settings_version(display_version().into());
        ui.set_volume(cfg.volume);
        let cache_size = format_bytes(app.cache.size_bytes());
        ui.set_settings_cache_size(cache_size.into());

        // Громкость из настроек
        app.player.send(auris_player::Cmd::SetVolume(cfg.volume));
    }
}

/// Человекочитаемая версия: `0.0.1-alpha.1` → `0.0.1a`, `0.0.1-beta.1` →
/// `0.0.1b`, `0.0.1-beta.2` → `0.0.1c`, `0.0.1-rc.1` → `0.0.1rc`.
pub fn display_version() -> String {
    let v = env!("CARGO_PKG_VERSION");
    let Some((core, suffix)) = v.rsplit_once('-') else {
        return v.to_string();
    };
    let (kind, num) = match suffix.split_once('.') {
        Some((k, n)) => (k, n.parse::<u32>().unwrap_or(1).max(1)),
        None => (suffix, 1),
    };
    match kind {
        // alpha.1 → «a», beta.1 → «b», beta.2 → «c», beta.3 → «d»…
        "alpha" => format!("{core}{}", char::from(b'a' + (num - 1) as u8)),
        "beta" => format!("{core}{}", char::from(b'b' + (num - 1) as u8)),
        "rc" => format!("{core}rc{num}"),
        _ => v.to_string(),
    }
}

/// Приветствие по времени суток (UTC+3).
fn greeting_now() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let hour = (secs / 3600 + 3) % 24;
    match hour {
        6..=11 => "Доброе утро",
        12..=17 => "Добрый день",
        18..=22 => "Добрый вечер",
        _ => "Доброй ночи",
    }
    .to_string()
}

/// Человекочитаемый размер.
pub fn format_bytes(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["Б", "КБ", "МБ", "ГБ", "ТБ"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} {}", UNITS[0])
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}

/// Случайный device_id из 10 алфавитно-цифровых символов.
fn random_device_id() -> String {
    const CHARS: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    let mut seed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.subsec_nanos() as u64 ^ d.as_secs())
        .unwrap_or(0x1234_5678);
    (0..10)
        .map(|_| {
            seed = seed
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            let idx = ((seed >> 33) as usize) % CHARS.len();
            CHARS[idx] as char
        })
        .collect()
}
