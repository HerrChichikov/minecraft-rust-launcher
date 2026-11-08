# Phase 4 — Authentication

Обновлено: 2026-11-08. Phase 4 завершена в заявленном объёме: Ely.by password/TOTP (flow согласован пользователем), development mock, backend-owned сессия, optional OS credential storage и verified agent preparation. Финальный code commit — `69fafb2595abd563270ba411b329687cbd50b593`; дальнейшая фиксация результатов меняет только документацию. Локальные проверки, GUI smoke и native CI перечислены ниже по фактическому состоянию. Phase 5 не начата.

## Поведение

- Ely.by: authenticate по login/password; отдельный TOTP challenge; refresh с token rotation; validate с пустым успешным body; invalidate при logout. Endpoint config централизован, credentials отправляются только на fixed HTTPS authserver.ely.by; redirects и автоматические retries auth POST запрещены. Connect timeout 5 секунд, полный request 15 секунд, response до 128 КиБ. HTTP 429/5xx/network отличаются от rejected credentials; raw error bodies не возвращаются в UI.
- TOTP: opaque challenge ID, пароль/clientToken остаются backend-only. Время challenge — 120 секунд, максимум 5 попыток; cancel/logout очищают secret. Desktop supervisor очищает истёкший challenge даже при idle UI; при занятом auth worker очистка завершается после bounded request. UI очищает password/TOTP field при отправке; повторный ввод кода не передаёт пароль обратно в webview.
- ClientToken генерируется backend, response обязан вернуть тот же token; профиль/UUID берутся из provider response, а не из логина. Refresh не меняет profile UUID. Ошибка refresh не заменяет действующую сессию. Refresh и другие auth mutations single-flight: service владеет единственной сессией; dedicated auth mutex использует try_lock и возвращает AuthBusy, сетевой I/O выполняется на blocking worker. Workspace/UI thread не ждут этот I/O.
- Logout сначала удаляет local session/challenge, затем пытается отозвать remote token. Неудачный remote revoke возвращается отдельным notice при уже завершённом local logout. signout всех аккаунтов не используется.
- Session/LaunchIdentity не сериализуются; accessToken/clientToken — zeroizing SecretString с redacted Debug. IPC snapshots содержат только account summary/capabilities/revision/challenge ID. Пароль и TOTP на диск не записываются; без сохранения токены живут только до завершения процесса. Флажок сохранения использует Keychain / Windows Credential Manager / Linux Secret Service; plaintext fallback нет. Без флажка сессия остаётся только в памяти. Копии строк/буферов внутри JS/HTTP runtime не имеют гарантированного полного стирания.
- Mock type/input/registry branch компилируются только `debug_assertions` + opt-in `dev-auth`; ordinary debug и release не предлагают mock. `npm run dev:mock` включает явно помеченный Development / Mock. Ник: 3–16 ASCII letters/digits/underscore; UUID соответствует Java nameUUIDFromBytes UTF-8 `OfflinePlayer:<exact-name>` без namespace, с сохранением регистра.
- Tauri get_auth/login/continue_login/cancel_login/refresh_auth/restore_auth/logout доступны только bundled main window; payload deny_unknown_fields. Session/token import и provider URLs из IPC/config отсутствуют. Revision защищает mutations и UI от позднего initial response. AuthService.launch_identity перед будущим spawn повторяет build/provider gate, validate и при необходимости single-flight refresh; Ely.by identity требует verified agent. Play пока отключён.

## Системное хранилище токенов

Keyring 4.2 v1 выбирает только native Keychain (macOS), Credential Manager (Windows), Secret Service (Linux). Адаптер использует отдельный последовательный worker: Windows backend предупреждает о ненадёжном ordering при доступе к одной записи из разных threads. Namespace и одна credential reference фиксированы backend; произвольные ключи из IPC не принимаются. Unit/IPC tests используют injected memory fixture и не читают пользовательский OS store.

Сохраняется private schemaVersion=1 record только с Ely.by profile/accessToken/clientToken, без password/TOTP/agent URLs. Record никогда не используется как публичный DTO. Mock provider в saved record запрещён независимо от build. Restore требует явного действия, проверяет токен и обновляет его, получая profile binding из provider response; network failure сохраняет record для retry. Refresh перезаписывает один OS record; если OS write после server rotation не удался, свежая сессия остаётся в памяти с явным notice. Logout удаляет launcher entry и пытается выполнить invalidate; неудачи удаления/revoke сообщаются отдельно, local session уже удалена.

Настоящий macOS Keychain write/read/delete проверен только на новом disposable record `dev.launcher.desktop.smoke`; реальных credentials тест не касался. Live Windows/Linux credential store проверяется на соответствующих доступных машинах; headless CI не выдаётся за такой smoke. `cargo run -p launcher-desktop --example credential_smoke --locked` — opt-in native API test с очисткой своей записи. Saved-session roundtrip, rotation, store failure, TOTP remember intent и запрет forged profile покрыты HTTP/injected-store tests.

## Authlib Injector

Pinned official build **56**, version **1.2.8**, source commit `5437f5b465ee3f5be0a1e5e1ae3c50978114f6fb`, SHA256 `9c7f4343e6c82034958ffb48c14a2cb0c85928be7283103ce17da00c6d5a7b10`. JAR скачивается только по exact HTTPS URL официального artifact host, redirects запрещены. До 3 попыток, 30 секунд/request, 8 МиБ body limit, cancellation; SHA256 до atomic publication и при каждом jvm_arguments. Путь content-addressed, managed I/O no-follow.

Аргументы: reviewed `-Dauthlibinjector.noLogFile` и `-javaagent:<verified-path>=https://account.ely.by/api/authlib-injector`. Prefetched metadata не подставляется автоматически. Core предоставляет argv для будущего Launch Engine; JVM/agent/Minecraft execution не заявлены. JAR отсутствует в launcher packages и CI artifacts.

