//! Настройки приложения (JSON рядом с программой: `<корень>/config`).

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Качество загрузки/воспроизведения.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Quality {
    /// 64–128 кбит/с.
    Low,
    /// 192 кбит/с.
    Normal,
    /// 320 кбит/с (требует подписку).
    High,
}

impl Default for Quality {
    fn default() -> Self {
        Self::High
    }
}

impl Quality {
    /// Максимальный битрейт для качества.
    pub fn max_bitrate(self) -> u64 {
        match self {
            Quality::Low => 128,
            Quality::Normal => 192,
            Quality::High => 320,
        }
    }
}

/// Настройки темы.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThemeSettings {
    /// Идентификатор пресета (`fluent-dark`, `pastel-warm`, …).
    pub preset: String,
    /// Пользовательский акцент в формате `#RRGGBB` (перекрывает пресет).
    #[serde(default)]
    pub accent_override: Option<String>,
}

impl Default for ThemeSettings {
    fn default() -> Self {
        Self {
            preset: "fluent-dark".into(),
            accent_override: None,
        }
    }
}

/// Настройки загрузок.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadSettings {
    /// Каталог загрузок (выбирается пользователем).
    pub dir: PathBuf,
    /// Качество.
    pub quality: Quality,
}

impl Default for DownloadSettings {
    fn default() -> Self {
        let dir = dirs::audio_dir()
            .or_else(dirs::download_dir)
            .unwrap_or_else(|| PathBuf::from("."))
            .join("Auris");
        Self {
            dir,
            quality: Quality::High,
        }
    }
}

/// Памятка окна.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowSettings {
    /// Ширина, px.
    pub width: f32,
    /// Высота, px.
    pub height: f32,
}

impl Default for WindowSettings {
    fn default() -> Self {
        Self {
            width: 1280.0,
            height: 800.0,
        }
    }
}

/// Корневые настройки приложения.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(default)]
pub struct Config {
    /// Тема оформления.
    pub theme: ThemeSettings,
    /// Загрузки.
    pub downloads: DownloadSettings,
    /// Громкость при старте [0..1].
    pub volume: f32,
    /// Окно.
    pub window: WindowSettings,
    /// Каталог кеша (пусто → стандартный кеш ОС).
    pub cache_dir: Option<PathBuf>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            theme: ThemeSettings::default(),
            downloads: DownloadSettings::default(),
            volume: 0.7,
            window: WindowSettings::default(),
            cache_dir: None,
        }
    }
}

impl Config {
    /// Путь к файлу настроек: `<корень данных>/config/config.json`
    /// (по умолчанию — рядом с исполняемым файлом, см. [`crate::paths`]).
    pub fn path() -> anyhow::Result<PathBuf> {
        Ok(crate::paths::config_dir().join("config.json"))
    }

    /// Загружает настройки; при отсутствии/поломке создаёт дефолтные.
    pub fn load() -> anyhow::Result<Self> {
        let path = Self::path()?;
        if !path.exists() {
            let cfg = Self::default();
            cfg.save()?;
            return Ok(cfg);
        }
        let raw = std::fs::read_to_string(&path)?;
        match serde_json::from_str(&raw) {
            Ok(cfg) => Ok(cfg),
            Err(e) => {
                log::warn!("config is corrupted ({e}), resetting to defaults");
                let cfg = Self::default();
                cfg.save()?;
                Ok(cfg)
            }
        }
    }

    /// Сохраняет настройки.
    pub fn save(&self) -> anyhow::Result<()> {
        let path = Self::path()?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, serde_json::to_string_pretty(self)?)?;
        Ok(())
    }

    /// Каталог кеша (обложки, аудио): `<корень>/cache` по умолчанию.
    pub fn cache_root(&self) -> PathBuf {
        self.cache_dir
            .clone()
            .unwrap_or_else(crate::paths::cache_dir)
    }
}
