# Хранилище и профили

Статус: предложение Phase 2. Filesystem adapter в desktop передаёт core абсолютный app data root из Tauri path resolver. Не пишем в `.app`, Program Files, cwd приложения или системную `.minecraft`.

## Размещение

Базы: Windows LocalAppData, macOS `~/Library/Application Support`, Linux XDG data home. Используем `app_local_data_dir()` через [Tauri PathResolver](https://docs.rs/tauri/latest/tauri/path/struct.PathResolver.html): на Windows он отличается от `app_data_dir()`, который использует RoamingAppData. Точные пути не собираем по username. Logs/cache используют соответствующие системные app directories. Пользовательский data root хранится в `app_config_dir()`; сама bootstrap config не перемещается вместе с root.

```text
<data-root>/
  profiles/<profile-uuid>/
    config.json
    game/                      # saves, options, screenshots; не перезаписываем repair
    logs/                      # launcher capture; game/logs принадлежит игре
    runs/<run-uuid>/natives/
  shared/
    metadata/versions/<sha1>.json
    versions/<version-id>/<metadata-sha1>/client.jar
    libraries/<official-artifact-path>
    assets/indexes/<index-id>.json
    assets/objects/<first-two>/<sha1>
    runtimes/<platform>/<component>/<manifest-sha1>/
    agents/<sha256>/authlib-injector.jar
  state/
    verification-index.json
    accounts.json              # credential references, no secrets
    runs.json                  # diagnostic/reconciliation identity, no token
    locks/
```

Assets уже content-addressed; runtime и version metadata разделены по hash. Libraries сохраняют official path ради classpath совместимости; expected hash связывается с install plan. Если один library path получил новый hash, используемый файл не перезаписываем: новая revision storage path выбирается явно. Не добавляем CAS+hardlinks+GC без практической необходимости.

## Атомарность и индексы

Config/index small JSON пишутся во временный файл в той же папке, `sync_all`, затем atomic replace с platform-tested Windows replacement. Родительский directory sync на Unix по возможности; failure возвращается явно. Publication immutable objects не заменяет используемый объект. Network/disk I/O не держит mutex всего приложения. Индекс обновляется после публикации, никогда раньше.

Индекс verification — восстанавливаемый cache, не authority. При его повреждении выполняется повторное хеширование без удаления игры. Ключ включает root identity, relative path и expected hash; root migration/metadata revision инвалидируют соответствующие records. Старый size/hash не применяется к новой metadata.

Профиль изолирует изменяемые saves/config/logs. Repair трогает только ресурсы из InstallPlan; не очищает всю game directory. Смена root в MVP — переключение на новый/ранее созданный launcher root с проверкой schema/permissions; автоматическое перемещение гигабайтов и пользовательских миров отложено. UI объясняет, что старые данные остаются, до подтверждения сохранения настройки. Во время установки/игры смена root запрещена.

## Безопасные пути

Profile ID принимается как UUID, display name не становится путём. Любые metadata paths: reject absolute/UNC/drive/prefix, NUL, parent traversal, Windows alternate streams, reserved device names и escape через separators. Вычисляем destination относительно разрешённого root, проверяем ancestry; одной строковой `starts_with` недостаточно.

Все managed parents проверяются `symlink_metadata`, без прохождения неожиданных symlinks/reparse points. Runtime links обрабатываются особой ограниченной политикой из [Java docs](JAVA_RUNTIME.md). Для native extraction не допускаем ZIP symlinks, traversal, absolute paths и zip bombs. Возможная local race между check/open — отдельный риск; нужен no-follow/handle-based подход там, где platform APIs позволяют. Не заявляем защиту от администратора или процесса с теми же правами, изменяющего managed tree.

Locks: OS-backed app/root lock и profile/resource lock. Существование старого lock-файла после crash не означает занятость; lock удерживается ОС. Runtime generation нельзя удалить или заменить, пока есть живой пользователь JVM. Cleanup ограничен собственными `.part`/run staging после проверки, что они не используются.
