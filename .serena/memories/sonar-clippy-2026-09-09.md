2026-09-09: Sonar project `Djoe-Denne_IAMRusty` had 120 OPEN Clippy MAJOR issues collapsing to 33 unique locations.

Fixed in code: missing_const_for_fn, doc_markdown (`OpenFGA`/`MailHog`/`AuthZ`), redundant_pub_crate (world_read `pub`), needless_pass_by_value (iam-http error mappers by ref), option_if_let_else (project_repository map_or_else), match_same_arms (sentinel-sync ProjectPublished), expect_used Telegraph setup_event_mapping → Result, Hive fixture docs (# Errors/# Panics + short first paragraph), significant_drop_tightening (drop MutexGuard).

False positive kept: Hive `organization_invitation_repository.rs` `serde_json::to_value(...).expect("Serialize")` — issue `AaBYFf_0Oc6KcqsU_Qat` marked falsepositive per Aug 2026 policy.

Quality gate still ERROR on `new_coverage` 64.1% vs 80%. Security hotspots TO_REVIEW: 0.