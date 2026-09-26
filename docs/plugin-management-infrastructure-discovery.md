# Discovery — Infrastructure de management et d’exécution des plugins

## 1. Résumé exécutif

L’objectif est de construire une plateforme capable d’exécuter des **plugins fournis par des utilisateurs** de manière :

- **sécurisée** ;
- **très économe en ressources** ;
- **autoscalable** ;
- **adaptable dynamiquement** à la charge réelle ;
- **peu coûteuse à petite échelle** ;
- **massivement optimisable à grande échelle** ;
- compatible avec des workloads **CPU/RAM** comme **GPU/IA** ;
- sans exposer Kubernetes ou l’infrastructure interne aux développeurs de plugins.

La plateforme doit tendre vers une expérience proche d’un **serverless spécialisé** :

> l’utilisateur fournit un repository, une image, un exécutable ou un artefact ; la plateforme construit, profile, déploie, isole, route, scale et optimise automatiquement le plugin.

L’architecture ne doit pas dépendre d’un modèle « un plugin = un pod Kubernetes ». Kubernetes doit progressivement devenir une **couche de gestion de capacité**, tandis qu’un **scheduler de plugins** et des **runtimes mutualisés** assurent le placement fin des workloads.

---

## 2. Objectifs d’infrastructure

### 2.1 Objectifs prioritaires

1. **Minimiser le coût d’exploitation par unité de travail utile.**
2. **Réduire au maximum les ressources inutilisées** :
   - CPU idle ;
   - RAM réservée inutilement ;
   - GPU idle ;
   - pods conservés chauds sans nécessité ;
   - nœuds faiblement utilisés.
3. **Ne pas sacrifier la sécurité** pour améliorer la densité.
4. **Absorber automatiquement les variations importantes de trafic.**
5. **Permettre le scale-to-zero** pour les plugins peu utilisés.
6. **Conserver suffisamment de headroom** pour absorber les pics sans interruption perceptible.
7. **Décorréler le contrat du plugin de son runtime réel** afin de pouvoir changer d’implémentation sans casser l’écosystème.
8. **Mesurer objectivement les économies** par rapport à une architecture conventionnelle.

### 2.2 Ambition économique

La cible recherchée est un gain pouvant atteindre **un ordre de grandeur (~10×)** sur certaines classes de workloads par rapport à des architectures très surprovisionnées ou inefficaces.

Ce gain ne doit pas reposer sur une seule optimisation, mais sur l’addition de plusieurs leviers :

- right-sizing automatique ;
- scale-to-zero ;
- pooling ;
- bin packing agressif ;
- suppression de l’idle ;
- runtimes partagés ;
- préchauffage intelligent ;
- choix dynamique du hardware ;
- batching ;
- mutualisation GPU ;
- meilleur placement ;
- utilisation opportuniste de la capacité déjà payée.

La métrique de référence doit être le **coût par unité de travail utile sous contrainte de SLO**, et non simplement le coût brut de l’infrastructure.

---

## 3. Contrat du plugin

Le développeur ne doit pas fournir directement des manifestes Kubernetes arbitraires.

La plateforme expose un contrat abstrait, par exemple :

```yaml
apiVersion: platform.example/v1
kind: Plugin

metadata:
  name: example-plugin

spec:
  source:
    git:
      repository: https://github.com/example/plugin
      revision: main

  interface:
    protocol: grpc
    port: 50051

  runtime:
    mode: auto

  resources:
    mode: auto

  scaling:
    mode: auto

  objectives:
    latency:
      p95: 100ms
    maxErrorRate: 0.1%

  security:
    isolation: auto
```

Le développeur décrit **le besoin fonctionnel**.

La plateforme décide ensuite :

- comment construire l’artefact ;
- quel runtime utiliser ;
- combien de ressources réserver ;
- quelle stratégie de scaling employer ;
- dans quel pool exécuter le plugin ;
- quelle isolation appliquer ;
- quand maintenir ou supprimer des instances chaudes.

---

## 4. Sources et formats de plugins

Plusieurs modes d’entrée doivent être possibles.

### 4.1 Repository Git

Flux :

