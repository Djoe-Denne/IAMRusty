---
name: creating-wiremock-fixtures
description: Author a typed wiremock-backed fixture for an external HTTP collaborator in a RustyCog/Manifesto-style integration test suite. Use when adding a new fake for an outbound HTTP dependency, wrapping rustycog_testing::wiremock::MockServerFixture, building a per-collaborator MockService struct with chainable mock_* helpers, exposing reset() for mid-test re-arrangement, dealing with first-match-wins matcher ordering, or when the user mentions wiremock, MockServerFixture, OpenFgaMockService, stubbing HTTP, mocking an external service/provider, denial tests for permission-gated routes, fixtures under tests/fixtures/, or asserts that flip mid-test (grant-revoke-deny shapes).
---

# Creating Wiremock Fixtures

This skill explains how to build a typed wiremock-backed fixture for an external HTTP collaborator inside a RustyCog/Manifesto-style integration test suite. It mirrors the Hive `ExternalProviderMockService` and Telegraph `SmtpService` shapes that already exist in this repo.

## When to use this skill

Trigger when:

- A new service test needs to fake an outbound HTTP dependency.
- The user wants to add a `tests/fixtures/<collaborator>/` module for a mocked external service.
- Code references `rustycog_testing::wiremock::MockServerFixture`, `wiremock::MockServer`, `Mock::given(...)`, or `ResponseTemplate::new(...)`.
- A test is flaky because mocks leak between tests (suggests missing `MockServerFixture` reset semantics).
- The user is writing or extending a `MockService` struct that exposes `mock_*` helpers.

Do **not** use this skill when:

- The test needs protocol-level fidelity (real SMTP framing, Kafka acks, Postgres SQL parsing). Reach for a `testcontainers` container instead.
- The collaborator is internal to the same service; prefer an in-process trait + fake.

## Background: the shared mock server

`rustycog-testing` exposes a single `wiremock::MockServer` for the whole test process via `rustycog_testing::wiremock`. Key contract:

- The first call to `MockServerFixture::new()` binds a `TcpListener` to `127.0.0.1:3000` and starts the wiremock server. All later callers share the same `Arc<MockServer>`.
- `MockServerFixture::new()` calls `reset_all_mocks()` eagerly, so each fixture starts clean.
- `Drop for MockServerFixture` schedules another async reset on the current Tokio runtime (best effort — only works if a runtime handle is available).
- Because the server is a process-wide singleton on a fixed port, **tests must run with `#[serial]`**. Parallel tests will see each other's mounted mocks and fight for the port.

Full reference: read `obsidian/AI FOR ALL/projects/rustycog/references/wiremock-mock-server-fixture.md` only if you need the API surface or isolation semantics in detail.

## Workflow

Follow these steps in order. Don't skip the `_fixture` field or the `#[serial]` step.

### 1. Lay out the fixture module

Under the consumer crate's `tests/fixtures/<collaborator>/`, create three files:

```
tests/fixtures/<collaborator>/
├── mod.rs          # Namespace + factory struct
├── service.rs      # MockService struct + mock_* methods
└── resources.rs    # Request/response DTOs (serde-derived)
```

Wire the module into `tests/common.rs` (or wherever the test harness lives) so the namespace is reachable from individual test files.

### 2. Define the request/response DTOs (`resources.rs`)

