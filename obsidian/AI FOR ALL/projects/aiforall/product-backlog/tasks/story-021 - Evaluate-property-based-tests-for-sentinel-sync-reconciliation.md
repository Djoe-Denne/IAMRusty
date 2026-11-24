---
id: STORY-021
title: Evaluate property-based tests for sentinel-sync reconciliation
status: Accepted intent
assignee: []
created_date: '2026-11-24T09:51:53Z'
labels: []
dependencies: []
references:
  - workers/sentinel-sync/src/reconcile.rs
documentation:
  - >-
    https://github.com/Djoe-Denne/IAMRusty/tree/experiment/hive-pagination-proptest/experiments/hive-pagination-proptest
type: spike
ordinal: 29000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
As an AIForAll maintainer, I want to evaluate a bounded Proptest pilot for the pure `sentinel-sync` tuple diff so I can decide whether generated tests catch regressions worth a new dev dependency. The [Hive pagination pilot](https://github.com/Djoe-Denne/IAMRusty/tree/experiment/hive-pagination-proptest/experiments/hive-pagination-proptest) illustrates the method on a historical regression; it is not a change to adopt in Hive. Out of scope: production logic changes, live OpenFGA tests, and workspace-wide rollout.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 E1: Given generated desired and existing tuple collections, including empty, overlapping, duplicated, and reordered inputs, applying the planned deletes and writes to the existing set yields exactly the desired set.
- [ ] #2 E2: Given the reconciled set, computing the diff again yields no writes and no deletes; permuting the inputs or adding duplicate tuples does not change the sets of planned writes or deletes.
- [ ] #3 E3: A bounded, reproducible native unit run passes against the current implementation. A local deliberate fault fails with a reduced counterexample. Record what this adds over the existing examples and recommend keeping or dropping the test-only dependency; no OpenFGA instance or Docker is required.
<!-- AC:END -->
