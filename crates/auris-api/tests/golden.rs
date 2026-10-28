//! Golden-тесты: десериализация зафиксированных ответов API.
//!
//! Фикстуры собраны по структуре из открытой спецификации
//! сообщества (MarshalX/yandex-music-api). Тесты не ходят в сеть
//! и гарантируют, что рефакторинг моделей не ломает разбор полей.

use auris_api::{models::*, sign};

fn fixture(name: &str) -> String {
    let path = format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(path).unwrap_or_else(|e| panic!("cannot read {name}: {e}"))
}

#[test]
fn parses_track() {
    let t: Track = serde_json::from_str(&fixture("track.json")).unwrap();
    assert_eq!(t.id, 5275115);
    assert_eq!(t.title, "Группа крови");
    assert_eq!(t.duration_ms, 285_000);
    assert!(t.available);
    assert_eq!(t.artists.len(), 1);
    assert_eq!(t.artists[0].name, "Кино");
    assert_eq!(t.albums.len(), 1);
    assert_eq!(t.albums[0].title, "Группа крови");
    assert_eq!(
        t.cover_url("200x200").unwrap(),
        "https://avatars.yandex.net/get-music-content/2a000001/xxx/200x200"
    );
    assert_eq!(t.composite_id(), "5275115:5275114");
    assert_eq!(t.duration_sec(), 285);
}

