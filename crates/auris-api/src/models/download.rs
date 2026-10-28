//! Варианты загрузки трека.

use serde::{Deserialize, Serialize};

use super::flex;

/// Вариант загрузки трека из `/tracks/{id}/download-info`.
///
/// `download_info_url` — ссылка на XML-метаданные, из которых вычисляется
/// прямая ссылка на аудиофайл (см. [`crate::sign::direct_link`]).
///
/// ВАЖНО: живой API отдаёт поле как `downloadInfoUrl` (как и
/// MarshalX/yandex-music-api). Раньше модель ожидала просто `url`,
/// из-за чего поле молча оказывалось пустым и воспроизведение падало
/// с «relative URL without a base».
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
    /// Ссылка на XML с метаданными файла (`downloadInfoUrl`).
    #[serde(default)]
    pub download_info_url: String,
    /// Поставщик.
    #[serde(default)]
    pub supplier: Option<String>,
    /// Превью-лимит (для отрывков без подписки).
    #[serde(default)]
    pub preview: Option<bool>,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Живая форма ответа `/tracks/{id}/download-info`: поле с URL
    /// называется `downloadInfoUrl` (сверено с MarshalX/yandex-music-api).
    #[test]
    fn parses_live_download_info() {
        let raw = r#"[
            {
                "codec": "aac",
                "bitrateInKbps": 64,
                "gain": true,
                "preview": false,
                "downloadInfoUrl": "https://storage.mds.yandex.net/audio-headers/03/ae/track.aac",
                "direct": false,
                "supplier": "default"
            },
            {
                "codec": "mp3",
                "bitrateInKbps": 320,
                "gain": true,
                "preview": false,
                "downloadInfoUrl": "https://storage.mds.yandex.net/audio-headers/03/ae/track.mp3",
                "direct": true
            }
        ]"#;
        let infos: Vec<DownloadInfo> = serde_json::from_str(raw).expect("parse");
        assert_eq!(infos.len(), 2);
        assert_eq!(infos[0].codec, "aac");
        assert_eq!(infos[0].bitrate_in_kbps, 64);
        assert!(
            infos[1].download_info_url.starts_with("https://"),
            "URL должен подтянуться из downloadInfoUrl, а не остаться пустым"
        );
        assert_eq!(infos[1].bitrate_in_kbps, 320);
        assert!(infos[1].direct);
    }

    /// Раньше модель ждала `url`, живой API отдаёт `downloadInfoUrl`:
    /// регрессия не должна возвращаться молча — URL обязан быть непустым
    /// для помеченных `direct` вариантов.
    #[test]
    fn wrong_field_name_leaves_url_empty() {
        let raw = r#"[{"codec":"mp3","bitrateInKbps":192,"url":"https://x/y"}]"#;
        let infos: Vec<DownloadInfo> = serde_json::from_str(raw).expect("parse");
        assert_eq!(infos[0].download_info_url, "");
    }
}
