# ADR-0606 : Le lab local-full utilise un cluster Kind dédié, isolé du legacy, avec ses dépendances et états dans le cluster

- Statut : Proposed
- Réalité : Partial
- Date : 2026-10-04
- Décideurs : à remplir à l'acceptation ; autorisation humaine du périmètre local et du contexte full le 2026-10-04, pas Accept implicite de cette ADR
- Jalon concerné : Local-full / Vague 4 locale (hors Apparatus P0–P6)
- SuperSède : aucune
- SuperSédée par : —
- Related : [0308](0308-mesh-authn-jwt.md), [0412](0412-oauth-transaction-persistante-liee-navigateur.md), [0601](0601-cluster-trust-namespaces-standalones.md), [0603](0603-tranche-locale-deploy-kind-apparatus-lazaret.md), [0604](0604-j3-overlay-demo-monolith-kind-invoke.md), [0605](0605-gold-path-kind-j3-dns-attach.md)

`Proposed` ne ratifie pas les choix techniques. `Réalité : Partial` : profil/manifests/helpers et rendu offline D6 présents ; aucun nouveau runtime, trafic, durabilité ou restore prouvé. SDK22 PASS n'est ni une compilation root complète ni une preuve IAM/mesh. Cette ADR décide une **frontière de lab** ; elle ne transforme pas les photographies J3/gold ou les cibles cloud en livraisons actuelles.

## Contexte

La demande est LOCAL COMPLET, sans cloud/GKE/Flux distant, HSM ou vendor fictivement livré. Le cluster legacy `aiforall-local` est étranger à cette tâche : ni son nom ni son image ne donnent un droit de mutation. Les overlays historiques J3/gold sont hybrides/monolithiques ; leur preuve ne suffit pas aux standalones, à la concurrence ni à la reprise du profil full.

L'utilisateur autorise explicitement un nouveau contexte full trois nœuds, après intégration et IT finale. Cette séparation change une frontière durable de déploiement, d'accès réseau et de stockage : une ADR forward distincte est préférable à réécrire 0404/0500 ou à étendre silencieusement 0604/0605. Le numéro 0606 est libre dans la plage Vague 4 à la rédaction.

## Décision

1. **Lab dédié.** Le profil full cible uniquement cluster `aiforall-local-full`, contexte `kind-aiforall-local-full`, un control-plane et deux workers. Jamais upgrade/recreate/delete implicite du legacy. Réutiliser exige inventaire et bail des IDs exacts, topologie/image/CNI compatibles ; mismatch ou propriété inconnue ⇒ STOP/INCONCLUSIVE et arbitrage humain. Nom/label/image seuls ne prouvent pas la propriété.
2. **Frontière locale isolée.** Les ressources full se placent dans `aiforall-local-full`, `aiforall-local-apparatus`, `aiforall-local-plugins`. Ce découpage du lab ne ratifie pas la carte cloud 0601. Les quatre services plateforme standalone, Lazaret et les dépendances de leurs parcours résident in-Kind ; pas de DB/queue/secrets obligatoires sur l'hôte. Images épinglées, migrations writer sérialisées avant scale-out et CNI enforce sont à vérifier réellement, pas à déduire du YAML.
3. **Trust et accès.** La CA projet reste propriétaire du trust ; les validateurs ne reçoivent que son matériau public nécessaire. Aucun Secret vivant écrasé ou lu pour constituer une preuve. Le mapping source actuel est HTTP loopback `127.0.0.1:18080 → control-plane:30080 → Envoy:10000`, API loopback port aléatoire ; cela ne livre pas un listener edge TLS inventé. Conflit de port ⇒ STOP, pas interruption de son propriétaire. La preuve TLS/mTLS entre composants reste distincte du mapping HTTP hôte.
4. **État dans le lab, pas promesse HA.** Les états persistants utilisent leurs PVC locaux déclarés ; sauvegarde/restauration vise une destination nouvelle et distincte, jamais une DB/PVC/PV vivante existante. Node-local n'est pas un stockage répliqué : survie pod, arrêt/reprise nœud, perte nœud et restore sont des propriétés différentes. Les limites DEV/volatiles/simulées sont exposées ; elles ne retirent aucun constat local du scope sans décision humaine.
5. **Preuve bornée.** Après intégration : compilation/units ciblées sous slot Docker unique, IT testcontainers, puis E2E sur le contexte full autorisé avec inventaire/bail. Chaque constat reçoit preuve au hash courant et qualification LIVE / SIMULÉ / NOT RUN / limite. Render, review source, pods Ready ou SDK22 ne constituent pas une clôture globale. Nettoyage réversible des seules ressources possédées selon le cycle de vie local ; aucune suppression/prune/reset automatique.

## Ancrage API au bail — S-11, 2026-10-04

