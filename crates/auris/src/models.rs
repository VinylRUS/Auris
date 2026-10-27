//! Сборка UI-моделей из данных API + общий кеш обложек.
//!
//! Важно: `slint::Image` не `Send`, поэтому между потоками передаются
//! **пути** кеш-файлов обложек, а сами `Image` создаются только внутри
//! event loop (в замыканиях `ui_inv`).

use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::{Arc, Mutex};

use auris_api::{Album, Artist, Playlist, Track};
use slint::{Image, SharedString};

use crate::{BestItem, MainWindow, TileItem, TrackItem};

/// Помощник: применить закешированный текст песни к UI
/// (`None` = текста нет).
pub trait LyricsUi {
    fn set_cached_lyrics(&self, text: Option<String>);
}

impl LyricsUi for MainWindow {
    fn set_cached_lyrics(&self, text: Option<String>) {
        match text {
            Some(t) => {
                self.set_lyrics_state(2);
                self.set_lyrics_text(SharedString::from(t));
            }
            None => {
                self.set_lyrics_state(3);
                self.set_lyrics_text("".into());
            }
        }
    }
}

/// Вариант градиентной заглушки (стабильный по id трека).
pub fn cover_variant(id: i64) -> i32 {
    ((id.abs() % 10) as i32) + 1
}

/// Загружает картинку по кеш-пути (или пустая → заглушка).
fn load_img(path: Option<&String>) -> Image {
    path.and_then(|p| Image::load_from_path(Path::new(p)).ok())
        .unwrap_or_default()
}

/// Лёгкий снимок путей обложек и лайков (Send).
#[derive(Default, Clone)]
pub struct CoversSnapshot {
    /// Пути обложек треков: track_id → файл в кеше.
    pub track_covers: HashMap<i64, String>,
    /// Пути обложек плейлистов: (uid, kind) → файл в кеше.
    pub playlist_covers: HashMap<(i64, i64), String>,
    /// Пути обложек альбомов: album_id → файл в кеше.
    pub album_covers: HashMap<i64, String>,
    /// Пути обложек исполнителей: artist_id → файл в кеше.
    pub artist_covers: HashMap<i64, String>,
    /// Лайкнутые треки.
    pub likes: HashSet<i64>,
}

/// Общий кеш UI-данных.
#[derive(Default)]
pub struct UiCache {
    /// Обложки/лайки.
    pub covers: CoversSnapshot,
    /// Последние результаты поиска.
    pub search: Vec<Track>,
    /// Треки открытого плейлиста.
    pub playlist_tracks: Vec<Track>,
    /// Список загрузок.
    pub downloads: Vec<Track>,
    /// Тексты песен: track_id → текст (`None` = текста нет, отрицательный кеш).
    pub lyrics: HashMap<i64, Option<String>>,
}

/// Разделяемый кеш.
pub type SharedCache = Arc<Mutex<UiCache>>;

/// Строка для Slint.
pub fn s(v: impl Into<String>) -> SharedString {
    SharedString::from(v.into())
}

/// Собирает UI-элемент трека (вызывать в event loop).
pub fn track_item(
    t: &Track,
    cov: &CoversSnapshot,
    current_id: Option<i64>,
    playing: bool,
) -> TrackItem {
    TrackItem {
        id: t.id as i32,
        title: s(t.title.clone()),
        artists: s(t.artists_display()),
        album: s(t.album_title().to_string()),
        duration_sec: t.duration_sec() as i32,
        cover: load_img(cov.track_covers.get(&t.id)),
        cover_variant: cover_variant(t.id),
        liked: cov.likes.contains(&t.id),
        is_current: current_id == Some(t.id),
        is_playing: playing && current_id == Some(t.id),
        dl_state: 0,
        dl_progress: 0,
    }
}

/// Собирает список UI-элементов треков (вызывать в event loop).
pub fn track_items(
    tracks: &[Track],
    cov: &CoversSnapshot,
    current_id: Option<i64>,
    playing: bool,
    dl_states: &HashMap<i64, i32>,
) -> Vec<TrackItem> {
    tracks
        .iter()
        .map(|t| {
            let mut item = track_item(t, cov, current_id, playing);
            if let Some(state) = dl_states.get(&t.id) {
                item.dl_state = *state;
            }
            item
        })
        .collect()
}

