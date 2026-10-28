---
title: Telegraph References Index
category: navigation
tags: [index, references, communication]
sources:
  - _sources/opencode-telegraph-outbox-20261027/inventory.json
summary: >-
  Telegraph references : entities, API and delivery, durable outbox, owned HTTP and scoped SQS/OpenFGA recovery proofs.
provenance:
  extracted: 0.94
  inferred: 0.04
  ambiguous: 0.02
updated: 2026-10-27T19:27:11Z
---

# Telegraph References

Use `[[projects/rustycog/references/index]]` for shared command, config, HTTP, queue, permission, and testing behavior. Use the pages below for what Telegraph changes, adds, or leaves unresolved on top of that baseline.

- [[projects/telegraph/references/telegraph-entity-model]]
- [[projects/telegraph/references/telegraph-event-processing]]
- [[projects/telegraph/references/telegraph-http-and-notification-api]]
- [[projects/telegraph/references/telegraph-runtime-and-configuration]]
- [[projects/telegraph/references/telegraph-service]]
- [[projects/telegraph/references/telegraph-testing-and-smtp-fixtures]]

## See Also

- [[projects/telegraph/concepts/durable-notification-publication]] — same writer transaction, transport gate and durable eligibility.
- [[projects/aiforall/skills/proving-leased-outbox-recovery]] — a fixed recovery deadline without weakening transport or access oracles.
- [[projects/aiforall/references/mission-telegraph-outbox-20261027]] — delivered commits, architect/author/reviewer contributions and explicit proof limits.
- [[projects/rustycog/references/rustycog-outbox]] — recorder, leases and retries.
- [[projects/rustycog/references/rustycog-http]] — HTTP owner and observed stop/join.

- [[projects/rustycog/references/index]] — Canonical RustyCog crate map.
- [[references/rustycog-service-construction]] — Shared service-construction baseline that Telegraph specializes.
- [[skills/building-rustycog-services]] — Reusable RustyCog workflow behind the shared baseline.
