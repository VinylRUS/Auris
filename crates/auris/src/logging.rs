//! Журналирование Auris: файл в папке установки + эхо в консоль.
//!
//! Журнал живёт рядом с программой: `<папка установки>/logs/auris.log`
//! (портативный режим выбирается в [`auris_core::paths`]; при
//! не-перезаписываемом каталоге установки — в каталоге данных ОС).
//! По размеру файл ротируется: `auris.log` → `auris.log.1` … `auris.log.3`.
//!
//! Уровень задаётся переменной `AURIS_LOG` (`trace`/`debug`/`info`/`warn`/
//! `error`), по умолчанию `info` — в журнал попадают ошибки и heartbeat
//! опроса OAuth. Для детальной диагностики сети: `AURIS_LOG=debug`.

use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use log::{LevelFilter, Log, Metadata, Record};

/// Максимальный размер журнала до ротации.
const MAX_SIZE: u64 = 2 * 1024 * 1024;
/// Сколько поколений журнала хранить: `auris.log.1` … `auris.log.N`.
const GENERATIONS: u32 = 3;

/// Состояние файла журнала (за мьютексом).
struct LogSink {
    file: Option<File>,
    /// Байт записано в текущий файл (для ротации на ходу).
    written: u64,
}

struct FileLogger {
    level: LevelFilter,
    path: PathBuf,
    sink: Mutex<LogSink>,
}

impl FileLogger {
    /// Пишет строку в файл; при ошибке файл будет переоткрыт позже.
    fn write_line(&self, line: &str) {
        let mut sink = self.sink.lock().expect("log mutex");
        if sink.file.is_none() {
            *sink = LogSink {
                file: open_log_file(&self.path),
                written: 0,
            };
        }
        let ok = sink
            .file
            .as_mut()
            .is_some_and(|f| f.write_all(line.as_bytes()).is_ok());
        if !ok {
            sink.file = None;
            return;
        }
        sink.written += line.len() as u64;
        if sink.written >= MAX_SIZE {
            // Ротация на ходу: закрываем текущий, сдвигаем поколения,
            // начинаем заново.
            sink.file = None;
            rotate(&self.path);
            *sink = LogSink {
                file: open_log_file(&self.path),
                written: 0,
            };
        }
    }
}

impl Log for FileLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= self.level
    }

    fn log(&self, record: &Record) {
        if !self.enabled(record.metadata()) {
            return;
        }
        let ts = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
        let line = format!(
            "{ts} {:<5} [{:<22}] {}\n",
            record.level().as_str(),
            record.target(),
            record.args()
        );
        // Эхо в консоль (в GUI-сборке Windows безопасно молчит).
        eprint!("{line}");
        self.write_line(&line);
    }

    fn flush(&self) {
        if let Some(f) = self.sink.lock().expect("log mutex").file.as_mut() {
            let _ = f.flush();
        }
    }
}

/// Открывает (или создаёт) файл журнала.
fn open_log_file(path: &Path) -> Option<File> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).ok()?;
    }
    OpenOptions::new().create(true).append(true).open(path).ok()
}

/// Путь поколения журнала: `auris.log.1`, `auris.log.2`, …
fn generation_path(path: &Path, n: u32) -> PathBuf {
    PathBuf::from(format!("{}.{}", path.display(), n))
}

/// Сдвигает поколения: `auris.log` → `.1` → … → удаление старейшего.
///
/// `remove_file` перед `rename` нужен для Windows (там rename
/// не выполняется поверх существующего файла).
fn rotate(path: &Path) {
    for i in (1..GENERATIONS).rev() {
        let from = generation_path(path, i);
        let to = generation_path(path, i + 1);
        if from.exists() {
            let _ = fs::remove_file(&to);
            let _ = fs::rename(&from, &to);
        }
    }
    let _ = fs::remove_file(generation_path(path, 1));
    let _ = fs::rename(path, generation_path(path, 1));
}

/// Ротация при старте, если файл вырос сверх лимита.
fn rotate_if_large(path: &Path) {
    let oversized = fs::metadata(path)
        .map(|m| m.len() >= MAX_SIZE)
        .unwrap_or(false);
    if oversized {
        rotate(path);
    }
}

/// Инициализирует журналирование.
///
/// Ошибки не фатальны: если файл недоступен, пишем только в консоль.
pub fn init() {
    let level = std::env::var("AURIS_LOG")
        .ok()
        .and_then(|v| v.parse::<LevelFilter>().ok())
        .unwrap_or(LevelFilter::Info);

    let dir = auris_core::paths::logs_dir();
    let path = dir.join("auris.log");

    rotate_if_large(&path);
    let file = open_log_file(&path);
    if file.is_none() {
        eprintln!(
            "auris: журнал недоступен — пишу только в консоль ({})",
            path.display()
        );
    }

    let path_display = path.display().to_string();
    let logger = FileLogger {
        level,
        path,
        sink: Mutex::new(LogSink { file, written: 0 }),
    };
    if log::set_boxed_logger(Box::new(logger)).is_ok() {
        log::set_max_level(level);
    }

    log::info!("Auris {} запускается", env!("CARGO_PKG_VERSION"));
    log::info!(
        "папка данных: {} (портативный режим: {})",
        auris_core::paths::root().display(),
        if auris_core::paths::is_portable() {
            "да, всё рядом с программой"
        } else {
            "нет, каталог установки не доступен для записи"
        }
    );
    log::info!("журнал: {path_display}");
    log::info!("подробная диагностика сети: задайте AURIS_LOG=debug");
}
