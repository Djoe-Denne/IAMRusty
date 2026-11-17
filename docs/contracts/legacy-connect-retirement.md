# Retire standalone GitHubConnect and GitLabConnect

Date: 2026-11-15. Milestone: IAM-IdP / OAuthConnect.
Authority: the explicit user removal instruction recorded in the
[mission report](../missions/20261114-local-it-fixture-recovery.md).
Local continuity ledger: `.cursor/handoffs/20261115-retire-legacy-connect.md`.
Implementation baseline: AIForAll `8379f5cd5869ab7190c9c5c4585f255f9a698386`,
SDK `54af36c22047b995a4f362f3907abeadbc858ec2`.

## Decision and reconciliation

Remove the two standalone implementations, including their dedicated tests, and
their active repository bindings. Keep generic OAuthConnect and IAM integration.
This executes [ADR 0413](../adr/0413-oauthconnect-multi-instances-profils.md)'s
already accepted single-service topology; **no new ADR or acceptance is needed**.
ADR 0413 remains **Accepted / Partial**. No global supersession of earlier ADRs
or promotion of deployment/security evidence follows.

ADR 0413's validation section, its
[main contract §§6–7](oauthconnect-multi-instances.md), ADR 0408's dated update,
and the [IAM v2 contract §§1, 6](iam-oauthconnect-v2.md) previously deferred
legacy removal or required temporary legacy consumers. The new user instruction
replaces that sequencing restriction **for repository source, dedicated tests,
and tracked build/configuration retirement only**. It does not waive real-provider
identity/cutover gates or authorize operations on an existing deployment.
Deletion is permitted now; it is not proof that a live migration succeeded.

Older “OIDC/service/IAM not delivered” snapshots in ADR 0413 and the main contract
are not a reason to rebuild or retain legacy services: the later
[IAM results](../missions/20261108-iam-oauthconnect-v2-results.md) document local
v2 delivery and its remaining limits. Those results are historical, not new test
results for this removal. Preserve all dated ADRs, contracts and evidence.

## Exact removal boundary

- Delete the tracked contents of `services/GitHubConnect/` and
  `services/GitLabConnect/`, including their dedicated source, manifests,
  configuration samples, Dockerfiles, fixtures and tests. The root's inventory
  reports **60 tracked files: 33 GitHub and 27 GitLab; zero untracked/ignored**.
  Recheck before deletion; preserve any newly discovered private or unknown work.
  The parent-owned tiny GitLab fixture fix is explicitly superseded by removal,
  not a fixture repair to retain elsewhere.
- Remove these **12 packages**, and only their workspace/path/build bindings:
  `github-connect-{service,configuration,domain,infra,http,setup}` and
  `gitlab-connect-{service,configuration,domain,infra,http,setup}`.
  Update `Cargo.toml` and `Cargo.lock` without unrelated upgrades or an SDK bump.
- Remove exclusively legacy selections from shared build/deployment files:
  `ops/deploy/build/targets.json` roles `github`/`gitlab` targeting the deleted
  Dockerfiles, corresponding chart roles **even if disabled**, old binary/image
  lists, Compose service definitions/dependencies, scripts, routing and generated
  certificate inputs. Retain the generic `oauth` role. Delete whole shared files
  only if the parent's exact inventory proves they are exclusively legacy;
  otherwise remove or migrate the relevant entries, not their other consumers.
- Update current service documentation/examples and IAM defaults as below.
  Old service handbook pages may become historical pointers to OAuthConnect;
  do not leave their standalone launch instructions as current guidance.
  Historical ADR/evidence references are intentionally allowed to remain.

This is **not** permission for a repository-wide `github`/`gitlab` string purge.
The root owns the exact shared-file inventory. Its supplied Serena reference
checks keep both legacy client types inside their respective trees; its exact
`github_connect`/`gitlab_connect` import checks found no external Rust imports.
These checks do not classify credential paths, templates or historical prose.

## Retained contracts and configuration changes

- Keep all `services/OAuthConnect/` crates/tests and the generic GitHub **OAuth**
  and GitLab **OIDC** profiles. Keep `crates/idp-connect-contract/`, its tests,
  vendor-neutral DTOs/ports, HMAC helpers and v2 refusal/default behavior.
  Shared v1 DTOs/methods/tests are **not** part of this deletion; do not combine
  retirement with a shared API cleanup or reinterpret v1 as an OIDC fallback.
