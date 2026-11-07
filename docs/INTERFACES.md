# Основные интерфейсы

Статус: в Phase 1 созданы Rust traits и базовые модели для profiles/installer/runtime/auth/process/storage; без concrete service implementations. Ниже описана полная целевая спецификация, которая будет уточняться на соответствующих этапах. Core использует `Result<T, LauncherError>` и native async contracts; `async-trait` не добавлен. Dynamic plugins отсутствуют.

Фактически работающий IPC — `get_snapshot()` без пользовательских параметров: schemaVersion, phase=foundation, версии, binary platform, developmentBuild и feature availability. Core `ApplicationService::snapshot` не зависит от Tauri; `SnapshotDto` существует только в desktop adapter. Все feature flags сейчас false. Wire contract проверяется одним JSON fixture в Rust и TypeScript. Другие IPC commands из таблицы ещё не зарегистрированы.

## Core

| Модуль / операция | Вход → выход | Контракт |
|---|---|---|
| `ProfileRepository.create/load/save/list` | typed ID / settings → Profile | Schema/path validation, atomic save, без credentials |
| `MetadataRepository.resolve(version)` | version ID → verified metadata | По official manifest; content hash и cache provenance |
| `RuleEvaluator.allows(rules, context)` | OS/JVM arch/features → bool | Ordered rules; неизвестные условия не становятся безусловным allow |
| `Installer.plan(profile, platform)` | Profile → InstallPlan | Только metadata, без побочного запуска executable |
| `Installer.prepare(plan, mode, cancel, events)` | Plan + Quick/Full → InstalledSnapshot | Проверка, bounded download, atomic commits; cancel идемпотентен |
| `RuntimeManager.ensure(javaVersion, platform, cancel)` | component/major → VerifiedRuntime | Не использует PATH Java, проверяет platform и executable |
| `RuntimeManager.probe(customExecutable)` | validated path → JavaInfo | Без shell, очищенное env, timeout и bounded output |
| `AuthProvider.capabilities()` | → supported auth steps | UI не угадывает login flow |
| `AuthProvider.authenticate(AuthInput)` | credentials/challenge → AuthOutcome | Authenticated(Session) или Challenge; нет ложного успеха |
| `AuthProvider.refresh(Session)` | → rotated Session | Атомарное обновление credential store; single-flight по account |
| `AuthProvider.validate(Session)` | → Valid / ReauthRequired | Transport error отдельно от неверного токена |
| `AuthProvider.logout(Session)` | → LogoutOutcome | Отзыв token где поддержан, удаление local secrets; remote failure отдельно |
| `AuthProvider.launch_identity(Session)` | → LaunchIdentity | Без зависимости от installer; provider задаёт требование agent |
| `CredentialStore.read/write/delete` | opaque reference ↔ secret | OS implementation, ошибки недоступности, no plaintext fallback |
| `LaunchPlanner.build(metadata, installed, identity, settings)` | → LaunchSpec | Validated executable/cwd/argv/env; reject unresolved placeholders |
| `ProcessSupervisor.spawn(spec)` | → RunHandle | Direct spawn, stdout/stderr readers, single authoritative wait owner |
| `ProcessSupervisor.request_stop(runId)` | → StopOutcome | Только принадлежащий приложению run, platform semantics из launch docs |
| `ApplicationService.play/prepare/repair/cancel/snapshot` | typed requests → operation/state | Координация locks, auth, events; UI не управляет шагами отдельно |

`AuthInput`: DevelopmentNickname; Password с optional TOTP; в будущем BrowserCallback. Неподдержанный вариант — `UnsupportedAuthFlow`. Challenge имеет короткоживущий opaque ID, никакого повторного возврата пароля в UI. Поведение transport через общий reusable HTTP client; HTTP test doubles подключаются через base URL policy только в test build.

## IPC

Имена ниже — проект команд, а не утверждение о наличии Tauri API:

| Command | Вход | Ответ |
|---|---|---|
| `get_snapshot` | optional profileId | sanitized state + sequence |
| `list_profiles`, `create_profile` | validated displayName для create | summaries / created profile |
| `save_settings` | typed settings | validated settings; Busy при изменении root во время операции |
| `get_auth_capabilities` | providerId | supported UI flow, development flag |
| `login`, `continue_login` | typed auth request / challenge | account summary или challenge |
| `logout` | accountId | sanitized outcome |
| `prepare`, `repair`, `play` | profileId | operationId, current state |
| `cancel_operation` | operationId | accepted/already terminal |
| `stop_game` | runId | accepted/already exited |

Mock input/registration отсутствует в release backend. Для login по паролю secret передаётся один раз в invoke, очищается frontend state после ответа, IPC/body tracing отключён. Пароль не возвращается в ответе.

`launcher://state` и `launcher://progress` — window-targeted sanitized events. Оболочка для чтения game logs — bounded Tauri channel с явной подпиской. Названия событий внутренние, не стандарт Tauri. `get_snapshot` нужен при старте/reconnect; постоянного polling нет.

