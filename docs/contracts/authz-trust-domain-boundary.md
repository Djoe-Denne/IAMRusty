# Contrat ratifié — frontière AuthZ des trust domains

- Version : 1, 2026-10-28 ; **Accepted / Unimplemented**.
- Canon décisionnel : [ADR-0314](../adr/0314-principal-authority-through-openfga.md), complète [0305](../adr/0305-account-identity-trust-domain.md).
- Base examinée : application c72ae362b950c63d6736a6818c68babc5a8d6f40 ; SDK d60a88b2f2f499768a807941a1b4c80c9a366b3f.
- Ratification : explicite par le lead le 2026-10-28 sous délégation utilisateur de la politique approuvée révision 1, après examen des alternatives, sources, compatibilité et oracles ; aucune acceptation par l'auteur ni preuve de code livré. Intégration vérifiée des documents requise avant publication de corriger-identites.

## 1. Invariants

1. Un JWT vérifié ou un principal mesh reçu sur le transport de confiance de 0308 produit le même couple exact `(iss, sub)`. Ni header externe, ni UUID préinjecté, ni org/kid ne constitue cette preuve.
2. Deux issuers indépendants reconnus avec le même sub n’ont ni la même clé de cache, ni le même sujet Check, ni les mêmes grants/suppressions. L’exception est une équivalence plateforme historique explicitement attestée (§3), jamais une équivalence entre autorités indépendantes.
3. FGA reste le PDP ; Hive reste SoT membership/rôles. La permission concerne l’organisation du path exact, jamais une organisation du JSON ni le dernier UUID imbriqué par erreur. IAM ne devient pas le PDP.
4. Un refus précède commande, RPC IAM, mutation DB/outbox/registry/provider. Les quotas, signatures/versions autoritatives, instantanés vides, rotation et révocation/fraîcheur maximale de 60 s existants ne changent pas.
5. Les identités organisation ne sont pas bannies : un membre org qualifié avec un grant légitime peut agir dans Hive. Un HumanAccount ou état métier stocké seulement par UUID n’accepte pas une identité incompatible sous prétexte que FGA a autorisé son autre sujet.
6. Lecture publique `user:*` inchangée, sans héritage write/invoke/secret. Un principal authentifié refusé n’est jamais remplacé par wildcard pour contourner ce refus. Absence réelle de principal sur route optional conserve le chemin public actuel.

## 2. Interfaces SDK — noms normatifs, détails privés libres

Dans permission, introduire `PrincipalRef { issuer: String, subject: Uuid }`, validé (issuer non vide, non paddé, 1..=256 octets UTF-8). Égalité d’issuer octet pour octet : pas de trim, lowercase, suppression de slash, décodage URL, sous-chaîne `/iam` ou extraction de slug. Refuser plutôt que normaliser une identité invalide.

`Subject` distingue `LegacyPlatform`, `Principal(PrincipalRef)` et `Wildcard` ; conserver `Subject::new(Uuid)` comme constructeur legacy **explicitement plateforme**, utilisable pour les fixtures et producteurs account-only déjà prouvés. Ajouter `Subject::from_principal(PrincipalRef, &PrincipalPolicy) -> Result<Subject, PrincipalError>` et `Subject::wildcard()`. Un HTTP authentifié ne peut appeler new(UUID) directement. La variante Principal conserve l’autorité même quand son rendu FGA bénéficie de la compatibilité plateforme. Adapter Clone/ownership si Copy n’est plus possible, sans fuite d’autorité dans serde/Eq/Hash.

`PrincipalPolicy` est construite au composition root : `platform_issuer` exact obligatoire, plus `allow_historical_platform_issuer` explicite (false hors fenêtre de compatibilité vérifiée). Pas de valeur réseau déduite du Host, X-Forwarded-Host ou du premier token. Dans les consommateurs, ajouter ces champs dans `[auth.jwt]` et les structures SDK correspondantes ; workers reçoivent la même paire sous `[principal]`. L’issuer HS256 existant n’est pas une preuve que tous les issuers JWKS sont plateforme. IAM fournit sa valeur attendue à partir de sa configuration signante, monolithe inclus. Configuration absente/incohérente : échec de démarrage ; pas de valeur permissive.