Mirror the real collaborator's wire types. Use `serde::{Deserialize, Serialize}` so `set_body_json` can serialize them.

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionTestResponseBody {
    pub connected: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Member {
    pub username: String,
    pub email: Option<String>,
    pub display_name: Option<String>,
}
```

Keep these struct names aligned with the real collaborator's API so production code can share the types if needed later.

### 3. Build the `MockService` struct (`service.rs`)

Hold **both** the `Arc<MockServer>` (for mounting) and the `MockServerFixture` (for drop-time reset). The fixture goes in a `_fixture` field — nothing reads it; its only job is to live until the service is dropped.

```rust
use rustycog_testing::wiremock::MockServerFixture;
use std::sync::Arc;
use wiremock::{matchers::{method, path, body_string_contains}, Mock, MockServer, ResponseTemplate};

use super::resources::*;

pub struct ExternalProviderMockService {
    server: Arc<MockServer>,
    _fixture: MockServerFixture,
}

impl ExternalProviderMockService {
    pub async fn new() -> Self {
        let fixture = MockServerFixture::new().await;
        let server = fixture.server();
        Self { server, _fixture: fixture }
    }

    pub fn base_url(&self) -> String {
        self.server.uri()
    }
}
```

For protocol-shaped fakes that need host/port separately (e.g. SMTP-as-HTTP), expose `host()`, `port()`, and `uri()` instead of (or alongside) `base_url()`.

### 4. Add one async `mock_*` method per scenario

Return `&Self` so callers can chain arrangements. Pin both `method(...)` and `path(...)` — loose matches catch unrelated requests since the server is process-wide.

```rust
pub async fn mock_validate_config_ok(&self) -> &Self {
    Mock::given(method("POST"))
        .and(path("/config/validate"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(serde_json::json!({"ok": true}))
        )
        .mount(&*self.server)
        .await;
    self
}

pub async fn mock_validate_config_fail(&self, message_contains: &str) -> &Self {
    Mock::given(method("POST"))
        .and(path("/config/validate"))
        .and(body_string_contains(message_contains))
        .respond_with(
            ResponseTemplate::new(400)
                .set_body_json(serde_json::json!({"error": message_contains}))
        )
        .mount(&*self.server)
        .await;
    self
}
```

### 5. Wrap construction in a namespace struct (`mod.rs`)

Match the convention used by `ExternalProviderFixtures` and `SmtpFixtures`:

```rust
pub mod resources;
pub mod service;

pub struct ExternalProviderFixtures;

impl ExternalProviderFixtures {
    pub async fn service() -> service::ExternalProviderMockService {
        service::ExternalProviderMockService::new().await
    }
}
```

Tests then read as `let provider = ExternalProviderFixtures::service().await;`.

### 6. Compose multi-step scenarios when needed

For protocol flows (SMTP handshake, multi-call APIs), add higher-level methods that call the lower-level `mock_*` helpers in sequence. For ad-hoc flows, expose a builder.

```rust
pub async fn mock_successful_email_send(&self, email: &SmtpEmail) -> &Self {
    self.mock_greeting(SmtpResponse::service_ready())
        .await
        .mock_ehlo(SmtpCapabilities::default_localhost())
        .await
        .mock_mail_from(&email.from, SmtpResponse::ok())
        .await;
    for to_addr in &email.to {
        self.mock_rcpt_to(to_addr, SmtpResponse::ok()).await;
    }
    self.mock_data(email, SmtpResponse::ok())
        .await
        .mock_quit(SmtpResponse::closing())
        .await
}
```

### 7. Wire the fake into the service-under-test

Point the service's config at `mock.base_url()` (or `host()`/`port()`). The simplest path is to construct the `MockService` first and then build the test config with the fake's URL.

### 8. Mark every test that touches the fixture `#[serial]`

Use `serial_test::serial` (already a dev-dep across the repo). The shared singleton + fixed port means parallel tests will clobber each other.

```rust
#[tokio::test]
#[serial]
async fn external_provider_validates_config() {
    let provider = ExternalProviderFixtures::service().await;
    provider.mock_validate_config_ok().await;
    // ... drive the service, assert behavior ...
}
```

### 9. Expose `reset()` for mid-test re-arrangement

If your fixture is held across multiple test phases (e.g. a setup-time permissive default plus per-test overrides), expose `reset()` from the wrapper:

```rust
pub async fn reset(&self) {
    self._fixture.reset().await;
}
```

Tests that need to **override** a stub mounted earlier in the same test (or by `setup_test_server`) call `reset()` first, then mount the new stubs. wiremock matches in **registration order, first-match wins**, so a per-tuple deny mounted after an existing catch-all allow will never fire — the reset is the only way to give the new stub priority.

Example shapes that need this:

- **Denial test** when the harness pre-mounts a permissive default:
  ```rust
  openfga.reset().await;
  openfga.mock_check_deny(member, Permission::Admin, resource).await;
  ```
- **Phase-flip test** (grant ➜ revoke ➜ deny):
  ```rust
  // Phase 1
  openfga.reset().await;
  openfga.mock_check_allow(member, Admin, resource).await;
  // ... API call that "revokes" ...
  // Phase 2
  openfga.reset().await;
  openfga.mock_check_deny(member, Admin, resource).await;
  ```

### 10. Beware of caches in front of the wiremock-faked path

If the production code wraps the call in a cache (e.g. `CachedPermissionChecker`), the second request for the same key never reaches the wiremock fake — so re-arranging stubs has no effect. Make the cache TTL configurable in the production type and set it to 0 in test configs. The canonical example is the `cache_ttl_seconds` field added to `OpenFgaClientConfig`; `Manifesto/setup/src/app.rs` skips the `CachedPermissionChecker` decoration entirely when the value is 0.

## Matcher cheat sheet

| Need | Matcher | Notes |
|------|---------|-------|
| HTTP method | `method("POST")` | Always pin this. |
| Exact URL path | `path("/foo/bar")` | Always pin this. |
| Body substring | `body_string_contains("...")` | Best for negative scenarios and content-aware stubs. For variable text bodies, match on a few significant words (3+ chars) to survive template changes. |
| Header value | `header("authorization", "Bearer ...")` | Use when the collaborator distinguishes responses on auth/content-type. |
| Query param | `query_param("key", "value")` | Use for paged or filtered endpoints. |
| JSON body shape | `body_json(serde_json::json!({...}))` | Use when an exact JSON match is required. |

Response side:

```rust
ResponseTemplate::new(200)
    .set_body_json(typed_dto)
    .insert_header("content-type", "application/json")
```

Map error scenarios by deriving the status from the modeled response:

```rust
ResponseTemplate::new(if response.code >= 400 { 400 } else { 200 })
```

## Inspection (assert what was sent)

```rust
pub async fn received_requests(&self) -> Vec<wiremock::Request> {
    self.server.received_requests().await.unwrap_or_default()
}

pub async fn verify_email_sent(&self, subject: &str, recipient: &str) -> bool {
    self.received_requests()
        .await
        .iter()
        .any(|req| {
            req.url.path() == "/smtp/data" && {
                let body = String::from_utf8_lossy(&req.body);
                body.contains(subject) && body.contains(recipient)
            }
        })
}
```

Wrap this in named helpers (`verify_email_sent`, `email_count`, `verify_member_lookup`) so test bodies stay declarative.

## Common pitfalls

- **Forgetting `#[serial]`.** The wiremock server is shared across the whole test process and bound to a fixed port. Parallel tests will see each other's mocks and may fail to bind.
- **Holding only `Arc<MockServer>`.** If you discard the `MockServerFixture`, you lose the drop-time reset and the next test inherits your mounted mocks. Always keep it in a `_fixture` field.
- **Bypassing `MockServerFixture::new()`.** Constructing helpers from `get_mock_server()` directly skips the eager reset. Mocks will stack up across tests because wiremock matches the first registered mock that fits.
- **Loose matchers.** A `Mock::given(method("POST"))` with no `path(...)` will swallow requests intended for sibling fixtures arranged in the same test.
- **Dropping the fixture outside a Tokio runtime.** Auto-reset on `Drop` only fires if `tokio::runtime::Handle::try_current()` succeeds. Synchronous teardown skips cleanup silently.
- **Trying to relocate the listener.** Port `127.0.0.1:3000` is hard-coded. Configure the service-under-test to point at the fake's `base_url()` rather than trying to move the server.
- **Mounting a per-tuple deny on top of a catch-all allow without `reset()` first.** First-match-wins means the catch-all swallows the request before the deny is considered. Always `reset()` first when overriding a default mounted earlier.
- **Forgetting downstream caches.** A second request for the same cache key never reaches the fake — re-arranging stubs has no effect until the cache TTL expires. Make the production cache TTL configurable and disable it in test configs (`cache_ttl_seconds = 0` is the established pattern).

## Checklist before merging the fixture

- [ ] Module layout: `mod.rs`, `service.rs`, `resources.rs`.
- [ ] `MockService` holds both `server: Arc<MockServer>` and `_fixture: MockServerFixture`.
- [ ] Constructor goes through `MockServerFixture::new().await`.
- [ ] At least one `mock_*` method exists per scenario the tests need; each returns `&Self`.
- [ ] `base_url()` (or `host()`/`port()`/`uri()`) exposed for configuring the service-under-test.
- [ ] Namespace factory struct (`<Name>Fixtures::service().await`) provided.
- [ ] All consuming tests are `#[serial]`.
- [ ] `reset()` exposed when the fixture is held across multiple test phases or when a harness pre-mounts a default that tests will override.
- [ ] Inspection helpers added if the suite needs to assert on outbound requests.
- [ ] If the production code caches the wiremock-faked call, the cache TTL is configurable and the test config sets it to 0.

## Reference examples in this repo

Read these only when the situation calls for it — not up-front.

- Hive's external-provider fake: `Hive/tests/fixtures/external_provider/{mod.rs, service.rs, resources.rs}` — small, REST-shaped, one mock method per endpoint.
- Telegraph's SMTP-as-HTTP fake: `Telegraph/tests/fixtures/smtp/{mod.rs, service.rs, resources.rs}` — protocol-shaped, scenario composition, scenario builder, request inspection.
- OpenFGA `Check` fake: `rustycog/rustycog-testing/src/permission/{mod.rs, service.rs, resources.rs}` — in-crate fixture (lives inside `rustycog-testing` so every consumer of `rustycog-permission` reuses it). Demonstrates `reset()`, the `client_config()` convenience that returns a fully-arranged `OpenFgaClientConfig`, the cache-companion-setting pattern (`cache_ttl_seconds`), and the wildcard helpers `mock_check_allow_wildcard` / `mock_check_deny_wildcard` for anonymous-public-read tests (see [[concepts/anonymous-public-read-via-wildcard-subject]] in the wiki).
- Shared fixture implementation: `rustycog/rustycog-testing/src/wiremock/mod.rs` — `MockServerFixture` + singleton lifecycle.
- Canonical consumer wiring: `Manifesto/tests/common.rs`, `Manifesto/setup/src/app.rs`, `Manifesto/config/test.toml`, and `Manifesto/tests/component_api_tests.rs` (read tests 4 / 5 / 6 for the deny / multi-tuple / phase-flip arrangement patterns respectively).

## Related skills

- The `rustycog` project skill (`.cursor/skills/rustycog/SKILL.md`) for surrounding RustyCog wiring decisions.
- `rustycog/references/using-rustycog-testing.md` for the broader test-server bootstrap path that test cases plug into alongside this fixture.
