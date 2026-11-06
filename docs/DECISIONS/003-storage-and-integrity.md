# ADR-003: shared resources и isolated mutable data

Дата: 2026-11-06. Статус: Accepted (Phase 0 согласована 2026-11-06).

Контекст: profiles должны изолировать миры/настройки, но большие libraries/assets/Java нельзя дублировать без необходимости. Прерывание загрузки не должно портить исправную установку.

Решение: shared immutable managed resources, per-profile game/log/run directories. Маленькие JSON configs/index, schema version, atomic same-filesystem publication, OS-backed locks. Quick fingerprint cache плюс SHA1 при изменении; full repair — hash всех managed resources. Executable и agent rehash перед запуском.

Альтернативы: общая `.minecraft` смешивает mutable data; копия игры на профиль расходует диск; SQLite/CAS+hardlinks с GC дают дополнительные механизмы без нужды MVP; только existence check не обнаруживает corruption; full hashing всех assets при каждом Play ухудшает latency.

Последствия: index восстанавливаемый и не защищает от умышленной equal-fingerprint подмены. Runtime in use не патчится. Repair не удаляет worlds. Изменение data root в MVP переключает root без автоматического переноса файлов.

Проверка: two-profile isolation, concurrent download, interrupted atomic write, corrupt index, full repair, unsafe-path tests на целевых ОС.