`permission_middleware` et `optional_permission_middleware` utilisent le JwtPrincipal vérifié et la politique commune. `with_principal` enlève/remplace les extensions d’identité héritées avant installation. Supprimer tout chemin qui fait autorité depuis `Extension<Uuid>` seule. Conserver les extractors UUID legacy seulement comme vue plateforme vérifiée : `AuthUser` et `OptionalAuthUser` refusent une identité non plateforme au lieu de retourner son sub. Introduire `AuthPrincipal`/`OptionalAuthPrincipal` pour les handlers réellement compatibles. Pas de restriction d’issuer globale dans le vérificateur JWT pour simuler ce correctif.

Le trait `PermissionChecker::check(Subject, Permission, ResourceRef)` reste la seam. OpenFGA, InMemory, Cached et Metrics partagent la sémantique. La clé cache inclut l’autorité exacte et le genre du sujet ; jamais `user_id` seul. `cache_ttl_seconds=0` reste un bypass en IT. Une ancienne sérialisation Subject sans issuer est lisible comme **LegacyPlatform**, pas comme preuve AuthN ; interdit de l’utiliser pour reconstituer un principal de requête.

Erreurs (codes internes stables ; réponse externe minimale sans issuer/token) :
- `PRINCIPAL_REQUIRED` : 401 si absence de principal authentifié, même si un UUID est présent ; JWT invalide conserve 401.
- `PRINCIPAL_INVALID` / `PRINCIPAL_INCOMPATIBLE` : 403 au guard si représentation invalide ou interface UUID-only incompatible.
- `AUTHZ_DENIED`, `AUTHZ_RESOURCE_INVALID`, `AUTHZ_CHECK_FAILED` : 403, comme aujourd’hui ; aucune erreur checker n’est un allow.
- `PRINCIPAL_POLICY_UNAVAILABLE` : refus fermé (503 pour extractor hors root configuré), jamais poursuite.
- `MEMBER_PRINCIPAL_AMBIGUOUS` : 409 pour cible métier non univoque, sans effet ; ne pas choisir arbitrairement un membre de même UUID.
- `EVENT_PRINCIPAL_UNRESOLVED` : erreur de projection/quarantaine visible, aucune écriture de tuple, pas un succès silencieux. Réutiliser la gestion d’échec du worker, sans nouvelle infrastructure de queue.

## 3. Représentation FGA et droits persistés

Le type reste `user`. Pour une autorité non plateforme : `user:p1.<base64url-sans-padding(UTF8(issuer))>.<uuid-en-minuscules-avec-tirets>`. Alphabet sans séparateurs FGA ni espace ; issuer au plus 256 octets, ID borné à 382 caractères (hors `user:`). Encodage réversible et injectif, pas de dépendance à une collision de hash. Réutiliser le codec SDK dans le writer, pas deux implémentations divergentes. `user:*`, objets `organization/project/component/notification` et usersets gardent leur syntaxe et sémantique. Aucun changement de relation ni nouveau type au modèle.

L’issuer plateforme exact conserve le fil `user:<uuid>` pour les droits **attribués** à cette autorité par provenance. `iamrusty` dans une ancienne ligne DB peut être rapproché de cette autorité par migration attestée ; son admission comme issuer de JWT exige en plus la fenêtre de vérification explicite. Aucun autre émetteur terminé par `/iam` n’est alias. `platform_issuer_aliases` / `membership_lookup_issuers` ne peuvent plus établir cette confiance par forme d’URL : la compatibilité est configurée puis matérialisée à la migration, les lookups courants sont exacts.

**Pas de fallback** de sujet p1 vers user:UUID, même en panne, permission absente, replay, renommage de slug ou migration partielle. Changer l’issuer exact est un changement d’identité ; aucun suivi automatique via org_id, email ou HumanAccount. Un tuple legacy dont l’issuer a été perdu n’est pas automatiquement un droit plateforme.

## 4. Hive, événements et sentinel-sync

