# Покрытие API Яндекс.Музыки

Аудит по спецификации [MarshalX/yandex-music-api](https://github.com/MarshalX/yandex-music-api)
(актуально на 2026-10). Оценка: что уже реализовано в `auris-api`, что добавлено
в этой итерации, что сознательно не реализуется.

## Сводка

| Раздел MarshalX | Статус | Методы `auris-api` |
|---|---|---|
| account | ✅ реализовано | `account_status` |
| albums | ✅ **добавлено** | `album_with_tracks`, `albums` |
| artists | ✅ **добавлено** | `artists_brief_info` |
| search | ✅ реализовано | `search`, `search_tracks`, `search_suggest` |
| tracks | ✅ расширено | `tracks`, `track_download_info`, **`tracks_similar`**, **`track_supplement`** (тексты) |
| playlists | ✅ реализовано | `users_playlists_list`, `user_playlist` |
| likes | ✅ **расширено** | `users_likes_tracks`, **`users_likes_albums`**, **`users_likes_playlists`**, **`users_likes_artists`**, `likes_tracks_add/remove` |
| landing | ✅ расширено | `landing_new_releases`, `landing_chart`, **`landing`** (personalPlaylists, mixes) |
| feed | ✅ реализовано | `feed` |
| radio (rotor) | ✅ **расширено** | **`rotor_stations_dashboard`**, `rotor_session_new`, `rotor_session_tracks`, `rotor_session_feedback` |
| device_auth | ✅ реализовано | `auth::request_device_code`, `auth::poll_token` |
| queue (синхронизация устройств) | ⏳ отложено | — |
| music_history (история серверная) | ⏳ отложено (локальная история уже есть) | — |
| playlists (мутации: create/rename/delete) | ⏳ отложено | — |
| presaves, pins, clips, concerts, disclaimers, labels, metatags, credits, ynison | ❌ вне scopes десктоп-плеера | — |

## Решения

- **queue** — синхронизация очереди между устройствами; добавим, когда появится
  «продолжить на другом устройстве». Протокол: `GET /queues`, `PUT /queue`.
- **music_history** — серверная история прослушиваний; в Auris уже есть локальная
  (`auris-core::history`), серверная не критична для MVP.
- **Мутации плейлистов** — создание/переименование/удаление; UI для редактирования
  плейлистов ещё не готов, методы добавим вместе с ним.
- **ynison** — WebSocket-протокол синхронизации плеера между устройствами;
  отдельный большой модуль, вне текущего этапа.

## Проверка форматов

Все модели покрыты golden-тестами (`tests/golden.rs`, фикстуры в
`tests/fixtures/`): живые ответы зафиксированы и парсинг не сломается при
рефакторинге. OAuth-эндпоинты Яндекса отвечают в snake_case (в отличие от
основного API — camelCase), что зафиксировано тестом `parses_device_code_and_token`.
