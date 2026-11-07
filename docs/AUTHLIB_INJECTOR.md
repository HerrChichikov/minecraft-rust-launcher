# Pinned Authlib Injector

Phase 4 фиксирует upstream binary без модификаций; в launcher packages JAR отсутствует. Core downloader вызывается только явным installer/launch workflow или developer smoke; автоматического latest update нет.

| Поле | Значение |
|---|---|
| Version / build | 1.2.8 / 56 |
| Descriptor | https://authlib-injector.yushi.moe/artifact/56.json |
| Binary | https://authlib-injector.yushi.moe/artifact/56/authlib-injector-1.2.8.jar |
| SHA256 | `9c7f4343e6c82034958ffb48c14a2cb0c85928be7283103ce17da00c6d5a7b10` |
| Source commit | `5437f5b465ee3f5be0a1e5e1ae3c50978114f6fb` |
| Copyright | Haowei Wen and contributors; notices remain in the unmodified upstream source/binary |
| License | AGPL v3-or-later with AUTHLIB-INJECTOR exception |

[Corresponding source at pinned commit](https://github.com/yushijinhun/authlib-injector/tree/5437f5b465ee3f5be0a1e5e1ae3c50978114f6fb), [source archive](https://github.com/yushijinhun/authlib-injector/archive/5437f5b465ee3f5be0a1e5e1ae3c50978114f6fb.tar.gz), [upstream COPYING](https://github.com/yushijinhun/authlib-injector/blob/5437f5b465ee3f5be0a1e5e1ae3c50978114f6fb/COPYING.md), [local full license](licenses/authlib-injector.txt). Exception covers loading as Java Agent, unmodified binary inclusion without linking, and IPC; obligations for redistribution of the agent itself remain. No warranty is provided by upstream license.

[Launcher protocol](https://yushijinhun.github.io/authlib-injector/en/launcher-technical-specification.html), [official artifact API](https://yushijinhun.github.io/authlib-injector/en/get-authlib-injector.html), [Ely.by integration](https://docs.ely.by/en/authlib-injector.html). Agent requires independent authentication; it does not create an Ely.by session. No client/server compatibility or secure-profile/chat readiness is claimed before real game/server smoke.

Before any future distribution with an embedded agent, review the exact distribution method, notices/source access and packaging. Current tests keep downloaded JAR only in ignored isolated test cache; CI uploads text receipts only.