- Membership : clé unique `(organization_id, issuer, user_id)`, déjà portée par le schéma ; ne pas la réduire. Les lookup/update/delete ciblent les trois champs. Stocker l’issuer exact, pas l’alias heuristique.
- Propriétaire : ajouter `owner_issuer` à l’entité, persistence et événement OrganizationCreated, renseigné depuis le principal créateur. `owner_user_id` reste compatible mais n’est pas seul un principal. Un propriétaire historique non attribuable bloque la réconciliation, pas de default arbitraire.
- Membres : actor et target sont distincts. Ajouter un `issuer` cible explicite aux demandes de création/édition ciblée ; pour les anciennes routes `{user_id}`, l’absence d’issuer cible signifie uniquement l’autorité vérifiée du caller, jamais « premier membre portant ce UUID ». Pour agir dans une autre autorité, fournir l’issuer cible, le traiter comme sélecteur non privilégié et vérifier les permissions existantes. Réponses membership exposent issuer + user_id ; listes peuvent montrer plusieurs autorités sans les fusionner.
- Tout handler Hive compatible passe à AuthPrincipal ; commandes/usecases conservent l’autorité de l’acteur dans leur contexte. Un UUID d’audit peut rester une vue locale seulement si l’issuer est conservé à côté et n’est jamais utilisé comme preuve de droit.
- `MemberJoined/Removed/RolesUpdated.issuer` contient l’issuer de la ligne **cible**, y compris quand acteur et cible diffèrent. Ni le JWT de l’admin ni une valeur de body seule ne le remplace. Writes et deletes sont encodés par la même seam.
- `OrganizationDeleted` ajoute `owner_issuer` et `member_principals: Vec<{issuer, user_id}>`, pris depuis le writer avant suppression. Conserver les anciens champs pour décodage legacy, sans fusionner/dédupliquer les membres par UUID. Une suppression enlève chaque principal qualifié ; un event ancien uniquement UUID ne peut effacer toutes les autorités.
- Les événements nouveaux qui changent des droits portent ces autorités ; les champs peuvent rester optionnels pour décoder les anciens messages, mais l’absence n’autorise pas à émettre un grant legacy. Les constructeurs/producteurs doivent remplir les valeurs avant publication. Replay ancien : résolution attestée depuis snapshot/SoT/tombstone, sinon quarantaine. Ne pas deviner un issuer à partir du path d’issuer d’une autre org.
- `Tuple::for_subject` (nouvelle seam) reçoit Subject et partage le rendu canonique. `Tuple::user` reste réservé aux événements account-only attestés plateforme. Hive utilise for_subject sur tous les bras droits/roles/delete. Le test actuel qui exige que issuer soit ignoré est remplacé par la propriété de séparation.

## 5. Frontières encore UUID-only

IAM PlatformUser conserve sa garde exacte et `org.is_none()` ; ne pas la remplacer par la simple présence d’un Subject. Les routes Hive signer sont au contraire FGA/principal-aware : une identité organisation **avec son grant propre** est admissible, sans accès aux endpoints HumanAccount IAM. Ne pas appeler IAM pour découvrir si un caller est admin.

Manifesto et Telegraph possèdent des données de membres/comptes/destinataires adressées par UUID. Dans ce lot, ne pas convertir ces schémas en multi-autorité : les usages authentifiés UUID-only rejettent explicitement les autres autorités avant lookup/commande, y compris lorsqu’un grant qualifié est présent. Les deux usages Subject(UUID) de Manifesto (`project`, `world_read`) ne reçoivent qu’une vue plateforme déjà validée, jamais le sub brut d’un principal org. Les routes de lecture publique conservent leurs positifs anonymes et plateforme ; un principal org peut lire ce qui est réellement public par un chemin explicitement public utilisant OptionalAuthPrincipal, sans exploiter son UUID dans les branches membres/admin. Une ressource non publique ou suspendue ne bénéficie pas de ce chemin. Ce n’est pas une conversion implicite en anonyme d’un refus de permission.

Même règle pour tout consommateur UUID-only trouvé lors de l’inventaire final (notamment Lazaret à la frontière HumanAccount), sans modifier sa session workload, grants, protocole ou schéma. Une extension fonctionnelle à ces identités est hors lot : remonter le besoin et ses chemins au lead/architecte avant édition. Les événements Manifesto/Telegraph UUID-only restent plateforme seulement après ce contrôle producteur. Ne pas confondre le destinataire d’une notification HumanAccount avec une identité de membership Hive.