```text
Git repository
    ↓
build
    ↓
artifact / image OCI / WASM
    ↓
validation
    ↓
benchmark
    ↓
deployment
```

Le repository ne doit pas être cloné dynamiquement dans chaque pod de production.

La construction doit être effectuée en amont afin d’obtenir un artefact :

- reproductible ;
- versionné ;
- hashé ;
- scannable ;
- cachable.

### 4.2 Image OCI

Un utilisateur avancé peut fournir directement une image OCI.

Avantages :

- pas de build ;
- déploiement rapide ;
- digest immuable ;
- meilleure reproductibilité.

### 4.3 Exécutable natif

Un artefact natif peut être exécuté dans un **runner générique** déjà chaud.

L’objectif est de supprimer une grande partie du cold start Kubernetes.

### 4.4 WebAssembly

À plus grande échelle, WASM peut devenir un runtime privilégié pour les plugins compatibles :

- très faible overhead ;
- instanciation extrêmement rapide ;
- sandbox forte ;
- densité élevée ;
- modèle de capabilities explicite.

Un fallback natif doit rester disponible pour les workloads incompatibles avec WASI/WASM.

---

## 5. Lifecycle d’un plugin

Le lifecycle cible est :

```text
soumission
   ↓
build
   ↓
analyse statique / sécurité
   ↓
sandbox
   ↓
détection interface
   ↓
tests fonctionnels
   ↓
tests de charge
   ↓
profil de capacité
   ↓
choix du runtime
   ↓
déploiement
   ↓
observation production
   ↓
retuning automatique
```

Chaque nouvelle version de l’artefact doit produire un nouveau **profil de capacité associé au digest**.

---

## 6. Profilage automatique avant déploiement

La plateforme doit générer ou exécuter automatiquement des tests de charge.

Pour gRPC, l’analyse peut exploiter :

- les `.proto` ;
- gRPC Reflection ;
- la liste des méthodes ;
- le type des RPC :
  - unary ;
  - client streaming ;
  - server streaming ;
  - bidirectional streaming.

### 6.1 Mesures à collecter

Pour chaque configuration testée :

- RPS ;
- concurrence ;
- p50/p95/p99 ;
- taux d’erreur ;
- CPU ;
- working set mémoire ;
- allocations ;
- I/O ;
- bande passante réseau ;
- saturation ;
- temps de démarrage ;
- coût estimé.

Exemple :

```text
1 CPU / 768 MiB

concurrency 4  → 52 RPS   p95 48 ms
concurrency 8  → 98 RPS   p95 72 ms
concurrency 16 → 141 RPS  p95 170 ms
concurrency 32 → 146 RPS  p95 700 ms
```

La plateforme peut en déduire :

- capacité sûre ;
- point de saturation ;
- concurrence cible ;
- intérêt du scaling vertical ;
- intérêt du scaling horizontal ;
- ressources minimales ;
- headroom recommandé.

---

## 7. Capacity Profile

Le résultat des benchmarks doit être matérialisé dans un objet versionné.

Exemple :

```yaml
kind: PluginCapacityProfile

spec:
  imageDigest: sha256:...

  protocol: grpc

  resources:
    cpu: 750m
    memory: 768Mi

  measured:
    safeConcurrency: 12
    saturationConcurrency: 19
    safeRps: 134
    maxRps: 181

  startup:
    processP95: 3ms
    podCachedP95: 400ms
    podUncachedP95: 1.8s
    newNodeP95: 20s

  autoscaling:
    metric: concurrency
    target: 8
    minReplicas: 1
    maxReplicas: 500
```

Ce profil devient une donnée de première classe du scheduler.

---

## 8. Dimensionnement assisté par IA

L’IA ne doit pas être dans la boucle temps réel de l’autoscaling.

### 8.1 Boucle lente : IA / optimiser

Rôle :

- choisir la métrique pertinente ;
- choisir la classe de ressources ;
- évaluer le risque de burst ;
- choisir le headroom ;
- détecter les incohérences de benchmark ;
- proposer une politique de scaling ;
- détecter un drift entre benchmark et production ;
- proposer un retuning.

Un modèle à sortie typée/contrainte de type Jev est adapté à ce rôle.

