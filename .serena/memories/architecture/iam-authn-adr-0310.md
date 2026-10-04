# ADR-0310 — admission atomique epochs/JWKS

Statut : Accepted. Réalité : Partial. Canon : docs/adr/0310-admission-epochs-budget-jwks.md ; voir le fichier ADR.

- Accord explicite utilisateur 2026-10-04 : « Valider ces limites (Recommended) », promotion demandée uniquement pour0310 ; aucune Accept0606/cloud implicite.
- Ratifié : global768KiB/réserve plateforme64KiB/entrée4KiB, org8 epochs et4 nouveaux kids/fenêtre glissante3600s, plateforme16 ; nouveaux RSA2048..8192/exposant u64 valide/issuer<=1024octets.
- Writer atomique tous lifecycle paths, snapshot DB/serializer complet ; configure même binding complète+n/e = même Active/kid/timestamps, refus429/409 sans retraite partielle. Consommateurs1MiB/cache60 inchangés.
- Partial = primitives registry/epochs/publication déjà présentes seulement ; idempotence/admission/quota non livrés/provés, S-10 HIGH BLOCK. Ratification ≠ override HIGH ni livraison.
- Contrat§14 autoritaire pour B/parent/E ; implémentation, revue ciblée puis IT publisher/races/plateforme après60 et E2E intégrés encore requis. Aucun run/app patch dans cette ratification.
- Mise à jour 2026-10-04 — migrations aplaties : aucune donnée en production ; schéma IAM initial unique `m20220101_000001_initial_schema.rs`. §14.B supersédé : task-local/préflight/backfill legacy supprimés ; colonne/default/index/trigger d'admission conservés, logique S-10 inchangée. Migrations incrémentales seulement quand un état persisté devra être préservé. Statut Accepted / réalité Partial inchangés ; aucune preuve IT/E2E ajoutée.
