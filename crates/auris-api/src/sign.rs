//! Вычисление прямых ссылок на аудиофайлы.
//!
//! `download-info` отдаёт не файл, а ссылку на XML-метаданные
//! `<host>/<path>/<s>/<ts>`. Прямая ссылка собирается по той же
//! формуле, что и в официальных клиентах:
//!
//! ```text
//! sign = md5(SIGN_SALT + path[1..] + s)
//! url  = https://{host}/get-mp3/{sign}/{ts}{path}
//! ```
//!
//! Никакого обхода DRM здесь нет: для зашифрованного (encraw) контента
//! API просто не отдаёт валидные ссылки.

use md5::{Digest, Md5};

use crate::error::{ApiError, Result};

/// Соль подписи (публично известна, используется всеми клиентами сообщества).
pub const SIGN_SALT: &str = "XGRlBW9FXlekgbPrRHuSiA";

/// Ответ XML-метаданных.
#[derive(Debug, Clone, PartialEq)]
pub struct FileMeta {
    /// Хост CDN.
    pub host: String,
    /// Путь до файла.
    pub path: String,
    /// Метка времени.
    pub ts: String,
    /// Соль ответа.
    pub s: String,
}

/// Извлекает текст первого тега `<tag>…</tag>` из XML.
fn extract_tag(xml: &str, tag: &str) -> Option<String> {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let start = xml.find(&open)? + open.len();
    let end = xml[start..].find(&close)? + start;
    Some(xml[start..end].trim().to_string())
}

/// Разбирает XML-метаданные.
pub fn parse_meta(xml: &str) -> Result<FileMeta> {
    let get = |tag: &str| {
        extract_tag(xml, tag)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| ApiError::Unexpected(format!("missing <{tag}> in download XML")))
    };
    Ok(FileMeta {
        host: get("host")?,
        path: get("path")?,
        ts: get("ts")?,
        s: get("s")?,
    })
}

/// Собирает прямую ссылку из метаданных.
pub fn build_direct_link(meta: &FileMeta) -> String {
    let path = meta.path.strip_prefix('/').unwrap_or(&meta.path);
    let mut hasher = Md5::new();
    hasher.update(SIGN_SALT.as_bytes());
    hasher.update(path.as_bytes());
    hasher.update(meta.s.as_bytes());
    let sign = hex(&hasher.finalize());
    format!(
        "https://{}/get-mp3/{}/{ts}{path}",
        meta.host,
        sign,
        ts = meta.ts,
        path = meta.path
    )
}

/// Загружает XML по ссылке из `download-info` и сразу собирает прямую ссылку.
pub async fn direct_link(http: &reqwest::Client, info_url: &str) -> Result<String> {
    // Пустой/относительный URL — симптом рассинхронизации модели с API
    // (например, переименовали поле). Reqwest в таком случае падает с
    // малопонятным «builder error: relative URL without a base» —
    // заменим на явную причину.
    if !info_url.starts_with("http://") && !info_url.starts_with("https://") {
        return Err(ApiError::Unexpected(format!(
            "download-info вернул непригодный URL ({info_url:?}): \
             модель рассинхронизирована с API или трек недоступен"
        )));
    }
    let xml = http
        .get(info_url)
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;
    let meta = parse_meta(&xml)?;
    Ok(build_direct_link(&meta))
}

/// Нижний регистр hex.
fn hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push_str(&format!("{b:02x}"));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Пустой/относительный URL должен давать понятную ошибку,
    /// а не «builder error: relative URL without a base» из reqwest.
    #[tokio::test]
    async fn rejects_relative_url_with_clear_error() {
        let http = reqwest::Client::new();
        for bad in ["", "/get-audio-info", "storage.mds.yandex.net/x"] {
            let err = direct_link(&http, bad).await.unwrap_err();
            let msg = format!("{err}");
            assert!(msg.contains("непригодный URL"), "got: {msg}");
        }
    }

    #[test]
    fn parses_meta_and_builds_link() {
        let xml = "<download-info>\
                   <schema>http</schema>\
                   <host>storage.mds.yandex.net</host>\
                   <path>/get-music-content/03/ae/track.file</path>\
                   <ts>67000000</ts>\
                   <region>225</region>\
                   <s>a1b2c3d4e5</s>\
                   </download-info>";
        let meta = parse_meta(xml).expect("parse");
        assert_eq!(meta.host, "storage.mds.yandex.net");
        assert_eq!(meta.path, "/get-music-content/03/ae/track.file");
        assert_eq!(meta.ts, "67000000");
        assert_eq!(meta.s, "a1b2c3d4e5");

        let link = build_direct_link(&meta);
        assert!(link.starts_with("https://storage.mds.yandex.net/get-mp3/"));
        assert!(link.ends_with("/67000000/get-music-content/03/ae/track.file"));

        // Эталон подписи: md5("XGRlBW9FXlekgbPrRHuSiA" + "get-music-content/03/ae/track.file" + "a1b2c3d4e5")
        let mut h = Md5::new();
        h.update(b"XGRlBW9FXlekgbPrRHuSiAget-music-content/03/ae/track.filea1b2c3d4e5");
        assert!(
            link.contains(&hex(&h.finalize())),
            "link must embed the salted md5"
        );
    }
}
