# Architecte ruling — lexical guard residue (2026-10-05)

Authority: architecte session `ses_ef312e3a3ffeb32eRzKlUKElox`, read-only contract reconciliation. No ADR amendment or additional user arbitration required; existing scans remain unchanged.

Canonical boundaries: ADR 0006:129–149 (P2 reconciliation isolated from event consumer); ADR 0007:159–161 (Manifesto ignorant of Lazaret, no scan exception); ADR 0008:25–43 (P4 operator outside Manifesto and Lazaret). Proposed ADR 0605 grants no exception.

Five locally failing scans are lexical occurrences in four lines, not an identified behavioral architecture violation. Comments and cfg(test) source must remain scanned. Parent authorizes only these four substitutions:

1. `services/Manifesto/infra/src/event/consumer.rs:19`: replace `rustycog consumers poll` with `rustycog consumers read from`. Preserve the all_queue_urls/sentinel-message explanation and code.
2. `services/Lazaret/application/src/invoke.rs:78`: documentation of NamespacedDigestDnsPluginLocator becomes `Digest-based routing in one namespace validated as a DNS-1123 label.`
3. `services/Lazaret/configuration/src/lib.rs:241`: documentation of PluginHopConfig::namespace becomes `Plugin service namespace for digest DNS routing; never trimmed or defaulted when explicit.` Parent explicitly assigns this single-line shared-configuration edit to the mechanical worker; no other configuration mutation.
4. `services/Manifesto/application/src/usecase/component.rs:593`: cfg(test) fixture endpoint becomes `http://127.0.0.1:8080/catalog-fixture`. All four callers concern digest/capabilities/UoW; attach_from_catalog does not use endpoint. This is not a production URL.

Forbidden: changing assertions/traversals/tokens/allowlists, excluding comments/tests, moving offending source, concatenating/obfuscating forbidden strings, altering production behavior, APIs, ports, migrations, defaults, namespace validation, explicit-value handling or .svc addressing. A genuine forbidden production mechanism requires stop/escalation.

Acceptance under the next exclusive Docker Cargo lease after Telegraph releases it:
- Five previously failed selectors pass, all 43 Manifesto guards remain green.
- Six manifesto-application attach_from_catalog_tests pass.
- fmt check and diff-check pass; exactly four additional textual source edits.
- IAM/key work, Telegraph and protected Hive files remain untouched.

Architecte simulated scans in memory with zero residual occurrences, but this is static evidence, not Rust test execution.
