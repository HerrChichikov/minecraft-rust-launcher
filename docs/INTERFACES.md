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
