# Phase 3 — Minecraft Installer

Дата: 2026-11-07. Реализация проходит валидацию; native CI и проверка desktop UI ещё выполняются. Phase 4 не начата.

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
- Rust: metadata fixtures всех четырёх targets, ordered rules, legal/cyclic/escaping runtime links, corrupt hash retry, offline file reuse, same-size repair, cancellation/oversized write preservation, URL policy, storage suites и IPC ACL. Последние counts фиксируются после окончательного run.

## Зависимости и ограничения

reqwest 0.13.5 blocking + Rustls, SHA1 0.10.7 (Mojang compatibility), regex 1.x; Tokio process/time/io для отменяемого bounded Java probe. Произвольных frontend URL/path commands нет. Mock auth не добавлен; release guard сохранён. Minecraft, runtime и native JARs не включены в пакеты лаунчера и не загружаются в GitHub artifacts.

Native CI дополнен реальной установкой/повтором/loader spike на Windows/Linux/macOS Intel/ARM64. Каждый job использует isolated RUNNER_TEMP root; source checkout и packaged app остаются отдельно. CARGO_BUILD_TARGET теперь одинаков для check/test/package, чтобы не собирать release dependencies повторно в другом каталоге.

Осталось до закрытия Phase 3: финальный native CI, desktop smoke и фиксация receipts/packages. Никаких заявлений о готовности полного Minecraft launch/production auth/GPU/install smoke всех пакетов.
