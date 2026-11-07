# План тестирования

Статус: план полного MVP. Реальные проверки Phase 2 перечислены в [review](PHASE_2_REVIEW.md); остальные строки не означают готовую реализацию. Phase 0 — отдельно в [историческом review](PHASE_0_REVIEW.md). Unit/integration tests не зависят от доступности Mojang/Ely.by; network e2e запускаются отдельным заданием.

## Матрица

| Область | Unit / integration сценарии | Критерий |
|---|---|---|
| Metadata | Manifest/1.21.1/asset/runtime parsing, отсутствующие поля, malformed JSON, unsupported format | Явный typed error, никакого guessed successful plan |
| Rules | Absent/empty rules, order allow/disallow, OS, JVM arch, regex, features true/false, string/array values | Golden выбранный набор для 4 platform contexts |
| Installer | Missing, correct, corrupt, same-size changed, index lost, full repair | Исправный reuse без download, повреждённый восстановлен |
| Download | Bad hash/size, overlong body, disconnect, timeout, 404/429/5xx, redirects, cancel | Bounded retry; partial не виден как complete |
| Atomicity | Cancel/crash до/после publish, concurrent same resource, disk full, permissions, sharing violation | Old valid data сохраняются, snapshot только complete |
| Assets | Duplicate hashes, leading-zero hash, unexpected logical names | Одна загрузка на hash, content-addressed destination |
| Natives | Modern artifact JAR retained in classpath, macOS patched classifier, per-run isolation | Никакого flatten/overwrite, runtime smoke на всех arch |
| Runtime | Catalog platform mapping, candidate policy, major/arch mismatch, raw files, exec bits | Правильный runtime, no PATH fallback |
| Runtime links | Valid `../java.base`, absolute target, escape, cycle, symlink parent | Только internal links; complete tree до publish |
| Runtime recovery | Corrupt binary, generation in use, missing lib, interrupted staging | Используемый runtime не изменяется; восстановление корректно |
| Auth | Provider contract, invalid creds, TOTP, expired/network difference, refresh rotation/race, empty body | Не теряем действующую сессию при transient error |
| Mock | Стабильный Java-compatible offline UUID, регистр, invalid name | Явный development identity |
| Production | release+dev-auth compile-fail; release без feature, поддельные IPC/config provider IDs | Backend не создаёт mock session и не запускает с ней игру |
| Secrets | HTTP errors/body, argv, split-chunk stdout/stderr, credential store unavailable | No known secret во всех контролируемых sinks/DTO |
| Launch args | CP order/delimiter, spaces/Unicode, placeholders, logging `${path}`, feature-gated values | Golden Vec argv, неизвестный placeholder до spawn |
| Settings | RAM overflow/min>max, classpath delimiter in path, NUL, @argfile, injected agent | Reject до запуска, UI actionable error |
| Processes | Fake child normal/nonzero/spawn failure, flood без newline, cancel, kill/wait race, double Play | UI не блокируется, один run/profile, child reaped |
| Storage | Profile isolation, newer schema, corrupted index, root switch Busy, traversal/UNC/ADS/reparse | Не затронуты saves и соседние directories |
| UI | Login/challenge/error, progress/cancel, running/exit, keyboard/focus, reconnect sequences | Последнее состояние без stale event rollback |

Часть tests работает на filesystem/process fixtures, часть на local HTTP fixture server. Plain HTTP loopback разрешён только test client policy, недоступен production config. Никаких live credentials в fixtures. Реальные metadata снимки хранят provenance и становятся regression fixtures после Phase 1; scripts исследования не подменяют core tests.

## Интеграционные уровни

1. Pure core: parsing/rules/hash/argv deterministic tests.
2. Local integration: HTTP fault server + temporary root + fake child executable; на каждой целевой ОС собственный fake child, без зависимости от shell utilities.
3. Desktop integration: custom commands ACL, backend validation, event subscription/reconnect, release mock probes. Проверяем конкретные compiled artifacts, не только frontend build flags.
4. Opt-in upstream test: metadata freshness и небольшой checksum download, offline fixtures остаются стандартным CI. Полная установка не запускается на каждом PR.
5. Phase 7 manual/controlled graphical smoke реального Minecraft и package install. Headless CI build не подтверждает работу GPU/GLFW, macOS first thread или server login.

