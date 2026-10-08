# ADR-0312 : Le writer réserve des slots JWKS à coût maximal fixe plutôt que recalculer les budgets exacts

- Statut : Accepted
- Réalité : Implemented
- Date : 2026-10-08
- Décideurs : Djoé Denne ; périmètre approuvé le 2026-10-06, rédaction Proposed le 2026-10-07 ; Accept explicite le 2026-10-08, pas Accept autonome
- Jalon concerné : architecture actuelle / IAM admission-publication ; hors P-Apparatus
- SuperSède : [0310](0310-admission-epochs-budget-jwks.md), uniquement le mécanisme de calcul/capacité (§§2–3), effective après Accept explicite du 2026-10-08 ; budgets, réserve, quotas et autres invariants conservés
- SuperSédée par : —
- Related :0310,0311,0304,0306,0308,0200–0202

## Contexte

JWKS global, consommateur strict1MiB et fraîcheur60s. Les conversions/sérialisations de variants et rescans complets par admission coûtent inutilement ; rare activation et quotas KMS ne garantissent pas la taille globale. L'autorité entreprise et toutes ses barrières restent. À la rédaction du 2026-10-07, 0310 demeurait Accepted/Partial sans supersession effective ni clôture S-10 ; la substitution ciblée est désormais effective (mise à jour du 2026-10-08 ci-dessous).

## Décision

- Remplacer tarification variable répétée par slots de coût maximal fixe, cardinalité globale finie, réserve plateforme et caps par domaine. Pending/Active/Retiring non expirée occupent un slot ; revoke/expiry libèrent selon horloge DB et transaction, sans effacer churn.
- Conserver writer786432octets, réserve plateforme65536, budget global org720896, entrée4096, caps8org/16plateforme,4nouveauxkids/3600s, RSA2048..8192 et erreurs400/409/429.720 du test n'est pas720 entreprises. Aucun plafond SDK/TTL relevé.
- Constantes ratifiées et prouvées : `SLOT_BYTES_MAX=4096`, `SLOT_CAP_ORG=8`, `SLOT_CAP_PLATFORM=16`, `SLOT_COUNT_GLOBAL=191`, `SLOT_COUNT_GLOBAL_ORG=175`, `JWKS_ENVELOPE_BYTES=11`. `JWKS_SLOT_FRAME_BYTES_MAX = JWKS_ENVELOPE_BYTES + SLOT_COUNT_GLOBAL * (SLOT_BYTES_MAX + 1) = 782538` octets. Les budgets writer/org/réserve/consommateur restent `786432/720896/65536/1048576` octets. Preuves `ratified_policy_is_finite_and_exact` et `compact_unicode_escaping_reservation_covers_every_reachable_status_and_envelope` : tous champs/statuts atteignables, optional fields, UTF8/échappements, enveloppe et séparateurs inclus ; borne globale et réserve démontrées, pas de division naïve.
- Public/binding canonicalisés une fois ; validation ponctuelle de pire entrée permise si les bornes seules ne prouvent pas4096. Réutiliser n/e/DTO ; changement de binding/version refait validation. Admission par comptages indexés ou compteurs SQL transactionnels sous writer global, pas mémoire process/précheck seul/full-table parse.
- Snapshot canonique matérialisé/sérialisé par révision, transition ET expiration ; même predicate/horloge. Mesurer le payload déjà produit AVANT activation/retraite/effets ; refus sans dommages, ni troncature/éviction. Conserver Pending/prépublication, mutation epoch, publisher et fence ; pas de reset de trust60s par GET.
- Idempotence binding complète ; promotions/retries ne refacturent pas. Churn distribué et historique malgré revoke conservés via requêtes indexées/agrégats, pas chargement/tri global ; rate limiter local n'est pas un remplacement.
- Préflight legacy/counters/snapshots, drain naturel ou révocation autorisée si hors nouvelle capacité ; aucune suppression automatique d'Active ni bascule sur données non conformes.
- Fixtures neutres préparées admises ; frontière seedée seulement si cohérente/validée indépendamment, puis vrais appels/refus/race dernière place et snapshot avant/après. Mapping risques/assertions ancien→nouveau obligatoire ; aucun skip, baisse de RSA, retrait provider/transport/guards ou SQL auto-confirmant.

## Conséquences

Capacité plus conservatrice approuvée ; nombres dérivés et ratifiés sur les tests de preuve ci-dessous ; aucun gain promis. Comptages/indices, snapshots et expiry nécessitent migration/invalidation cohérentes. Garder concurrence inter-org/inter-replicas, refus Active intacte, proof public/privé et fences après I/O. Tests oracle JSON/escaping/variants, réserve/dernier slot, idempotence/churn/revoke/expiry/legacy et N/N+1 indispensables. Rollback contrôlé préserve state/histoire/trust ; jamais restaurer PEM production interdit par0311. Aucun nouveau runner, resetDB, service ou rate-limit équivalent choisi ici.

## Références

- `services/IAMRusty/domain/src/entity/signing_publication.rs:19–30` ; `services/IAMRusty/domain/src/entity/signing_key.rs:470,813,835` ; `domain/src/entity/token.rs` ; repository signing_key_registry ; publisher token usecase.
- `docs/adr/0310-admission-epochs-budget-jwks.md:23–27,45` ;0304/0308 ;0200–0202.
- `docs/local/20261006-signing-simplification-inventory.md`, `20261006-iam-it-runtime-contract.md` et prompt20261007.
- Réalité Implemented : constantes/entrées publiques préparées livrées dans `services/IAMRusty/domain/src/entity/signing_publication.rs:19–30`, comptage `SigningSlotCounts` branché (`signing_key.rs:470`) et preuves numériques/échappements présentes (`signing_key.rs:813,835`).

## Réalité courante — 2026-10-08

- Ratification explicite de Djoé Denne : **Accepted / Implemented** ; la substitution ciblée du calcul/capacité de 0310 devient effective, sans remplacer son autorité writer, ses budgets/réserve, son churn, son idempotence ni ses refus sans dommages.
- Implémentation livrée après la rédaction Proposed, vérifiée par le contrôleur au HEAD `e8fcaea` : `services/IAMRusty/domain/src/entity/signing_publication.rs:19–30` porte les constantes ci-dessus ; `services/IAMRusty/domain/src/entity/signing_key.rs:470` utilise `SigningSlotCounts` pour l'admission par slots fixes. Les tests `ratified_policy_is_finite_and_exact` (`signing_key.rs:813`) et `compact_unicode_escaping_reservation_covers_every_reachable_status_and_envelope` (`signing_key.rs:835`) établissent les bornes numériques et de sérialisation.
- Suites IAM **unitaires + IT vertes sur master**, exécutées et confirmées par Djoé Denne le 2026-10-08 ; suites `signing_epoch_writer`, `transit_protocol` et `signing_admission` étoffées. **Attestation utilisateur**, sans artefact CI archivé et sans nouveau run dans ce lot. Ni E2E exact-route/Envoy de 0412 ni clôture sécurité globale S-10 ne sont attestées ici.
- Écart documentaire résiduel : `services/IAMRusty/domain/src/entity/signing_publication.rs:1–4` porte encore « ADR-0312 candidate » / « These numbers remain Proposed ». Ce commentaire historique ne décrit plus le Statut canonique ; aucun code n'est modifié par cette réconciliation.
