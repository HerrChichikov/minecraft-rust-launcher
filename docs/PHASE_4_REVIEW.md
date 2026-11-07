# Phase 4 — Authentication

Дата: 2026-11-07. Реализованы Ely.by password/TOTP (flow согласован пользователем), development mock, backend-owned сессия в памяти и verified agent preparation. Локальные проверки, GUI smoke и native CI перечислены ниже по фактическому состоянию. Phase 5 не начата.

## Поведение

- Ely.by: authenticate по login/password; отдельный TOTP challenge; refresh с token rotation; validate с пустым успешным body; invalidate при logout. Endpoint config централизован, credentials отправляются только на fixed HTTPS authserver.ely.by; redirects и автоматические retries auth POST запрещены. Connect timeout 5 секунд, полный request 15 секунд, response до 128 КиБ. HTTP 429/5xx/network отличаются от rejected credentials; raw error bodies не возвращаются в UI.
- TOTP: opaque challenge ID, пароль/clientToken остаются backend-only. Время challenge — 120 секунд, максимум 5 попыток; cancel/logout очищают secret. Desktop supervisor очищает истёкший challenge даже при idle UI; при занятом auth worker очистка завершается после bounded request. UI очищает password/TOTP field при отправке; повторный ввод кода не передаёт пароль обратно в webview.
- ClientToken генерируется backend, response обязан вернуть тот же token; профиль/UUID берутся из provider response, а не из логина. Refresh не меняет profile UUID. Ошибка refresh не заменяет действующую сессию. Refresh и другие auth mutations single-flight: service владеет единственной сессией; dedicated auth mutex использует try_lock и возвращает AuthBusy, сетевой I/O выполняется на blocking worker. Workspace/UI thread не ждут этот I/O.
- Logout сначала удаляет local session/challenge, затем пытается отозвать remote token. Неудачный remote revoke возвращается отдельным notice при уже завершённом local logout. signout всех аккаунтов не используется.
- Session/LaunchIdentity не сериализуются; accessToken/clientToken — zeroizing SecretString с redacted Debug. IPC snapshots содержат только account summary/capabilities/revision/challenge ID. Пароль и TOTP на диск не записываются; токены живут только до завершения процесса. OS credential adapters/«Запомнить меня» пока не добавлены, plaintext fallback нет. Копии строк/буферов внутри JS/HTTP runtime не имеют гарантированного полного стирания.
- Mock type/input/registry branch компилируются только `debug_assertions` + opt-in `dev-auth`; ordinary debug и release не предлагают mock. `npm run dev:mock` включает явно помеченный Development / Mock. Ник: 3–16 ASCII letters/digits/underscore; UUID соответствует Java nameUUIDFromBytes UTF-8 `OfflinePlayer:<exact-name>` без namespace, с сохранением регистра.
- Tauri get_auth/login/continue_login/cancel_login/refresh_auth/logout доступны только bundled main window; payload deny_unknown_fields. Session/token import и provider URLs из IPC/config отсутствуют. Revision защищает mutations и UI от позднего initial response. AuthService.launch_identity перед будущим spawn повторяет build/provider gate, validate и при необходимости single-flight refresh; Ely.by identity требует verified agent. Play пока отключён.

## Authlib Injector

Pinned official build **56**, version **1.2.8**, source commit `5437f5b465ee3f5be0a1e5e1ae3c50978114f6fb`, SHA256 `9c7f4343e6c82034958ffb48c14a2cb0c85928be7283103ce17da00c6d5a7b10`. JAR скачивается только по exact HTTPS URL официального artifact host, redirects запрещены. До 3 попыток, 30 секунд/request, 8 МиБ body limit, cancellation; SHA256 до atomic publication и при каждом jvm_arguments. Путь content-addressed, managed I/O no-follow.

Аргументы: reviewed `-Dauthlibinjector.noLogFile` и `-javaagent:<verified-path>=https://account.ely.by/api/authlib-injector`. Prefetched metadata не подставляется автоматически. Core предоставляет argv для будущего Launch Engine; JVM/agent/Minecraft execution не заявлены. JAR отсутствует в launcher packages и CI artifacts.

Лицензия AGPL-3.0-or-later с Authlib Injector exception: [полный текст](licenses/authlib-injector.txt), [notice/source review](AUTHLIB_INJECTOR.md). Условия распространения самого agent сохраняются; production distribution assessment остаётся release gate.

## Проверки

Результаты local/GUI/native CI обновляются после завершения. Наличие compile/build/fixture tests не объявляется реальным Ely.by/game/server smoke.

- Frontend: production password form без mock, dev capabilities, backend identity/reconnect/logout, password clearing, TOTP continuation/cancel, pending/sanitized errors, failed remote logout notice, stale response ordering, malformed/spoofed DTO.
- Core HTTP fixtures: authenticate, wrong password/TOTP, challenge expiry/cancel, refresh rotation/client binding, empty validate/invalidate, HTTP 503/302/malformed/oversized response, failed refresh preservation/local logout, validate/refresh at launch boundary без fallback.
- Core mock: Java UUID Notch golden vector/case/invalid nickname, refresh/validate/logout, owned session/revisions, Debug/DTO secrecy.
- Desktop: shared Rust/TS contract, actual IPC dev login/logout, production forged-provider/unknown-field/ACL denial, invalid real inputs/unknown challenge before network.
- Compile-fail release guards core + desktop; release IPC without feature.
- Official agent smoke: SHA256, same-size corruption rejected before argv, repair/reuse; без execution.

## Ограничения

Реальный вход test account в Ely.by, TOTP/refresh/revoke на upstream, secure credential storage и совместимость agent/Game 1.21.1/server ещё не подтверждены. Mock не считается доказательством real auth. Сессии не переживают restart приложения. Desktop supervisor удаляет expired challenge, backend HTTP timeout ограничивает задержку очистки при активном request. AuthProvider — bounded synchronous contract, вызываемый на blocking worker, как InstallEngine; Tokio desktop используется для deadline supervisor.

Первый CI run 42346631288 не начал ни одного шага из-за account billing/spending restriction. Пользователь отдельно подтвердил перевод репозитория в Public; API подтвердил public visibility. Повторная native matrix должна проверить финальный code commit; старый blocked run не считается валидацией.

Следующий функциональный этап — Phase 5 Launch Engine по отдельному запросу пользователя.
