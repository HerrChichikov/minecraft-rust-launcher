# Запуск игры и процесс

Phase 5 реализована. Подготовка использует [официальные метаданные 1.21.1](https://piston-meta.mojang.com/v1/packages/82ba6501da174a665c1787775f946487422f2432/1.21.1.json). Результаты запуска и ограничения описаны в [отчёте](PHASE_5_REVIEW.md).

## LaunchSpec

`PreparedLaunch` проверяет сохранённые настройки, повторно проверяет и при необходимости восстанавливает файлы через установщик, а для Ely.by готовит закреплённый Authlib Injector. Перед spawn Tauri проверяет, что версия сессии не изменилась, и запрашивает свежий `LaunchIdentity`. Core собирает `LaunchSpec` с отдельными аргументами и скрытым Debug. Java запускается напрямую через [Tokio Command](https://docs.rs/tokio/latest/tokio/process/index.html).

Порядок аргументов: проверенный agent для Ely.by, RAM и разрешённые JVM options, JVM arguments из metadata, main class, game arguments из metadata. Classpath сохраняет порядок библиотек и native JAR, затем добавляет client JAR. Записи относительны к фиксированной игровой папке профиля, чтобы не повторять длинный абсолютный путь в командной строке Windows. Разделитель — `;` на Windows и `:` на Unix. Пробелы и Unicode передаются в отдельных аргументах.

Metadata JVM arguments обязательно сохраняют macOS `-XstartOnFirstThread`; startup first thread и ARM natives проверяются отдельно. Не используем `java -jar client.jar`: metadata задаёт main class и classpath.

## Подстановка

Сначала rules, затем подстановка только в выбранных arguments. Известные `${...}` разбираются по шаблонам, значение подставляется внутрь одного argv item без повторной интерпретации как shell. Никакого splitting после подстановки, включая пустые значения. Оставшийся неизвестный placeholder вызывает InvalidLaunchArguments до spawn.

| Фактический placeholder | Источник |
|---|---|
| `auth_player_name` | выбранный remote/mock profile name |
| `auth_uuid` | profile UUID без дефисов |
| `auth_access_token` | secret только из AuthProvider |
| `user_type` | provider mapping; для authlib-injector `mojang` по его спецификации |
| `clientid`, `auth_xuid` | пустые строки в Mock и Ely.by, так как оба провайдера не используют Microsoft identity |
| `version_name`, `version_type` | metadata id/type |
| `game_directory` | абсолютный isolated profile game dir |
| `assets_root`, `assets_index_name` | shared assets root, имя проверенного индекса `id-sha1` |
| `natives_directory` | writable per-run directory |
| `launcher_name`, `launcher_version` | build info |
| `classpath` | library paths + client, соединённые platform delimiter |
| `resolution_width`, `resolution_height` | только при включённом custom resolution; вне MVP |
| `quickPlayPath`, `quickPlaySingleplayer`, `quickPlayMultiplayer`, `quickPlayRealms` | только при включённых соответствующих features; вне MVP |

В snapshot 21 уникальный game/JVM placeholder. `${classpath_separator}` не встречается в этой ревизии, но если появится в поддержанном metadata, его смысл — platform delimiter; `${path}` относится отдельно к logging config. `auth_session` не встречается: не добавляем выдуманный session UUID в game args.

Пустые `clientid` и `auth_xuid` проверены реальным development-запуском. Вход в Ely.by подтверждён в Phase 4, а запуск Authlib Injector в игровой JVM проверен в Phase 5 с синтетической сессией. Совместимость Ely.by с игровым сервером требует отдельной проверки.

## JVM настройки

Backend проверяет положительные MiB, min ≤ max, разумную верхнюю границу по памяти машины и overflow. RAM flags создаёт launcher. `extraJvmArgs` — массив аргументов, UI вводит по одному на строку; без неоднозначного shell parser. Reject NUL/newline, `@argfile`, main-class/JAR selection, `-cp/-classpath/--class-path`, `-jar`, module-path/main-module switches, `-Xms/-Xmx` overrides, `-javaagent/-agentlib/-agentpath`, опасные overrides auth/natives/logging properties. MVP разрешает ограниченные диагностические/GC JVM options с валидацией; список расширяется явно, не пропускает любой unknown switch.

Пользовательский custom Java — явное локальное исполнение выбранного пользователем executable. Автоматический Java Agent допускается только из trusted descriptor с SHA256 и reviewed version; пользовательские JVM args не обходят эту политику. Launcher удаляет переменные окружения, которыми можно подмешать параметры JVM и нативные библиотеки. Для своей Java нужен совместимый с HotSpot runtime 21. Перед запуском отклоняются конфликтующие настройки сборщика мусора.

## Жизненный цикл

Одна игра за раз в приложении. Core удерживает дескриптор дочернего процесса и файловые блокировки до подтверждённого выхода. Профиль и версия его настроек проверяются в backend. Две параллельные попытки запуска не получают одну и ту же блокировку. Нативные библиотеки извлекаются в новую папку внутри `profiles/<id>/runs/`, которую лаунчер очищает после выхода.

`start_game` возвращает состояние подготовки. Приложение публикует `launcher://game` только для main window. `get_game` возвращает последний статус и монотонный sequence для переподключения. UI показывает подготовку, запуск, работу и завершение, в том числе код ненулевого выхода. `stop_game` принимает только идентификатор текущего запуска. До spawn он отменяет подготовку. После spawn он принудительно завершает принадлежащий лаунчеру процесс Java и ждёт его выхода. Для сохранения мира нужно выйти через меню игры. Пока запуск или игра активны, закрытие Launcher блокируется.

Перед spawn записывается `profiles/<id>/runs/active.json`. После подтверждённого выхода запись удаляется. При аварии самого Launcher запись блокирует повторный запуск профиля, пока оператор не убедится, что прежняя игра закрыта. PID в записи служит подсказкой и не используется для завершения чужого процесса. Для восстановления закройте Launcher и убедитесь, что прежний Minecraft завершён, затем переименуйте только `active.json` нужного профиля в `active.recovered.json` и снова откройте Launcher. Перезагрузка машины позволяет удостовериться, что прежний процесс завершился, если его состояние невозможно проверить.

## Журналы и токены

Java stdout и stderr читаются параллельно. Хранятся до 128 последних строк. Длинные строки отбрасываются, строки с исходным токеном целиком заменяются, управляющие символы удаляются. По завершении пишется один локальный `profiles/<id>/logs/launcher-last-run.json`. В IPC попадают только состояние, код выхода и признак сохранения журнала. Собственная конфигурация Log4j оставляет один консольный вывод, Authlib Injector отключает отдельный файл, JVM отключает fatal report и core dump.

Протокол Minecraft передаёт access token в отдельном аргументе `--accessToken`. ОС может показывать его локальному наблюдателю с доступом к процессу. Подменённая пользователем Java и диагностические средства ОС находятся за границей доверия приложения. [Документация Java 21](https://docs.oracle.com/en/java/javase/21/docs/specs/man/java.html), [документация Ely.by](https://docs.ely.by/en/minecraft-auth.html).
