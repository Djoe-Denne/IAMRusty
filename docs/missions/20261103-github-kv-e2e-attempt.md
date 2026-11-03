# GitHub local — import KV v2 et préflight E2E du 3 novembre 2026

## Résultat borné

- **Seed OpenBao : terminé.** Le Job existant de provisionnement non-root a
  traité les deux documents `secret/github-connect/oauth` (`client_secret`) et
  `secret/github-connect/iam` (`hmac_secret`), avec le contrôle CAS0 existant.
- **E2E métier : NOT RUN.** Les gardes avant signup refusent encore le profil
  effectif : IAM ne connaît que GitLab ; l'inverse des notifications/deliveries
  Telegraph et la frontière de fin de consommation ne sont pas qualifiés.
- Inventaire mesh conservé : **39 attendus, 0 exécuté, 0 PASS, 0 FAIL,
  39 BLOCKED au préflight**. Ce ne sont pas 39 requêtes métier tentées.
  Les **6 obligations Apparatus restent INCOMPLETE / NOT RUN**.

Le seed n'a pas ajouté GitHub au registry IAM, déployé GitHubConnect, lancé un
login fournisseur ou modifié l'adaptateur `SIMULATED_PROVIDER`. Il ne suffit pas
à qualifier un parcours OAuth réel ou la consommation des secrets par la CI.

## Cible et préconditions mesurées

Source `master@572c1a6`, SDK `93b890f`. Cluster existant
`kind-aiforall-local-full`, bail original ancré ; aucun bootstrap, nouveau
cluster, build/load ou redéploiement général. Docker Desktop et les trois nodes
full étaient déjà actifs ; les 15 conteneurs Docker préexistants ne sont pas
adoptés pour un nettoyage. Le node legacy et le registre arrêté restent hors
des mutations de cette campagne.

Avant import, l'ancre a vérifié le contexte, les IDs, l'endpoint et l'UID
Kubernetes. Le Pod OpenBao, sa chaîne de propriétaire et son incarnation ont
été revalidés. Les scripts montés `run-openbao.sh`, `seed-openbao.sh` et
`openbao-workload.py` étaient identiques aux sources revues ; le marqueur de
bootstrap est présent. Ce wrapper ne crée ce marqueur qu'après révocation du
root de bootstrap et refus403 de son lookup. Aucun root ou bearer n'a été lu.

Ces observations ne remplacent pas une campagne exhaustive de refus TLS,
permissions ou endurance. La source privée des credentials, hors Git et sans
`.env`, a été validée par le parser ops avant transmission.

## Mutation minimale effectuée

Le profil GitHub complet n'a pas été rejoué sur le GitLab courant : son entrée
`configure(github=True)` attend le registre de base vide. Seuls les objets
nécessaires au seed autonome ont été créés et leurs UID ACKés dans le bail :

- compte de service **`github-seed`**, sans automount implicite ;
- **`allow-github-seed-to-openbao`** et **`allow-openbao-from-github-seed`** :
  peers par labels dans le même namespace, TCP8443 uniquement ; aucune policy
  préexistante patchée, pas d'egress fournisseur/public ajouté ;
- Job **`github-openbao-seed-ghkv-f7253d3aa09945af`**, généré par
  `github_seed_job`, image Python épinglée, identité projetée/audience du rôle
  de seed, TLS/CA explicites, deadline240s et TTL300 ;
- Secret d'inputs neuf, monté seulement par ce Job, avec ownerReference sur
  l'UID ACKé du Job. Aucun rendu de sa valeur sur disque ou dans les logs.

Le transfert par Secret Kubernetes peut matérialiser une copie dans etcd ; ce
n'est pas une garantie zéro disque ou KMS/HSM. La source opérateur reste un
fichier privé protégé par ACL. OpenBao DEV reste volontairement volatile.

Le helper existant utilise l'authentification Kubernetes `github-seed`, contrôle
le credential non-root/TTL, valide les deux documents avant écriture, refuse
les valeurs conflictuelles/supprimées et écrit CAS0 seulement si absents.
La condition JobComplete et `succeeded=1` ont été observées pour le même UID.
OpenBao est resté sur le même Pod et conteneur après import. Aucun changement
d'application, de donnée métier, de certificat ou d'instance OpenBao.

Un succès du Job ne prouve pas à lui seul les versions KV inchangées sur une
seconde invocation, les scénarios négatifs du coffre ou la durabilité cloud.

## Pourquoi les parcours ne sont pas lancés

Après le seed, les gardes existantes ont été appelées sans fabriquer de faux
inputs E2E ou de credentials de session :

1. `Services.iam_providers` a relu la configuration effectivement montée dans
   le Pod IAM et sa provenance. Résultat : **`providers=[gitlab]`**, SHA256
   `ba3e0bc3ae84e993426fc037ea34f2b3542849bacc5b41eec11bbe0aa9c6e84e`,
   identités Deployment/ReplicaSet/Pod attestées. Les cas S2S fixes `github`
   exigent son enregistrement avant le lookup du token.
