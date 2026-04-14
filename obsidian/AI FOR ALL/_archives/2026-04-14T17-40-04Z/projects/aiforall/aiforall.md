---
title: >-
  AIForAll
category: project
tags: [platform, microservices, rust, visibility/internal]
sources:
  - README.md
summary: >-
  Repo-level map of the AIForAll platform covering its core services, shared infrastructure, and local development workflow.
provenance:
  extracted: 0.85
  inferred: 0.15
  ambiguous: 0.00
created: 2026-04-14T16:54:59.5971424Z
updated: 2026-04-14T16:54:59.5971424Z
---

# AIForAll

AIForAll is a Rust-based microservices workspace centered on `[[projects/iamrusty/iamrusty]]`, `[[entities/telegraph]]`, shared libraries in `[[projects/rustycog/rustycog]]`, and event contracts such as `[[projects/hive-events/hive-events]]`.

## Key Ideas

- The workspace is organized as a `[[concepts/event-driven-microservice-platform]]` with clear service boundaries and shared local infrastructure.
- A top-level Docker Compose flow runs IAMRusty and Telegraph alongside PostgreSQL and LocalStack for local development.
- Shared patterns and building blocks are factored into `[[concepts/shared-rust-microservice-sdk]]`, which reduces duplication across services. ^[inferred]
- The broader project-service direction described in `[[projects/manifesto/manifesto]]` extends the same platform model beyond identity and messaging.

## Open Questions

- The top-level README names `iam-events`, but this source batch does not explain how it differs from `[[projects/hive-events/hive-events]]`.
- Telegraph is described as handling SMS as well as email and notifications, but the current ingest set only documents the notification side in detail. ^[ambiguous]

## Sources

- [[references/aiforall-platform]] — Repository overview and shared dev workflow
