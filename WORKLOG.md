# WORKLOG — журнал работы над Auris

Живой документ для сохранения контекста между сессиями разработки.
Каждая сессия дописывает раздел сверху вниз (новые — выше старых).

---

## Сессия 6 — 2026-10-28: 0.0.1c — «подключаем всё, что можно подключить»

**Задача:** пользователь: «Делаем рабочую версию 0.0.1c (предрелизная).
Подключаем всё что можно подключить. Настройки и профиль залезли под панель
воспроизведения.» Каждый юнит — отдельная ветка → коммит → пуш → merge в main.

### Сделано (7 мержей в main, CI зелёный на всех)

1. **fix(ui): сайдбар под плеером** — «Настройки» и карточка профиля
   перекрывались плавающим PlayerBar (сайдбар был height:100%).
   Теперь `parent.height - 104px`, как в Яндекс.Музыке.
2. **feat(catalog): альбомы и исполнители** — плитки альбомов (новинки,
   лучший результат) открывают `albums/{id}/with-tracks` (тома → список);
   `artists/{id}/brief-info` → популярные треки; библиотека получила секции
   «Альбомы»/«Исполнители» из `users/likes/albums` + `likes/artists`
   (обложки 300x300 кешируются); TileItem: 3=альбом, 4=исполнитель.
3. **feat(lyrics): панель текста песни** — LyricsPanel справа (взаимоисключаема
   с очередью), кнопка-пузырь в плеере, `tracks/{id}/supplement` с кешем по
   треку (включая отрицательный), автообновление при смене трека.
   Грабли: Slint тогглил `lyrics-open`, потом Rust тогглил обратно → панель
   не открывалась; фикс — Rust единственный владелец состояния. Вторые
   грабли: pages-host не учитывал 380px панели → страница просвечивала
   под полупрозрачным фоном.
4. **feat(radio): станции дашборда** — `rotor/stations/dashboard` → секция
   «СТАНЦИИ» (дедуп по сиду), фолбэк на жанры/настроение при ошибке.
5. **feat(search): подсказки** — `search/suggest` с дебаунсом 350 мс
   (AtomicU64 поколений), попап до 8 строк, клик подставляет и ищет.
   Грабли: у TextInput в Slint 1.18 `edited` БЕЗ аргументов.
6. **Версия** — `0.0.1-beta.2` → отображается `0.0.1c` (display_version
   переписан: alpha.N→aN, beta.1→b, beta.2→c…). Апдейтер парсит полный
   semver, старые сборки увидят обновление.
7. **Верификация** — cargo check 0 варнингов; тесты api/core/player —
   22 passed; Xvfb-скриншоты всех страниц + VLM-аудит (сайдбар выше плеера,
   лирика не просвечивает, станции/секции на месте); test_nav_resize —
   1280×800 на всех 13 шагах. Диск забивался target/ (2.7 ГБ дублей) →
   SIGBUS линкера, почищено.

### Скриншоты
`/home/z/my-project/download/ui-{home,library,radio,settings,search,lyrics}.png`
(лирика — через `AURIS_UI_DEMO_LYRICS=1`).

---

## Сессия 5 — 2026-10-27 (поздняя): вход, геометрия окна, растяжка сеток, аудит API

**Задача:** пользователь: «1. Не работает вход + на логине нет верхней панели.
2. UI кривой, окно сжимается при нажатии кнопок. Вдохновись Spotify/Я.Музыкой
(не копируя). Сначала — красивое окно логина. 3. Проверить весь API
(MarshalX/yandex-music-api).»

### Найденные и исправленные корневые причины (5 коммитов, все в main)

