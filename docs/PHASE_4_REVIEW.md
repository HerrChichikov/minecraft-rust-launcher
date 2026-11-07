# Phase 4 — Authentication

Дата: 2026-11-07. Реализован разрешённый MVP вариант: development mock и подготовленный Ely.by provider с явным AuthUnavailable. Реальный password/TOTP/OAuth flow не включён; решение по обработке пароля запрошено отдельно. Phase 5 не начата.

## Поведение

- Debug без feature не предлагает mock. Только `debug_assertions` + opt-in `dev-auth` компилируют MockAuthProvider и nickname input branch. `npm run dev:mock` открывает этот вариант.
- Ник: 3–16 ASCII letters/digits/underscore. UUID соответствует Java nameUUIDFromBytes UTF-8 `OfflinePlayer:<exact-name>` без namespace; case сохраняется. Token backend-only, zeroizing, Debug redacted; session/identity не сериализуются.
- AuthService владеет единственной сессией процесса. Login/logout защищены revision; invalid/stale login сохраняет прежнюю сессию. Logout удаляет local secrets. Mock не требует remote revoke. После перезапуска аккаунт не восстанавливается; plaintext/OS credential persistence не добавлены.
- AuthProvider имеет capabilities/authenticate/refresh/validate/logout/launch_identity. Ely.by config централизует auth base и injector API root. Пока все реальные операции возвращают AuthUnavailable, HTTP credentials не отправляются. Не реализован неподтверждённый desktop OAuth с embedded client secret.
- Tauri `get_auth`, `login`, `logout` доступны только bundled main window; payload deny_unknown_fields. Нет команды импорта session/token или произвольного provider URL. Snapshot содержит public account summary, capabilities, revision и memory_only; токены не попадают в DTO/events/files.
- UI получает возможности от backend, явно маркирует Development / Mock, показывает pending/error и backend identity, восстанавливает текущую сессию при remount. Revision guard не допускает rollback от позднего initial response. Production показывает недоступность реального входа и не предлагает тестовый ник. Play остаётся отключён до Phase 5.

## Authlib Injector

Pinned official build **56**, version **1.2.8**, source commit `5437f5b465ee3f5be0a1e5e1ae3c50978114f6fb`, SHA256 `9c7f4343e6c82034958ffb48c14a2cb0c85928be7283103ce17da00c6d5a7b10`. Descriptor повторно получен с официального API. JAR скачивается только по exact HTTPS URL официального artifact host; redirects запрещены. До 3 попыток, 30 секунд/request, 8 МиБ body limit, cancellation; SHA256 проверяется до atomic publication и при каждом вызове jvm_arguments. Путь content-addressed; managed no-follow filesystem adapter сохраняет границу root.

В аргументах предусмотрены `-Dauthlibinjector.noLogFile` (проверено по Logging.java pinned source) и `-javaagent:<verified-path>=https://account.ely.by/api/authlib-injector`. Prefetched metadata не подставляется автоматически. Код предоставляет проверенные argv для будущего Launch Engine; JVM/agent/Minecraft execution ещё не реализован. JAR не включён в launcher packages и не загружается в CI artifacts.

Лицензия: AGPL-3.0-or-later с Authlib Injector exception для unmodified inclusion/IPC/Java agent usage; текст сохранён в [licenses/authlib-injector.txt](licenses/authlib-injector.txt), notices/source links — [AUTHLIB_INJECTOR.md](AUTHLIB_INJECTOR.md). Условия распространения самого agent сохраняются; production distribution assessment остаётся release gate.

## Проверки

Локальные результаты и GUI/native CI обновляются после завершения проверок. Фактический статус сохраняется в validation/phase-4-local.json и validation/phase-4-ci.json, когда эти проверки завершены. Наличие сборки не объявляется real Ely.by/game/server smoke.

- Frontend: capabilities, no production mock UI, backend identity/reconnect/logout, pending/revision rejection, malformed/spoofed DTO, sanitization, refresh/initial response ordering.
- Core: Java UUID golden vector Notch, case sensitivity, nickname validation, provider refresh/validate/logout, session ownership/revision/Debug secrecy, AuthUnavailable.
- Desktop: shared Rust/TS auth contract, actual IPC login/logout in dev-auth, production forged provider denial, unknown-field rejection, remote/other-window ACL.
- Compile-fail release guards core + desktop; release IPC restrictions without feature.
- Opt-in official agent smoke: SHA256, same-size corruption rejected before argv, repair and reuse; no agent execution. CI выполняет его на четырёх targets.

Следующий этап — Phase 5 Launch Engine по отдельному запросу. Реальный Ely.by вход, secure credential adapter, agent/Game 1.21.1/server compatibility остаются явно непроверенными и не подменяются mock success.
