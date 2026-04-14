---
title: >-
  AIForAll Platform README
category: references
tags: [reference, platform, operations, visibility/internal]
sources:
  - README.md
summary: >-
  Source summary for the top-level AIForAll README covering repo layout, shared Docker workflow, and service communication.
provenance:
  extracted: 0.95
  inferred: 0.05
  ambiguous: 0.00
created: 2026-04-14T16:54:59.5971424Z
updated: 2026-04-14T16:54:59.5971424Z
---

# AIForAll Platform README

This source is the repo's operational overview for `[[projects/aiforall/aiforall]]`.

## Key Ideas

- The repository contains multiple microservices, notably `[[projects/iamrusty/iamrusty]]`, `[[entities/telegraph]]`, shared Rust crates, and event definitions.
- A shared Docker Compose stack runs IAMRusty and Telegraph with PostgreSQL and LocalStack.
- IAMRusty publishes user-signup events that Telegraph consumes through SQS-backed local infrastructure.
- Service-specific configuration is delegated to each subproject rather than centralized at the root.

## Open Questions

- The doc is strong on local operations but light on service internals beyond IAMRusty and Telegraph.
- It references `iam-events` without expanding on that contract crate.

## Sources

- [[projects/aiforall/aiforall]] — Platform overview page
- [[concepts/event-driven-microservice-platform]] — Cross-service communication pattern