1. **Вход не работал — два бага** (`fix(api)`):
   - `auth.rs`: структуры OAuth ждали camelCase (`deviceCode`), а
     oauth.yandex.ru отвечает snake_case (`device_code`) — падение на первом
     шаге. Убран `rename_all`, фикстуры переписаны с живого ответа.
   - `main.rs`: без токена клиент создавался `with_token("")` →
     `is_authorized()` = true → приложение ПРОПУСКАЛО экран входа и работало
     гостевым анонимом (API отвечает /account/status без авторизации).
     Теперь пустой токен = не авторизован.
   - Проверено: креды Device Flow верные (сравнены с 4 живыми реализациями);
     «Wrong client secret» из контейнера — антибот-блок IP датацентра.
2. **Окно 800×600 на логине и сжатие до 1205px при навигации** (`fix(ui)`):
   - Логин: пустой корневой layout → preferred = 0 → winit-дефолт 800×600.
     Фикс: явный `set_size` из Rust (восстановление из конфига 1280×800).
   - Сжатие: страницы в HorizontalLayout участвовали в констрейнтах окна
     (страница настроек сжимала окно). Фикс: контент на positional-размещении.
   - Титлбар теперь ВСЕГДА виден (закрыть/свернуть на логине), min 960×600.
3. **Новый экран логина** (`feat(ui)`): бренд-панель (логотип, фичи,
   анимированная волна, свечение) + карточка входа (QR, код, «таблетка»
   ya.ru/device, спиннер, отмена, резервный вход по токену). VLM: «качественно
   и современно, уровень Spotify/официального клиента».