### 8.2 Boucle rapide : contrôleur déterministe

Rôle :

- mesurer RPS/concurrence/queue ;
- déterminer le nombre d’instances ;
- appliquer immédiatement la politique ;
- ne jamais dépendre d’un LLM.

Architecture :

```text
benchmark + production metrics
           ↓
      Capacity Profile
           ↓
    optimizer / IA
           ↓
      Scaling Policy
           ↓
  KPA / HPA / KEDA
           ↓
        runtime
```

---

## 9. Retuning en production

La configuration initiale n’est qu’une hypothèse.

La plateforme doit observer :

- écart entre RPS prédit et réel ;
- latence ;
- erreurs ;
- CPU ;
- mémoire ;
- queue ;
- temps de démarrage ;
- fréquence des scale-up ;
- durée d’idle ;
- coût.

Exemple :

```text
benchmark:
safe concurrency = 12

production:
safe concurrency ≈ 7
```

La boucle lente peut modifier :

- target concurrency ;
- min replicas ;
- max replicas ;
- headroom ;
- CPU/RAM ;
- stratégie de scale-down ;
- activation scale ;
- classe de runner ;
- type de nœud.

---

## 10. Demande inconnue et prévision

Le benchmark mesure la **capacité** mais ne peut pas prédire l’**attrait du plugin**.

Il faut distinguer :

### Capacity Profile

```text
1 instance = 500 RPS safe
```

### Demand Profile

```text
expected      = 400 RPS
p95           = 1 500 RPS
high case     = 6 000 RPS
confidence    = low
```

Au lancement d’un nouveau plugin, la prévision est nécessairement incertaine.

Avec l’historique, la plateforme peut exploiter :

- trafic passé ;
- plugins similaires ;
- nombre de projets activant le plugin ;
- croissance ;
- saisonnalité ;
- événements de release ;
- historique de bursts.

Une faible confiance doit entraîner une politique plus conservatrice :

- davantage de headroom ;
- scale-down plus lent ;
- éventuellement plusieurs instances chaudes.

---

## 11. Pourquoi le HPA seul ne suffit pas

Un HPA peut décider instantanément :

```text
5 → 500 pods
```

mais cela ne signifie pas que 500 pods sont immédiatement disponibles.

La chaîne réelle est :

```text
autoscaler
   ↓
pods supplémentaires
   ↓
scheduler Kubernetes
   ↓
capacité node disponible ?
   ├── oui → start
   └── non
        ↓
      pods Pending
        ↓
      Karpenter / Cluster Autoscaler
        ↓
      nouvelles machines
        ↓
      boot
        ↓
      réseau
        ↓
      image
        ↓
      readiness
```

Les principaux risques sont :

- latence du provisioning ;
- capacité cloud indisponible ;
- quotas ;
- dépendances qui ne scalent pas ;
- saturation DB/cache ;
- thundering herd ;
- goulots d’étranglement réseau.

La stratégie doit donc être :

```text
buffer
+
autoscaling pods
+
autoscaling nodes
+
headroom
+
backpressure
```

---

## 12. Plugins HTTP/gRPC

gRPC est un protocole naturel pour les appels interservices de la plateforme.

Pour les RPC unary, les métriques principales peuvent être :

- RPS ;
- concurrence ;
- latence ;
- CPU.

Pour le streaming, il faut ajouter :

- nombre de streams actifs ;
- messages/seconde ;
- durée moyenne ;
- CPU par stream ;
- bande passante.

Le simple RPS devient insuffisant pour les streams longs.

---

## 13. Knative, KEDA et HPA

### 13.1 Knative

Approprié pour :

- workloads request-driven ;
- scale-to-zero ;
- HTTP/gRPC ;
- services stateless ;
- gestion de bursts ;
- activation rapide.

Concepts intéressants :

- Activator ;
- buffer ;
- target concurrency ;
- panic mode ;
- activation scale ;
- min/max scale.

### 13.2 KEDA

Approprié pour :

- workers ;
- queues ;
- jobs asynchrones ;
- événements ;
- métriques custom.

KEDA peut scaler selon :

- backlog ;
- Prometheus ;
- Redis ;
- Kafka ;
- autres sources ;
- scaler externe custom.

