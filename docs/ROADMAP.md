# План реализации

Phase 0 согласована пользователем 2026-11-06. Phase 1 реализована и прошла все четыре native CI build targets; готова к review. Результаты и ограничения — в [отчёте](PHASE_1_REVIEW.md). Phase 2 завершена, native checks/builds всех четырёх targets пройдены; результаты проверок — в [отчёте](PHASE_2_REVIEW.md). Phase 3 завершена: native install/reuse/loader/tests/packages всех четырёх targets и installer GUI smoke на macOS ARM64 пройдены: [отчёт](PHASE_3_REVIEW.md). Phase 4 завершена: Ely.by password/TOTP, development mock, session owner, optional OS credential storage и verified agent preparation; local checks и все четыре native CI targets pass. Live Ely.by authenticate подтверждён пользователем; ограничения TOTP/restore/platform GUI явно перечислены в [отчёте](PHASE_4_REVIEW.md). Репозиторий переведён в Public с отдельного согласия пользователя. Phase 5 завершена: запуск Minecraft 1.21.1, управление процессом и журналы. Реальный development launch подтверждён на macOS ARM64, CI прошёл на всех четырёх целевых платформах. [Отчёт](PHASE_5_REVIEW.md). Phase 6 реализует экраны «Игра», «Аккаунт» и «Настройки», событийный прогресс и управление операциями. [Отчёт](PHASE_6_REVIEW.md). Phase 7–8 не начаты. На переходе проверяем критерии и получаем согласование, если оно требуется исходным заданием; не считаем количество файлов готовностью этапа.

| Этап | Результат | Критерий выхода / зависимости |
|---|---|---|
| **0 Research & Architecture** | Эти документы, источники, metadata evidence, ADR | Offline evidence checks и review пользователя; никакого application code |
| **1 Project Foundation** | Tauri 2, React/TS strict/SCSS/Vite, два Rust crates, lint/test/CI, lockfiles | Clean locked build всех четырёх targets; blank app opens на доступных машинах; недоступные checks явно отмечены, сборки не объявлены успешными без CI |
| **2 Storage & Profiles** | Default profile, config schemas, root paths/locks, settings, verification-index storage | Isolation/migration/unsafe-path/concurrency tests, без установки в shared `.minecraft` |
| **3 Minecraft Installer** | Metadata-based client/libraries/assets/logging/runtime, repair/cancel | Полная установка 1.21.1 на каждой доступной платформе, повтор без лишних download, corruption/interruption recovery; первые ARM native compatibility spikes |
| **4 Authentication** | Provider contract, dev mock, compile guard, выбранный Ely.by flow и verified agent integration | Contract/negative release tests; real provider либо честный AuthUnavailable; решение по flow обязательно до его реализации |
| **5 Launch Engine** | Argv/classpath/rules/placeholders, process monitor, logs/exit/RAM/options | Golden tests и реальный development launch 1.21.1; Mock не проходит production gate |
| **6 UI/UX** | Три экрана, loading/error/progress/cancel/running, settings | UI tests, keyboard/reconnect, responsiveness; никакого fake success |
| **7 Cross-platform Validation** | Windows/Linux/macOS Intel/ARM64 package + real game reports | Полная [smoke matrix](TEST_PLAN.md), исправление всех platform blockers |
| **8 MVP Release** | Production packages, license/notices, signing decision, docs | Реальный production login/server smoke, mock недоступен, все критические tests pass; без рабочего real auth этап остаётся незавершённым |

На каждом этапе отчёт содержит: фактические функции, изменённые модули, выполненные команды/результаты, оставшиеся проблемы, критерии готовности, предложение следующего этапа. Если проверена одна ОС, в отчёте указана одна ОС.

## После MVP

Несколько profiles/versions через новые metadata adapters; modloaders через слой плана установки; custom Yggdrasil provider через AuthProvider; updater как отдельный application workflow с подписанными metadata/packages. Не строим plugin sandbox, remote execution SDK, distributed cache, background daemon и собственный auth server заранее.

При добавлении новой версии обязательны её реальные metadata fixtures, полный placeholder/rules review, runtime mapping и platform smoke. Наличие generic parser не означает заявленную поддержку всех Minecraft versions.
