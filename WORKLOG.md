# WORKLOG — журнал работы над Auris

Живой документ для сохранения контекста между сессиями разработки.
Каждая сессия дописывает раздел сверху вниз (новые — выше старых).

---

## Сессия 3 — 2026-10-27 (продолжение): фикс Windows-сборки, worklog в репо

**Задача:** CI на main упал на Windows; перенести worklog в репозиторий;
выпустить релиз v0.0.1b.

- **Причина падения main-CI:** `app.ui_inv(|ui| ui.hide())` — в Slint 1.18
  `ComponentHandle::hide()` возвращает `Result<(), PlatformError>`, а замыкание
  `ui_inv` должно возвращать `()`. Локально на Linux не ловилось: код под
  `#[cfg(target_os = "windows")]`.
- **Фикс:** `app.ui_inv(|ui| { let _ = ui.hide(); });` — main зелёный на 3 ОС.
- **Вывод на будущее:** Windows-only ветки (`cfg!`) не проверяются локальной
  сборкой на Linux — при правках cross-platform кода проверять типы вручную.

### Релизный пайплайн v0.0.1b (тег): первый прогон упал на macOS

- **Диагноз:** шаг Package делал `cd target/release && tar czf ../../../…` —
  из `target/release` три уровня вверх = **выше корня workspace**
  (`/Users/runner/work/Auris`, а workspace — `…/Auris/Auris`). Архив
  создавался вне workspace, upload-artifact (`if-no-files-found: error`)
  его не находил → failure на macOS/Ubuntu.
- **Это старый баг из v0.0.1a:** прежний workflow крепил файлы к релизу
  мягким glob’ом — пустое совпадение молча пропускалось, поэтому релиз
  v0.0.1a «успешен», но без ассетов.
- **Windows прошёл** (баг замаскирован: `Auris-Setup-*.exe` лежит в корне
  workspace — upload нашёл его) — **Inno Setup установщик собирается**, ✅.
- **Фикс:** Unix: `tar czf … -C target/release auris` (без cd);
  Windows: `7z a ..\..\…` (два уровня, не три).
- **Ре-тег:** тег удалён и пересоздан на фиксовом коммите
  (workflow читается из коммита тега — иначе фиксы не подхватятся).

---

## Сессия 2 — 2026-10-27: 0.0.1b — установщик, автообновление, живой API

**Задача:** сборка exe на GitHub, релиз с установщиком и автообновлением,
живой API. Версия 0.0.1b.

### Сделано (3 коммита, запушены в main)

1. `feat(update)` — 2a84e92:
   - `auris-core/src/update.rs`: манифест `latest.json` (GitHub Releases),
     semver-сравнение (форматы `0.0.1-beta.1` и `0.0.1b`, порядок
     alpha < beta < rc), скачивание с прогрессом/отменой, SHA-256; юнит-тесты.
   - `auris/src/update.rs`: тихая проверка на старте (+4 с), карточка
     «Обновление» в настройках (статусы 0–5, прогресс-бар, отмена, ошибки);
     Windows: скачать → верифицировать → запустить установщик → закрыть окно;
     Unix: открыть страницу релиза.
   - Иконка `assets/icon.ico|png` (Fluent, эквалайзер; генератор — PIL-скрипт).
   - `build.rs`: winresource — иконка + FileVersion/ProductVersion в exe.
   - Версия `0.0.1-beta.1`, отображение `0.0.1b` (`app::display_version`).
2. `ci(release)` — 2659622:
   - `installer/auris.iss` (Inno Setup 6): per-user `{localappdata}\Auris`,
     без UAC, `CloseApplications=force`, RU/EN, деинсталлятор.
   - `installer/make_manifest.py`: генерация `latest.json` (версия из
     Cargo.toml, SHA-256 артефактов).
   - CI: `build` → `release` (Windows собирает ISCC) → `publish`
     (SHA256SUMS.txt, latest.json, тело релиза = аннотация тега).
3. `docs` — 824d0ee: README (раздел «Установка», статус 0.0.1b), CHANGELOG.

### Проверки

- Живой API: `POST oauth.yandex.ru/device/code` — 200, креды верны.
- Локально (Linux): `cargo fmt` ок; `cargo check --workspace --all-targets` —
  0 предупреждений; `cargo test -p auris-api -p auris-core` — 15 passed.

### Окружение сессии (важно для будущих)

- Rust 1.99.1 через rustup (`~/.cargo/bin`); в контейнере **нет sudo** —
  dev-библиотеки развёрнуты в sysroot `/home/z/my-project/sysroot/root`
  (`apt-get download` + `dpkg -x`), env: `/home/z/my-project/scripts/env.sh`
  (PKG_CONFIG_PATH/CPATH/LIBRARY_PATH).
- Токен GitHub и локальный worklog — вне репо (в `/home/z/my-project/`).

### Не доделано (перенос на следующую сессию)

- Аннотированный тег `v0.0.1b` + пуш → CI соберёт релиз.
- Верификация ассетов: `Auris-Setup-0.0.1b-x64.exe`, `latest.json`,
  `SHA256SUMS.txt`; проверка канала
  `releases/latest/download/latest.json`.
- ⚠️ Релиз **не** помечать pre-release — иначе `releases/latest` его не
  отдаст и автообновление его не увидит.

---

## Сессия 1 — 2026-10-27 (ранее): 0.0.1a — каркас

**Задача:** весь каркас приложения с нуля (см. CHANGELOG 0.0.1a).

- Workspace: `auris-api` (клиент API, Device Flow, golden-тесты),
  `auris-player` (rodio+symphonia), `auris-core` (конфиг/темы/keyring/кеш/
  загрузки/история), `auris` (Slint UI по Fluent-мокапам).
- CI на 3 ОС; тег `v0.0.1a` (pre-release, ассетов нет).

---

## Сводка проекта

| Что | Где |
|---|---|
| Репозиторий | https://github.com/VinylRUS/Auris |
| Стек | Rust, Slint 1.18, rodio+symphonia, reqwest(rustls), tokio |
| Правовая позиция | Без обхода DRM; ссылки только из официального download-info; OAuth Device Flow с публичными кредами Android-приложения |
| Канал обновлений | `releases/latest/download/latest.json`; схема — `auris-core::update::UpdateManifest` |
| Ключевые файлы | `crates/auris-core/src/update.rs`, `crates/auris/src/update.rs`, `installer/auris.iss`, `installer/make_manifest.py`, `.github/workflows/ci.yml` |
| Тег → версия | тег `v0.0.1b` ↔ Cargo `0.0.1-beta.1` ↔ отображение `0.0.1b` |