Un scaler externe gRPC permet d’intégrer un moteur de décision propre à la plateforme.

### 13.3 HPA

Le HPA reste un contrôleur bas niveau utile lorsque l’on veut piloter directement :

- replicas ;
- métriques custom ;
- comportement de montée/descente ;
- fenêtres de stabilisation.

---

## 14. Warm-up et cold start

Un binaire Rust peut démarrer en quelques millisecondes, mais le cold start réel comprend aussi :

- scheduling ;
- CRI ;
- sandbox ;
- CNI ;
- image pull ;
- readiness ;
- propagation du routage.

Il faut mesurer séparément :

```text
process startup
pod startup — image cachée
pod startup — image non cachée
new-node startup
```

### 14.1 Warm-up applicatif

En Rust AOT, il n’existe pas le même besoin de warm-up qu’avec un JIT Java.

Le warm-up applicatif reste utile pour :

- pools DB ;
- DNS ;
- TLS ;
- caches ;
- regex/lazy init ;
- fichiers ;
- modèles ;
- dépendances externes.

La plateforme peut fournir un lifecycle optionnel :

```protobuf
rpc Warmup(WarmupRequest) returns (WarmupResponse);
rpc Health(HealthRequest) returns (HealthResponse);
```

Le benchmark doit déterminer automatiquement si le warm-up apporte réellement un gain.

---

## 15. États de chaleur

Un plugin peut être maintenu dans plusieurs états :

### COLD

- aucun pod ;
- aucun node garanti ;
- coût minimal ;
- latence de démarrage maximale.

### WARM NODE

- node disponible ;
- artefact/image préchargé ;
- aucune instance active.

### WARM SERVICE

- au moins une instance active ;
- dépendances initialisées ;
- réponse quasi immédiate.

### LAUNCH READY

- capacité pré-scalée ;
- plusieurs instances chaudes ;
- large headroom ;
- utilisé pour les lancements prévisibles.

Le Capacity Planner peut choisir automatiquement cet état en fonction :

- du trafic ;
- du coût ;
- de la popularité ;
- des bursts ;
- du SLO ;
- du cold start.

---

## 16. UX en période de surcharge

Une montée en charge peut durer plusieurs secondes voire dizaines de secondes si de nouveaux nœuds doivent être provisionnés.

Cela ne doit pas être présenté comme une panne.

Le gateway peut exposer un état :

```text
QUEUED
SCALING
RUNNING
COMPLETED
```

L’utilisateur peut voir :

> Charge inhabituelle détectée.  
> De nouvelles capacités sont en cours de démarrage.  
> Temps estimé : 5–15 secondes.

La plateforme peut calculer cette estimation à partir :

- du nombre d’instances Ready ;
- du nombre d’instances désirées ;
- des pods Pending ;
- du temps historique de startup ;
- des nœuds en provisioning ;
- de la taille de la queue.

---

## 17. Limite du modèle « un plugin = un pod »

À petite échelle, le modèle suivant est simple :

```text
Plugin A → Deployment A
Plugin B → Deployment B
Plugin C → Deployment C
```

Mais à grande échelle il entraîne :

- énormément de pods ;
- overhead Kubernetes ;
- capacité idle ;
- reservations CPU/RAM trop importantes ;
- multiplication des HPA ;
- cold starts inutiles.

Le modèle cible doit progressivement devenir :

```text
Kubernetes
   ↓
pool de capacité
   ↓
runtime partagé
   ↓
scheduler de plugins
   ↓
instances de plugins
```

---

## 18. Warm Runner Pool

Une première évolution pragmatique consiste à maintenir un pool de pods génériques déjà Ready.

```text
gRPC Gateway
     ↓
Plugin Scheduler
     ↓
┌─────────┬─────────┬─────────┐
│ Runner  │ Runner  │ Runner  │
│ A       │ FREE    │ C       │
└─────────┴─────────┴─────────┘
```

Le runner :

1. reçoit l’affectation d’un plugin ;
2. récupère l’artefact ;
3. vérifie hash/signature ;
4. démarre le process ;
5. effectue le warm-up ;
6. annonce sa readiness ;
7. reçoit le trafic ;
8. est recyclé lorsqu’il devient inutile.

