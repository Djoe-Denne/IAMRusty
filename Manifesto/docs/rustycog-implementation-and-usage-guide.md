# RustyCog Implementation and Usage Guide (Manifesto Reference)

This guide explains how to implement and use RustyCog in this workspace, using `Manifesto` as the concrete reference.

It is intentionally precise about:

- command factory usage (`application/src/command/factory.rs`),
- configuration loading and overrides (`configuration/src/lib.rs` + `config/*.toml`),
- what is currently active in `Manifesto` vs what is only defined.

If you want a beginner-friendly, step-by-step build path for the main Rustycog crates, continue with:

- `Manifesto/docs/rustycog-service-build-guide.md`

## 1) Scope and terminology

In this repository, "factory command" means the **command registry factory** pattern (not a CLI command).  
The factory is the place where command handlers are registered and exposed to `GenericCommandService`.

## 2) RustyCog crates used by Manifesto

Core crates used in the `Manifesto` runtime path:

- `rustycog-command` (command contract, registry, execution service),
- `rustycog-config` (typed config loading + env override rules),
- `rustycog-db` (database connection pool),
- `rustycog-http` (app state + route builder + middleware),
- `rustycog-permission` (permission fetcher contracts and checks),
- `rustycog-events` (event publisher wiring).

## 3) Actual startup flow in Manifesto

The current runtime sequence is:

1. `Manifesto/src/main.rs` loads config via `manifesto_configuration::load_config()`.
2. `main` builds `Application` via `manifesto_setup::Application::new(config)`.
3. `Application::new_with_maybe_event_publisher` in `Manifesto/setup/src/app.rs`:
   - creates `DbConnectionPool`,
   - creates event publisher,
   - builds repositories/domain services/use cases,
   - builds command registry via `ManifestoCommandRegistryFactory::create_manifesto_registry(...)`,
   - wraps registry in `GenericCommandService`,
   - creates `AppState`.
4. `Application::run` calls `create_app_routes(...)` in `Manifesto/http/src/lib.rs`.
5. `RouteBuilder` builds and starts the HTTP server with auth/permission middleware.

## 4) Command factory usage (precise)

### 4.1 Current factory entrypoint

`Manifesto/application/src/command/factory.rs` exposes:

- `ManifestoCommandRegistryFactory::create_manifesto_registry(project_usecase, component_usecase, member_usecase) -> CommandRegistry`

This method:

- starts with `CommandRegistryBuilder::new()`,
- calls grouped registrations:
  - `register_project_handlers(...)`,
  - `register_component_handlers(...)`,
  - `register_member_handlers(...)`,
- returns `builder.build()`.

### 4.2 Critical contract: command type string must match registration key

For each command:

1. `impl Command for <YourCommand>` returns a `command_type()` string.
2. The factory registers the handler with a key string.
3. These two strings must match exactly.

Example pattern used in Manifesto:

```rust
// In command struct implementation:
fn command_type(&self) -> &'static str { "create_project" }

// In factory registration:
.register::<CreateProjectCommand, _>("create_project".to_string(), create_handler, error_mapper)
```

If they do not match, execution fails at runtime with `handler_not_found`.

### 4.3 Registration pattern to follow

Each command registration in the factory should include:

- command type (`register::<C, _>(...)`),
- concrete handler (`Arc<...CommandHandler>`),
- error mapper (`Arc<...ErrorMapper>`).

Pattern:

```rust
builder = builder
    .register::<CreateProjectCommand, _>(
        "create_project".to_string(),
        create_handler,
        error_mapper.clone(),
    )
    .register::<GetProjectCommand, _>(
        "get_project".to_string(),
        get_handler,
        error_mapper,
    );
```

### 4.4 Where it is used

`Manifesto/setup/src/app.rs` wires the factory result:

```rust
let command_registry = ManifestoCommandRegistryFactory::create_manifesto_registry(
    project_usecase,
    component_usecase,
    member_usecase,
);
let command_service = Arc::new(GenericCommandService::new(Arc::new(command_registry)));
```

From there, HTTP handlers call:

- `state.command_service.execute(command, context).await`

### 4.5 Add-a-command checklist

When adding a command in Manifesto:

