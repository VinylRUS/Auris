//! Точка входа Auris.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod bridge;
mod callbacks;
mod demo;
mod logging;
mod media;
mod models;
mod playback;
mod qr;
mod update;

slint::include_modules!();

use std::sync::Arc;

fn main() -> anyhow::Result<()> {
    // Журнал — в папку установки (logs/auris.log), эхо — в консоль.
    // Инициализируем первым: всё, что ниже, уже попадает в файл.
    logging::init();
    // Настройки/токен/история из старых системных каталогов —
    // переносим к программе (однократно, копированием).
    auris_core::paths::migrate_legacy_files();

    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()?;

    let config = auris_core::Config::load()?;
    let cache = auris_core::cache::Cache::new(config.cache_root(), default_http_client())?;
    let mut tokens = auris_core::TokenStorage::default();
    let token = tokens.load().unwrap_or(None);

    // Без токена — неавторизованный клиент (пустая строка здесь ранее
    // считалась «авторизацией» и пропускала экран входа).
    let api = Arc::new(std::sync::Mutex::new(match &token {
        Some(t) if !t.is_empty() => auris_api::Client::with_token(t.clone()),
        _ => auris_api::Client::new(),
    }));

    let (player, player_events) = auris_player::spawn()?;

    let ui = MainWindow::new()?;

    // Явный размер окна: восстановленный из настроек или дефолт.
    // ВАЖНО: без set_size окно создаётся по preferred-размеру корневого
    // layout, который на экране входа пуст → winit-дефолт 800×600.
    // set_size задаёт размер детерминированно (и флаг has_explicit_size
    // отключает дальнейшие авто-подгонки под layout).
    {
        let (w, h) = {
            let cw = config.window.width;
            let ch = config.window.height;
            if (960.0..=3840.0).contains(&cw) && (600.0..=2160.0).contains(&ch) {
                (cw, ch)
            } else {
                (1280.0, 800.0)
            }
        };
        ui.window().set_size(slint::LogicalSize::new(w, h));
    }

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

    // Демо-режим UI: AURIS_UI_DEMO=1 — мок-данные без сети.
    if std::env::var_os("AURIS_UI_DEMO").is_some() {
        demo::fill(&ui);
    } else {
        app.bootstrap();

        // Тихая проверка обновлений в фоне.
        update::startup_check(&app);
    }

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
