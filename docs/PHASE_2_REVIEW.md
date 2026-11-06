# Phase 2 — Storage & Profiles

Дата: 2026-11-07. **Phase 2 завершена и готова к review.** Локальные проверки и native CI для Windows x64, Linux x64, macOS Intel и macOS ARM64 пройдены. Phase 3 не начата.

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

Release guard core/desktop — pass. Release IPC — 7 tests pass. macOS ARM64 .app/.dmg собраны; npm audit — 0 vulnerabilities. Native GUI macOS ARM64 — pass: создан/выбран дополнительный профиль, max RAM=3072 МиБ сохранён; quit/reopen восстановил selection и RAM. Native CI matrix — pass на всех четырёх targets.

## Ограничения и следующий этап

Нет старой реализованной production schema для миграции: unknown schemas не перезаписываются. Не выполняется автоматический перенос файлов между roots. Locks advisory Unix; не заявляется защита от администратора/процесса с теми же правами. Windows parent-directory flush не обещается. Interrupted staging сохраняется, автоматического GC нет. Непустой root без marker не принимается: при прерывании самого первого создания до публикации marker может потребоваться ручная проверка каталога или выбор новой пустой папки. Settings operations сериализуются коротким отдельным mutex; будущий installer не должен держать его во время downloads/hash.

Java file проверяется без его выполнения; поддержка Java 21/architecture и JVM flag compatibility относятся к runtime/launch. Signing остаётся internal ad-hoc; install/GPU/game smoke не выполнены на Windows/Linux/Intel. Установка Minecraft, авторизация, запуск и полный UI следующих фаз не реализованы. После успешной валидации Phase 2 предлагается Phase 3 — Minecraft Installer; переход требует отдельного запроса пользователя.

Локальные числовые результаты и SHA256 пакета: [phase-2-local.json](validation/phase-2-local.json). Smoke использовал штатную app data директорию Launcher; создан дополнительный профиль, выбранный после перезапуска. Пользовательские worlds не создавались и не удалялись.

## Native CI и пакеты

Проверенный application/workflow commit: `db802c78778601fe4f539cb3c71bdc599eaa736c`. [Run 42290329410, attempt 2 — success](https://github.com/HerrChichikov/minecraft-rust-launcher/actions/runs/42290329410). Frontend quality + четыре desktop jobs завершены успешно. После этого коммита дополняются только отчёты; application source и workflow не меняются.

| Target | Native debug/release checks | Пакеты |
|---|---|---|
| Windows x64 | pass | NSIS `.exe`, WiX `.msi` |
| Linux x64 | pass | `.deb` |
| macOS Intel | pass | `.dmg` |
| macOS ARM64 | pass | `.dmg` |

Все targets выполнили workspace tests, core dev-auth tests, release compile-fail guard, release IPC tests и packaging; tests не исключались из Windows/Linux. Загружено четыре target artifacts с retention 14 дней. Job/step outcomes, archive digests и IDs: [phase-2-ci.json](validation/phase-2-ci.json).

Первый Linux test run выявил EBADF при fsync O_PATH capability. Directory sync исправлен: readable `.` открывается относительно held Dir, fsync не отключён. Финальный source проверен повторно на Linux. В первой попытке финального run Ubuntu Azure mirror не успел скачать WebKit за 15 минут; повторён только Linux job на том же source SHA. Вторая попытка прошла полностью.

Локальный ARM64 пакет: `target/release/bundle/dmg/Launcher_0.1.0_aarch64.dmg`, 2 648 871 bytes, SHA256 `83a1e03a628a2b78a24164bb272e3354b9343d010f8567e7bf25b3c5f1624aae`. Подпись `.app` проверена `codesign --verify --deep --strict`; это internal ad-hoc build. Native GUI create/select/save/reopen smoke выполнен на storage implementation commit `42c70f89319bd45f141ba166bed8521f095a4a36`; финальный macOS пакет дополнительно открыт с восстановленными настройками. После Linux sync fix повторно пройдены local workspace и release IPC tests; UI source не менялся.

Выходной критерий Phase 2 выполнен: profiles/settings persistence, isolation, unsafe paths, revisions, OS concurrency/crash release, schema preservation, recoverable index и native checks. Следующий этап — Phase 3 Minecraft Installer, после отдельного запроса пользователя.
