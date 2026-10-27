//! Плейлисты и владельцы.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::flex;
use super::track::Track;

/// Владелец плейлиста.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Owner {
    /// Идентификатор пользователя.
    #[serde(default, deserialize_with = "flex::i64")]
    pub uid: i64,
    /// Логин.
    #[serde(default)]
    pub login: String,
    /// Отображаемое имя (`displayName`; в некоторых ответах — `name`).
    #[serde(default)]
    pub display_name: Option<String>,
    /// Короткое имя (дубль `displayName`).
    #[serde(default)]
    pub name: Option<String>,
}

/// Плейлист.
///
/// Поле `tracks` API возвращает в двух форматах: массив треков
/// (для `/users/{uid}/playlists/{kind}`) или пустое (для `/list`).
/// В очередях треки дополнительно оборачиваются в `{"track": …}` —
/// кастомный десериализатор принимает оба варианта.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Playlist {
    /// Порядковый «kind» плейлиста у владельца.
    #[serde(default, deserialize_with = "flex::i64")]
    pub kind: i64,
    /// Ревизия (версия для синхронизации).
    #[serde(default, deserialize_with = "flex::i64")]
    pub revision: i64,
    /// Владелец.
    #[serde(default)]
    pub owner: Option<Owner>,
    /// Идентификатор владельца (дублируется).
    #[serde(default, deserialize_with = "flex::i64")]
    pub uid: i64,
    /// Название.
    #[serde(default)]
    pub title: String,
    /// Описание.
    #[serde(default)]
    pub description: Option<String>,
    /// Число треков.
    #[serde(default, deserialize_with = "flex::opt_i64")]
    pub track_count: Option<i64>,
    /// Обложка (объект или строка — обрабатывается в [`Playlist::cover_url`]).
    #[serde(default)]
    pub cover: Option<Value>,
    /// Треки плейлиста (если вернулись в ответе).
    #[serde(default, deserialize_with = "playlist_tracks")]
    pub tracks: Vec<Track>,
    /// Тип сгенерированного плейлиста (`playlistOfTheDay`, `recentTracks`…).
    #[serde(default)]
    pub generated_playlist_type: Option<String>,
    /// Публичный ли плейлист.
    #[serde(default)]
    pub is_public: Option<bool>,
}

impl Playlist {
    /// URL обложки плейлиста размера `size` (например `300x300`).
    ///
    /// Обложка приходит либо объектом `{"uri": …}`, либо строкой;
    /// для плейлистов-подборок без обложки возвращает `None`.
    pub fn cover_url(&self, size: &str) -> Option<String> {
        let uri = match self.cover.as_ref()? {
            Value::String(s) => Some(s.clone()),
            Value::Object(map) => map.get("uri").and_then(|v| v.as_str()).map(String::from),
            _ => None,
        }?;
        let path = uri.replace("%%", size);
        if path.starts_with("http") {
            Some(path)
        } else {
            Some(format!("https://{path}"))
        }
    }

    /// Заголовок владельца для UI.
    pub fn owner_display(&self) -> String {
        self.owner
            .as_ref()
            .and_then(|o| {
                o.display_name
                    .clone()
                    .or_else(|| o.name.clone())
                    .or_else(|| Some(o.login.clone()))
            })
            .unwrap_or_default()
    }
}

/// Десериализатор треков плейлиста: принимает `[Track, …]`,
/// `[{"track": Track}, …]` и `null`.
fn playlist_tracks<'de, D>(d: D) -> std::result::Result<Vec<Track>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    struct Wrapper {
        track: Option<Track>,
    }

    let raw = Option::<Vec<Value>>::deserialize(d)?.unwrap_or_default();
    let mut out = Vec::with_capacity(raw.len());
    for item in raw {
        if item.get("track").is_some() {
            if let Ok(w) = serde_json::from_value::<Wrapper>(item) {
                if let Some(t) = w.track {
                    out.push(t);
                }
            }
        } else if let Ok(t) = serde_json::from_value::<Track>(item) {
            out.push(t);
        }
    }
    Ok(out)
}
