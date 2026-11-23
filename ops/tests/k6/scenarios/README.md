# k6 E2E scenarios

For a standard API scenario, add a script and one entry in `catalog.json`. Use
`scenarioContract(id)` from `../lib/case-contract.js` (adjust the import to your
script's directory). The entry describes named checks and their roles, the
total HTTP requests, and the bounded runner timeout. The script defines the
actual assertions. Set one VU and one iteration, keep the `group` system tag,
use the contract's thresholds, and export its summary. Use `start()` and
`finish()` around the scenario body and `check()` in the body, setup or
teardown. The campaign runs every registered case in catalogue order.

Use real test APIs for fixtures. Record run-scoped ownership before or as each
write occurs. In normal runs, teardown removes owned data and checks the
result. If setup fails after a write, compensate before rethrowing; k6 does
not run teardown in that case. When compensation is not possible, run only
where the environment owner can dispose of the resulting isolated state.

## Run a scenario directly

On a prepared, authorized, isolated target, set `K6_ORIGIN`, `K6_RUN_ID` and
`K6_CAMPAIGN_FILE` from the current target's receipt. Set `K6_SUMMARY_PATH` to a
fresh private path. Configure trust for the preparation's test CA in the
platform or runner, without skipping TLS validation. The health scenario
uses the same code as CI:

```sh
k6 run --no-usage-report ops/tests/k6/scenarios/smoke/iam-health.js
```

The command does not start, prepare, or dispose of the lab. A direct scenario
result is not the full infrastructure and correlation verdict. Running a SAN
stimulus directly does not rotate or restore a certificate.

## Run the whole catalogue

An authorized repository writer can comment `/k6-e2e` on an internal PR.
The [hosted campaign](../ci/README.md#run-the-complete-campaign-on-a-pr) uses the
current PR merge's saved builds or fails before running the suite. It is
separate from automatic `master` E2E, Pages, and the ordinary PR checks.

The historical IAM stateless and provider-link pilots are not catalogue
entries. They require their own prepared targets and are not qualified by
this campaign. Do not discover them by globbing the scenario directory.
See the [scenario registration contract](../../../../docs/contracts/k6-scenario-registration.md).
