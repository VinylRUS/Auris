//! Кеш обложек и аудио.
//!
//! Обложки скачиваются в `covers/` по md5(url); UI получает локальный
//! путь и показывает его через `Image`. Аудио перед воспроизведением
//! целиком кладётся в `audio/` — это и стриминг-буфер, и офлайн-кеш.

use std::path::PathBuf;

use md5::{Digest, Md5};

/// Кеш-менеджер.
#[derive(Debug, Clone)]
pub struct Cache {
    /// Корень кеша.
    pub root: PathBuf,
    /// HTTP-клиент.
    pub http: reqwest::Client,
}

/// Готовый к проигрыванию файл: либо уже в кеше, либо свежескачанный.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CachedFile {
    /// Файл уже лежал в кеше.
    Hit(PathBuf),
    /// Файл скачан сейчас.
    Fetched(PathBuf),
}

impl Cache {
    /// Создаёт кеш (с подкаталогами) по корню.
    pub fn new(root: impl Into<PathBuf>, http: reqwest::Client) -> anyhow::Result<Self> {
        let root = root.into();
        std::fs::create_dir_all(root.join("covers"))?;
        std::fs::create_dir_all(root.join("audio"))?;
        Ok(Self { root, http })
    }

    /// Размер кеша в байтах (рекурсивно, для настроек).
    pub fn size_bytes(&self) -> u64 {
        dir_size(&self.root)
    }

    /// Полная очистка кеша.
    pub fn clear(&self) -> anyhow::Result<()> {
        for sub in ["covers", "audio"] {
            let dir = self.root.join(sub);
            if dir.exists() {
                for entry in std::fs::read_dir(&dir)? {
                    let entry = entry?;
                    if entry.path().is_file() {
                        std::fs::remove_file(entry.path())?;
                    }
                }
            }
        }
        Ok(())
    }

    // ------------------------------------------------------------------
    // Обложки
    // ------------------------------------------------------------------

    /// Локальный путь обложки URL (проверять существование отдельно).
    pub fn cover_path(&self, url: &str) -> PathBuf {
        self.root
            .join("covers")
            .join(format!("{}.img", digest(url)))
    }

    /// Возвращает путь к обложке, при необходимости скачивая её.
    /// Ошибки сети логируются и возвращают исходный URL — UI сам
    /// решает, что показать вместо картинки.
    pub async fn cover(&self, url: &str) -> String {
        let path = self.cover_path(url);
        if path.exists() {
            return path_to_str(&path);
        }
        match self.http.get(url).send().await {
            Ok(resp) if resp.status().is_success() => match resp.bytes().await {
                Ok(bytes) => {
                    if let Err(e) = std::fs::write(&path, &bytes) {
                        log::warn!("cannot cache cover: {e}");
                        return url.to_string();
                    }
                    path_to_str(&path)
                }
                Err(e) => {
                    log::warn!("cover body failed: {e}");
                    url.to_string()
                }
            },
            _ => url.to_string(),
        }
    }

    // ------------------------------------------------------------------
    // Аудио
    // ------------------------------------------------------------------

    /// Локальный путь аудиофайла трека.
    pub fn audio_path(&self, track_id: i64, quality_tag: &str) -> PathBuf {
        self.root
            .join("audio")
            .join(format!("{track_id}-{quality_tag}.audio"))
    }

    /// Скачивает файл по прямой ссылке в кеш и возвращает путь.
    ///
    /// Файл пишется во временный `.part` и переименовывается атомарно,
    /// чтобы не подсунуть плееру половину файла после сбоя.
    pub async fn fetch_audio(
        &self,
        track_id: i64,
        quality_tag: &str,
        url: &str,
    ) -> anyhow::Result<CachedFile> {
        let final_path = self.audio_path(track_id, quality_tag);
        if final_path.exists() {
            return Ok(CachedFile::Hit(final_path));
        }

        let tmp = final_path.with_extension("part");
        let resp = self.http.get(url).send().await?.error_for_status()?;
        let total = resp.content_length();

        use futures_util::StreamExt;
        let mut stream = resp.bytes_stream();
        use tokio::io::AsyncWriteExt;
        let mut file = tokio::io::BufWriter::new(tokio::fs::File::create(&tmp).await?);
        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            file.write_all(&chunk).await?;
        }
        let _ = total;
        file.flush().await?;
        drop(file);
        tokio::fs::rename(&tmp, &final_path).await?;
        Ok(CachedFile::Fetched(final_path))
    }
}

fn digest(s: &str) -> String {
    let mut h = Md5::new();
    h.update(s.as_bytes());
    let out = h.finalize();
    out.iter().map(|b| format!("{b:02x}")).collect()
}

fn path_to_str(p: &std::path::Path) -> String {
    p.to_string_lossy().into_owned()
}

fn dir_size(dir: &std::path::Path) -> u64 {
    let mut total = 0;
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                total += dir_size(&p);
            } else if let Ok(md) = entry.metadata() {
                total += md.len();
            }
        }
    }
    total
}
