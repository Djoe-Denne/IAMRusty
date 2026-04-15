---
title: CommandContext
category: entities
tags: [rustycog, commands, tracing, visibility/internal]
sources:
  - rustycog/rustycog-command/src/lib.rs
summary: CommandContext carries execution-scoped metadata such as execution ID, optional user ID, request ID, and key-value metadata.
provenance:
  extracted: 0.9
  inferred: 0.05
  ambiguous: 0.05
created: 2026-04-15T17:15:56.0808743Z
updated: 2026-04-15T17:15:56.0808743Z
---

# CommandContext

`CommandContext` is the command-execution context type used by `[[projects/rustycog/references/rustycog-command]]`.

## Key Ideas

- It includes `execution_id` by default to support traceability for each command execution.
- It optionally carries `user_id` and `request_id` so transport-level identity/trace information can flow into command handlers.
- A metadata map allows service-specific context without changing the core command trait.
- Builder-style helpers (`with_user_id`, `with_request_id`, `with_metadata`) make context enrichment explicit at call sites.

## Open Questions

- There is still no strict cross-service convention for which metadata keys should be mandatory across HTTP and queue pathways. ^[inferred]

## Sources

- [[projects/rustycog/references/rustycog-command]]
- [[entities/command-registry]]
- [[concepts/command-registry-and-retry-policies]]
