# Architecture Decision Records

ADR 001–006 приняты вместе с подтверждением Phase 0 пользователем 2026-11-06. Это архитектурные решения, не утверждение о реализации всех сервисов. ADR-007 фиксирует инженерные уточнения toolchain Phase 1.

| ADR | Решение |
|---|---|
| [001](001-core-boundary.md) | Два crates, независимый core и Tauri adapter |
| [002](002-metadata-and-runtime.md) | Mojang metadata и official managed runtime |
| [003](003-storage-and-integrity.md) | Shared cache, isolated profiles, atomic publication |
| [004](004-auth-and-production.md) | Provider boundary, dev-only mock, production fail closed |
| [005](005-launch-and-ipc.md) | Direct process argv, events, owned lifecycle |
| [006](006-dependencies-and-release.md) | Минимальные зависимости, locked builds, platform evidence |
| [007](007-foundation-tooling.md) | Совместимый toolchain, security override и минимальный граф Phase 1 |

Изменение решения обновляет ADR с причиной и последствиями. Новое решение, отменяющее старое, связывается ссылкой; история не стирается.

- [ADR-008: локальное хранилище и профили](008-storage-profiles.md) — schemas, capability paths, locks, atomic publication, root switching.

- [ADR-009: metadata-based installer](009-minecraft-installer.md) — HTTP, integrity, runtime generations, cancellation, native compatibility.

- [ADR-010: Ely.by password/TOTP и memory-only session](010-password-totp-auth.md).
