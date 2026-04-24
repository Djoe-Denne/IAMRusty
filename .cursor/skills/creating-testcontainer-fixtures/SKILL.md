---
name: creating-testcontainer-fixtures
description: Author a real Docker-backed testcontainer fixture for protocol-level integration tests in a RustyCog/Manifesto-style service. Use when adding a new container-backed fake for an outbound dependency (Postgres, LocalStack-SQS, Kafka, MailHog, Redis, Mongo, MinIO, Vault, NATS, S3, etc.), wrapping the `testcontainers` crate behind a typed singleton, deciding between shared placement under `rustycog-testing/src/common/` and service-local placement under `<Service>/tests/fixtures/`, extending `ServiceTestDescriptor` with a new capability flag like `has_redis`, surviving leaked containers across `cargo test` runs, picking between `port = 0` random-port + env-var publication versus fixed `with_mapped_port`, wiring a real-protocol client API onto a `Test<Thing>` struct, when the user mentions `testcontainers`, `GenericImage`, `ContainerAsync`, `with_mapped_port`, `cleanup_existing_*_container`, `OnceLock<Arc<Mutex<Option<Arc<...>>>>>`, MailHog, LocalStack, Kafka acks, real SMTP framing, Postgres SQL fidelity, or asks how the SQS/Kafka/SMTP testcontainer fixtures are built.
---

# Creating Testcontainer Fixtures

This skill explains how to add a real Docker-backed testcontainer fixture to the workspace. It mirrors the existing `sqs_testcontainer.rs`, `kafka_testcontainer.rs`, and `Telegraph/tests/fixtures/smtp/testcontainer.rs` shapes — pick the same pattern when adding the next protocol (Redis, Mongo, MinIO, Vault, NATS, etc.).

## When to use this skill

Trigger when:

- A test needs to assert against the **real** protocol, not just "what the service would have sent". Examples: SMTP framing, Kafka ack semantics, Postgres SQL behavior, real serializer output, S3 multipart upload mechanics.
- The user wants to add a `*_testcontainer.rs` module under `rustycog/rustycog-testing/src/common/` or a `tests/fixtures/<thing>/testcontainer.rs` under a service.
- Code references `testcontainers::{GenericImage, ContainerAsync, ImageExt, runners::AsyncRunner}` or `with_mapped_port`.
- A test is flaky because the previous run left a container holding the port (`docker ps` shows a stale `<service>_test-<thing>` container).
- The user asks how the existing SQS/Kafka/SMTP fixtures are built and wants to add a new one in the same shape.

Do **not** use this skill when:

- The test only needs to assert what the service *would have sent* under controlled responses. Reach for [.cursor/skills/creating-wiremock-fixtures/SKILL.md](../creating-wiremock-fixtures/SKILL.md) instead — it's faster, more deterministic, and parallelizes better.
- The collaborator is internal to the same service. Prefer an in-process trait + fake.
- The protocol is HTTP and a wiremock fake would suffice. Telegraph keeps both an `SmtpService` (wiremock) and a `TestSmtp` (MailHog testcontainer) — that's the right shape only when you genuinely need both.

## Background: the existing fixtures

`rustycog-testing` already ships two shared testcontainer fixtures under `rustycog/rustycog-testing/src/common/`:

- `sqs_testcontainer.rs` — LocalStack 3.0.2 with `SERVICES=sqs`, container name `iam_test-localstack-sqs`, port published into the test config via env-var mutation (`IAM_QUEUE__SQS__PORT`, etc.).
- `kafka_testcontainer.rs` — same scaffold, different image and env-var prefix.

Telegraph adds a service-local one at `Telegraph/tests/fixtures/smtp/testcontainer.rs`:

- `TestSmtp` runs MailHog (`mailhog/mailhog:latest`), container name `telegraph_test-smtp`, with two pinned mapped ports (1025 for SMTP, 8025 for the admin REST API).

All three follow the same singleton + defensive-Docker-cleanup pattern. **Read at least one of them** before authoring a new fixture — they encode several non-obvious lifecycle decisions that the type signatures alone don't make clear.

Full wiki reference: `obsidian/AI FOR ALL/skills/creating-testcontainer-fixtures.md`. Read it only if you need the prose rationale; this skill is the actionable version.

## Workflow

Follow these steps in order. Step 0 is decision-making; everything else is code.

### 0. Decide where the fixture lives

