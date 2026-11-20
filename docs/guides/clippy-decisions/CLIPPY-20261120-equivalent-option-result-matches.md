# CLIPPY-20261120-equivalent-option-result-matches — Lazy branch conversions

- Status: Accepted for these five expressions only.
- Date: 2026-11-20
- Rule: `clippy::option_if_let_else`
- Approval: the requesting user approved all five scoped changes in the
  2026-11-20 campaign discussion, rather than only the shared IAM test fixture.

## Context and precedent

The [October precedent](../../../.agents/skills/aiforall-sonar-policy/SKILL.md)
preserves existing `match` expressions. This exception covers only the five
reviewed expressions in the final remediation round, which the master analysis
at `ae31ddab` imported 47 times. It does not replace that precedent.

## Decision and invariants

Use `map_or_else`, not eager `map_or`, to evaluate exactly one branch.

- IAM HTTP connector: trim valid UTF-8; retain the original bytes otherwise.
- IAM setup and shared test fixture: keep the existing deadlines, error messages,
  and precedence of earlier errors. The conversion runs after the same await.
- Lazaret setup: use the configured CA only in the existing TLS branch; do not
  fall back after a TLS error.
- Hive signer test: make the same platform-versus-organization key choice.

Do not generalize this conversion to matches whose branches move the same
non-`Copy` value, perform ordered effects, or are clearer as explicit control flow.
Do not change public signatures, wire defaults, or Sonar issue statuses.

## Validation

The Windows all-features library run passed 221 tests (including IAM
connector, IAM setup, Lazaret setup, and the rerun of the Apparatus operator
suite). The campaign compares both full Windows Clippy lanes against the
original snapshot. Local validation does not launch the real integration
fixtures.
