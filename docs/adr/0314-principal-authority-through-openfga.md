# ADR-0314 : Conserver l’autorité du principal dans les sujets et projections OpenFGA

- Statut : Accepted
- Réalité : Partial
- Date : 2026-10-28
- Décideurs : proposition architecte ; ratification explicite du lead le 2026-10-28 dans la délégation utilisateur de la mission authentification, politique approuvée révision 1
- Jalon concerné : AuthN/AuthZ multi-autorité, hors P-Apparatus
- SuperSède : aucune
- SuperSédée par : —
- Related : [0305](0305-account-identity-trust-domain.md), [0302](0302-authn-jwt-authz-openfga.md), [0303](0303-sentinel-sync-worker-fga.md), [0308](0308-mesh-authn-jwt.md), [0402](0402-hive-organisations.md)

The lead explicitly ratified this decision on 2026-10-28 under approved user
delegation. It resolves the OpenFGA multi-issuer question left open by 0305
without changing 0305's authentication scope. Ratification was not evidence of
implementation. The [secondary contract](../contracts/authz-trust-domain-boundary.md)
remains in force; see the dated implementation snapshot below.

## Contexte

0305 impose `(iss, sub)` mais laisse explicitement le schéma FGA ouvert. À la base application `c72ae362b950c63d6736a6818c68babc5a8d6f40` et SDK `d60a88b2f2f499768a807941a1b4c80c9a366b3f`, `with_principal` conserve le principal et insère aussi son UUID. `permission_middleware` réduit ce contexte à `Subject::new(user_id)` ; `Subject`, son Display et le cache ne connaissent pas l’autorité. Les projections Hive utilisent `Tuple::user` même quand un événement porte `issuer`. Deux autorités reconnues peuvent ainsi désigner le même sujet FGA.

Hive persiste déjà un issuer de membership, mais `platform_issuer_aliases` accepte toute URL terminée par `/iam` comme alias historique. Les événements de création/suppression d’organisation ne portent pas les autorités des propriétaires/membres. Corriger seulement le lecteur HTTP laisserait des writers, suppressions et alias incohérents. Les gardes account-only IAM ne protègent pas les routes signer Hive.

## Décision

1. Le sujet utilisateur authentifié conserve l’issuer exact et le sub UUID jusqu’au checker et à sa clé de cache. Le UUID seul reste un identifiant local, jamais une preuve d’autorité. Les chemins JWT direct et mesh aboutissent au même résolveur.
2. Conserver le type FGA `user` et les relations actuelles. L’unique autorité plateforme explicitement configurée garde `user:<uuid>` pour les droits historiques dont la provenance est établie. Les autres autorités utilisent `user:p1.<base64url-sans-padding(UTF8(iss))>.<uuid-canonique>`. Cet encodage injectif ne dépend ni de `org`, ni du hostname de la requête, ni du kid. Borne de représentation : issuer de 1 à 256 octets UTF-8 ; au-delà, refus explicite au contrôle de permissions, jamais troncature/hash de secours.
3. L’équivalence avec `iamrusty` est une compatibilité de données plateforme attestée et, pour des JWT historiques, une fenêtre de vérification explicitement configurée. Aucun alias déduit d’un suffixe, d’une forme d’URL ou d’un sub identique. La politique exacte est commune aux lecteurs et writers.
4. Hive reste la source de vérité des membres/rôles. Événements et projections utilisent le principal du membre/propriétaire effectivement persisté, non celui de l’acteur administrateur ; suppression/rotation de rôles ne touche que cette autorité. Événements ambigus et anciens tuples non attribuables sont mis en quarantaine, pas promus en droits plateforme.
5. Refus local des identités incompatibles aux interfaces qui adressent encore un HumanAccount ou un état UUID-only. Ce n’est pas un bannissement AuthN des identités organisation : elles restent représentées, stockées dans Hive et autorisables par leur propre sujet FGA. Les routes qui ne peuvent les préserver ne les réduisent pas silencieusement au UUID.
6. Bascule coordonnée après réconciliation de l’état legacy depuis les sources de vérité. Pas de double lecture « sujet qualifié OU UUID », de double grant permissif ou de rollout mixte reader/writer exposé. Conserver la lecture publique `user:*` sans convertir un principal incompatible en anonyme.

## Conséquences

Impact architectural, API SDK publique et format persistant des clés de tuples. Changement borné du type Subject, de ses constructeurs/serde/cache et des projections ; pas de nouveau service, PDP IAM, type FGA ou relation. Les commandes/réponses qui adressent un membre sans issuer doivent être qualifiées ou refuser l’ambiguïté avant effet. L’ajout d’issuer aux propriétaires Hive et aux événements de suppression est nécessaire pour éviter de fabriquer l’autorité à la projection.

