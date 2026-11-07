# ADR-010 — Ely.by password/TOTP и OS credentials и memory session

Дата: 2026-11-07. Статус: принято. Пользователь явно выбрал реализацию документированного password + TOTP flow; desktop OAuth/PKCE не подтверждён и не используется.

ElyByAuthProvider использует fixed HTTPS auth base из provider configuration, reusable bounded blocking client без redirects/automatic POST retries. AuthService управляет password challenge (120 секунд, максимум 5 попыток), clientToken binding, rotation, validate-before-launch и local logout с отдельным remote revoke notice. Desktop commands выполняют I/O на blocking worker; dedicated mutex try_lock обеспечивает single-flight без ожидания UI/Workspace mutex.

Пароли/TOTP не сохраняются. Access/client token хранятся только в памяти процесса; Keyring v1 использует нативный OS credential store по явному флажку, без plaintext fallback. После restart доступен explicit restore с validate+refresh и profile binding. Без сохранения нужен новый вход. OS операции идут на одном worker thread, user unit tests изолированы fixture adapter. Desktop deadline supervisor очищает idle challenge; активный HTTP request имеет 15-секундный timeout.

Mock type/input/branch доступны только debug+dev-auth; release feature guard и отрицательные IPC tests обязательны. Authlib Injector pinned/verified/argv adapter подготовлен отдельно; наличие real provider не означает выполненный agent/game/server smoke. Источники — AUTHENTICATION.md; фактические проверки — PHASE_4_REVIEW.md.

При ошибке OS write после token rotation свежая session сохраняется в памяти с notice; saved record не подменяется plaintext. Logout удаляет app-owned запись, а failures удаления и revoke возвращаются явно. Namespace фиксирован backend; записи не содержат password/TOTP и не принимают mock provider IDs.