## Игровой smoke на каждой платформе

Чистая VM/машина без system Java → установить пакет → создать профиль → development login → скачать официальный runtime/игру → главное меню Minecraft → создать/открыть локальный тестовый мир → закрыть игру → убедиться в Ready и exit status → повторный запуск без повторного download → испортить один cache object → repair → запустить снова. Отдельно cancel/restart download и max/min RAM.

Для production дополнительно реальный Ely.by login/refresh/restart/logout, agent SHA256 check, подключение к согласованному серверу с правильной server-side auth, профиль/скины и secure-profile/chat поведение. Token visibility/log inspection обязательны. Не используем production worlds для stop/crash tests.

Каждый smoke report: commit, artifact hash, OS/build/arch, JVM version/arch, GPU/driver, сценарий, exit outcome, sanitized logs, pass/fail. Статус «поддерживается» только после pass; неподтверждённые targets остаются planned.

## Реализованные проверки Storage & Profiles

- `crates/launcher-core/tests/storage.rs`: 18 test cases, из них два — child-process helpers. Настоящие temp roots, restart, UUID/name/path separation, world sentinel, switch/back, nested/unmanaged/corrupt root отказ до commit, revisions, custom Java file validation, leases, corrupt/future JSON/index, symlink denial. Процессный lock holder завершается принудительно; оставшиеся lock-файлы не блокируют повторный open.
- `storage/filesystem.rs`: portable traversal/UNC/ADS/device names; failed oversized write сохраняет старый JSON; publication fault до rename сохраняет старые данные, post-rename sync failure возвращает Durability при уже опубликованных новых данных. Обычная запись вызывает настоящий Unix directory sync, поэтому Linux O_PATH regression ловится без отдельного OS mock.
- `src-tauri/src/tests.rs`: реальный handler/ACL и isolated paths, create/select/save persisted JSON, stale rejection; denial чужому window/remote origin до создания config/data; fs/shell/auth/play недоступны. Release выполняет тот же набор с custom-protocol.
- Frontend: 21 test cases суммарно foundation/client/profiles. Workspace DTO shared fixture, malformed responses, loading, create/select, pending save, sanitized conflict + draft preservation/refresh, explicit no-migration root switch, browser without backend.

Результаты native matrix и точный tested commit берутся из PHASE_2_REVIEW / validation/phase-2-ci.json. Symlink checks не пропускаются на Windows. Disk full/permission-specific errors и реальное внезапное отключение питания не симулируются: fault injection подтверждает границы publication, но не заменяет storage hardware testing.

## Phase 3 — фактическое покрытие

Core installer tests: четыре version/rule platform fixtures и asset hash dedup; ordered rules, unknown predicates/bad regex; четыре runtime manifests, legal/escaping/cyclic links; retry corrupted same-size body, hash recheck/reuse/repair, cancelled streaming response cleanup, oversized publication preserving old file, strict HTTPS upstream policy. JVM aliases normalization на всех targets; Windows дополнительно normal/verbatim executable path conversion. Существующие storage/concurrency/schema/secret Debug tests сохранены.

Frontend: installation reconnect, loading, no premature Ready, sequence rollback, operation-ID cancellation, sanitized errors/retry и malformed DTO. Desktop: installer state/unknown cancel/ACL, отказ remote/other window до filesystem initialization. Native debug и release suites выполняются без исключения targets.

Upstream CI smoke: install official Minecraft/Java, recheck with downloadedBytes=0, real LWJGL/GLFW/FreeType loading through installed runtime. Local macOS ARM64 дополнительно проверяет same-size asset repair (9101 bytes only), retained world sentinel и runtime generation repair without rewriting old generation. Это не полный Minecraft/window/GPU/server smoke — он остаётся последующим этапом.