| Where | When |
|---|---|
| `rustycog/rustycog-testing/src/common/<thing>_testcontainer.rs` (shared) | Production has a `rustycog-*` shared client for this infra (event publisher, DB pool). Multiple services will reuse the fixture. |
| `<Service>/tests/fixtures/<thing>/testcontainer.rs` (service-local) | Only this service speaks the protocol, or the wire-level parsing is service-specific. |

Heuristic: SQS and Kafka are shared because every event-driven service uses them; MailHog is service-local because only Telegraph speaks SMTP. If unsure, start service-local — promoting later is straightforward, demoting from shared is not.

### 1. Extend `ServiceTestDescriptor` (shared fixtures only)

Skip this step entirely for service-local fixtures.

`rustycog/rustycog-testing/src/common/service_test_descriptor.rs` defines the trait. The capability flags are **not** defaulted, so adding a new one is a breaking change every implementor must absorb in the same change-set:

```rust
#[async_trait]
pub trait ServiceTestDescriptor<T>: Send + Sync + 'static {
    type Config: ...;
    async fn build_app(&self, ...) -> anyhow::Result<()>;
    async fn run_app(&self, ...) -> anyhow::Result<()>;
    async fn run_migrations_up(&self, ...) -> anyhow::Result<()>;
    async fn run_migrations_down(&self, ...) -> anyhow::Result<()>;
    fn has_db(&self) -> bool;
    fn has_sqs(&self) -> bool;
    fn has_redis(&self) -> bool;  // ← new flag
}
```

Then update every implementor: `IamServiceTestDescriptor`, `TelegraphTestDescriptor`, `HiveTestDescriptor`, `ManifestoTestDescriptor`, plus any in-tree test fixtures. Most return `false`. Have the shared fixture builder branch on the flag so services that don't need the container pay zero startup cost.

### 2. Add the singleton + container wrapper

Two static items at the top of the file:

```rust
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};
use testcontainers::{runners::AsyncRunner, ContainerAsync, GenericImage, ImageExt};
use tokio::sync::Mutex;
use tracing::{debug, info, warn};

static TEST_<THING>_CONTAINER: OnceLock<Arc<Mutex<Option<Arc<Test<Thing>Container>>>>> = OnceLock::new();
static <THING>_CLEANUP_REGISTERED: AtomicBool = AtomicBool::new(false);

pub struct Test<Thing>Container {
    container: ContainerAsync<GenericImage>,
    pub endpoint_url: String,
    pub port: u16,
}

impl Test<Thing>Container {
    pub async fn cleanup(self) {
        info!("Stopping and removing test <Thing> container");
        if let Err(e) = self.container.stop().await { warn!("Failed to stop: {}", e); }
        if let Err(e) = self.container.rm().await { warn!("Failed to remove: {}", e); }
    }
}
```

Why each layer of `OnceLock<Arc<Mutex<Option<Arc<...>>>>>` exists:

- `OnceLock` — lazy single initialization of the slot.
- `Arc<Mutex<...>>` — share the slot across the whole process; serialize start/stop.
- `Option<Arc<...>>` — distinguish "not started" from "running, here's the handle".
- Inner `Arc<...>` — multiple test fixtures hold cheap clones without fighting for ownership.

Do not flatten this — every layer prevents a real race condition observed in the existing fixtures.

### 3. Add the `Test<Thing>` fixture struct

This is the public surface tests touch. It owns the typed client:

```rust
pub struct Test<Thing> {
    pub client: <ClientType>,         // SDK or reqwest::Client
    pub endpoint_url: String,
    pub port: u16,
    // ... whatever else tests need to read
}

impl Test<Thing> {
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let (_container, config) = get_or_create_test_<thing>_container().await?;
        // ... build client, set env vars, wait for ready, return ...
    }
}
```

Construct the SDK client *after* the container starts and *after* `wait_for_ready` (step 6) succeeds. A premature client build sees timeouts on the first call.

### 4. Implement `get_or_create_test_<thing>_container()`

Heart of the fixture. Order matters — match `sqs_testcontainer.rs` exactly:

