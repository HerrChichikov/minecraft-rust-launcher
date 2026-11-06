# Зависимости

Статус: в Phase 1 созданы Cargo/npm manifests и **Cargo.lock + package-lock.json**. Toolchain: Rust 1.99.1, Node 26.13.0, bundled npm 11.20.0. Прямые npm dependencies закреплены exact. Ниже исходный план полного MVP; dependencies следующих этапов не добавлены заранее.

## Реальный набор Phase 1

- Rust core: serde 1.0.229, thiserror 2.0.21, uuid 1.29.0, zeroize 1.9.1; serde_json 1.0.154 для tests. Desktop: tauri 2.12.5, tauri-build 2.7.4 и те же shared declarations.
- Frontend: React/React DOM 19.3.1, TypeScript 6.0.3, Vite 8.3.6, React plugin 6.2.1, Sass 1.108.0, Tauri API/CLI 2.12.4.
- Checks: ESLint 10.14.0, typescript-eslint 8.72.1, Prettier 3.9.10, Vitest 5.1.0, Testing Library React 16.3.3 / user-event 14.6.8 / jest-dom 7.0.1, jsdom 31.0.0. Остальные exact versions — в manifests/lock.
- TypeScript 7 не выбран из-за официального peer range lint tooling. Scoped compatibility decision и minimatch security override описаны в [ADR-007](DECISIONS/007-foundation-tooling.md).
- reqwest, Tokio как direct dependency, tokio-util, SHA libraries, MD5, keyring, HTTP test server не установлены как direct dependencies: работающих сетевых/runtime/auth workflows ещё нет. Tokio и часть crypto присутствуют транзитивно у Tauri.

Локальный frontend build, core/desktop tests и release ARM64 build проверены. Остальные target results указаны отдельно в [отчёте](PHASE_1_REVIEW.md); lock-файл сам по себе не доказывает cross-platform поддержку.

## Runtime и desktop

| Зависимость | Назначение / почему нужна |
|---|---|
| `tauri`, `tauri-build`, `@tauri-apps/api`, `@tauri-apps/cli`, все 2.x | Desktop/IPC/build; versions проверяем как совместимый набор, не приравниваем patch numbers |
| `tokio`, `tokio-util` | Async HTTP/files/process, Semaphore, CancellationToken; только необходимые features |
| `reqwest` с rustls | Один HTTP client, streaming download, redirects/timeouts; не frontend HTTP plugin |
| `serde`, `serde_json` | Metadata, settings, DTO |
| `thiserror` | Типизированные ошибки core |
| `sha1`, `sha2` | Mojang integrity и agent SHA256 |
| `uuid` с v4 | Profile/run/account IDs; generation только backend |
| `md-5` | Только development-auth, точная совместимость Java offline UUID; в release feature не нужна |
| `regex` | Metadata OS arch/version predicates, bounded engine |
| `tracing`, `tracing-subscriber` | Structured log levels и controlled sinks; без secret Debug |
| `zeroize` | Минимизация времени жизни secret bytes |
| `keyring` v4 с `v1`, desktop adapter | Простые get/set/delete в системном store; без тяжёлого `cli` feature и без mock/sample fallback |
| OS-specific process/locking adapter dependency по необходимости | Сначала std/platform APIs; crate добавляем только после выбранного API и лицензии |

JSON index вместо SQLite, raw runtime вместо LZMA, native JAR loader вместо собственного универсального archive installer уменьшают граф. ZIP crate не нужен для текущего варианта 1.21.1; появится при поддержке legacy native formats. `async-trait`, OAuth SDK, updater plugin, shell/fs/http frontend plugins, DI container, plugin framework и Java dependency resolver сейчас не нужны.

У keyring v4 API отличается от старых примеров: [документация](https://docs.rs/keyring/latest/keyring/) разделяет v1/cli и рекомендует keyring-core для более сложного управления stores. Для наших простых credentials операций v1 — исходный кандидат; если platform tests требуют точного выбора stores, используем `keyring-core` + нужные platform stores за тем же adapter, с отдельным ADR. Не копируем v3 usage без проверки.

## Frontend и проверки

| Зависимости | Назначение |
|---|---|
| `react`, `react-dom`, TypeScript, соответствующие `@types` | Три экрана, strict typing |
| `vite`, `@vitejs/plugin-react`, `sass` | Build/dev/SCSS, статические bundled assets |
| ESLint, typescript-eslint, React/hooks plugins | Linting и effects hygiene |
| Prettier | Формат TS/SCSS/docs |
| Vitest, Testing Library React/user-event, DOM environment | UI states, validation, subscription races |
| Rust dev: `tempfile`, HTTP test server (`wiremock` или equivalent) | Изолированные disks, network fault fixtures; выбор одного HTTP test tool |

Для трёх экранов не нужны Redux, routing library, UI framework, animation library и react-query; React reducer/context + тонкий IPC client достаточно. При росте пересматриваем по измеримой потребности.

## Проверенные ориентиры на 2026-11-06

- [Tauri latest release](https://github.com/tauri-apps/tauri/releases/tag/tauri-cli-v2.12.4) — **CLI 2.12.4**; это не номер всех Tauri crates.
- [React versions](https://react.dev/versions) — **19.3**; кандидат для frontend.
- [Vite releases](https://vite.dev/releases) — текущая поддерживаемая ветка **8.3**. [Vite guide](https://vite.dev/guide/) требует Node 20.19+ либо 22.12+; для проекта выбираем поддерживаемый LTS Node и фиксируем точную версию в Phase 1.
- [Tokio docs](https://docs.rs/tokio/latest/tokio/process/index.html) — 1.54.1; [keyring docs](https://docs.rs/keyring/latest/keyring/) — 4.2.0. Это наблюдения источников, не уже разрешённый dependency graph.

Непроверенные точные patch numbers других пакетов намеренно не выдаются за актуальные. Перед merge Phase 1: `cargo metadata`/`npm ls`, clean locked install, четыре target builds, license inventory и audit report. Не игнорируем peer-dependency errors через force.
