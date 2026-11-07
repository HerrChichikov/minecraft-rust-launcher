# ADR-010 — Ely.by password/TOTP и memory-only session

Дата: 2026-11-07. Статус: принято. Пользователь явно выбрал реализацию документированного password + TOTP flow; desktop OAuth/PKCE не подтверждён и не используется.

ElyByAuthProvider использует fixed HTTPS auth base из provider configuration, reusable bounded blocking client без redirects/automatic POST retries. AuthService управляет password challenge (120 секунд, максимум 5 попыток), clientToken binding, rotation, validate-before-launch и local logout с отдельным remote revoke notice. Desktop commands выполняют I/O на blocking worker; dedicated mutex try_lock обеспечивает single-flight без ожидания UI/Workspace mutex.

Пароли/TOTP не сохраняются. Access/client token хранятся только в памяти процесса; OS credential adapters и «Запомнить меня» отложены без plaintext fallback. После restart нужен новый вход. Desktop deadline supervisor очищает idle challenge; активный HTTP request имеет 15-секундный timeout.

Mock type/input/branch доступны только debug+dev-auth; release feature guard и отрицательные IPC tests обязательны. Authlib Injector pinned/verified/argv adapter подготовлен отдельно; наличие real provider не означает выполненный agent/game/server smoke. Источники — AUTHENTICATION.md; фактические проверки — PHASE_4_REVIEW.md.
