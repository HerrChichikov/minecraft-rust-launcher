# CI/CD — Phase 1

Создан [.github/workflows/ci.yml](../.github/workflows/ci.yml). Workflow запускается по push, pull_request и workflow_dispatch, с `contents: read`, отменой устаревших runs и без release publishing/signing secrets. Наличие YAML не равно успешному run: актуальные результаты перечислены в [Phase 1 review](PHASE_1_REVIEW.md).

## Quality job

Ubuntu 22.04; exact Node из `.node-version`; `npm ci --ignore-scripts`; `npm run check` (Prettier/ESLint/strict TypeScript/Vitest/Vite build); `npm audit --audit-level=moderate`. Lock-файлы обязательны, dependencies не обновляются внутри CI.

## Native desktop matrix

| Runner | Rust target | Artifacts |
|---|---|---|
| windows-2022 | x86_64-pc-windows-msvc | NSIS .exe, WiX .msi |
| ubuntu-22.04 | x86_64-unknown-linux-gnu | .deb |
| macos-15-intel | x86_64-apple-darwin | .dmg |
| macos-15 | aarch64-apple-darwin | .dmg |

Проверяется реальная rustc host architecture, чтобы изменение runner label не замаскировало target. Linux job устанавливает WebKitGTK/GTK prerequisites; baseline не поднимается на новый glibc случайно. [Официальный runner catalog](https://github.com/actions/runner-images) проверен на 2026-11-06; macos-14 retired, поэтому не используется.

Каждый desktop job: pinned Rust toolchain/components через repository toolchain file; clean npm install; frontend lint/build; rustfmt; Clippy all-targets; workspace tests; core development-feature tests; expected compile rejection release+dev-auth; release IPC tests с tauri/custom-protocol; Tauri locked bundle build. Windows/Linux не требуют наличия Java, поскольку игры на этом этапе ещё нет.

Actions checkout 7.0.1, setup-node 7.1.0, upload-artifact 7.0.2 закреплены по commit SHA официальных tags; checkout не сохраняет credentials. Artifacts имеют target в имени и retention 14 дней, публикуются только как внутренние run artifacts. Отсутствие package file — ошибка job. Rust build cache пока не добавлен: сначала подтверждается чистая матрица.

## Production guard

`scripts/check-release-guard.py` требует nonzero compiler outcome с точным `LAUNCHER_DEV_AUTH_FORBIDDEN` для core и desktop. Случайная ошибка компиляции не считается pass. Release IPC tests проверяют разрешённый read-only snapshot, отсутствие login/mock/play/fs/shell и отказ чужому window/remote origin. В Phase 4 эти проверки будут расширены реальным provider registry; сейчас ни одного provider нет.

## Packaging и подпись

Локальный macOS app подписывается ad-hoc `-` для development/internal проверки. Это не notarized production distribution. Настройка Developer ID/notarization требует Apple credentials и отдельного защищённого CI environment; Windows signing — сертификата или signing service и timestamping. Секреты не доступны untrusted PRs. [macOS signing](https://v2.tauri.app/distribute/sign/macos/) и [Windows signing](https://v2.tauri.app/distribute/sign/windows/).

Сборка .deb должна устанавливать package dependencies через package manager; .exe/.msi проверяются независимо. Package build не заменяет installation smoke. Для полного MVP дополнительно нужен реальный Minecraft/GPU/runtime/keychain/auth smoke на каждом target из TEST_PLAN, который foundation CI не выполняет.

Пользователь уточнил repository: `HerrChichikov/minecraft-rust-launcher`; connection с правами записи подтверждена. Не создаём публичный release и не меняем default branch в обход review: source changes предназначены для отдельной codex/phase-1-foundation ветки. Статус реального выполнения проверяется в GitHub Actions, не выводится из наличия YAML.