Cela permet de remplacer un cold start Kubernetes par un simple spawn d’exécutable dans un environnement déjà provisionné.

---

## 19. Autoscaling du pool plutôt que du plugin

Le plugin scheduler calcule lui-même :

```text
Plugin A → 12 instances
Plugin B → 3 instances
Plugin C → 0 instance
```

Kubernetes ne voit que :

```text
Runner Pool
```

L’autoscaling Kubernetes se fait alors sur le niveau de remplissage du pool.

Exemple :

```text
50 runners
48 occupés
2 libres

free headroom = 4%
```

Politique :

```text
target free slots = 20%
panic threshold   = 5%
```

Sous le seuil critique :

```text
50 → 70 runners
```

Puis, si les nodes manquent :

```text
Karpenter
   ↓
nouveaux nodes
```

---

## 20. Pools de ressources

Tous les plugins ne doivent pas partager la même classe de runner.

Exemple :

```text
XS   250m CPU / 256 MiB
S    500m CPU / 512 MiB
M    1 CPU / 1 GiB
L    2 CPU / 4 GiB
XL   4 CPU / 8 GiB
```

Le profilage choisit automatiquement la classe.

Un plugin peut ensuite migrer de classe lorsque les mesures production l’exigent.

---

## 21. Routage dynamique

Dans un runtime mutualisé, un plugin ne possède plus nécessairement un Service Kubernetes stable.

Le routeur doit maintenir une table :

```text
plugin-a:
  runner-017:50051
  runner-028:50051
  runner-031:50051
```

Le scheduler publie dynamiquement :

```text
ADD instance
REMOVE instance
DRAIN instance
```

Le gateway gRPC ou un proxy programmable peut ensuite router vers les instances actuellement disponibles.

---

## 22. WebAssembly comme étape de densification

À très grande échelle, le modèle peut évoluer vers :

```text
Node
  ↓
WASM Host
  ├── Plugin A
  ├── Plugin B
  ├── Plugin C
  ├── Plugin D
  └── ...
```

Avantages :

- très forte densité ;
- instanciation rapide ;
- faible mémoire ;
- sandbox ;
- capabilities explicites ;
- mutualisation forte.

Deux runtimes peuvent coexister :

### WASM

Pour les plugins compatibles :

- multi-tenant ;
- très dense ;
- faible cold start.

### Native

Pour les plugins nécessitant :

- syscalls spécifiques ;
- bibliothèques natives ;
- accès matériel ;
- capacités incompatibles WASI.

---

## 23. Sécurité

Le code du plugin doit être considéré comme **non fiable**.

Le système doit empêcher notamment :

- accès aux secrets de la plateforme ;
- accès à d’autres plugins ;
- accès arbitraire au réseau ;
- consommation CPU/RAM illimitée ;
- fork bomb ;
- filesystem arbitraire ;
- escalade de privilèges.

### Stratégies possibles

#### WASM

Isolation par capabilities et sandbox runtime.

#### Native sandbox

Isolation renforcée via :

- gVisor ;
- Kata Containers ;
- microVM ;
- seccomp ;
- cgroups ;
- namespaces ;
- filesystem read-only ;
- network policy.

Le niveau de sécurité peut devenir lui-même un paramètre du scheduler.

---

## 24. Bin packing et consolidation

L’objectif à grande échelle est de maintenir les machines aussi remplies que possible sans compromettre les SLO.

Deux niveaux :

```text
Plugin Scheduler
→ compacte les plugins dans les runtimes

Karpenter
→ compacte les runtimes dans les nodes
```

Exemple :

```text
Node 1  94%
Node 2  91%
Node 3  89%
Node 4  22%
```

Le système peut :

1. migrer/drainer les workloads du Node 4 ;
2. déplacer les plugins ailleurs ;
3. supprimer le Node 4.

L’idle doit être traité comme une inefficacité mesurable.

---

## 25. Plugins GPU / IA

Les workloads GPU doivent utiliser une architecture séparée du pool CPU.

```text
Plugin Scheduler
      │
      ├── CPU/WASM pool
      ├── Native pool
      └── GPU Scheduler
```

