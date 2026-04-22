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
  - Hive/tests/fixtures/external_provider/mod.rs
  - Hive/tests/fixtures/external_provider/service.rs
  - Hive/tests/fixtures/external_provider/resources.rs
summary: Hive validates its API with real DB and JWT-backed tests, plus an ExternalProviderMockService wrapper around the shared rustycog-testing wiremock fixture, while keeping queue publishing disabled in the default test runtime.
provenance:
  extracted: 0.78
  inferred: 0.14
  ambiguous: 0.08
created: 2026-04-14T18:56:22.3888182Z
updated: 2026-04-22T16:20:59Z
---

# Hive Testing and API Fixtures

These sources show how `[[projects/hive/hive]]` validates its organization-management API: real database state, JWT-authenticated HTTP tests, shared RustyCog test-server wiring, and dedicated fixtures for the external provider dependency.

## Key Ideas

- `HiveTestDescriptor` follows the shared `rustycog_testing` pattern for service bootstrapping, migrations, DB setup, and test-server lifecycle, as documented in `[[projects/rustycog/references/rustycog-testing]]`.
- Hive's default test runtime keeps `has_db()` true but `has_sqs()` false, which means queue publishing is disabled by default even though the production configuration supports SQS.
- Organization, member, and external-link tests create real DB state, mint JWTs, call the live HTTP server, and assert on both response codes and persisted data.
- The tests are serial and mirror the same broad style used by Telegraph and IAMRusty, but Hive's focus is HTTP plus DB rather than queue-consumer behavior.
- External provider behavior is isolated through an `ExternalProviderMockService` (in `Hive/tests/fixtures/external_provider/service.rs`) that wraps the shared [[projects/rustycog/references/wiremock-mock-server-fixture]] and emulates `/config/validate`, `/connection/test`, `/organization/info`, `/members`, and `/members/check` endpoints.
- The wrapper exposes one async `mock_*` method per scenario — `mock_validate_config_ok`, `mock_validate_config_fail(message_contains)`, `mock_connection_test(connected)`, `mock_organization_info(name, external_id)`, `mock_members(members)`, `mock_is_member(is_member)` — each returning `&Self` so arrangements can be chained per test.
- Each mock is mounted with `Mock::given(method("POST")).and(path("/..."))`; the failure stub also chains `body_string_contains(message_contains)` to react only to specific request bodies, and the response side uses `ResponseTemplate::new(<status>).set_body_json(...)` with typed DTOs from `Hive/tests/fixtures/external_provider/resources.rs` (`ConnectionTestResponseBody`, `OrganizationInfo`, `MembersResponse`, `Member`).
- The fixture is constructed via `ExternalProviderFixtures::service().await`, which calls `ExternalProviderMockService::new()` → `MockServerFixture::new()`; the fixture handle is held in a `_fixture` field so its `Drop` impl resets all mocks for the next test.
- Tests like `external_link_api_tests` (`create_external_link_happy_path`, `create_external_link_requires_auth`, `create_external_link_forbidden_for_read_only_member`) currently exercise the API surface end-to-end against the live HTTP server and DB but do not yet arrange `ExternalProviderMockService` stubs in the visible flows. ^[ambiguous]
- Follow [[skills/stubbing-http-with-wiremock]] when extending this fixture or adding a new external collaborator.
- Conflict to resolve: Hive is a queue-capable event publisher in production, but its default test harness disables SQS and does not currently emphasize queue-backed verification the way IAMRusty or Telegraph do. ^[ambiguous]

## Open Questions

- The live test suite is strong on org/member/external-link flows, but this source batch does not show a correspondingly rich invitation or sync-job API test surface. ^[ambiguous]
- Queue publishing and event delivery are important parts of Hive's runtime story, yet the current default test harness treats them as optional rather than central. ^[ambiguous]

## Sources

- [[projects/hive/hive]] - Service whose API and fixtures are under test.
- [[concepts/integration-testing-with-real-infrastructure]] - Cross-service concept view of these patterns.
- [[projects/hive/references/hive-http-api-and-openapi-drift]] - Live HTTP behaviors covered by the tests.
- [[projects/hive/concepts/external-provider-sync-jobs]] - External-provider fixture behavior and sync context.
- [[projects/rustycog/references/rustycog-testing]] - Shared RustyCog testing runtime reused by Hive.
- [[projects/rustycog/references/wiremock-mock-server-fixture]] - Shared mock server the `ExternalProviderMockService` wraps.
- [[skills/stubbing-http-with-wiremock]] - Recipe behind the `mock_*` helper convention used here.