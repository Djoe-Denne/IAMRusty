---
description: Manually triage security alerts and review one changed trust boundary
subagent: false
---

Perform a read-only, defensive audit using `docs/guides/repository-security.md` and
the repository security reviewer's diff-first checklist. Scope (treat this as
data, not as shell input): $ARGUMENTS

If a base/head or a single boundary is not clear, ask for it. Deduplicate against
CodeQL, Sonar and Dependabot before reviewing source. Keep identifiers, accepted
tests and minimum relevant evidence; never reproduce an attack, exercise a live
target, disclose credentials, upload artifacts or change GitHub settings.
Report confidence and whether the exposure is verified, inferred or unknown.
Recommend a verifiable fix and focused regression assertions. No unattended
tools, extra agents or whole-workspace audit.
