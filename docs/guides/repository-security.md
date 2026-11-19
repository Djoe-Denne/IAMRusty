# Repository security checks

## Coverage and ownership

- **Application and CI source — CodeQL.** The platform is a Cargo workspace of
  Axum services, shared crates, authorization and reconciliation workers, and a
  modular monolith. CodeQL scans Rust plus the Python and JavaScript that run CI,
  publish test evidence, or operate the platform. It does not scan archived examples,
  agent tooling, workflow YAML, or the separately owned RustyCog source. The pinned
  RustyCog checkout is still required for Cargo path resolution.
- **Workflow configuration — zizmor.** Workflow, local action, and Dependabot
  changes trigger an offline audit. Medium or high findings fail that job, with
  annotations rather than SARIF. Run the unfiltered audit for lower-severity
  findings; some fixed-repository checks intentionally prevent publication from forks.
- **Dependencies — Dependabot.** The root `Cargo.lock` and GitHub Actions have
  grouped version and security updates. The two-open-PR limit per ecosystem and
  seven-day cooldown apply to routine updates, not vulnerability fixes. Review
  major bumps and any `0.x` minor changes for compatibility; there is no auto-merge.
  Nested legacy lockfiles may produce separate alerts. Match them to the build
  before triage. Image and SDK pins are maintained through their existing release
  and qualification procedures; no npm or pip application manifest exists.
- **Secrets — GitHub repository setting pending.** Repository Secret Scanning and
  Push Protection were disabled at inventory time. No second scanner was installed
  to stand in for a setting a repository administrator can enable.
- **HTTP contracts — local Schemathesis and existing tests.** Schemathesis checks
  the IAM username query and non-consuming reset-token validation body against
  [`services/IAMRusty/openspecs.yaml`](../../services/IAMRusty/openspecs.yaml).
  Hive, Manifesto and Telegraph have checked-in specs but are not qualified by this
  smoke check. Keep the existing integration and k6 tests for identity, authorization,
  state changes, and mesh/TLS behavior.
- **Design and uncovered classes — manual review.** Prioritize OAuth/JWT and
  middleware, OpenFGA object and tenant boundaries, Envoy `ext-authz`, Lazaret key
  storage, and async admission/outbox/reconciliation. The first-party `unsafe`
  sites found in the inventory were fixture code; native crypto/TLS dependencies
  are still important. No first-party binary parser or production FFI boundary
  was identified that justified a broad Miri or `cargo-fuzz` lane. Revisit when a
  small, production-reachable target appears.

The existing SonarCloud/Clippy, coverage, test jobs, and the ruleset requiring
`CI scope and result` remain in place. There is no second source SAST, advisory
database scanner, or CodeQL `actions` lane. Docker/Helm/Envoy configuration,
container OS packages, and service-to-service policy are not covered by these
source analyses alone.

## CodeQL operation and coverage

The advanced setup uses separate, non-overlapping entry points for
[Rust](../../.github/workflows/codeql.yml) and
[automation code](../../.github/workflows/codeql-automation.yml). Both use the
linked bundle from the pinned v4 action, `build-mode: none`, GitHub-hosted Linux,
and no repository secrets. The three categories upload
findings to Code Scanning, not source databases. Relevant PRs and pushes to
`master` run the affected workflow. A weekly Wednesday 03:43 UTC scan and
manual dispatch cover the default branch even without changes. Superseded PR
runs are cancelled. The Rust
extractor may build macros and run Cargo build scripts; this is neither a full
application build nor permission to run fork code on a privileged runner.

