# ADR-0508: Register standard k6 scenarios once

- Status: Proposed
- Reality: Unimplemented
- Date: 2026-11-23
- Author: `architecte`, with the primary session
- Decision makers: Pending explicit acceptance
- Supersedes: None; complements [ADR-0504](0504-validation-fonctionnelle-linux-k6.md).
- Contract: [k6 scenario registration](../contracts/k6-scenario-registration.md)

## Context

STORY-020 requires the same standard k6 script in local use and CI. At
`7d517eb4`, a new scenario needs separate registrations for dispatch, evidence,
reporting, and artifact upload. Removing the external check list without a
replacement would allow a skipped assertion to leave a green result.

## Decision

Use a single ordered, bounded JSON catalogue that both k6 and the evidence
receiver read. k6 owns test behavior and assertions, including API-based fixture
setup and teardown. The existing campaign remains responsible for verified
infrastructure, correlation, cancellation and cleanup. The SAN and OAuth
specialized lifecycles remain explicit. A run-bound receipt records selection;
only validated results enter a fixed publication directory.

## Alternatives

A recursive JavaScript glob would admit helpers and input-dependent pilots.
Executing JavaScript to discover contracts would add a second orchestration
protocol. A single explicit entry allows validation before effects and does not
require a new dependency or a second runner.

## Consequences

Standard scenarios require a script and one declaration; their normal setup
and assertions can run directly on an already prepared and isolated target.
Independent failure and cleanup evidence is still required for campaign
success. This proposal does not retire native Windows or qualify WSL OAuth.
An opt-in pre-merge qualification has a separate trust boundary from the
automatic `master` workflow; neither the default triggers nor the PR gate
changes.
