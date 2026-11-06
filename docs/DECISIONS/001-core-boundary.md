# ADR-001: независимый launcher-core

Дата: 2026-11-06. Статус: Accepted (Phase 0 согласована 2026-11-06).

Контекст: installer/launch должны тестироваться без webview и оставаться пригодными для будущего CLI или другого UI.

Решение: Rust workspace из core crate и desktop crate; внутри core обычные модули application/core/installer/runtime/auth/launcher/profiles/storage. Desktop адаптирует paths, credential store и EventSink. React имеет только typed IPC client, без filesystem/network orchestration.

Альтернативы: один Tauri crate проще bootstrap, но привязывает tests к desktop и облегчает смешивание слоёв. Crate на каждый модуль даёт строгие границы, но для одного vanilla profile преждевременно усложняет API и сборку.

Последствия: два manifests и явные DTO conversions; чистые domain types не получают Tauri derives. Abstraction вводится на реальной внешней границе. Updater/plugins не реализуются ради гипотетического расширения.

Проверка: core tests компилируются без Tauri; dependency inspection подтверждает отсутствие desktop imports.
