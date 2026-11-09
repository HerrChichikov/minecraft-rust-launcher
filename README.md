# Minecraft Launcher

Основа desktop-лаунчера Minecraft Java Edition на Tauri 2, Rust и React / TypeScript / SCSS / Vite.

**Phase 5: Launch Engine.** Кнопка «Играть» проверяет установку и запускает Minecraft 1.21.1 с настройками активного профиля. Launcher показывает состояние и код выхода. Реальный запуск с тестовым входом проверен на macOS ARM64. [Результаты Phase 5](docs/PHASE_5_REVIEW.md).

## Запуск разработки

Нужны Rust **1.99.1**, Node **26.13.0** / npm **11.20.0** и [системные зависимости Tauri](https://v2.tauri.app/start/prerequisites/).

```sh
npm ci --ignore-scripts
npm run dev
```

На текущей машине инструменты установлены локально в ignored `.tools`; системный PATH не изменялся. Здесь команды можно запускать через wrapper:

```sh
scripts/with-local-tools.sh npm run dev
```

`npm run dev:web` открывает только Vite server. В обычном браузере показано отсутствие desktop backend; демонстрационный Rust-ответ не подставляется.

## Проверки и сборка

```sh
npm run check
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo test --workspace --features dev-auth --locked
python3 scripts/check-release-guard.py
npm run tauri -- build --ci -- --locked
```

`npm run dev:mock` включает Development / Mock. Обычный `npm run dev` не включает его. Release с `dev-auth` отвергается compiler/build guard; production не принимает mock provider ID и не восстанавливает тестовую сессию из config.

## Структура

- `frontend/` — экран профилей и диагностики, typed IPC clients, unit tests, SCSS.
- `crates/launcher-core/` — независимое файловое хранилище, профили, validation и contracts следующих этапов.
- `src-tauri/` — `get_snapshot` и ограниченные profile/storage commands, DTO, ACL/CSP и desktop packaging.
- `tests/contracts/` — общий Rust/TypeScript wire-contract fixture.
- `.github/workflows/ci.yml` — pinned quality checks и build matrix Windows x64 / Linux x64 / macOS Intel / macOS ARM64; без публичного release.
- `docs/` — архитектура, ADR, исследования и отчёты.

## Результаты и документация

Начните с [отчёта Phase 5](docs/PHASE_5_REVIEW.md). [Phase 4](docs/PHASE_4_REVIEW.md) — Ely.by, развитие тестового входа и защищённое хранение токенов. [Phase 3](docs/PHASE_3_REVIEW.md) — установщик Minecraft/Java. [Phase 2](docs/PHASE_2_REVIEW.md) — завершённое хранилище и профили. [Phase 1](docs/PHASE_1_REVIEW.md) содержит исторические результаты foundation. [Phase 0](docs/PHASE_0_REVIEW.md) — историческое исследование, согласованное пользователем.

| Документ                                   | Содержание                                   |
| ------------------------------------------ | -------------------------------------------- |
| [Требования](docs/PRODUCT_REQUIREMENTS.md) | Границы MVP и критерии приёмки               |
| [Архитектура](docs/ARCHITECTURE.md)        | Модули, структура, диаграммы                 |
| [Контракты](docs/INTERFACES.md)            | Реальный bootstrap и целевые API             |
| [Модель данных](docs/DATA_MODEL.md)        | Профили, аккаунты, установки, состояние      |
| [Разработка](docs/DEVELOPMENT.md)          | Toolchain, команды, smoke и ограничения      |
| [Зависимости](docs/DEPENDENCIES.md)        | Реальный graph и план MVP                    |
| [ADR](docs/DECISIONS/README.md)            | Обоснования решений                          |
| [CI/CD](docs/CI_CD.md)                     | Матрица, пакеты и signing                    |
| [Тестирование](docs/TEST_PLAN.md)          | План полного MVP                             |
| [Риски](docs/RISKS.md)                     | Открытые технические риски                   |
| [Roadmap](docs/ROADMAP.md)                 | Этапы реализации                             |
| [Источники](docs/SOURCES.md)               | Первичная документация и metadata provenance |

Исследовательские снимки проверяются offline: `python3 docs/research/verify_snapshot.py`. Эта проверка не заменяет application tests.

Phase 6 начинается после согласования. Перед production-выпуском нужны real session/game/server smoke на всех платформах и согласованная подпись/распространение.
