# Phase 0 — результаты для согласования

Дата: 6 ноября 2026, Asia/Yekaterinburg. Статус: **исследование и проект документов подготовлены; ожидается review пользователя**. Application implementation не начата.

Исторический отчёт на момент завершения Phase 0. Пользователь подтвердил переход к Phase 1 2026-11-06; текущий статус вынесен в [Phase 1 review](PHASE_1_REVIEW.md). Упоминания отсутствующего кода/lockfiles ниже относятся к завершению Phase 0.

## Предложение

Tauri 2 + React/TypeScript strict/SCSS/Vite, отдельный независимый Rust `launcher-core`, тонкий desktop adapter, metadata-driven installer и launch planner. Shared assets/libraries/managed runtime и отдельная mutable game directory каждого profile. AuthProvider отделён от установки, mock исключён из release, реальный provider и Java Agent проходят отдельную проверку. GUI получает sanitized events/snapshot, а запуском и Child lifecycle управляет Rust.

## Что подготовлено

| Требование Phase 0 | Результат |
|---|---|
| Product Requirements | [PRD](PRODUCT_REQUIREMENTS.md), P01–P16 и границы MVP |
| Архитектура, project structure, interaction diagram | [Architecture](ARCHITECTURE.md), две Mermaid diagrams |
| Интерфейсы и модель данных | [Interfaces](INTERFACES.md), [Data model](DATA_MODEL.md) |
| Решения с обоснованием | [Шесть ADR](DECISIONS/README.md), alternatives/consequences/validation |
| Installation/Java/auth/launch/storage/security | Отдельные технические документы в README index |
| Зависимости | [Minimal dependency proposal](DEPENDENCIES.md), current version observations; graph пока не разрешён |
| Test/CI/CD plan | [Tests](TEST_PLAN.md), [CI/CD](CI_CD.md), native build + graphical smoke matrix |
| Технические риски и развитие | [Risks](RISKS.md), [Roadmap](ROADMAP.md) |
| Источники и воспроизводимые данные | [Sources](SOURCES.md), 10 public JSON snapshots + [provenance](research/provenance.json) |

## Подтверждено непосредственно по metadata

- Target metadata SHA1: `82ba6501da174a665c1787775f946487422f2432`; запись manifest обновлена 2026-11-03, releaseTime остаётся 2024-08-08. Старые snippets не считаем единственной актуальной формой.
- Main class `net.minecraft.client.main.Main`, Java component `java-runtime-delta`, major 21.
- 97 library records, natives как обычные artifact JAR, нет legacy natives fields; freetype содержит `natives-macos-patch`.
- 21 game/JVM placeholder, включая `clientid`/`auth_xuid`; logging `${path}` обрабатывается отдельно.
- Asset index 17: 3911 logical objects и 3888 уникальных hashes.
- Runtime candidate присутствует для Windows x64, Linux x64, macOS Intel, macOS ARM64; в снимке все 21.0.7. Это не утверждение о latest security Java patch.
- Unix runtime manifests содержат internal links, в том числе относительные `../java.base/...`; macOS executable внутри `jre.bundle/Contents/Home/bin/java`.
- Public Ely.by injector adapter возвращает metadata/public key. Agent descriptor — 1.2.8/build 56 с SHA256; сам binary не скачивался.

Metadata bytes получены с официальных HTTPS endpoints; version/index/runtime JSON проверены по parent SHA1/size. Parent root manifests не имеют добавленной нами upstream signature. Полные source URLs — в provenance.

## Проверки этого этапа

Команда `python3 docs/research/verify_snapshot.py` успешно выполнила **5279 отдельных assertions по данным**. Это одна offline research validation, **не 5279 тестов лаунчера**. Машиночитаемый результат: [verification report](research/verification-report.json).

Проверены hashes/sizes сохранённых bytes, parent descriptor chain, основные version fields, формы library paths/hash, asset counts, наличие raw runtime downloads, platform executable entries/exec bit и принадлежность объявленных link targets дереву manifest. Assertions не доказывают безопасность будущего installer и не заменяют его integration tests.

Проверка Markdown прошла: 26 Markdown-файлов, 52 локальные ссылки разрешаются, code fences парные. Rust/frontend tests, clippy/lint/build, full downloads и game smoke **не запускались**, потому что application code отсутствует и Phase 0 запрещает его реализацию.

Изменённые области: README, `docs/`, `docs/DECISIONS/`, `docs/research/`. Функциональные модули launcher не создавались. Не устанавливались packages, Java, game client или agent. Нет публикации, внешних сообщений, commits или release artifacts.

## Что остаётся решить и проверить

1. **Согласовать архитектуру для Phase 1.** В исходном `prompt.md`, раздел 20: «После завершения Phase 0 остановись и представь результаты на проверку. Не переходи к реализации без подтверждения».
2. До Phase 4 выбрать real Ely.by flow. Рекомендация: документированный password/TOTP без нового сервиса. Варианты browser OAuth требуют подтверждения public native flow или нового broker, которого нет в scope. [Сравнение](AUTHENTICATION.md#выбор-для-phase-4).
3. Проверить реальный token/provider root/agent/game/server flow, secure profile keys и mapping clientid/xuid. Public metadata не даёт этого доказательства.
4. Утвердить minimal OS versions, app name/bundle ID и smoke machines до упаковки. Точные dependency versions/lockfiles и CI runners фиксируются при foundation.
5. Production без real auth остаётся fail-closed internal artifact. Полный MVP release блокируется до real login/server smoke, даже если development mock запускает игру.

## Критерии готовности Phase 0

Документы, источники, структура, контракты, модель, diagrams, ADR, tests/CI plans, roadmap и риски подготовлены. Исследовательские данные проверены offline. Открытые executable/platform/auth риски имеют конкретные gates, не замаскированы. Пользовательское согласование пока не получено; **Phase 1 не начата**.

Следующий предлагаемый этап: Project Foundation — реальные Tauri/React/Rust workspace manifests, locked dependencies, strict checks и четыре CI build targets. Начать только после подтверждения.
