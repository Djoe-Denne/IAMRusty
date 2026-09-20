---
title: >-
  RustyCog
category: project
tags: [rustycog, sdk, platform, visibility/internal]
sources:
  - rustycog/README.md
  - .agents/skills/rustycog-submodule/SKILL.md
  - skills/building-rustycog-services.md
summary: >-
  SDK git submodule. Pin T14b : dual-bind HTTP 8080 + tls_port 8443 ;
  T11b auth client optionnelle (0858eab).
provenance:
  extracted: 0.74
  inferred: 0.20
  ambiguous: 0.06
created: 2026-04-15T17:15:56Z
updated: 2026-09-20T10:35:00Z
---

# RustyCog

RustyCog (`rustycog-framework`, imported as `rustycog`) is the feature-gated SDK every AIForAll service composes: command registry, config, HTTP, events, DB, permissions, logging, testing.

This vault’s detailed crate reference pages (`projects/rustycog/references/rustycog-*`) were linked from older indexes but are **missing from disk** after later syncs. Until they are restored, use the portable skills below as the crate map. ^[ambiguous]

## Pin

`AIForAll/rustycog/` is a git submodule, not a vendored tree. See [[projects/aiforall/concepts/rustycog-git-submodule]].

Pin T11b (`0858eab`) : authentification client TLS **optionnelle** pour `/lazaret/session`. Pin T14b : **dual-bind** `port` 8080 + `tls_port` 8443 (mesh HTTPS Hive/IAM/Telegraph). Détail mesh : [[projects/aiforall/concepts/https-platform-mesh]].

## Crate skills

- [[skills/building-rustycog-services]]
- [[skills/using-rustycog-core]]
- [[skills/using-rustycog-config]]
- [[skills/using-rustycog-db]]
- [[skills/using-rustycog-command]]
- [[skills/using-rustycog-events]]
- [[skills/using-rustycog-http]]
- [[skills/using-rustycog-permission]]
- [[skills/using-rustycog-testing]]
- [[skills/using-rustycog-logger]]

## Related platform pages

- [[concepts/shared-rust-microservice-sdk]]
- [[concepts/architecture-coherence-across-services]]
- [[projects/aiforall/concepts/rustycog-git-submodule]] — pin on **main**; cherry-pick SDK APIs before bumping.
- [[projects/rustycog/references/isolated-wiremock-fixture]] — private WireMock listener for parallel collaborator stubs.
- [[projects/aiforall/aiforall]]
- [[projects/lazaret/lazaret]]
- [[journal/2026-09-20]]