#[test]
fn parses_search() {
    let s: Search = serde_json::from_str(&fixture("search.json")).unwrap();
    let tracks = s.tracks.as_ref().expect("tracks block");
    assert_eq!(tracks.total, 4242);
    assert_eq!(tracks.results.len(), 2);
    assert_eq!(tracks.results[0].artists_display(), "Кино");
    let best = s.best.as_ref().unwrap();
    assert_eq!(best.r#type, "track");
    let best_track = best.as_track().unwrap();
    assert_eq!(best_track.title, "Группа крови");
    assert_eq!(s.artists.as_ref().unwrap().results[0].name, "Кино");
    assert_eq!(s.albums.as_ref().unwrap().results[0].title, "Группа крови");
    assert_eq!(
        s.playlists.as_ref().unwrap().results[0].title,
        "Кино — все треки"
    );
}

#[test]
fn parses_feed_and_playlists() {
    let f: Feed = serde_json::from_str(&fixture("feed.json")).unwrap();
    assert_eq!(f.generated_playlists.len(), 3);
    assert_eq!(f.generated_playlists[0].r#type, "playlistOfTheDay");
    let pl = f.generated_playlists[0].playlist.as_ref().unwrap();
    assert_eq!(pl.title, "Плейлист дня");
    assert_eq!(pl.track_count, Some(60));
    assert_eq!(pl.tracks.len(), 0);
    // Первый два плейлиста приходят в поле data (живой формат),
    // третий — в playlist (совместимый алиас).
    assert_eq!(
        f.generated_playlists[1].playlist.as_ref().unwrap().title,
        "Премьера"
    );
    assert_eq!(
        f.generated_playlists[2].playlist.as_ref().unwrap().title,
        "Микс"
    );

    let lists: Vec<Playlist> = serde_json::from_str(&fixture("playlists.json")).unwrap();
    assert_eq!(lists.len(), 2);
    assert_eq!(lists[0].title, "Мне нравится");
    assert_eq!(lists[0].owner_display(), "Alexander");
    assert_eq!(lists[0].cover_url("300x300").is_some(), true);

    // Плейлист с треками в двух форматах обёртки.
    let with_tracks: Playlist = serde_json::from_str(&fixture("playlist_tracks.json")).unwrap();
    assert_eq!(with_tracks.tracks.len(), 2);
    assert_eq!(with_tracks.tracks[0].title, "Группа крови");
    assert_eq!(with_tracks.tracks[1].title, "Судно");
}

#[test]
fn parses_account_status() {
    let s: Status = serde_json::from_str(&fixture("account_status.json")).unwrap();
    assert_eq!(s.uid(), 123_456_789);
    assert_eq!(s.display_name(), "Александр");
    assert!(s.has_subscription(), "subscription.until в будущем");
}

#[test]
fn parses_guest_account_status_from_live_envelope() {
    // Регрессия «данные не подтягиваются»: живой гостевой ответ
    // (зафиксирован 2026-10-28). Обёртка result разворачивается,
    // permissions.default приходит СТРОКАМИ — обе формы должны
    // разбираться, иначе падает весь вход.
    let v: serde_json::Value = serde_json::from_str(&fixture("account_status_guest.json")).unwrap();
    let payload = auris_api::unwrap_result(v);
    let s: Status = serde_json::from_value(payload).unwrap();
    let account = s.account.clone().expect("account inside result");
    assert_eq!(account.region, Some(10_000));
    assert_eq!(account.uid, 0, "у гостя нет uid");
    let perms = s.permissions.clone().expect("permissions");
    assert_eq!(perms.values, vec!["landing-play", "feed-play", "mix-play"]);
    assert_eq!(perms.default.len(), 3, "default строками тоже разбирается");
    assert_eq!(perms.default[0].value.as_deref(), Some("landing-play"));
    let _ = s.has_subscription(); // не паникует и не мешает разбору
}

#[test]
fn parses_likes() {
    // Живой формат: укороченные записи {id, albumId} — полные
    // карточки догружаются отдельным запросом /tracks.
    let l: auris_api::api::LikesShort = serde_json::from_str(&fixture("likes.json")).unwrap();
    assert_eq!(l.revision, 42);
    assert_eq!(l.tracks.len(), 2);
    assert_eq!(l.tracks[0].id, 527_5115);
    assert_eq!(l.tracks[0].album_id, Some(5_275_114));
    assert_eq!(l.tracks[1].id, 116_376);
}

#[test]
fn parses_likes_full() {
    // Гидратированная форма (после догрузки /tracks) — для UI.
    let l: auris_api::api::LikesTracks = serde_json::from_str(&fixture("likes_full.json")).unwrap();
    assert_eq!(l.revision, 42);
    assert_eq!(l.tracks.len(), 2);
    assert_eq!(l.tracks[1].title, "Судно");
}

#[test]
fn parses_download_info_and_signs() {
    let infos: Vec<DownloadInfo> = serde_json::from_str(&fixture("download_info.json")).unwrap();
    assert_eq!(infos.len(), 3);
    assert_eq!(infos[2].codec, "mp3");
    assert_eq!(infos[2].bitrate_in_kbps, 320);
    assert!(infos[2].direct);

    let xml = fixture("download_info.xml");
    let meta = sign::parse_meta(&xml).unwrap();
    assert_eq!(meta.host, "storage.mds.yandex.net");
    let link = sign::build_direct_link(&meta);
    assert!(link.starts_with("https://storage.mds.yandex.net/get-mp3/"));
    assert!(link.contains("/6702839/get-music-content/03/ae/file.mp3"));
}

#[test]
fn parses_rotor_session() {
    let s: RotorSession = serde_json::from_str(&fixture("rotor_session.json")).unwrap();
    assert_eq!(s.radio_session_id.as_deref(), Some("abc-123"));
    let tracks = s.tracks();
    assert_eq!(tracks.len(), 2);
    assert_eq!(tracks[0].title, "Группа крови");
    assert_eq!(tracks[1].id, 116_376);

    let b: RotorBatch = serde_json::from_str(&fixture("rotor_batch.json")).unwrap();
    assert_eq!(b.batch_id.as_deref(), Some("batch-2"));
    assert_eq!(b.tracks().len(), 1);
}

#[test]
fn parses_device_code_and_token() {
    // Формат зафиксирован по живому ответу oauth.yandex.ru (snake_case).
    let d: auris_api::DeviceCode = serde_json::from_str(&fixture("device_code.json")).unwrap();
    assert_eq!(d.user_code, "ggjs53l2");
    assert!(d.verification_url.starts_with("https://ya.ru/device"));
    assert_eq!(d.interval, 5);
    assert_eq!(d.expires_in, 300);
    assert!(!d.device_code.is_empty());

    let t: auris_api::OAuthToken = serde_json::from_str(&fixture("oauth_token.json")).unwrap();
    assert_eq!(t.token_type.as_deref(), Some("bearer"));
    assert!(t.access_token.starts_with("y0_"));
    assert_eq!(t.expires_in, Some(31_536_000));
    assert!(t.refresh_token.is_some());
}

#[test]
fn parses_album_with_tracks() {
    let a: auris_api::AlbumWithTracks =
        serde_json::from_str(&fixture("album_with_tracks.json")).unwrap();
    assert_eq!(a.album.id, 5275114);
    assert_eq!(a.album.title, "Группа крови");
    assert_eq!(a.album.artists.len(), 1);
    assert_eq!(a.labels.len(), 1);
    assert_eq!(a.all_tracks().len(), 2);
    assert_eq!(a.all_tracks()[1].title, "Закрой за мной дверь, я ухожу");
}

#[test]
fn parses_artist_brief() {
    let b: auris_api::ArtistBriefInfo =
        serde_json::from_str(&fixture("artist_brief.json")).unwrap();
    assert_eq!(b.artist.id, 1595);
    assert_eq!(b.artist.name, "Кино");
    assert_eq!(b.popular_tracks.len(), 2);
    assert_eq!(b.albums.len(), 1);
    assert_eq!(b.similar_artists.len(), 2);
    assert_eq!(b.similar_artists[1].name, "Наутилус Помпилиус");
}

#[test]
fn parses_similar_tracks() {
    let s: auris_api::SimilarTracks =
        serde_json::from_str(&fixture("similar_tracks.json")).unwrap();
    assert_eq!(s.similar_tracks.len(), 2);
    assert_eq!(s.similar_tracks[0].title, "Звезда по имени Солнце");
}

#[test]
fn parses_supplement() {
    let s: auris_api::Supplement = serde_json::from_str(&fixture("supplement.json")).unwrap();
    let lyrics = s.lyrics.expect("lyrics");
    assert!(lyrics.text.contains("Первый куплет"));
    assert_eq!(
        lyrics.major.as_ref().map(|m| m.name.as_str()),
        Some("Яндекс Музыка")
    );
}

#[test]
fn parses_rotor_dashboard() {
    let d: auris_api::RotorDashboard = serde_json::from_str(&fixture("dashboard.json")).unwrap();
    assert_eq!(d.stations.len(), 3);
    let wave = &d.stations[0];
    assert_eq!(
        wave.seed().as_deref(),
        Some("user:onyourwave"),
        "сид живёт внутри station.id"
    );
    assert_eq!(
        wave.station.as_ref().map(|s| s.name.clone()),
        Some("Моя волна".into())
    );
    let pl = &d.stations[1];
    assert_eq!(pl.seed().as_deref(), Some("playlist:39852"));
    assert_eq!(pl.station.as_ref().unwrap().name, "Плейлист дня");
}

#[test]
fn parses_likes_albums() {
    #[derive(serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Resp {
        likes: Vec<auris_api::LikedAlbum>,
    }
    let v: Resp = serde_json::from_str(&fixture("likes_albums.json")).unwrap();
    assert_eq!(v.likes.len(), 1);
    assert_eq!(v.likes[0].album.title, "Этажи");
    assert_eq!(v.likes[0].album.artists[0].name, "Молчат Дома");
    assert_eq!(v.likes[0].timestamp, Some(1730000000));
}

#[test]
fn parses_landing_blocks() {
    // Живой формат: массив блоков с полем type (как его собирает
    // `Client::landing` из полезной нагрузки).
    let blocks: Vec<auris_api::LandingBlock> =
        serde_json::from_str(&fixture("landing.json")).unwrap();
    let l = Landing { blocks };
    let personal = l.personal_playlists();
    assert_eq!(personal.len(), 1);
    assert_eq!(personal[0].title, "Плейлист дня");
    let mixes = l.mixes();
    assert_eq!(mixes.len(), 1);
    assert_eq!(mixes[0].title, "Микс свежести");
}
