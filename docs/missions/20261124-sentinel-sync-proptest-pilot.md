# Sentinel Sync tuple reconciliation property-test pilot

STORY-021 evaluated a package-local Proptest dev-dependency against the pure
`diff_tuples` function. The baseline was
`b2362c6b40cd562b36c4b9800ac4fa4d53d1ac3f`. This work did not change
production code or require a running OpenFGA instance.

## Native Windows results

- `cargo test --locked -p sentinel-sync --test diff_tuples_proptest` passed
  six tests. The property uses 512 generated cases with seed `20261124`;
  deterministic examples separately pin empty, overlapping, duplicated, and
  reordered inputs. It checks exact reconciliation, no further changes after
  reconciliation, and invariant write and delete sets.
- A separate temporary test target called the real diff and deliberately
  removed one planned delete before applying the same oracle. The property
  failed on the resulting set mismatch and reduced the case to empty desired
  input and one existing tuple. After removing the temporary source, all six
  tests passed again with the lockfile unchanged.

The test binary calls the pure function without I/O. This result does not
qualify live OpenFGA behavior, authorization changes, or any IAM proposal.

## Decision for this pilot

Keep Proptest as a dev-dependency of `sentinel-sync` only. Compared with the
existing singleton examples, the property checks interacting set invariants
and produces a small diagnostic when a planned operation goes missing. The
pilot does not establish that generated tests outperform exhaustive tests
over this small domain or justify wider adoption.
