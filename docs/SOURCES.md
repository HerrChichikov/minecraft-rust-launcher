# Источники исследования

Проверено 2026-11-06 (дата пользователя, Asia/Yekaterinburg). Использованы первичные источники. Где документация не определяет desktop behavior, решение помечено как предложение/риск, а не факт API.

## Mojang metadata

| Источник | Для чего |
|---|---|
| [Version manifest v2](https://piston-meta.mojang.com/mc/game/version_manifest_v2.json) | Выбор 1.21.1, version URL и SHA1 |
| [1.21.1 metadata](https://piston-meta.mojang.com/v1/packages/82ba6501da174a665c1787775f946487422f2432/1.21.1.json) | Main class, arguments, libraries, Java, assets, logging |
| [Asset index 17](https://piston-meta.mojang.com/v1/packages/4ca8e1a1edd37a703c5d1594f71d409852cd9770/17.json) | Content-addressed objects |
| [Official Java catalog](https://launchermeta.mojang.com/v1/products/java-runtime/2ec0cc96c44e5a76b9c8b7c39df7210883d12871/all.json) | Четыре platform keys, component candidates |
| Runtime per-platform manifests | Конкретные URLs в [provenance](research/provenance.json), files/directory/link/permissions |

Web reader не открыл manifest JSON; эти первичные данные получены HTTPS curl напрямую и проверены локально. Это не scrape стороннего wiki. Root catalogs доверяются HTTPS, дочерние JSON дополнительно сверены с published parent checksum/size. SHA256 provenance фиксирует наши bytes; не является upstream signature.

## Tauri, Rust и frontend

- [Architecture](https://v2.tauri.app/concept/architecture/) — webview/Rust desktop boundaries.
- [Capabilities](https://v2.tauri.app/security/capabilities/), [permissions](https://v2.tauri.app/security/permissions/) — custom command ACL и remote access.
- [Calling frontend](https://v2.tauri.app/develop/calling-frontend/) — events и channels, ограничения event capability model.
- [Prerequisites](https://v2.tauri.app/start/prerequisites/), [Debian](https://v2.tauri.app/distribute/debian/), [GitHub pipeline](https://v2.tauri.app/distribute/pipelines/github/) — native builds, Linux baseline.
- [macOS signing](https://v2.tauri.app/distribute/sign/macos/), [Windows signing](https://v2.tauri.app/distribute/sign/windows/) — будущая distribution setup.
- [Tauri CLI 2.12.4 release](https://github.com/tauri-apps/tauri/releases/tag/tauri-cli-v2.12.4), [React versions](https://react.dev/versions), [Vite releases](https://vite.dev/releases), [Vite guide](https://vite.dev/guide/) — текущие семейства и tooling prerequisites.
- [Rust Command](https://doc.rust-lang.org/std/process/struct.Command.html), [Tokio process](https://docs.rs/tokio/latest/tokio/process/index.html) — argv, spawn, wait и cancellation caveats.
- [Tauri PathResolver](https://docs.rs/tauri/latest/tauri/path/struct.PathResolver.html) — app local/config/cache/log paths, различие Windows roaming/local storage.
- [Keyring 4.2](https://docs.rs/keyring/latest/keyring/), [keyring-core](https://docs.rs/keyring-core/latest/keyring_core/) — актуальное разделение API/store adapters.
- [GitHub runner images](https://github.com/actions/runner-images) — runner availability/architecture нужно проверять при настройке CI.
- [Tauri WebDriver](https://v2.tauri.app/develop/tests/webdriver/) — текущие desktop automation options; реальный Minecraft smoke всё равно требует graphical environment.

## Ely.by и Authlib Injector

- [Minecraft auth](https://docs.ely.by/en/minecraft-auth.html) — password/TOTP, refresh/validate/invalidate, game session/profile.
- [OAuth](https://docs.ely.by/en/oauth.html) — game scope, code/token/info, documented client_secret.
- [Ely.by agent guide](https://docs.ely.by/en/authlib-injector.html) — обязательность собственного launcher login и javaagent.
- [Profile API](https://docs.ely.by/en/api.html) — remote UUID/name lookup не является login.
- [Launcher technical specification](https://yushijinhun.github.io/authlib-injector/en/launcher-technical-specification.html), [server technical specification](https://yushijinhun.github.io/authlib-injector/en/yggdrasil-server-technical-specification.html) — API roots, ALI, arguments, session endpoints.
- [Download API](https://yushijinhun.github.io/authlib-injector/en/get-authlib-injector.html), [public artifact descriptor](https://authlib-injector.yushi.moe/artifact/latest.json) — build/version/download_url/checksums.sha256.
- [Ely.by public adapter metadata](https://account.ely.by/api/authlib-injector) — наблюдаемый serverName, public key и feature flags; сохранено без private data.
- [Project COPYING](https://github.com/yushijinhun/authlib-injector/blob/develop/COPYING.md), [AGPL with exception](https://github.com/yushijinhun/authlib-injector/blob/develop/AGPLv3-with-authlib-injector-exception.txt) — просмотрены license terms, agent не распространяется в этом repository.

## Современные launcher/native подходы

- [Prism data layout](https://prismlauncher.org/wiki/getting-started/data-location/) и [managed Java](https://prismlauncher.org/wiki/getting-started/installing-java/) — instances/shared resources и runtime management.
- [XMCL installer docs](https://xmcl.app/en/core/installer) — metadata-driven installation tasks и upstream runtime bootstrap.
- [LWJGL 3.3.3 loader](https://raw.githubusercontent.com/LWJGL/lwjgl3/3.3.3/modules/lwjgl/core/src/main/java/org/lwjgl/system/Library.java) и [platform mapping](https://raw.githubusercontent.com/LWJGL/lwjgl3/3.3.3/modules/lwjgl/core/src/main/java/org/lwjgl/system/Platform.java) — classpath native extraction и architecture-aware resource paths.

Код чужих launchers не копировался. Доступность библиотечных API и метаданных не подменяет доказательство работающей установки/авторизации/игры. Все решения поверх наблюдений обоснованы в ADR.
