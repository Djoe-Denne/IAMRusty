---
title: >-
  IAMRusty Testing and Fixtures Guides
category: references
tags: [reference, testing, fixtures, visibility/internal]
sources:
  - IAMRusty/docs/TESTING_GUIDE.md
  - IAMRusty/docs/FIXTURES_GUIDE.md
summary: >-
  Source summary for IAMRusty's integration-test harness, fixture system, and real-infrastructure testing patterns.
provenance:
  extracted: 0.94
  inferred: 0.04
  ambiguous: 0.02
created: 2026-04-14T17:03:47.5107188Z
updated: 2026-04-14T17:03:47.5107188Z
---

# IAMRusty Testing and Fixtures Guides

These sources document how `[[projects/iamrusty/iamrusty]]` tests end-to-end behavior using the same philosophy captured in `[[concepts/integration-testing-with-real-infrastructure]]` and `[[skills/testing-rust-services-with-fixtures]]`.

## Key Ideas

- Integration tests use real database and HTTP infrastructure, with optional Kafka and SQS test containers for event verification.
- The default performance strategy is container reuse plus table truncation rather than starting a fresh environment for every test.
- External integrations are mocked with service fixtures for GitHub and GitLab, while domain state is created through strongly typed `DbFixtures` builders.
- Tests standardize around serial execution, shared setup helpers, and consistent HTTP/JWT utilities.
- The docs recommend starting with mock event assertions and only escalating to queue-backed tests for critical integration paths.

## Open Questions

- Some message-queue tests are documented as ignored or Docker-dependent, so the standard CI policy for those paths is not explicit. ^[ambiguous]
- The sources explain patterns well, but they do not rank which test layers are mandatory for every new feature.

## Sources

- [[concepts/integration-testing-with-real-infrastructure]] — Main testing architecture distilled from these docs
- [[skills/testing-rust-services-with-fixtures]] — Operational workflow for using fixtures effectively
- [[projects/iamrusty/iamrusty]] — Project page where these test practices apply first
