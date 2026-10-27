//! Варианты загрузки трека.

use serde::{Deserialize, Serialize};

use super::flex;

/// Вариант загрузки трека из `/tracks/{id}/download-info`.
///
/// `url` — ссылка на XML-метаданные, из которых вычисляется прямая
/// ссылка на аудиофайл (см. [`crate::sign::direct_link`]).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadInfo {
    /// Кодек (`mp3`, `aac`).
    #[serde(default)]
    pub codec: String,
    /// Битрейт, кбит/с.
    #[serde(default, deserialize_with = "flex::u64")]
    pub bitrate_in_kbps: u64,
    /// Применена ли нормализация громкости.
    #[serde(default)]
    pub gain: bool,
    /// Разрешено ли прямое скачивание этому пользователю
    /// (фактически — индикатор подписки).
    #[serde(default)]
    pub direct: bool,
    /// Ссылка на XML с метаданными файла.
    #[serde(default)]
    pub url: String,
    /// Поставщик.
    #[serde(default)]
    pub supplier: Option<String>,
    /// Превью-лимит (для отрывков без подписки).
    #[serde(default)]
    pub preview: Option<bool>,
}
