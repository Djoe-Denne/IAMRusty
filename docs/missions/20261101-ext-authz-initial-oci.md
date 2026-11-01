# ext-authz — première mise en place OCI locale

## Mandat et résultat

Second **« oui » explicite** après la bascule stockage/CRI : remplacer uniquement
l'image ext-authz et sa politique de pull, sans changement du reste du Deployment.
Base exécutée `master@1f7cbbe47ee84d976cbc5c015fc4dd5bf286d161`, SDK
`93b890ff7b821e9cb2e7d0aeb81957d098778a2f`.

**Placement initial PASS** : nouveau Pod Ready, zéro restart, digest OCI réellement
utilisé par le conteneur CRI. **Refresh à digest identique PASS**, zéro mutation.
Ce no-op ne qualifie pas un refresh entre deux digests différents.

Pas de modification de code de production, nouveau build, publication fabriquée,
`kind load`, purge ou recréation de cluster. Pas de commande/Job de migration,
bootstrap OpenBao ou full deploy ajouté. La reprise normale des conteneurs
préexistants peut rejouer leurs initialisations idempotentes habituelles ; elle
ne constitue pas une preuve d'absence de toute requête SQL.

## Préflight et préservation

Reprise des trois nœuds et du registre applicatif **originaux**, par le bail
`fcfd0b2884e14748a56f1e0c9d8c1602`. Pas de `local-up`, qui pourrait déclencher une
récupération OpenBao hors de cette autorisation. Nœud local partagé hors périmètre.

Les quatre égalités avec le vrai reçu `aiforall.image-release/v1` ont été revérifiées :
snapshot des octets courants, inputs ext-authz, SDK consommé, inputs non-Rust du
refresh. Réutilisation du digest publié, sans reconstruire pour changer une
étiquette de révision. Manifest/config OCI revérifiés via le registre mTLS et
cache CRI exact contrôlé sur les trois nœuds. Ancienne image également conservée
et vérifiée sur les trois nœuds pour le retour arrière.

Comparaison des configurations runtime : Env, Cmd, Entrypoint, WorkingDir,
ExposedPorts, Volumes, Healthcheck identiques. Seul `USER` diffère dans les images :
`65534` → `65534:65534`. Le premier préflight strict a conservé
`BLOCKED_RUNTIME_CONFIG`, sans patch du Deployment.

