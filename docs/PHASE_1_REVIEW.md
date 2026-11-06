# Phase 1 — Project Foundation

Дата: 2026-11-06, Asia/Yekaterinburg. Переход к Phase 1 подтверждён пользователем. Ниже фактический локальный результат; cross-platform CI выполняется отдельно в [репозитории](https://github.com/HerrChichikov/minecraft-rust-launcher/actions).

## Реализовано

- Rust workspace: `launcher-core` и `launcher-desktop`, edition 2024, pinned toolchain, Cargo.lock, rustfmt/Clippy policy.
- Независимый ApplicationService bootstrap; typed errors, platform model и базовые traits/models для profiles, storage, installer, Java runtime, AuthProvider, process supervisor и events. У этих будущих сервисов нет фиктивных implementations.
- Tauri 2 desktop adapter с одним read-only get_snapshot, versioned DTO и отдельной командной ACL. Только main bundled window может читать snapshot; remote origins, другие окна и несуществующие auth/play/fs/shell commands отклоняются.
- React/TypeScript strict/SCSS/Vite, точные npm dependencies и package-lock. Foundation screen показывает реальные версии/platform/build mode из Rust; имеет loading, sanitized error, explicit retry и отдельное browser-required состояние.
- Нет активного Play, профилей, login, Java/Minecraft download. Это следующие фазы, а не готовая функциональность.
- Backend-only SecretString с zeroize и redacted Debug; LaunchSpec не раскрывает argv/path в Debug и не сериализуется.
- Dev-auth feature зарезервирована, отсутствует в default features и запрещена в release двумя guards. MockAuthProvider ещё не реализован.
- GitHub Actions: frontend quality и четыре native build targets, pinned action SHAs, locked installs, release guard/IPC checks и internal package artifacts. Публичный release и updater не создаются.

## Локальные проверки

| Проверка | Результат |
|---|---|
| Clean npm ci --ignore-scripts --offline | Pass, lock-файл воспроизводим, install scripts не требуются |
| npm run check | Pass: Prettier, ESLint, strict typecheck, 10 Vitest tests, production Vite build |
| npm audit после исправления minimatch | 0 vulnerabilities в выбранном graph |
| cargo fmt / Clippy all-targets -D warnings | Pass |
| cargo test --workspace --locked | 7 tests pass: 3 core integration + 4 desktop unit/IPC |
| cargo test core с dev-auth | Те же 3 core tests pass; это не login smoke |
| Release compiler guard | Core и desktop отвергли запрещённую feature с точным expected diagnostic |
| Release desktop IPC с tauri/custom-protocol | 4 tests pass, включая ACL negatives |
| macOS ARM64 release .app | Собран; binary architecture arm64, ad-hoc signature verify pass |
| macOS ARM64 UI smoke | App открыт: tauri://localhost, «Ядро подключено», 0.1.0, macOS ARM64, Release; Play disabled |
| macOS Intel cross-build .app | Собран; binary architecture x86_64, ad-hoc signature verify pass |
| macOS ARM64 / Intel .dmg | Созданы локальные внутренние packages; не notarized release |

Intel cross-build не равен native Intel запуску. Windows/Linux до успешного CI не объявляются build-verified. Полный игровой smoke относится к Phase 7: Minecraft ещё не установлен и не запускался.

Первый CI run выявил Windows test-harness startup failure STATUS_ENTRYPOINT_NOT_FOUND. Добавлен Common Controls v6 manifest через linker для test executable и app согласно upstream Tauri issue; это исправление сборки, а не исключение тестов. Повторная матрица должна подтвердить его на Windows runner.

## Артефакты

- ARM64 app: `target/release/bundle/macos/Launcher.app` (~6.76 MiB).
- ARM64 DMG: `target/release/bundle/dmg/Launcher_0.1.0_aarch64.dmg` (~2.35 MiB).
- Intel app: `target/x86_64-apple-darwin/release/bundle/macos/Launcher.app` (~7.20 MiB).
- Intel DMG: `target/x86_64-apple-darwin/release/bundle/dmg/Launcher_0.1.0_x64.dmg` (~2.53 MiB).

Artifacts не входят в source tree. Точные SHA256 локальных packages — в [validation report](validation/phase-1-local.json). App/bundle ID пока рабочие (`Launcher`, `dev.launcher.desktop`). Нет Developer ID, Windows certificate или notarization; эти условия документированы, как допускается заданием.

## Обоснованные уточнения

[ADR-007](DECISIONS/007-foundation-tooling.md): совместимый TypeScript 6 вместо пока неподдерживаемого lint-инструментами TS 7; reviewed minimatch 11.0.1 override для устранения advisory; только используемые direct dependencies. Сетевые/Java/keyring libraries будут добавлены вместе с реальными implementations.

Исходные ADR 001–006 отмечены как согласованные. README, DEVELOPMENT, ARCHITECTURE, INTERFACES, DEPENDENCIES, CI_CD и ROADMAP обновлены для distinction между работающим foundation и будущими сервисами.

## Критерии готовности и следующий этап

Локальные foundation requirements выполнены. Полный cross-platform build criterion подтверждается только успешной четырёхплатформенной CI matrix; её run/results должны быть проверены перед объявлением Phase 1 полностью принятой. Наличие workflow не подменяет этот критерий. Native package install smoke и игра на всех платформах ещё впереди.

Следующий этап после review — Phase 2 Storage & Profiles: schema validation, default profile, атомарные settings, безопасные пути, profile isolation и locks. Phase 2 не начата автоматически. Выбор real Ely.by flow остаётся решением до Phase 4 и не блокирует foundation/storage.
