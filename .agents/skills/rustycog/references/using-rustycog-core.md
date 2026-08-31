# Using RustyCog Core

Use this guide when adopting the `rustycog-core` crate (error primitives) in a service.

## Workflow

- Define domain-layer failures with `DomainError` constructors so use-case code stays explicit.
- Convert domain errors at application boundaries into `ServiceError` (directly or via `From<DomainError>`).
- After persist, missing `id` is `ok_or_else` + `DomainError::internal_error("… missing id after persist")` — not `unwrap` / `expect` on the business id.
- Fallible string/enum mappings use `FromStr` / `TryFrom`. Keep infallible `From<Enum> for &str`.
- Use `ServiceError` constructors consistently in handlers and adapters instead of handwritten status/message pairs.
- Rely on `http_status_code()` and `is_retryable()` semantics in upper layers rather than duplicating category logic.

## Common Pitfalls

- Mixing custom ad hoc error enums with `ServiceError` in the same execution path.
- Dropping field/resource context when converting domain errors.
- Treating all failures as retryable instead of honoring `ServiceError` category semantics.

## Source files

- `rustycog/rustycog-core/src/error.rs`

## Key types

- `DomainError` — domain-layer failure constructors
- `ServiceError` — application/transport-layer error with `http_status_code()` and `is_retryable()`