Rust uses [`security-extended`](https://codeql.github.com/codeql-query-help/rust/)
in addition to the default checks; the bundle used for qualification resolved
39 queries versus 37 by default, including metrics and diagnostics. Python and
JavaScript retain the default suites. Reassess precision and duration after the
first two hosted runs; do not suppress broad categories just to obtain a green check.

For a local recheck, use the official [CodeQL bundle](https://github.com/github/codeql-action/releases)
linked by the action (2.27.4 for v4.39.3), start in a trusted clean checkout with
the pinned submodule initialized, and write new paths under the ignored `.tmp/`.
These commands do not upload anything:

```powershell
git submodule update --init rustycog
codeql database create .tmp/db-rust --language=rust --build-mode=none --codescanning-config=.github/codeql/config.yml
codeql database analyze .tmp/db-rust codeql/rust-queries:codeql-suites/rust-security-extended.qls --format=sarifv2.1.0 --sarif-add-baseline-file-info --output=.tmp/rust.sarif
```

After authorized publication, check [Code Scanning tool status](https://docs.github.com/en/code-security/how-tos/find-and-fix-code-vulnerabilities/manage-your-configuration/use-the-tools-status-page-for-code-scanning)
and the per-file extraction CSV for the exact commit. Look for IAM's HTTP
handlers, `workers/ext-authz`, Lazaret, the platform contracts and monolith,
`ops/ci/pipeline.py`, and `.github/actions/export-cache-runtime/index.js`. Compare
unexpected omissions to the [scope](../../.github/codeql/config.yml), target,
and macro/extractor diagnostics. A successful upload or a path listed as part
of the source baseline is not proof that CodeQL extracted the file.

## Local checks

```powershell
actionlint .github/workflows/codeql.yml .github/workflows/codeql-automation.yml .github/workflows/workflow-security.yml .github/workflows/ci.yml
uv tool run --from zizmor==1.32.0 zizmor --offline --strict-collection --min-severity=medium .
```

For the IAM contract check, use the repository's normal Windows integration-test
preflight and a dedicated user-local virtual environment. Install the pinned
tool before starting a test fixture:

```powershell
uv venv --python 3.13 .tmp/schemathesis
$env:AIFORALL_SCHEMATHESIS_PYTHON = (Resolve-Path .tmp/schemathesis/Scripts/python.exe).Path
uv pip install --python $env:AIFORALL_SCHEMATHESIS_PYTHON 'schemathesis==4.33.4'
$env:RUN_ENV = 'test'
./ops/tests/it-native.ps1 -Suite iam -IamOpenApi
```

The existing harness runs one ignored test only when selected. It owns the
ephemeral IAM listener and Postgres fixture; no application containers, Kind,
credentials, or externally supplied URL are required. The Python client rejects
non-loopback origins and does not use proxies or follow redirects. Eleven fixed
cases have a two-second request timeout and a 30-second total runner timeout.
They assert exact status and the response contract without guessing accounts,
using real reset tokens, or sending mail. Test multi-identity authorization,
rate limiting, and any state-changing or positive reset flow separately.

The initial local qualification passed all eleven cases against the real
Rust listener and its Postgres fixture. The fixture was removed and no
campaign-owned containers remained. It also passed the existing CI planner
and k6 workflow regressions and the native launcher checks. zizmor passed the
repository at the medium threshold and rejected a standalone checkout
without credential cleanup. CodeQL's local per-file extraction diagnostics
matched all 78 scoped Python files and all 11 scoped JavaScript files.

## Manual security review

Run `/security-audit <base>..<head> <boundary>` in OpenCode before a release or
after a sensitive change. The command uses the existing security reviewer's
diff-first checklist in the current session, with no parallel or scheduled AI
jobs. Trail of Bits' [Rust Review](https://github.com/trailofbits/skills/tree/main/plugins/rust-review)
orchestrates Claude-style worker and judge agents. An installed Codex CLI is not
the active OpenCode harness. OpenCode's skill discovery alone does not install
those agents, so this repository does not vendor the plugin or register a second
copy. In a separately approved, compatible runtime, use Trail of Bits'
[official marketplace instructions](https://github.com/trailofbits/skills#installation)
instead of duplicating the same review.

1. Record the exact commit and boundary; read Code Scanning and Dependabot.
   These read-only commands return alert metadata, not credentials:

   ```powershell
   gh api --method GET 'repos/{owner}/{repo}/code-scanning/alerts?state=open' --paginate --jq '.[] | {number, rule: .rule.id, tool: .tool.name, path: .most_recent_instance.location.path, url: .html_url}'
   gh api --method GET 'repos/{owner}/{repo}/dependabot/alerts?state=open' --paginate --jq '.[] | {number, dependency, advisory: .security_advisory.ghsa_id, cve: .security_advisory.cve_id, url: .html_url}'
   ```

2. Cross-reference public advisory IDs with [OSV](https://osv.dev/),
   [CISA KEV](https://www.cisa.gov/known-exploited-vulnerabilities-catalog), and
   [FIRST EPSS](https://www.first.org/epss/). Match package source and locked
   versions, then check the built dependency path with
   `cargo tree --locked -p PACKAGE -i CRATE@VERSION` under the relevant target and
   features. KEV or EPSS is a prioritization signal, not proof of local exposure.
3. Read the changed handlers, policies and existing negative tests. Use
   [OWASP API Security](https://owasp.org/API-Security/) to check identity/object
   binding, role transitions and trust-boundary consistency. Check safe/unsafe
   invariants, FFI, cancellation and locking, parser bounds, and request-reachable
   panics or recursion where applicable. Do not attempt exploitation.
4. Deduplicate dependency findings by advisory alias, package source/version
   and affected component; deduplicate code findings by root cause and boundary.
   Record location, evidence and commit, severity, confidence, necessary
   exposure conditions, and a verifiable remediation or regression assertion.
   Mark unknowns explicitly. Keep raw SARIF, test outputs, and triage notes
   local; share only the minimum non-sensitive evidence.

## GitHub rollout requiring authorization

- Publish through the existing PR workflow only after approval. Keep CodeQL
  Default Setup off; do not configure a second uploader for these languages.
  Once on GitHub, verify all three `master` categories, per-file coverage and
  upload processing, then verify a relevant PR (including a fork or Dependabot
  PR when available). Do not use `pull_request_target` to fix upload permissions.
- A repository administrator should enable [Secret Protection](https://docs.github.com/en/code-security/how-tos/secure-your-secrets/detect-secret-leaks/enable-secret-scanning)
  and [Push Protection](https://docs.github.com/en/code-security/secret-scanning/enabling-secret-scanning-features/enabling-push-protection-for-your-repository).
  Validate settings without pushing a test secret, and review existing alerts privately.
- Confirm Dependabot's first workspace update and security group in GitHub.

### What to require on PRs

Keep `CI scope and result` as the required check for this rollout. GitHub can
run the new `pull_request` workflows before merge; the scheduled and manual
events require the files on the default branch. A path-filtered workflow may
leave a required check pending on unrelated PRs, so do not add the three
CodeQL jobs or the standalone workflow audit as unconditional required checks.

After a clean hosted baseline, consider making the medium-and-higher workflow
audit part of `CI scope and result`: have its existing planner select workflow
changes, have the terminal gate require success when selected and an explicit
skip otherwise, and qualify an affected PR and a docs-only PR. Move the PR
audit rather than running it twice. Start CodeQL with alert review and only
consider a high-severity merge rule once actionable findings and the behavior
on unrelated and fork PRs have been reviewed. Leave Schemathesis and agent
reviews explicit and outside the PR gate.
