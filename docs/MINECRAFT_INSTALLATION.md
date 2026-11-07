# Установка Minecraft

Статус: реализован Phase 3 installer; локальная реальная установка/повтор/repair прошли, native CI четырёх targets пройден. Реальные результаты — [Phase 3 review](PHASE_3_REVIEW.md). Источники: [official manifest](https://piston-meta.mojang.com/mc/game/version_manifest_v2.json), [metadata 1.21.1](https://piston-meta.mojang.com/v1/packages/82ba6501da174a665c1787775f946487422f2432/1.21.1.json), локальные [снимки](research/provenance.json).

## Цепочка доверия и план

1. GET version manifest по настроенному official HTTPS bootstrap URL. Выбрать `id=1.21.1`, без использования `latest.release` как версии профиля.
2. Получить version JSON по `url`, проверить SHA1 из manifest до разбора. Повторно выпущенные metadata могут менять hash даже для того же version ID; установленный профиль хранит metadata revision, обновление metadata требует нового плана проверки.
3. Проверить `id`, `mainClass`, `downloads.client`, `arguments`, `libraries`, `assetIndex`, `javaVersion`, optional `logging.client`. Неподдерживаемую форму отклонить явно, не придумать defaults для обязательных полей.
4. Построить InstallPlan: client JAR; разрешённые библиотеки; asset index и объекты; logging config; runtime manifest и его файлы. Metadata остаётся единственным источником URLs, hashes и Java major/component.
5. Проверить локальные файлы, загрузить недостающее/повреждённое, подготовить runtime/natives; сохранить установочный snapshot лишь после завершения всех зависимостей. Наличие snapshot не отменяет check перед Play.

Bootstrap URLs и `https://resources.download.minecraft.net` — централизованная upstream-конфигурация. Из asset hash формируется стандартный URL `/<first-two-hex>/<hash>`, а не URL из произвольного имени asset. Это формат content-addressed сервиса, не hardcode списка ресурсов. Client и libraries скачиваются только по metadata; Maven dependency resolution и сторонний dependency solver не нужны.

## Rules

Без `rules` элемент разрешён. При наличии rules начальное решение — disallow; применяем правила последовательно, последнее совпавшее задаёт результат. Поля внутри условия AND; `os.name`, regex `os.version`/`os.arch` и feature flags учитываются вместе. macOS называется `osx`; runtime catalog использует другие ключи. Features отсутствующие в context считаются false; неизвестный predicate не даёт безусловного разрешения. Regex компилируется с ограничениями; metadata error не проглатывается.

Обработка game/JVM arguments использует тот же evaluator. `value` может быть строкой либо массивом строк. Не передаём `--demo`, custom resolution и quick play arguments при выключенных features. В MVP все эти features false. Тестируем ordered allow/disallow, пустой список, отрицательные features, OS+arch+version.

## Libraries и natives

В проверенном JSON 97 library records. Для 1.21.1 natives находятся в обычных `downloads.artifact` JAR, а legacy `natives`/`downloads.classifiers` отсутствуют. Нельзя требовать legacy structure или запускать только client JAR. Сохраняем порядок разрешённых библиотек из metadata и добавляем client JAR в classpath.

OS rules 1.21.1 разрешают несколько architecture-specific native JAR для одной ОС. Сохраняем предписанный upstream набор текущей ОС: исключаем другие ОС, но не угадываем отсутствие необходимости по суффиксу имени. Это важно для `natives-macos-patch` у freetype в текущей ревизии. LWJGL выбирает ресурсы по OS/JVM architecture; [его loader](https://raw.githubusercontent.com/LWJGL/lwjgl3/3.3.3/modules/lwjgl/core/src/main/java/org/lwjgl/system/Library.java) умеет извлекать native из classpath. Нельзя распаковывать несколько архитектур в одну плоскую папку с перезаписью.

Для MVP используем upstream native-JAR loading/extraction в отдельный per-run `natives_directory`, переданный штатными JVM properties. При `Extracting` launcher готовит каталог и runtime, фактическая native extraction LWJGL происходит при загрузке классов после spawn. UI не показывает фиктивный процент для этого процесса. Перед каждым запуском каталог свежий; удаляем его только после окончания run. Проверка целостности относится к source JAR; у созданных JVM файлов нет опубликованных Mojang per-entry SHA1. Нативный smoke закрывает эту границу.

Добавление legacy version в будущем потребует adapter для metadata classifiers/extract.exclude и безопасного ZIP extraction. В 1.21.1 он не нужен; версии с неподдержанным форматом получают явный UnsupportedVersion, а не частичную установку. Оптимизация исключения лишних architecture JAR возможна только после fixtures и cross-platform smoke.

## Assets, client и logging

Asset index скачивается с size/SHA1. Снимок index `17` содержит 3911 имён, 3888 уникальных hashes: задачи дедуплицируем по hash. Файлы лежат в `assets/objects/<prefix>/<hash>`, index — `assets/indexes/<id>.json`. Logical asset names не используются как пути записи для этого формата. `virtual`/`map_to_resources`, если появятся при расширении версий, требуют отдельной проверенной реализации; не включаем её по умолчанию.

Client JAR хранится по metadata revision в shared storage. Logging config скачивается по `logging.client.file` с hash/size; подстановка `${path}` применима к `logging.client.argument`, отдельно от game placeholders. Пропуск logging config может сломать диагностику, поэтому он часть плана.

## Проверка, восстановление и отмена

Quick check: path safety → regular file → expected size → fingerprint из verification index. При новом/изменённом файле вычисляем SHA1. Для cache assets/libraries допускается reuse hash при совпавшем fingerprint; это оптимизация от случайных изменений, а не защита от локального злоумышленника, сохраняющего timestamps. Full repair всегда пересчитывает hashes. Runtime executable и agent заново хешируются перед исполнением; восстановление runtime описано отдельно.

Download: один reusable HTTP client, не более 8 одновременных requests по умолчанию; timeouts, максимум 3 попытки для temporary failure с backoff/jitter. Hash mismatch очищает partial и повторяет с лимитом; 401/403, постоянный 404 и disk full не ретраятся бесконечно. Download size ограничивается expected size, JSON дополнительно имеет верхний предел. Для файла без опубликованного size используем hash и жёсткий role-specific лимит; не считаем его исправным по существованию.

Каждый файл пишется в уникальный temp в том же filesystem, проверяется size/hash, flush/sync, затем atomic publish. Другой поток видит только завершённый объект. При Windows sharing violation показываем понятную ошибку/ограниченный retry. Нет глобальной транзакции на всю игру: корректные immutable objects остаются пригодными после отмены. Старый установленный runtime не перезаписывается пофайлово.

В MVP resume частичного файла не нужен: незавершённый partial скачивается заново, уже проверенные объекты сохраняются. Cancel останавливает enqueue/requests и hash workers между chunks; после точки успешного publish объект сохраняется. Progress отражает отдельно verified/reused/downloaded bytes, не считает reuse за сетевую скорость.

Реализованная проверка всегда пересчитывает SHA1. Fingerprint shortcut пока не используется; режим Quick не даёт сокращённой проверки. Libraries хранятся по expected SHA1 revision, asset index filename/launch ID включает metadata hash, чтобы не перезаписывать используемую revision. Resume byte-range не реализован, staging не считается готовым объектом.
