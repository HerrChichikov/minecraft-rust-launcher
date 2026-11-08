# CI/CD

Создан [.github/workflows/ci.yml](../.github/workflows/ci.yml). Workflow запускается по push, pull_request и workflow_dispatch, с `contents: read`, отменой устаревших runs и без release publishing/signing secrets. Наличие YAML не равно успешному run: актуальные результаты перечислены в [Phase 4 review](PHASE_4_REVIEW.md), предыдущие этапы зафиксированы ниже.

Phase 1 matrix подтверждена [успешным run 42259358488](https://github.com/HerrChichikov/minecraft-rust-launcher/actions/runs/42259358488) на source commit `0f9480b9ac5058683728296472fe9325384e5283`: frontend quality и четыре native desktop jobs, 4 uploaded target artifacts. Фиксация этих результатов — documentation-only; application source и workflow после проверенного коммита не меняются.

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

Linux prerequisites устанавливаются noninteractive с ограниченными network timeouts/retries и 15-минутным лимитом шага. Windows test executables получают Common Controls v6 manifest через linker; тесты не исключаются из Windows matrix из-за upstream test-harness manifest bug.

Actions checkout 7.0.1, setup-node 7.1.0, upload-artifact 7.0.2 закреплены по commit SHA официальных tags; checkout не сохраняет credentials. Artifacts имеют target в имени и retention 14 дней, публикуются только как внутренние run artifacts. Отсутствие package file — ошибка job. Rust build cache пока не добавлен: сначала подтверждается чистая матрица.

## Production guard

`scripts/check-release-guard.py` требует nonzero compiler outcome с точным `LAUNCHER_DEV_AUTH_FORBIDDEN` для core и desktop. Случайная ошибка компиляции не считается pass. Release IPC tests проверяют разрешённые snapshot/storage/auth commands и отказ чужому window/remote origin. Начиная с Phase 4 production registry разрешает только Ely.by: forged mock provider/input и unknown fields отвергаются backend. Mock input type отсутствует в release; Play/fs/shell commands недоступны.

## Packaging и подпись

Локальный macOS app подписывается ad-hoc `-` для development/internal проверки. Это не notarized production distribution. Настройка Developer ID/notarization требует Apple credentials и отдельного защищённого CI environment; Windows signing — сертификата или signing service и timestamping. Секреты не доступны untrusted PRs. [macOS signing](https://v2.tauri.app/distribute/sign/macos/) и [Windows signing](https://v2.tauri.app/distribute/sign/windows/).

Сборка .deb должна устанавливать package dependencies через package manager; .exe/.msi проверяются независимо. Package build не заменяет installation smoke. Для полного MVP дополнительно нужен реальный Minecraft/GPU/runtime/keychain/auth smoke на каждом target из TEST_PLAN, который foundation CI не выполняет.

Пользователь уточнил repository: `HerrChichikov/minecraft-rust-launcher`; connection с правами записи подтверждена. Не создаём публичный release и не меняем default branch в обход review: Phase 1 опубликована в codex/phase-1-foundation; Phase 2 — в codex/phase-2-storage. Статус реального выполнения проверяется в GitHub Actions, не выводится из наличия YAML.

Phase 2 сохраняет ту же native matrix и расширяет workspace/release IPC tests: реальные storage operations, revision conflict и denial до filesystem initialization. Результат Phase 2 отслеживается отдельно в PHASE_2_REVIEW.md.

Phase 2 matrix подтверждена [run 42290329410 (attempt 2)](https://github.com/HerrChichikov/minecraft-rust-launcher/actions/runs/42290329410) на source commit `db802c78778601fe4f539cb3c71bdc599eaa736c`: пять successful jobs, четыре package artifacts. Linux-only rerun потребовался из-за медленного Ubuntu mirror при установке prerequisites; application/workflow commit не менялся. Детальные outcomes/digests — [phase-2-ci.json](validation/phase-2-ci.json). Финальная фиксация результатов documentation-only, с `[skip ci]`; все code/workflow changes проверены реальным run.

## Phase 3 matrix

Добавлен real upstream smoke в isolated RUNNER_TEMP: install Minecraft 1.21.1 + Java, повтор с assertion downloadedBytes=0, Java source loader spike LWJGL/GLFW/FreeType. Это не запуск Minecraft, не GPU/window/login smoke. Game/runtime files в artifacts не загружаются; сохраняются только text receipts и launcher packages. Backend unit HTTP fixtures остаются offline/loopback.

CARGO_BUILD_TARGET совпадает для check/test/guard/package, чтобы release dependencies переиспользовались. Linux prerequisites используют официальный HTTPS archive.ubuntu.com вместо нестабильно медленного Azure mirror; signed apt verification не отключается. Upload receipt step работает с always(), сетевые ошибки видны в logs. Текущий итог — PHASE_3_REVIEW.md.

Phase 3 финальный [run 42314460830](https://github.com/HerrChichikov/minecraft-rust-launcher/actions/runs/42314460830) — success на `b6a6c14f809afa984121a975a4773c911e935f43`: пять jobs, четыре package artifacts, четыре smoke receipt artifacts. После code freeze изменяются только документация/receipts с `[skip ci]`. Installer GUI smoke финального packaged Release отдельно пройден на macOS ARM64: start/cancel/retry до Ready, исходное хранилище восстановлено; receipt — validation/phase-3-gui.json.

## Phase 4 matrix

Финальный [run 42350741468](https://github.com/HerrChichikov/minecraft-rust-launcher/actions/runs/42350741468) — success на `69fafb2595abd563270ba411b329687cbd50b593`: frontend quality и четыре native jobs. Default/dev-auth workspace tests, production feature guards, release IPC restrictions, official installer/Java reuse/loader smoke, pinned agent download/integrity/corruption/repair и package builds прошли на Windows x64, Linux x64, macOS Intel и ARM64. Четыре package artifacts и четыре receipt artifacts перечислены с digest в [phase-4-ci.json](validation/phase-4-ci.json). Финальная публикация результатов — documentation-only `[skip ci]`.

Перед запуском пользователь отдельно подтвердил перевод репозитория в Public; visibility проверена через API. Первоначальный private run 42346631288 был заблокирован account billing/spending limit ещё до выполнения steps; он не считается тестированием. Billing settings не менялись. Headless jobs используют HTTP fixtures/injected credential store, не получают credentials реального Ely.by account и не считаются native keychain GUI test. Agent JAR и игровые/runtime файлы не входят в artifacts.
