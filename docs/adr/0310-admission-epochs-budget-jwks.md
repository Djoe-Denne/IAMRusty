# ADR-0310 : Le writer borne atomiquement l'admission des epochs pour préserver le JWKS global et une réserve plateforme

- Statut : Accepted
- Réalité : Partial
- Date : 2026-10-04
- Décideurs : utilisateur, ratification explicite le 2026-10-04 : « Valider ces limites (Recommended) » ; promotion demandée par l’utilisateur, pas Accept autonome
- Jalon concerné : IAM AuthN / fermeture locale S-10 (hors Apparatus P0–P6)
- SuperSède : aucune
- SuperSédée par : —
- Related : [0304](0304-jwt-acces-plateforme-rs256-jwks.md), [0306](0306-hive-iam-configuration-signature.md), [0308](0308-mesh-authn-jwt.md), [0309](0309-remote-signer.md)

`Accepted` ratifie ce contrat et le tuple numérique ci-dessous, sur accord explicite utilisateur du 2026-10-04. `Réalité : Partial` ne concerne que les primitives déjà présentes : registry primaire, epochs/remplacement atomique, publication complète et consommateurs bornés. **Idempotence configure/admission-budget/churn ne sont pas implémentés ou prouvés ; S-10/HIGH reste BLOCK.** Aucun run ou patch applicatif par cette rédaction.

## Contexte

Un admin légitime d'une organisation peut configurer/rotater son propre signer. La source crée un nouveau kid même à binding complète identique, puis conserve les anciennes epochs Retiring pendant TTL access+skew60. Le JWKS partagé peut dépasser le plafond consommateur1MiB ; absence de nouveau snapshot valide puis fail-closed60 rend l'authentification globale indisponible. Le TTL/cache/HTTP bound reste correct : le défaut est l'admission tenant vers le publisher partagé, pas un besoin de relever sa limite ou de rejeter tous les principals org.

Source localisée : `.cursor/review-briefings/20261004-security-iam-infra-final.md` S-10, root `5348a63`, SDK publié `ca2e35fcd56279e9e52625d0df9381f240f3390d`. Pas de nouvelle analyse des constats déjà traités.

## Décision

1. **Autorité unique writer.** Toutes les mutations lifecycle platform/org — bootstrap, configure, rotate, insert Pending/prépublication, promotion/update/revoke — passent la même admission transactionnelle primaire. Advisory transactionnel global, puis org/issuer/rows dans un ordre fixe ; aucune opération remote sous lock. Les chemins lecture/auth/mint et leur fence final ne prennent pas ce verrou.
2. **Publication entière et réservation exacte.** Même predicate avec horloge DB et même DTO/serializer JSON compact UTF8 pour publication et calcul. Pending/Active et Retiring non expirées inclus ; Revoked/Retiring expirées exclus. Réserver la plus grande taille sérialisée de chaque état encore atteignable à bindings identiques, plus envelope/separators. Contrôler aussi la vraie taille de JwkSet ; jamais tronquer/évincer une clé admissible pour tenir le plafond.
3. **Réserve plateforme et quotas ratifiés.** Total768KiB (786432), réserve plateforme64KiB (65536), coût org<=704KiB (720896) ; entrée réservée<=4096octets ; org8 epochs publiables et4 nouveaux kids/fenêtre glissante3600s ; plateforme16 epochs publiables, sans quota de churn tenant. Les promotions ne refacturent pas un même kid ; révoquer n'efface pas l'histoire de churn. Réserve plateforme bornée, pas garantie de rotations infinies.
4. **Bornes de nouveaux matériaux ratifiées.** RSA valide2048..8192bits, exposant canonique impair >=3 tenant dans u64/accepté par backend ; n/e base64url sans padding/leading-zero, RS256, kid opaque32hex et issuer URL valide<=1024octetsUTF8. Le plafond d'entrée comprend les échappements et métadonnées, pas seulement le PEM.
5. **Configure réellement idempotent.** Sous lock après probe, comparer toute binding scope/org/issuer/algorithm/provider_type/provider_key_ref/credential_ref et n/e normalisés. Active identique ⇒ même ligne/kid/timestamps, aucune retraite/admission. Pas égalité du public seule, secret lookup ou credential fallback. Divergence expected epoch⇒409 ; changement de binding/matériau exige nouvelle admission. Rotation explicite garde sa sémantique.
6. **Refus sans dommages.** Capacité/epochs/churn⇒429 redacted, conflit epoch409, matériau invalide400 ; aucun effet partiel/retraite de l'ancien Active. Legacy hors budget⇒préflight/drain naturel/revocation autorisée, pas drop ni blocage des Active valides ; état non clos tant que la borne n'est pas rétablie. Snapshot déjà trop gros n'est pas magiquement réparé par déploiement de quota.

