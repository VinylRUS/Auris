//! Менеджер загрузок: очередь, прогресс, файлы в пользовательском каталоге.
//!
//! Загрузка разрешена только для контента, который API отдаёт с
//! `direct: true` (фактически — при действующей подписке). Это
//! осознанная правовая позиция проекта: никакого обхода ограничений.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use auris_api::{Client, DownloadInfo, Track};

use crate::config::Quality;

/// Статус загрузки трека.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DownloadState {
    /// В очереди.
    Queued,
    /// Скачивается (доля 0..1).
    Progress(f64),
    /// Готово.
    Done,
    /// Ошибка (текст в логах).
    Failed,
}

#[derive(Debug, Default)]
struct Shared {
    states: HashMap<i64, DownloadState>,
}

/// Менеджер загрузок.
#[derive(Clone)]
pub struct DownloadManager {
    api: Arc<Mutex<Client>>,
    shared: Arc<Mutex<Shared>>,
    quality: Arc<Mutex<Quality>>,
    dir: Arc<Mutex<PathBuf>>,
}

impl DownloadManager {
    /// Создаёт менеджер.
    pub fn new(api: Arc<Mutex<Client>>, dir: PathBuf, quality: Quality) -> Self {
        Self {
            api,
            shared: Arc::new(Mutex::new(Shared::default())),
            quality: Arc::new(Mutex::new(quality)),
            dir: Arc::new(Mutex::new(dir)),
        }
    }

    /// Обновить настройки.
    pub fn configure(&self, dir: PathBuf, quality: Quality) {
        *self.dir.lock().expect("downloads dir") = dir;
        *self.quality.lock().expect("downloads quality") = quality;
    }

    /// Каталог загрузок.
    pub fn dir(&self) -> PathBuf {
        self.dir.lock().expect("downloads dir").clone()
    }

    /// Снимок состояний загрузок.
    pub fn states(&self) -> HashMap<i64, DownloadState> {
        self.shared
            .lock()
            .map(|s| s.states.clone())
            .unwrap_or_default()
    }

    /// Имя файла трека: `Исполнитель - Название.mp3`.
    fn file_name(track: &Track) -> String {
        let sanitize = |s: &str| -> String {
            s.chars()
                .map(|c| match c {
                    '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '-',
                    _ => c,
                })
                .collect::<String>()
                .trim()
                .to_string()
        };
        format!(
            "{} - {}.mp3",
            sanitize(&track.artists_display()),
            sanitize(&track.title)
        )
    }

    /// Подбирает вариант загрузки под качество.
    ///
    /// Возвращает `None`, если трек недоступен для скачивания
    /// (нет подписки / регион / трек удалён).
    pub fn pick_info<'a>(&self, infos: &'a [DownloadInfo]) -> Option<&'a DownloadInfo> {
        let max_br = self.quality.lock().expect("quality").max_bitrate();
        // Приоритет: разрешённый битрейт (≤ max), затем максимальный из них.
        infos
            .iter()
            .filter(|i| i.codec == "mp3")
            .max_by_key(|i| (i.bitrate_in_kbps <= max_br, i.bitrate_in_kbps))
    }

    /// Ставит трек в очередь на загрузку (спавнит задачу).
    pub fn enqueue(&self, track: Track, runtime: &tokio::runtime::Handle) {
        self.set_state(track.id, DownloadState::Queued);
        let mgr = self.clone();
        let track_id = track.id;
        runtime.spawn(async move {
            if let Err(e) = mgr.run(track).await {
                log::error!("download of track {track_id} failed: {e}");
                mgr.set_state(track_id, DownloadState::Failed);
            }
        });
    }

    fn set_state(&self, track_id: i64, state: DownloadState) {
        let mut s = self.shared.lock().expect("downloads state poisoned");
        s.states.insert(track_id, state);
    }

    async fn run(&self, track: Track) -> anyhow::Result<()> {
        let client = self.api.lock().expect("api").clone();
        let infos = client
            .track_download_info(track.id)
            .await
            .map_err(|e| anyhow::anyhow!("download-info: {e}"))?;
        let Some(info) = self.pick_info(&infos) else {
            anyhow::bail!("нет доступных вариантов: вероятно, нужна подписка");
        };
        if !info.direct {
            anyhow::bail!("вариант не помечен как direct (нужна подписка)");
        }

        let link = auris_api::sign::direct_link(client.http(), &info.url)
            .await
            .map_err(|e| anyhow::anyhow!("direct link: {e}"))?;

        let dir = self.dir.lock().expect("downloads dir").clone();
        let target = dir.join(Self::file_name(&track));
        std::fs::create_dir_all(&dir)?;

        let tmp = target.with_extension("part");
        let resp = client.http().get(&link).send().await?.error_for_status()?;
        let total = resp.content_length();

        use futures_util::StreamExt;
        let mut stream = resp.bytes_stream();
        use tokio::io::AsyncWriteExt;
        let mut file = tokio::io::BufWriter::new(tokio::fs::File::create(&tmp).await?);
        let mut written: u64 = 0;
        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            file.write_all(&chunk).await?;
            written += chunk.len() as u64;
            let frac = total.map(|t| (written as f64 / t as f64).clamp(0.0, 1.0));
            if let Some(f) = frac {
                if (written % 262_144).max(1) >= 1 {
                    self.set_state(track.id, DownloadState::Progress(f));
                }
            }
        }
        file.flush().await?;
        drop(file);
        tokio::fs::rename(&tmp, &target).await?;
        self.set_state(track.id, DownloadState::Done);
        Ok(())
    }
}