S-11/MEDIUM reste ouvert : sources actuelles vérifient aliases/HTTPS loopback et IDs Docker séparément, sans preuve que kubectl vise ce control-plane. La frontière du lab exige une leasev2 liant contexte/cluster, ID CP exact, mapping publié TCP6443 host/port, kube-system UID et kubeconfig fixe task-owned. Le contrôle mapping/endpoint précède **tout API GET**, même UID ; seulement ensuite vérifier l’UID avant mutations. Absence/ambiguïté/stale/mismatch ⇒ STOP, jamais export/adopt/retarget de legacy/foreign. Ancrage UID initial seulement après création task-owned attestée et endpoint vérifié ; credentials Kind de cette création protégés, aucun Secret vivant lu ni raw config loggée.

Contrat exécutable : `.cursor/review-briefings/20261003-auth-fixes-interface-contract.md` §15, helpers PS/Python partagent modèle/vecteurs. D corrige sources, E/D couvrent fake-command wrong-port (zéro API), UID stale (zéro mutation), missing mapping/leasev1 et alias protégé déguisé ; aucun runtime demandé ici. Cette garde est **à implémenter/prouver**, pas une propriété déjà livrée ni une nouvelle autorisation de contexte.

## Conséquences

- Blast radius architectural : `ops/deploy/apps/overlays/local-full/`, `ops/deploy/kind/cluster-local-full.yaml`, helpers full, configuration mesh/CA/URLs, namespaces operator/Adm-A et paramètres des preuves E2E ; IAM setup/monolith/tests ne sont pas réimplémentés ici.
- Migration additive : nouveau lab/namespace/états ; aucune migration des ressources legacy ou substitution de Secret. Dépendances et images requises doivent être disponibles avant apply ; échec ⇒ restitution honnête des ressources créées et preuves manquantes.
- Séparer cible durable et source actuelle : quatre services peuvent se répartir sur workers ; Lazaret conserve sa limite process-local/single replica. PVC PostgreSQL/Zot/Redis/admission-store déclarés ; OpenBao DEV et queues LocalStack restent volatils. Catalogue et SMTP MailHog sont simulés ; OAuth externe est désactivé par défaut, donc OAuth/PKCE E2E exige la fixture/config prévue, pas une assertion de livraison vendor.
- Les gates restants sont réseau réellement enforce, crypto/trust/CA-only, transactions/concurrence/refresh/reset/purge, attach/admission/invoke, arrêt/reprise et backup/restore distinct. LocalStack/OpenBao DEV/PVC seuls ne prouvent pas durabilité ou HA.

## Alternatives rejetées

- Transformer le cluster legacy en full : propriété étrangère et risque de perte d'état ; pas d'autorisation implicite.
- Garder les dépendances sur l'hôte et appeler le profil complet/isoprod : reconduit l'écart hybride J3 et masque les frontières réseau/stockage.
- Déduire livraison/HA de trois nœuds, 112 objets ou PVC : confond source, topologie et comportement réel.
- Appliquer GKE/Flux/HSM pour fermer le lab : hors demande ; 0600–0602 restent des cibles futures non livrées.

## Non décidé ici

- Accept formel de 0606 et des ADR encore Proposed ; cloud/prod/HA, vendors OAuth/HSM, remplacement du backend DEV/volatile ou tolérance définitive de ses limites.
- Une limite n'est acceptée comme clôture de constat qu'après preuve ou arbitrage explicite humain. Pas d'Accept autonome ni Supersede d'une Accepted.
- Reproductibilité J3/gold dans le périmètre autorisé : ne pas transférer leur résultat historique aux standalones ; rendre INCONCLUSIVE si nécessaire.

## Références et état des preuves

- Source : `ops/deploy/apps/overlays/local-full/README.md`, `kustomization.yaml`, `namespaces.yaml` ; `ops/deploy/kind/cluster-local-full.yaml`.
- Helpers : `ops/deploy/bootstrap-local-full.ps1`, `deploy-local-full.py`, `render-local-full.py`, `local-full-images.py`, `local-full-storage.py`, `local-full-node-recovery.py`.
- État D6 : `ops/deploy/PACKAGE-D-CHECKPOINT.md` rapporte rendu offline **112 ressources**, image nœud épinglée/topologie1+2, mapping loopback et projection CA-only. Aucun nouveau test runtime D ou architectural ; 111 décrit le rendu source antérieur.
- Baseline wiki antérieure : `projects/aiforall/decisions/0603-tranche-locale.md`, `0604-j3-overlay-demo-monolith.md`, `0605-gold-path-kind.md` ; anciennes limites explicitement historiques. Canon local et auth : ADR0308/0412/0603–0605.
- Contrat source/proof : `.cursor/review-briefings/20261003-auth-fixes-interface-contract.md` §§8–13 ; autorisation utilisateur du 2026-10-04. Root HEAD `5348a63`, modifications non committées ; SDK `ca2e35fcd56279e9e52625d0df9381f240f3390d` publié/sélectionné selon parent, 22 tests purs PASS limités au SDK.
- Preuves intégrées compilation root complète, IT, E2E réseau/durabilité/reprise/restore : **NOT RUN / non fournies pour cet ensemble à cette réconciliation**. 0404/0500 et 0600–0602 inchangés.
