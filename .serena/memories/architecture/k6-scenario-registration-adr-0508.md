# STORY-020 — k6 scenario registration (ADR-0508)

Canonical ADR: `docs/adr/0508-k6-scenario-registration.md`. Status: Proposed.

- One versioned, bounded catalogue drives ordered scenario selection and independent completeness evidence.
- k6 owns assertions and API fixture lifecycles; Python owns the verified environment, process custody, and closure.
- Preserve explicit SAN restoration, sandboxed OAuth, and post-merge admission. No generic hooks or duplicate launcher.
- The Windows k6 adapters are retired. Use direct Linux/WSL for an isolated scenario and `/k6-e2e` for the full PR campaign on exact saved builds. WSL browser smoke is not OAuth qualification.

See the ADR and `docs/contracts/k6-scenario-registration.md`.
