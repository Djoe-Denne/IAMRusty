---
title: Hive Testing and API Fixtures
category: references
tags: [reference, testing, fixtures, visibility/internal]
sources:
  - Hive/config/test.toml
  - Hive/tests/common.rs
  - Hive/tests/organization_api_tests.rs
  - Hive/tests/members_api_tests.rs
  - Hive/tests/external_link_api_tests.rs
  - Hive/tests/fixtures/external_provider/service.rs
summary: Hive validates its API with real DB and JWT-backed tests, plus Wiremock-style external-provider fixtures, while keeping queue publishing disabled in the default test runtime.
provenance:
  extracted: 0.79
  inferred: 0.13
  ambiguous: 0.08
created: 2026-04-14T18:56:22.3888182Z
updated: 2026-04-14T18:56:22.3888182Z
---

# Hive Testing and API Fixtures

These sources show how `[[projects/hive/hive]]` validates its organization-management API: real database state, JWT-authenticated HTTP tests, shared RustyCog test-server wiring, and dedicated fixtures for the external provider dependency.

## Key Ideas

- `HiveTestDescriptor` follows the shared `rustycog_testing` pattern for service bootstrapping, migrations, DB setup, and test-server lifecycle.
- Hive's default test runtime keeps `has_db()` true but `has_sqs()` false, which means queue publishing is disabled by default even though the production configuration supports SQS.
- Organization, member, and external-link tests create real DB state, mint JWTs, call the live HTTP server, and assert on both response codes and persisted data.
- The tests are serial and mirror the same broad style used by Telegraph and IAMRusty, but Hive's focus is HTTP plus DB rather than queue-consumer behavior.
- External provider behavior is isolated through a `MockServerFixture` that emulates `/config/validate`, `/connection/test`, `/organization/info`, `/members`, and `/members/check` endpoints.
- Conflict to resolve: Hive is a queue-capable event publisher in production, but its default test harness disables SQS and does not currently emphasize queue-backed verification the way IAMRusty or Telegraph do. ^[ambiguous]

## Open Questions

- The live test suite is strong on org/member/external-link flows, but this source batch does not show a correspondingly rich invitation or sync-job API test surface. ^[ambiguous]
- Queue publishing and event delivery are important parts of Hive's runtime story, yet the current default test harness treats them as optional rather than central. ^[ambiguous]

## Sources

- [[projects/hive/hive]] - Service whose API and fixtures are under test.
- [[concepts/integration-testing-with-real-infrastructure]] - Cross-service concept view of these patterns.
- [[references/hive-http-api-and-openapi-drift]] - Live HTTP behaviors covered by the tests.
- [[concepts/external-provider-sync-jobs]] - External-provider fixture behavior and sync context.