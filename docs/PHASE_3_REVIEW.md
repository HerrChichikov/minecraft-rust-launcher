# Phase 3 — Minecraft Installer

Дата: 2026-11-07. **Phase 3 завершена и готова к review:** реализация, native CI всех четырёх targets и интерактивный installer GUI smoke на macOS ARM64 пройдены. Phase 4 не начата.

Работают metadata-derived загрузка Minecraft 1.21.1, библиотеки/rules, 3888 уникальных asset objects, logging config, managed Java 21, проверка SHA1/size и repair. Download workers ограничены восемью, temporary network/hash failures имеют максимум три попытки; все redirect destinations проверяются по HTTPS allowlist. Незавершённый файл не публикуется. Отмена кооперативная: между chunks; заблокированный HTTP request ограничен 30 секундами. Metadata control requests также имеют ограниченные retries; offline metadata fallback пока не реализован. Недоступность upstream даёт явную ошибку.

Runtime скачивается raw по официальному manifest в отдельную generation, ссылки создаются последними после проверки target chains. Поддерживаются только внутренние ссылки на файлы, как в текущих manifests; ссылки на каталоги явно отклоняются. До публикации выполняется Java probe с очищенным environment, timeout 5 секунд, bounded stdout/stderr и проверкой major/architecture. Повреждённая generation не изменяется пофайлово: создаётся новая; автоматического удаления старых generations/staging нет. Custom Java проверяется только после явного выбора в профиле. macOS host detection учитывает Rosetta; отдельный Rosetta smoke остаётся Phase 7.

Library paths включают expected SHA1, client — metadata revision. Asset index хранится под именем `<upstream-id>-<sha1>.json`; это же имя передаётся future launch как assetIndexId. Индекс не перезаписывает используемую revision. Profile game/saves не участвуют в repair. Все записи проходят capability-relative adapter с no-follow parents, installation/profile leases запрещают конкурирующую установку и root switch.

Проверка сейчас всегда пересчитывает SHA1: fingerprint shortcut Quick не используется. Это сознательное упрощение; verification-index остаётся восстанавливаемым hint storage, а не authority. Corrupt index карантинируется при открытии installation store, future schema отклоняется. Частичные downloads не продолжаются с byte-range offset: проверенные файлы переиспользуются, partial скачивается заново.

Tauri: start_install/get_installation/cancel_install, main-only ACL, bounded progress rate, sequence ordering и operation ID. HTTP и файловые операции выполняются на blocking workers, UI mutex не удерживается во время установки. UI подключает event listener перед snapshot, отклоняет старые sequence, показывает ошибки/отмену и ожидает реального terminal outcome. Play остаётся отключён: authentication и launch engine — следующие фазы.

## Фактические локальные проверки

macOS ARM64:

- Первая реальная установка: **1 022 469 614 bytes** скачанных artifacts; Java 21 ARM64 probe — pass.
- Повторная проверка: **0 downloaded artifact bytes**, reuse файлов и той же runtime generation — pass. Metadata HTTP requests выполняются повторно.
- Same-size corruption: повреждённый asset 9101 bytes восстановлен; скачано ровно **9101 bytes**; world sentinel сохранён.
- Реальная загрузка native LWJGL, GLFW и FreeType из установленных JAR — pass, architecture aarch64. Это loader spike, не Minecraft/GPU/window smoke.
- `npm run check`: format/ESLint/strict TS/**29 frontend tests**/Vite build — pass.
- Rust: metadata fixtures всех четырёх targets, ordered rules, legal/cyclic/escaping runtime links, corrupt hash retry, offline file reuse, same-size repair, cancellation/oversized write preservation, URL policy, storage suites и IPC ACL. **32 core + 8 desktop tests — pass** (Windows дополнительно проверяет normal/verbatim executable paths); desktop suite также pass в release. Оба release dev-auth guard — pass. npm audit — 0 vulnerabilities.

