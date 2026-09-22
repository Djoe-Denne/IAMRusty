# ADR-0601 : L’unité cluster V1 est 4+1 Deployments (iam, hive, manifesto, telegraph + lazaret) dans des namespaces `aiforall-*` identiques, sans tenancy par namespace

- Statut : Proposed
- Réalité : Unimplemented
- Date : 2026-09-20
- Décideurs : (à remplir à l’acceptation)
- Jalon concerné : Cloud-portable (hors Apparatus P0–P6)
- SuperSède : aucune — **complète** le Non décidé « Déploiement Kubernetes / un seul vs quatre Deployments » d’[0404](0404-runtime-microservices-et-monolithe.md) **sans** la réécrire ni la SuperSéder
- SuperSédée par : —

`Accepted` ratifie une cible. `Réalité` décrit le dépôt. Aucun manifeste cluster, aucun namespace `aiforall-*`, aucun Deployment standalone k8s n’existe aujourd’hui. Cette ADR **ne SuperSède pas** [0008](0008-apparatus-p4-k8s-isolation-outside-manifesto.md) (Accepted / Unimplemented) : elle **place** P4 dans la plateforme [0600](0600-cloud-portable-opentofu-k8s-gitops.md). Le split admit vs signer **reste ouvert dans 0008**.

**Interdit** : rédiger ceci comme ADR-0009 / 0411 / 0503.

## Contexte

[0404](0404-runtime-microservices-et-monolithe.md) (Accepted / Implemented) fige le **dual runtime** : quatre standalones Compose **et** `oodhive-monolith` préfixé laptop. Elle laisse ouvert : « Déploiement Kubernetes / un seul vs quatre Deployments. » `sentinel-sync` est **hors dual**, absent du Compose par défaut ([0303](0303-sentinel-sync-worker-fga.md), 0404).

[0008](0008-apparatus-p4-k8s-isolation-outside-manifesto.md) fige le moteur P4 : K8s **hors** `Manifesto/*/src` ; 4 SA ; plugins autre namespace ; operator + Jobs ; enveloppe OCI ≠ image CRI ; Cosign + Transit ≠ KV Lazaret ; Adm-A seule source `VALID`. Elle ne fige **pas** la liste des namespaces plateforme ni le nombre de Deployments métier.

Tenancy actuelle = org Hive + OpenFGA ([0402](0402-hive-organisations.md), [0302](0302-authn-jwt-authz-openfga.md)). La tentation k8s est le ns-per-tenant ou un seul Deployment monolithe « pour simplifier ».

Écart **non corrigé** dans 0500 : photo Compose sans OpenBao / Lazaret / mesh ; fichier réel les contient. 0601 ne réécrit pas 0500. Redis KV Lazaret : présent en config (`Lazaret/config/*/toml` `[redis]`) **et absent** de `docker-compose.yml` (KV compose = postgres). Cluster B/C **embarque** Redis dans `aiforall-data` ; couche A **conserve** le KV postgres actuel.

## Décision

### Unité de déploiement cluster

Unité cluster V1 = **4+1 Deployments** : `iam`, `hive`, `manifesto`, `telegraph` **+** `lazaret`, plus **`sentinel-sync`** sur les couches **B et C** (pas dans le Compose défaut, inchangé vis-à-vis 0404 / 0500).

Le **monolithe** = laptop / démo seulement. **Pas** un Deployment prod / staging / kind canon.

Tenancy = org Hive + FGA. **Ns-per-tenant interdit V1.**

### Namespaces (préfixe `aiforall-`, identiques sur chaque cluster)

| Namespace | Workloads |
|---|---|
| `aiforall-platform` | iam, hive, manifesto, telegraph, sentinel-sync |
| `aiforall-gateway` | **Lazaret seul** (SA `gateway`, `invoke`) |
| `aiforall-apparatus` | operator, admit/sign, controller, Jobs build |
| `aiforall-plugins` | pods untrusted **uniquement** |
| `aiforall-data` | Postgres multi-DB, OpenFGA, Redis KV Lazaret, queue adapter |
| `aiforall-secrets` | OpenBao (KV ≠ Transit), ESO |
| `aiforall-gitops` | contrôleurs Flux |
| `aiforall-obs` | OpenTelemetry Collector **ou** Alloy, Tempo, Grafana, Loki, Prometheus V1 — [0602](0602-observabilite-portable-otlp-lgtm.md) |