```rust
async fn get_or_create_test_<thing>_container()
    -> Result<(Arc<Test<Thing>Container>, <Config>), Box<dyn std::error::Error>>
{
    let container_mutex = TEST_<THING>_CONTAINER.get_or_init(|| Arc::new(Mutex::new(None)));
    let mut container_guard = container_mutex.lock().await;

    if let Some(ref container) = *container_guard {
        let config = load_config_part::<<Config>>("...").expect("...");
        return Ok((container.clone(), config));
    }

    info!("Creating new <Thing> test container");

    // 1. Defensive eviction of any stale container from a previous run.
    cleanup_existing_<thing>_container().await;

    // 2. Clear cached random port (if your config crate caches it like SqsConfig does).
    <Config>::clear_port_cache();

    // 3. Resolve the port the test config wants.
    let config = load_config_part::<<Config>>("...").expect("...");
    let port = config.actual_port();

    // 4. Build and start the image.
    let image = GenericImage::new("<image>", "<tag>")
        .with_env_var("KEY", "value")
        .with_container_name("<service>_test-<thing>")
        .with_mapped_port(port, testcontainers::core::ContainerPort::Tcp(<container_port>));

    info!("Starting <Thing> container on port {}...", port);
    let container = image.start().await?;

    let endpoint_url = format!("http://localhost:{}", port);

    let test_container = Arc::new(Test<Thing>Container { container, endpoint_url, port });
    *container_guard = Some(test_container.clone());

    register_<thing>_cleanup_handler().await;
    Ok((test_container, config))
}
```

Critical: `cleanup_existing_<thing>_container().await` runs *before* port resolution, so `Ctrl-C`-leaked containers are evicted before the new one tries to bind.

### 5. Add the defensive Docker cleanup

`testcontainers` cleanup only fires when `Drop` runs. `Ctrl-C` and panics in startup skip it. Add a `docker stop` / `docker rm -f` shellout as the safety net:

```rust
async fn cleanup_existing_<thing>_container() {
    use std::process::Command;

    debug!("Checking for existing <Thing> test containers");
    let containers = ["<service>_test-<thing>"];
    for container_name in &containers {
        let _ = Command::new("docker").args(&["stop", container_name]).output();
        let _ = Command::new("docker").args(&["rm", "-f", container_name]).output();
        debug!("Cleaned up container: {}", container_name);
    }
}

async fn register_<thing>_cleanup_handler() {
    if <THING>_CLEANUP_REGISTERED.swap(true, Ordering::SeqCst) { return; }
    info!("Registering <Thing> test container cleanup handler");
}
```

The container name **must be unique per fixture** — `iam_test-localstack-sqs`, `telegraph_test-smtp`, etc. Reusing a name across fixtures will let one fixture's cleanup tear down another fixture's running container.

### 6. Add a protocol-aware readiness probe

Containers report "started" before they accept connections. Skipping the probe gives the first test a flaky timeout.

```rust
async fn wait_for_ready(&self) -> Result<(), Box<dyn std::error::Error>> {
    let url = format!("http://{}:{}/<healthz>", self.host, self.port);
    for _ in 0..30 {
        match self.client.get(&url).send().await {
            Ok(r) if r.status().is_success() => {
                debug!("<Thing> is ready");
                return Ok(());
            }
            _ => tokio::time::sleep(Duration::from_millis(100)).await,
        }
    }
    Err("readiness timeout".into())
}
```

Pick a probe that succeeds *only* once the protocol you actually use is up:

- LocalStack: `GET /_localstack/health`.
- MailHog: `GET /api/v1/messages`.
- Postgres: `SELECT 1` via the SQLx/SeaORM pool.
- Kafka: list topics or describe cluster — broker leader election is post-startup.

A bare TCP-port-open check is **not** sufficient for stateful services — Kafka in particular accepts TCP long before it has a controller.

### 7. Wire the port into `test.toml`

Two patterns. Default to the first; switch to the second only when the protocol or admin URL needs a stable port.

#### Pattern A: `port = 0` + env-var publication (preferred)

`test.toml`:

```toml
[<config>]
host = "localhost"
port = 0  # OS-assigned random port
```

Fixture publishes the resolved port via env-var mutation **inside the container constructor**:

```rust
unsafe {
    std::env::set_var("<SERVICE>__<CONFIG>__HOST", host);
    std::env::set_var("<SERVICE>__<CONFIG>__PORT", &port.to_string());
    // ... and any other connection knobs
}
```

The `unsafe` block is unavoidable — `std::env::set_var` is `unsafe` since the env API was tightened. Confine env mutation to this one function so the surface stays small.

##### 7a. Shape the typed config the same way

Pattern A only works if the typed config struct already exposes `host: String` + `port: u16` separately (with `port = 0` reserved for "pick random"). If the struct currently carries a single `api_url: String` (or `endpoint_url`, `connection_string`, etc.), **refactor it first** — otherwise the fixture has no way to publish a typed port the application config can pick up.

