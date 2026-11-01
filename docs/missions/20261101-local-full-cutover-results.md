# Bascule locale préservant les données — résultats

> État historique de clôture en `1f7cbbe`. Après un second accord explicite,
> la [première mise en place OCI ext-authz](20261101-ext-authz-initial-oci.md) a
> été exécutée. Les mentions ci-dessous d'image locale/Never et d'accord encore
> nécessaire décrivent la fin de la tranche stockage/CRI, pas l'état après ce GO.

## Périmètre et verdict

Après le **« oui » explicite** pour l'indisponibilité contrôlée, sauvegarde et
restauration vérifiées, bascule PostgreSQL/Zot puis activation CRI progressive :

- **Stockage PASS** : les deux Deployments montent leurs PVC existants ; données
  restaurées et vérifiées avant remplacement normal des Pods.
- **CRI PASS, 3/3 nœuds** : configuration ciblée, restart workers puis control-plane,
  RuntimeReady/NetworkReady, nœuds Ready et UID original vérifiés à chaque étape.
- **OCI → CRI PASS, 3/3 nœuds** : registre applicatif mTLS existant, `crictl pull`
  puis `inspecti` du digest exact ; aucun nouveau build, `kind load` ou import `ctr`.
- **Persistance post-CRI PASS, 8/8 bases** : mêmes valeurs/séquences/large objects,
  schémas restaurés et globals ; **7/7 writers rétablis à 1**, templates inchangés.
- **Refresh NOT RUN** : image live ancienne différente du reçu OCI précédent.
  Pas de faux reçu ni de modification de la garde pour obtenir un PASS.

Base exécutée : `master@6a322278d0696dad37f6795144e6e545a136f181`, SDK
`93b890ff7b821e9cb2e7d0aeb81957d098778a2f`. Pas de modification du code de
production dans cette tranche ; opérateurs bornés jetables sous `.tmp`.

## Identités conservées

Cluster Kubernetes UID `39e0f244-3e4c-477e-8b9a-36bfb0ead766`, bail original
`fcfd0b2884e14748a56f1e0c9d8c1602`, contexte `kind-aiforall-local-full`.
Les trois conteneurs nœuds ont gardé leurs identités ; ni cluster neuf ni reset.

- PostgreSQL : Deployment `ad4c44eb-fd07-473e-b680-6b5bbf68052b` ; PVC
  `a1788d06-1b53-48f0-b9fb-5585aa0fb48b`, PV
  `23493d64-3154-42f6-b107-502106277dec`. Pod source
  `2e4e55e7-5052-4094-8e56-35b6354591bf` → Pod restauré
  `4af20475-ab6b-4cb8-823d-21405066ca0a`.
- Zot P4 **2.1.8** : Deployment `42c578b3-f484-4be3-9060-7aee1fa592ea` ; PVC
  `3f85e585-74eb-4411-861b-8217d673bbe3`, PV
  `4b7d3806-fd2d-4948-ba43-252dc070543a`. Pod source
  `f147d8a4-dadd-4ae4-a5c9-e952df10c8db` → Pod restauré
  `e39732c1-60e0-4ded-8600-e0e3ad475139`.

Seul l'élément `data` de chaque PodTemplate a été remplacé, avec tests JSON CAS
UID/resourceVersion et ancien élément. Images, environnement, configuration,
certificats et volumes auxiliaires `init`, `config`, `htpasswd` inchangés.
Les anciens Pods ont été retirés par Recreate normal après les preuves ; Zot a
été dégelé pour sa terminaison gracieuse. Aucun processus source laissé gelé.

## Préservation PostgreSQL

Sept writers attestés ont été mis à 0 : IAM, Hive, Telegraph, Manifesto, Lazaret,
OpenFGA, sentinel-sync. Leurs réplicas initiaux étaient tous à 1. Aucun autre client
PostgreSQL pendant capture et vérifications.

- **8 bases**, dumps custom **184615 octets**, TOC de chaque archive relue.
- Rôles/globals conservés, incluant les propriétés d'authentification privées.
  Sur la cible neuve, seul le `CREATE ROLE postgres` déjà réalisé par initdb est
  omis ; ALTER/attributs inchangés et globals redumpés identiques.
