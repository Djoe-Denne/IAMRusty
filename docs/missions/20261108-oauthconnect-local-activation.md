# Activation locale OAuthConnect et fournisseur simulé

## Trajectoire finale retenue : reconstruction locale bornée

Après clarification, l'utilisateur a préféré une reconstruction rapide du lab à
la poursuite de la convergence en place. [ADR-0608](../adr/0608-recreer-local-full-pour-oauthconnect.md)
et [contrat borné](../contracts/local-full-oauthconnect-recreation.md) consignent
l'autorisation de perte des données du seul `aiforall-local-full`, les exclusions
et les limites production. Les sections suivantes conservent l'historique des
tentatives, pas une exigence de les poursuivre.

Les trois anciens nœuds et leurs trois volumes `/var`, attestés sans autre
consommateur, ont été arrêtés puis retirés par Kind ; leur absence a été vérifiée.
Le nœud legacy et le PostgreSQL opaque ont gardé leurs IDs et états. Ni images,
ni caches, ni registre OCI externe, ni credentials, ni PKI n'ont été supprimés.
Nouveau bail v2/kubeconfig privé et nouvel UID `kube-system` : bootstrap/CNI et
chargement des images existantes réussis, sans recompilation. Installation neuve
`--oauth-connect` en cours, sans mode de préservation des anciennes identités.

Disposition des changements : gardes TLS opt-in, correction du binding, acquittement
des applies par objet et sélection explicite du profil lifecycle conservés. Le
spike de réconciliation réseau, jamais appliqué, a été remplacé par la décision
ADR0608 : sources exactes et empreintes sauvegardées dans l'archive privée,
module/tests/hook CLI retirés de la livraison. La dette reste documentée, pas
déclarée résolue. **67 tests ciblés du contenu conservé passent.**

## Périmètre et décisions

Activation explicitement demandée du profil Ops existant `--oauth-connect` dans
`kind-aiforall-local-full`, pas un déploiement cloud. GitHub et `simulated` sont
deux instances du même OAuthConnect. Les connecteurs historiques ne sont pas
supprimés par cette opération ; aucun E2E GitHub/GitLab externe n'est déduit.

L'utilisateur a autorisé la sauvegarde puis la levée du verrou `acceptance-e2e`
daté du 6 novembre, dont le PID était absent. La préimage a été sauvegardée et
comparée avant retrait du seul marqueur. Il demande de **laisser l'environnement
actif** et de conserver les identités TLS sans rotation automatique.

## Préflight et préparation

- Base `81afdfce1114ed1efb7bc9282aee98f24597be86`, `master` ; SDK
  `93b890ff7b821e9cb2e7d0aeb81957d098778a2f` inchangé.
- Bail v2 d'origine, kubeconfig dédié, endpoint et UID Kubernetes vérifiés. Les
  trois nœuds full étaient déjà actifs ; aucun nœud ou cluster créé/redémarré.
  Le nœud legacy et le PostgreSQL opaque préexistant restent hors nettoyage.
- Préflights stockage, ownership OAuth, namespaces, CRD/RBAC et absence de Job
  actif réussis. Avant activation : OAuthConnect absent ; IAM, Lazaret et
  GitHubConnect avaient chacun zéro replica prêt.
- Identifiants GitHub existants validés sans affichage ni copie. Identifiants
  simulés distincts générés dans un fichier privé propriétaire/read-only.
- Neuf images Linux construites avec l'entrée commune puis chargées dans les
  nœuds par le parcours `--build-manifest`. Aucun changement global de builder,
  suppression de cache ou utilisation du target Windows dans Docker.
- Dix IDs d'images (artefacts + neuf runtimes) enregistrés à la création dans
  `.tmp/docker-resources-20261108.log`, owner
  `ops-oauthconnect-activation-20261108`, rétention `no`.

## Préservation TLS