The canonical shape, mirrored by `DatabaseConfig`, `SqsConfig`, and `OpenFgaClientConfig`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct <Thing>Config {
    #[serde(default = "default_<thing>_scheme")]
    pub scheme: String,             // optional: only when the URL prefix isn't fixed
    #[serde(default = "default_<thing>_host")]
    pub host: String,
    #[serde(default = "default_<thing>_port")]
    pub port: u16,                  // 0 ⇒ resolve at first call to actual_port()
    // ... protocol-specific fields ...
}

static <THING>_PORT_CACHE: OnceLock<Arc<Mutex<HashMap<String, u16>>>> = OnceLock::new();

impl <Thing>Config {
    /// Reconstruct the URL from scheme/host/(actual)port.
    pub fn url(&self) -> String {
        format!("{}://{}:{}", self.scheme, self.host, self.actual_port())
    }

    /// Resolve `port == 0` to a free random port; cache process-wide so
    /// the fixture and the app boot path agree on the same number.
    pub fn actual_port(&self) -> u16 {
        if self.port == 0 {
            let key = format!("<thing>:{}", self.host);
            let cache = <THING>_PORT_CACHE.get_or_init(|| Arc::new(Mutex::new(HashMap::new())));
            let mut guard = cache.lock().unwrap();
            if let Some(&p) = guard.get(&key) { return p; }
            let p = pick_free_port();
            guard.insert(key, p);
            p
        } else {
            self.port
        }
    }

    pub fn clear_port_cache() {
        if let Some(cache) = <THING>_PORT_CACHE.get() { cache.lock().unwrap().clear(); }
    }
}
```

Two non-obvious requirements:

- **The cache must be keyed on something stable**, not on the `<Thing>Config` instance. Every reload of the typed config produces a *new* struct, but the fixture and the application boot path must resolve to the *same* port. Key by `host` (or `host + region`, etc.) so a reload still hits the cached value.
- **Call `<Thing>::clear_port_cache()` inside `get_or_create_*_container()` before `actual_port()`**, so a fresh container start picks a fresh port instead of reusing one whose container has been torn down (e.g. across `cargo test` runs that share `OnceLock`-backed singletons).

If the config struct genuinely lives outside `rustycog-config`, the `PORT_CACHE` and helpers belong in the same crate as the struct. Prefer putting reusable infra config in `rustycog-config` first (as `OpenFgaClientConfig` now does), and only keep the cache elsewhere for service-local config types.

After refactoring the struct, propagate the change: every `test.toml` / `default.toml` / `development.toml` that previously set the single-field URL must be rewritten to `host = "..."` + `port = <fixed-or-0>`, and any production callers that read `config.url` directly must switch to `config.url()` (or whatever you named the builder method).

#### Pattern B: Fixed `with_mapped_port`

Only when a stable URL matters (admin API endpoints, well-known port the production code hard-codes). MailHog's choice:

```toml
# Telegraph/config/test.toml
[communication.email.smtp]
port = 1025
```

```rust
.with_mapped_port(smtp_config.port, ContainerPort::Tcp(1025))
.with_mapped_port(8025, ContainerPort::Tcp(8025))  // admin API
```

Cost: parallel CI jobs sharing a host kernel will collide. Only safe when each runner has its own kernel namespace.

### 8. Wire into `setup_test_server()`

Construct the container *before* the app boots, then clear any prior in-container state at the top of `setup_test_server()` for isolation. Mirror the existing pattern:

```rust
pub async fn setup_test_server() -> (TestServer, TestSqs, ...) {
    // 1. Start the container (or reuse the singleton).
    let sqs = TestSqs::new().await.expect("failed to start SQS test container");

    // 2. Clear in-container state so each test starts on a known floor.
    sqs.purge_queue().await.ok();

    // 3. Now boot the app — config has already been mutated with the resolved port.
    let server = ...;

    (server, sqs, ...)
}
```

If multiple tests need to drive the container (assert messages, clear state mid-test), expand the tuple `setup_test_server()` returns the way Manifesto's harness returns the `OpenFgaMockService` handle. Read `Manifesto/tests/common.rs` for the canonical "factory returns a service-handle alongside the boot bundle" shape.

### 9. Add typed assertion helpers (do not let tests speak the raw protocol)

Wrap the SDK client with named methods that return typed values. Test bodies must read declaratively:

```rust
// SQS example (from sqs_testcontainer.rs)
pub async fn wait_for_messages(&self, expected_count: usize, max_wait_secs: u64)
    -> Result<Vec<String>, Box<dyn std::error::Error>>;