Le GPU scheduler doit tenir compte de :

- VRAM ;
- architecture GPU ;
- compute ;
- modèle chargé ;
- localité du modèle ;
- batchabilité ;
- temps de chargement ;
- durée estimée ;
- priorité ;
- coût.

---

## 26. Mutualisation GPU

Le modèle à éviter est :

```text
1 plugin IA = 1 GPU dédié
```

Les stratégies préférées sont :

### Shared model serving

Plusieurs plugins utilisent le même modèle partagé.

```text
Plugin A ─┐
Plugin B ─┼── shared inference service ── GPU
Plugin C ─┘
```

### Dynamic batching

Plusieurs requêtes compatibles sont regroupées :

```text
req ┐
req ├── batch ── GPU
req ┤
req ┘
```

### Model locality

Favoriser un GPU sur lequel le modèle est déjà chargé.

### Adapters / LoRA

Partager le modèle de base et charger uniquement les adapters nécessaires lorsque c’est possible.

### MIG / partitionnement

Pour certains workloads, partitionner physiquement/logiquement un gros GPU afin d’améliorer :

- isolation ;
- densité ;
- prévisibilité.

---

## 27. Scheduling GPU basé sur backlog

Contrairement aux services CPU interactifs, les tâches GPU peuvent être accumulées jusqu’à atteindre une masse critique.

Chaque job peut comporter :

```text
predicted GPU time
VRAM required
model
batch class
priority
deadline
```

Le scheduler calcule :

```text
queued GPU work
vs
cost of spawning hardware
```

Une machine n’est lancée que lorsque :

- le backlog justifie économiquement le spawn ;
- une tâche prioritaire l’exige ;
- un SLA l’impose.

Une fois la machine lancée, toute capacité inutilisée peut servir à vider les jobs basse priorité compatibles.

---

## 28. Backfilling GPU

Lorsqu’un workload prioritaire provoque l’ouverture d’une machine GPU, les ressources inutilisées peuvent être utilisées pour exécuter d’autres tâches.

Exemple :

```text
GPU
 ├── training LoRA prioritaire
 └── capacité libre
       ↓
    generations image
    jobs basse priorité
```

Le scheduler doit empêcher le backfill de détériorer significativement le workload principal.

Les tests doivent mesurer :

- VRAM restante ;
- compute restant ;
- impact sur la latence ;
- impact sur la durée du training.

---

## 29. Provider GPU dynamique

À petite échelle, les GPU peuvent être loués via un provider serverless/externe.

À plus grande échelle :

```text
faible / imprévisible
→ GPU serverless externe

stable
→ instances réservées / engagées

très stable / très grosse charge
→ infrastructure dédiée
```

Le plugin ne doit pas demander :

```text
je veux une L40S
```

mais plutôt :

```text
VRAM >= 24 GiB
latency = interactive
batchable = true
```

Le scheduler choisit le hardware.

---

## 30. Observabilité

L’observabilité est indispensable au contrôle automatique.

Mesures globales :

- CPU utilisé / réservé ;
- RAM utilisée / réservée ;
- GPU utilization ;
- VRAM ;
- RPS ;
- queue depth ;
- p95/p99 ;
- erreurs ;
- startup latency ;
- nombre de scale events ;
- coût réel ;
- capacité idle.

Mesures par plugin :

```text
cost / request
CPU-seconds / request
GB-seconds / request
GPU-seconds / job
working set
safe concurrency
burst history
```

---

## 31. Mesure des économies

La plateforme doit maintenir une baseline théorique :

```text
BASELINE
architecture Kubernetes conventionnelle
```

et comparer :

```text
OPTIMIZED
runtime partagé + right-sizing + pooling + scale-to-zero
```

Exemple de métriques :

```text
CPU-hours avoided
GB-hours avoided
GPU-hours avoided
nodes avoided
idle percentage
cost / million requests
cost / 1000 generations
```

L’objectif est de pouvoir démontrer :

> À SLO et charge identiques, la plateforme consomme X % moins de CPU, Y % moins de RAM et Z % moins de GPU qu’un déploiement conventionnel.

