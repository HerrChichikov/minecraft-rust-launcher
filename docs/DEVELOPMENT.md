# Разработка

Реализованы foundation и Phase 2 Storage & Profiles: Rust workspace, Tauri 2 adapter, React/TypeScript/SCSS/Vite, базовые contracts, restricted IPC, тесты и CI. Это ещё не установщик и не готовый Minecraft launcher.

## Окружение

Pinned toolchain: Rust 1.99.1 (`rust-toolchain.toml`, edition 2024), Node 26.13.0 (`.node-version`), bundled npm 11.20.0. Минимальный Rust в workspace — 1.99. Frontend требует Node 26.x. TypeScript 6.0.3 выбран из совместимого peer range typescript-eslint; подробности в ADR-007.

[Prerequisites Tauri](https://v2.tauri.app/start/prerequisites/): macOS Command Line Tools; Windows MSVC C++ tools/WebView2; Linux GTK/WebKitGTK 4.1 build dependencies. Текущая macOS foundation deployment baseline — 13.0. Linux packages собираются на Ubuntu 22.04 baseline; заявленная совместимость ОС для полного продукта требует Phase 7 smoke.

На машине этой задачи Node/Rust подготовлены в `.tools`, caches в `.cache`; shell startup files и системный PATH не менялись. `scripts/with-local-tools.sh` передаёт локальные PATH/CARGO_HOME/RUSTUP_HOME только дочерней команде. На других машинах можно использовать обычную установленную среду; содержимое `.tools` в Git не отправляется.

## Команды

Из корня repository:

```sh
npm ci --ignore-scripts
npm run dev
```

В текущем workspace при отсутствии системного npm/cargo:

```sh
scripts/with-local-tools.sh npm ci --ignore-scripts
scripts/with-local-tools.sh npm run dev
```

`npm run dev` запускает Tauri и Vite hook. `npm run dev:web` — только frontend server 127.0.0.1:1420; в браузере корректно отображается desktop-required, без fake backend. Порт strict, remote hosts не допускаются.

```sh
npm run check
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo test -p launcher-core --features dev-auth --locked
python3 scripts/check-release-guard.py
cargo test -p launcher-desktop --release --locked --lib --features tauri/custom-protocol
```

`npm run check`: Prettier, ESLint с type-aware rules, strict typecheck, Vitest, production frontend build. `.prettierignore` исключает исторические docs и raw research JSON; downloaded bytes нельзя форматировать из-за опубликованных hashes. Rustfmt проверяется отдельно. Prettier не заменяет Rustfmt.

`--ignore-scripts` используется при clean install. Native npm dependencies доступны как prebuilt packages; ненужные dependency install scripts не разрешаются автоматически. Actual graph проверяется через `npm ls`, `npm audit`; нельзя использовать audit fix --force или игнорировать peer conflicts. Открытый security override документируется в ADR-007.

## Desktop packaging

```sh
npm run tauri -- build --ci -- --locked
```

На macOS можно отдельно выбрать `--bundles app` или `--bundles dmg`. Для Intel cross-build сначала `rustup target add x86_64-apple-darwin`, затем `npm run tauri -- build --ci --target x86_64-apple-darwin --bundles app -- --locked`. Это не запускает Intel binary и не подтверждает native Intel smoke. Готовый уже собранный app можно упаковать `npm run tauri -- bundle --ci --bundles dmg`.

App name `Launcher` и bundle ID `dev.launcher.desktop` — рабочие значения foundation, их нужно согласовать до выпуска. macOS использует ad-hoc identity `-`: это не Developer ID и не notarization. Не запускаем production updater и не публикуем release автоматически.

На Windows `build.rs` передаёт Common Controls v6 manifest через MSVC linker всем executable targets, включая unit-test harness. Стандартное Tauri embedding охватывает только application binary; без этого Windows cargo test может завершиться STATUS_ENTRYPOINT_NOT_FOUND до запуска тестов. Manifest не запрашивает administrator privileges. См. [upstream issue](https://github.com/tauri-apps/tauri/issues/13419).

## Реальные границы

`ApplicationService::snapshot()` читает build/platform и возвращает feature availability. `src-tauri` превращает её в DTO schema v1. Main window имеет явные разрешения snapshot/storage/installation/auth; другие окна и remote origin отклоняются в actual IPC tests. Play и произвольные fs/shell/http APIs у frontend отсутствуют; сетевой I/O выполняет backend.

`dev-auth` включает MockAuthProvider только с debug assertions; используйте `npm run dev:mock`. В release его запрещают и core cfg guard, и desktop build profile guard. SecretString, session и LaunchIdentity не сериализуются; Debug скрывает secrets. Production registry содержит только Ely.by.

Storage/profiles, installer/runtime и authentication реализованы на этапах 2–4. Launch Engine остаётся следующим этапом; agent preparation не запускает Minecraft. Статус и ограничения каждого этапа зафиксированы в review.

## Отчёты

[Phase 1 review](PHASE_1_REVIEW.md) содержит фактические проверки и платформенные ограничения. `.github/workflows/ci.yml` даёт native runner matrix; без успешного run нельзя считать Windows/Linux подтверждёнными. Исследовательская проверка `python3 docs/research/verify_snapshot.py` остаётся offline и независимой от application tests.

## Профили и настройки Phase 2

Рабочая UI-панель появляется после подключения desktop bootstrap. Первый `get_workspace` открывает OS app config, создаёт marker/root layout/default profile и удерживает OS locks до завершения процесса. Для Tauri tests используются только injected TempDir paths — системные app data не затрагиваются.

Штатный data root — `app_local_data_dir()/data`, config — `app_config_dir()/launcher.json`. Поле нового root требует абсолютного пути. Переключение не переносит файлы; можно вернуться в прежний root и восстановить его selection. Не редактируйте config параллельно работающему лаунчеру. Если JSON повреждён или schema новее, лаунчер сохранит исходный файл и покажет ошибку; автоматического reset нет.

В UI RAM в МиБ; current physical RAM — верхняя граница сохранения. Managed Java пока только выбор будущего runtime, без установки. Custom Java проверяется как файл; major/architecture probe и запуск не относятся к Phase 2. Поддерживаемые JVM options указаны рядом с полем; агент, classpath, @argfile и собственные RAM flags не принимаются.

Локальный smoke Phase 2 создал в app data дополнительную конфигурацию с max RAM=3072 МиБ. Это настоящий persisted profile, без игры/Java/пользовательских миров. Детальные результаты и SHA256 DMG — в validation/phase-2-local.json. GUI smoke на Windows/Linux/Intel не выполнялся на этой машине; native CI tests/builds учитываются отдельно.

## Phase 3 — установка и smoke

В desktop доступна отдельная панель установки: кнопка читает активный backend profile, запускает start_install и показывает events/snapshot. Cancel адресуется operation ID; остановка stalled request может занять до 30 секунд. При ошибке повтор использует проверенные файлы. Выход из процесса оставляет только unpublished staging/parts; следующая попытка не считает их установленными. Автоматический cleanup/GC и metadata offline fallback пока отсутствуют.

Opt-in CLI для разработчика (скачивает около 1–2 ГБ и выполняет официальный Java probe, игру не запускает): `cargo run -p launcher-core --example install_smoke --locked -- /absolute/isolated-smoke-root`. Корень должен быть отдельным тестовым каталогом. Повторный лог можно проверить scripts/check-install-reuse.py; native spike — scripts/check-native-smoke.py с тем же root. Для Windows используйте native absolute path; XML/JSON/paths сохраняются backend, shell команд из UI нет.

Linux build prerequisites в CI загружаются с official archive.ubuntu.com HTTPS без recommended extras. CARGO_BUILD_TARGET одинаков для всех Rust checks/builds. System Java не требуется: native smoke использует downloaded Mojang runtime. Installer UI smoke packaged Release на macOS ARM64 пройден: start, cancel и retry до Ready. Исходные data root/profile и настройки восстановлены; receipt — validation/phase-3-gui.json. Это не game/GPU/login smoke.

## Phase 4 — development auth

`npm run dev:mock` (`tauri dev --features dev-auth`) включает backend mock. Обычный dev и release его не предлагают. Для packaged debug UI: `npm run tauri -- build --debug --features dev-auth --bundles app -- --locked`. Ник 3–16 ASCII letters/digits/underscore; UI постоянно помечает тестовую сессию. После перезапуска процесса нужен повторный вход. Токен не отображается и не сохраняется.

`cargo test --workspace --features dev-auth --locked` проверяет также desktop IPC с mock. Release IPC suite выполняется без feature, compile-fail guards проверяют оба crates. Opt-in `cargo run -p launcher-core --example agent_smoke --locked -- /absolute/isolated-agent-smoke` скачивает pinned official agent, проверяет порчу/repair/reuse и не исполняет JAR. Используйте отдельный тестовый root.

Ely.by доступен в обычных debug/release сборках: login/password, отдельный TOTP challenge, обновление/выход. Credentials вводите только в desktop форме; не передавайте их через CLI arguments/chat/fixtures. По умолчанию сессия memory-only; флажок сохраняет токены только в OS store. Неверный код можно повторить до истечения challenge; cancel возвращает форму пароля. Для повторного upstream smoke необходим собственный test account, его успешный login не подменяется fixture tests.

Сохранённая сессия восстанавливается отдельной кнопкой (validate+refresh), без чтения keychain при обычном get_auth. Удалить saved session можно без входа отдельной кнопкой. `cargo run -p launcher-desktop --example credential_smoke --locked` создаёт новый уникальный record в test-only namespace, читает и удаляет его; не используйте реальные токены в smoke. Store unavailable — явный memory-only outcome, без файлового fallback.
