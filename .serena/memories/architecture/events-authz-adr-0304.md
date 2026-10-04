# ADR-0304 — état source 2026-10-04

Statut : Accepted. Réalité : Partial. Canon : docs/adr/0304-jwt-acces-plateforme-rs256-jwks.md ; voir le fichier ADR.

- RS256/JWKS/trust : sources de métadonnées, snapshot vide autoritatif et fraîcheur monotone60 écrites ; f060d47/ba69c9e = historique, pas état courant.
- SDK ca2e35fcd56279e9e52625d0df9381f240f3390d publié selon parent, checkout/gitlink worktree sélectionné ; 22 pure SDK PASS selon parent, pas units/publisher IAM. Root HEAD5348a63 non committé.
- ANY-RS256 access/registration : codec commun, fence SELECT primaire après signer, Active seul émet ; paire PEM/candidat contrôlés dans source B revue PASS statique.
- Remote signer HTTP 0309 présent/Partial, sans HSM/KMIP/adapters cloud livrés ; aucun cloud futur exigé pour la preuve du lab.
- Compilation root complète, units IAM, IT SQL/publication/concurrence et E2E mesh encore à exécuter après intégration ; pas Implemented ni fermeture globale.
- Mise à jour 2026-10-04 — migrations aplaties : aucune donnée en production ; schéma IAM initial unique `m20220101_000001_initial_schema.rs`, outbox comprise, sans préflight/backfill legacy. Migrations incrémentales seulement quand un état persisté devra être préservé. Statut Accepted / réalité Partial inchangés ; aucune preuve IT/E2E ajoutée.
