# STORY-020 — k6 scenario registration (ADR-0508)

Canonical ADR: `docs/adr/0508-k6-scenario-registration.md`. Status: Proposed. Reality: Unimplemented.

- One versioned, bounded catalogue drives ordered scenario selection and independent completeness evidence.
- k6 owns assertions and API fixture lifecycles; Python owns the verified environment, process custody, and closure.
- Preserve explicit SAN restoration, sandboxed OAuth, and post-merge admission. No generic hooks or duplicate launcher.
- WSL browser smoke is not OAuth qualification; do not retire Windows without parity.

See the ADR and `docs/contracts/k6-scenario-registration.md`.
