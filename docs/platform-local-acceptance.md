# Local acceptance — final integration gate, not a completed run

No IT/E2E has been executed by Package E. Source coverage below is **uncompiled**;
static parsing does not close the security, persistence or network findings.
Do not reuse historical S2S hashes as evidence for this working tree.

## Integration prerequisites (reserved common/fixtures/Cargo owner)

- `workers/ext-authz/Cargo.toml` dev dependency:
  `iam-domain = { path = "../../services/IAMRusty/domain" }`. External tests now
  serialize `JwkSet::from_registry_keys`; no invented `status`/trust JSON defaults.
- `IdpConnectMockService::received_requests(&self) -> Vec<wiremock::Request>`:
  expose `self.server.received_requests().await.unwrap_or_default()` for outbound
  authorize/token/profile assertions. Tests never print captured credentials.
- `common::setup_test_replica(&TestFixture)` returns the existing harness result
  shape `(String, reqwest::Client)` in a `Result`. It must build a **distinct IAM
  app/listener** against the same writer DB/security configuration, without
  migration reset, server singleton reuse, or copying an in-memory nonce set.
  Return the same once-prefixed `/iam` base URL convention as `setup_test_server`.
- `common::setup_test_server_with_pkce()` returns
  `Result<(TestFixture, String, reqwest::Client), Box<dyn std::error::Error>>`.
  Explicitly enable the configured GitHub sender/receiver PKCE capability;
  keep other tests' dual-HMAC and standalone configuration unchanged. The typed
  connector fixture must carry S256 parameters through authorize rather than
  silently ignoring `Some` fields.
- `common::setup_test_server_with_signing_keys(&[SigningKey])` returns the same
  triple/result shape. Seed actual writer registry rows before serving, bind the
  active fixed test PEM/platform kid and the platform issuer from its row, use
  RS256 verification against the real GET JWKS publisher, and disable relaxed
  legacy trust for this fixture. It must not serve an invented JWKS JSON mock.
- Keep B migrations, writers and the three-field `IamHttpSecurityContext` wired
  on every real listener. SDK transport/strict-metadata integration remains its
   own gate; tests must not compensate with injected fake principals or peers.

### Phase 3 integration requests

The earlier replica/PKCE/signing-key helpers, `build_test_iam_app`,
`setup_test_server_with_rate_limits`, ext-authz `iam-domain` dev dependency and
root OAuth cleanup supervision are now present in the working tree;
this observation is not compilation or execution proof. Additional reserved APIs:

- `common::build_test_iam_app(&TestFixture, SecurityConfig) ->
  Result<iam_setup::app::IAMRustyApp, Box<dyn std::error::Error>>` (or the harness's
  compatible error type): use the existing fixture's real primary DB, actual
  app builder and mock outbound event publisher. Preserve the supplied explicit
  security mode/rate policy. Build no listener, reset no DB and launch no tasks.
  This supports real dual-bind transport and root supervision tests; it must not
  manually synthesize the mandatory HTTP context from unrelated mock use cases.
- `common::setup_test_server_with_rate_limits(AuthRateLimitConfig)` returns the
  usual triple/result. Force the requested enabled limits into the actual root,
  use OS accepted-socket peer metadata, no fake ConnectInfo or proxy SAN. Its
  fixture must not automatically disable rate limiting after receiving the policy.
- `IAMRustyApp::start_background_tasks`/`stop_background_tasks` must retain and
  signal BOTH outbox and OAuth cleanup handles. `app_cleanup_supervision.rs`
  asserts both handles join while actual cleanup SQL is blocked by a primary
  table lock. Integration6's constructor remains owned by B/integration.
- Shared test JWT registry mocks (owned by integration) must implement
  `confirm_active_for_emission` from their known stored key and all required
  status/material/trust bindings; no unconditional true/default implementation.
  OAuth writer mocks must implement strict bounded `purge_expired` explicitly.
- `writer_barrier.rs` uses existing SQLx/SeaORM dependencies to open an independent
  primary pool from the harness connection options. Integration must ensure the
  fixture writer pool permits three simultaneously blocked session operations
  plus the observer; otherwise the causal test fails rather than using sleeps.

