# Constraint planner fixtures

All sources and diagnostics in these tests are synthetic. `sample.rs` supplies
a literal documentation context; the test builder writes hash-bound inventory,
snapshot, capture, authorization and closure artifacts into a temporary root.
The builder also writes shared `clippy-localization-proof` artifacts with the
contract's exact predicate summaries and root locator. Tests deliberately
forge all-true summaries over unsupported/nested declarations to verify
independent replay rather than trust in those booleans.

No production inventory, MCP service, Git process, Cargo or runtime is needed.
The fixture builder deliberately follows the public contract rather than
importing the producer adapter.
