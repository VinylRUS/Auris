//! Автообновление через GitHub Releases.
//!
//! Канал обновлений — файл `latest.json` в «последнем» релизе:
//! <https://github.com/VinylRUS/Auris/releases/latest/download/latest.json>
//!
//! Схема: приложение сравнивает версию из манифеста со своей, при наличии
//! новой скачивает установщик (Windows) или открывает страницу релиза
//! (macOS/Linux), проверяя SHA-256 из манифеста.

use std::path::PathBuf;

use serde::Deserialize;
use sha2::{Digest, Sha256};

/// Репозиторий-канал обновлений.
pub const REPO: &str = "VinylRUS/Auris";

/// URL манифеста последнего релиза.
pub const MANIFEST_URL: &str =
    "https://github.com/VinylRUS/Auris/releases/latest/download/latest.json";

/// Страница релизов (для платформ без установщика).
pub const RELEASES_URL: &str = "https://github.com/VinylRUS/Auris/releases/latest";

/// Платформа текущей сборки (ключ в манифесте).
pub fn platform_key() -> &'static str {
    if cfg!(target_os = "windows") {
        "windows-x86_64"
    } else if cfg!(target_os = "macos") {
        "macos-aarch64"
    } else {
        "linux-x86_64"
    }
}

/// Манифест `latest.json` из релиза.
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateManifest {
    /// Версия в формате Cargo (`0.0.1-beta.1`).
    pub version: String,
    /// Тег релиза (`v0.0.1b`).
    #[serde(default)]
    pub tag: String,
    /// Краткие примечания к релизу.
    #[serde(default)]
    pub notes: String,
    /// Артефакты по платформам.
    #[serde(default)]
    pub platforms: std::collections::BTreeMap<String, PlatformAsset>,
}

/// Артефакт обновления для платформы.
#[derive(Debug, Clone, Deserialize)]
pub struct PlatformAsset {
    /// Прямая ссылка на скачивание.
    pub url: String,
    /// Контрольная сумма SHA-256 (hex, нижний регистр).
    #[serde(default)]
    pub sha256: Option<String>,
}

impl UpdateManifest {
    /// Артефакт для текущей платформы.
    pub fn asset(&self) -> Option<&PlatformAsset> {
        self.platforms.get(platform_key())
    }
}

/// Тянет и разбирает манифест последнего релиза.
pub async fn fetch_manifest(http: &reqwest::Client) -> anyhow::Result<UpdateManifest> {
    let resp = http.get(MANIFEST_URL).send().await?;
    if !resp.status().is_success() {
        anyhow::bail!("manifest HTTP {}", resp.status());
    }
    let manifest: UpdateManifest = resp.json().await?;
    Ok(manifest)
}

/// Разбирает версию вида `0.0.1-beta.1` или `0.0.1b` в сравнимые части.
///
/// Возвращает `(major, minor, patch, pre)`, где `pre` — идентификатор
/// пре-релиза (`Some(("beta", 1))`) или `None` для финала.
fn parse_version(v: &str) -> Option<(u64, u64, u64, Option<(String, u64)>)> {
    let v = v.trim().trim_start_matches('v');
    let (core, pre) = match v.split_once('-') {
        Some((c, p)) => (c, Some(p)),
        None => (v, None),
    };
    let mut it = core.split('.');
    let major = it.next()?.parse().ok()?;
    let minor = it.next()?.parse().ok()?;
    // Patch может нести суффикс: `1b`, `1rc2`.
    let patch_raw = it.next().unwrap_or("0");
    let digits: String = patch_raw
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .collect();
    let patch: u64 = if digits.is_empty() {
        0
    } else {
        digits.parse().unwrap_or(0)
    };
    let suffix: String = patch_raw
        .chars()
        .skip_while(|c| c.is_ascii_digit())
        .collect();
    let pre = match (pre, suffix.is_empty()) {
        // Явный пре-релиз через дефис приоритетнее.
        (Some(p), _) => Some(split_pre(p)),
        (None, false) => Some(split_pre(&suffix)),
        (None, true) => None,
    };
    Some((major, minor, patch, pre))
}

/// `"beta.1"` → `("beta", 1)`; `"b"` → `("b", 0)`.
fn split_pre(p: &str) -> (String, u64) {
    match p.split_once('.') {
        Some((n, r)) => (n.to_lowercase(), r.parse().unwrap_or(0)),
        None => (p.to_lowercase(), 0),
    }
}

/// Приоритет пре-релизного идентификатора (semver: alpha < beta < rc).
fn pre_rank(name: &str) -> u64 {
    match name {
        "pre" | "alpha" | "a" => 1,
        "beta" | "b" => 2,
        "rc" => 3,
        _ => 4,
    }
}

