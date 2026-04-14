---
title: Wiki Index
---

# Wiki Index

*This index is automatically maintained. Last updated: 2026-04-14T18:56:22.3888182Z*

## Projects

- [[projects/aiforall/aiforall]] - Repo-level map of services, shared infra, and local workflows ( #platform #microservices #rust)
- [[projects/hive/hive]] - Organization-management service for orgs, members, invitations, external links, sync jobs, and event publishing via `hive-events` ( #organizations #permissions #integrations)
- [[projects/hive-events/hive-events]] - Shared event-contract crate for organization-domain integrations ( #events #sqs #integration)
- [[projects/iamrusty/iamrusty]] - Rust IAM service for OAuth, email/password auth, JWTs, provider linking, and queue-backed identity events ( #iam #oauth #security)
- [[projects/manifesto/manifesto]] - Project-management service and practical blueprint for RustyCog-style services ( #projects #orchestration #blueprint)
- [[projects/telegraph/telegraph]] - Queue-driven communication service that consumes IAM events, renders descriptors/templates, and exposes a JWT-protected notification read model ( #communication #events #notifications)
- [[projects/rustycog/rustycog]] - Shared Rust SDK/workspace for commands, config, HTTP, permissions, events, logging, DB access, and testing ( #sdk #rust #platform)

## Concepts

- [[concepts/component-based-project-orchestration]] - Projects orchestrate independently implemented components through contracts and registries ( #projects #components #orchestration)
- [[concepts/command-registry-and-retry-policies]] - Repo services use typed command registries for HTTP and queue work, with IAMRusty, Telegraph, and Hive differing in retry wiring and registry breadth ( #commands #reliability #rust)
- [[concepts/descriptor-driven-communications]] - Telegraph builds emails and notifications from per-event TOML descriptors and Tera templates instead of hardcoding channel content ( #communication #templates #events)
- [[concepts/event-driven-microservice-platform]] - Transport-neutral domain events let Hive, IAMRusty, Telegraph, and other services coordinate through Kafka, SQS, or disabled queue backends ( #architecture #microservices #events)
- [[concepts/external-provider-sync-jobs]] - Hive links organizations to external providers, validates configs over HTTP, and starts sync jobs that publish domain events ( #integrations #sync #organizations)
- [[concepts/hexagonal-architecture]] - Domain services, use cases, adapters, and a setup composition root keep IAMRusty layered and swappable ( #architecture #hexagonal #ddd)
- [[concepts/integration-testing-with-real-infrastructure]] - Repo services use real DB, queue, and HTTP fixtures, with Hive adding DB-plus-mock-provider tests alongside IAMRusty's Kafka and Telegraph's SQS/SMTP patterns ( #testing #integration #fixtures)
- [[concepts/invitation-driven-membership]] - Hive models invitations as tokenized membership objects with roles, expiry, and event emission for downstream notification flows ( #organizations #invitations #membership)
- [[concepts/jwt-secret-storage-abstraction]] - JWT signing is resolved from HMAC or PEM-backed secret storage before token services and JWKS are built ( #security #jwt #auth)
- [[concepts/multi-channel-delivery-modes]] - Telegraph's config and storage model describe multiple channels, but the active runtime currently wires email and notifications more fully than SMS ( #communication #notifications #sms)
- [[concepts/organization-resource-authorization]] - Hive combines route guards with runtime organization, member, role, and resource permission lookups instead of static ACLs alone ( #authorization #permissions #organizations)
- [[concepts/oauth-provider-linking]] - Authenticated users can attach provider accounts safely while preserving one provider-agnostic user record ( #iam #oauth #authentication)
- [[concepts/oauth-state-and-csrf-protection]] - OAuth callbacks rely on encoded operation state, nonce handling, and context-aware error paths, with some doc-code drift around expiry ( #security #oauth #csrf)
- [[concepts/queue-driven-command-processing]] - Telegraph routes SQS events through a rustycog command service so async consumers and HTTP handlers share typed orchestration ( #events #commands #queue)
- [[concepts/shared-rust-microservice-sdk]] - RustyCog splits common service infrastructure into coordinated crates and generic extension traits ( #sdk #rust #platform)
- [[concepts/structured-service-configuration]] - AIForAll services use typed config loaders, but IAMRusty, Telegraph, and Hive diverge in env prefixes, queue models, and service-specific sections ( #configuration #env #rust)

## Entities

## Skills

- [[skills/building-event-driven-notification-services]] - Build a Telegraph-style service by combining queue-driven commands, descriptor-based template rendering, and an authenticated notification read model ( #events #services #rust)
- [[skills/building-organization-management-services]] - Build a Hive-style service with resource-backed permissions, event-publishing use cases, and real API fixtures ( #organizations #permissions #services)
- [[skills/building-rustycog-services]] - Workflow for scaffolding a RustyCog service with config, logging, command execution, RouteBuilder routes, permissions, and tests ( #rustycog #scaffolding #services)
- [[skills/extending-iamrusty-with-oauth-providers]] - Add a provider by updating enum mappings, infra clients, setup wiring, routes, config, and fixtures together ( #oauth #services #rust)
- [[skills/testing-rust-services-with-fixtures]] - Build serial integration tests with `TestFixture`, `DbFixtures`, provider mocks, and selective queue-backed verification ( #testing #fixtures #rust)

## References

- [[references/aiforall-platform]] - Top-level repo overview, Docker workflow, and service communication ( #reference #platform #operations)
- [[references/hive-command-execution]] - Hive registry coverage, event-publishing use cases, and the gap between registered commands and live routes ( #reference #commands #events)
- [[references/hive-data-model-and-schema]] - Organizations, members, invitations, external links, sync jobs, and the permission/resource schema behind Hive ( #reference #schema #organizations)
- [[references/hive-http-api-and-openapi-drift]] - Hive's live route table, custom HTTP errors, and the larger OpenAPI contract that is not fully wired today ( #reference #api #organizations)
- [[references/hive-runtime-and-configuration]] - `HIVE_*` config loading, queue publishing, retry settings, and outbound IAM/external-provider service config ( #reference #configuration #integrations)
- [[references/hive-service]] - Hive crate layout, runtime wiring, event publishing, and organization-management service boundaries ( #reference #organizations #architecture)
- [[references/hive-testing-and-api-fixtures]] - Real DB, JWT, and external-provider fixture patterns for the Hive API tests ( #reference #testing #fixtures)
- [[references/iamrusty-api-and-auth-flows]] - Current route surface, validated handler contracts, incomplete-registration behavior, and auth-flow doc-code drift ( #reference #api #oauth)
- [[references/iamrusty-command-execution]] - Registry composition, handler coverage, retry configuration, and the current command naming and policy model ( #reference #commands #reliability)
- [[references/iamrusty-runtime-and-security]] - Runtime config, JWT secret resolution, queue wiring, TLS, and OAuth hardening details with noted implementation drift ( #reference #configuration #security)
- [[references/iamrusty-service]] - Crate layout, runtime composition, shared `rustycog` dependencies, and event-publishing entry points ( #reference #iam #architecture)
- [[references/iamrusty-testing-and-fixtures]] - Real-infrastructure test patterns, DB fixtures, wiremocked providers, and Kafka integration-test coverage ( #reference #testing #fixtures)
- [[references/manifesto-service]] - Manifesto product model, setup, and project-service ADR summary ( #reference #projects #components)
- [[references/platform-building-blocks]] - Shared SDK crates and event-contract packages that provide common runtime, transport, and testing foundations ( #reference #sdk #events)
- [[references/rustycog-crate-catalog]] - Code-backed inventory of the current RustyCog crates, their responsibilities, and packaging caveats ( #reference #rustycog #sdk)
- [[references/rustycog-service-construction]] - Manifesto-authored build guides checked against the current command, config, HTTP, permission, and logging crates ( #reference #rustycog #architecture)
- [[references/telegraph-event-processing]] - SQS consumption, command dispatch, descriptor loading, and the current email-vs-notification processor wiring ( #reference #events #communication)
- [[references/telegraph-http-and-notification-api]] - JWT-protected notification endpoints, ownership checks, and the gap between the live route table and broader communication DTOs ( #reference #api #notifications)
- [[references/telegraph-runtime-and-configuration]] - `TELEGRAPH_*` config loading, queue routing, template paths, and local runtime drift such as ports and SMS config ( #reference #configuration #events)
- [[references/telegraph-service]] - Telegraph crate layout, parallel runtime, and the shared rustycog building blocks behind the service ( #reference #communication #architecture)
- [[references/telegraph-testing-and-smtp-fixtures]] - Real SQS, SMTP, DB, and JWT-backed integration tests for Telegraph's API and event pipeline ( #reference #testing #fixtures)

## Synthesis

## Journal