pub async fn purge_queue(&self) -> Result<(), Box<dyn std::error::Error>>;

// MailHog example (from Telegraph's TestSmtp)
pub async fn get_emails(&self) -> Result<Vec<TestEmail>, ...>;
pub async fn email_count(&self) -> Result<usize, ...>;
pub async fn has_email(&self, subject: &str, recipient: &str) -> Result<bool, ...>;
pub async fn clear_emails(&self) -> Result<(), ...>;
```

If a test ends up calling `client.send().queue_url(...).message_body(...)` directly, that's a missing helper.

### 10. Mark every test that touches the fixture `#[serial]`

Same rule as the wiremock fixture. The singleton + shared port mean parallel tests will see each other's state and clobber each other's connections.

```rust
#[tokio::test]
#[serial]
async fn it_publishes_an_event() { ... }
```

## Common pitfalls

- **Reusing a container name across fixtures.** `cleanup_existing_*` matches on exact name; a duplicated name means one fixture's cleanup tears down another fixture's running container. Always namespace as `<service>_test-<thing>`.
- **Skipping `cleanup_existing_*_container().await` before `image.start()`.** `Ctrl-C` between runs leaves the container alive. The next run's `start()` fails with a confusing "address already in use" instead of evicting the stale one cleanly.
- **Holding only a clone of the inner client without keeping the singleton populated.** If the only `Arc<Test<Thing>Container>` reference goes out of scope during teardown, the container drops and the next test pays full restart cost. Keep the `OnceLock` slot populated for the whole test process.
- **Leaving `port = 0` in config but hard-coding the port elsewhere.** The whole point of `port = 0` is the fixture publishes the resolved port via env. Any other config file or `const` with a fixed port baked in will route the service-under-test to the wrong place silently.
- **Polling without a deadline.** Both `wait_for_messages` and `wait_for_ready` cap their loops with `max_wait_secs` / a fixed iteration count. A bare `while !ready { sleep(100ms).await }` will hang the suite when the container fails to start, with no useful error.
- **Skipping `#[serial]`.** All shared-singleton fixtures rely on the singleton not being raced. Same rule as the wiremock fixture.
- **Calling `set_var` outside the fixture constructor.** Env mutation is process-global; if multiple call sites set conflicting values, the last writer wins. Confine env mutation to `get_or_create_*`.
- **Letting a production cache mask the round-trip.** Same caveat as for wiremock — a `Cached*Client` decorator added by production wiring will swallow the second request. Make the cache TTL configurable and disable it in tests (`cache_ttl_seconds = 0` is the established pattern).
- **TCP-port-open as a readiness probe for Kafka or any stateful service.** Kafka accepts TCP long before it has a controller. Use a protocol-level probe (list topics, `SELECT 1`, etc.).
- **Picking fixed `with_mapped_port` for a generic fixture.** It's fine for Telegraph's MailHog (single-tenant, dev-host) but wrong for any fixture that might run in parallel CI. Default to `port = 0` + env publication.
- **Calling `TcpListener::bind("127.0.0.1:0")` directly inside the fixture instead of going through the typed config.** Tempting because it works without touching the config struct, but it bypasses `actual_port()`'s shared cache — the application boot path then has no way to learn the same port without the fixture round-tripping it through env vars. Always wire the typed `<Thing>Config` first (step 7a), call `<Config>::actual_port()` from the fixture, and let the cache do the de-duplication. The OpenFGA fixture initially shipped with a raw `TcpListener::bind` and had to be back-fitted later — don't repeat that.
- **Carrying a single `api_url` / `endpoint_url` / `connection_string` on the typed config.** Pattern A only works with `host: String` + `port: u16` *separately* exposed at the same level as the env-var prefix. If the existing config is a flat URL string, refactor it (step 7a) before authoring the fixture — there is no clean way to publish a typed port into a URL-shaped slot.
- **Adding the new descriptor flag with a default.** The trait is intentionally not defaulted so adding `has_redis` is a compile error in every service that hasn't opted in. Don't paper over this with `fn has_redis(&self) -> bool { false }` on the trait — make every descriptor declare it explicitly.

## Checklist before merging the fixture

