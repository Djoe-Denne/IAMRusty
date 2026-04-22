---
title: RustyCog References Index
category: navigation
tags: [index, references, sdk]
summary: >-
  Canonical map from RustyCog crates to detailed crate references and shared concept/entity/skill pages, including current workspace packaging caveats.
provenance:
  extracted: 0.9
  inferred: 0.06
  ambiguous: 0.04
updated: 2026-04-19T10:59:36Z
---

# RustyCog References

This index is the canonical semantic map for RustyCog crates.

## Inventory

- [[projects/rustycog/references/rustycog-core]]
- [[projects/rustycog/references/rustycog-command]]
- [[projects/rustycog/references/rustycog-config]]
- [[projects/rustycog/references/rustycog-db]]
- [[projects/rustycog/references/rustycog-events]]
- [[projects/rustycog/references/rustycog-http]]
- [[projects/rustycog/references/rustycog-permission]]
- [[projects/rustycog/references/rustycog-testing]]
- [[projects/rustycog/references/wiremock-mock-server-fixture]]
- [[projects/rustycog/references/openfga-mock-service]]
- [[projects/rustycog/references/rustycog-server]]
- [[projects/rustycog/references/rustycog-logger]]
- [[projects/rustycog/references/rustycog-meta]]

## Workspace and Packaging Reality

- Root workspace members include: `rustycog-core`, `rustycog-server`, `rustycog-command`, `rustycog-db`, `rustycog-events`, `rustycog-http`, `rustycog-config`, `rustycog-permission`, `rustycog-testing`.
- `rustycog/Cargo.toml` defines `rustycog-meta`, which depends on the crate set above plus `rustycog-logger`.
- `rustycog/README.md` still references macros/examples that are not visible in the checked-in tree. ^[ambiguous]

## Crate Semantic Map

- Core -> entities: [[entities/service-error]], [[entities/domain-error]] | concept: [[concepts/shared-rust-microservice-sdk]] | skill: [[skills/using-rustycog-core]]
- Command -> entities: [[entities/command-registry]], [[entities/command-context]] | concept: [[concepts/command-registry-and-retry-policies]] | skill: [[skills/using-rustycog-command]]
- Config -> entities: [[entities/queue-config]] | concept: [[concepts/structured-service-configuration]] | skill: [[skills/using-rustycog-config]]
- DB -> entities: [[entities/db-connection-pool]] | concept: [[concepts/structured-service-configuration]] | skill: [[skills/using-rustycog-db]]
- Events -> entities: [[entities/domain-event]], [[entities/event-publisher]], [[entities/queue-config]] | concept: [[concepts/event-driven-microservice-platform]] | skill: [[skills/using-rustycog-events]]
- HTTP -> entities: [[entities/route-builder]], [[entities/resource-id]], [[entities/permission-checker]], [[entities/resource-ref]] | concept: [[concepts/centralized-authorization-service]] | skill: [[skills/using-rustycog-http]]
- Permission -> entities: [[entities/permission-checker]], [[entities/subject]], [[entities/resource-ref]], [[entities/resource-id]] | concept: [[concepts/openfga-as-authorization-engine]] | skill: [[skills/using-rustycog-permission]]
- Testing -> entities: [[entities/event-publisher]], [[entities/queue-config]], [[entities/route-builder]] | concept: [[concepts/integration-testing-with-real-infrastructure]] | skills: [[skills/using-rustycog-testing]], [[skills/stubbing-http-with-wiremock]] | wiremock fixtures: [[projects/rustycog/references/wiremock-mock-server-fixture]], [[projects/rustycog/references/openfga-mock-service]]
- Logger -> concept: [[concepts/structured-service-configuration]] | skill: [[skills/using-rustycog-logger]]
- Server -> entity: [[entities/health-checker]] | usage guidance is documented directly in [[projects/rustycog/references/rustycog-server]]
- Meta -> concept: [[concepts/shared-rust-microservice-sdk]] | usage guidance is documented directly in [[projects/rustycog/references/rustycog-meta]]

## Build Workflow

- End-to-end composition playbook: [[skills/building-rustycog-services]]
- Guide-vs-runtime drift analysis: [[references/rustycog-service-construction]]

## Known Gaps To Track

- `rustycog-logger` is included by `rustycog-meta` but not listed as a root workspace member. ^[ambiguous]
- `rustycog-server` naming still suggests broader server bootstrap scope than the current health-only surface. ^[ambiguous]
- `create_multi_queue_event_publisher()` accepts queue sets but currently wraps one publisher instance. ^[ambiguous]
- `rustycog-config` SQS endpoint conventions still mix AWS and Scaleway vocabulary. ^[ambiguous]