## Source tests

- S1: `organization_account_guard.rs` positively verifies a real published org
  principal, then denies its victim-sub `/me`, Link START and Relink START before
  connector traffic or target transaction mutation; platform `/me` stays valid.
- S4/T3: ext-authz stand-in uses actual publisher serialization and positive
  controls; issuer/org substitution, Pending, temporal/type/signature negatives.
  `registry_publication.rs` covers last-key revoke → empty → outage, all known
  kids stale after the 60-second budget, and replacement recovery. Its real-clock
  59/61-second samples supplement, not replace, C's exact-boundary unit tests.
- C1: `session_persistence.rs` and OAuth browser completion issue and rotate real
  sessions without inserting refresh fixtures. T4 retains exactly one winner.
- S6: `oauth_browser_transactions.rs` uses real START/Set-Cookie/state, wrong or
  absent browser/provider/intention, expiry, replay, two tabs, replica consumption,
  Link then Relink callback without Bearer, and connector S256/verifier assertions.
- S8: `session_writer_concurrency.rs` exercises actual PostgreSQL writer lock
  overlap, exactly one reset, no old-credential session surviving, post-reset
  issuance, and purge failure rollback. A fixture-owned SQL trigger/function is
   removed before assertions. It now uses a separate primary pool and observed
   PostgreSQL blocking graph, with winner password/reset-token assertions.
- E-2..E-5: `auth_transactions_postgres.rs` adds same-old-token rotation winner/
  RecordNotFound and no orphan, row-locked OAuth consume/verifier deletion/TTL,
  actual isolated-schema migration fail-closed and Login-target CHECK, and stale
  registration password without username/session mutation. Browser tests provide
  the separate multi-listener replay evidence. No database test has run.
- Public password reset × rotation now has an HTTP/Postgres causal barrier in
  `session_persistence.rs`, purged old chains and genuine new-password login.
- Legacy `auth_oauth_callback.rs`, `auth_oauth_provider_failures.rs` and positive
  `relink_provider.rs` callbacks now obtain real START/cookie/state using test-local
  `support/browser_flow.rs`. Deliberate forgery negatives remain explicit.
  Relink success callbacks send no Bearer; protected START still tests expiry.
- `oauth_browser_transactions.rs` adds stored target/redirect mismatch, ignored
  retargeting query parameters, and injected writer failure/no outbound exchange/
  rollback/recovery with redacted public error payload.
- `public_refresh_expired_access.rs` uses a real published RS256 platform key,
  independently proves signature/issuer/audience with only diagnostic expiry
  checking disabled, denies protected `/me`, then rotates public refresh with
  that genuinely expired Bearer and verifies the replacement in PostgreSQL.
- `signing_epoch_writer.rs` proves committed independent-primary transitions
  override a stale Active object at emission confirmation. B owns signer-await
  interleaving unit tests; this does not replace those.
- `oauth_cleanup_lifecycle.rs` exercises real SQL failure/retry, bounded purge,
  cooperative shutdown during blocked SQL and DEBUG sentinel redaction. Its actor
  protocol evidence is separate from the root supervision test.
- `http_peer_source.rs` requires enabled source budgets on a real listener and
  varies untrusted XFF/account/principal headers without resetting OS source.
  `https_mesh_optional_mtls.rs` now requires real-core fixture construction and
  includes no-client-CA, wrong-server-CA and wrong-SAN transport cases.
- S9: `transit_protocol.rs` covers missing named credential before HTTP, strict
  path rejection, a positive Sign control followed by denied alternate-operation
  redirect, and bounded timeout without retry. It does not prove real TLS or ACLs.

The last two E source scenarios are now written in E5: Hive Admin configure
permissions and verified IAM→IdP HTTPS. Their exact remaining shared-helper and
manifest integration requirements are below. All integrated runtime/transport
checks and repeated worker-failure proof remain mandatory after final IT.
The route matrix below is source-only. No skips, compile-only success or legacy
fake states can close those gates.

## T6/T7 parent-final local-full cases

