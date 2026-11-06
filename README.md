# Minecraft Launcher

Основа desktop-лаунчера Minecraft Java Edition на Tauri 2, Rust и React / TypeScript / SCSS / Vite.

**Текущий этап — Phase 1.** Работают desktop-оболочка, подключение к независимому Rust core и диагностика сборки. Установка Minecraft, Java, профили, авторизация и запуск игры ещё не реализованы; кнопка «Играть» отключена. Функциональность следующих этапов описана интерфейсами, без фиктивных успешных ответов.

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
cargo test -p launcher-core --features dev-auth --locked
python3 scripts/check-release-guard.py
npm run tauri -- build --ci -- --locked
```

`dev-auth` пока только зарезервированная development feature. Mock provider будет реализован в Phase 4; release с этой feature отвергается compiler/build guard уже сейчас.

## Структура

- `frontend/` — экран foundation, typed IPC client, unit tests, SCSS.
- `crates/launcher-core/` — независимый bootstrap, типизированные ошибки и contracts для следующих этапов.
- `src-tauri/` — read-only `get_snapshot`, DTO, ACL/CSP и desktop packaging.
- `tests/contracts/` — общий Rust/TypeScript wire-contract fixture.
- `.github/workflows/ci.yml` — pinned quality checks и build matrix Windows x64 / Linux x64 / macOS Intel / macOS ARM64; без публичного release.
- `docs/` — архитектура, ADR, исследования и отчёты.

## Результаты и документация

Начните с [отчёта Phase 1](docs/PHASE_1_REVIEW.md): он разделяет реальные локальные результаты и ещё не проверенные платформы. [Phase 0](docs/PHASE_0_REVIEW.md) — историческое исследование, согласованное пользователем.

| Документ                                   | Содержание                                   |
| ------------------------------------------ | -------------------------------------------- |
| [Требования](docs/PRODUCT_REQUIREMENTS.md) | Границы MVP и критерии приёмки               |
| [Архитектура](docs/ARCHITECTURE.md)        | Модули, структура, диаграммы                 |
| [Контракты](docs/INTERFACES.md)            | Реальный bootstrap и целевые API             |
| [Модель данных](docs/DATA_MODEL.md)        | Профили, аккаунты, установки, состояние      |
| [Разработка](docs/DEVELOPMENT.md)          | Toolchain, команды, smoke и ограничения      |
| [Зависимости](docs/DEPENDENCIES.md)        | Реальный graph Phase 1 и план MVP            |
| [ADR](docs/DECISIONS/README.md)            | Обоснования решений                          |
| [CI/CD](docs/CI_CD.md)                     | Матрица, пакеты и signing                    |
| [Тестирование](docs/TEST_PLAN.md)          | План полного MVP                             |
| [Риски](docs/RISKS.md)                     | Открытые технические риски                   |
| [Roadmap](docs/ROADMAP.md)                 | Этапы реализации                             |
| [Источники](docs/SOURCES.md)               | Первичная документация и metadata provenance |

Исследовательские снимки проверяются offline: `python3 docs/research/verify_snapshot.py`. Эта проверка не заменяет application tests.

Phase 2 не начинается автоматически. Перед публичным выпуском нужны реальная авторизация, игровой smoke на всех платформах и согласованная подпись/распространение.
