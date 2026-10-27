//! Радио-сессии («Моя волна», станции) — новый Session API.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::track::Track;

/// Сессия радио, созданная `POST /rotor/session/new`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RotorSession {
    /// Идентификатор сессии (используется для треков и feedback).
    #[serde(default)]
    pub radio_session_id: Option<String>,
    /// Идентификатор текущей партии треков.
    #[serde(default)]
    pub batch_id: Option<String>,
    /// Последовательность треков.
    #[serde(default)]
    pub sequence: Vec<SequenceItem>,
    /// Принятые сервером сиды.
    #[serde(default)]
    pub accepted_seeds: Option<Vec<Value>>,
    /// Завершена ли сессия.
    #[serde(default)]
    pub terminated: Option<bool>,
}

/// Следующая партия треков сессии (`GET /rotor/session/{id}/tracks`).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RotorBatch {
    /// Идентификатор партии.
    #[serde(default)]
    pub batch_id: Option<String>,
    /// Последовательность треков.
    #[serde(default)]
    pub sequence: Vec<SequenceItem>,
    /// Завершена ли сессия.
    #[serde(default)]
    pub terminated: Option<bool>,
    /// Не найдена ли сессия.
    #[serde(default)]
    pub unknown_session: Option<bool>,
}

/// Элемент последовательности радио.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SequenceItem {
    /// Тип элемента (`track`, `ad`).
    #[serde(default, rename = "type")]
    pub r#type: String,
    /// Трек.
    #[serde(default)]
    pub track: Option<Track>,
    /// Лайкнут ли трек.
    #[serde(default)]
    pub liked: bool,
    /// Составной идентификатор `trackId:albumId`.
    #[serde(default)]
    pub track_id: Option<String>,
}

impl RotorSession {
    /// Все валидные треки сессии (без рекламы).
    pub fn tracks(&self) -> Vec<Track> {
        self.sequence
            .iter()
            .filter(|s| s.r#type == "track")
            .filter_map(|s| s.track.clone())
            .collect()
    }
}

impl RotorBatch {
    /// Все валидные треки партии (без рекламы).
    pub fn tracks(&self) -> Vec<Track> {
        self.sequence
            .iter()
            .filter(|s| s.r#type == "track")
            .filter_map(|s| s.track.clone())
            .collect()
    }
}
