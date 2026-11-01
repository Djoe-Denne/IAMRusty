# Bascule préservant les données — ancien local-full

**Plan approuvé et exécuté le 2026-11-01 pour PostgreSQL/Zot et l'activation CRI.**
Résultats et limites : [compte rendu de bascule](../missions/20261101-local-full-cutover-results.md).
Ce document ne donne pas d'autorisation permanente de rejouer la migration.
Périmètre : les trois nœuds du
bail original `fcfd0b2884e14748a56f1e0c9d8c1602`, contexte
`kind-aiforall-local-full`. Ni cluster neuf, ni reset, ni déploiement global.
Approbation explicite nécessaire pour la courte indisponibilité et le remplacement
des Pods après sauvegarde/restauration vérifiées. `--confirm-quiesce` seul ne
donne pas cette approbation ; le deploy normal refuse le changement de stockage.

## État observé avant bascule le 2026-11-01

- PostgreSQL : Deployment UID `ad4c44eb-fd07-473e-b680-6b5bbf68052b`, Pod UID
  `2e4e55e7-5052-4094-8e56-35b6354591bf`, worker, `data=emptyDir`,
  `/var/lib/postgresql/data`, PGDATA `pgdata`. PostgreSQL15.12 épinglé.
- Zot **P4 dans Kubernetes**, pas le registre applicatif mTLS externe : Deployment
  UID `42c578b3-f484-4be3-9060-7aee1fa592ea`, Pod UID
  `f147d8a4-dadd-4ae4-a5c9-e952df10c8db`, worker2, `data=emptyDir`, `/var/lib/zot`.
- Les PVC `postgres-data`, `zot-data`, `admission-store` attendent un consommateur.
  Leurs données cibles sont distinctes des données actuelles des emptyDir.
- Aucun Pod operator créé : refus PSA du hostPath. Cela ne prouve pas que les
  anciens répertoires hostPath des nœuds sont vides ; le vérifier avant sa bascule.
- Les volumes auxiliaires PostgreSQL `init` et Zot `config`/`htpasswd` doivent
  rester inchangés. L'index live Zot `data` est0, l'index de la base source est2 :
  **résoudre chaque nom dans la spec relue**, ne jamais recopier l'index source.

Ces UID sont des observations, pas une autorisation permanente : revalider
ancrage, moteur, endpoint, namespace, Deployment/ReplicaSet/Pod et PVC à la reprise.

## Séquence bornée après accord

1. **Préflight privé** : mutex du bail original ; identité/UID/RV relus ; inventaire
   des writers, Jobs et réplicas initiaux ; vérification espace, outils et images
   déjà épinglés. Inconnu, Job actif, données supplémentaires ou identité changée ⇒
   arrêt et clarification. Aucun GET Secret. Archives owner-only hors Git, jamais
   sorties en chat ou dans un journal public ; empreintes et métadonnées seules.
2. **Quiescer les writers**, pas les Pods source PostgreSQL/Zot. Conserver leurs
   emptyDir jusqu'à validation de la restauration. Ne pas utiliser la branche Zot
   de `local-full-storage.py backup` : elle scale le Deployment à0, ce qui perdrait
   précisément cet emptyDir. Cet outil exige aussi une source PVC déjà liée.
3. **Sauvegarder PostgreSQL au niveau logique**, toutes les bases applicatives et
   les objets globaux/rôles, pas seulement `iam_dev`. Écritures applicatives/DDL
   arrêtées pendant la capture ; inventaire complet avant/après, sommes des archives,
   contrôle `pg_restore --list` des dumps custom. Conserver aussi les définitions
   globales privées. Prévoir explicitement les rôles système déjà créés dans une
   cible neuve ; pas de `--clean` ni d'ignore-errors pour masquer un restore partiel.
4. **Sauvegarder Zot** : empêcher les écritures sans détruire le Pod source.
   Un gel réversible du processus n'est admissible qu'après revalidation de
   l'absence de livenessProbe et de la fin de startupProbe ; sinon arrêter ici.
   Archiver `/var/lib/zot` en octets, valider chemins/membres/sommes/inventaire,
   conserver la possibilité de dégeler la source sur échec. Aucun secret/config
   de montage auxiliaire n'est exporté dans l'archive de données.
5. **Préparer les PVC existants sans les remplacer**, via helpers éphémères isolés,
   non-root et images épinglées, sans Service ni writers. Binding sur le nœud source
   approprié via `nodeSelector` (pas `nodeName`, qui court-circuiterait le scheduler
   avec WaitForFirstConsumer), UID PVC/PV journalisés. Refuser toute cible non vide/inconnue ; aucune
   réutilisation d'un PV étranger. Restaurer PostgreSQL avec le même outil majeur,
   puis vérifier bases/rôles/extensions, schémas, inventaires et comptes de lignes ;
   restaurer Zot et vérifier fichiers/index/blobs et sommes. Ces vérifications
   techniques ne valent pas le futur E2E métier.
   Préserver séparément `postgresql.conf`, `postgresql.auto.conf`, `pg_hba.conf`
   et `pg_ident.conf` : un dump logique ne les transporte pas. Le trust temporaire
   du helper isolé ne doit jamais être exposé par le Pod applicatif. Vérifier
   octets, propriétaire, permissions et dates avant la bascule.
6. **Barrière avant remplacement** : archives récupérables et restauration vérifiées,
   writers encore arrêtés, source toujours intacte. Sans cette preuve, ne pas
   modifier les PodTemplates. Nettoyage des helpers par leurs UID seulement, selon
   l'autorisation accordée ; aucune suppression générique.