- [ ] Decision recorded: shared (`rustycog-testing/src/common/`) vs. service-local (`<Service>/tests/fixtures/`).
- [ ] If shared: `ServiceTestDescriptor` got the new flag; every implementor declares it.
- [ ] `static TEST_<THING>_CONTAINER: OnceLock<Arc<Mutex<Option<Arc<...>>>>>` declared (do not flatten the layers).
- [ ] `Test<Thing>Container` owns the `ContainerAsync<GenericImage>` and exposes `cleanup()`.
- [ ] `get_or_create_*` calls `cleanup_existing_*` first, then resolves port, then builds image with a unique container name, then starts.
- [ ] `cleanup_existing_*` shellouts to `docker stop` and `docker rm -f` with a unique name.
- [ ] `wait_for_ready()` uses a protocol-aware probe with a bounded retry loop.
- [ ] Typed `<Thing>Config` exposes `host: String` + `port: u16` (not a single `api_url`), with `port = 0` ⇒ `actual_port()` random + cached + `clear_port_cache()` (step 7a). Pre-existing single-URL configs are refactored in the same change-set.
- [ ] Port wiring: prefer `port = 0` + env-var mutation in the constructor; use fixed `with_mapped_port` only when a stable URL is required.
- [ ] All env mutation lives inside the fixture constructor, in one `unsafe` block.
- [ ] `setup_test_server()` constructs the fixture *before* booting the app and clears prior in-container state at the top.
- [ ] Public API exposes typed assertion helpers — no raw SDK calls in test bodies.
- [ ] All consuming tests are `#[serial]`.
- [ ] If the production code caches calls to this infra, the cache TTL is configurable and the test config sets it to 0.

## Reference examples in this repo

Read these only when the situation calls for it — not up-front.

- **Shared SQS fixture (LocalStack)**: `rustycog/rustycog-testing/src/common/sqs_testcontainer.rs` — canonical singleton pattern, env-var publication, defensive Docker cleanup, typed assertion helpers (`wait_for_messages`, `purge_queue`, `verify_event_published`).
- **Shared Kafka fixture**: `rustycog/rustycog-testing/src/common/kafka_testcontainer.rs` — same scaffold, different image and env-var prefix.
- **Shared OpenFGA fixture**: `rustycog/rustycog-testing/src/common/openfga_testcontainer.rs` — protocol-aware fixture that loads the consumer's `[openfga]` config via `load_config_part::<OpenFgaClientConfig>("openfga")`, calls `actual_port()` to materialize a `port = 0` config into a free random host port, and publishes per-service `_OPENFGA__SCHEME/HOST/PORT/STORE_ID/AUTHORIZATION_MODEL_ID` env vars. Demonstrates the host/port split documented in step 7a (the `OpenFgaClientConfig` was originally a single `api_url: String` and was refactored alongside the fixture so `port = 0` would Just Work).
- **Service-local MailHog fixture**: `Telegraph/tests/fixtures/smtp/testcontainer.rs` — fixed-mapped-port pattern, REST-API client wrapping (`get_emails`, `email_count`, `has_email`), `cleanup_container()` and `cleanup_existing_smtp_container()` for orderly + defensive teardown.
- **Descriptor trait**: `rustycog/rustycog-testing/src/common/service_test_descriptor.rs` — the non-defaulted capability flags (`has_db`, `has_sqs`).
- **Test-config wiring patterns**: `IAMRusty/config/test.toml` (port = 0), `Telegraph/config/test.toml` (fixed mapped port).
- **`setup_test_server()` shape** when the harness needs to return a fixture handle alongside the server: `Manifesto/tests/common.rs` (mirrors the OpenFGA mock pattern).

## Sister skill

When the test only needs to assert "what the service would have sent" rather than round-tripping the protocol, use [.cursor/skills/creating-wiremock-fixtures/SKILL.md](../creating-wiremock-fixtures/SKILL.md) instead. Telegraph deliberately keeps both an `SmtpService` (wiremock) and a `TestSmtp` (MailHog testcontainer) side by side because they answer different questions.

## Related

- The `rustycog` project skill (`.cursor/skills/rustycog/SKILL.md`) for surrounding RustyCog wiring decisions.
- `rustycog/references/using-rustycog-testing.md` for the broader test-server bootstrap path that fixtures plug into.
- Wiki page `obsidian/AI FOR ALL/skills/creating-testcontainer-fixtures.md` for the prose rationale behind each step here.