Лицензия AGPL-3.0-or-later с Authlib Injector exception: [полный текст](licenses/authlib-injector.txt), [notice/source review](AUTHLIB_INJECTOR.md). Условия распространения самого agent сохраняются; production distribution assessment остаётся release gate.

## Проверки

Финальная native matrix [42350741468](https://github.com/HerrChichikov/minecraft-rust-launcher/actions/runs/42350741468) на code commit `69fafb2595abd563270ba411b329687cbd50b593` — success: Frontend quality и все четыре desktop targets (Windows x64, Linux x64, macOS Intel и ARM64). Пройдены default/dev tests, production compile guards, release IPC restrictions, official install/reuse/loader smoke, pinned agent integrity smoke и packaging. Загружено 4 package artifacts и 4 text-receipt artifacts; IDs, digests и steps — [CI receipt](validation/phase-4-ci.json). Headless CI не проверяет реальный account/OS credential store.

Локально: 42 frontend tests; 48 core default / 50 core dev-auth tests (включая subprocess helpers), 12 desktop debug / 12 desktop release tests. Prettier/ESLint/strict TS/Vite, rustfmt, Clippy default/dev-auth, оба production compile guards и offline npm audit (0 vulnerabilities) — pass. Native Keychain disposable roundtrip и official agent download/corruption/repair/reuse — pass. В одном раннем parallel storage run однократно упала lease-reopen assertion; isolated и последующие полные runs прошли. Добавлена диагностика assertion; runtime lock implementation не менялась.

Ручной smoke до добавления OS persistence: development mock login/UUID/refresh/snapshot/logout, потеря memory-only сессии после restart, release Ely.by form без mock — pass. Пользователь самостоятельно ввёл credentials в native Release Launcher и сообщил «Вход выполнен успешно». Это подтверждает live authenticate со слов пользователя на build `5094c3e9f90feba59b7add49ad8c3b45d2f6b5d2`; использование TOTP, upstream refresh/revoke и сохранение реальной сессии не подтверждены. Идентификаторы аккаунта и secrets не включены в отчёт.

Финальная local сборка/хеш и границы GUI проверки — [local receipt](validation/phase-4-local.json). Повторный GUI smoke финального OS-storage build не завершён: инструмент управления native Launcher отвечает тайм-аутом. Успех compile/IPC/HTTP fixtures и disposable Keychain test не выдаётся за GUI roundtrip реального сохранённого account.

- Frontend: production password form без mock, dev capabilities, backend identity/reconnect/logout, password clearing, TOTP continuation/cancel, pending/sanitized errors, failed remote logout notice, stale response ordering, malformed/spoofed DTO.
- Core HTTP fixtures: authenticate, wrong password/TOTP, challenge expiry/cancel, refresh rotation/client binding, empty validate/invalidate, HTTP 503/302/malformed/oversized response, failed refresh preservation/local logout, validate/refresh at launch boundary без fallback.
- Core mock: Java UUID Notch golden vector/case/invalid nickname, refresh/validate/logout, owned session/revisions, Debug/DTO secrecy.
- Desktop: shared Rust/TS contract, actual IPC dev login/logout, production forged-provider/unknown-field/ACL denial, invalid real inputs/unknown challenge before network.
- Compile-fail release guards core + desktop; release IPC without feature.
- Official agent smoke: SHA256, same-size corruption rejected before argv, repair/reuse; без execution.

## Ограничения

Live authenticate подтверждён пользователем; TOTP/refresh/revoke на upstream, реальный saved-account restore и совместимость agent/Game 1.21.1/server ещё не подтверждены. Mock не считается доказательством real auth. Memory-only session не переживает restart; сохранённая сессия восстанавливается явной кнопкой, после validate+refresh на Ely.by. Desktop supervisor удаляет expired challenge, backend HTTP timeout ограничивает задержку очистки при активном request. AuthProvider — bounded synchronous contract, вызываемый на blocking worker, как InstallEngine; Tokio desktop используется для deadline supervisor.

Первый CI run 42346631288 не начал ни одного шага из-за account billing/spending restriction. Пользователь отдельно подтвердил перевод репозитория в Public; API подтвердил public visibility. Финальный run 42350741468 завершился успешно; старый blocked run не считается валидацией. Billing/payment settings не изменялись.

## Изменённые модули и критерии выхода

Core: `crates/launcher-core/src/auth/` (provider/service/Ely.by/mock/credentials/agent), ошибки и dependency features. Desktop: `src-tauri/src/commands/auth.rs`, `src-tauri/src/credentials.rs`, main-only capabilities/command manifest, examples. Frontend: `frontend/src/features/auth/`, contract/tests и интеграция панели. Также обновлены lockfiles, CI/guard scripts, ADR, security/interface/development docs и agent license notice.

| Критерий Phase 4 | Результат |
|---|---|
| AuthProvider и backend session owner | Реализованы; HTTP/service/IPC tests pass |
| Mock только opt-in development, production запрет backend | Реализован; Java UUID golden vector и оба compile guards pass |
| Ely.by flow согласован и реализован по документации | Password + TOTP выбран пользователем; fixture coverage pass; live authenticate подтверждён пользователем |
| Authlib Injector preparation и лицензия | Official pinned download/integrity/argv, license/source notice; execution относится к Phase 5 |
| OS credential storage без plaintext fallback | Native adapters + injected roundtrip/failure tests; macOS disposable native API smoke pass; live Windows/Linux остаются platform validation |
| Cross-platform build/tests | Все четыре native CI targets success |

Следующий функциональный этап — Phase 5 Launch Engine по отдельному запросу пользователя.