- Toutes les valeurs de lignes triées déterministiquement, inventaire des
  relations, séquences, large objects et propriétés des bases vérifiés.
- Schéma complet vérifié : dix réécritures de casts CHECK/index après restore
  prouvées individuellement par PostgreSQL sur tables temporaires vides puis
  rollback, avant égalité complète du dump attendu. Pas de normalisation regex
  globale, contrainte ignorée, `--clean`, réinitialisation ou ignore-errors.
- Les quatre fichiers de configuration PostgreSQL, dont `pg_hba.conf`, ont été
  copiés séparément vers le clone **offline** et comparés octet/mode/owner/mtime.
  Le trust temporaire du helper isolé n'a pas été exposé au Pod applicatif.
- Les huit bases ont été revérifiées dans le nouveau Pod avant la phase CRI.

La première comparaison DDL brute a correctement refusé les casts normalisés,
avant tout remplacement source. Preuve FAIL conservée ; la reprise a réutilisé
les bases déjà restaurées, sans reset du PVC.

Explication primaire des casts :
[PostgreSQL — Tom Lane](https://www.postgresql.org/message-id/1055940.1616595998%40sss.pgh.pa.us).

## Préservation Zot P4

Zot distroless n'a pas de shell. Après vérification Pod/owner/CRI/PID/nœud,
absence de liveness et fin de startupProbe : SIGSTOP ciblé, tar depuis le seul
emptyDir data sur le nœud attesté, restauration non-root dans le PVC isolé.

Trois membres ont été vérifiés : octets/SHA, type, taille, uid/gid, mode, mtime.
Premier essai refusé pour un bit setgid de répertoire perdu par l'extraction.
Deuxième refusé car un fichier interne de 32 KiB changeait après SIGCONT. Dernier
essai borné : capture fraîche, actualisation du clone offline possédé dans un
seul gel, mêmes chemins exigés, réparation exacte du setgid par son propriétaire,
égalité complète avant bascule. Archives et échecs conservés. Aucun nettoyage ou
retrait de fichier dans le clone, aucun volume vidé/remplacé ou source écrasée
pendant la préparation. Le retrait normal du Pod source et de son emptyDir
intervient ensuite, uniquement après vérification, comme décrit plus haut.

Helpers PostgreSQL/Zot : Jobs non-root, image PostgreSQL épinglée, ingress refusé,
PVC liés par nodeSelector et owner chains journalisées, stop gracieux puis Complete,
TTL de 120 s. **GC vérifié : les deux Jobs et leurs deux Pods sont absents.**

## Activation containerd et consommation OCI

containerd **2.0.3** : fichier disque v2, dump effectif v3. Configurations exactes,
permissions 644/root0:0 et propriétés systemd sauvegardées sur les trois nœuds.
`KillMode=process`, sans ExecStop/ExecStopPost : pas de destruction des shims.

Le candidat legacy `io.containerd.grpc.v1.cri.registry` n'activait pas le plugin
images ; refus hors ligne, **0 installation, 0 restart** dans cet essai.
Le candidat direct `[plugins."io.containerd.cri.v1.images".registry]`,
`config_path="/etc/containerd/certs.d"`, conserve `version=2` sur disque. Son dump
effectif ne change qu'exactement ce champ. Tous les candidats validés **avant**
installation ; restart progressif et revalidation de chaque nœud. Plugin transfer
et CNI inchangés, aucun effacement d'image/cache ; originaux récupérables conservés.

Le registre applicatif **Zot 2.1.22**, distinct du Zot P4, a gardé le conteneur
`12d9ff217e8639f56e1c4cfc0ffca103929e1cbb925cdc3efe247ad6555c28d1`, sa PKI
et ses données. Trust reader limité à son authority, sans rotation de certificat.
Publication existante revérifiée avant la connexion/pull :

`aiforall-apps-registry:5000/apps/ext-authz@sha256:dd015871d73913647e8515be64a32d44dc9de074f6f4f8a8894158778a9ff008`

Les trois `crictl pull` ont retourné
`sha256:d08bc79ba41e4b25dc42abd56ca9d404504936323e47877330d58ec21338a66d` ;
`inspecti` a confirmé le repoDigest demandé sur chacun. Un ID retourné par CRI
ne remplace pas le digest OCI et ne donne aucune hypothèse sur un Docker `.Id`.

La capture des **octets courants** après bascule confirme quatre égalités avec
le reçu publié : snapshot source, inputs ext-authz, octets du SDK consommé,
inputs non-Rust du refresh. Le reçu porte une révision historique `afbccf9`, la
base courante est `6a32227`, mais les octets pertinents sont identiques. C'est
une preuve locale `snapshot-final-v1`, pas une release cloud issue d'un master
exact. Aucun rebuild inutile pour modifier seulement une étiquette de révision.

## Limites et suite

ext-authz live :
`aiforall-ext-authz:sha256-ac992d3e7bf9b69402f1e59a71db8fd50512c1d903e6d784c5bb01a70d9415bb`,
`imagePullPolicy: Never`. Cette image ne correspond pas au reçu OCI précédent.
Le refresh exige cette égalité exacte : **non exécuté**, pas de reçu « avant »
fabriqué/adopté, pas de patch de contournement. Une première mise en place OCI
est une transition distincte à planifier avant la preuve d'un refresh ultérieur.

Prochaine transition proposée, **non exécutée et sans accord déduit de la bascule
stockage** : attester le Deployment/image legacy et garder son retour arrière ;
vérifier le reçu et les inputs courants, comparer la configuration runtime des
deux images ; patch CAS du seul conteneur ext-authz vers le digest OCI et
`IfNotPresent`, puis rollout/readiness et contrôle des autres champs inchangés.
Aucun bootstrap, migration SQL, full deploy ou changement de certificat.
Une fois cette référence réellement déployée, elle pourra constituer un véritable
reçu précédent ; un no-op identique ne prouvera pas une mise à jour d'image.

L'installation IAM/Lazaret reste pré-`openbao-snapshot`, l'operator référence
encore OpenBao HTTP/hostPath et `admission-store` reste Pending. Pas de full deploy
ou upgrade implicite. OpenBao local volontairement volatile ; E2E auth/métier,
release cloud issue d'un master exact, pins/outils cloud et publication distante
restent non qualifiés.

## Preuves et clôture opérationnelle

Archives owner-only hors Git :
`C:/Users/djden/.aiforall-local-cutovers/20261101-4f15ec3ba214/`.
Pas de données, dumps, clés ni certificats publiés. Preuves principales :
`postgres-restore-resume-proof.json`, `postgres-config-proof.json`,
`zot-preserve-final-proof.json`, `storage-switch-proof.json`,
`containerd-activation-direct-proof.json`, `registry-cri-after-cutover-proof.json`,
`post-cri-persistence-resume-proof.json`, `migration-helpers-gc-proof.json`.
Complément de fraîcheur : `current-ext-authz-inputs-proof.json`.
Échecs précédents gardés avec noms distincts.

Registre append-only : `.tmp/docker-resources-20261101.log` ; handoff :
`.cursor/handoffs/20261031-ops-portable-deployment-discovery.md`.
Validation de code réutilisée : 61 tests ciblés PASS en 32,406 s, syntaxe de 10 fichiers
Python et revue source 6/6 de `62eeea5`, sans delta production depuis. Ces résultats ne sont
pas présentés comme de nouveaux tests E2E.

Après les pulls, le nouveau Pod PostgreSQL a conservé son UID et ses huit bases
ont été comparées à la restauration vérifiée, avant réouverture des writers.
Les sept réplicas ont ensuite été rétablis à 1 avec CAS UID/RV, ancien replica 0 et
vérification de chaque template inchangé. Cette reprise des réplicas ne vaut pas
qualification auth/métier ou disponibilité globale.

### Vérification infra indépendante

`infra-verifier`, même session `ses_e67e398a5ffeKtbbD2J4h18v7g`, a vérifié le live
via le bail original, sans écriture/pull/test supplémentaire ni lecture de Secret,
dump DB ou log applicatif : **12 critères, 10 PASS et 2 états préexistants traités
séparément**.

Confirmés : UID cluster/nœuds, Deployment/PVC invariants, seuls volumes data
modifiés, auxiliaires intacts, nouveaux Pods PostgreSQL/Zot Ready sans restart,
7 writers à 1 et templates avant identiques, GC helpers, hashes du fichier
containerd identiques aux candidats sur les trois nœuds, hosts-dir CRI effectif,
`inspecti` du digest exact sur chacun, ext-authz ancien inchangé, OpenBao inchangé.
Quelques événements de readiness pendant gel/restart se sont résolus.

- **Manifesto reste non-Ready** : déjà 0 ready et 27 restarts au préflight. Après
  reprise, même RS/hash/template, CrashLoopBackOff/exit1 au démarrage. Symptôme
  préexistant, aucune nouvelle régression attribuée à la bascule ; **cause racine
  INCONCLUSIVE**, sans analyse des logs applicatifs. Les données post-CRI ont
  leur preuve distincte, pas une preuve de santé applicative.
- **apparatus-operator non créé** : RS datant du 2026-10-31, ReplicaFailure
  hostPath interdit par PSA baseline dès sa création. Les événements du jour
  réessaient le même objet ; pas un nouveau défaut du cutover. `admission-store`
  toujours Pending, hors bascule.

Limites de l'indépendance : les empreintes DB ont été lues dans les preuves JSON,
pas recalculées par le reviewer ; le fichier containerd a été comparé au candidat
et le chemin CRI parsé dans le dump live, sans comparaison complète du dump
effectif avec les archives. La comparaison complète des champs effectifs avait
été réalisée par l'opérateur avant installation et après chaque restart.

### Retour au repos

**`just local-stop` exit0**, puis contrôle réel des IDs par l'autorité du bail et
du registre : trois nœuds full, registre applicatif et cinq helpers enregistrés
arrêtés, tous conservés. Zéro conteneur de ce bail encore running. Les cinq
helpers étaient déjà exited ; ils n'ont pas été supprimés.

Inventaires complets : **12 conteneurs avant / 12 après, mêmes ID et états finaux**.
Cela ne signifie pas « jamais touchés » : les trois nœuds et le registre ont été
repris puis arrêtés pendant le lot, avec les modifications ciblées attestées.
Deux helpers historiques non adoptés restent exited ; le nœud local partagé
`40201bda8bb2d1eeaa51287be96fc3448752c219dba74791fa326c5071705f29`
reste running. Aucun conteneur supprimé, aucun prune/reset ; caches et archives
récupérables conservés.

Seize registres ont été examinés avant cleanup ; deux contiennent les neuf IDs
du bail, tous retrouvés et lisibles. La comparaison finale distingue ressources
préexistantes sous bail et ressources nouvellement créées : une première
invocation du comparateur avec les anciens IDs comme « nouveaux possédés » a
signalé un conflit, conservé dans sa preuve brute. La preuve corrigée compare
l'inventaire complet inchangé et vérifie séparément les neuf IDs attestés exited.
Aucune suppression n'a été fondée sur cette classification.

Preuves : `final-runtime-inventory.json`,
`final-runtime-inventory-comparison-corrected.json` et log privé
`local-stop-final-run.log`. Bail actif libéré, aucun agent ou job hôte restant.
Desktop/WSL conservés à cause du nœud partagé non visé ; delta RAM non mesuré.
OpenBao local perd normalement son état mémoire, sans rotation des CA.

Les quatre livrables documentaires ont été publiés en
`283b36babcd9d5e6570601fa40471abdff7c4ba2` sur `origin/master`, référence distante
vérifiée ; cette section archive ensuite vérification/cleanup, sans delta code.
Inventaire Git conservé : **82 branches, 5 worktrees**, aucun créé pour ce lot,
SDK inchangé. Modification UI Obsidian étrangère conservée, SHA
`9c9aa84337ede35c018c450cd2a270becc3f82f6b56d44517baaad59bb397084`.
Pas d'équipe Ensemble à archiver ni de branche temporaire à fusionner/supprimer.

**Tranche autorisée terminée ; objectif global non clôturé** : première mise en
place OCI ext-authz, ancien OpenBao/operator et qualifications différées restent
distincts. Aucun runtime gardé actif pour attendre une décision.