/// Собирает UI-плитку плейлиста (вызывать в event loop).
pub fn playlist_tile(pl: &Playlist, cov: &CoversSnapshot) -> TileItem {
    TileItem {
        title: s(pl.title.clone()),
        subtitle: s(format!(
            "{} треков",
            pl.track_count.unwrap_or(pl.tracks.len() as i64)
        )),
        cover: load_img(cov.playlist_covers.get(&(pl.uid, pl.kind))),
        cover_variant: cover_variant(pl.kind + pl.uid),
        item_type: 0,
        owner_uid: pl.uid as i32,
        kind: pl.kind as i32,
    }
}

/// Плитка «Мне нравится».
pub fn likes_tile(count: usize) -> TileItem {
    TileItem {
        title: s("Мне нравится"),
        subtitle: s(format!("{count} треков")),
        cover: Image::default(),
        cover_variant: 7,
        item_type: 2,
        owner_uid: 0,
        kind: 3,
    }
}

/// Плитка альбома (вызывать в event loop).
pub fn album_tile(a: &Album, cov: &CoversSnapshot) -> TileItem {
    TileItem {
        title: s(a.title.clone()),
        subtitle: s(a
            .artists
            .first()
            .map(|x| x.name.clone())
            .unwrap_or_else(|| "Альбом".into())),
        cover: load_img(cov.album_covers.get(&a.id)),
        cover_variant: cover_variant(a.id),
        item_type: 3,
        owner_uid: 0,
        kind: a.id as i32,
    }
}

/// Плитка исполнителя (вызывать в event loop).
pub fn artist_tile(a: &Artist, cov: &CoversSnapshot) -> TileItem {
    TileItem {
        title: s(a.name.clone()),
        subtitle: s("Исполнитель"),
        cover: load_img(cov.artist_covers.get(&a.id)),
        cover_variant: cover_variant(a.id),
        item_type: 4,
        owner_uid: 0,
        kind: a.id as i32,
    }
}

/// Плитка трека-истории (вызывать в event loop).
pub fn history_tile(t: &Track, cov: &CoversSnapshot) -> TileItem {
    TileItem {
        title: s(t.title.clone()),
        subtitle: s(t.artists_display()),
        cover: load_img(cov.track_covers.get(&t.id)),
        cover_variant: cover_variant(t.id),
        item_type: 1,
        owner_uid: 0,
        kind: t.id as i32,
    }
}

/// Лучший результат поиска (вызывать в event loop).
pub fn best_item(best: &Option<auris_api::Best>, cov: &CoversSnapshot) -> Option<BestItem> {
    let best = best.as_ref()?;
    if let Some(t) = best.as_track() {
        return Some(BestItem {
            item_type: 0,
            title: s(t.title.clone()),
            subtitle: s(t.artists_display()),
            cover: load_img(cov.track_covers.get(&t.id)),
            cover_variant: cover_variant(t.id),
            track_id: t.id as i32,
            owner_uid: 0,
            kind: 0,
        });
    }
    if let Some(a) = best.as_artist() {
        return Some(BestItem {
            item_type: 1,
            title: s(a.name.clone()),
            subtitle: s("Исполнитель"),
            cover: Image::default(),
            cover_variant: cover_variant(a.id),
            track_id: 0,
            owner_uid: 0,
            kind: a.id as i32,
        });
    }
    if let Some(al) = best.as_album() {
        return Some(BestItem {
            item_type: 2,
            title: s(al.title.clone()),
            subtitle: s(al
                .artists
                .first()
                .map(|a| a.name.clone())
                .unwrap_or_default()),
            cover: load_img(cov.album_covers.get(&al.id)),
            cover_variant: cover_variant(al.id),
            track_id: 0,
            owner_uid: 0,
            kind: al.id as i32,
        });
    }
    if let Some(pl) = best.as_playlist() {
        return Some(BestItem {
            item_type: 3,
            title: s(pl.title.clone()),
            subtitle: s(pl.owner_display()),
            cover: load_img(cov.playlist_covers.get(&(pl.uid, pl.kind))),
            cover_variant: cover_variant(pl.kind),
            track_id: 0,
            owner_uid: pl.uid as i32,
            kind: pl.kind as i32,
        });
    }
    None
}