## 6. Migration et déploiement

Pas de données production annoncé par 0305 (2026-10-04) ne signifie pas « aucun état local à préserver aujourd’hui ». Le consommateur inventorie DB, tuples et événements en attente, sans suppression/reset implicite.

1. Suspendre l’admission des opérations concernées et les producteurs/projecteurs le temps de la bascule ; identifier versions et snapshot de restauration. Pas de rollout exposé mélangeant ancien lecteur et nouveau writer. Une simple fenêtre de coexistence de deux schémas n’est pas sûre.
2. Préparer migration additive owner_issuer et champs événements. Reconstituer depuis SoT les principals plateforme/org ; traiter `iamrusty` uniquement avec provenance plateforme. Les doublons de même sub/iss distincts restent distincts. Une donnée ambiguë est isolée pour résolution, jamais promue en grant.
3. Recalculer les tuples d’organisation depuis propriétaires/membres/rôles attribués ; supprimer les anciennes relations aplaties dans le périmètre réconcilié puis écrire les tuples qualifiés/plateforme prouvés. Ne pas supposer que tous les user:UUID existants sont légitimes plateforme : ils peuvent provenir d’un événement org ancien. Préserver les autres relations uniquement quand provenance plateforme confirmée ; rapporter les inconnues, pas les recopier aveuglément.
4. Rejouer seulement les événements résolus, avec la même politique. Supprimer les alias de lookup une fois les lignes historiques réconciliées. Aucune réécriture des événements déjà publiés sans trace de conversion.
5. Déployer lecteurs/writers alignés, purger les caches d’allow de la version antérieure, vérifier positifs plateforme/org et refus croisés puis réouvrir. Si les données ne sont pas toutes attribuables, rester fermé sur les ressources affectées ; pas de PASS global ni de perte de droits masquée.

Rollback : restauration coordonnée DB/tuples/events/caches uniquement sous fenêtre fermée et inventaire contrôlé ; ne jamais réexposer l’ancien reader multi-issuer vulnérable. Le contrat n’autorise aucune commande destructive/runtime : le lead attribue l’exécution selon les garde-fous de la mission.

## 7. Compatibilité runtime et allowlist du consommateur

Une même politique par trust domain de déploiement, injectée via setup, standalone et monolithe. Le monolithe ne recrée pas de checker permissif ni de constructeur spécial. Le chemin mesh ne fait confiance qu’au principal du transport attesté de 0308 ; headers seuls refusés. Pas de changement du protocole ext-authz nécessaire.

Allowlist fermée par frontières (les motifs bornent exactement les sous-arbres admissibles ; aucun autre service par effet de compilation) :
- SDK isolé : `rustycog/rustycog-permission/src/{lib,checker}.rs`, `rustycog/rustycog-http/src/{middleware_auth,middleware_permission,extractors,jwt_handler,builder,lib}.rs`, `rustycog/rustycog-config/src/**` pour les seuls champs de PrincipalPolicy, réexports `rustycog/src/lib.rs` si nécessaires, tests `rustycog/tests/{permission_middleware,mesh_gateway_auth}.rs` et unit tests des modules touchés ; fixtures sous `rustycog/rustycog-testing/src/{http,common}/**` pour l’identité/Subject seulement. Vérifier les chemins réels du SDK avant édition ; si un module nommé est ailleurs, rapporter le déplacement au lead, pas de scan/rename global.
- Hive : `services/Hive/{domain,application,http,infra,setup}/src/**` seulement principal/owner/member/guards/DTO/contexte, `services/Hive/migration/src/**` pour owner_issuer et compatibilité attestée, `services/Hive/config/*.toml`, `services/Hive/configuration/src/**` pour policy ; tests `services/Hive/tests/{organization_signer_permissions,members_api_tests,member_update_api_tests,organization_api_tests,common}.rs` et tests unitaires member_issuer.
- Contrats/projection : `crates/hive-events/src/{organization,member}.rs`, `workers/sentinel-sync/src/{config,fga_client,translator/hive}.rs`, sa configuration locale et tests existants de ces modules ; migration de réconciliation en outil explicitement attribué par le lead, pas de script runtime implicite.
- Consommateurs connus : `services/Manifesto/application/src/usecase/{project,world_read}.rs`, `services/{Manifesto,Telegraph}/{http,setup,configuration}/src/**` seulement gardes/vues principal et policy ; leurs config TOML et tests d’identité/public-read touchés ; `runtime/monolith/src/**` seulement propagation policy. Pour un consommateur supplémentaire (ex. Lazaret), fournir référence et demander attribution au lead avant édition, sans élargir la décision.
- Dépendances/lock des crates énumérées : uniquement codec déjà disponible ou dépendance minimale justifiée ; aucun bump général. `ops/openfga/model.fga` **inchangé** ; pas de nouveau type/relations. Gitlink SDK, index ADR, handbook/ledger et fichiers de configuration déploiement partagés détenus/intégrés par le lead, pas modifiés concurremment.