1. Add command struct + `Command` impl in `application/src/command/*`.
2. Add command handler (`CommandHandler<C>` impl).
3. Add or reuse an error mapper.
4. Export symbols in `application/src/command/mod.rs`.
5. Register in the appropriate factory registration function.
6. Ensure handler code builds `CommandContext` and executes through `state.command_service`.
7. Add integration tests that cover success and failure (`401/403/422` where relevant).

## 5) Configuration behavior in Manifesto (precise and current)

### 5.1 How config is loaded

`Manifesto/configuration/src/lib.rs` uses `rustycog_config::load_config_fresh::<ManifestoConfig>()`.

Loader behavior:

- reads `RUN_ENV` (default: `development`),
- file selection:
  - `RUN_ENV=test` -> `config/test.toml`,
  - `RUN_ENV=production` -> `config/production.toml`,
  - any other value -> `config/development.toml`,
- applies environment variable overrides with prefix `MANIFESTO_`,
- nested keys use `__` as separator.

Important precision note:

- `config/default.toml` exists, but with the current loader implementation it is **not** automatically merged as a base file.

### 5.2 Environment variable override examples (PowerShell)

```powershell
$env:RUN_ENV="development"
$env:MANIFESTO_SERVER__PORT="8090"
$env:MANIFESTO_DATABASE__HOST="localhost"
$env:MANIFESTO_DATABASE__CREDS__USERNAME="postgres"
$env:MANIFESTO_DATABASE__CREDS__PASSWORD="postgres"
$env:MANIFESTO_SERVICE__COMPONENT_SERVICE__BASE_URL="http://localhost:9000"
cargo run -p manifesto-service
```

### 5.3 Which `Manifesto` config sections are actively used now

Actively consumed in runtime wiring:

- `server` -> used to start HTTP server (`config.server`),
- `database` -> used by `DbConnectionPool::new(&config.database)`,
- `queue` -> used by `create_multi_queue_event_publisher(&config.queue, ...)`,
- `service.component_service.base_url` -> used when creating `ComponentServiceClient`.

Defined but not currently consumed end-to-end:

- `service.component_service.timeout_seconds` is defined in config, but setup currently passes hardcoded `30` seconds.
- `logging.level` exists, but `main` initializes tracing directly and does not call `setup::config::setup_logging`.
- `[command.retry]` exists in `Manifesto/config/*.toml`, but `ManifestoConfig` currently has no `command` field and factory uses `CommandRegistryBuilder::new()` (default retry policy only).

## 6) Enabling command retry config in Manifesto factory (recommended)

To make `[command.retry]` effective in Manifesto:

1. Add `command: rustycog_config::CommandConfig` to `ManifestoConfig`.
2. Give it a default in `Default for ManifestoConfig`.
3. Pass `config.command.clone()` from setup to factory.
4. In factory, build registry with:
   - `let registry_config = RegistryConfig::from_retry_config(&command_config.retry);`
   - `let mut builder = CommandRegistryBuilder::with_config(registry_config);`

Reference implementation pattern exists in:

- `IAMRusty/application/src/command/factory.rs`

Precision note:

- `CommandConfig` supports `[command.overrides.<command_type>]`, but current registry wiring pattern in this workspace typically applies one global retry policy (`command.retry`) unless you explicitly implement per-command resolution.

## 7) Practical run/test commands (Manifesto)

Run service:

```powershell
$env:RUN_ENV="development"
cargo run -p manifesto-service
```

Run tests:

```powershell
$env:RUN_ENV="test"
cargo test -p manifesto-service
```

Run migration crate:

```powershell
cargo run -p manifesto-migration -- up
```

## 8) Quick validation checklist

- Factory registers every command implemented in `application/src/command/*`.
- Every `command_type()` string matches the factory registration key.
- `RUN_ENV` is set correctly before running.
- Required env overrides use `MANIFESTO_` prefix and `__` nesting.
- `resources/permissions/*.conf` exists for each resource used by `RouteBuilder`.
- New endpoints have tests for auth and permission behavior.

---

If you keep this factory/configuration contract strict, RustyCog stays predictable: commands are discoverable, retries are explicit, and runtime behavior matches configuration intent.
