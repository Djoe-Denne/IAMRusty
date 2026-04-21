# Using RustyCog Command

Use this guide when composing `rustycog-command`.

## Workflow

- Define one command struct per operation and implement `Command` (`command_type`, `command_id`, `validate`).
- Implement `CommandHandler<YourCommand>` and keep business logic in use cases/services.
- Register handlers through `CommandRegistryBuilder` with stable command keys and error mappers.
- Build `RegistryConfig` from runtime retry config when service policy should be externally configurable.
- Expose one shared command service in `AppState` so HTTP and queue adapters reuse the same execution path.

## Common Pitfalls

- Letting command key strings drift from `command_type()` values.
- Forgetting that `max_attempts = 0` disables retries entirely.
- Registering handlers in multiple places and creating split command surfaces.

## Source files

- `rustycog/rustycog-command/src/lib.rs`
- `rustycog/rustycog-command/src/registry.rs`
- `rustycog/rustycog-config/src/lib.rs`

## Key types

- `Command`, `CommandHandler<T>` — define and handle one command per operation
- `CommandRegistryBuilder`, `RegistryConfig` — construct the registry with retry policy
- `GenericCommandService` — single execution surface shared by HTTP and queue adapters
