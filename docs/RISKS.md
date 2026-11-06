# Технические риски

Статус на 2026-11-06. Вероятность качественная; владельцем инженерных проверок является разработчик, продуктовых решений — заказчик.

| ID / приоритет | Риск и основание | Что делать / gate |
|---|---|---|
| R01 / blocker release | Mock запрещён в production, реальный auth может отсутствовать | Production fail closed; выбрать/проверить real provider до Phase 8 |
| R02 / high | Ely OAuth docs требуют secret, native PKCE не подтверждён | Password/TOTP либо подтверждённый native flow; решение до Phase 4 |
| R03 / high | Два Ely API семейства + современная authlib 1.21.1 | Test account: login/refresh/validate/agent/root/server smoke; не считать public GET доказательством auth |
| R04 / high | Secure profile/chat key и server-side auth могут отличаться | Проверить согласованный сервер; не отключать validation без решения |
| R05 / high | Native JAR format отличается от старых launcher tutorials, macOS patch classifier | Сохранить metadata classpath и LWJGL loading, четыре native smoke |
| R06 / high | Нет реальных Windows/Linux/macOS Intel/ARM smoke | CI matrix + machines; до этого targets planned |
| R07 / high | Catalog Delta сейчас даёт 21.0.7, freshness не равна latest Java security patch | Повторная оценка upstream перед релизом; при необходимости отдельное решение runtime policy |
| R08 / high | HTTPS+SHA1 metadata не являются подписью от издателя | Trusted bootstrap/host policy, reviewed agent SHA256, upstream compromise остаётся |
| R09 / high | Runtime symlinks/reparse/ZIP traversal и local race | Handle/no-follow validation, staged generation, adversarial path tests |
| R10 / high | Token в argv, сторонние crash/log/dump файлы | No full argv logs, scrub controlled sinks, no automatic raw report upload; local observer риск остаётся |
| R11 / medium | Fingerprint cache пропустит умышленную подмену с теми же атрибутами | Full repair + fresh executable/agent hash; не заявлять anti-malware гарантию |
| R12 / high | Disk full, interruption, sharing violation и два launcher instances | Atomic publication, OS locks, interrupted install tests на каждой ОС |
| R13 / high | macOS quarantine, signing runtime, first-thread, Rosetta | Реальный downloaded DMG и managed Java на двух native arch |
| R14 / medium | Linux glibc/WebKitGTK, GPU driver, noexec и отсутствие Secret Service | Oldest baseline build, desktop smoke, actionable error/memory-only credentials |
| R15 / medium | Launcher crash оставляет JVM и теряет Child handle | Run identity reconciliation, conservative duplicate block, no arbitrary PID kill |
| R16 / medium | Агент AGPL exception и обновление бинарника | Pin exact version/hash/license/source, notices; никаких silent latest upgrades |
| R17 / medium | Vite/React/Tauri/keyring APIs обновились | Phase 1 clean locked graph, strict typecheck, platform build before claiming compatibility |
| R18 / medium | Resource/log flood или malformed metadata | Concurrency/size/retry/rotation limits, negative tests |
| R19 / medium | Minimum ОС, brand/bundle ID, server endpoint пока не согласованы | Решить до packaging; не блокирует документацию и core foundation |

Самые ранние технические spikes после foundation: runtime + native startup на macOS ARM64, full metadata-derived argv, release mock rejection. Исследовательские JSON уже снижают неопределённость форматов, но не закрывают эти executable/GPU риски.