2. `Database.qualify_notifications_cleanup` refuse toujours avant HTTP métier
   ou SQL : pas d'API de suppression ni de corrélation source-event/fence de fin
   des consommateurs qualifiées pour les effets asynchrones Telegraph.

Le driver `e2e` n'a donc pas été invoqué. Aucun signup, compte, session,
provider-token synthétique, organisation, notification ou delivery n'a été
créé par cette tentative. Les 39 états bloqués viennent de l'inventaire existant
`Results`, pas d'une réduction du dénominateur. Le wrapper legacy et M6 Kind
auto-provisionnant n'ont pas été utilisés comme repli.

## Vérification et preuves

Opérations parent : préflight métadonnées, seed via primitives existantes
`runtime.operation_lock`/`runtime.create`, puis inspection des gardes avant
signup. Aucun Cargo ou test unitaire/IT supplémentaire : les résultats
précédents ne sont pas recomptés en E2E.

Preuves owner-only hors Git :
`$HOME/.aiforall-local-validations/20261103-github-kv-d28b3e9907f8/` :
`docker-before.json`, `metadata-before.json`, `seed-proof.json` et
`e2e-prerequisites.json`. Aucun contenu, token ou hash de valeur KV dans ces
rapports. Continuité détaillée sous `.cursor/handoffs/20261103-github-kv-e2e-attempt.md`.

### Vérification indépendante et nettoyage constaté

Le vérificateur infra rend **PASS borné** : autorité de cible, UID et spécifications
des trois ajouts persistants conformes, même incarnation OpenBao, scripts et
marqueur conformes, pas de changement des inventaires préexistants observés.
Les comptes de service passent de 4 à 5 et les NetworkPolicies de 25 à 27 ; les
autres mappings nom→UID et nombres restent inchangés : 18 Deployments, 2 Jobs,
19 ConfigMaps, 17 Services. Les replicas prêts observés sont inchangés.
Les 15 conteneurs Docker gardent leurs IDs et états initiaux.

Le Job temporaire est **absent** après TTL, avec **zéro Pod `github-seed`**.
La suppression du Secret d'inputs n'a **pas été observée directement** : elle
reste déduite de l'ownerReference vers le Job disparu ; aucun Secret GET n'est
utilisé pour transformer cette limite en preuve. L'image du Job n'est plus
observable après GC : son pin est vérifié à la source et le succès du Job reste
la preuve historique du parent. Le marqueur observé est `bootstrap-ready`,
pas un hypothétique fichier `bootstrap-revoked`.

Les comptes de service et deux policies de seed restent volontairement dans le
bail comme setup borné. Le seed autonome est une opération parent ponctuelle,
pas une nouvelle entrée CLI idempotente ni une réconciliation du catalogue de
déploiement : un prochain déploiement/contrôle de catalogue doit tenir compte
de ces ajouts, sans les réadopter ou rejouer aveuglément leur création.

Aucun conteneur Docker, node ou registre n'a été créé, repris ou arrêté par ce
lot ; aucun teardown global n'est nécessaire. Les nœuds préexistants restent
dans leur état initial. Il n'y a pas d'affirmation de nettoyage global des
ressources d'autres missions ni de disparition du Secret directement prouvée.

Preuve du spécialiste :
`$HOME/.aiforall-local-validations/20261103-github-kv-infraverify-93f18d87f6dc/infra-verify-proof.json`.
Sa première tentative de vérification, en échec sur une supposition du script
vérificateur concernant les initContainers, est conservée séparément ; la
correction et la seconde vérification étaient en lecture seule.

Ce PASS infra ne qualifie ni login GitHub, ni E2E métier, ni négatifs de
permissions/réseau/versions KV, ni hardening global ou durabilité cloud.

## Suite nécessaire

- Raccorder GitHubConnect et enregistrer GitHub **additivement** dans IAM en
  conservant GitLab et les autres paramètres ; configurer le même HMAC depuis
  le coffre, sans nouvelle permission fournisseur anticipée.
- Qualifier l'inverse exact et la fin des traitements Telegraph **avant signup**.
- Raccorder séparément le vrai OAuth navigateur : l'adaptateur mesh courant
  exige encore `SIMULATED_PROVIDER` ; un credential valide n'en change pas le mode.
- Conserver les 39 oracles mesh et les 6 obligations Apparatus, puis exécuter les
  parcours réels et leurs inverses. Aucun de ces points n'est fermé par le seed.

Références : [contrat de raccord OpenBao](../contracts/openbao-local-githubconnect-deployment.md)
(Proposed / Partial), [raccord E2E existant](../contracts/local-full-existing-e2e-wiring.md),
[scope login et future délégation](../guides/github-sign-in-and-delegation.md).
