# Manifesto correction — conventions 2026-09-09

- Project mutations require an **active DB membership** (or org admin), not leftover OpenFGA Write. Tests that only `grant_write` without inserting a member get 403.
- `require_project_mutation_actor`: Suspended → owner / project admin / org admin only.
- OpenFGA 1.5 Read rejects `tuple_key.object = "project:"` (empty object id). `OpenFgaWriteClient::read_manifesto_tuples` reads the whole store (`page_size` only) then filters `project`/`component`.
- Restore during grace: same member id, revoke old grants, apply requested grant only (`project/read` for join).
- CAS: delete locks **current** revision; update/transfer compare `revision - 1` after in-memory increment.
- Reconcile: exact DB→FGA writes+deletes, no store reset, ignore organization tuples.
