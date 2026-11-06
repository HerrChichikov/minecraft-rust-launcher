# Phase 2 — Storage & Profiles

Дата: 2026-11-07. Реализация готова; native CI и GUI smoke выполняются. Phase 3 не начата. Этот отчёт будет дополнен фактическим результатом, до этого все четыре target не считаются подтверждёнными для Phase 2.

## Что работает

- System app directories через Tauri; стабильная bootstrap config, default `data` subdirectory, root UUID и schemaVersion=1.
- Первый профиль 1.21.1, backend UUID v4, создание/выбор/переименование, сохранение RAM, managed/custom Java path и ограниченного списка JVM arguments. Версия/архитектура Java и запуск пока не проверяются.
- `profiles/UUID/game`, logs и runs изолированы. Display name не становится путём. Shared layout создан без скачивания игры/Java.
- Атомарная публикация JSON и профиля, optimistic revisions, root context для IPC, app/root/profile OS leases, отказ смены занятого root.
- Switch root без миграции и удаления прежних данных; при возврате восстанавливается selection этого root. UI объясняет поведение до нажатия кнопки.
- Capability-relative no-follow managed filesystem, переносимая lexical path validation, неизвестные schemas fail closed. Повреждённые settings сохраняются для восстановления.
- Verification index storage, bounded records, quarantine/rebuild corrupt cache. Индекс не выдаёт данные за проверенные файлы; hashing/repair — Phase 3.
- Реальные Tauri commands + ACL для bundled main window. UI ждёт backend outcome, сохраняет draft после ошибки, отклоняет malformed DTO и не подставляет browser mock.

## Изменения

Core: storage/filesystem, workspace и index; profiles/validation; typed errors и profiles availability. Desktop: WorkspaceState, bounded spawn_blocking adapters, sanitized errors, command permissions и isolated mock IPC tests. Frontend: ProfilesPanel, typed storage client, SCSS и tests. Schema fixture общий Rust/TypeScript. Dependencies: cap-std/cap-fs-ext 4.0.3, sysinfo 0.40.1 (system only), tempfile 3.27.0 (dev only), serde_json теперь production core. Новых frontend dependencies нет.

## Локальные проверки

macOS ARM64, Rust 1.99.1 / Node 26.13.0:

- `npm run check` — pass: format, ESLint, strict TS, **21 frontend tests**, production Vite build.
- `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings` — pass.
- `cargo test --workspace --locked` — pass: **24 core + 7 desktop tests** (включая два child-process helper tests).
- Проверены restart round trip, Unicode/space paths, world isolation, switch/back без migration, stale writes, RAM/JVM validation, corrupt/future schemas, interrupted staging/JSON, pre/post commit faults, symlink denial, межпроцессные locks и освобождение после kill.
- IPC tests: create/select/save пишет настоящий config; stale revision отказ; чужой window/remote origin не создаёт файлов. Auth/play/fs/shell остаются недоступными.

Release guard core/desktop — pass. Release IPC — 7 tests pass. macOS ARM64 .app/.dmg собраны; npm audit — 0 vulnerabilities. GUI smoke и native matrix ещё выполняются.

## Ограничения и следующий этап

Нет старой реализованной production schema для миграции: unknown schemas не перезаписываются. Не выполняется автоматический перенос файлов между roots. Locks advisory Unix; не заявляется защита от администратора/процесса с теми же правами. Windows parent-directory flush не обещается. Interrupted staging сохраняется, автоматического GC нет. Settings operations сериализуются коротким отдельным mutex; будущий installer не должен держать его во время downloads/hash.

Java file проверяется без его выполнения; поддержка Java 21/architecture и JVM flag compatibility относятся к runtime/launch. Signing остаётся internal ad-hoc; install/GPU/game smoke не выполнены на Windows/Linux/Intel. Установка Minecraft, авторизация, запуск и полный UI следующих фаз не реализованы. После успешной валидации Phase 2 предлагается Phase 3 — Minecraft Installer; переход требует отдельного запроса пользователя.
