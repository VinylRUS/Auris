//! Автообновление: проверка по кнопке и на старте, скачивание установщика.
//!
//! Логика манифеста и загрузки — в `auris_core::update`; здесь только
//! связка с UI (карточка «Обновление» в настройках) и запуск установщика.

use std::sync::atomic::Ordering;
use std::sync::Arc;

#[cfg(target_os = "windows")]
use slint::ComponentHandle;

use crate::app::App;
use crate::MainWindow;

/// Тихая проверка обновлений на старте (не показывает ошибки сети).
pub fn startup_check(app: &Arc<App>) {
    let app = Arc::clone(app);
    app.spawn(|app| async move {
        tokio::time::sleep(std::time::Duration::from_secs(4)).await;
        check(&app, false).await;
    });
}

/// Проверяет обновления и обновляет карточку в настройках.
///
/// `manual` — проверка по кнопке (показываем «проверяем…» и ошибки).
pub async fn check(app: &Arc<App>, manual: bool) {
    if manual {
        app.ui_inv(|ui| ui.set_update_state(1));
    }
    let http = crate::default_http_client();
    match auris_core::update::fetch_manifest(&http).await {
        Ok(manifest) => {
            let newer = auris_core::update::is_newer(&manifest.version, env!("CARGO_PKG_VERSION"));
            if newer {
                log::info!(
                    "доступна версия {} (текущая {})",
                    manifest.version,
                    env!("CARGO_PKG_VERSION")
                );
                let version = manifest.version.clone();
                let notes = manifest.notes.clone();
                app.ui_inv(move |ui| {
                    ui.set_update_version(version.into());
                    ui.set_update_notes(notes.into());
                    ui.set_update_state(3);
                });
            } else {
                if manual {
                    log::info!("обновлений нет (сервер: {})", manifest.version);
                }
                app.ui_inv(|ui| ui.set_update_state(2));
            }
        }
        Err(e) => {
            log::warn!("update check: {e:#}");
            if manual {
                let msg = format!("Не удалось проверить обновления: {e:#}");
                app.ui_inv(move |ui| {
                    ui.set_update_error(msg.into());
                    ui.set_update_state(5);
                });
            }
        }
    }
}

/// Скачивает и запускает установщик (Windows) либо открывает страницу релиза.
#[allow(unused_variables)] // app не нужен на платформах без установщика
pub fn install(app: &Arc<App>) {
    // На платформах без установщика — сразу страница релиза.
    #[cfg(not(target_os = "windows"))]
    {
        let _ = open::that(auris_core::update::RELEASES_URL);
        return;
    }

    #[cfg(target_os = "windows")]
    install_windows(app)
}

/// Windows: манифест → скачивание → запуск установщика.
#[cfg(target_os = "windows")]
fn install_windows(app: &Arc<App>) {
    app.update_cancel.store(false, Ordering::Relaxed);
    let app = Arc::clone(app);
    app.spawn(|app| async move {
        let http = crate::default_http_client();
        let manifest = match auris_core::update::fetch_manifest(&http).await {
            Ok(m) => m,
            Err(e) => {
                let msg = format!("Не удалось получить сведения об обновлении: {e:#}");
                app.ui_inv(move |ui| {
                    ui.set_update_error(msg.into());
                    ui.set_update_state(5);
                });
                return;
            }
        };
        let asset = match manifest.asset() {
            Some(a) => a.clone(),
            None => {
                // Для платформы нет артефакта — открываем страницу релиза.
                let _ = open::that(auris_core::update::RELEASES_URL);
                return;
            }
        };
        let file_name = asset
            .url
            .rsplit('/')
            .next()
            .unwrap_or("auris-update.bin")
            .to_string();

        app.ui_inv(|ui| {
            ui.set_update_state(4);
            ui.set_update_progress(0.0);
        });

        let cancel = Arc::clone(&app.update_cancel);
        let ui_app = Arc::clone(&app);
        let result = auris_core::update::download_asset(
            &http,
            &asset,
            &file_name,
            move |frac| {
                let app = Arc::clone(&ui_app);
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(ui) = app.ui.upgrade() {
                        ui.set_update_progress(frac as f32);
                    }
                });
            },
            move || cancel.load(Ordering::Relaxed),
        )
        .await;

        match result {
            Ok(Some(path)) => {
                #[cfg(target_os = "windows")]
                {
                    match std::process::Command::new(&path).spawn() {
                        Ok(_) => {
                            log::info!("установщик запущен: {}", path.display());
                            // Закрываем окно: run() вернёт управление,
                            // main сохранит настройки и выйдет.
                            app.ui_inv(|ui| {
                                let _ = ui.hide();
                            });
                        }
                        Err(e) => {
                            let msg = format!("Не удалось запустить установщик: {e}");
                            app.ui_inv(move |ui| {
                                ui.set_update_error(msg.into());
                                ui.set_update_state(5);
                            });
                        }
                    }
                }
                #[cfg(not(target_os = "windows"))]
                {
                    // На Unix обновление ставится из пакета — открываем релиз.
                    let _ = path;
                    let _ = open::that(auris_core::update::RELEASES_URL);
                }
            }
            Ok(None) => {
                // Отменено пользователем — возвращаемся к «доступно».
                app.ui_inv(|ui| ui.set_update_state(3));
            }
            Err(e) => {
                let msg = format!("Ошибка скачивания: {e:#}");
                app.ui_inv(move |ui| {
                    ui.set_update_error(msg.into());
                    ui.set_update_state(5);
                });
            }
        }
    });
}

/// Отмена скачивания.
pub fn cancel(app: &Arc<App>) {
    app.update_cancel.store(true, Ordering::Relaxed);
}

/// Регистрирует колбэки UI обновления.
pub fn init(app: &Arc<App>, ui: &MainWindow) {
    ui.set_update_installer(cfg!(target_os = "windows"));
    {
        let app = Arc::clone(app);
        ui.on_update_check(move || {
            let app = Arc::clone(&app);
            app.spawn(|app| async move {
                check(&app, true).await;
            });
        });
    }
    {
        let app = Arc::clone(app);
        ui.on_update_install(move || install(&app));
    }
    {
        let app = Arc::clone(app);
        ui.on_update_cancel(move || cancel(&app));
    }
}