7. **Bascule ciblée** : patch JSON CAS UID+RV, test du nom puis remplacement du
   seul volume data, mêmes images/env/config/certificats et autres volumes.
   Attendre chacun des deux workloads, recontrôler PVC/PV/UID et données restaurées.
   Ne pas appliquer le rendu entier : il inclut aussi un upgrade OpenBao et d'autres
   changements hors de cette bascule. Operator traité séparément après inspection
   de son ancien hostPath et de son câblage OpenBao pré-TLS.
8. **Reprise/retour arrière** : avant remplacement, dégeler les sources et restaurer
   les réplicas sur échec. Après remplacement, le retour arrière s'appuie sur la
   sauvegarde/PVC restauré, **jamais sur un retour à emptyDir vide**. Conserver
   archives et preuve, puis arrêter les ressources possédées devenues inutiles.

## Activation CRI indépendante

Avant activation : containerd2.0.3, fichier disque **v2**, dump effectif **v3**,
hosts-dir CRI vide. Ne pas déduire la table utile de la seule version disque.
Le YAML Kind corrigé n'a aucun effet rétroactif. Après autorisation ciblée :

- sauvegarder la configuration exacte et ses permissions par ID de nœud ;
- modifier seulement `plugins."io.containerd.cri.v1.images".registry.config_path`,
  valider le TOML et la configuration effective, sans toucher au plugin transfer ;
  sur ces nœuds, ajouter directement la table images tout en gardant `version=2`
  a été prouvé hors ligne puis exécuté. La table legacy
  `io.containerd.grpc.v1.cri.registry` était ignorée dans le dump effectif ; sa
  tentative a été refusée avant toute installation/restart. Exiger l'égalité de
  tous les autres champs du dump, pas simplement la présence du chemin ;
- redémarrer containerd un nœud à la fois, workers puis control-plane, revalider
  original UID, CRI et nœud Ready à chaque étape ; arrêter si un invariant diverge ;
- installer seulement la confiance reader du registre applicatif possédé avec
  l'autorité existante, puis `crictl pull`/`inspecti` du digest publié sur chaque
  nœud ; pas de `ctr`/`kind load` à la place de la preuve CRI ;
- garder le rollback exact de configuration. Ni nouveau cluster, ni effacement de
  cache, ni rotation CA. Les éventuels redémarrages applicatifs pendant l'opération
  ne sont pas qualifiés par un simple nœud Ready.

La bascule stockage doit précéder toute opération susceptible de remplacer les Pods
source. L'upgrade des identités OpenBao pré-snapshot et la disponibilité complète
de `local-up` restent des étapes distinctes ; coffre local toujours volatile.

## Précisions tirées de l'exécution

- Les chemins sont attestés depuis le Pod et le PV : source
  `kubernetes.io~empty-dir/data`, cible `/var/local-path-provisioner/…`, pas un
  chemin `/opt` supposé. Ne pas publier le contenu des archives privées.
- Zot P4 2.1.8 est distroless : gel SIGSTOP/SIGCONT de la tâche CRI identifiée,
  archive depuis le nœud, pas d'`exec sh` dans Zot. Capture/restauration/bascule
  réussies dans un seul gel ; après reprise, un fichier interne de 32 KiB changeait.
  Le clone offline possédé a été actualisé sans suppression ni reset, et son
  unique bit setgid perdu à l'extraction rétabli par son propriétaire non-root.
- Comparer tous les octets/permissions/propriétaires/mtimes Zot. Pour PostgreSQL,
  toutes les valeurs triées, séquences et large objects sont vérifiés, ainsi que
  rôles/globals et schémas complets. Dix normalisations CHECK/index ont été
  prouvées par le parseur PostgreSQL sur tables temporaires vides puis rollback,
  suivies d'une égalité du dump entier ; aucune contrainte n'a été ignorée.
- Les sept writers restent quiescés pendant stockage + CRI pour permettre les
  empreintes stables après restart ; rétablir ensuite leurs réplicas d'origine
  avec CAS UID/resourceVersion et vérifier les PodTemplates inchangés.
- Le pull OCI ne prouve pas un refresh : le Deployment ext-authz ancien référence
  encore une image locale avec `Never`, différente du reçu OCI précédent.
  Ne pas inventer/adopter de reçu « avant » pour satisfaire la garde du refresh.

## Sources de la procédure

- [Kubernetes — emptyDir](https://kubernetes.io/docs/concepts/storage/volumes/#emptydir) :
  persiste entre crashs de conteneurs, pas entre suppressions/remplacements de Pod.
- [StorageClass — binding](https://kubernetes.io/docs/concepts/storage/storage-classes/#volume-binding-mode) :
  WaitForFirstConsumer et contrainte `nodeSelector`, pas `nodeName`.
- [PostgreSQL15 — SQL dump](https://www.postgresql.org/docs/15/backup-dump.html) et
  [pg_dumpall](https://www.postgresql.org/docs/15/app-pg-dumpall.html) : globals/rôles
  non inclus dans pg_dump seul ; snapshots de bases distinctes non synchronisés,
  d'où la quiescence des writers pendant la capture multi-base.
- [containerd2.0.3 — hosts](https://raw.githubusercontent.com/containerd/containerd/v2.0.3/docs/hosts.md)
  et [CRI image pull](https://github.com/containerd/containerd/blob/v2.0.3/internal/cri/server/images/image_pull.go) :
  configuration examinée au tag du binaire réellement installé.