Mêmes noms sur kind, staging, prod (cluster-par-env, 0600). Pas de suffixe `-staging` dans le nom du namespace. **4+1 Deployments inchangés** (obs n’est pas un 6ᵉ slice HTTP).

### Identités

| Plan | Quoi |
|---|---|
| Utilisateur | JWT IAM ([0302](0302-authn-jwt-authz-openfga.md), [0400](0400-iamrusty-identite-hexagonale.md)) |
| Mesh app | mTLS T14b, CA **platform-mesh** (IAM / Hive / Telegraph) |
| Workload Lazaret | CA **distincte** (≠ platform-mesh) |
| Kubernetes | **4 SA** : `build`, `admit-sign`, `controller`, `gateway` (Run-A 0008 / identités 0003) |

SPIRE **pas V1**.

Split admit vs signer : **ouvert dans 0008**. Les 4 SA sont **conceptuels** ; **colocation admit+sign autorisée** jusqu’au split. Ne pas créer un 5ᵉ SA « pour anticiper ».

Job build **sans** token API cluster (0008). Plugins **uniquement** dans `aiforall-plugins`.

### Réseau

NetworkPolicy **default-deny**. Plugins → Lazaret `invoke` seulement (0004 / 0007). Pas d’I/O plugin vers `aiforall-data` / `aiforall-secrets` / API Kubernetes / **`aiforall-obs` (OTLP deny)**. `aiforall-platform` + `aiforall-gateway` + `sentinel-sync` **MAY** envoyer OTLP au collector ([0602](0602-observabilite-portable-otlp-lgtm.md)). UI Grafana via Gateway, pas depuis les plugins.

Kyverno (0600) enforce digest/signature **staging/prod**. **Kyverno ≠ Adm-A** : n’émet pas `VALID`.

### Données

| Composant | V1 |
|---|---|
| Postgres | **1 instance**, **6 bases** comme Compose : `iam_*`, `telegraph_*`, `hive_*`, `manifesto_*`, `lazaret_*`, `openfga_*` |
| Queue | Adaptateur rustycog : LocalStack (A) / SQS-like (B/C prod). **Kafka plus tard** |
| Redis | KV Lazaret dans `aiforall-data` (B/C). Couche A : KV postgres, pas de Redis Compose |
| Secrets plugin | Références **opaques** à l’`invoke`, **pas** ESO dans `aiforall-plugins` |
| OpenBao | KV (plugins / app) **isolé** de Transit (signatures 0008) |

### P4 s’insère ici — 0008 intacte

| Invariant 0008 | Placement 0601 / 0600 |
|---|---|
| Hors `Manifesto/*/src` ; zéro token `k8s`/`kubernetes` Manifesto | Operator / Jobs dans `aiforall-apparatus` + `deploy/p4/` (Kustomization Flux **séparée**) |
| Enveloppe OCI ≠ image CRI | kubelet n’exécute que `image@sha256` issu de l’enveloppe **admise** |
| Registry portable | Pas de registry cloud obligatoire ; ACL sur le registre |
| 4 SA ; Job build sans token API ; plugins autre ns | SA ci-dessus ; `aiforall-plugins` |
| Adm-A worker-only = seule source `VALID` | Kyverno / Gateway / Flux = enforceurs, **pas** source |
| Transit ≠ KV Lazaret | Transit dans le TCB signer ; KV dans `aiforall-secrets` |
| Kind **pas** prérequis T2 unitaire | Preuves P4 T2+ selon prompt 0008 ; kind = couche B 0600 |

K8s-as-P3 reste **interdit**.

### V1 vs plus tard

| In V1 | Plus tard |
|---|---|
| 4+1 Deployments + sentinel-sync B/C | Host Apparatus dans le dual (P5, 0002 / 0406 — encore 0404 Non décidé) |
| 8 namespaces `aiforall-*` identiques (dont `aiforall-obs`, 0602) | Ns-per-tenant (interdit V1, pas un « later » par défaut) |
| 4 SA conceptuels ; colocation admit+sign OK | Split admit vs signer **si** 0008 le tranche |
| 1 Postgres, 6 DB ; queue SQS-like ; Redis KV B/C | Kafka ; Velero (0600) |
| NetworkPolicy default-deny ; plugins → invoke | SPIRE ; mesh sidecar |
| P4 dans `aiforall-apparatus` + `deploy/p4/` | Nest HTTP / 6ᵉ hexagone (rejeté 0008) |

