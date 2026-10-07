# ADR-0312 : Le writer réserve des slots JWKS à coût maximal fixe plutôt que recalculer les budgets exacts

- Statut : Proposed
- Réalité : Unimplemented
- Date : 2026-10-07
- Décideurs : Djoé Denne ; périmètre approuvé le2026-10-06, statut formel Proposed demandé
- Jalon concerné : architecture actuelle / IAM admission-publication ; hors P-Apparatus
- SuperSède : aucune effective tant que Proposed ; substitution ciblée du calcul/capacité de0310 après Accept explicite
- SuperSédée par : —
- Related :0310,0311,0304,0306,0308,0200–0202

## Contexte

JWKS global, consommateur strict1MiB et fraîcheur60s. Les conversions/sérialisations de variants et rescans complets par admission coûtent inutilement ; rare activation et quotas KMS ne garantissent pas la taille globale. L'autorité entreprise et toutes ses barrières restent.0310 demeure Accepted/Partial sans supersession administrative ni clôture S-10.

## Décision

- Remplacer tarification variable répétée par slots de coût maximal fixe, cardinalité globale finie, réserve plateforme et caps par domaine. Pending/Active/Retiring non expirée occupent un slot ; revoke/expiry libèrent selon horloge DB et transaction, sans effacer churn.
- Conserver writer786432octets, réserve plateforme65536, budget global org720896, entrée4096, caps8org/16plateforme,4nouveauxkids/3600s, RSA2048..8192 et erreurs400/409/429.720 du test n'est pas720 entreprises. Aucun plafond SDK/TTL relevé.
- SLOT_MAX_BYTES, FRAME_MAX_BYTES, MAX_ORG_SLOTS et capacité plateforme restent **TBD par preuve avant bascule**. Inclure tous champs/statuts atteignables, optional fields, UTF8/échappements, enveloppe et séparateurs ; prouver borne globale et réserve. Aucune division naïve ni valeur inventée.
- Public/binding canonicalisés une fois ; validation ponctuelle de pire entrée permise si les bornes seules ne prouvent pas4096. Réutiliser n/e/DTO ; changement de binding/version refait validation. Admission par comptages indexés ou compteurs SQL transactionnels sous writer global, pas mémoire process/précheck seul/full-table parse.
- Snapshot canonique matérialisé/sérialisé par révision, transition ET expiration ; même predicate/horloge. Mesurer le payload déjà produit AVANT activation/retraite/effets ; refus sans dommages, ni troncature/éviction. Conserver Pending/prépublication, mutation epoch, publisher et fence ; pas de reset de trust60s par GET.
- Idempotence binding complète ; promotions/retries ne refacturent pas. Churn distribué et historique malgré revoke conservés via requêtes indexées/agrégats, pas chargement/tri global ; rate limiter local n'est pas un remplacement.
- Préflight legacy/counters/snapshots, drain naturel ou révocation autorisée si hors nouvelle capacité ; aucune suppression automatique d'Active ni bascule sur données non conformes.
- Fixtures neutres préparées admises ; frontière seedée seulement si cohérente/validée indépendamment, puis vrais appels/refus/race dernière place et snapshot avant/après. Mapping risques/assertions ancien→nouveau obligatoire ; aucun skip, baisse de RSA, retrait provider/transport/guards ou SQL auto-confirmant.

## Conséquences

Capacité plus conservatrice approuvée, nombres dérivés encore à prouver ; aucun gain promis. Comptages/indices, snapshots et expiry nécessitent migration/invalidation cohérentes. Garder concurrence inter-org/inter-replicas, refus Active intacte, proof public/privé et fences après I/O. Tests oracle JSON/escaping/variants, réserve/dernier slot, idempotence/churn/revoke/expiry/legacy et N/N+1 indispensables. Rollback contrôlé préserve state/histoire/trust ; jamais restaurer PEM production interdit par0311. Aucun nouveau runner, resetDB, service ou rate-limit équivalent choisi ici.

## Références

- `services/IAMRusty/domain/src/entity/signing_key.rs:354–459,473–491` ; `domain/src/entity/token.rs` ; repository signing_key_registry ; publisher token usecase.
- `docs/adr/0310-admission-epochs-budget-jwks.md:23–27,45` ;0304/0308 ;0200–0202.
- `docs/local/20261006-signing-simplification-inventory.md`, `20261006-iam-it-runtime-contract.md` et prompt20261007.
- Réalité Unimplemented : aucune implémentation slots ni preuve numérique livrée par cette rédaction.