Cet écart est entièrement surchargé par le `securityContext` **déjà présent** :
`runAsUser=65534`, `runAsGroup=65534`, `runAsNonRoot=true`, inchangé. La vérification
effective CRI avant/après confirme exactement UID 65534, GID 65534 et
`additionalGids=[65534]`. Pas de normalisation aveugle ni de relâchement du guard
général de `refresh`. Source primaire lue :
[Kubernetes KEP213 — run-as-group](https://github.com/kubernetes/enhancements/tree/master/keps/sig-node/213-run-as-group).

L'ancien ext-authz est redevenu Ready avant le patch ; l'état non-Ready capturé
immédiatement au redémarrage des nœuds était transitoire.

## Mutation exacte et preuve du runtime

Deployment UID conservé : `f643b83a-4d23-4fe3-bb33-06100f605d97`.
Patch JSON avec tests UID, resourceVersion, ancienne image et ancienne policy,
puis remplacement des **deux seuls champs** :

- Image locale précédente :
  `aiforall-ext-authz:sha256-ac992d3e7bf9b69402f1e59a71db8fd50512c1d903e6d784c5bb01a70d9415bb`.
- Nouvelle image :
  `aiforall-apps-registry:5000/apps/ext-authz@sha256:dd015871d73913647e8515be64a32d44dc9de074f6f4f8a8894158778a9ff008`.
- `imagePullPolicy` : `Never` → `IfNotPresent`.

Ancien Pod `2889b8d9-fb3b-4812-a8df-a595731ac3c4` → nouveau
`b59d392c-9ad6-4b7c-a9d1-ce643720adeb`, Running/Ready, zéro restart.
Conteneur CRI `ec1d32aadab4dff1fad6ae1ad598f5356fb1321616df07e5aca81a116f732af9`
sur le worker2 original. L'attribution CRI lie ce conteneur au repoDigest
demandé ; `inspecti` confirme la config
`sha256:d08bc79ba41e4b25dc42abd56ca9d404504936323e47877330d58ec21338a66d`.
Owner Pod→ReplicaSet→Deployment attesté ; le reste de la spec cible, les specs des
17 autres Deployments et les UID/specs des trois PVC du namespace sont restés
identiques. Le namespace apparatus est hors de cette comparaison.

Le digest était déjà en cache après les vrais pulls 3/3 de la
[bascule CRI](20261101-local-full-cutover-results.md) : cette tranche prouve son
utilisation effective par le nouveau Pod, pas un téléchargement supplémentaire.

## Baseline légitime du futur refresh

Le reçu publié existant est désormais associé à une image **réellement déployée**.
`published-receipt-binding.json` lie son SHA256, le Deployment, le nouveau Pod et
le digest ; le reçu n'a été ni réécrit, ni inventé/adopté pour tromper le préflight.
Une copie **octet pour octet** est conservée dans le dossier privé sous
`published-release-manifest.json`, avec liaison et SHA dans
`durable-receipt-binding.json` : le futur précédent ne dépend donc pas de la
survie d'un fichier `.tmp`. SHA du reçu original/copie :
`b938d5d15c7f3b54f4a5535935ec027740407c1d48395cbbdb57ed55878a1675`.

Une invocation réelle de `deploy-local-full.py refresh --targets ext-authz` a
utilisé ce même reçu publié en avant/après. Exit0, sortie
`image-only refresh complete: 0 deployment(s); no provisioning or migrations`,
spec/génération/UID inchangés. Cela valide le chemin strict no-op après placement
initial, **pas** une mise à jour d'image. Aucune modification artificielle du code
ou rebuild inutile pour produire un second digest de démonstration.

À la prochaine modification réelle de Rust : garder ce vrai reçu comme précédent,
construire/publier les octets modifiés avec les caches existants, puis utiliser
le nouveau reçu. Recettes/configuration/migrations et profil doivent rester
compatibles ; sinon un plan distinct est nécessaire.

## Preuves, vérification et retour au repos

Preuves owner-only hors Git :
`C:/Users/djden/.aiforall-local-cutovers/20261101-4f15ec3ba214/ext-authz-initial-oci/`.
Principaux fichiers : `before-live.json`, `initial-oci-preflight-proof.json`,
`rollback-image-cache-proof.json`, `legacy-effective-user-proof.json`,
`patch-intent.json`, `after-live.json`, `initial-oci-placement-proof.json`,
`published-receipt-binding.json`, `durable-receipt-binding.json`,
`published-release-manifest.json`, `refresh-noop-proof.json`, `proof-inventory.json`.
Aucune configuration privée complète, clé, certificat ou contenu métier publié.

Registre : `.tmp/docker-resources-20261101.log`. Handoff :
`.cursor/handoffs/20261031-ops-portable-deployment-discovery.md`.
Syntaxe des deux opérateurs temporaires validée ; pas de nouveau Cargo, build ou
campagne E2E. Les 61 tests de la base source restent la validation existante,
sans delta de production à retester.

### Vérification indépendante

Même session `infra-verifier` `ses_e67e398a5ffeKtbbD2J4h18v7g`, autorité du bail
original, aucun pull/test/restart/écriture supplémentaire : **7/7 critères PASS,
0 FAIL, 0 INCONCLUSIVE**.

Vérifiés indépendamment sur le live : mêmes identités cluster/nœuds/Deployment,
exactement deux champs de spec modifiés, nouveau Pod Ready sans restart et owner RS,
containerID/UID Pod/digest OCI, UID/GID/groupes effectifs identiques à la preuve
avant, 17 autres templates/UID et trois PVC inchangés. Reçu source relu : SHA
conforme à sa liaison ; génération live 2 égale au snapshot après placement.
Le reviewer a lu la sortie no-op et vérifié qu'aucun update inter-digests n'était
revendiqué.

Limites : namespace apparatus hors des snapshots ; aucune réanalyse PG/Manifesto/
operator ni log applicatif. Configs image privées non ouvertes par le reviewer :
l'écart USER s'appuie sur le JSON de preuve et son contrôle effectif CRI live.
Événements historiques du Pod non collectés ; état Ready et zéro restart observés.

### Retour au repos

**`just local-stop` exit0**, puis inspect réel via les autorités du bail/registre :
trois nœuds full, registre et cinq helpers enregistrés arrêtés, conservés. Zéro
conteneur de ce bail encore running. Les cinq helpers étaient déjà exited.

Inventaires complets : **12 avant / 12 après, mêmes ID et états finaux**. Aucun
nouveau Docker, retrait ou prune. Cela ne signifie pas que le runtime n'a jamais
été touché : nœuds/registre ont été repris, le Deployment a reçu le patch décrit,
puis les ressources du bail ont été arrêtées. Nœud local partagé `40201bda…`
toujours running, deux helpers historiques non adoptés toujours exited.

Seize registres examinés avant cleanup, neuf IDs du bail retrouvés, tous les
événements pertinents lisibles. Comparaison finale sans conflit de propriété :
ressources préexistantes sous bail séparées des nouveaux IDs créés (aucun).
Preuves : `pre-cleanup-resource-registry-proof.json`, `final-runtime-inventory.json`,
`final-runtime-comparison.json`, log privé `local-stop-run.log`.

Bail actif libéré, aucun agent/job hôte encore actif ; Desktop/WSL partagés
conservés, delta RAM non mesuré. Archives, reçu original/copie, anciennes images,
données, certificats et caches retenus, pas de suppression à arbitrer pour finir
cette opération. OpenBao local perd normalement son état mémoire à l'arrêt.

Documentation uniquement livrée dans cette tranche ; aucune branche, worktree ou
équipe créée à fusionner/retirer/archiver. Les 82 branches/5 worktrees préexistants,
SDK et modification UI Obsidian étrangère sont conservés. Diff whitespace et liens
locaux vérifiés ; pas de relance des 61 tests pour ce delta documentaire.

**Tranche autorisée terminée**. La copie privée du vrai reçu fournit le précédent
légitime du prochain refresh, sans dépendre de `.tmp` ni inventer de preuve.

La disponibilité globale de l'ancienne plateforme, l'upgrade OpenBao/operator,
l'E2E auth/métier, le refresh entre deux digests et la livraison cloud restent
hors de cette preuve. OpenBao local demeure volontairement volatile.