/// `remote` строго новее `current`?
pub fn is_newer(remote: &str, current: &str) -> bool {
    let (r, c) = match (parse_version(remote), parse_version(current)) {
        (Some(r), Some(c)) => (r, c),
        // Не удалось разобрать — считаем, что обновления нет.
        _ => return false,
    };
    // Числовые части.
    if r.0 != c.0 {
        return r.0 > c.0;
    }
    if r.1 != c.1 {
        return r.1 > c.1;
    }
    if r.2 != c.2 {
        return r.2 > c.2;
    }
    // Одинаковое ядро: финал новее любого пре-релиза.
    match (&r.3, &c.3) {
        (None, None) => false,
        (None, Some(_)) => true,
        (Some(_), None) => false,
        (Some(rp), Some(cp)) => {
            let (rr, cr) = (pre_rank(&rp.0), pre_rank(&cp.0));
            if rr != cr {
                return rr > cr;
            }
            rp.1 > cp.1
        }
    }
}

/// Скачивает файл обновления во временный каталог с прогрессом.
///
/// `progress` вызывается с долей завершения (0.0..1.0), `cancelled` —
/// опрашивается между чанками: при `true` скачивание останавливается и
/// возвращается `Ok(None)`.
///
/// Возвращает путь к скачанному файлу после проверки SHA-256
/// (если манифест содержит сумму).
pub async fn download_asset(
    http: &reqwest::Client,
    asset: &PlatformAsset,
    file_name: &str,
    mut progress: impl FnMut(f64) + Send,
    mut cancelled: impl FnMut() -> bool + Send,
) -> anyhow::Result<Option<PathBuf>> {
    let resp = http.get(&asset.url).send().await?;
    if !resp.status().is_success() {
        anyhow::bail!("download HTTP {}", resp.status());
    }
    let total = resp.content_length().unwrap_or(0);

    let dir = std::env::temp_dir().join("auris-update");
    std::fs::create_dir_all(&dir)?;
    // Уникальное имя, чтобы не конфликтовать с параллельными запусками.
    let path = dir.join(format!(
        "{}-{}",
        std::process::id(),
        file_name.replace(['/', '\\'], "_")
    ));

    let mut file = tokio::fs::File::create(&path).await?;
    let mut stream = resp.bytes_stream();
    let mut downloaded: u64 = 0;
    use futures_util::StreamExt;
    use tokio::io::AsyncWriteExt;
    while let Some(chunk) = stream.next().await {
        if cancelled() {
            let _ = tokio::fs::remove_file(&path).await;
            return Ok(None);
        }
        let chunk = chunk?;
        file.write_all(&chunk).await?;
        downloaded += chunk.len() as u64;
        if total > 0 {
            progress(downloaded as f64 / total as f64);
        }
    }
    file.flush().await?;
    drop(file);
    progress(1.0);

    // Контрольная сумма.
    if let Some(expected) = &asset.sha256 {
        let expected = expected.trim().to_lowercase();
        if !expected.is_empty() {
            let bytes = tokio::fs::read(&path).await?;
            let mut hasher = Sha256::new();
            hasher.update(&bytes);
            let actual = hex(&hasher.finalize());
            if actual != expected {
                let _ = std::fs::remove_file(&path);
                anyhow::bail!("SHA-256 не совпал: ожидался {expected}, получен {actual}");
            }
        }
    }
    Ok(Some(path))
}

/// HEX без внешних зависимостей.
fn hex(bytes: &[u8]) -> String {
    const T: &[u8; 16] = b"0123456789abcdef";
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push(T[(b >> 4) as usize] as char);
        s.push(T[(b & 0xf) as usize] as char);
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versions() {
        assert!(is_newer("0.0.2", "0.0.1-beta.1"));
        assert!(is_newer("0.1.0", "0.0.9"));
        assert!(is_newer("1.0.0", "0.9.9"));
        assert!(is_newer("0.0.1-beta.2", "0.0.1-beta.1"));
        assert!(is_newer("0.0.1-rc.1", "0.0.1-beta.9"));
        assert!(is_newer("0.0.1", "0.0.1-beta.1"));
        assert!(is_newer("0.0.1", "0.0.1-rc.1"));
        assert!(!is_newer("0.0.1-beta.1", "0.0.1-beta.1"));
        assert!(!is_newer("0.0.1-beta.1", "0.0.2"));
        assert!(!is_newer("0.0.1-beta.2", "0.0.1"));
        // «Народный» формат 0.0.1b тоже должен сравниваться.
        assert!(!is_newer("0.0.1b", "0.0.1-beta.1"));
        assert!(is_newer("0.0.1b", "0.0.1a"));
        assert!(is_newer("0.0.1c", "0.0.1b"));
    }

    #[test]
    fn manifest_parse() {
        let raw = r#"{
            "version": "0.0.1-beta.1",
            "tag": "v0.0.1b",
            "notes": "Первый релиз с установщиком",
            "platforms": {
                "windows-x86_64": {
                    "url": "https://example.com/Auris-Setup-0.0.1b-x64.exe",
                    "sha256": "aa"
                }
            }
        }"#;
        let m: UpdateManifest = serde_json::from_str(raw).unwrap();
        assert_eq!(m.version, "0.0.1-beta.1");
        assert!(m.asset().is_some() || !cfg!(target_os = "windows"));
    }
}