Hors scope : remote signer, Envoy TLS/listeners, OAuth/connecteurs, cloud/HSM, UX linking/switch, migration générale des schémas Manifesto/Telegraph/Lazaret, nouvelle infra de tests et publication distante par ce lot.

## 8. Validation nommée — à inscrire au plan QA avant exécution

**Lot documentaire actuel : aucun test comportemental ni runtime exécuté.** Le hook pré-commit a involontairement invoqué `cargo fmt --all -- --check` pendant la tentative de livraison : FAIL environnement Windows OS206 (« Nom de fichier ou extension trop long »), sans commit créé ni preuve rouge fonctionnelle. Aucun autre Cargo exécuté et aucun fmt PASS revendiqué. Le lead autorise pour le seul commit des trois documents une omission du hook par configuration Git éphémère, sans modifier sa configuration persistante ; la QA Rust finale n’est pas dispensée. Les noms ci-dessous sont des régressions à implémenter, pas des tests existants déclarés verts.

- `recognized_same_sub_distinct_issuers_do_not_share_organization_admin` dans organization_signer_permissions : deux vrais JWT RS256, même sub, issuers plateforme et org distincts, kid/clés et métadonnées de confiance reconnus par le vrai vérificateur. Préflight positif de reconnaissance de **chaque** JWT jusqu’au principal/Check ; une 401 du second invalide l’oracle de collision. Réutiliser le vrai harness Hive/Postgres/OpenFGA ; mock du RPC IAM sortant seulement.
- Grant Admin uniquement au principal plateforme sur A : plateforme configure A = 200 et exactement le RPC attendu ; org au même sub = 403, compteur RPC inchangé, snapshots writer Hive/outbox/IAM mock sans effet. Body contenant B ne change pas la ressource autorisée.
- Ajouter le grant du **principal org** sur A : org = 200 ; sans ce positif une interdiction globale pourrait faire passer le test. Révoquer uniquement org : org redevient 403, plateforme reste autorisée. Répéter avec deux issuers org reconnus de même sub pour exclure une solution « plateforme vs tous les autres fusionnés ».
- `principal_cache_keys_include_issuer`, `uuid_extension_alone_is_not_a_permission_principal`, `optional_principal_denial_is_not_wildcard`, absence de ressource/checker en erreur/refus = 403, absence de principal requis = 401. Positifs wildcard/public-read inchangés ; garde mesh avec principal attesté vs headers forgés.
- `hive_membership_and_projection_preserve_target_issuer` : deux lignes `(A,iss1,sub)/(A,iss2,sub)`, changement/suppression de l’une laisse l’autre ; inclure acteur iss1 et cible iss2, owner created/deleted, all-relations delete, event sans issuer mis en quarantaine, issuer arbitraire finissant `/iam` jamais alias.
- `legacy_platform_grants_require_provenance` : droits historiques plateforme prouvés préservés, ancien grant org aplati jamais promu, absence de fallback p1→UUID, migration/replay idempotents et reprise fermée sur état ambigu.
- `uuid_only_consumer_rejects_foreign_authority_before_lookup` et positifs de lecture publique sur Manifesto/Telegraph selon diff. Témoins standalone et routeur monolithe avec la même policy ; compteurs de lookup/effets, pas seulement codes HTTP.