---

## 32. Architecture cible

```text
                           CONTROL PLANE
                 ┌────────────────────────────┐
                 │ Plugin Registry            │
                 │ Build / Artifact Manager   │
                 │ Capacity Planner           │
                 │ AI Policy Tuner            │
                 │ Plugin Scheduler           │
                 │ Telemetry / Cost Engine    │
                 │ Security Policies          │
                 └──────────────┬─────────────┘
                                │
                ┌───────────────┼────────────────┐
                │               │                │
                ▼               ▼                ▼
          WASM Runtime     Native Runners    GPU Scheduler
                │               │                │
                └───────────────┼────────────────┘
                                ▼
                         Kubernetes
                                │
                         HPA / KEDA
                                │
                           Karpenter
                                │
                     physical/cloud nodes
```

---

## 33. Architecture V1 pragmatique

Pour éviter de construire trop tôt une infrastructure complexe :

```text
V1
──
Kubernetes
+ OCI/native plugins
+ warm runner pools
+ gRPC gateway
+ scheduler simple
+ benchmarks automatiques
+ HPA/KEDA
+ Karpenter
+ sandbox native
```

Cette V1 doit déjà conserver les abstractions nécessaires pour évoluer ensuite sans casser le contrat plugin.

---

## 34. Trajectoire d’évolution

### Étape 1 — V1

```text
Plugin → runner Kubernetes
```

- architecture simple ;
- sécurité ;
- benchmarking ;
- scaling automatique.

### Étape 2 — Pooling

```text
Plugin → warm runner pool
```

- suppression d’une grande partie du cold start ;
- headroom partagé ;
- meilleur taux d’utilisation.

### Étape 3 — Scheduler propriétaire

```text
plugin scheduler
→ placement dynamique
→ bin packing
→ retuning
```

### Étape 4 — WASM

```text
shared hosts
→ très forte densité
→ scale-to-zero massif
```

### Étape 5 — Infrastructure optimisée

```text
predictive capacity
heterogeneous nodes
dynamic provider selection
GPU batching
model locality
automated FinOps
```

---

## 35. Principes d’architecture à conserver

1. **Plugin != Pod.**
2. **Le plugin décrit ses besoins, pas l’infrastructure.**
3. **Le runtime doit être interchangeable.**
4. **Le benchmark produit une politique d’exploitation.**
5. **L’IA règle les politiques ; elle ne contrôle pas la boucle temps réel.**
6. **L’idle est une inefficacité mesurable.**
7. **Le headroom doit être partagé autant que possible.**
8. **Les GPU doivent être batchés et mutualisés.**
9. **La sécurité doit permettre davantage de densité, pas seulement ajouter du coût.**
10. **Kubernetes gère la capacité physique ; le scheduler de plugins gère la capacité logique.**
11. **Le système doit pouvoir évoluer d’une petite infrastructure peu coûteuse vers une plateforme serverless spécialisée sans réécrire le contrat utilisateur.**
12. **Les économies doivent être mesurées expérimentalement à SLO identique.**

---

## 36. Questions ouvertes / prochaines découvertes

- Choix précis du runtime WASM :
  - Wasmtime direct ;
  - wasmCloud ;
  - Spin/SpinKube ;
  - autre runtime.
- Frontière exacte entre scheduler maison et KEDA/Knative.
- Format définitif du `Plugin` CRD.
- Format du `CapacityProfile`.
- Stratégie de sandbox native :
  - gVisor ;
  - Kata ;
  - microVM.
- Design du routing gRPC dynamique.
- Méthode de cache et distribution des artefacts.
- Politique de headroom globale.
- Modèle mathématique de placement/bin packing.
- Métriques de coût de référence.
- Benchmark reproductible permettant de mesurer le gain réel par rapport à une architecture Kubernetes conventionnelle.
- Architecture GPU :
  - Triton ;
  - vLLM ;
  - runtimes spécialisés ;
  - MIG/DRA ;
  - provider externe versus capacité propre.
- Méthode de migration d’un plugin entre classes de runners.
- Gestion des streams gRPC longs lors d’un drain/migration.
- Définition des SLO et des modes de dégradation contrôlée.
