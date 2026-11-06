# ADR-005: direct process и событийное состояние

Дата: 2026-11-06. Статус: Accepted (Phase 0 согласована 2026-11-06).

Контекст: путь, ник и JVM args не должны превращаться в shell code; UI должен оставаться отзывчивым при установке и игре.

Решение: Tokio process direct executable + Vec argv, metadata substitution после rules, typed settings validation и запрещённые override categories. Один supervisor владеет Child/wait и profile lock. UI получает sanitized throttled events, high-volume logs — bounded channel; snapshot для startup/reconnect с sequence.

Альтернативы: shell command string опасен и ломает quoting; frontend orchestration дублирует business state; постоянный polling расходует ресурсы и даёт устаревшие ответы; unbounded events/log buffers раздувают память. Фоновый daemon ради window close лишний для текущего сценария.

Последствия: game token остаётся в argv согласно upstream protocol; локальную видимость полностью скрыть нельзя. Controlled sinks проходят redaction. При window close во время игры по умолчанию launcher остаётся открыт. Force stop не называется гарантированным graceful save. После crash нужен conservative run reconciliation.

Проверка: argv golden для четырёх ОС/arch, subprocess flood/exit/spawn-failure tests, reconnect race и double Play. Tauri app command permissions заданы явно; event payload не содержит credentials.
