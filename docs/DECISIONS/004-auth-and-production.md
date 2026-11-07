# ADR-004: provider boundary и запрет mock в release

Дата: 2026-11-06. Статус: Accepted (Phase 0 согласована 2026-11-06); выбор реального Ely.by flow требует продуктового решения до Phase 4.

Контекст: Development обязан запускаться с mock; production не должен иметь скрытого mock fallback. Ely.by имеет Yggdrasil и OAuth, а изученный OAuth требует client_secret.

Решение: AuthProvider выдаёт backend-only Session/LaunchIdentity. Mock с `dev-auth` + debug_assertions, release+feature compile error, mock registration в release отсутствует. Provider settings не обходят сборку. Production AuthUnavailable блокирует Play, пока нет рабочего real provider. Секреты OS store либо память, без plaintext fallback.

Альтернативы: спрятать toggle в UI недостаточно; environment variable для mock позволяет release bypass. OAuth с embedded secret не защищает secret. Новый broker добавляет сервис за пределами MVP. Рекомендуемый первый real flow — документированный password/TOTP, если владелец продукта согласует обработку пароля.

Последствия: можно подготовить production artifact без логина для внутренних проверок, но выпуск функционального MVP ждёт real auth/server smoke. UUID не доказывает auth. Authlib Injector pinned/reviewed с SHA256, license/source/notice review; установка agent не заменяет login.

Проверка: compile-fail exact guard, release IPC/config attack probes, no secrets in DTO/logs, test account refresh/invalidate и согласованный server smoke.

Обновление Phase 4: пользователь выбрал password/TOTP; реализация и OS persistence — [ADR-010](010-password-totp-auth.md). Release mock guard сохранён, real account/game/server smoke учитываются отдельно.