Read `.agents/skills/local-runtime-lifecycle/SKILL.md` first. Use the parent's
existing lease: IT is existing testcontainers, E2E is only
**`kind-aiforall-local-full` / cluster `aiforall-local-full` only for this entire
three-node isolated lab**, as explicitly authorized by the user. The acceptance
helper's closed context list contains only that context; no current-kubeconfig
fallback or adoption of an old lease. The pre-existing `kind-aiforall-local`
cluster is foreign and must remain untouched. Historical
`mesh-authn-kind-e2e.sh` retains its old default; do not use it for this full lab
without a separately implemented explicit full/isolation mode. Never retarget
apparatus P4 IT or protected contexts.

S-11 entrypoint contract: use only `ops/tests/local-full-acceptance.py` for E's
full-lab observations, with `--context kind-aiforall-local-full --lease <leasev2>`.
It loads D's common authority from the fixed repo path; **every** API get/exec
checks exact leased CP6443 publication and fixed task-owned kubeconfig before UID,
then uses `--kubeconfig <lease.kubeconfig_path> --context kind-aiforall-local-full`.
Raw host curl/kubectl probe subprocesses are removed. Leasev1/changed config/stale
UID/ambient overrides stop, never adopt. The parent ledger/compiler is reserved.
Exact schema/argv are in
`.cursor/review-briefings/20261004-e-admission-anchor-wiring.md`.

Do not invoke `mesh-authn-kind-e2e.sh` or retarget its hardcoded legacy CTX for
this full lab: ambient alias/current-context, create/apply, FGA bootstrap,
set-env/rollout and polling/logging bypass this authority. The remote
`mesh-authn-kind-e2e-cases.sh` also hardcodes old platform DNS. Stateful legacy
replay remains **blocked for full use** until parent brokers full-topology fixture
inputs and guarded setup/exec/cleanup; route-matrix is not a substitute for it.
Legacy semantics/default context are intentionally preserved.

`ops/tests/local-full-acceptance.py` does **not** deploy, build, start/stop nodes,
create probes or delete resources. Every action requires an explicit lease and
namespace/resource UIDs. Parent inventory must record exact probe/database Pod
UIDs first. It takes the existing D operation lock. No credentials, response
bodies, database rows or private certificate bytes are printed.

1. Provision two leased Running probe pods with curl/nslookup and the same test
   CA/client identity. The authorized one has D's `local-full-client` selector;
   the negative pod has no selector enabling an allow policy. Run
   `network-policy` with explicit allowed/denied namespace and Pod names, service,
   CA/cert/key **paths**, port and exact path. Both resolve DNS and target the same
   ClusterIP/SNI. Authorized HTTP vs forbidden curl timeout is the gate. DNS/TLS
   errors, connection refusal and port-forward are not deny evidence. Repeat with
   each representative namespace/profile hop required by D; one Envoy case is
   not coverage of every NetworkPolicy.
   Set `--tls-host` to the actual certificate SAN when it differs from Service DNS;
   DNS is still probed independently and `--resolve` targets the exact ClusterIP.
   After parent provisioning and UID inventory, run the observational public
   route matrix with the authorized probe and real Envoy destination:

   ```sh
   python ops/tests/local-full-acceptance.py route-matrix \
     --lease "$PARENT_LEASE" --context kind-aiforall-local-full \
     --namespace "$ENVOY_NS" --service "$ENVOY_SERVICE" \
     --allowed-namespace "$PROBE_NS" --allowed-pod "$ALLOW_PROBE" \
     --tls-host "$ENVOY_SAN" --port "$ENVOY_TLS_PORT" \
     --ca-path "$PROBE_CA_PATH" --cert-path "$PROBE_CERT_PATH" \
     --key-path "$PROBE_KEY_PATH" --cases ops/tests/mesh-public-route-cases.json
   ```

   Cases cover HEAD, exact public/protected paths, wrong methods and neighbors
   without access JWT/principal headers; expected status sets are explicit.
   No HTTP body or credentials are output. Missing probe/lease/route fails.
   D's full-profile source now targets Envoy container `10000`, NodePort `30080`,
   host loopback **`127.0.0.1:18080`**, with 112 resources reported by D's offline
   render (CA-only Secret). This is not a runtime observation by E. TLS probes
   must use the explicitly provisioned TLS Service port/SAN, not assume plaintext
   host port 18080 supports HTTPS. Bootstrap occurs only after all final IT pass.