Suites de référence ciblées après accord du lead et arbre gelé : `organization_signer_permissions`, `organization_account_guard`, SDK `--features http --test permission_middleware --test mesh_gateway_auth` et `--features http --lib http::`. Ajouter les commandes de tests permission/projections/migration/public-read réellement modifiés ; ne pas relancer toutes les suites IAM inchangées par principe. Format/check/clippy au slot unique natif Windows (24 jobs configurés) selon le diff. IT via fixtures existantes, pas Kind. Revue sécurité indépendante du diff intégré et revue bornée des oracles adversariaux avant clôture ; aucune qualification production implicite.

## 9. Sources et limites

Sources détaillées dans ADR-0314 ; corps with_principal/permission_middleware/optional_permission_middleware/Subject, refs Serena, HIR Subject (6 modules agrégés : SDK checker/permission/http/testing et Manifesto project/world_read), projections/événements et test signer lus. Les refs Serena incluent de nombreux tests absents du résumé HIR ; aucune liste n’est une preuve exhaustive. Index et Serena rattachés à la racine canonique, worktree documentaire non indexé ; sources à la base B vérifiée. QMD/HIR indisponibles en phase planning, disponibles après transition et effectivement utilisés sans réindexation. Pas de conclusion d’absence fondée sur zéro graphe.

Contradictions explicites : 0305 Implemented ne couvre pas son schéma FGA Non décidé ; handbook authz réduit encore l’identité à sub ; alias Hive par forme d’URL et test translator actuel sont incompatibles avec cette cible. Le skill permission décrit encore un mock FGA en IT : 0200/0302 et le harness réel priment. Ne pas modifier les Accepted pour faire passer la proposition pour déjà livrée.

## Annexe — intégration lead

Consommateur documentaire : **corriger-identites**, non publié par cet auteur. Entrées nécessaires : texte ADR/contrat explicitement ratifié, HEAD documentaire intégré, SDK exact et ownership des consommateurs confirmé. Ligne d’index à intégrer par le lead : `0314 | Conserver l’autorité du principal dans les sujets et projections OpenFGA | Accepted / Unimplemented | contrat authz-trust-domain-boundary`.

### Attributions ciblées du lead après inventaire du consommateur — 2026-10-28

Ces précisions d'ownership n'ajoutent aucune frontière métier à la décision ratifiée :
- Les targets SDK `permission_middleware` et `mesh_gateway_auth` sont déclarées dans Cargo.toml mais leurs fichiers réels sont `rustycog-http/tests/permission_middleware_tests.rs` et `rustycog-http/tests/mesh_gateway_auth.rs`. Ils sont attribués sans renommage ; `AuthUser`/`OptionalAuthUser` résident dans middleware_auth.rs, pas dans le module JSON extractors.
- `workers/sentinel-sync/src/main.rs` est attribué uniquement pour injecter la PrincipalPolicy explicite dans HiveTranslator ; pas de refonte du worker ni du protocole de queue.
- `services/{GitHubConnect,GitLabConnect,Lazaret}/setup/src/app.rs`, leurs champs de configuration strictement nécessaires et TOML de ces services sont attribués uniquement pour le plumbing de PrincipalPolicy/constructeurs SDK. Ne déduire aucune plateforme du Host, du secret HMAC, du JWKS ou d'un simple issuer HS256. Lazaret HTTP est workload-only selon l'inventaire : aucune nouvelle garde HumanAccount, route, permission, session workload ou migration à y inventer.
- `services/IAMRusty/configuration/src/lib.rs::http_verifier_auth` et l'éventuel wiring de policy IAM restent brokerisés par le lead après intégration du lot remote et disponibilité du SDK qualifié. Le consommateur identités n'édite pas ce fichier en concurrence ; sa compilation/qualification globale reste une vérification intégrée ultérieure, pas une raison de choisir une policy permissive.
- Les helpers SDK legacy de vérification pure peuvent conserver leur API sans policy. Ils ne constituent pas une voie HTTP UUID-only autorisée : sans injection explicite, refus local, sans déduction de plateforme. La validation de configuration des composition roots demeure fail-closed.

