# Модель данных

Статус: проект контрактов. Все JSON с `schemaVersion`; миграция пишет новую версию атомарно, неизвестная более новая схема даёт явную ошибку без перезаписи.

| Тип | Поля и инварианты |
|---|---|
| `ProfileId` | UUID v4, генерирует backend; не имя папки от пользователя |
| `Profile` | id, displayName, minecraftVersion, relativeGameDir, settings, schemaVersion; в MVP version=1.21.1 |
| `LauncherSettings` | dataRoot, activeProfileId, selectedAccountId; без токенов |
| `LaunchSettings` | minRamMiB, maxRamMiB, runtimeSelection, extraJvmArgs: список строк |
| `RuntimeSelection` | Managed или Custom с абсолютным executable path; custom проверяется перед каждым запуском |
| `Platform` | OS, native host arch, process arch, selected JVM arch, OS version для rules; это разные признаки |
| `VersionMetadata` | id/type/mainClass/javaVersion/arguments/libraries/downloads/assetIndex/logging; сохраняем source hash |
| `ResourceSpec` | resource ID, official HTTPS URL, expected size/hash, validated relative destination, role |
| `InstallPlan` | version metadata hash, platform/features, resource specs без дублей, runtime manifest hash |
| `InstalledSnapshot` | plan hash, verified resource references, runtime ID, native generation; не постоянный флаг «установлено» |
| `VerificationRecord` | path key, expected hash/size, observed size, mtime_ns, ctime/file identity где доступны, verification time, schema version |
| `RuntimeDescriptor` | component, major, manifest hash, platform key, relative executable, installation generation |
| `AccountRecord` | local account ID, provider ID, remote profile UUID/name, credential reference; без пароля/access token |
| `Session` | provider ID, profile, access token secret, clientToken secret при Yggdrasil, optional expiry; только backend |
| `LaunchIdentity` | name, remote UUID, secret token, userType, optional provider-specific clientId/xuid, agent requirement |
| `Operation` | random ID, profile ID, stage, completed/total bytes, sequence, optional error |
| `GameRun` | random local run UUID, profile ID, process handle/PID, startedAt, exit outcome; PID не является авторизацией |

Profile UUID, local AccountId, Yggdrasil clientToken и GameRun UUID имеют разные типы. Их нельзя подменять друг другом. UUID провайдера не является доказательством владения аккаунтом.

`Secret` не реализует раскрывающий `Debug`/`Serialize`; очистка памяти по возможности при освобождении. `Session` и `LaunchSpec` не являются IPC DTO. UI получает `AccountSummary { accountId, providerName, playerName, profileUuid, isDevelopment }`.

## Состояния

Installation state: `Initializing → Checking → Downloading/Repairing → Extracting → Ready`; стадии с нулём работы пропускаются. Launch state: `Ready → Launching → Running → Ready/Error`. Отмена до spawn: `Cancelling → Cancelled`; ошибки содержат стадию и recoverability. Повтор переводит через Checking. Отмена installation не завершает запущенную игру.

Процент этапа считаем из известных totals; общий прогресс становится determinate после планирования. Хеширование и извлечение показывают собственные счётчики. Скорость считается по фактически полученным байтам и монотонному времени; ожидание HTTP не изображается как продвижение.

Все значения с числовыми размерами проверяются на overflow до вычисления byte counts. Длительности и timeout — monotonic; timestamps нужны для диагностики, не для доказательства целостности.
