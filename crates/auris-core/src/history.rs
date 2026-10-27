//! Локальная история прослушиваний («Недавно слушали»).

use std::path::PathBuf;

use auris_api::Track;
use serde::{Deserialize, Serialize};

/// Максимум сохраняемых записей.
const CAP: usize = 60;

/// История.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct History {
    /// Недавно проигранные треки (новые в начале).
    #[serde(default)]
    pub tracks: Vec<Track>,
    /// Когда треки слушали (unix-секунды, параллельно `tracks`).
    #[serde(default)]
    pub played_at: Vec<u64>,
}

impl History {
    /// Путь к файлу истории.
    fn path() -> anyhow::Result<PathBuf> {
        Ok(dirs::data_dir()
            .ok_or_else(|| anyhow::anyhow!("no data directory"))?
            .join("auris")
            .join("history.json"))
    }

    /// Загружает историю (отсутствие файла — не ошибка).
    pub fn load() -> Self {
        let Ok(path) = Self::path() else {
            return Self::default();
        };
        std::fs::read_to_string(path)
            .ok()
            .and_then(|raw| serde_json::from_str(&raw).ok())
            .unwrap_or_default()
    }

    /// Сохраняет историю.
    pub fn save(&self) -> anyhow::Result<()> {
        let path = Self::path()?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, serde_json::to_string_pretty(self)?)?;
        Ok(())
    }

    /// Добавляет трек в начало истории (без дублей подряд).
    pub fn push(&mut self, track: &Track) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        // Убираем предыдущее вхождение того же трека.
        if let Some(pos) = self.tracks.iter().position(|t| t.id == track.id) {
            self.tracks.remove(pos);
            self.played_at.remove(pos);
        }
        self.tracks.insert(0, track.clone());
        self.played_at.insert(0, now);

        let overflow = self.tracks.len().saturating_sub(CAP);
        for _ in 0..overflow {
            self.tracks.pop();
            self.played_at.pop();
        }
    }

    /// Первые `n` треков (для «Недавно слушали»).
    pub fn recent(&self, n: usize) -> Vec<Track> {
        self.tracks.iter().take(n).cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn track(id: i64, title: &str) -> Track {
        let mut t = Track::default();
        t.id = id;
        t.title = title.into();
        t
    }

    #[test]
    fn dedupes_and_caps() {
        let mut h = History::default();
        for i in 0..80 {
            h.push(&track(i, "t"));
        }
        h.push(&track(42, "t"));
        assert_eq!(h.tracks.len(), CAP);
        assert_eq!(h.tracks[0].id, 42, "повторный push поднимает трек наверх");
        assert_eq!(h.recent(10).len(), 10);
    }
}
