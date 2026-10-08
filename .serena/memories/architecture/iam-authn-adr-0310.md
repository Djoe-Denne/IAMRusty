# IAM AuthN — ADR-0310 admission epochs/JWKS

Statut : Accepted. Réalité : Partial. Canon : docs/adr/0310-admission-epochs-budget-jwks.md ; voir le fichier ADR.

- Ratification initiale explicite 2026-10-04 : budget768KiB/réserve64KiB/entrée4KiB, org8 epochs +4 nouveaux kids/3600s, plateforme16 ; RSA2048..8192/exposant u64/issuer<=1024 octets.
- SuperSédée partiellement par ADR-0312 Accepted / Implemented le 2026-10-08 : uniquement calcul/capacité (§§2–3), désormais slots à coût maximal fixe/cardinalités finies. Accepted / Partial conservés pour le reste, pas retrait de l’ADR entière.
- Writer primaire atomique, publication entière, budgets/réserve/caps/churn, binding complète idempotente, consommateurs1MiB/fraîcheur60s, fences et refus sans dommages inchangés.
- Mécanisme livré selon 0312 : signing_publication.rs:19–30 et signing_key.rs:470,813,835 (préfixe services/IAMRusty/domain/src/entity/) ; units + IT IAM vertes sur master confirmées par l’utilisateur le 2026-10-08, sans artefact CI archivé. L’assertion admission/capacité non livrées est historique ; pas clôture sécurité globale S-10 ni preuve E2E ici.
- Migration initiale unique m20220101_000001_initial_schema.rs, aucune donnée en production ; §14.B task-local/préflight/backfill legacy historiquement remplacé, colonne/default/index/trigger d’admission conservés.
