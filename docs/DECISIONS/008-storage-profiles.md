# ADR-008 — локальное хранилище и профили

Дата: 2026-11-07. Статус: принято для Phase 2.

## Решение

Используем schemaVersion=1 JSON, один стабильный bootstrap `launcher.json` в app_config_dir и отдельный выбранный root с UUID. Default data root — `app_local_data_dir()/data`: на macOS configuration/data bases могут совпасть. Bootstrap хранит только root и revision; activeProfileId и его revision принадлежат конкретному root. Смена root переключает authority, не переносит файлы. Первый пустой root получает один профиль 1.21.1; собственные файлы пользователя и системная `.minecraft` не принимаются как новый root.

Display name не участвует в пути. UUID v4 генерирует core; профиль публикуется rename из `.creating-UUID` после создания game/logs/runs/config. Незавершённые staging остаются для диагностики и не появляются в списке. Автоматического удаления миров/GC нет. Optimistic revisions и rootId отклоняют запросы из устаревшего UI.

Для managed I/O выбраны cap-std/cap-fs-ext 4.0.3: открытые directory capabilities, одно имя на каждый no-follow open, лексическая переносимая проверка metadata paths. OS aliases в родителях явно выбранного root разрешаются canonicalize один раз. Root сам не может быть symlink/reparse point. Это не обещание защиты от администратора/того же пользователя, перемещающего открытые каталоги, подменяющего hardlinks или изменяющего файлы в обход лаунчера.

`File::try_lock` из std удерживает app/root/profile locks. Файлы locks никогда не удаляются: их наличие после crash не означает занятость. ProfileLease удерживает root и OS handle даже после drop service; switch запрещён при живом lease. Locks advisory на Unix; все будущие installer/launch workflows обязаны участвовать в этом протоколе. Индекс пока меняется под root lease; параллельные resource workers появятся с installer.

Small JSON записывается create_new temp в том же directory, sync_all(file), rename/replace, Unix directory sync. Windows использует cap-std platform rename; native CI проверяет повторную замену существующего config. Ошибка parent sync после rename имеет отдельный Durability outcome: UI получает ошибку, а переключённый root в памяти соответствует опубликованному bootstrap. Windows parent directory flush не обещается. На Unix fsync использует отдельный readable descriptor, открытый как `.` относительно held Dir; Linux O_PATH capability нельзя синхронизировать напрямую.

Локальный ProfileRepository синхронный: bounded JSON операции выполняются через Tauri spawn_blocking. Короткая блокировка сериализует только настройки WorkspaceState; snapshot bootstrap и остальной app не блокируются. Network/hash/install jobs не должны держать этот mutex. Для RAM используется sysinfo 0.40.1 с feature system; без запуска Java. Перед сохранением custom Java проверяются absolute path, regular file и executable bit Unix / `.exe` Windows. Версия и architecture проверяются в следующих фазах.

## Ограничения

Schema 1 — первая реализованная схема; предыдущих production schemas для миграции нет. Неизвестные schemas запрещают запись. Повреждённые settings не сбрасываются. Corrupt verification cache переносится в `verification-index.invalid-UUID.json`, создаётся пустой cache; future schema остаётся нетронутой. Индекс хранит подсказки, не подтверждает integrity, rehash реализуется в Phase 3.

RAM 256…physical MiB, min<=max, hard cap 1 TiB. Load допускает settings с большей RAM, чем на текущей машине, чтобы их можно было исправить; save использует текущий physical limit. Extra JVM args: максимум 32, ограниченный список GC flags и MaxGCPauseMillis. Произвольные agents/classpath/argfiles/-D/OnError/RAM arguments не принимаются. Параметры могут быть несовместимы с конкретной Java; реальный probe/launch — последующие фазы.

## Проверка

Temporary directories с Unicode/пробелами, реальный повторный open, filesystem round trips, isolated world sentinel, stale revision, corrupt/future schema preservation, no-follow symlinks, процессные OS locks и принудительное завершение holder, fault injection до publication и после rename. Tauri mock IPC проходит реальный handler/ACL, использует изолированные paths. Frontend проверяет shared DTO fixture и не показывает успех до ответа backend. Итоговые результаты/платформы — в PHASE_2_REVIEW.md.