## Conséquences

- 0404 **inchangée** : dual runtime laptop intact ; le Non décidé K8s « 1 vs 4 » est **fermé ici** (réponse : **4+1**, pas 1).
- On ne documente **pas** le monolithe comme unité cluster. On n’ajoute **pas** `sentinel-sync` au Compose défaut « pour coller à B/C ».
- Un tenant Hive supplémentaire ≠ un namespace. FGA reste la tenancy.
- Secrets plugin : jamais un Secret ESO monté dans `aiforall-plugins`.
- 0500 **non réécrite** malgré l’écart Compose (openbao / lazaret / mesh) et l’absence de Redis en couche A.
- 0008 **non réécrite**. Skill `aiforall-new-service` **non** pour l’operator (0008 BC-A).

## Alternatives rejetées

| Option | Pourquoi pas (maintenant) |
|---|---|
| Monolithe = Deployment prod / kind | 0404 : laptop/démo ; standalones = défaut documenté |
| Un seul Deployment « plateforme » | Ferme 0404 du mauvais côté ; blast IAM/Hive/Manifesto/Telegraph |
| Ns-per-tenant | Tenancy = Hive + FGA ; [multi-tenancy k8s](https://kubernetes.io/docs/concepts/security/multi-tenancy/) ≠ org métier V1 |
| SPIRE V1 | 4 SA + 2 CA T14b suffisent |
| ESO dans `aiforall-plugins` | Refs opaques à l’invoke (0004 / 0007 T12) |
| OpenBao Transit = KV plugin | 0008 |
| Fusion `deploy/p4/` × overlays Manifesto | 0008 hors Manifesto |
| Kind obligatoire pour T2 unitaire P4 | 0008 : Kind pas prérequis T2 |
| 5ᵉ SA « admit » + « signer » anticipé | Split encore ouvert dans 0008 |
| Plugins dans `aiforall-platform` | 0003 / 0008 Run-A : autre namespace |
| Kafka V1 | Queue = adaptateur rustycog SQS-like |

## Non décidé ici

- Backend OTLP / LGTM+Tempo — **fermé** par [0602](0602-observabilite-portable-otlp-lgtm.md) (complète Q3 0600, ne SuperSède pas 0600). Helm LGTM vs charts séparés reste dans 0602.
- Split admission vs signer (0008).
- Inclusion d’un host Apparatus dans le dual (0404 / P5).
- Produit registry IT (zot / ORAS vs `registry:2`) — contrainte 0008 : artifacts **et** pull d’image.
- CPU / budget numériques (`APP-06`).

## Références

- Wiki : `obsidian/AI FOR ALL/projects/aiforall/decisions/0601-cluster-topology.md` ; 0008 `obsidian/AI FOR ALL/projects/manifesto/decisions/0008-apparatus-p4-k8s.md` ; mesh `obsidian/AI FOR ALL/projects/aiforall/concepts/https-platform-mesh.md`
- Dépôt : `docs/adr/0008-apparatus-p4-k8s-isolation-outside-manifesto.md`, `docs/adr/0404-runtime-microservices-et-monolithe.md`, `docs/adr/0500-config-typee-et-compose-local.md`, `docker-compose.yml` (6 DB `create-databases`), `Lazaret/config/development.toml`
- Contrat : `docs/platform-cloud-v1-implementation-contract.md`
- Web : [kind](https://kind.sigs.k8s.io/), [Gateway API](https://gateway-api.sigs.k8s.io/), [Kubernetes multi-tenancy](https://kubernetes.io/docs/concepts/security/multi-tenancy/), [Flux bootstrap](https://fluxcd.io/flux/installation/bootstrap/), [ESO OpenBao](https://external-secrets.io/latest/provider/openbao/), [Kyverno](https://kyverno.io/docs/policy-types/overview/), [12factor config](https://12factor.net/config)
- Preuve d’implémentation : `aucune`
