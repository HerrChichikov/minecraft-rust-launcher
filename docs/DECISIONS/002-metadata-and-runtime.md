# ADR-002: official metadata и managed Java

Дата: 2026-11-06. Статус: Accepted (Phase 0 согласована 2026-11-06).

Контекст: 1.21.1 metadata задаёт Java component/major, client/libraries/assets/arguments; native artifacts отличаются от legacy classifiers. Системной Java может не быть.

Решение: resolver читает официальные форматы и URLs, rules обрабатываются единообразно. Runtime устанавливается по official catalog + per-platform manifest, raw files, immutable generation. Classpath сохраняет разрешённый upstream native-JAR набор; native extraction выполняет штатный LWJGL в isolated run dir.

Альтернативы: hardcoded list быстро устареет; сторонний universal launcher SDK скрывает критические правила и может принести лишние loader/auth flows; системная Java нарушает требование автоматической установки. Самостоятельное flatten-extract всех native JAR создаёт architecture collisions. Generic Maven resolver не нужен: metadata уже перечисляет artifacts.

Последствия: bootstrap URLs централизованы и требуют обслуживания. Поддержка только проверенного 1.21.1 формата; новая версия требует fixtures/smoke, но не переписывания downloader. Лишние architecture JAR, разрешённые одним OS rule, сохраняются ради upstream semantics; оптимизация позже по evidence.

Проверка: official JSON chain, 4 runtime manifest fixtures, ordered rules/argv golden tests, native graphical smoke. JSON доступность не равна runtime compatibility.
