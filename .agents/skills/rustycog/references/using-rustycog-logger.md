# Using RustyCog Logger

Use this guide when initializing tracing through `rustycog-logger`.

## Workflow

- Ensure your app config implements `HasLoggingConfig` (and `HasScalewayConfig` if Loki feature is enabled).
- Call `setup_logging(&config)` once early in startup before building long-lived components.
- Set `logging.level` for coarse control and `logging.filter` for targeted directive overrides.
- Enable Loki integration only when running with the matching feature and valid remote credentials.
- Keep tracing init in one place to avoid competing global subscriber setup.

## Common Pitfalls

- Calling logging setup repeatedly in test/runtime paths and expecting reinitialization semantics.
- Mixing manual `tracing_subscriber` setup with `setup_logging()` in the same process.
- Enabling Loki feature without complete config/credentials.

## Source files

- `rustycog/rustycog-logger/src/lib.rs`
- `rustycog/rustycog-config/src/lib.rs`

## Key types

- `HasLoggingConfig` / `HasScalewayConfig` — required traits on the app config
- `setup_logging(&config)` — single global initializer; call once
- `logging.level`, `logging.filter` — config knobs for level and per-target directives
