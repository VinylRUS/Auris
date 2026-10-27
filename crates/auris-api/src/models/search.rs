//! Результаты поиска.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::flex;
use super::playlist::Playlist;
use super::track::{Album, Artist, Track};

/// Блок результатов поиска с треками.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TracksBlock {
    /// Всего найдено.
    #[serde(default, deserialize_with = "flex::i64")]
    pub total: i64,
    /// Результаты на странице.
    #[serde(default, deserialize_with = "flex::i64")]
    pub per_page: i64,
    /// Порядок блока.
    #[serde(default, deserialize_with = "flex::i64")]
    pub order: i64,
    /// Треки.
    #[serde(default)]
    pub results: Vec<Track>,
}

/// Блок результатов поиска с альбомами.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AlbumsBlock {
    /// Всего найдено.
    #[serde(default, deserialize_with = "flex::i64")]
    pub total: i64,
    /// Результаты.
    #[serde(default)]
    pub results: Vec<Album>,
}

/// Блок результатов поиска с исполнителями.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtistsBlock {
    /// Всего найдено.
    #[serde(default, deserialize_with = "flex::i64")]
    pub total: i64,
    /// Результаты.
    #[serde(default)]
    pub results: Vec<Artist>,
}

/// Блок результатов поиска с плейлистами.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaylistsBlock {
    /// Всего найдено.
    #[serde(default, deserialize_with = "flex::i64")]
    pub total: i64,
    /// Результаты.
    #[serde(default)]
    pub results: Vec<Playlist>,
}

/// Лучший результат поиска.
///
/// `result` приходит разными сущностями в зависимости от `type`
/// (track / artist / album / playlist / podcast), поэтому хранится
/// как JSON и типизируется через [`Best::as_track`] и друзей.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Best {
    /// Тип результата.
    #[serde(default, rename = "type")]
    pub r#type: String,
    /// Сырой результат.
    #[serde(default)]
    pub result: Option<Value>,
    /// Подсказка-текст (для типа `text`).
    #[serde(default)]
    pub text: Option<String>,
}

impl Best {
    /// Лучший результат как трек.
    pub fn as_track(&self) -> Option<Track> {
        serde_json::from_value(self.result.clone()?).ok()
    }

    /// Лучший результат как исполнитель.
    pub fn as_artist(&self) -> Option<Artist> {
        serde_json::from_value(self.result.clone()?).ok()
    }

    /// Лучший результат как альбом.
    pub fn as_album(&self) -> Option<Album> {
        serde_json::from_value(self.result.clone()?).ok()
    }

    /// Лучший результат как плейлист.
    pub fn as_playlist(&self) -> Option<Playlist> {
        serde_json::from_value(self.result.clone()?).ok()
    }
}

/// Ответ `/search`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Search {
    /// Блок треков.
    #[serde(default)]
    pub tracks: Option<TracksBlock>,
    /// Блок альбомов.
    #[serde(default)]
    pub albums: Option<AlbumsBlock>,
    /// Блок исполнителей.
    #[serde(default)]
    pub artists: Option<ArtistsBlock>,
    /// Блок плейлистов.
    #[serde(default)]
    pub playlists: Option<PlaylistsBlock>,
    /// Лучшее совпадение.
    #[serde(default)]
    pub best: Option<Best>,
    /// Была ли исправлена опечатка.
    #[serde(default)]
    pub misspell_corrected: Option<bool>,
}
