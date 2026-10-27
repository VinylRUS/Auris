//! Точка входа Auris.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod bridge;
mod callbacks;
mod media;
mod models;
mod playback;
mod qr;
mod update;

slint::include_modules!();

use std::sync::Arc;

fn main() -> anyhow::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()?;

    let config = auris_core::Config::load()?;
    let cache = auris_core::cache::Cache::new(config.cache_root(), default_http_client())?;
    let mut tokens = auris_core::TokenStorage::default();
    let token = tokens.load().unwrap_or(None);

    let api = Arc::new(std::sync::Mutex::new(auris_api::Client::with_token(
        token.clone().unwrap_or_default(),
    )));

    let (player, player_events) = auris_player::spawn()?;

    let ui = MainWindow::new()?;

    let app = Arc::new(app::App::new(
        rt.handle().clone(),
        api,
        player,
        player_events,
        cache,
        config,
        tokens,
        ui.as_weak(),
    ));

    app::App::init(&app, &ui);
    app.bootstrap();

    // Тихая проверка обновлений в фоне.
    update::startup_check(&app);

    // Таймер опроса плеера (держим в main — Slint Timer не Send).
    let tick_app = Arc::clone(&app);
    let timer = slint::Timer::default();
    timer.start(
        slint::TimerMode::Repeated,
        std::time::Duration::from_millis(300),
        move || {
            tick_app.on_tick();
        },
    );
    std::mem::forget(timer); // живёт до конца процесса

    ui.run()?;

    // После закрытия окна: сохранить геометрию и настройки.
    app.shutdown();
    Ok(())
}

/// Общий HTTP-клиент с разумными таймаутами.
pub(crate) fn default_http_client() -> reqwest::Client {
    reqwest::Client::builder()
        .user_agent(concat!("Auris/", env!("CARGO_PKG_VERSION")))
        .connect_timeout(std::time::Duration::from_secs(10))
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .expect("HTTP client")
}
