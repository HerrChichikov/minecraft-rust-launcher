# Запуск игры и процесс

Статус: проект Phase 5. Источник — [официальные metadata 1.21.1](https://piston-meta.mojang.com/v1/packages/82ba6501da174a665c1787775f946487422f2432/1.21.1.json). Реального запуска ещё не было.

## LaunchSpec

Core строит `executable: PathBuf`, `args: Vec<OsString>`, `cwd: PathBuf`, очищенную environment policy, log targets и redaction context. Это внутренний тип без раскрывающего Debug/Serialize. Запуск через [Tokio Command](https://docs.rs/tokio/latest/tokio/process/index.html), без shell, `sh -c`, `cmd /C` и shell quoting.

Порядок: validated launcher RAM/options → metadata JVM arguments с rules → logging argument → approved provider agent arguments → metadata main class (`net.minecraft.client.main.Main`) → metadata game arguments с rules. Classpath — ordered allowed library artifact paths, включая native JAR, затем client JAR; `;` на Windows, `:` на Unix. Path с classpath delimiter в компоненте отклоняется: такие имена нельзя однозначно представить JVM classpath. Пробелы/Unicode поддерживаются отдельными argv items.

Metadata JVM arguments обязательно сохраняют macOS `-XstartOnFirstThread`; startup first thread и ARM natives проверяются отдельно. Не используем `java -jar client.jar`: metadata задаёт main class и classpath.

## Подстановка

Сначала rules, затем подстановка только в выбранных arguments. Известные `${...}` разбираются по шаблонам, значение подставляется внутрь одного argv item без повторной интерпретации как shell. Никакого splitting после подстановки, включая пустые значения. Оставшийся неизвестный placeholder вызывает UnresolvedPlaceholder до spawn.

| Фактический placeholder | Источник |
|---|---|
| `auth_player_name` | выбранный remote/mock profile name |
| `auth_uuid` | profile UUID без дефисов |
| `auth_access_token` | secret только из AuthProvider |
| `user_type` | provider mapping; для authlib-injector `mojang` по его спецификации |
| `clientid`, `auth_xuid` | provider-specific значения; у Mock/Ely.by не подделываем Microsoft identity, исходный вариант — пустые строки с сохранением argv pair; проверить smoke |
| `version_name`, `version_type` | metadata id/type |
| `game_directory` | абсолютный isolated profile game dir |
| `assets_root`, `assets_index_name` | shared assets root, metadata assetIndex.id |
| `natives_directory` | writable per-run directory |
| `launcher_name`, `launcher_version` | build info |
| `classpath` | library paths + client, соединённые platform delimiter |
| `resolution_width`, `resolution_height` | только при включённом custom resolution; вне MVP |
| `quickPlayPath`, `quickPlaySingleplayer`, `quickPlayMultiplayer`, `quickPlayRealms` | только при включённых соответствующих features; вне MVP |

В snapshot 21 уникальный game/JVM placeholder. `${classpath_separator}` не встречается в этой ревизии, но если появится в поддержанном metadata, его смысл — platform delimiter; `${path}` относится отдельно к logging config. `auth_session` не встречается: не добавляем выдуманный session UUID в game args.

Значения `clientid`/`auth_xuid` для альтернативной авторизации и современная authlib совместимость — проверяемая гипотеза, не доказанная JSON. Если game/provider требуют другое поведение, исправляется provider mapping с тестом; не подставляется фиктивный Xbox ID.

## JVM настройки

Backend проверяет положительные MiB, min ≤ max, разумную верхнюю границу по памяти машины и overflow. RAM flags создаёт launcher. `extraJvmArgs` — массив аргументов, UI вводит по одному на строку; без неоднозначного shell parser. Reject NUL/newline, `@argfile`, main-class/JAR selection, `-cp/-classpath/--class-path`, `-jar`, module-path/main-module switches, `-Xms/-Xmx` overrides, `-javaagent/-agentlib/-agentpath`, опасные overrides auth/natives/logging properties. MVP разрешает ограниченные диагностические/GC JVM options с валидацией; список расширяется явно, не пропускает любой unknown switch.

Пользовательский custom Java — явное локальное исполнение выбранного пользователем executable. Автоматический Java Agent допускается только из trusted descriptor с SHA256 и reviewed version; пользовательские JVM args не обходят эту политику. Environment убирает JVM option injection.

## Lifecycle

Process supervisor владеет Child до `wait`, а profile lock держится до terminal outcome. stdout/stderr читаются одновременно, chunks имеют верхнюю границу даже без newline. При медленном UI старые log messages в ring buffer вытесняются, file sink продолжает ограниченную ротацию; сообщение о пропусках не скрывается. Обычный exit code 0 возвращает Ready; crash/nonzero — Error с safe diagnostics; spawn error освобождает lock. UI не ждёт процесс в invoke.

У Minecraft client нет универсального IPC «сохранить и выйти». Основной корректный выход — меню игры. `stop_game` предлагает принудительное завершение с предупреждением о несохранённом мире; POSIX SIGTERM с коротким timeout затем kill, Windows controlled termination текущего child. Не выдаём это за гарантированное graceful save. Не завершаем чужие PID по сохранённому числу; проверяем принадлежащий run handle. Platform-specific process-tree cleanup добавляется только для принадлежащего JVM tree и проверяется отдельно.

Закрытие окна при Running предлагает оставить лаунчер открытым или завершить игру и выйти; по умолчанию остаётся открытым. Это позволяет не проектировать фоновый daemon/reattach в MVP. Аварийное завершение самого лаунчера не означает, что JVM завершилась: после рестарта выполняем reconciliation живого run по PID+start identity, не разрешаем дубликат при неопределённости. Не обещаем управление осиротевшим процессом без проверки identity. При невозможности безопасно подтвердить состояние предлагаем пользователю закрыть игру.

Access token требуется в game argv и может быть виден локальным процессам/администратору. Не логируем полный LaunchSpec; scrub stdout/stderr, HTTP и crash excerpts до записи. Скрыть токен от всех локальных наблюдателей при стандартном Minecraft launch нельзя. [Rust Command](https://doc.rust-lang.org/std/process/struct.Command.html) устраняет shell interpretation, но не эту особенность протокола.
