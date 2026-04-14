---
title: Wiki Index
---

# Wiki Index

*This index is automatically maintained. Last updated: 2026-04-14T17:13:01.1911009Z*

## Projects

- [[projects/aiforall/aiforall]] — Repo-level map of services, shared infra, and local workflows ( #platform #microservices #rust)
- [[projects/iamrusty/iamrusty]] — Identity service for OAuth, JWTs, typed config, and real-infrastructure testing ( #iam #oauth #security)
- [[projects/manifesto/manifesto]] — Project-management service and practical blueprint for RustyCog-style services ( #projects #orchestration #blueprint)
- [[projects/rustycog/rustycog]] — Shared Rust SDK/workspace for commands, config, HTTP, permissions, events, logging, DB access, and testing ( #sdk #rust #platform)
- [[projects/hive-events/hive-events]] — Shared event-contract crate for organization-domain integrations ( #events #sqs #integration)

## Concepts

- [[concepts/event-driven-microservice-platform]] — Transport-neutral domain events let services coordinate through Kafka, SQS, or disabled queue backends ( #architecture #microservices #events)
- [[concepts/hexagonal-architecture]] — Domain logic stays isolated from adapters through layered service design ( #architecture #hexagonal #ddd)
- [[concepts/component-based-project-orchestration]] — Projects orchestrate independently implemented components through contracts and registries ( #projects #components #orchestration)
- [[concepts/shared-rust-microservice-sdk]] — RustyCog splits common service infrastructure into coordinated crates and generic extension traits ( #sdk #rust #platform)
- [[concepts/oauth-provider-linking]] — IAMRusty links multiple OAuth identities to one provider-agnostic user model ( #iam #oauth #authentication)
- [[concepts/structured-service-configuration]] — Typed loaders use RUN_ENV profiles, nested env overrides, queue enums, and cached random ports ( #configuration #env #rust)
- [[concepts/jwt-secret-storage-abstraction]] — JWT signing is decoupled from secret backend choice and key format ( #security #jwt #auth)
- [[concepts/oauth-state-and-csrf-protection]] — OAuth callbacks are hardened with encoded state, nonce, expiry, and exact redirects ( #security #oauth #csrf)
- [[concepts/integration-testing-with-real-infrastructure]] — Integration tests reuse shared servers, real containers, and optional Kafka/LocalStack fixtures ( #testing #integration #fixtures)
- [[concepts/command-registry-and-retry-policies]] — Command registries centralize validation, timeouts, retries, metrics, and tracing behind one execution surface ( #commands #reliability #rust)

## Entities

- [[entities/telegraph]] — Communication service that consumes events and turns them into notifications ( #service #notifications #messaging)

## Skills

- [[skills/building-rustycog-services]] — Workflow for scaffolding a RustyCog service with config, logging, command execution, RouteBuilder routes, permissions, and tests ( #rustycog #scaffolding #services)
- [[skills/testing-rust-services-with-fixtures]] — Workflow for testing Rust services with shared fixtures, JWT helpers, and selective queue-backed checks ( #testing #fixtures #rust)

## References

- [[references/aiforall-platform]] — Top-level repo overview, Docker workflow, and service communication ( #reference #platform #operations)
- [[references/iamrusty-service]] — IAMRusty features, configuration, and architecture summary ( #reference #iam #oauth)
- [[references/iamrusty-runtime-and-security]] — IAM runtime config, JWT, and OAuth hardening guides ( #reference #configuration #security)
- [[references/iamrusty-testing-and-fixtures]] — IAM testing harness and fixture-system guidance ( #reference #testing #fixtures)
- [[references/iamrusty-command-execution]] — IAM command registry and retry-policy guidance ( #reference #commands #reliability)
- [[references/manifesto-service]] — Manifesto product model, setup, and project-service ADR summary ( #reference #projects #components)
- [[references/platform-building-blocks]] — Shared SDK crates and event-contract packages that provide common runtime, transport, and testing foundations ( #reference #sdk #events)
- [[references/rustycog-service-construction]] — Manifesto-authored build guides checked against the current command, config, HTTP, permission, and logging crates ( #reference #rustycog #architecture)
- [[references/rustycog-crate-catalog]] — Code-backed inventory of the current RustyCog crates, their responsibilities, and packaging caveats ( #reference #rustycog #sdk)

## Synthesis

## Journal