2. Seed real persistent domain records through the public application flow.
   Provide an ordered, nonempty domain `SELECT` file (no credentials/tokens), a
   leased PostgreSQL Pod, database and representative Deployment. `snapshot`
   writes a NEW metadata/digest file. SQL runs inside `BEGIN READ ONLY`.
3. Parent deploys a changed configuration and versioned image using D's lease
   helper. `rollout-check --before <snapshot>` requires a changed template AND
   image, all old pods replaced, all replicas Ready, unchanged PVC/PV identities
   and exact domain-query data digest. It does not accept a readiness-only proof.
4. Parent uses D's consent-gated `plan|interrupt|recover` helper for an exact
   leased worker. Observe surviving workload readiness during outage separately;
   after recovery use `recovery-check`: some real pod replacement, worker-spread
   Ready replicas, cleared interruption ledger, unchanged storage/data. Repeat
   for each selected leased worker with fresh before evidence. The checker alone
   does not prove during-outage availability or transparent local-path failover.
5. D `backup` then `restore` into a NEW empty distinct namespace/database/PVC.
   Run `restore-check` with the source snapshot, target namespace/database/Pod
   and identical domain SELECT. It requires equal domain data and distinct Bound
   storage; count-only fingerprints are insufficient. Never overwrite a source.

All action-specific fixture arguments are mandatory; missing infrastructure fails
instead of skipping. Parent owns cleanup/fixture IDs on success **and** failure.
Record actual run outcomes and source/script/rendered-manifest/SDK commit hashes
only after executing the integrated cases. Preserve volatile fixtures while the
parent still needs them; no cluster delete/reset or automatic resource deletion.

## E4 fixture ownership and registration boundary

`support/fixture_cleanup.rs` catches assertion unwinds, joins
`common::cleanup_test_servers(&fixture)` on both success/failure, and only then
returns/resumes the panic. It cleans the exact per-fixture listener bookkeeping,
not SDK singleton containers or unknown shared resources. New E protocol tests
use it, including replica fixtures and the migrated legacy callback/Relink suites
(62 guarded test scopes at E4 static inspection). Test-owned auxiliary tasks use abort-on-drop
guards and explicit normal-path joins; no asynchronously spawned teardown/reset.
Parent inventory/lease reconciliation remains mandatory after process kills.

`registration_usage_boundary.rs` exercises password and OAuth registration from
actual HTTP issuance under the real published writer-bound RS256 kid. Signature,
issuer, registration audience/sub, flows/provider_info and exact 24h TTL are
verified independently; actual account guards deny its Bearer while dedicated
completion works, and actual access cannot be reused as registration. Email
verification before completion is explicit fixture arrangement, not a fabricated
session. The shared async registration codec/fence is B/integration-owned.

`organization_signer_configuration.rs` uses the actual facade, primary registry,
probe and rotator: missing/empty/other-org credential references deny before
credential resolution/vendor traffic; a matching RSA Transit challenge is the
positive control and persists exactly one Active organization epoch. This does
not claim live OpenBao ACL coverage: Admin authorization belongs to Hive, not the
IAM facade. The E5 Hive test separately covers that actual gate under the newly
authorized single-file scope.

`request_trace_redaction.rs` instruments each actual request future (plus the
owned real-core listener), not a globally reset subscriber. A nonsecret witness
prevents empty-capture green; actual login secrets, generated state/cookie/URL,
Bearer rejection and a called IdP token endpoint returning a sentinel error body
must not appear in DEBUG capture. It does not claim external proxy/vendor logs.

Classification: these new IAM source suites are final-only **HTTP/PostgreSQL or
outbound protocol IT**, including local TLS/trace tests using the real-core
testcontainers fixture. The Python route helper is final full-lab E2E. Formatting,
AST/JSON parsing and whitespace checks are static only; no test result is implied.

