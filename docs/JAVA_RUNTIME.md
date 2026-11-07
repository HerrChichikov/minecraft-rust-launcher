# Управление Java

Статус: managed runtime установлен и probe выполнен на macOS ARM64; full native install/reuse/probe/loader matrix четырёх targets пройдена в Phase 3. [Результаты](PHASE_3_REVIEW.md).

Версия 1.21.1 указывает `javaVersion = { component: "java-runtime-delta", majorVersion: 21 }`. Component нельзя угадывать из major или hardcode в installer. Bootstrap [runtime catalog Mojang](https://launchermeta.mojang.com/v1/products/java-runtime/2ec0cc96c44e5a76b9c8b7c39df7210883d12871/all.json) задаётся в одном upstream config; все file URLs/hash/size берём из выбранного manifest.

## Проверенная доступность на 2026-11-06

| Host / JVM target | Ключ catalog | Delta version | Executable относительно runtime root |
|---|---|---|---|
| Windows x64 | `windows-x64` | 21.0.7 | `bin/java.exe` |
| Linux x64 glibc | `linux` | 21.0.7 | `bin/java` |
| macOS Intel x64 | `mac-os` | 21.0.7 | `jre.bundle/Contents/Home/bin/java` |
| macOS Apple Silicon ARM64 | `mac-os-arm64` | 21.0.7 | `jre.bundle/Contents/Home/bin/java` |

Это наблюдение upstream catalog, не обещание, что 21.0.7 — последний security patch Java 21. При выпуске повторно проверяем каталог. Если upstream не даёт подходящего component/architecture, возвращаем RuntimeUnavailable. Не переключаемся молча на системную Java, другой major или сторонний download provider.

## Алгоритм

1. Platform adapter определяет host architecture отдельно от архитектуры приложения; учитывает Rosetta. По умолчанию JVM native host architecture, и тот же target используется для libraries/rules.
2. Выбирает component для platform из catalog. Сейчас у каждого target один candidate с rollout progress=100. При нескольких выбирает последнюю fully available подходящую запись по `version.released`; staged rollout не превращает в случайную политику. Отсутствие fully available candidate — понятная ошибка.
3. Проверяет size/SHA1 runtime manifest. Installation generation идентифицируется его hash и platform, не одним именем Java.
4. Создаёт staging directory; обрабатывает `directory`, `file`, `link`. В MVP берём `downloads.raw`; LZMA не нужен и не добавляет dependency. Файлы проверяются по raw size/SHA1, executable bit из manifest применяется на Unix.
5. Links создаются последними. `..` в target не запрещён целиком: официальные legal symlinks используют `../java.base/...`. Разрешаем только нормализованный target внутри runtime staging root; проверяем всю цепочку, отсутствие циклов и выхода через symlink. Для Windows catalog в снимке links нет.
6. Проверяет complete tree, internal link targets, executable path и права. После успешной проверки публикует immutable generation, обновляет reference атомарно. Профиль никогда не запускается из staging.
7. Через absolute executable вызывает Java probe с timeout 5 секунд и ограниченным выводом (`-XshowSettings:properties -version` как исходный вариант). Проверяет major и architecture; не доверяет только имени папки. stdout/stderr Java учитываются оба. Пользовательский executable исполняется только после явного выбора пользователем.

Использование raw сохраняет исполняемые файлы/библиотеки и legal files ровно как у upstream. Не меняем bundle contents без необходимости: macOS подпись runtime и Gatekeeper проверяются на реальном downloaded package в Phase 7.

## Integrity и настройки

Per-file индекс ускоряет обычную проверку. Перед исполнением rehash executable; для immutable runtime generation проверяем fingerprint остальных файлов, modified files rehash, full repair — весь manifest. При повреждении используемой generation не изменяем файлы под работающей JVM: новую generation публикуем отдельно, восстановление ждёт освобождения runtime lock при необходимости.

Managed runtime — default. Custom Java хранится как абсолютный путь, без PATH lookup; проверяются executable, major, arch и возможность запуска. Другая архитектура не вызывает скрытого fallback: пользователь получает RuntimeIncompatible, пока такая комбинация не прошла smoke. JVM env очищается от `JAVA_TOOL_OPTIONS`, `_JAVA_OPTIONS`, `JDK_JAVA_OPTIONS`, `CLASSPATH` и переменных внедрения библиотек по platform policy.

Открытые риски: noexec mount, права, glibc/GPU зависимости, quarantine, Rosetta и security freshness runtime. Все требуют real-machine smoke; JSON сам по себе их не закрывает. [Provenance](research/provenance.json) содержит URLs и hashes четырёх manifests.

Реализация Phase 3 берёт raw files, проверяет весь manifest при reuse и выполняет bounded Tokio probe. Runtime CAS objects копируются в новую generation (без hardlinks), legal file links создаются последними. Поддерживаются только ссылки, в итоге указывающие на manifest file; directory links отклоняются. Старые generations и interrupted staging автоматически не удаляются. После repair старая generation остаётся нетронутой.

Java architecture aliases нормализуются: amd64/x86_64 → x64, arm64/aarch64 → ARM64. Для Windows Java-facing executable path использует обычный Win32/UNC вид; verbatim prefix остаётся внутренним filesystem representation. Phase 5 должен применять тот же execution_path adapter ко всем Java-facing paths, включая classpath/asset arguments.
