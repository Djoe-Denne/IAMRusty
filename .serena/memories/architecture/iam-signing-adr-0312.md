# IAM admission-publication — ADR-0312

Statut : Accepted. Réalité : Implemented. Canon : docs/adr/0312-admission-jwks-par-slots-bornes.md ; voir le fichier ADR.

- Accept explicite de Djoé Denne le 2026-10-08 ; supersession partielle du calcul/capacité de 0310 effective, autres invariants conservés ; 0310 reste Accepted / Partial.
- Constantes livrées : SLOT_BYTES_MAX=4096, SLOT_CAP_ORG=8, SLOT_CAP_PLATFORM=16, SLOT_COUNT_GLOBAL=191, SLOT_COUNT_GLOBAL_ORG=175, JWKS_ENVELOPE_BYTES=11 ; frame dérivé=782538. Budgets 786432/720896/65536 et consommateur1MiB inchangés.
- Code : services/IAMRusty/domain/src/entity/signing_publication.rs:19–30 ; SigningSlotCounts branché dans signing_key.rs:470 ; preuves ratified_policy_is_finite_and_exact et compact_unicode_escaping_reservation_covers_every_reachable_status_and_envelope (signing_key.rs:813,835).
- Writer atomicité, réserve, idempotence/churn, fraîcheur60s, publication entière et fences conservés ; pas de gain perf promis.
- Units + IT IAM vertes sur master confirmées par Djoé Denne le 2026-10-08, attestation utilisateur sans artefact CI archivé ; pas E2E 0412 ni clôture sécurité globale S-10. Commentaire source candidate/Proposed (signing_publication.rs:1–4) périmé, non modifié dans le lot documentaire.
