# Разработка — Phase 1

Реализован foundation: Rust workspace, Tauri 2 adapter, React/TypeScript/SCSS/Vite, базовые contracts, restricted IPC, тесты и CI. Это ещё не установщик и не готовый Minecraft launcher.

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

## Реальные границы

`ApplicationService::snapshot()` читает build/platform и возвращает feature availability. `src-tauri` превращает её в DTO schema v1; единственное разрешение main window — allow-get-snapshot. Другие окна/remote origin и login/play/fs/shell команды отклоняются в tests. Серверных запросов и произвольных filesystem APIs у frontend нет.

`dev-auth` зарезервирован для Phase 4, без MockAuthProvider. В release запрещается и core cfg guard, и desktop build profile guard. Не объясняем feature как готовый mock login. SecretString и LaunchSpec не сериализуются, Debug скрывает токены/argv.

Контракты остальных модулей — API scaffolding с documentation, без TODO/unimplemented bodies и без успешных заглушек. Перед Phase 2 уточняются schema/path invariants; перед Phase 3 — cancellation/resource verification contracts. Код следующего этапа не объявляется работающим заранее.

## Отчёты

[Phase 1 review](PHASE_1_REVIEW.md) содержит фактические проверки и платформенные ограничения. `.github/workflows/ci.yml` даёт native runner matrix; без успешного run нельзя считать Windows/Linux подтверждёнными. Исследовательская проверка `python3 docs/research/verify_snapshot.py` остаётся offline и независимой от application tests.