Oracle complémentaire inscrit avant exécution : `qualified_principal_encoding_preserves_utf8_and_size_bound` (1/256 octets valides, 257 refusé, UTF-8 multioctets exact, pas de troncature). Vérification de source lead : sujet complet maximal 387 octets, sous TupleKey.user max512 de l'API épinglée par la fixture OpenFGA v1.5.0 ; aucun test serveur réalisé par cette vérification.

Attribution complémentaire après inventaire du WIP applicatif : `services/Hive/application/tests/command_coverage.rs` et `services/Hive/tests/sqs_event_routing_tests.rs`, uniquement pour adapter les constructions DTO/events et leurs assertions d'issuer au contrat. Ces fichiers ne donnent pas autorisation de modifier un protocole de queue, d'activer une queue opt-in ou de réécrire le harness. Les tests d'intégration réels demeurent soumis au préflight et au créneau Cargo convenus.

### Sous-lot de raccordement IAM, après intégration remote

Le correctif remote est désormais intégré (`6b6d364`) et son auteur arrêté.
Le lead peut attribuer le raccordement IAM à un implementer isolé, sans édition
concurrente du lot identités. Entrées : SDK publié `72845d9` et ce contrat ratifié.
Périmètre : configuration/lib.rs et setup/app.rs, uniquement transport des champs
PrincipalPolicy explicites depuis `[auth.jwt]` et validation avant l'I/O de
signature ; tests de configuration/setup affectés ; cinq TOML IAM existants et
le seul `ops/deploy/apps/overlays/local-full/config/iam.toml` pour déclarer la
policy de leur autorité effective. Les sources OAuth/remote, les profils du
navigateur en cours d'arbitrage et le gitlink restent hors édition de ce sous-lot.

Préserver les helpers de vérification pure et les règles crypto déjà testées ;
ne pas inventer une plateforme depuis HS issuer, Host, JWKS ou un secret. Les
deux champs explicites requis par le SDK doivent être réellement consommés,
absence/invalidité refusées au composition root. La compatibilité historique
ne devient jamais true implicitement, y compris dans les fixtures. Les valeurs
des TOML doivent être cohérentes avec l'autorité effectivement signée, sans
changer public_base_url ni le schéma produit pour faire passer les tests.

### Clés distinctes du test de collision

Attribution du lead : `services/Hive/Cargo.toml`, uniquement dev-dependencies
`jsonwebtoken.workspace` et `rsa.workspace` déjà résolues. Elles servent à
générer trois clés de test réellement indépendantes, pas trois kids sur la même
clé. Cargo.lock : au plus les deux références correspondantes du package
hive-service, sans nouvelle version/checksum/provider. Le lock remote déjà
intégré doit être préservé à la réunion des branches ; aucun update général.
Cette attribution ne permet ni édition implicite du SDK publié, ni lancement
d'IT, de queue ou de runtime avant préflight et créneau dédiés.

Attribution complémentaire : `services/Hive/tests/fixtures/db.rs`, uniquement
le constructeur d'organisations de test et ses champs d'autorité. Pour ses
nouvelles organisations plateforme, la provenance explicite peut être
`Some(TEST_JWT_ISSUER)` : ce même fichier importe déjà cette autorité de fixture
et l'utilise pour ses nouveaux membres. Ne pas transformer ce défaut de fixture
en backfill de lignes historiques ou en attestation pour une entrée externe.
Les scénarios d'autorité étrangère ou de provenance absente doivent pouvoir
déclarer leur propre issuer ou None explicitement, sans être masqués par ce
témoin positif. Aucun changement du cycle de vie DB/testcontainers ou des queues.

### Compatibilité des fixtures SDK RS256 sous feature testing

