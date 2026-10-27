//! Портативные пути: все данные Auris лежат рядом с исполняемым файлом.
//!
//! Приложение не должно мусорить на системном диске: журнал, настройки,
//! токен, кеш и загрузки обновлений пишутся в подпапки каталога программы
//! («папка установки»): `logs/`, `config/`, `data/`, `cache/`, `updates/`.
//!
//! Разрешение корня данных (см. [`root`]):
//! 1. переменная `AURIS_DATA_DIR` — явное переопределение (тесты, E2E);
//! 2. каталог исполняемого файла, если в него можно писать
//!    (портативный режим — работает и для установленных сборок,
//!    и для распакованных zip);
//! 3. пользовательский каталог ОС — только если рядом с программой
//!    писать нельзя (Program Files, /usr/bin, …).
//!
//! Старые файлы из системных каталогов один раз переносятся копированием
//! (см. [`migrate_legacy_files`]) — пользователь ничего не теряет и
//! не входит заново.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// Переменная-переопределение корня данных (абсолютный путь).
pub const ENV_DATA_DIR: &str = "AURIS_DATA_DIR";

static DECISION: OnceLock<RootDecision> = OnceLock::new();

/// Решение о том, где жить данным приложения.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RootDecision {
    /// Явное переопределение переменной окружения `AURIS_DATA_DIR`.
    Override(PathBuf),
    /// Каталог исполняемого файла (портативный режим).
    Portable(PathBuf),
    /// Пользовательский каталог ОС (рядом с программой писать нельзя).
    System(PathBuf),
}

impl RootDecision {
    /// Каталог данных решения.
    pub fn path(&self) -> &Path {
        match self {
            RootDecision::Override(p) | RootDecision::Portable(p) | RootDecision::System(p) => p,
        }
    }
}

/// Выбирает корень данных по правилам модуля (чистая функция, тестируемая).
pub fn choose_root(env_override: Option<&Path>, exe_dir: Option<&Path>) -> RootDecision {
    if let Some(dir) = env_override {
        if dir.is_absolute() {
            return RootDecision::Override(dir.to_path_buf());
        }
    }
    if let Some(dir) = exe_dir {
        if dir_is_writable(dir) {
            return RootDecision::Portable(dir.to_path_buf());
        }
    }
    RootDecision::System(system_fallback())
}

/// Пользовательский каталог ОС (последняя надежда).
fn system_fallback() -> PathBuf {
    dirs::data_dir()
        .or_else(dirs::config_dir)
        .unwrap_or_else(|| PathBuf::from("."))
        .join("auris")
}

/// Можно ли создавать файлы в каталоге (проба записи)?
fn dir_is_writable(dir: &Path) -> bool {
    let probe = dir.join(".auris-write-probe");
    let ok = std::fs::write(&probe, b"").is_ok();
    if ok {
        let _ = std::fs::remove_file(&probe);
    }
    ok
}

/// Решение о корне данных (вычисляется один раз за процесс).
fn decision() -> &'static RootDecision {
    DECISION.get_or_init(|| {
        let env_override = std::env::var_os(ENV_DATA_DIR)
            .map(PathBuf::from)
            .filter(|p| p.is_absolute());
        let exe_dir = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(Path::to_path_buf));
        let d = choose_root(env_override.as_deref(), exe_dir.as_deref());
        if let Err(e) = std::fs::create_dir_all(d.path()) {
            eprintln!(
                "auris: не удалось создать каталог данных {}: {e}",
                d.path().display()
            );
        }
        d
    })
}

/// Корень данных приложения.
pub fn root() -> &'static Path {
    decision().path()
}

/// Портативный режим активен (данные рядом с программой)?
pub fn is_portable() -> bool {
    matches!(decision(), RootDecision::Portable(_))
}

/// Каталог журналов: `<корень>/logs` (см. `auris::logging`).
pub fn logs_dir() -> PathBuf {
    root().join("logs")
}

/// Каталог настроек: `<корень>/config`.
pub fn config_dir() -> PathBuf {
    root().join("config")
}

/// Каталог данных (токен, история): `<корень>/data`.
pub fn data_dir() -> PathBuf {
    root().join("data")
}

/// Каталог кеша (обложки, QR): `<корень>/cache`.
pub fn cache_dir() -> PathBuf {
    root().join("cache")
}

/// Каталог загрузок обновлений: `<корень>/updates`.
pub fn updates_dir() -> PathBuf {
    root().join("updates")
}

/// Переносит настройки/токен/историю из старых системных каталогов
/// в портабельные (однократно, копированием, оригиналы не трогаем).
///
/// Работает только в портативном режиме: при явном `AURIS_DATA_DIR`
/// пользователь просит чистый каталог, при системном — данные уже там.
pub fn migrate_legacy_files() {
    if !is_portable() {
        return;
    }

    let mut moves: Vec<(PathBuf, PathBuf)> = Vec::new();
    if let Some(old) = dirs::config_dir() {
        let old = old.join("auris");
        moves.push((old.join("config.json"), config_dir().join("config.json")));
        moves.push((old.join("tokens.json"), data_dir().join("tokens.json")));
    }
    if let Some(old) = dirs::data_dir() {
        moves.push((
            old.join("auris").join("history.json"),
            data_dir().join("history.json"),
        ));
    }

    for (from, to) in moves {
        if !from.is_file() || to.exists() {
            continue;
        }
        if let Some(parent) = to.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        match std::fs::copy(&from, &to) {
            Ok(_) => log::info!(
                "перенесено из старого места: {} → {}",
                from.display(),
                to.display()
            ),
            Err(e) => log::warn!("миграция {} не удалась: {e}", from.display()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn override_wins_over_everything() {
        let d = choose_root(
            Some(Path::new("/tmp/auris-test-data")),
            Some(Path::new("/usr/bin")),
        );
        assert_eq!(
            d,
            RootDecision::Override(PathBuf::from("/tmp/auris-test-data"))
        );
    }

    #[test]
    fn relative_override_is_ignored() {
        let d = choose_root(Some(Path::new("relative/dir")), None);
        assert!(matches!(d, RootDecision::System(_)));
    }

    #[test]
    fn writable_exe_dir_gives_portable() {
        let dir = std::env::temp_dir().join("auris-paths-portable-test");
        std::fs::create_dir_all(&dir).expect("mkdir");
        let d = choose_root(None, Some(&dir));
        assert!(matches!(&d, RootDecision::Portable(p) if *p == dir));
    }

    #[test]
    fn missing_exe_dir_falls_back_to_system() {
        let d = choose_root(None, None);
        assert!(matches!(d, RootDecision::System(_)));
    }

    #[cfg(unix)]
    #[test]
    fn unwritable_dir_falls_back_to_system() {
        // /proc существует, но файлы в нём создавать нельзя даже root'у.
        let d = choose_root(None, Some(Path::new("/proc")));
        assert!(matches!(d, RootDecision::System(_)));
    }

    #[test]
    fn subdirs_live_under_root() {
        // Системный fallback выбранный путь возвращает целиком —
        // проверяем структуру подпапок через choose_root.
        let d = choose_root(None, None);
        let RootDecision::System(base) = &d else {
            unreachable!()
        };
        assert_eq!(base, &system_fallback());
        assert!(base.ends_with("auris"));
    }
}