L'ancien chemin PKI était incomplet ; le bundle complet enregistré pour le
registre correspond à la CA publique OpenBao live, mais pas à la feuille Lazaret
conservée historiquement. La première tentative PKI a été refusée sans réparation
en place ; l'extension OAuthConnect a ensuite été générée sous la CA actuelle.

Le flag opt-in **`deploy --oauth-connect --preserve-platform-tls`** évite
l'application des trois Secrets d'identité existants et conserve la référence
TLS OpenBao enregistrée. Les consommateurs et montages sont contrôlés par UID,
les acquittements d'inputs antérieurs sont requis, et la CA OpenBao doit être
identique. Seule la feuille OAuthConnect est créée lors de cette activation.
Ce mécanisme ne lit pas les Secrets live, ne prouve pas leurs octets/UID actuels
et ne désactive aucune validation TLS. Il ne recrée pas une identité manquante.

La dette demandée par l'utilisateur est conservée dans
[certificats locaux et reprise Docker](../guides/local-tls-reprise.md).
Le souvenir d'expiration pendant un arrêt n'a pas été confirmé : les certificats
locaux inspectés n'étaient pas expirés. Il ne faut pas assimiler ce cas à une
validation globale de tous les certificats du cluster.

## Validations à ce stade

- Tests ciblés finaux : 37 réussites (onze préservations TLS, dont câblage réel de
  `deploy_locked` sous mocks, plus 26 Ops/simulateur existants).
- Suite offline Ops finale : **231 tests, 229 réussites, deux ignorés opt-in, zéro
  échec**, exit0. Les ignorés ne constituent aucune preuve runtime.
- Le flag de préservation est refusé hors de `deploy --oauth-connect` (exit2
  attendu), notamment pour un rendu offline.
- Build Linux neuf cibles et chargement Kind : exit0, reçus privés conservés.
- Revue infra indépendante statique : PASS, aucun blocage. Gaps de tests CA/flags
  comblés, combinaison `--isolated-plugins` refusée et limite de détection runtime
  d'un Secret absent précisée. Le profil ne garantit pas le fonctionnement des
  connecteurs legacy laissés présents dans l'inventaire.
- Premier essai runtime refusé **avant mutation** : le runtime n'était pas lié
  au bail avant les préflights API. Oracle d'ordre reproduit rouge, puis correction
  bornée : les baux déjà ancrés sont liés/revalidés avant le premier GET ; la
  première installation pending reste liée après bootstrap. Suite finale verte.
- Deuxième tentative de déploiement : exit1 sur l'apply de fondation, quatre
  conflits SSA avec le gestionnaire `kubectl-patch` (deux champs egress, deux
  ingress). L'apply est partiel : une feuille TLS Connect a été créée et des
  ressources de fondation ont pu être appliquées avant le refus. Le handler a
  tenté de rétablir les nombres de replicas initiaux ; l'état réel est à vérifier.
  Aucun `--force-conflicts`, nettoyage ou nouvelle tentative aveugle. Diagnostic
  Kubernetes read-only en cours avant une réconciliation bornée qui conserve les
  réparations précédentes. **Activation non terminée**, pas de succès live/E2E.

## Archives et publication

Les reçus, logs, inputs simulés et inventaires avant/après restent dans une
archive privée propriétaire hors Git ; le ledger de reprise est
`.cursor/handoffs/20261108-ops-oauthconnect-reprise.md`. Aucun secret ou fichier
PKI ne fait partie des livrables durables. Le commit publié, le résultat runtime
et l'inventaire final seront renseignés après vérification effective.

Le cycle quotidien sera enregistré explicitement dans un nouveau dossier privé,
sans écraser l'ancien état ni adopter son registre OCI arrêté. Le même bail et
les deux sources privées inchangées restent requis. Le marqueur de profil explicite
borne la reprise/readiness à OAuthConnect tout en conservant les anciens objets
dans l'inventaire d'ownership ; sans marqueur, l'ambiguïté est toujours refusée.
Les trois oracles de continuité supplémentaires portent le gate ciblé à
66 réussites (TLS, OAuthConnect, lifecycle et simulateur).
