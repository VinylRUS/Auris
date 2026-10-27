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
fn parses_likes() {
    let l: auris_api::api::LikesTracks = serde_json::from_str(&fixture("likes.json")).unwrap();
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
    let d: auris_api::DeviceCode = serde_json::from_str(&fixture("device_code.json")).unwrap();
    assert_eq!(d.user_code, "ABCD-1234");
    assert!(d.verification_url.starts_with("https://ya.ru/device"));
    assert_eq!(d.interval, 5);

    let t: auris_api::OAuthToken = serde_json::from_str(&fixture("oauth_token.json")).unwrap();
    assert_eq!(t.token_type.as_deref(), Some("bearer"));
    assert!(t.access_token.starts_with("y0_"));
}
