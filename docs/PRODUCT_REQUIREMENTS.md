# Product Requirements

Статус: предложение на согласование, Phase 0. Все функции ниже — требования к будущей реализации.

## Назначение и границы

Desktop-лаунчер для собственного сервера Minecraft Java Edition; в MVP одна конфигурация vanilla 1.21.1. Пользователь входит, устанавливает игру без ручной установки Java, запускает её и видит состояние процесса. Закрытие игры не закрывает лаунчер.

Целевые пакеты: Windows x64 — NSIS `.exe` и WiX `.msi`; Linux x64 — `.deb`; macOS Intel и ARM64 — отдельные `.dmg`. Поддержка платформы подтверждается установкой пакета и игровым smoke, а не только компиляцией. Минимальные версии ОС уточняются в Phase 1 по требованиям Tauri и Mojang runtime; предложенная Linux baseline — Ubuntu 22.04 / Debian 12, проверить в Phase 7.

В MVP не входят Bedrock, моды, модлоадеры, каталог серверов, plugin runtime, собственный auth backend и автообновление лаунчера. Для updater достаточно ADR и будущей точки интеграции; пустой работающий «updater» не создаём.

## Проверяемые требования

| ID | Поведение | Проверка / этап |
|---|---|---|
| P01 | Четыре целевые архитектуры, нужные установочные пакеты | CI build + package install + smoke, 1/7/8 |
| P02 | Создаётся default-профиль со стабильным ID и отдельными game/logs | Storage integration, 2 |
| P03 | Общие неизменяемые ресурсы не копируются в каждый профиль | Два профиля, единый cache, разные saves/options, 2/3 |
| P04 | Игра, logging config, библиотеки, assets получены по Mojang metadata | Fixture + HTTP integration, 3 |
| P05 | Java выбирается по component/major/OS/arch, загружается автоматически | Runtime fixture + четыре smoke, 3/7 |
| P06 | Quick-check перед запуском, hash при изменении; полная проверка по запросу | Missing/corrupt/same-size corruption/repair, 3 |
| P07 | Прерывание загрузки сохраняет исправные файлы; cancel доступен в UI | Disconnect/cancel/restart, 3/6 |
| P08 | Mock-вход с валидным ником и стабильным UUID только в development | Positive debug + negative release, 4 |
| P09 | Release не может зарегистрировать mock, даже через IPC/config | Compile-fail feature + release IPC probes, 4/8 |
| P10 | Реальный provider отделён от установки; нет ложного успешного входа | Auth contract tests, 4 |
| P11 | Запуск использует metadata JVM/game args, classpath и natives | Golden argv + реальная игра, 5/7 |
| P12 | Нельзя одновременно запускать один профиль дважды | Race/process integration, 5 |
| P13 | stdout/stderr читаются без блокировки UI, код выхода обрабатывается | Большие логи / crash / normal exit, 5/6 |
| P14 | RAM min/max, корень данных, managed/custom Java и JVM args настраиваются | Backend validation + UI tests, 2/5/6 |
| P15 | Токены хранятся в credential storage или только в памяти | Keychain unavailable + log/IPC inspection, 4/8 |
| P16 | Документация соответствует реализации, критические проверки проходят | Review + CI, каждый этап |

Development Mock не подтверждает владение Minecraft, не обещает доступ к online-серверам и явно обозначается в интерфейсе. Production без рабочего реального provider показывает «Вход недоступен» и блокирует Play. Это безопасное промежуточное состояние, но **не готовый пользовательский релиз**.

## UI и настройки

Три экрана: вход; главная; настройки. Вход показывает provider, ник/логин, пароль только для password provider, отдельный TOTP challenge при необходимости. Главная: имя, placeholder аватара, профиль, версия, статус установки, большая «Играть», этап/байты/скорость, отмена, состояние игры, переход в настройки. Настройки: min/max RAM в MiB, корень данных, managed/custom Java executable, список дополнительных JVM arguments.

Предлагаемые defaults: 1024/4096 MiB, с проверкой доступной памяти перед сохранением/запуском. Если машина не удовлетворяет этим значениям, UI предлагает допустимые; не сохраняем заведомо нерабочие настройки. Это defaults продукта, а не значения из Mojang metadata.

Ошибки локализуемые: понятное сообщение, действие «Повторить»/«Открыть настройки», correlation ID. Stack traces и raw HTTP responses в UI не попадают. Навигация клавиатурой, видимый focus, progress с текстовой подписью, reduced motion.

## Решения владельца продукта

1. Согласовать Phase 0 перед Phase 1.
2. До Phase 4 выбрать реальный Ely.by flow: [варианты и рекомендация](AUTHENTICATION.md#выбор-для-phase-4).
3. До релиза нужны доступ к Ely.by test account, согласованная серверная интеграция, имя/bundle ID, минимальные ОС и ресурсы для smoke. До упаковки используем рабочее название, не выдавая его за утверждённый бренд.
