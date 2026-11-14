# Constraint planner fixtures

All sources and diagnostics in these tests are synthetic. `sample.rs` supplies
a literal documentation context; the test builder writes hash-bound inventory,
snapshot, capture, authorization and closure artifacts into a temporary root.
The builder also writes shared `clippy-localization-proof` artifacts with the
contract's exact predicate summaries and root locator. Tests deliberately
forge all-true summaries over unsupported/nested declarations to verify
independent replay rather than trust in those booleans.

Range-local cases use synthetic hashed MCP text/structured payloads and the
shared utility. Observation cases construct a minimal Cargo/module/origin
chain and ordered `who_uses` pages, including optional caller omissions,
distinct repeated sites, same-file loops and outside-diagnostic consumers.
Forged context, query, source and publication cases must never create a hard
union or promote an unresolved observation.

No production inventory, MCP service, Git process, Cargo or runtime is needed.
The fixture builder deliberately follows the public contract rather than
importing the producer adapter.
