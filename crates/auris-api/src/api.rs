//! Методы API, сгруппированные по разделам официальной спецификации.

use serde_json::Value;

use crate::client::Client;
use crate::error::{ApiError, Result};
use crate::models::*;

impl Client {
    // ------------------------------------------------------------------
    // Аккаунт
    // ------------------------------------------------------------------

    /// `GET /account/status` — данные аккаунта и подписки.
    pub async fn account_status(&self) -> Result<Status> {
        self.get("/account/status", &[]).await
    }

    // ------------------------------------------------------------------
    // Лента и лендинги
    // ------------------------------------------------------------------

    /// `GET /feed` — главная лента (сгенерированные плейлисты).
    pub async fn feed(&self) -> Result<Feed> {
        self.get("/feed", &[]).await
    }

    /// `GET /landing3/new-releases` — новые релизы (для чипа «Новинки»).
    pub async fn landing_new_releases(&self) -> Result<Vec<Album>> {
        let v: Value = self.get("/landing3/new-releases", &[]).await?;
        Ok(extract_albums(&v))
    }

    /// `GET /landing3/chart` — чарт (для чипа «Чарт»).
    pub async fn landing_chart(&self) -> Result<Vec<Track>> {
        let v: Value = self.get("/landing3/chart", &[]).await?;
        // Чарт приходит блоком playlists→tracks либо сразу tracks.
        if let Some(tracks) = v.get("tracks").and_then(|t| t.as_array()) {
            if let Ok(parsed) = serde_json::from_value::<Vec<Track>>(Value::Array(tracks.clone())) {
                return Ok(parsed);
            }
        }
        for block in ["playlists", "podcasts"] {
            if let Some(items) = v.get(block).and_then(|t| t.as_array()) {
                for item in items {
                    if let Some(tracks) = item.get("tracks").and_then(|t| t.as_array()) {
                        if let Ok(parsed) =
                            serde_json::from_value::<Vec<Track>>(Value::Array(tracks.clone()))
                        {
                            return Ok(parsed);
                        }
                    }
                }
            }
        }
        Err(ApiError::Unexpected("chart: no tracks block found".into()))
    }

    /// `GET /landing3` — полный лендинг (персональные плейлисты, миксы).
    pub async fn landing(&self, blocks: &[&str]) -> Result<Landing> {
        let blocks = blocks
            .iter()
            .map(|b| b.to_string())
            .collect::<Vec<_>>()
            .join(",");
        self.get("/landing3", &[("blocks", blocks)]).await
    }

    // ------------------------------------------------------------------
    // Поиск
    // ------------------------------------------------------------------

    /// `GET /search` — поиск по трекам.
    pub async fn search_tracks(&self, text: &str, page: u32) -> Result<Search> {
        self.search(text, "track", page).await
    }

    /// `GET /search` — универсальный поиск.
    ///
    /// `kind`: `all` | `track` | `artist` | `album` | `playlist`.
    pub async fn search(&self, text: &str, kind: &str, page: u32) -> Result<Search> {
        self.get(
            "/search",
            &[
                ("text", text.to_string()),
                ("nocorrect", "false".to_string()),
                ("type", kind.to_string()),
                ("page", page.to_string()),
                ("playlist-in-best", "true".to_string()),
            ],
        )
        .await
    }

    /// `GET /search/suggest` — подсказки поиска.
    pub async fn search_suggest(&self, part: &str) -> Result<Vec<String>> {
        #[derive(serde::Deserialize)]
        struct Suggestions {
            #[serde(default)]
            suggestions: Vec<Suggestion>,
        }
        #[derive(serde::Deserialize)]
        struct Suggestion {
            #[serde(default)]
            value: String,
        }
        let s: Suggestions = self
            .get("/search/suggest", &[("part", part.to_string())])
            .await?;
        Ok(s.suggestions.into_iter().map(|x| x.value).collect())
    }

    // ------------------------------------------------------------------
    // Треки
    // ------------------------------------------------------------------