## E5 last two source scenarios and exact integration requests

- `services/Hive/tests/organization_signer_permissions.rs` is the only new Hive
  file authorized to E. It uses real organizations/member rows, the existing real
  PostgreSQL/OpenFGA testcontainers harness and actual HTTP route guard
  `Admin, organization`. DENY keeps the typed outbound IAM HTTP request count at
  the startup baseline; ALLOW on A reaches exactly the expected internal RPC,
  checks the forwarded body and uses A's stored slug, not caller B's slug/ID.
  A grant on A still denies B, and revocation denies before RPC again. The test
  uses the actual returned auth configuration's explicit isolated HS256 secret,
  issuer and audience. It does not infer policy from namespace names. No protected
  Hive fixture or member test was edited; no Kind harness is used.
- Reserved `services/Hive/tests/common.rs` helper required:

  ```rust
  pub async fn setup_test_server_with_iam_service(
      iam: hive_configuration::IamServiceConfig,
  ) -> Result<(
      HiveTestFixture, String, reqwest::Client, TestOpenFga,
      rustycog::config::AuthConfig,
  ), Box<dyn std::error::Error>>;

  pub async fn cleanup_test_servers(fixture: &HiveTestFixture) -> anyhow::Result<()>;
  ```

  Build an isolated actual `AppBuilder` listener on the existing real fixture DB/
  OpenFGA. Override only its supplied outbound IAM config, retain explicit test
  verifier policy, return the **actual post-build** auth config and once-prefixed
  base URL. Register exact fixture-owned listener/task handles and join only those
  in cleanup. Do not reuse/reset the SDK singleton listener, legacy `static mut
  APP` or stop shared containers. The test-local IAM mock implements strict typed
  configure matching and `received_requests`; no fixture exports/mod edits needed.
- `services/IAMRusty/tests/verified_idp_https.rs` plus
  `support/verified_idp_tls.rs` use the delivered additive
  `HttpIdpConnector::with_security_mode_and_roots(..., SecurityMode::Verified,
  Vec<reqwest::Certificate>)`. The SDK pin has no `TestTlsConfig`; the parent
  explicitly authorized this service-local typed TLS collaborator, generated
  with the existing SDK `mtls_client_auth.rs` rcgen pattern. It owns a bound random
  socket, generated CA/leaf/temp files, watch shutdown and explicitly joined task.
  No public opaque-client injection, global roots/env mutation or TLS bypass.
- Positive TLS exercises actual HMAC authorize, paired S256/verifier exchange and
  profile. Separate wrong-CA and correct-CA/wrong-IP-SAN cases produce no HTTP
  handler receipts. A 307 between two **both trusted** HTTPS fixtures is not
  followed; only the original authenticated request is observed. Cleanup executes
  before rethrowing assertions on success/failure.
- Reserved `services/IAMRusty/Cargo.toml` dev dependency needed by the fixture:
  `axum-server = { workspace = true }`. Existing workspace is `0.7` with
  `tls-rustls`; rcgen `0.13.2`, rustls and tempfile are already declared for IAM.
  No version upgrade or manifest edit by E. Also ensure existing E4
  `organization_signer_configuration.rs` direct `sha2` use has
  `sha2 = { workspace = true }` (workspace `0.10.8`) if not already integrated.
- Legacy E callers in `auth_complete_registration.rs` and
  `auth_username_flow_part2.rs` now await the actual async registration helpers
  **with the real shared codec**, not a config-only RSA construction. They require
  the following integration-owned readonly fixture getter:

  ```rust
  pub async fn fixture_jwt_codec(
      fixture: &TestFixture,
  ) -> Result<(
      std::sync::Arc<iam_infra::token::JwtTokenService>, String,
  ), Box<dyn std::error::Error>>;
  ```

  Return the exact root-created Arc shared by access/registration and its actual
  configured platform issuer. Broker capture/access to that root variable with
  B/integration; do not reconstruct from config, downcast a generic command, create
  a secondary registry/pool, invent an Active row or expose private PEM. Existing
  per-fixture ownership bookkeeping may retain this actual Arc. Missing lookup is
  an error, never fallback to config-only signing. The new registration boundary
  scenario still issues entirely through actual HTTP and needs no getter.

