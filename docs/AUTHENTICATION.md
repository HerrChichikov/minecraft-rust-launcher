# Авторизация

Статус Phase 4: пользователь выбрал password + TOTP; реализованы Ely.by authenticate/refresh/validate/invalidate, backend challenge и memory-only session, development mock и production guards. Реальные upstream authenticate/refresh/session join с пользовательским аккаунтом ещё не выполнялись. Public injector metadata Ely.by получены и сохранены в [снимке](research/ely-api-metadata.json).

## Два документированных семейства

[Minecraft auth Ely.by](https://docs.ely.by/en/minecraft-auth.html) описывает Yggdrasil-compatible вызовы к `https://authserver.ely.by`:

| Вызов | Существенные поля / результат |
|---|---|
| `POST /auth/authenticate` | username, password, clientToken, optional requestUser; accessToken, selectedProfile, availableProfiles |
| `POST /auth/refresh` | accessToken, clientToken, optional requestUser; новый token/profile |
| `POST /auth/validate` | accessToken; успех с пустым body, expired отдельно от network failure |
| `POST /auth/invalidate` | accessToken, clientToken; отзыв конкретного token |

`/auth/signout` отзывает все сессии и требует пароль: не используем его для обычного Logout. TOTP поддерживается как `password:token` после соответствующего challenge. Пароль и TOTP не сохраняются. Profile name/UUID берём из provider response, не генерируем из логина. Информация о textures/profile доступна через документированный `GET /session/profile/{uuid}`. Пустые успешные responses не пытаемся десериализовать как обязательный JSON.

[OAuth Ely.by](https://docs.ely.by/en/oauth.html) отдельно документирует authorization code: `https://account.ely.by/oauth2/v1`, token exchange/refresh — `https://account.ely.by/api/oauth2/v1/token`, account info — `https://account.ely.by/api/account/v1/info`. Нужны `account_info` и `minecraft_server_session`; `offline_access` выдаёт refresh token. Это не произвольный OAuth login: без game scope токен нельзя считать Minecraft session. Документированный exchange требует client_secret. Поддержка public desktop client/PKCE в изученных материалах не установлена; secret в desktop binary не является секретом.

## Выбор для Phase 4

| Вариант | Преимущества | Ограничения |
|---|---|---|
| **Yggdrasil password + TOTP, рекомендуем для первого реального provider** | Документирован, не требует нового сервиса или embedded client_secret | Лаунчер обрабатывает пароль; требуется хороший secret lifecycle и договорённость с владельцем продукта |
| OAuth через подтверждённый native/public flow | Системный браузер, пароль не проходит через launcher | До подтверждения PKCE/public client и redirect registration нельзя считать реализуемым по имеющимся docs |
| OAuth через собственный broker | Secret хранится на сервере, browser login | Новая инфраструктура, эксплуатация и threat model; за пределами согласованного MVP |

Решение пользователя 2026-11-07: реализовать password + TOTP. Broker сейчас не проектировать как обязательный фундамент. Не смешиваем Yggdrasil clientToken и OAuth refresh_token.

## Provider и сессии

`AuthProvider` из [контрактов](INTERFACES.md) отделяет flow от installer. `ElyByAuthProvider` получает provider config: stable ID, auth base, injector API root, allowed hosts и agent policy. API base не разбросан по коду. В будущем другой Yggdrasil endpoint меняет provider config/implementation, не Minecraft installer.

Yggdrasil clientToken генерируется backend и хранится вместе с credentials для аккаунта/provider. Access token живёт в backend; credential store обновляется после успешной refresh. Refresh single-flight, чтобы две операции не использовали устаревшие credentials. Validate перед запуском; invalid/expired требует refresh либо reauth, сетевой сбой не подменяется неверным паролем. Для долгой установки session проверяется повторно перед spawn. Точный TTL не угадываем из сторонних примеров.

`AccountRecord` хранит только credential reference. macOS Keychain, Windows credential store, Linux Secret Service доступны через adapter. Если secure store недоступен/заблокирован — вход только на текущую сессию в памяти либо явный отказ в «Запомнить меня». Plaintext fallback нет. Logout удаляет local secrets и пытается отозвать remote token; ошибку отзыва показываем отдельно.

## Authlib Injector

[Документация Ely.by](https://docs.ely.by/en/authlib-injector.html) требует agent и самостоятельный login в launcher. [Launcher specification](https://yushijinhun.github.io/authlib-injector/en/launcher-technical-specification.html) задаёт `-javaagent:<verified-path>=<api-root>` и optional `-Dauthlibinjector.yggdrasil.prefetched=<base64-public-metadata>` до main class. `${auth_access_token}` — token, `${auth_uuid}` — UUID без дефисов, `${user_type}` — `mojang`. GameRun UUID и clientToken не передаются вместо access token.

API root можно разрешить через `X-Authlib-Injector-API-Location`; это discovery, не бесконтрольное доверие новому host. Для MVP используем заранее согласованный HTTPS config. Наблюдаемый `https://account.ely.by/api/authlib-injector` возвращает `serverName=Ely.by`, `feature.enable_profile_key=true` и public signature key. Это подтверждает наличие adapter metadata, но не доказывает совместимость token от другого base URL. Связку authserver login + выбранный injector root проверяем на Phase 4 test account и реальном 1.21.1; не изобретаем token exchange между ними.

[Официальный download API](https://yushijinhun.github.io/authlib-injector/en/get-authlib-injector.html): `GET /artifacts.json`, `GET /artifact/{build_number}.json`, либо discovery `/artifact/latest.json`. Descriptor содержит `download_url` и `checksums.sha256`. В исследовании latest — build 56, version 1.2.8. Agent **не скачан и не запущен**. В продукте используем reviewed/pinned build descriptor и SHA256, не обновляем executable автоматически на произвольный latest. Перед каждым запуском проверяем hash; download/redirect host только из approved policy.

[COPYING](https://github.com/yushijinhun/authlib-injector/blob/develop/COPYING.md) и [полный текст лицензии](https://github.com/yushijinhun/authlib-injector/blob/develop/AGPLv3-with-authlib-injector-exception.txt) указывают AGPL v3-or-later с исключением для agent usage/unmodified binary inclusion/IPC. Это не разрешение игнорировать условия распространения самого agent. До выбора конкретного binary фиксируем его version/source commit, license text, notices и способ доступа к corresponding source. В Phase 0 agent не включается в приложение.

Launcher не выполняет за Minecraft `join/hasJoined`: это протокол game authlib ↔ auth server и game server ↔ auth server. Наш сервер должен поддерживать согласованную Ely.by-интеграцию; один agent на клиенте не делает произвольный online-server совместимым. Secure profile/chat key compatibility 1.21.1 проверяется с сервером, не отключением проверок без разбора.

## Mock Auth

Только `cfg(all(debug_assertions, feature = "dev-auth"))`, feature не default. Release + dev-auth вызывает compile_error; никакого runtime env flag, обходящего это. Mock registry/type/login branch не компилируются в production. В UI используются build capabilities backend, а не только `import.meta.env.DEV`.

Ник: 3–16 ASCII letters/digits/underscore, ошибка для остальных. Стабильный UUID — Java-compatible nameUUIDFromBytes от UTF-8 `OfflinePlayer:<exact-name>` с MD5 и UUIDv3 bits, без добавления UUID namespace. Регистр сохраняется; это development identity, не remote UUID и не доказательство входа. Token — явно тестовое backend value, не логируется. UI постоянно показывает «Development / Mock». Сессия не восстанавливается из production config.

Production без реального provider блокирует Play с AuthUnavailable. Нужны negative release tests, включая подделанный providerId в IPC и сохранённой config. [ADR-004](DECISIONS/004-auth-and-production.md) фиксирует release gate.

## Реализовано в Phase 4

См. [review](PHASE_4_REVIEW.md) и [ADR-010](DECISIONS/010-password-totp-auth.md). AuthProvider имеет bounded synchronous методы; desktop запускает их на blocking worker. AuthService владеет memory-only session, revision и backend challenge. get_auth/login/continue_login/cancel_login/refresh_auth/logout имеют main-only ACL. Password POST без redirects/retries; timeout 15 секунд, body до 128 КиБ, ошибки sanitized. Challenge 120 секунд/5 попыток, deadline supervisor очищает secret даже при idle UI. ClientToken binding и profile UUID проверяются при rotation. launch_identity validate/refresh повторяется перед будущим spawn; mock fallback отсутствует.

AccountRecord/OS CredentialStore выше описывают будущую persistence. Сейчас нет OS adapters/«Запомнить меня» или plaintext fallback; restart требует нового входа. Logout удаляет local secrets до попытки remote invalidate, remote failure — отдельный notice. Собственные backend buffers zeroizing; гарантии полного стирания копий внутри JS/HTTP runtime нет.

Pinned agent downloader/argv adapter, license/source/notice review и SHA256 smoke — в [AUTHLIB_INJECTOR.md](AUTHLIB_INJECTOR.md). Java agent ещё не выполнялся; real session/game/server smoke не заявлены.
