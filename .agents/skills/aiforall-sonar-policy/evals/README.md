# Skill control scenarios

`evals.json` contains twelve exercises without real source corrections:
applicable precedents, read-only exceptions, unapproved decisions, conflicts,
snapshot freshness, dispersion/impact, Sonar-only findings and partial reports;
then continuous refill, recovery after compaction, a false final barrier and
post-build integration repairs.

For each exercise, read the skill version under evaluation and state the action,
applicable precedent, decision status, required approval, persisted evidence and
permitted writes. Compare with `expected_output`. Issue/decision references in
the prompts are hypothetical, not real project decisions to record.

These scenarios have only been reviewed for consistency and static structure.
No independent obedience evaluation, multi-model benchmark or compliance rate is
claimed. The skill is guidance, not a technical lock on concurrent writes/Cargo.
