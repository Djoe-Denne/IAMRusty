# Plan d'upgrade lab `kind-aiforall-local` — RATIFIÉ par l'utilisateur le 2026-10-20

Source : proposition de l'architecte (session `ses_ea74493afffe7YLzuZwgYWHULI`, vérifiée par la lead). **L'exécution reste une campagne runtime DISTINCTE** (gates/préflight préalables obligatoires) — ce document = cible + procédure ratifiées, pas un GO d'exécution.

## Cible ratifiée

- **Kubernetes v1.35.8** (node image `kindest/node`, digest à collecter/vérifier au préflight)
- **Kind v0.33.0** (containerd **2.3.4** embarqué — recoupé avec le Dockerfile + métadonnées des couches OCI)
- **Calico v3.33.0** (compat K8s 1.35 vérifiée docs Tigera ; remplace le pin 3.29.7 du lock documentaire)
- Knative 1.23 reste la cible Serving (min 1.34 satisfait)

## Préflight fail-safe (AVANT destruction)

1. Inventaire avant : contextes, baux, workloads présents, état des clusters (aucune mutation).
2. Télécharger + vérifier l'image node cible AVANT la suppression (disponibilité acquise avant destruction — jamais découvrir un problème de registre après).
3. Préparer le registre `.tmp/docker-resources-<YYYYMMDD>.log` (append-only) : image téléchargée avec ID Engine RÉEL (distinct du digest OCI), nœuds créés (IDs complets + état + bail), volumes créés (Name observé), événements de suppression des anciens objets (historique conservé). Images Calico dans containerd du nœud = inventaire CRI séparé, jamais de faux IDs Docker. Création opaque ⇒ `UNKNOWN` + `ask`.
4. Échec de création : `--retain` (IDs partiels conservés), erreur archivée, **STOP** — aucun deuxième create, nettoyage ou rollback automatique.

## Périmètre de modification repo (à exécuter pendant la campagne)

- `ops/deploy/kind/cluster.yaml` : pin node image → v1.35.8 (+ digest), pod subnet 10.244.0.0/16 inchangé, disable_default_cni inchangé.
- Guard partagé de sélection de version préservant `kind-aiforall-local-full` (cluster protégé, intouché).
- `ops/deploy/common.ps1` / README : hunks d'upgrade local uniquement.

## Reconstruction

- kind ne supporte pas l'upgrade in-place : **delete + recreate** de `kind-aiforall-local` uniquement (jamais les autres clusters ; kubeconfig privé du cluster concerné).
- Inventaire après : kubelet v1.35.8, containerd 2.3.4, nœud Ready, Calico/CoreDNS disponibles, aucun kindnet, pool unique `10.244.0.0/16`, nouveaux IDs/UID/kubeconfig (jamais de réemploi de l'ancien ancrage), images CNI conformes aux pins, contrôle réseau positif/négatif (NetworkPolicy enforcement avec canary disponible), comparaison avant/après des autres conteneurs/clusters/volumes/services externes sans modification imputable.
- **Ne pas restaurer automatiquement les workloads** — seulement les composants approuvés (migrations/données restaurables selon leur procédure validée). Les anciennes identités Pod/Revision et clés publiques d'enrollment ne deviennent PAS valides dans le nouveau cluster.
- La sortie « socle reconstruit » ne vaut NI gVisor opérationnel, NI Serving qualifié, NI fermeture B5/0009.

## Rollback borné (seconde reconstruction destructive — pas de downgrade in-place)

1. Preuves de l'échec + IDs/volumes du nouveau cluster conservés.
2. Fermer les accès concernés ; vérifier qu'aucun autre bail ne consomme ce cluster.
3. Confirmation humaine de suppression des nouveaux IDs (pertes depuis la reconstruction).
4. Supprimer UNIQUEMENT `aiforall-local` (kubeconfig privé).
5. Inverser UNIQUEMENT les hunks d'upgrade local : node `v1.32.2@sha256:142f543559cc55d64e1ab9341df08e5ced84bd2e893736da8f51320f26f5950b` ; Calico **3.29.7** ; guard correspondant.
6. Anciens outils conservés + troisième bail/ancrage neuf → recreate via `Initialize-LocalKind`.
7. Restauration selon procédure validée. **Jamais un etcd 1.35 dans 1.32.**
- Aucun `git reset`, restauration globale de fichiers, prune ou suppression de volume partagé. Les changements C/D/T indépendants sont conservés.
- Après rollback : **Knative 1.23 reste NO-GO** sur ce lab ; ne pas rouvrir les bindings migrés via endpoint digest ou ancien dispatch.

## Accord humain consigné (2026-10-20)

Versions (1.35.8/0.33.0/3.33.0/2.3.4) + périmètre repo (guard partagé préservant local-full) + pertes explicitées à l'exécution + procédure destructrice/reconstructrice + rollback. **Fin de campagne : arrêt gracieux des seules ressources possédées devenues inutiles** (sauf bail actif/keepalive explicite) — aucun arrêt global Docker Desktop/WSL affectant d'autres workloads.

## Sources (consultées le 2026-10-20 par l'architecte)

Kind 0.33.0 releases (images/digests) + Dockerfile containerd ; Calico 3.29/3.33 requirements ; Kubernetes patch releases/EOL ; Knative RELEASE-SCHEDULE (minima K8s) ; kind delete kubeconfig + create retain (code source).