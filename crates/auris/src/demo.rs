//! Демо-режим интерфейса: `AURIS_UI_DEMO=1` — окно с мок-данными, без сети.
//! Предназначен для разработки и скриншотов UI (обложки — градиентные заглушки).

use std::rc::Rc;

use slint::{Image, ModelRc, SharedString, VecModel};

use crate::{MainWindow, TileItem, TrackItem};

/// Заполнить окно демо-данными (вызывается вместо `bootstrap`).
pub fn fill(ui: &MainWindow) {
    log::info!("UI demo mode: мок-данные без сети");
    ui.set_logged_in(true);
    ui.set_user_name("Александр".into());
    ui.set_user_initial("А".into());
    ui.set_user_login("alex-demo".into());
    ui.set_user_plan("Подписка активна".into());
    ui.set_has_subscription(true);
    ui.set_greeting("Добрый вечер".into());

    // ---- Плеер ----
    ui.set_cur_title("Демо-трек".into());
    ui.set_cur_artists("Auris Test Ensemble".into());
    ui.set_cur_cover_variant(3);
    ui.set_playing(true);
    ui.set_cur_liked(true);
    ui.set_position_sec(73.0);
    ui.set_duration_sec(213.0);
    ui.set_volume(0.72);
    ui.set_shuffle(true);
    ui.set_repeat(1);

    // ---- Треки ----
    let tracks: Vec<TrackItem> = vec![
        track(
            1,
            "Полночный трафик",
            "Kedr Livanskiy",
            "Lopussa",
            214,
            true,
            true,
            true,
            0,
        ),
        track(
            2,
            "Синевата мечта",
            "Молчат Дома",
            "Этажи",
            245,
            false,
            false,
            true,
            0,
        ),
        track(
            3,
            "Ночных дорогов свет",
            "ssshhhiiittt!",
            "Побег",
            198,
            false,
            false,
            false,
            2,
        ),
        track(
            4,
            "Далеко идти",
            "Три дня дождя",
            "Все circle",
            232,
            false,
            false,
            false,
            0,
        ),
        track(
            5,
            "Белые ночи",
            "Гречка",
            "Сталь и шёлк",
            187,
            false,
            false,
            false,
            3,
        ),
        track(
            6,
            "Тёплый бетон",
            "Хмыров",
            "Север",
            176,
            false,
            false,
            false,
            0,
        ),
    ];
    ui.set_search_tracks(model(tracks.clone()));
    ui.set_has_results(true);
    ui.set_has_best(true);
    ui.set_search_best(crate::BestItem {
        item_type: 0,
        title: "Полночный трафик".into(),
        subtitle: "Трек · Kedr Livanskiy · Lopussa".into(),
        cover: Image::default(),
        cover_variant: 3,
        track_id: 1,
        owner_uid: 0,
        kind: 0,
    });
    ui.set_search_total(1240);

    // ---- Очередь ----
    ui.set_queue_items(model(tracks.clone()));
    ui.set_queue_source("Поиск: «демо»".into());

    // ---- Волна ----
    ui.set_wave_active(true);
    ui.set_wave_upcoming(model(tracks[1..5].to_vec()));

    // ---- Скачанное ----
    ui.set_downloads_items(model(vec![
        track(
            3,
            "Ночных дорогов свет",
            "ssshhhiiittt!",
            "Побег",
            198,
            false,
            false,
            false,
            2,
        ),
        track(
            5,
            "Белые ночи",
            "Гречка",
            "Сталь и шёлк",
            187,
            false,
            false,
            false,
            3,
        ),
    ]));

    // ---- Главная/библиотека ----
    let recent: Vec<TileItem> = vec![
        tile("Lopussa", "Альбом · Kedr Livanskiy", 1, 1, 0, 1001),
        tile("Этажи", "Альбом · Молчат Дома", 2, 1, 0, 1002),
        tile("Дежавю", "Плейлист дня", 7, 0, 0, 1003),
        tile("Побег", "Альбом · ssshhhiiittt!", 4, 1, 0, 1004),
    ];
    let generated: Vec<TileItem> = vec![
        tile("Плейлист дня", "Обновляется ежедневно", 3, 0, 0, 2001),
        tile("Премьера", "Новые релизы для вас", 5, 0, 0, 2002),
        tile("Секретный", "Только для вас", 8, 0, 0, 2003),
        tile("Кинопоток", "Саундтреки", 9, 0, 0, 2004),
    ];
    let mut lib: Vec<TileItem> = vec![tile("Мне нравится", "128 треков", 2, 2, 0, 0)];
    lib.extend([
        tile("Дорога", "Плейлист · 42 трека", 6, 0, 0, 3001),
        tile("Фокус", "Плейлист · 30 треков", 10, 0, 0, 3002),
        tile("Ночной эфир", "Плейлист · 57 треков", 5, 0, 0, 3003),
    ]);
    ui.set_home_recent(model(recent));
    ui.set_home_generated(model(generated.clone()));
    ui.set_home_playlists(model(lib.clone()));
    ui.set_library_playlists(model(lib));

    // ---- Библиотека: альбомы и исполнители ----
    let albums: Vec<TileItem> = vec![
        tile("Этажи", "Молчат Дома", 2, 3, 0, 4001),
        tile("Побег", "ssshhhiiittt!", 4, 3, 0, 4002),
        tile("Lopussa", "Kedr Livanskiy", 1, 3, 0, 4003),
        tile("Сталь и шёлк", "Гречка", 6, 3, 0, 4004),
        tile("Голгофа", "Дурной вкус", 8, 3, 0, 4005),
        tile("Пыль", "Пасош", 10, 3, 0, 4006),
    ];
    let artists: Vec<TileItem> = vec![
        tile("Молчат Дома", "Исполнитель", 3, 4, 0, 5001),
        tile("ssshhhiiittt!", "Исполнитель", 5, 4, 0, 5002),
        tile("Кино", "Исполнитель", 7, 4, 0, 5003),
        tile("Гречка", "Исполнитель", 9, 4, 0, 5004),
        tile("Дурной вкус", "Исполнитель", 1, 4, 0, 5005),
        tile("Пасош", "Исполнитель", 6, 4, 0, 5006),
    ];
    ui.set_library_albums(model(albums));
    ui.set_library_artists(model(artists));

    // ---- Лирика ----
    ui.set_lyrics_state(2);
    ui.set_lyrics_text(
        "Демо-строка первой строфы\nвторая строка звучит в тишине\n\nПрипев:\nмы слушаем музыку\nвесь день напролёт\n\nФинальный куплет\nи тихий аккорд".into(),
    );

    // ---- Открытый плейлист (библиотека) ----
    ui.set_playlist_open(false);
    ui.set_playlist_title("Дорога".into());
    ui.set_playlist_subtitle("42 трека · 2 ч 47 мин".into());
    ui.set_playlist_tracks(model(tracks));
}

fn model<T: Clone + 'static>(v: Vec<T>) -> ModelRc<T> {
    ModelRc::new(Rc::new(VecModel::from(v)))
}

#[allow(clippy::too_many_arguments)]
fn track(
    id: i32,
    title: &str,
    artists: &str,
    album: &str,
    duration: i32,
    current: bool,
    playing: bool,
    liked: bool,
    dl: i32,
) -> TrackItem {
    TrackItem {
        id,
        title: SharedString::from(title),
        artists: SharedString::from(artists),
        album: SharedString::from(album),
        duration_sec: duration,
        cover: Image::default(),
        cover_variant: (id as i32 % 10) + 1,
        liked,
        is_current: current,
        is_playing: current && playing,
        dl_state: dl,
        dl_progress: 0,
    }
}

#[allow(clippy::too_many_arguments)]
fn tile(
    title: &str,
    subtitle: &str,
    variant: i32,
    item_type: i32,
    owner: i32,
    kind: i32,
) -> TileItem {
    TileItem {
        title: SharedString::from(title),
        subtitle: SharedString::from(subtitle),
        cover: Image::default(),
        cover_variant: variant,
        item_type,
        owner_uid: owner,
        kind,
    }
}