4. **Сетки не растягивались на ширину — корневая причина** (`feat(ui)`):
   у Flickable ширина по умолчанию = preferred контента (с for-детьми — баг
   slint#407). Фикс: паттерн из std-widgets ScrollView — обёртка Rectangle +
   явные `width/height/content-width` (8 страниц). Новый TileGrid с переносом
   строк и растяжкой; радио — карточки станций вместо чипов; очередь —
   растяжка текста; TrackRow — альбом фиксированной долей ширины (Spotify).
   VLM после правок: поиск 9/10, остальные 8/10.
5. **Аудит API** (`feat(api)` + docs/API_COVERAGE.md): добавлены
   album_with_tracks, artists_brief_info, tracks_similar, track_supplement
   (тексты), likes albums/playlists/artists, landing, rotor_dashboard.
   7 фикстур, 7 тестов (итого 22 зелёных).

### Ключевые выводы (важно для следующих сессий)

- Slint: `parent` недоступен в корневых биндингах компонента; `%` нельзя в
  border-radius; нет `else` у условных элементов; элементы внутри `if` не
  адресуемы снаружи (лечится `changed height => {}` колбэком или выносом).
  `Math.mod` вместо `%`. Свойства: `transform-rotation`, `transform-origin`.
- Дефолт `alignment` у layout = Stretch; пустой layout → preferred 0.
- Вёрстка контента внутри Flickable обязана следовать паттерну ScrollView.
- VLM-верификация скриншотов — в /home/z/my-project/scripts/vlm_*.mjs
  (z-ai-web-dev-sdk из /home/z/node_modules).

### Не доделано (перенос)

- Подключить новые API к UI: страницы альбома/исполнителя, тексты песен,
  лайки альбомов в библиотеке, станции из dashboard на странице радио.
- Плейлист-мутации (create/rename/delete) + queue-синхронизация (см.
  docs/API_COVERAGE.md).
- Релиз v0.0.1c после подключения новых фич (пайплайн уже настроен).

---

## Сессия 4 — 2026-10-27: редизайн интерфейса («внешка»)

**Задача:** пользователь: «Выглядит ужасно, надо доделывать внешку».
Диагноз: иконки были unicode-глифами (⌂ ∿ ⚙ ♥ ‖) — рендер зависел от
системного шрифта, выглядели дёшево; не было hover-состояний и анимаций;
вёрстка страниц «разъезжалась» (огромные пустоты из-за растяжения nested-Layout
внутри Flickable).

### Что сделано

- **Иконки**: 34 SVG Fluent-стиля; генератор живёт вне репо
  (`/home/z/my-project/scripts/gen_ui_icons.py`), сами файлы — в
  `crates/auris/ui/icons/*.svg` (белая краска, 24×24, stroke 1.7, round caps).
  Рендер — Slint `Image.colorize` (SVG уже поддержан из коробки: resvg в
  зависимостях, рендер в размере элемента). Глобал `Icons` в `ui/icons.slint`.
- **Выводы по Slint 1.18** (проверено по исходникам компилятора):
  - `Rectangle.color` — задепрекейченный алиас `background`, своё свойство
    иконок названо `tint`;
  - `Path.stroke-width` — в ЭКРАННЫХ пикселях, не масштабируется с viewbox;
  - дефолт `alignment` у VerticalLayout/HorizontalLayout = **Stretch** →
    если Flickable.content-height больше реального контента, nested-Layout
    растягиваются и появляются пустоты между секциями. Лечение: `alignment:
    start` внутреннему layout + честный content-height. Сделано на всех
    страницах.
  - Анимации: `iteration-count: -1; direction: alternate` — рабочий рецепт
    бесконечного «пинга-понга» (эквалайзер EqBars, «дышащие» волны Path).
- **Компоненты**: IconButton/PlayButton/ActionButton/Chip/SliderBar/TileCard/
  TrackRow/Toggle/SearchField — hover+pressed состояния, анимации 90–150 мс;
  TileCard: затемнение + круглая play-кнопка при наведении; TrackRow:
  эквалайзер/номер, действия проявляются при наведении (`interactive`
  у IconButton блокирует клики по скрытым кнопкам).
- **Титлбар**: Win11 caption-кнопки 46×40 (крест с #C42B1C), лого =
  эквалайзер в градиентном квадрате. **Сайдбар**: векторные иконки,
  карточка пользователя кликабельна (→ настройки). **Плеер-бар**: тень
  (drop-shadow), транспорт с векторными иконками, mute по клику на громкость
  (UI-state, без Rust-колбэков), убрана мёртвая кнопка «нота».
- **Демо-режим**: `AURIS_UI_DEMO=1` → окно с мок-данными (crates/auris/src/
  demo.rs) — для скриншот-тестирования UI без сети.
- **Верификация визуальная**: Xvfb :97 + `winit-software` + python-xlib
  скриншоты (генератор `/home/z/my-project/scripts/shot.py`, координаты
  навигации подобраны), проверка VLM: «уровень Spotify», дефектов нет.
  Скриншоты: `/home/z/my-project/download/ui-*.png`.

### Грабли окружения (важно для следующих сессий)

- Линковка GUI-бинарника локально: статический `libfreetype.a` из sysroot
  тянет libpng/zlib/bz2/brotli → `cargo rustc -p auris --bin auris --
  -Clink-arg=-lpng16 -Clink-arg=-lz -Clink-arg=-lbz2 -Clink-arg=-lbrotlidec
  -Clink-arg=-lbrotlicommon` (системные либы добавлены в LIBRARY_PATH в
  `scripts/env.sh`). `cargo test -p auris` локально не линкуется — тесты
  только в api/core (15 шт), GUI проверяется cargo check + CI.
- Xvfb: нужны `libxkbcommon-x11` + `libxcb-xkb` (deb → sysroot, LD_LIBRARY_PATH
  в env.sh), бэкенд `winit-software` (femtovg/GLX под Xvfb даёт чёрный экран).
- Диск: target/ распух до 7.6 ГБ → линкер ловил SIGBUS. Чистить
  `target/debug/{incremental,deps,build}` + `CARGO_INCREMENTAL=0`.

### Статус на конец сессии

- Коммит `feat(ui): полный редизайн — векторные иконки, hover-состояния,
  анимации, фикс вёрстки` запушен; main ждал CI; после зелёного — тег
  **v0.0.1b** (релизный пайплайн уже настроен сессиями 1–3).

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
