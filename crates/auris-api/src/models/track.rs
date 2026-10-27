//! Трек, исполнитель, альбом — базовые каталог-сущности.

use serde::{Deserialize, Serialize};

use super::flex;

/// Обложка (`avatars.yandex.net` URI со шкалой `%%`).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Cover {
    /// URI-шаблон обложки, например
    /// `avatars.yandex.net/get-music-content/12345/%%`.
    #[serde(default)]
    pub uri: Option<String>,
}

impl Cover {
    /// URL обложки заданного размера, например `200x200`.
    /// `None`, если обложки нет.
    pub fn url(&self, size: &str) -> Option<String> {
        let uri = self.uri.as_deref()?;
        let path = uri.replace("%%", size);
        if path.starts_with("http") {
            Some(path)
        } else {
            Some(format!("https://{path}"))
        }
    }
}

/// Издатель трека (лейбл).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Major {
    /// Идентификатор издателя.
    #[serde(default)]
    pub id: i64,
    /// Название (например, «Яндекс Музыка»).
    #[serde(default)]
    pub name: String,
}

/// Исполнитель.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Artist {
    /// Идентификатор.
    #[serde(default, deserialize_with = "flex::i64")]
    pub id: i64,
    /// Имя.
    #[serde(default)]
    pub name: String,
    /// Обложка исполнителя.
    #[serde(default)]
    pub cover: Option<Cover>,
    /// Различные исполнители (сборники).
    #[serde(default)]
    pub various: bool,
    /// Композитор.
    #[serde(default)]
    pub composer: bool,
}

/// Альбом.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Album {
    /// Идентификатор.
    #[serde(default, deserialize_with = "flex::i64")]
    pub id: i64,
    /// Название.
    #[serde(default)]
    pub title: String,
    /// Обложка.
    #[serde(default)]
    pub cover_uri: Option<String>,
    /// Число треков.
    #[serde(default, deserialize_with = "flex::opt_i64")]
    pub track_count: Option<i64>,
    /// Год издания.
    #[serde(default, deserialize_with = "flex::opt_i64")]
    pub year: Option<i64>,
    /// Жанр.
    #[serde(default)]
    pub genre: Option<String>,
    /// Доступен ли альбом.
    #[serde(default)]
    pub available: bool,
    /// Исполнители альбома.
    #[serde(default)]
    pub artists: Vec<Artist>,
}

impl Album {
    /// URL обложки альбома размера `size` (например `200x200`).
    pub fn cover_url(&self, size: &str) -> Option<String> {
        let uri = self.cover_uri.as_deref()?;
        Some(format!("https://{}", uri.replace("%%", size)))
    }
}

/// Трек.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Track {
    /// Идентификатор.
    #[serde(default, deserialize_with = "flex::i64")]
    pub id: i64,
    /// Название.
    #[serde(default)]
    pub title: String,
    /// Реальный идентификатор (для треков сборников).
    #[serde(default, deserialize_with = "flex::i64")]
    pub real_id: i64,
    /// Доступен ли трек.
    #[serde(default)]
    pub available: bool,
    /// Доступен ли по подписке Плюс.
    #[serde(default)]
    pub available_for_premium_users: bool,
    /// Полностью ли доступен без подписки.
    #[serde(default)]
    pub available_full_without_subscription: bool,
    /// Длительность, мс.
    #[serde(default, deserialize_with = "flex::u64")]
    pub duration_ms: u64,
    /// Размер файла, байт.
    #[serde(default, deserialize_with = "flex::u64")]
    pub file_size: u64,
    /// URI-шаблон обложки.
    #[serde(default)]
    pub cover_uri: Option<String>,
    /// Исполнители.
    #[serde(default)]
    pub artists: Vec<Artist>,
    /// Альбомы, в которые входит трек.
    #[serde(default)]
    pub albums: Vec<Album>,
    /// Издатель.
    #[serde(default)]
    pub major: Option<Major>,
    /// Тип источника (например `clip`).
    #[serde(default)]
    pub track_source: Option<String>,
}

impl Track {
    /// URL обложки трека размера `size` (например `200x200`).
    pub fn cover_url(&self, size: &str) -> Option<String> {
        let uri = self.cover_uri.as_deref()?;
        Some(format!("https://{}", uri.replace("%%", size)))
    }

    /// Строка исполнителей: «Кино», «A & B».
    pub fn artists_display(&self) -> String {
        self.artists
            .iter()
            .map(|a| a.name.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    }

    /// Название первого альбома (для списков).
    pub fn album_title(&self) -> &str {
        self.albums
            .first()
            .map(|a| a.title.as_str())
            .unwrap_or_default()
    }

    /// Составной идентификатор трека `id:albumId` (для радио и очередей).
    pub fn composite_id(&self) -> String {
        match self.albums.first() {
            Some(album) => format!("{}:{}", self.id, album.id),
            None => self.id.to_string(),
        }
    }

    /// Длительность в секундах.
    pub fn duration_sec(&self) -> u64 {
        self.duration_ms / 1000
    }
}
