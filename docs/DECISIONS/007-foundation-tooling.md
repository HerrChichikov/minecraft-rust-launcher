# ADR-007: совместимый toolchain Phase 1

Дата: 2026-11-06. Статус: Accepted — реализация Phase 1 в пределах согласованной архитектуры.

Контекст: latest TypeScript 7.0.2 не входит в peer range typescript-eslint 8.72.1 (`<6.1.0`), а транзитивная minimatch 10.2.6 в текущем ESLint/TypeScript ESLint затронута GHSA-g3hv-75wg-m249.

Решение: Rust 1.99.1, Node 26.13.0 LTS / bundled npm 11.20.0; TypeScript 6.0.3, совместимый с текущими lint-инструментами. Прямые npm dependencies pinned exact, Rust graph закреплён Cargo.lock. HTTP/Java/keyring/download dependencies не добавляются до этапа, на котором используются. Это уточнение минимального графа, а не смена стека.

Для minimatch используется явный override 11.0.1. Изучен [upstream changelog](https://raw.githubusercontent.com/isaacs/minimatch/main/changelog.md): major 11 удаляет часть APIs, поэтому слепая замена была бы неверной. Проверены actual consumers в installed ESLint/config-array/typescript-estree: они используют `Minimatch.match` / `minimatch()`, а не удалённые makeRe/AST/regexp APIs. Совместимость проверяется полноценным lint/typecheck/test/build; CI проверяет lint на четырёх платформах через отдельные команды при расширении matrix. [Security advisory](https://github.com/advisories/GHSA-g3hv-75wg-m249) указывает patched version 11.0.1.

Альтернативы: `npm audit fix --force` откатывает lint-пакеты на несовместимые старые версии; игнорировать advisory оставляет известный риск; немедленно переходить на TS 7 ломает официальный peer contract. Override пересматривается и удаляется, когда upstream packages сами перейдут на исправленную совместимую версию.

CI использует current native runners: macos-15-intel и macos-15, Windows 2022, Ubuntu 22.04 baseline. Actions pinned по commit SHA официальных release tags. Локальные инструменты установлены в ignored `.tools`, caches — в `.cache`; системный PATH не изменён.

Phase 1 contracts не являются реализацией installer/provider/runtime. Единственный app command — read-only get_snapshot. Auth/Play commands, frontend filesystem/http/shell plugins отсутствуют. Dev-auth feature пока лишь compile-time reservation и guard, не mock login.
