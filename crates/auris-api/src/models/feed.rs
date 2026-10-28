//! Лента «Главной».

use serde::{Deserialize, Serialize};

use super::playlist::Playlist;

/// Сгенерированный плейлист ленты (плейлист дня, премьера, …).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeneratedPlaylist {
    /// Тип (`playlistOfTheDay`, `recentTracks`, `newMix`, …).
    #[serde(default, rename = "type")]
    pub r#type: String,
    /// Готов ли.
    #[serde(default)]
    pub ready: bool,
    /// Уведомление.
    #[serde(default)]
    pub notify: Option<bool>,
    /// Плейлист.
    ///
    /// В живом ответе `/feed` плейлист лежит в поле `data`
    /// (GeneratedPlaylist.data в спецификации сообщества);
    /// `playlist` оставлен как совместимый алиас.
    #[serde(default, alias = "data")]
    pub playlist: Option<Playlist>,
}

/// Ответ `/feed`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Feed {
    /// Сгенерированные плейлисты.
    #[serde(default)]
    pub generated_playlists: Vec<GeneratedPlaylist>,
}