Attribution au même auteur SDK : `rustycog-http/src/jwt_rs256_tests.rs`, seulement
fixtures/factories de test, pour fournir les deux champs policy explicites aux
constructeurs déjà utilisés. Les assertions crypto/issuer/seed/algorithmes ne
sont ni retirées ni affaiblies. Default historique false ; true uniquement dans
le test nommé de fenêtre historique iamrusty, jamais comme défaut global.
Les helpers purs restent purs : employer les constructeurs configurés existants
quand le test exige aussi une vue UUID plateforme. Aucun code de production SDK,
manifeste, lock, nouveau helper public ou harness autorisé par cette attribution.

Le commit72845d9 publié reste immuable. Cette adaptation, si nécessaire après
preuve du gap, sera un nouveau commit test-only soumis à vérification et décision
de publication du lead, puis sélection exacte du gitlink. Aucun nouveau HEAD
non publié ne peut remplacer le pin déjà autorisé.

Précondition de compilation de cette suite : attribution additionnelle du seul
import `parse_jwks_document` dans `rustycog-http/src/jwks_seed_tests.rs` (bloc
`use super`). La baseline compile aussi ce module malgré son filtre RSA et
échoue E0425 avant tout test. Aucun autre corps, assertion ou contrat JWKS
n'est changé par cette correction mécanique de test.

### Application de la frontière UUID-only à l'auto-invitation interne

Le chemin `SyncServiceImpl::process_external_member_for_sync` appelle
`InvitationService::create_invitation_by_email` avec le seul UUID du propriétaire.
La garde HTTP ne protège pas ce consommateur interne. Appliquer ici la même
PrincipalPolicy déjà ratifiée et injectée depuis Hive setup : lorsque
`auto_invite` est activé, refuser un propriétaire absent/incompatible avant
tout appel fournisseur et toute mutation d'exécution. Conserver
l'autorité exacte et les erreurs opaques ; pas d'inférence depuis le demandeur
ou le Host. La synchronisation sans auto-invitation demeure admissible : aucun
bannissement global des organisations étrangères. Pas de changement du protocole
InvitationService, schéma, cache ou stockage. Il s'agit de fermer un appelant
interne de la frontière existante, pas de créer un nouveau mécanisme d'autorisation.

Ownership du même auteur : garde privée SyncService, wiring setup, constructions
et tests existants affectés. Témoins avant exécution : propriétaire plateforme
valide accepté, étranger/None refusés sans mutation d'exécution ni fournisseur, auto-invite=false
préservé, job historique relu et refusé avant appel si son propriétaire est devenu
incompatible. Nommer les sélecteurs avant run ; aucun runtime/IT implicite.

Précision après lecture des signatures : prepare/start_sync_job ne reçoit pas
auto_invite, et aucune intention d'inviter n'est persistée. La création de ce
job non invitant reste admise ; ne pas ajouter une clé JSON, un paramètre public
ou un refus global pour satisfaire une précondition inexistante. Le contrôle
porte sur `sync_members(job_id, auto_invite)` avec son booléen réel, avant
fournisseur, transition d'état, audit d'exécution et invocation d'invitation.
Un job préexistant est donc lui aussi recontrôlé lors de son exécution invitante.

### Propagation du refus de projection jusqu'au ledger existant

Constat source : `workers/sentinel-sync/src/handler.rs::translate` avale les
erreurs des traducteurs puis `handle_event` peut compléter un événement Hive
comme non pertinent. Cela contredit le refus/quarantaine visible déjà exigé.
Attribution au même auteur : ce handler et ses tests affectés, uniquement pour
propager une erreur d'événement revendiqué vers `fail_delivery` et l'erreur de
service existantes, sans tuple write ni complete/ack réussi. `Ok(None)` d'un
événement réellement inconnu garde sa sémantique actuelle ; le trait Translator
réserve Err à un échec après revendication. Aucun protocole de queue, nouvelle
DLQ ou stockage ajouté. Codes de refus opaques/stables, pas de payload, credential
ou issuer contrôlé reflété dans les logs/erreurs persistées.

Témoins nommés avant run : propriétaire non résolu et suppression legacy sans
principaux donnent ledger failed, zéro write/complete ; événement qualifié donne
write/completed, rejeu idempotent ; événement inconnu et traducteurs existants
préservés. Preuve ROUGE sur le handler existant si faisable avant correction.