DTO содержат `schemaVersion`, IDs, stage, progress, sequence. Errors: `code`, локализуемый `messageKey`, safe parameters, `retryable`, `correlationId`. Внутренний source error остаётся backend и проходит redaction. По [документации Tauri](https://v2.tauri.app/develop/calling-frontend/) events не имеют той же granular capability-защиты, что commands, поэтому секреты в них не передаём.

## Ошибки

Группы `Metadata`, `UnsupportedVersion/Platform`, `UnsafePath`, `Network`, `HashMismatch`, `DiskFull`, `PermissionDenied`, `Cancelled`, `ProfileBusy`, `RuntimeUnavailable/Incompatible`, `AuthRejected/Challenge/Expired`, `CredentialStoreUnavailable`, `InvalidArguments/UnresolvedPlaceholder`, `SpawnFailed`, `GameExitedUnexpectedly`, `StorageSchema`. У ошибки есть typed context без токенов. Offline и неверный пароль не сливаются в один ответ.

## Реализовано в Phase 2

К `get_snapshot` добавлены `get_workspace`, `create_profile`, `select_profile`, `save_profile`, `switch_data_root`. Все доступны только bundled main window; нет frontend fs/http/shell permission. FS I/O выполняется в spawn_blocking; Domain errors превращаются в code/messageKey/retryable без raw paths/source messages.

`WorkspaceSnapshot` schemaVersion=1: rootId, dataRoot, configurationRevision, revision (selection), activeProfileId, profiles, policy.maxRamMib. Общий Rust/TS fixture: tests/contracts/workspace.v1.json. ProfileId — UUID v4, backend-owned. Profile имеет свою revision.

Все mutation payloads вложены в `request` и deny_unknown_fields. Create: rootId/name. Select: rootId/profileId/expectedRevision selection. Save: rootId/profileId/expectedRevision profile/name/settings. Switch: rootId/expectedRevision configuration/path (absolute). Ответ — полностью прочитанный WorkspaceSnapshot; UI обновляет состояние после backend success. dataRoot — единственный явный пользовательский path authority; произвольных managed file paths UI не передаёт.

Коды ошибок: storage_busy, corrupt_document, unsupported_schema, revision_conflict, unsafe_path, invalid_settings, durability_failed, storage_failed. При durability_failed файл уже может быть опубликован; требуется refresh. Неизвестный код сводится к storage_failed. Profiles feature означает наличие реализации, а не успешное открытие любого повреждённого пользовательского root. Foundation bootstrap DTO phase="foundation" сохранён для совместимости протокола; этап UI — 02.

ProfileRepository теперь sync contract для bounded локального I/O, вызываемого на blocking worker; будущие сетевые installer/auth/runtime contracts остаются async. Secret Session/LaunchSpec по-прежнему не являются IPC DTO.

## Phase 3

InstallEngine — синхронный concrete service, вызываемый только с blocking worker; прежние неиспользуемые InstallPlan/InstalledVersion/async Installer sketches заменены реальными VersionPlan/PreparedInstallation. plan получает Profile + RuleContext; prepare получает VersionPlan/platform/EventSink/RuntimeSelection. Operation ID и Cancellation принадлежат backend. ResourceStore удерживает root/profile/installation leases и не выдаёт UI filesystem handles.

start_install request: rootId/profileId, без URL/executable/path; get_installation: current in-memory operation или null; cancel_install request: operationId, неизвестный ID запрещён. InstallationStatus schemaVersion=1 содержит operationId/profileId/profileName/stage/sequence/completedBytes/totalBytes/errorCode/terminal. Events launcher://installation направлены только main, не чаще 10 Hz для progress; terminal outcome всегда отправляется. Snapshot после reconnect восстанавливает последнюю операцию этого процесса; persisted installation descriptor не считается доказательством текущей integrity.

## Phase 4

get_auth без аргументов. login request={expectedRevision,providerId,input}, optional remember=false, где input — tagged Password {flow:"password",login,password} или debug+dev-auth DevelopmentNickname {flow:"development_nickname",nickname}. continue_login={expectedRevision,challengeId,totp}; cancel_login/refresh_auth/restore_auth/logout={expectedRevision}. Все DTO deny_unknown_fields; Session/token import/URL authority отсутствуют.

AuthSnapshot schemaVersion=1: revision, providers, public account|null, challenge {id,providerId,expiresInSeconds}|null, persistence=memory_only|system_credential_store, rememberSupported boolean, notice=null|remote_logout_failed|credential_store_unavailable|credential_delete_failed|logout_cleanup_failed. Shared Rust/TS fixture — tests/contracts/auth.v1.json. Memory-only состояние переживает remount webview; optional сохранение использует OS adapter, restore — только по явному запросу. UI очищает password/TOTP, хранит только public summary, отбрасывает stale revision. Deadline supervisor удаляет expired backend challenge.

AuthProvider — bounded sync contract (как InstallEngine), вызываемый на blocking worker; caps/authenticate/refresh/validate/logout/launch_identity. Dedicated auth mutex try_lock обеспечивает single-flight; get_auth быстро возвращает AuthBusy во время network request. AuthService владеет session, перед launch validate/refresh проверяет provider/build gate; caller не передаёт Session. Ely.by identity содержит requires_ely_by_agent=true. agent::ensure/VerifiedAgent.jvm_arguments пересчитывают pinned SHA256; spawn относится к Phase 5.

Auth errors: auth_unavailable, auth_rejected, auth_network, auth_protocol, auth_challenge_expired, auth_busy, authentication_required, unsupported_auth_flow; existing invalid_settings/revision_conflict остаются sanitized. Ошибка remote invalidate не отменяет local logout — отдельный notice.

CredentialStore.read/write/delete внедряется в AuthService; desktop native implementation — keyring 4.2 v1 на одном последовательном worker thread. Ни reference, ни serialized secret record не поступают в IPC. Restore выполняет validate+refresh до account summary; refresh/store failure не теряет новую in-memory session и сообщает memory fallback.