    /// `GET /tracks?track-ids=…` — треки по идентификаторам.
    pub async fn tracks(&self, ids: &[i64]) -> Result<Vec<Track>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let joined = ids
            .iter()
            .map(|i| i.to_string())
            .collect::<Vec<_>>()
            .join(",");
        let v: Value = self
            .get(
                "/tracks",
                &[
                    ("track-ids", joined),
                    ("with-positions", "true".to_string()),
                ],
            )
            .await?;
        // Ответ — массив треков; для сборников элементы имеют {track, id}.
        let arr = v
            .as_array()
            .cloned()
            .ok_or_else(|| ApiError::Unexpected("tracks: expected array".into()))?;
        let mut out = Vec::with_capacity(arr.len());
        for item in arr {
            if item.get("track").is_some() {
                if let Ok(t) = serde_json::from_value::<Track>(item["track"].clone()) {
                    out.push(t);
                }
            } else if let Ok(t) = serde_json::from_value::<Track>(item) {
                out.push(t);
            }
        }
        Ok(out)
    }

    /// `GET /tracks/{id}/download-info` — варианты загрузки.
    pub async fn track_download_info(&self, track_id: i64) -> Result<Vec<DownloadInfo>> {
        self.get(&format!("/tracks/{track_id}/download-info"), &[])
            .await
    }

    /// `GET /tracks/{id}/similar` — похожие треки.
    pub async fn tracks_similar(&self, track_id: i64) -> Result<SimilarTracks> {
        self.get(&format!("/tracks/{track_id}/similar"), &[]).await
    }

    /// `GET /tracks/{id}/supplement` — текст песни и видео.
    pub async fn track_supplement(&self, track_id: i64) -> Result<Supplement> {
        self.get(&format!("/tracks/{track_id}/supplement"), &[])
            .await
    }

    // ------------------------------------------------------------------
    // Альбомы и исполнители
    // ------------------------------------------------------------------

    /// `GET /albums/{id}/with-tracks` — альбом вместе с треками.
    pub async fn album_with_tracks(&self, album_id: i64) -> Result<AlbumWithTracks> {
        self.get(&format!("/albums/{album_id}/with-tracks"), &[])
            .await
    }

    /// `GET /albums?album-ids=…` — альбомы по идентификаторам.
    pub async fn albums(&self, ids: &[i64]) -> Result<Vec<Album>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let joined = ids
            .iter()
            .map(|i| i.to_string())
            .collect::<Vec<_>>()
            .join(",");
        let v: Value = self.get("/albums", &[("album-ids", joined)]).await?;
        Ok(v.as_array()
            .cloned()
            .map(|arr| {
                arr.into_iter()
                    .filter_map(|a| serde_json::from_value::<Album>(a).ok())
                    .collect()
            })
            .unwrap_or_default())
    }

    /// `GET /artists/{id}/brief-info` — карточка исполнителя
    /// (популярные треки, альбомы, похожие).
    pub async fn artists_brief_info(&self, artist_id: i64) -> Result<ArtistBriefInfo> {
        self.get(
            &format!("/artists/{artist_id}/brief-info"),
            &[("locale", "ru".to_string())],
        )
        .await
    }

    // ------------------------------------------------------------------
    // Плейлисты
    // ------------------------------------------------------------------

    /// `GET /users/{uid}/playlists/list` — плейлисты пользователя.
    pub async fn users_playlists_list(&self, uid: i64) -> Result<Vec<Playlist>> {
        self.get(&format!("/users/{uid}/playlists/list"), &[]).await
    }

    /// `GET /users/{uid}/playlists/{kind}` — плейлист с треками.
    pub async fn user_playlist(&self, uid: i64, kind: i64) -> Result<Playlist> {
        self.get(&format!("/users/{uid}/playlists/{kind}"), &[])
            .await
    }

    // ------------------------------------------------------------------
    // Лайки
    // ------------------------------------------------------------------

    /// `GET /users/{uid}/likes/tracks` — любимые треки.
    pub async fn users_likes_tracks(&self, uid: i64) -> Result<LikesTracks> {
        self.get(&format!("/users/{uid}/likes/tracks"), &[]).await
    }

    /// `GET /users/{uid}/likes/albums` — любимые альбомы.
    pub async fn users_likes_albums(&self, uid: i64) -> Result<Vec<LikedAlbum>> {
        #[derive(serde::Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Resp {
            #[serde(default)]
            likes: Vec<LikedAlbum>,
        }
        let r: Resp = self.get(&format!("/users/{uid}/likes/albums"), &[]).await?;
        Ok(r.likes)
    }

    /// `GET /users/{uid}/likes/playlists` — любимые плейлисты.
    pub async fn users_likes_playlists(&self, uid: i64) -> Result<Vec<LikedPlaylist>> {
        #[derive(serde::Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Resp {
            #[serde(default)]
            likes: Vec<LikedPlaylist>,
        }
        let r: Resp = self
            .get(&format!("/users/{uid}/likes/playlists"), &[])
            .await?;
        Ok(r.likes)
    }

    /// `GET /users/{uid}/likes/artists` — любимые исполнители.
    pub async fn users_likes_artists(&self, uid: i64) -> Result<Vec<LikedArtist>> {
        #[derive(serde::Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Resp {
            #[serde(default)]
            likes: Vec<LikedArtist>,
        }
        let r: Resp = self
            .get(&format!("/users/{uid}/likes/artists"), &[])
            .await?;
        Ok(r.likes)
    }

    /// `POST /users/{uid}/likes/tracks/add-multiple` — лайк.
    pub async fn likes_tracks_add(&self, uid: i64, track_ids: &[i64]) -> Result<()> {
        let mut form: Vec<(String, String)> = Vec::new();
        for id in track_ids {
            form.push(("track-ids".to_string(), id.to_string()));
        }
        let _: Value = self
            .post_form(&format!("/users/{uid}/likes/tracks/add-multiple"), form)
            .await?;
        Ok(())
    }

    /// `POST /users/{uid}/likes/tracks/remove` — снять лайк.
    pub async fn likes_tracks_remove(&self, uid: i64, track_ids: &[i64]) -> Result<()> {
        let mut form: Vec<(String, String)> = Vec::new();
        for id in track_ids {
            form.push(("track-ids".to_string(), id.to_string()));
        }
        let _: Value = self
            .post_form(&format!("/users/{uid}/likes/tracks/remove"), form)
            .await?;
        Ok(())
    }

    // ------------------------------------------------------------------
    // Радио («Моя волна»)
    // ------------------------------------------------------------------

    /// `GET /rotor/stations/dashboard` — станции дашборда
    /// (Моя волна, премьеры, коллекции и т.д.).
    pub async fn rotor_stations_dashboard(&self) -> Result<RotorDashboard> {
        self.get("/rotor/stations/dashboard", &[]).await
    }

    /// `POST /rotor/session/new` — новая сессия радио.
    ///
    /// `seeds` — например `["user:onyourwave"]` или
    /// `["user:onyourwave", "settingDiversity:discover"]`.
    pub async fn rotor_session_new(&self, seeds: &[String]) -> Result<RotorSession> {
        #[derive(serde::Serialize)]
        #[serde(rename_all = "camelCase")]
        struct Body<'a> {
            seeds: &'a [String],
            #[serde(skip_serializing_if = "Option::is_none")]
            include_tracks_in_response: Option<bool>,
            #[serde(skip_serializing_if = "Option::is_none")]
            interactive: Option<bool>,
        }
        let body = Body {
            seeds,
            include_tracks_in_response: Some(true),
            interactive: Some(true),
        };
        self.post_json("/rotor/session/new", &body).await
    }

    /// `GET /rotor/session/{id}/tracks` — следующая партия треков.
    pub async fn rotor_session_tracks(&self, session_id: &str) -> Result<RotorBatch> {
        self.get(&format!("/rotor/session/{session_id}/tracks"), &[])
            .await
    }

    /// `POST /rotor/session/{id}/feedback` — событие воспроизведения.
    ///
    /// `event` — `radioStarted` / `trackStarted` / `trackFinished` / `skip`.
    /// Ошибки логируются вызовом и не прерывают воспроизведение.
    pub async fn rotor_session_feedback(
        &self,
        session_id: &str,
        event: &str,
        track_id: Option<&str>,
        total_played_seconds: Option<f64>,
        batch_id: Option<&str>,
    ) -> Result<()> {
        #[derive(serde::Serialize)]
        #[serde(rename_all = "camelCase")]
        struct Event {
            r#type: String,
            timestamp: String,
            #[serde(skip_serializing_if = "Option::is_none")]
            track_id: Option<String>,
            #[serde(skip_serializing_if = "Option::is_none")]
            total_played_seconds: Option<f64>,
        }
        #[derive(serde::Serialize)]
        #[serde(rename_all = "camelCase")]
        struct Feedback<'a> {
            event: Event,
            #[serde(skip_serializing_if = "Option::is_none")]
            batch_id: Option<&'a str>,
        }
        let body = Feedback {
            event: Event {
                r#type: event.to_string(),
                timestamp: iso_now(),
                track_id: track_id.map(String::from),
                total_played_seconds,
            },
            batch_id,
        };
        let _: Value = self
            .post_json(&format!("/rotor/session/{session_id}/feedback"), &body)
            .await?;
        Ok(())
    }
}

/// Ответ `GET /users/{uid}/likes/tracks`.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LikesTracks {
    /// Ревизия библиотеки.
    #[serde(default)]
    pub revision: i64,
    /// Треки.
    #[serde(default)]
    pub tracks: Vec<Track>,
}

/// Текущее время в RFC 3339 (для feedback).
fn iso_now() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    // Простая конвертация unix → UTC без внешних крейтов.
    let days = secs / 86_400;
    let (y, m, d) = civil_from_days(days as i64);
    let rem = secs % 86_400;
    let (hh, mm, ss) = (rem / 3600, (rem % 3600) / 60, rem % 60);
    format!("{y:04}-{m:02}-{d:02}T{hh:02}:{mm:02}:{ss:02}.000Z")
}

/// Дни от эпохи → (год, месяц, день). Алгоритм Говарда Хиннанта.
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// Достаёт альбомы из ответа landing3.
fn extract_albums(v: &Value) -> Vec<Album> {
    for key in ["albums", "new_releases", "playlists"] {
        if let Some(arr) = v.get(key).and_then(|x| x.as_array()) {
            if let Ok(parsed) = serde_json::from_value::<Vec<Album>>(Value::Array(arr.clone())) {
                return parsed;
            }
        }
    }
    Vec::new()
}
