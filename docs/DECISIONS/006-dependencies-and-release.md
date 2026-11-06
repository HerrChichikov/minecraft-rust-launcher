# ADR-006: locked builds и подтверждение платформ

Дата: 2026-11-06. Статус: Accepted (Phase 0 согласована 2026-11-06).

Контекст: задание требует актуальные совместимые версии и четыре платформы. Существование исходников и target flags не доказывает, что пакет устанавливается и игра работает.

Решение: Phase 1 разрешает минимальный stable dependency graph, commits Cargo.lock/package-lock и exact toolchain/Node; builds с locked deps. CI native matrix и отдельная graphical smoke matrix. Пакеты каждой архитектуры маркируются planned/build-verified/smoke-verified по evidence. License/signing requirements документируются до release.

Альтернативы: «latest» без lock не воспроизводим; pin старых знакомых APIs без проверки не соответствует current ecosystem; только cross-compilation не закрывает runtime/GPU/keychain отличия. Lock-файлы без manifests в Phase 0 были бы фиктивным результатом.

Последствия: текущий список dependencies — предложение, не прошедший сборку граф. Минимальные ОС и runner availability подтверждаются в Phase 1. Недоступная smoke машина остаётся открытым release gate; signing может быть отложен только с честно указанным внутренним unsigned distribution status.

Проверка: clean locked install/build, no-mock release checks, artifact hash и test reports для каждого target. Автоматическое publishing/updater в MVP foundation не включается.