- Migrate tracked IAM defaults/examples still targeting
  `:8085/github-connect` or `:8086/gitlab-connect` to the **existing configured
  OAuthConnect listener/service**, with bases `/oauth-connect/github` and
  `/oauth-connect/gitlab`. Derive host, exposed port and TLS trust from that
  launch topology; neither old port identifies a separate provider service now.
  The generic configuration fixture uses internal HTTP `8080` / TLS `8443`;
  it does not establish a universal host mapping. The IAM adapter appends
  `/v2/authorize` and `/v2/complete`; do not put those suffixes in the base URL,
  duplicate prefixes, add old-route aliases or introduce protocol fallback.
- Preserve public provider IDs `github`/`gitlab`, IAM browser callback and
  relink-callback URLs, exact redirect allowlists and per-instance HMAC material.
  **Keep OpenBao references `github-connect/oauth` and `github-connect/iam`
  unchanged**, including the retained generic fixture
  `services/OAuthConnect/configuration/tests/fixtures/ops-connect.toml`.
  These are credential paths, not service bindings. Do not rename, rotate,
  reseed, delete or migrate them, token files, private inputs or live resources.
- Preserve HMAC over exact `METHOD\nPATH\nTIMESTAMP\nBODY`, the full
  `/oauth-connect/{instance}/v2/...` path, the 30-second window, instance isolation,
  verified TLS/defaults and explicit existing local exceptions. No user bearer,
  public S2S route, JWT authority, DB/events/FGA or OAuthConnect monolith nest.
  The monolith keeps IAM's HTTP adapter.
- Preserve IAM UUIDs and `(instance, opaque external ID)` identity, account/email
  rules, LOGIN/LINK/RELINK ordering, durable nonce/PKCE/state/cookie bindings,
  writer consume-and-purge before completion, no replay/retry/fallback, bounded
  responses and secret redaction. No SQL/schema/account/token migration or reset.
  GitHub remains primary-email-only as ratified; do not restore its old fallback.

## Validation and delivery requirements

The parent implements and records the following; none is executed by this author.

1. Reconcile the exact inventory with the deletion diff, including ignored/private
   files. No active build role, workspace member, dependency, default URL or route
   may select the deleted services. Residual matches must be classified as history,
   retained provider functionality or preserved credential references.
2. Verify Cargo metadata contains OAuthConnect and **none of the 12 legacy
   packages**. Run the existing full-workspace all-target build and workspace
   library-test campaign with its unchanged flags, against the retained source,
   **without package exclusions masking active code**. Windows native, one Cargo
   process, repository jobs=24; no target clean, profile/toolchain or SDK changes.
3. Preserve/run retained `idp-connect-contract --features server` and OAuthConnect
   regression suites for both profiles, boot validation, per-instance full-PATH
   HMAC, TLS/secrets, nonce/PKCE and OIDC rejection. Dedicated legacy tests are
   authorized deletions, not new failures to repair. If a deleted test is the only
   oracle for a retained shared security guarantee, identify an existing equivalent
   or port that minimal oracle to its retained owner; do not copy obsolete services
   or discard shared guarantees to obtain GREEN.
4. Run relevant existing IAM IT after the authorized native build/unit gates:
   `auth_oauth_start`, `auth_oauth_callback`, `auth_oauth_provider_failures`,
   `oauth_browser_transactions`, `auth_transactions_postgres`,
   `oauth_cleanup_lifecycle`, `relink_provider`, `verified_idp_https`; select
   additional existing targets only where the actual diff requires them.
   Reuse the current fixture harness, lifecycle/evidence skills and resource
   ledger. No Kind campaign or runtime destruction is implied by this contract.
5. Validate affected build metadata, chart renders, Compose/script/configuration
   bindings offline using their existing checks. A correct URL's syntax is not
   provider or deployment proof. Record failures and NOT RUN honestly.
6. Integrate this contract and link it from ADR 0413's **index entry**, with a dated
   source-retirement note; leave ADR statuses and historical evidence intact.
   The parent owns that reference, implementation, ledger and final report;
   this author owns only this file.

Real GitHub/GitLab E2E, exact deployed GitLab `sub == old id.to_string()`, existing
account/UUID continuity and any live cutover remain **unproven by this work**.
Do not infer them from compilation, mocks or source deletion. Any later operation
on live configuration/data, identity remapping, credential rotation or weakened
trust requires separate authorization; a GitLab subject mismatch stops cutover.
No Cargo, runtime operation, commit, push or global MCP change is authorized for
this documentation subtask.
