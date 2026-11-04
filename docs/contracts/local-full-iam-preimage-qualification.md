# Local-full — qualification IAM initiale et après snapshot

2026-11-04 — **Proposed / Unimplemented** pour ce complément de qualification.
Base inspectée : `1e89ff58143aaa59342d266d2611fd70036f43f4`.
Jalon : raccord Ops du local-full existant, hors Apparatus P0–P6.
Ce fichier propose un delta aux §2–3 du [contrat additif A1](local-full-github-additive-activation.md),
pas une permission, une activation, une ADR Accepted ou une preuve LIVE nouvelle.
Intégration et arbitrage du parent requis avant son consommateur.

**Accord utilisateur local reçu le 2026-11-04 :** « Autoriser la lecture ciblée
(Recommended) ». Il autorise la préparation de l'exception nommée décrite au §3
pour les deux seules configurations IAM du même bail, y compris le transit
possible de leur contenu complet dans la mémoire privée de kubectl avant
projection. Aucune extraction/décodage de `.data`, sortie ou capture brute n'est
autorisée. La méthode fermée de `local_full_target.py` et ses tests indépendants
doivent être admis par une révision native de politique avant code ; aucune
permission générale, nouvelle lecture effective ou implémentation n'est acquise
par le document seul. Cette autorisation locale prime sur les formulations
pré-arbitrage du §3, sans modifier le Statut d'une ADR ou le refus générique.

## 1. Sources, acquis et contradictions bornées

- [0409](../adr/0409-confiance-callback-oauth-idp-connect.md) et
  [0411](../adr/0411-idp-provider-slug-registry-fail-closed.md) sont
  **Accepted / Implemented** : HMAC et redirects conservés, catalogue au boot,
  restart IAM requis ; aucun changement applicatif proposé ici.
- [0505](../adr/0505-startup-secrets-sdk-openbao.md) est **Proposed / Implemented**,
  [0606](../adr/0606-local-full-kind-isole.md) **Proposed / Partial**. Leur code
  ou l'autorisation du lab ne ratifient pas une lecture de Secret.
