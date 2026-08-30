# Using RustyCog Config

Use this guide when wiring typed config with `rustycog-config`.

## Workflow

- Define your service config struct and implement `ConfigLoader` with the correct env prefix.
- Load startup config with `load_config_with_cache()` or `load_config_fresh()` depending on cache needs.
- Keep shared sections (`server`, `database`, `logging`, `queue`, `command`) aligned with RustyCog structs.
- Use `QueueConfig` as the single selector for Kafka/SQS/disabled behavior in event setup.
- Reserve `load_config_part()` for targeted reads and remember its section-based env prefixes.

## Common Pitfalls

- Assuming `load_config_part("server")` respects your service prefix instead of `SERVER_*`.
- Expecting `config/default.toml` to always be merged automatically.
- Defining queue or retry knobs in TOML but not wiring the corresponding runtime path.

## Source files

- `rustycog/rustycog-config/src/lib.rs`

## Key types

- `ConfigLoader` — trait for service config loading
- `QueueConfig` — Kafka/SQS/disabled selector consumed by event setup
- `load_config_with_cache()`, `load_config_fresh()`, `load_config_part()` — entry points