- Repair Java: новое поколение из проверенного кеша, **0 artifact download bytes**, прежний файл не изменён; world sentinel сохранён; Java/native probes pass.
- GUI smoke packaged Release `.app`: проверка существующей установки — Ready с 0 artifact download bytes; в отдельном пустом root виден progress, Cancel приводит к «Установка отменена», повтор проходит подготовку Java и завершается Ready. Исходные data root/active profile восстановлены, max RAM=3072 МиБ; оба config.json побайтово неизменны. [GUI receipt](validation/phase-3-gui.json).

Локальные receipts: [phase-3-local.json](validation/phase-3-local.json).

## Зависимости и ограничения

reqwest 0.13.5 blocking + Rustls, SHA1 0.10.7 (Mojang compatibility), regex 1.x; Tokio process/time/io для отменяемого bounded Java probe. Произвольных frontend URL/path commands нет. Mock auth не добавлен; release guard сохранён. Minecraft, runtime и native JARs не включены в пакеты лаунчера и не загружаются в GitHub artifacts.

Native CI дополнен реальной установкой/повтором/loader spike на Windows/Linux/macOS Intel/ARM64. Каждый job использует isolated RUNNER_TEMP root; source checkout и packaged app остаются отдельно. CARGO_BUILD_TARGET теперь одинаков для check/test/package, чтобы не собирать release dependencies повторно в другом каталоге.

Code/CLI/native validation, packages и installer GUI smoke на доступном macOS ARM64 подтверждены. Полный Minecraft launch/production auth/GPU и установка пакетов на всех ОС относятся к последующим фазам.

## Native validation findings

Первая matrix подтвердила реальную установку/reuse/loader на macOS ARM64, а также debug/release/ACL tests на Windows и Intel. Windows Java probe работал, но class loading с verbatim executable path `\\?\` приводил к HotSpot jimage crash. Java-facing executable path теперь преобразуется в обычный Win32 drive/UNC path; filesystem capabilities сохраняют canonical handles. Добавлен Windows regression test. Java architecture aliases amd64/x86_64 и arm64/aarch64 нормализуются, для macOS rules используется x86_64. [OpenJDK platform test library](https://github.com/openjdk/jdk/blob/master/test/lib/jdk/test/lib/Platform.java) подтверждает различие macOS os.arch.

Для Linux Azure mirror несколько раз не укладывался в 15 минут. CI переключён на официальный archive.ubuntu.com по HTTPS и устанавливает build prerequisites без recommends; набор обязательных dependencies сохранён. Smoke logs теперь публикуются при ошибке и сохраняются отдельными internal receipt artifacts, чтобы диагностика не терялась. Повторная matrix прошла для финального code/workflow commit.

## Финальная native matrix

Проверенный source/workflow commit: `b6a6c14f809afa984121a975a4773c911e935f43`. [Run 42314460830 — success](https://github.com/HerrChichikov/minecraft-rust-launcher/actions/runs/42314460830). Все пять jobs successful. После этого коммита дополняются только документы и receipts.

| Target | Debug/release/guard | Реальная установка + reuse + native loader | Пакеты |
|---|---|---|---|
| Windows x64 | pass | pass, amd64 | NSIS .exe / WiX .msi |
| Linux x64 | pass | pass, amd64 | .deb |
| macOS Intel | pass | pass, x86_64 | .dmg |
| macOS ARM64 | pass | pass, aarch64 | .dmg |

Сохранены четыре package artifacts и четыре text smoke receipts; game/runtime/JAR binaries в artifacts отсутствуют. IDs, archive digests и step outcomes — [phase-3-ci.json](validation/phase-3-ci.json). Реальная Windows class loading теперь проходит с нормализованным executable path; Intel probe принимает корректный Java os.arch alias.

Локальный ARM64 DMG: `target/release/bundle/dmg/Launcher_0.1.0_aarch64.dmg`, **5 056 003 bytes**, SHA256 `0c2ac15e21c0ece2734bb5b02992f87b086c201f8183e0e7ddc4e6a426685560`; `.app` signature verification pass, internal ad-hoc build. После ручной разблокировки Mac проверены start/cancel/retry в окне этой финальной сборки. Изменения кода после native CI не потребовались.

Следующий функциональный этап — Phase 4 Authentication, по отдельному запросу пользователя. Production release, реальный login/server/game/GPU smoke и notarization не являются результатами Phase 3.
