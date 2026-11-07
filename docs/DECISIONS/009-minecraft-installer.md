# ADR-009 — metadata-based installer

Дата: 2026-11-07. Статус: принято; native installer validation всех четырёх targets и installer GUI smoke macOS ARM64 пройдены.

InstallEngine работает синхронно в blocking worker; внутри использует reusable reqwest blocking/Rustls client и максимум восемь threads для resource transfer. Выбор оставляет cancellation и bounded file publication понятными без async filesystem abstractions. Java probe использует отдельный Tokio runtime: stdout/stderr читаются одновременно с ограничением размера, timeout/cancel закрывает reader futures и kill/wait child. Длительный I/O не держит WorkspaceState mutex.

Все пути ресурсов строятся backend из проверенных metadata. Libraries включают expected hash в storage path, client — metadata revision; asset index filename/launch identifier — id+hash. Immutable runtime generation собирается из raw CAS objects отдельными copies, links проверяются до создания и могут указывать только на manifest files внутри поколения. Publication — rename; предыдущее поколение не изменяется и не удаляется. Java executable и полный runtime rehash перед reuse/probe.

Для MVP оба verification modes используют full SHA1. Verification-index пока не ускоряет проверку: это сознательный выбор корректности и простоты; corrupted index восстанавливается, future schema блокирует запись. Reuse исключает artifact download, не metadata requests. Metadata offline fallback, byte-range resume, GC и legacy ZIP classifiers остаются отдельными задачами.

URL policy — exact official HTTPS hosts, без userinfo/необычных ports, проверка каждого redirect, bounded JSON/output/retries. SHA1 — опубликованный Mojang integrity digest, не подпись издателя. Profile/runtime execution selection не передаётся в installer request как произвольный URL/path.

Java os.arch aliases нормализуются; macOS host может отличаться от binary под Rosetta. Windows filesystem capabilities используют verbatim canonical paths, Java-facing executable — normal Win32/UNC representation: native loader spike выявил HotSpot jimage crash с verbatim argv[0]. Преобразование не меняет root authority. Phase 5 применит адаптер к остальным Java-facing paths.

Native CI проверяет реальную установку, zero-artifact-download повтор и loader LWJGL/GLFW/FreeType на четырёх targets. Это не Minecraft launch/GPU/login smoke. Runtime/game binaries не распространяются вместе с launcher packages или CI receipts. После разблокировки Mac отдельно пройден installer GUI smoke: start/cancel/retry до Ready; исходные root/profile восстановлены.