- [A1 §2–3](local-full-github-additive-activation.md#2-source-unique-du-registre-combiné)
  exige source privée, montage et UID courant, mais interdit Secret GET.
  La règle [infra-safety §2](../../.cursor/rules/infra-safety.mdc) exige une
  demande utilisateur explicite pour lire des Secrets.
- [Services.iam_providers](../../ops/tests/local_full_e2e.py) (293–354) vérifie
  Pod→ReplicaSet→Deployment, startup/montage et projection fixe
  `IAM_PROVIDER_SCRIPT` hash/projection/hash. Il n'accepte que Secret/ConfigMap,
  **pas** `emptyDir`. Son retour comprend `config_sha256` ; `run_action`
  (1048–1062) le sérialise actuellement dans `iam-provider-registry` : ne pas
  recopier ce retour brut dans les nouveaux reçus/rapports de qualification.
- [snapshot Ops](../../ops/deploy/local_full_openbao.py) (32–53) prépare
  `openbao-snapshot`, son image `PYTHON` épinglée, `/template`, `/runtime` et
  `private-config` Memory. Le code initial sélectionne le volume nommé `config` ;
  ce nom ne prouve pas qu'il s'agit du fichier OAuth effectivement sélectionné.
  Le [montage OAuth](../../ops/deploy/apps/overlays/local-full/oauth-proof/kustomization.yaml)
  nomme celui-ci `oauth-proof-config` et ajoute l'init de trust GitLab.
- [helper snapshot](../../ops/deploy/apps/overlays/local-full/helpers/openbao-workload.py)
  `snapshot` (91–113) : rôle `iam-hmac`, un seul marker GitHub, lecture KV HMAC,
  validation hex64/TOML puis publication du fichier privé. C'est la chaîne à
  qualifier, pas une autorisation générale des fichiers écrits par un init.
- [ancre](../../ops/deploy/local_full_target.py) : `parse_kubectl_operation`
  (112–218) refuse `get` sur `se/secret/secrets`, **avant** toute sonde ;
  `LocalClusterTargetAnchor.kubectl` (1137–1141) impose ce parseur et l'UID cible.
  `run` et [capture_failure](../../ops/deploy/private_diagnostics.py) conservent
  potentiellement des sorties d'erreur en fichiers privés : privé ne signifie
  pas autorisé à recevoir OAuth/HMAC ou leur empreinte.

Faits repris du ledger parent, sans accès privé/runtime par l'auteur :
`C:/Users/djden/source/repos/AIForAll/.cursor/handoffs/20261103-github-environment-e2e-launch.md`,
sections finales « Source IAM : égalité effective observée sans export » et
« Frontière de qualification IAM remontée par la production ». Les valeurs OAuth
originales ont été validées par `deploy.oauth_inputs` ; le template IAM inchangé
depuis le 29 octobre reconstruit en mémoire la configuration effective GitLab
seule, concordante avec la projection fixe existante. Deployment et référence du
Secret concordent au bail. **L'UID actuel du Secret reste inconnu** : seul son
ACK historique est acquis. Cette égalité ne comble pas le manque d'identité.
Les modifications non intégrées de pool-5/pool-6 ne sont pas des sources canoniques.

## 2. Préimage initiale : reconstruction privée, pas régénération

Proposition : qualifier soit le fichier IAM privé original complet, soit sa
**reconstruction déterministe en mémoire** depuis le template d'origine identifié
et les valeurs OAuth originales du même bail. Un template à markers seul reste
`SOURCE_IAM_UNAVAILABLE`. Ne jamais remplacer une valeur absente par une valeur
neuve, un défaut, la source GitHub ou un choix opérateur.

Préconditions communes, sous le verrou d'opération existant :

1. Ancre locale complète et bail d'origine valides ; namespace, Deployment,
   run/HEAD et epoch liés. Vérifier la sélection effective unique du fichier
   `/app/config/development.toml`, sa référence Secret et sa chaîne propriétaire.
2. Sources privées originales disponibles, schémas fermés et ACL/reparse/tailles
   validés par les gardes existantes, identité de run concordante. Références
   stables avant/après lecture et avant usage ; ambiguïté ou changement => refus.
   Template et substitutions revus doivent reproduire le rendu original : la
   version actuelle du renderer ne suffit pas par son seul nom.
3. Parser le document complet et vérifier GitLab seul, absence de markers,
   registry non ambigu et tous les paramètres A1 conservés. La preuve trusted
   compare l'empreinte du rendu attendu en mémoire à celle du script fixe,
   stable avant/après projection, dans le même Pod qualifié. Le script existant
   transmet des hashes, pas les octets du fichier : ne pas le décrire comme un
   export ou une comparaison directe de deux fichiers lus sur l'hôte.
4. Établir séparément l'UID courant du Secret selon §3 ; ACK historique, nom,
   référence de volume et égalité de configuration ne le remplacent jamais.

Le document reconstruit devient la préimage complète privée utilisée par la
composition A1, sans devoir créer un nouveau fichier pour la seule preuve.
Même HMAC GitLab, OAuthState, redirects callback/relink et autres paramètres ;
aucun rerender de déploiement global. Comparaisons et empreintes privées restent
en mémoire, y compris en échec ; seuls résultat qualifié et identités non secrètes
peuvent rejoindre les états existants. Aucune nouvelle base de preuves.

## 3. UID courant : permission manquante, recommandation conditionnelle

**Les droits et coutures actuels ne suffisent pas.** Le Pod ne référence que le
nom du Secret, et le contrôle A1 demande son UID actuel. Aucun bool opérateur,
attestation sans mesure, ACK ancien, `describe`, liste ou GET indirect ne ferme
ce gap. Ne pas contourner l'ancre en appelant son runner/prefix directement.

**Recommandation minimale, à soumettre par le parent à l'utilisateur :** exception
locale explicite de lecture **GET nommée**, finalité UID/RV seulement, limitée à
`kind-aiforall-local-full`, namespace `aiforall-local-full`, ancre et bail originaux,
pour le Secret effectivement sélectionné et déjà ACKé par ce bail. Le nom suffixé
est résolu depuis le montage et confronté à l'ACK ; pas reconstruit par préfixe,
fourni librement ou découvert par liste. Pour la postimage, même exception
uniquement pour le template privé créé/ACKé par l'activation (§4).

- Champs utiles : `metadata.name`, `namespace`, `uid`, `resourceVersion` et
  absence de `deletionTimestamp`, contrôlés avant/après la qualification. UID
  égal à l'ACK requis ; RV stable sur la fenêtre, pas forcément égal au RV ancien.
  Objet absent, remplacé ou modifié pendant le contrôle => refus, aucune adoption.
- Une sortie kubectl JSONPath fixe ne constitue **pas** une garantie de transport
  metadata-only : la lecture nommée ordinaire peut faire transiter le contenu du
  Secret dans la mémoire privée du client kubectl avant projection. L'accord
  demandé doit exposer cette limite ; aucune extraction/décodage de `.data`,
  sortie JSON/YAML/base64, fichier temporaire ou dump de réponse n'est nécessaire.
  Ce contrat n'autorise ni ce GET ni le transit de contenu à ce jour.
- Une vraie réponse metadata-only exige une négociation HTTP
  [`PartialObjectMetadata`](https://kubernetes.io/docs/reference/using-api/api-concepts/#metadata-only-fetches)
  (documentation officielle lue le 2026-11-04). Aucun chemin de cette nature n'a été
  démontré dans l'ancre actuelle ; ne pas prétendre qu'il est livré, ni ajouter
  proxy, extraction de credential, client global ou fallback de transport ici.
- L'éventuelle exception doit être une couture locale fermée de l'ancre, sous
  l'autorité de l'opération, jamais une option CLI `allow-secret`, ni suppression
  générale du refus. Elle nécessite **admission explicite de son fichier et de
  ses tests par le parent**, hors ownership production actuellement publié.
  L'autorisation utilisateur et cette admission sont deux prérequis distincts.
  Le RBAC effectif n'a pas été sondé ici : un accord humain n'atteste pas que
  l'API acceptera la lecture, et un refus API ne justifie aucun droit supplémentaire.
- Capturer/projeter uniquement les métadonnées fixes ; diagnostics constants sur
  erreur/timeout/décodage. Interdire la capture brute de ces lectures et des
  comparaisons privées **avant** `capture_failure`, y compris en traces/exception.
  Pas d'OAuth/HMAC, hash privé ou corps Secret dans un artefact, même privé.
  Ne pas modifier globalement les outils, profils, hook ou diagnostics.

**Alternative : conserver intégralement l'interdiction.** L'activation reste
`BLOCKED SOURCE_IAM_UNAVAILABLE` avant son premier écrit, même si la configuration
effective concorde. Le présent document ne ratifie aucune exception de règle.
L'accord de rétention E2E ne donne aucun droit de lecture de Secret.

## 4. Post-snapshot : une seule chaîne fermée admissible

Étendre la qualification existante pour **ce** snapshot IAM, pas pour tout
`emptyDir`, et conserver le chemin direct initial avec ses refus. Préflight de
la chaîne attendue avant écrit, puis vérification courante après rollout :

1. Ancre/bail/namespace et Deployment IAM ACKés ; Pod courant unique, non en
   suppression, chaîne contrôleur Pod→RS→Deployment et UIDs vérifiés. Images
   applicative/init et identifiants runtime concordent avec les images immuables
   publiées ; aucune image ou commande arbitraire issue d'un input.
2. Le template combiné est la préimage §2 plus la seule ligne GitHub A1, avec
   marker GitHub unique ; il reste **privé** car il contient le HMAC GitLab.
   Secret template créé exclusivement, ACK UID conservé, référence unique et UID
   courant mesuré selon §3 ; ACK perdu => incertain, pas adoption par GET.
3. Init `openbao-snapshot` unique : image `PYTHON` épinglée, commande exacte
   `python -B /scripts/openbao-workload.py iam-hmac`, script et dépendances du
   helper identiques aux sources revues du HEAD intégré (ConfigMap/UID possédés).
   SA, audience, projection JWT, CA publique, contexte non-root et montages
   `/run/identity`, `/run/openbao-ca`, `/scripts`, `/template` conformes au snapshot
   attendu. Identité, script, CA et template en lecture seule ; init terminé
   avec succès avant démarrage IAM. Nom d'init ou exit0 seuls insuffisants.
4. `/template/development.toml` provient exactement du Secret combiné sélectionné,
   pas du premier volume nommé `config`. `/runtime` est le seul écrivain init du
   volume `private-config`, `emptyDir.medium=Memory`, borne `1Mi`. Aucun autre
   init/sidecar/conteneur éphémère ne peut l'écrire. L'init trust GitLab existant
   reste permis **exactement** sur son propre volume/commande revus, sans accès
   écrivain au snapshot ; ne pas le supprimer pour simplifier la qualification.
5. IAM ne lit que le snapshot : montage unique `/app/config/development.toml`,
   `subPath=development.toml`, readOnly, pas `subPathExpr`, montage masquant ou
   fichier local additionnel. Startup/env sans override config/IdP/envFrom,
   mêmes paramètres autorisés et bundle de confiance GitLab conservés. Aucun
   écrivain supplémentaire par alias du volume ou chemin parent.
6. Même `IAM_PROVIDER_SCRIPT` fixe, hash/projection/hash stable et relecture des
   identités/specs encadrantes ; providers exactement GitHub+GitLab. Composer en
   mémoire le rendu attendu avec le HMAC GitHub de la source privée déjà validée
   par le seed CAS, puis comparer à la projection privée effective. Ainsi GitLab
   et tous ses paramètres/HMAC restent identiques à la préimage ; GitHub n'est
   pas validé par son seul slug. Aucun accès nouveau au KV accordé au qualifieur.
7. Lier snapshot et résultat à l'epoch courante via
   [coffer_epoch](../../ops/deploy/local_full_environment.py), ainsi qu'à
   l'intention/ACK de l'activation ou reprise existante. Epoch changée, init
   antérieur à l'epoch visée, redémarrage/incarnation ou spec changés pendant
   le contrôle => non-ready. Un snapshot ancien encore lisible n'atteste pas
   la reprise du nouveau coffre. Pas de polling/supervision nouvelle ; contrôles
   bornés aux points de préflight, postcondition et usage prévus.

Ces conditions sont cumulatives : montage, provenance, égalité et epoch ne se
substituent pas les uns aux autres. Pas de promotion `github_input=active`, ready,
reçu C/claim ou signup sur preuve manquante. A1 conserve CAS/intention/ACK et ses
refus en reprise ; erreur post-écrit conserve l'état non-ready et les données,
sans rollback aveugle. La vérification infra indépendante reste obligatoire.

## 5. Contrat consommateur et validation bornée

Impact proposé : **cross-module / trust Ops local**. Qualification dans
`Services.iam_providers` et son adaptateur trusted côté activation ; composition,
snapshot et lifecycle aux coutures A1 existantes. Ownership actuel inchangé :
pool-5 possède `local_full_e2e.py`/`local-full-acceptance.py`, pool-6 les sources Ops
snapshot/lifecycle. Le parent intègre ce document avant tout raccord entre leurs
deltas. Aucun symbole nouveau présenté comme déjà disponible ; la couture
d'ancre §3 reste bloquée sans permission et attribution distinctes.

Tests **hermétiques par test-engineer**, sans vrai bail, source privée, API ou KV :

- `test_openbao_local.py` : source originale absente/changée, template ou
  substitutions divergents, markers/doublons, sélection `config` au lieu du
  montage effectif ; préimage complète et HMAC GitLab conservés, zéro écrit si
  qualification initiale échoue. Sentinelles privées absentes de toute sortie.
- `test_local_full_environment.py` : ACK ancien avec UID actuel différent,
  suppression/recréation homonyme, RV/spec concurrente, ACK template perdu,
  CAS perdant, epoch changée/reprise interrompue ; non-ready et zéro promotion,
  aucune adoption ou destruction étrangère. Source privée revalidée à l'usage.
- `test_local_full_e2e.py` : double du transport fixe et de l'ancre ; cas direct
  initial conservé, snapshot exact accepté, puis mutation d'une condition §4 à
  la fois refusée (image/script/rôle/owner/UID/volume/mount/startup/writer/epoch,
  init non terminé, GitLab/HMAC modifié malgré les deux slugs). Hors autorité,
  GET Secret encore refusé avant toute sonde ; aucune liste, cible libre ou
  JSONPath libre introduite. Erreur/timeout/sortie malformée avec sentinelles
  n'atteignent ni `capture_failure` brut, ni reçu/rapport/hash public. Préflight
  manquant => zéro claim/reçu C/signup et conservation des négatifs stricts.
- Driver : `test_local_full_acceptance_source.py` seulement si sa sortie est
  modifiée par le raccord ; tests routes/profils seulement si leur diff le
  justifie. Conserver les 39 obligations mesh, 6 Apparatus et SIMULATED_PROVIDER.

[E2E C §8](local-full-telegraph-e2e-inverse.md#8-contrat-ops-c-ratifié-localement--un-seul-run)
reste inchangé : un seul run, pas de fallback strict→C, pas d'inverse final ni
nouveau créneau, revoke métier préservé et terminal RETAINED non nul. Cette
qualification IAM n'atteste ni inverse Telegraph ni départ E2E.

Aucune migration, nouveau protocole/stockage IAM/Telegraph/Sentinel, nouveau droit
workload/RBAC, infrastructure de preuve ou outil global. Aucun fichier commun,
ADR, index, profil ou ledger modifié par ce livrable ; lien entrant à poser par
le parent lors de l'intégration autorisée. Aucune ADR créée dans ce mandat borné,
aucun statut Accepted changé, donc aucun digest ADR à synchroniser. L'exception
Secret demeure une **proposition à arbitrage humain**, pas une politique générale.

Contrôle documentaire de remise : fichier présent, liens locaux vérifiés et
`git diff --check`. Aucun test Python, Cargo, kubectl, Docker ou runtime requis
ni exécuté pour ce document. Les commandes Python convenues appartiennent aux
futurs diffs Ops admis, pas à cette phase documentaire.