All five new scenarios are final-only IT: Hive real HTTP/PostgreSQL/OpenFGA and
four outbound local TLS protocol cases. No listener, handshake, container, test
or Cargo process was started by E5. Source completion is not execution proof.

## Explicit SDK fixture runner context (pending SDK safety publication/pin)

Required before a process first creates a PostgreSQL/OpenFGA/SQS/Kafka fixture:

- `RUSTYCOG_TEST_RUNNER_MODE=local|bridge`, explicit, never inferred from CI.
- `RUSTYCOG_TEST_RUN_ID`: parent-injected unique 1..40 ASCII alnum/hyphen.
- `RUSTYCOG_TEST_LEDGER_DIR`: existing absolute parent-approved directory, writable
  by the test process. Preserve exclusive `fixture-{run_id}-{pid}.jsonl` files;
  never truncate, remove or overwrite a previous ledger to make a retry pass.
- Bridge also needs `RUSTYCOG_TEST_RUNNER_HOST`: proven daemon-published-port
  address reachable from that worker, DNS/IPv4 only, no URL/port/userinfo/path,
  wildcard or loopback. IPv6 is unsupported. `TESTCONTAINERS_HOST_OVERRIDE` is not
  the SDK input. A Docker Desktop gateway name is not proof of connectivity.

Current root `coverage-integration` runs Cargo natively on hosted Ubuntu (no job
container): explicit `local` is appropriate. Workflow supplies
`fi-<github.run_id>-<github.run_attempt>-<strategy.job-index>` and a distinct absolute
`RUNNER_TEMP/rustycog-fixtures-<run_id>-<attempt>-<matrix.cache_key>` directory,
validates the ID/path and creates the directory before tests. No new Docker
privileges, Ryuk changes, transport flags or cleanup exemption. Other root jobs
compile or run non-testcontainer unit/HTTP stand-in suites; no mode is globally
injected into unknown/containerized consumers.

Future native-Linux manual fixture invocation (parent chooses values and grants
the runtime lease first; this example has NOT been executed):

```sh
export RUSTYCOG_TEST_RUNNER_MODE=local
export RUSTYCOG_TEST_RUN_ID='<parent-unique-run-id>'
export RUSTYCOG_TEST_LEDGER_DIR='<existing-or-new-absolute-approved-ledger-directory>'
test "${RUSTYCOG_TEST_LEDGER_DIR#/}" != "$RUSTYCOG_TEST_LEDGER_DIR"
umask 077
mkdir -p -- "$RUSTYCOG_TEST_LEDGER_DIR" # does not overwrite ledger files
# Then the parent's already-selected IT command, only after final authorization.
```

Windows project's tests still compile/run inside Docker under AGENTS.md: do NOT
copy the native-Linux `local` classification to that worker. Parent must broker
`bridge`, a proven non-loopback runner host, and an absolute worker-visible ledger
directory with persistent parent access before that IT command. No guessed bridge
address, opaque-client injection, socket privilege widening or ambient fallback.

Consumer teardown order, after SDK publication/pin:
finish protocol users; join owned app/listener tasks; finish/drop owned fixture
consumers per SDK lease rules; **await**
`rustycog::testing::common::join_fixture_creations()` on the originating live
runtime; inspect its `Result<(), String>` and reconcile exact IDs against the
parent ledger before runtime shutdown. Active consumers/UNKNOWN/errors forbid a
clean signoff; do not mask them with `let _ =`, detach a monitor or stop another
consumer's singleton. Shared consumer-source deltas are reserved Integration and
listed in `20261004-fixture-runner-consumer-wiring.md`.

Historical SDK ca2e35f/test evidence predates this fixture contract. The safety
sources, workflow inputs and static checks are source-ready only: safety API is
not yet published/pinned, bridge reachability and full IT/cleanup proof remain
pending. No CI/manual command was dispatched by this follow-up.
