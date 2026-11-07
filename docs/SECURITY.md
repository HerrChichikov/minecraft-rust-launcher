# Безопасность

Статус: требования полного MVP. В Phase 1 реально включены restricted Tauri app-command ACL/CSP, sanitized bootstrap DTO, backend-only secret types и release dev-auth guard; проверены positive/negative IPC tests и redaction Debug. В Phase 2 добавлены capability-relative managed paths, no-follow opens, schemas/size limits, atomic JSON, OS leases и settings validation (ADR-008). Network/runtime execution controls ниже относятся к будущим implementations, это не аудит готового MVP.

## Границы доверия

Webview считается потенциально скомпрометированным: все IPC arguments валидирует backend. Metadata/HTTP/cache/profile JSON — внешние или изменяемые данные. Java и agent — код, который запускается с правами пользователя. Backend не получает административных прав для обычной установки игры.

| Граница | Контроль | Проверка |
|---|---|---|
| UI → Rust | Typed commands, validation, own IDs/handles | Unknown ID, malformed DTO, command ACL negatives |
| Metadata → filesystem | Safe relative paths, no unexpected links/reparse traversal | ZIP slip, UNC, ADS, symlink escape, race tests |
| HTTP → trusted file | HTTPS, redirect validation, size/hash, staged atomic publish | Wrong hash/length, disconnect, redirect to HTTP/private host |
| Config → process | Absolute verified executable, argv array, env cleanup, options policy | Spaces/Unicode, @argfile, agent/classpath overrides |
| Session → logs/UI | Secret types, explicit DTO, scrub before sinks | Captured stdout/stderr, HTTP body, errors, event probes |
| Release → mock | Conditional compilation + compile-fail + IPC probes | Release with feature rejected; release without feature cannot login mock |

## Tauri

Используем local bundled content и строгий CSP; не загружаем remote scripts/fonts. Production connect-src ограничен собственным IPC; HTTP работает в Rust. Dev Vite origin/port задаётся отдельно и не переносится в release CSP.

Capabilities только для main window и точных permissions. Не включаем frontend shell/fs/http plugins. App commands перечисляются в `tauri_build::AppManifest::commands` с собственными permissions: [Tauri прямо отмечает](https://v2.tauri.app/security/capabilities/), что без такого ограничения зарегистрированные custom commands доступны webviews по умолчанию. Remote origins не получают capability. Events/channel payloads всегда sanitized, secret API через них отсутствует.

## Сеть и исполняемые файлы

Upstream policy централизована: Mojang metadata/data/libraries/assets; Ely.by только approved provider endpoints; agent только approved artifact host. Проверяем каждый redirect, запрещаем downgrade, userinfo и неразрешённые destinations; используем отдельные request policies по role. API для произвольного URL из frontend отсутствует. Изменение production provider config не позволяет добавить arbitrary executable host.

SHA1 используется ради совместимости с Mojang published checksums; это контроль целостности, не подпись издателя. Корни manifest доверяются через HTTPS/config. Для agent применяется опубликованный SHA256 и pinned reviewed build. SHA из того же скомпрометированного источника не защищает от компрометации upstream — этот риск явно остаётся.

JSON parsing, download output, log chunks, archive extraction имеют лимиты размера; HTTP retries и concurrency bounded. Native/runtime paths и permissions проверяются перед execution. Managed shared resources не должны быть writable сторонним пользователям. Система не обещает защиту от root/admin или равноправного вредоносного локального процесса.

## Credentials и диагностика

Нет passwords/access tokens в localStorage, profile JSON, telemetry, events и full argv logs. Credential store unavailable не включает plaintext fallback. Password UI state очищается после запроса, Rust memory минимизируется/zeroize по возможности. Никакого HTTP body logging для auth. Встроенную telemetry в MVP не добавляем.

Игра может печатать чувствительные данные: перед file/UI sinks применяется streaming redaction, включая secret на границе chunks и типовые bearer/accessToken поля; bounded carry хранит только необходимый хвост. Agent file logging отключается reviewed argument (`-Dauthlibinjector.noLogFile`) при интеграции, чтобы вывод проходил через наши sinks. Debug agent auth dump не включается.

Minecraft и JVM могут самостоятельно писать game logs, crash reports, heap dumps; это не полностью контролируется launcher scrubber. Не экспортируем их автоматически; перед ручным diagnostic export отдельная проверка/redaction. Процессные argv потенциально доступны локальным наблюдателям. Это известные ограничения стандартного launch protocol.

## Release

До production: dependency/license review, no mock gate, negative IPC tests, package install smoke, token leak tests и platform signing assessment. macOS/Windows signing secrets не доступны PR builds. Собственный updater позже требует отдельной подписи manifest/package и threat model, не одного checksum.

## Реализовано в Phase 4

Mock type/input/registry branch компилируются только debug+dev-auth. Production IPC отвергает поддельный provider ID, unknown fields/session import и untrusted origin/window; no runtime/config fallback. AuthService владеет process-only secrets, DTO содержат только summary. Login/logout revision защищает от stale writes; launch_identity повторяет build/provider gate. Real credential transmission, OS credential persistence и real auth refresh пока отсутствуют.

Pinned agent скачивается по exact official HTTPS URL без redirects, body ограничен 8 МиБ, SHA256 до publication и argv. File logging отключается reviewed noLogFile property. Game stdout streaming redaction и реальный агент выполняются только при будущей реализации Launch Engine; наличие argv helper не объявляет эти sinks уже реализованными.