La compatibilité des droits plateforme est conservée quand leur provenance est vérifiable ; un ancien tuple org aplati ne devient pas un droit plateforme du seul fait de son format. Une indisponibilité temporaire pour réconciliation est préférable à un mélange d’autorités. Les règles de migration, erreurs, interfaces, allowlist et oracles sont normatives dans le contrat lié.

## Alternatives écartées

- **Refuser toutes les identités organisation dans AuthN ou toutes les permissions** : correctif plus petit mais supprime le modèle multi-domaines de 0305 et ses parcours positifs. Le refus est réservé aux frontières réellement incompatibles.
- **UUID global ou claim org comme namespace** : perd l’émetteur ou confond contexte de trust et permission ; ne ferme pas la collision.
- **Ajouter un nouveau type FGA par autorité** : croissance du modèle et migration des relations inutiles ; l’ID opaque du type user suffit.
- **Tout renommer, plateforme incluse** : plus uniforme mais migration de tous les droits sans nécessité ; la compatibilité plateforme peut être bornée explicitement.
- **Fallback legacy / hash tronqué / alias par suffixe** : ambiguïté ou collision, réintroduit le défaut dans les readers, writers ou caches.

## Non décidé ici

Mint de JWT organisation, UX de switch/linking, généralisation des persistances Manifesto/Telegraph/Lazaret au multi-autorité, HSM/cloud, nouvel algorithme de révocation ou nouvelle plateforme de preuve. Un besoin d’étendre un parcours UUID-only au lieu de le refuser localement revient à l’architecte, pas à une migration implicite par le consommateur.

## Références et réalité

**Implementation snapshot — 2026-11-21 (application `50c36ade`, SDK `40a6a7a5`).**
Commit `537ee032` integrated issuer-qualified permission subjects. The selected
SDK retains the issuer in permission checks and cache identity; Hive persists
the member's issuer and [sentinel-sync](../../workers/sentinel-sync/src/translator/hive.rs)
uses it when projecting the member's permissions. The [Hive signer test](../../services/Hive/tests/organization_signer_permissions.rs)
exercises distinct issuers with the same subject, and the [November 13 report](../missions/20261113-coverage90-local.md)
records targeted Hive results, including preservation of the target issuer.

This is **Partial**: source integration and scoped test evidence do not establish
that legacy grants were reconciled in a deployed environment, that readers and
writers were cut over together there, or that every consumer supports organization
identities. The criteria for coordinated cutover and verified legacy provenance
are unchanged. No additional runtime tests were run for this update.

- Canon : 0305 §§Décision/État runtime/Non décidé ; 0302 garde OpenFGA PDP ; 0303 writers événementiels ; 0308 §7 confiance du principal mesh.
- SDK : `rustycog/rustycog-http/src/{middleware_auth,middleware_permission,jwt_handler}.rs`, `rustycog/rustycog-permission/src/{lib,checker}.rs`.
- Hive : `services/Hive/domain/src/entity/organization_member.rs`, `services/Hive/application/src/usecase/{member,organization}.rs`, `services/Hive/http/src/handlers/{members,organization_signer}.rs` ; `crates/hive-events/src/{member,organization}.rs`.
- Projections : `workers/sentinel-sync/src/{fga_client,translator/hive}.rs` ; `ops/openfga/model.fga`.
- Manifesto's [world-read use case](../../services/Manifesto/application/src/usecase/world_read.rs)
  still has platform-account-only paths. Its [public-read handlers](../../services/Manifesto/http/src/handlers/mod.rs)
  retain the authenticated issuer for audit and exclude non-platform principals
  from UUID-only membership and admin lookups.
- Handbook: [JWT authentication](../platform/authn-jwt.md) and
  [OpenFGA authorization](../platform/authz-openfga.md). ADRs 0200/0302 and
  the real Hive harness remain the references for integration tests.
- Wiki conception consultée via QMD : `obsidian/AI FOR ALL/projects/aiforall/decisions/0304-access-jwt-trust.md` ; son 0306 Implemented diffère du canon Partial, sans force normative.
- Historical evidence: the 2026-10-28 contract-only ratification did not run
  behavioral tests; its formatting attempt hit Windows OS206. See the
  [original validation record](../contracts/authz-trust-domain-boundary.md#8-validation-nommée--à-inscrire-au-plan-qa-avant-exécution).
  That record is not a verdict on the subsequent implementation.
