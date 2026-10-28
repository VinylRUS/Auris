//! Расширенные сущности каталога: альбом с треками, карточка исполнителя,
//! текст песни, похожие треки, дашборд станций радио, лайки альбомов и
//! плейлистов (разделы спецификации, которых не было в базовом наборе).

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::flex;
use super::playlist::Playlist;
use super::track::{Album, Artist, Track};

/// Альбом с треками (`GET /albums/{id}/with-tracks`).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AlbumWithTracks {
    /// Сам альбом (поля размазаны по корню ответа).
    #[serde(flatten)]
    pub album: Album,
    /// Треки, сгруппированные по томам (диски альбома).
    #[serde(default)]
    pub volumes: Vec<Vec<Track>>,
    /// Лейблы.
    #[serde(default)]
    pub labels: Vec<Label>,
}

/// Лейбл издателя.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Label {
    /// Идентификатор.
    #[serde(default, deserialize_with = "flex::i64")]
    pub id: i64,
    /// Название.
    #[serde(default)]
    pub name: String,
}

impl AlbumWithTracks {
    /// Все треки альбома (тома развернуты в один список).
    pub fn all_tracks(&self) -> Vec<Track> {
        self.volumes.iter().flatten().cloned().collect()
    }
}

/// Карточка исполнителя (`GET /artists/{id}/brief-info`).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtistBriefInfo {
    /// Исполнитель.
    #[serde(default)]
    pub artist: Artist,
    /// Популярные треки.
    #[serde(default)]
    pub popular_tracks: Vec<Track>,
    /// Альбомы (последние).
    #[serde(default)]
    pub albums: Vec<Album>,
    /// Похожие исполнители.
    #[serde(default)]
    pub similar_artists: Vec<Artist>,
}

/// Текст песни (`GET /tracks/{id}/supplement`).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Supplement {
    /// Текст песни.
    #[serde(default)]
    pub lyrics: Option<Lyrics>,
}

/// Текст песни.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Lyrics {
    /// Идентификатор текста.
    #[serde(default, deserialize_with = "flex::i64")]
    pub id: i64,
    /// Номер текста (версия).
    #[serde(default, deserialize_with = "flex::i64")]
    pub lyric: i64,
    /// Полный текст (строки через `\n`).
    #[serde(default)]
    pub text: String,
    /// Источник текста (название сервиса).
    #[serde(default)]
    pub major: Option<super::track::Major>,
}

/// Похожие треки (`GET /tracks/{id}/similar`).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SimilarTracks {
    /// Похожие треки.
    #[serde(default)]
    pub similar_tracks: Vec<Track>,
}

/// Дашборд станций радио (`GET /rotor/stations/dashboard`).
///
/// Полезная нагрузка — `{"dashboardId": …, "stations": […]}`:
/// станции лежат прямо в корне (без промежуточной обёртки).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RotorDashboard {
    /// Идентификатор дашборда.
    #[serde(default)]
    pub dashboard_id: String,
    /// Станции дашборда.
    #[serde(default)]
    pub stations: Vec<StationResult>,
}

/// Станция в результатах ротатора.
///
/// Идентификатор (тип+тег) лежит внутри `station.id` —
/// на верхнем уровне элемента его нет (см. StationResult
/// в спецификации сообщества).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StationResult {
    /// Описание станции.
    #[serde(default)]
    pub station: Option<Station>,
}

impl StationResult {
    /// Сид для `POST /rotor/session/new` (`user:onyourwave`).
    pub fn seed(&self) -> Option<String> {
        Some(self.station.as_ref()?.id.seed())
    }
}

/// Идентификатор станции.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StationId {
    /// Тип, например `user`, `genre`, `activity`.
    #[serde(default)]
    pub r#type: String,
    /// Тег, например `onyourwave`, `rock`, `to-sleep`.
    #[serde(default)]
    pub tag: String,
}

impl StationId {
    /// Сид для `POST /rotor/session/new`, например `user:onyourwave`.
    pub fn seed(&self) -> String {
        format!("{}:{}", self.r#type, self.tag)
    }
}

/// Описание станции.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Station {
    /// Идентификатор (дублирует `StationResult.id`).
    #[serde(default)]
    pub id: StationId,
    /// Человекочитаемое название, например «Моя волна».
    #[serde(default)]
    pub name: String,
    /// Есть ли свои треки у станции (иначе — только рекламные вставки).
    #[serde(default)]
    pub ropits: bool,
}

/// Лайкнутый альбом (`GET /users/{uid}/likes/albums`).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LikedAlbum {
    /// Момент лайка (unix).
    #[serde(default, deserialize_with = "flex::opt_i64")]
    pub timestamp: Option<i64>,
    /// Альбом.
    #[serde(default)]
    pub album: Album,
}

/// Лайкнутый плейлист (`GET /users/{uid}/likes/playlists`).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LikedPlaylist {
    /// Момент лайка (unix).
    #[serde(default, deserialize_with = "flex::opt_i64")]
    pub timestamp: Option<i64>,
    /// Плейлист.
    #[serde(default)]
    pub playlist: Playlist,
}

/// Лайкнутый исполнитель (`GET /users/{uid}/likes/artists`).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LikedArtist {
    /// Момент лайка (unix).
    #[serde(default, deserialize_with = "flex::opt_i64")]
    pub timestamp: Option<i64>,
    /// Исполнитель.
    #[serde(default)]
    pub artist: Artist,
}

/// Ответ лендинга (`GET /landing3`) — блоки персональных подборок.
///
/// Полезная нагрузка — массив блоков:
/// `[{"type": "personalPlaylists", "entities": […]}, …]`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Landing {
    /// Блоки лендинга.
    #[serde(default)]
    pub blocks: Vec<LandingBlock>,
}

/// Блок лендинга.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LandingBlock {
    /// Тип блока (`personalPlaylists`, `mixes`, `promotions`…).
    #[serde(default, rename = "type")]
    pub r#type: String,
    /// Заголовок блока.
    #[serde(default)]
    pub title: String,
    /// Сущности блока (гетерогенны — парсятся по месту).
    #[serde(default)]
    pub entities: Vec<Value>,
}

impl Landing {
    /// Персональные плейлисты (блок `personalPlaylists` → entities → data → playlist).
    pub fn personal_playlists(&self) -> Vec<Playlist> {
        extract_playlists(self, "personalPlaylists")
    }

    /// Миксы (блок `mixes` → entities → data → playlist).
    pub fn mixes(&self) -> Vec<Playlist> {
        extract_playlists(self, "mixes")
    }
}

fn extract_playlists(l: &Landing, block_type: &str) -> Vec<Playlist> {
    let mut out = Vec::new();
    let Some(block) = l.blocks.iter().find(|b| b.r#type == block_type) else {
        return out;
    };
    for e in &block.entities {
        // персональные плейлисты: { data: { playlist } };
        // миксы: { data: { playlist } } либо сразу { playlist }
        if let Some(p) = e
            .get("data")
            .and_then(|d| d.get("playlist"))
            .cloned()
            .or_else(|| e.get("playlist").cloned())
        {
            if let Ok(pl) = serde_json::from_value::<Playlist>(p) {
                out.push(pl);
            }
        }
    }
    out
}
