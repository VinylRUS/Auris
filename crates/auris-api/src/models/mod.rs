//! Модели данных API Яндекс.Музыки.
//!
//! Все поля толерантны к отсутствию: API регулярно расширяется,
//! поэтому `deny_unknown_fields` нигде не используется, а обязательные
//! с точки зрения бизнес-логики поля имеют разумные значения по умолчанию.

pub mod account;
pub mod catalog;
pub mod download;
pub mod feed;
pub mod playlist;
pub mod rotor;
pub mod search;
pub mod track;

/// Вспомогательные «гибкие» десериализаторы для полей,
/// которые API иногда отдаёт как число, а иногда — как строку.
pub mod flex {
    use serde::{Deserialize, Deserializer};

    /// `i64` из числа или числовой строки.
    pub fn i64<'de, D: Deserializer<'de>>(d: D) -> std::result::Result<i64, D::Error> {
        let v = serde_json::Value::deserialize(d)?;
        Ok(value_to_i64(&v))
    }

    /// `u64` из числа или числовой строки.
    pub fn u64<'de, D: Deserializer<'de>>(d: D) -> std::result::Result<u64, D::Error> {
        let v = serde_json::Value::deserialize(d)?;
        Ok(value_to_i64(&v).max(0) as u64)
    }

    /// `Option<i64>` (null → None).
    pub fn opt_i64<'de, D: Deserializer<'de>>(d: D) -> std::result::Result<Option<i64>, D::Error> {
        let v = serde_json::Value::deserialize(d)?;
        if v.is_null() {
            return Ok(None);
        }
        Ok(Some(value_to_i64(&v)))
    }

    fn value_to_i64(v: &serde_json::Value) -> i64 {
        match v {
            serde_json::Value::Number(n) => n.as_i64().unwrap_or(0),
            serde_json::Value::String(s) => s.parse().unwrap_or(0),
            _ => 0,
        }
    }
}

pub use account::{Account, Permissions, Status, Subscription};
pub use catalog::{
    AlbumWithTracks, ArtistBriefInfo, Label, Landing, LandingBlock, LikedAlbum, LikedArtist,
    LikedPlaylist, Lyrics, RotorDashboard, SimilarTracks, Station, StationId, StationResult,
    Supplement,
};
pub use download::DownloadInfo;
pub use feed::{Feed, GeneratedPlaylist};
pub use playlist::{Owner, Playlist};
pub use rotor::{RotorBatch, RotorSession, SequenceItem};
pub use search::{Best, Search};
pub use track::{Album, Artist, Cover, Major, Track};
