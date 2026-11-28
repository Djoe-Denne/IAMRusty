# Password recovery hosted qualification evidence

Target: GitHub-hosted disposable Kind, scope `STORY-003.03.04`. This evidence
does not close the story or enable replica admission in a deployed environment.
See the [admission contract](../contracts/password-recovery-replica-admission.md)
and [route inventory](../contracts/password-recovery-route-inventory.md).

## Bootstrap and unsuccessful admission preflight

- [Run 46126260341](https://github.com/Djoe-Denne/IAMRusty/actions/runs/46126260341),
  tested merge `1e53809f3c844aa3090bd4690f0efa49162c8b59`: Linux Check worker,
  IAM checkpoint HTTP writer, and PostgreSQL 15 subscriber startup passed.
  The six existing catalogue cases and owned cleanup passed.
- [Run 46128094060](https://github.com/Djoe-Denne/IAMRusty/actions/runs/46128094060),
  tested merge `629e52e78c6ab976be2cef7a29b5e490d7a54eb5`: bootstrap and the
  six catalogue cases passed. The direct IAM control failed before reset
  confirmation while the writer still had epoch zero. The fixture's
  configured JWKS URL pointed to the runner rather than the IAM pod. This
  run did not measure revocation. Owned cleanup passed.

## Direct IAM and nominal mesh

[Run 46130050099](https://github.com/Djoe-Denne/IAMRusty/actions/runs/46130050099)
tested merge `b945ec94f09da0777a1f3d60b547c1d9ee2aeb33` with RustyCog
`42084b803b6a926f482a4030a0ed127447b21072`. The corrected direct fixture
used its own loopback JWKS listener. The public `admission.json` receipt
recorded:

- Direct IAM `/api/me` first revoked response by **598 ms**; Envoy
  `/iam/api/me` first revoked response by **608 ms**. Intervals start
  before sending successful reset confirmation, a conservative lower
  bound on the commit time.
- A fresh mesh request at or after the measured five-second boundary
  received `403`. The nominal window recorded 27 replica-attributed
  decisions and zero independent IAM fallback calls. Legacy epoch-zero
  admission, prior renewal revocation, and the signed-in password-change
  exception passed.
- The late-Check listener returned `503` in **1,009 ms**. The relay had
  received a positive response from the real Check worker and delayed
  delivery by **1,250 ms**. Envoy recorded zero additional requests to
  protected IAM, and the normal listener still admitted that credential.

The overall run failed at `E2E_REPLICA_DECISION_SOURCE` during the degraded
scenario; the artifact does not identify whether it was the pre-reset or
catch-up attribution check. No degraded cutoff, fallback capacity, or
concurrent writer-ordering result is claimed from this run. Owned cleanup
passed, with zero owned running resources. The
[required PR check](https://github.com/Djoe-Denne/IAMRusty/actions/runs/46128949279)
also failed independently when the Manifesto LocalStack SQS fixture exited
before readiness.

The gateway's counter timestamp must be compared locally against the
measurement boundary. The pinned Kubernetes client's
[API time serialization](https://github.com/kubernetes/apimachinery/blob/v0.37.0/pkg/apis/meta/v1/time.go)
can round `--since-time` to a whole second, allowing an older line from
that same second to be returned. A focused unit test distinguishes the old
and new timestamps without dropping the exact counter-delta assertion.

The hosted fixture still lacks the actual monolith, the non-IAM service
routes, and the dedicated Manifesto workload credential path.

## Retry with fresh gateway counters

[Run 46131960634](https://github.com/Djoe-Denne/IAMRusty/actions/runs/46131960634)
tested merge `a7e5e5755efc7b361838ad0c0fc638b3cd76259a`. The nominal
first refusals were **343 ms** (direct IAM) and **350 ms** (mesh), with
28 replica-attributed observations, zero IAM fallback calls, and a
post-boundary `403`. The late-positive witness repeated: `503` in
1,006 ms, a 1,250 ms delayed positive response, and zero downstream calls.

The degraded check stopped at `E2E_RESET_STALE_REPLICA_UNPROVEN`. Its
checkpoint-table lock allows a reset that committed first to reach the
subscriber before the apply worker encounters a blocked checkpoint. The
next fixture locks the epoch table itself and distinguishes an unexpectedly
advanced replica from an unverified apply barrier. Both the catalogue and
cleanup passed; no degraded result is claimed from this run.