## Conséquences

- Contrat exécutable exact : `.cursor/review-briefings/20261003-auth-fixes-interface-contract.md` §14 ; APIs de retour ligne effective/publication snapshot, constructeur policy obligatoire, immutable DB admission timestamp/index signing_keys, propagation Hive429/409 et migration conservative y sont définis.
- B possède domaines/registry/usecases/adapters IAM ; parent brokerise setup/exports/migration/Cargo et écrivains partagés. Pas de mutation SDK/ext-authz cache/HTTP, nouveau endpoint/shard, sessions globales ou rejet global org.
- Coût : sérialisation de snapshot borné lors des rares mutations lifecycle, contention writer courte ; nouvelles admissions peuvent être refusées même avec permission métier. Le cycle émettre/retirer/révoquer et le point L de l'émission restent inchangés.
- Proofs : SQL races primaire inter-org, bytes exacts/promotions/réserve, no-op complet, rollback/churn/restart, puis vrai publisher et token plateforme non lié au tenant toujours accepté au refresh/après60. Source/review/SDK22 ne sont pas ces preuves.

## Alternatives rejetées

- Hausser1MiB ou prolonger60 : déplace l'attaque et affaiblit les bornes ratifiées.
- Omettre les clés admissibles ou expirer Retiring prématurément : casse les tokens encore légitimes.
- Quota mémoire/process ou simple précheck : bypass inter-replicas/concurrence et baisse de quota par revoke.
- Lock global auth ou shard public JWKS : nouvelle surface/contrat inutile pour cette correction ciblée.

## Non décidé ici

Capacité future à grande échelle/cloud et évolution ultérieure des limites ratifiées. Toute substitution conserve plafond<1MiB/réserve plateforme et caps org/churn finis et requiert un nouvel accord humain. La ratification présente ne livre pas la correction, ne retire pas S-10 du scope et ne constitue pas un override HIGH.

## Références

- Ratification humaine, 2026-10-04 : choix explicite « Valider ces limites (Recommended) », puis demande de promouvoir uniquement ADR0310. Tuple global768KiB/réserve64KiB/entrée4KiB/org8+4 nouveaux kids par3600s/plateforme16, nouvelles bornes RSA/exposant u64 valide/issuer1024. Aucun Accept d’ADR0606 ou d’autres propositions.

- Source : `services/IAMRusty/application/src/usecase/{organization_signer,token}.rs`, `domain/src/entity/{signing_key,token}.rs`, `domain/src/port/signing.rs`, `infra/src/repository/signing_key_registry.rs` (préfixe IAM pour les chemins abrégés).
- Consommateurs inchangés : `workers/ext-authz/src/jwks_cache.rs`, SDK épinglé `rustycog/rustycog-http/src/jwks.rs` ; body1MiB/fail-closed60.
- Baseline wiki antérieure : `projects/aiforall/decisions/0304-access-jwt-trust.md`, `0308-mesh-authn-jwt.md` ; canon0304/0306/0308/0309.
- Preuve actuelle : primitives source uniquement ; admission/idempotence/quota et tests ciblés **non livrés/NOT RUN**. Brief sécurité S-10 et contrat§14, pas clôture root/IT/E2E.

## Mise à jour 2026-10-04 — migrations aplaties

Il n'existe pas de données en production à préserver. Le schéma IAM est livré en un seul fichier de migration initiale, `services/IAMRusty/migration/src/m20220101_000001_initial_schema.rs`. Le contexte task-local, le préflight et le backfill de cutover legacy du 000006 sont supprimés : le contrat §14.B est supersédé sur ce point. La colonne `lifecycle_admitted_at`, son default DB, son index et son trigger d'immutabilité sont conservés dès la création ; la logique d'admission S-10 reste inchangée. Les migrations incrémentales seront réintroduites seulement quand un état persisté devra être préservé. Statut et réalité inchangés ; aucune preuve IT/E2E ajoutée.
