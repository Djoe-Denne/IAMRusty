---
title: Isolated WireMock fixture
category: references
tags: [testing, wiremock, rustycog, visibility/internal]
sources:
  - rustycog/rustycog-testing/src/wiremock/mod.rs
  - Manifesto/tests/fixtures/component_service/service.rs
  - C:/Users/djden/.cursor/projects/c-Users-djden-source-repos-AIForAll/agent-transcripts/23280216-f48f-426e-8a11-6b42b8184a6e/23280216-f48f-426e-8a11-6b42b8184a6e.jsonl
summary: >-
  MockServerFixture::isolated() starts a private listener so component-catalog stubs do not share the singleton :3000 server used by other fixtures.
provenance:
  extracted: 0.88
  inferred: 0.10
  ambiguous: 0.02
created: 2026-09-09T16:45:00Z
updated: 2026-09-09T16:45:00Z
---

# Isolated WireMock fixture

`MockServerFixture::new()` is a **shared** listener (historically `127.0.0.1:3000`). Parallel Manifesto tests that stub the component catalog on that singleton collide with other mocks.

`MockServerFixture::isolated().await` starts a **private** server. Manifesto’s `ComponentService` fixture uses it. Hive/Telegraph/IAM collaborator fixtures may keep `new()` when they own the singleton for that suite.

## Rules

- Do **not** use WireMock for OpenFGA **Check** in Hive / Telegraph / Manifesto HTTP ITs — use [[projects/rustycog/references/openfga-real-testcontainer-fixture]].
- Hold the fixture in a field so `Drop` resets mocks.
- There is no Manifesto HTTP test that boots sentinel-sync end-to-end. AuthZ IT either uses `TestOpenFga` or asserts Manifesto HTTP + DB. ^[inferred]

## Related

- [[skills/stubbing-http-with-wiremock]]
- [[projects/manifesto/references/manifesto-testing-and-fixtures]]
- [[skills/using-rustycog-testing]]
- [[projects/rustycog/rustycog]]
