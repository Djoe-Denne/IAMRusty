# Discovery Addendum — Capacity Intelligence & Decision Ledger

## 1. Objet du document

Ce document complète la discovery précédente consacrée à l’infrastructure de management des plugins.

Il se concentre uniquement sur les éléments ajoutés depuis :

- choix d’une approche pragmatique pour le **dimensionnement automatique** ;
- comparaison entre modèles de type **Jev/Laya** et modèles ML plus classiques ;
- choix proposé pour une V1 : **KEDA + External Scaler gRPC + logique déterministe + LightGBM** ;
- rôle futur possible d’un modèle de décision spécialisé de type Jev/Laya ;
- conception d’un **Decision Ledger** append-only ;
- collecte des données nécessaires pour :
  - auditer les décisions ;
  - comparer les politiques ;
  - entraîner ultérieurement un modèle spécialisé ;
  - mesurer les conséquences économiques et opérationnelles de chaque décision ;
  - tester de nouveaux modèles en mode shadow.

L’objectif est de donner à l’architecte une base suffisamment précise pour proposer une architecture ciblée, sans figer trop tôt le choix technologique final.

---

# 2. Problème à résoudre

L’infrastructure doit prendre régulièrement des décisions de capacité :

- augmenter ou réduire le nombre de runners ;
- modifier le headroom ;
- changer une classe CPU/RAM ;
- choisir un type de GPU ;
- décider de provisionner ou non une nouvelle machine ;
- choisir une durée de réservation ;
- décider si un backlog GPU justifie économiquement un spawn ;
- maintenir ou supprimer de la capacité chaude ;
- choisir entre plusieurs providers ou classes de hardware ;
- modifier une politique de scaling après observation de la production.

Ces décisions ont plusieurs objectifs parfois contradictoires :

```text
minimiser le coût
+
maintenir les SLO
+
éviter les saturations
+
réduire l’idle
+
absorber les bursts
+
éviter le surdimensionnement
```

Le système doit donc distinguer :

1. les décisions **mathématiques / déterministes** ;
2. les décisions **prédictives** ;
3. les décisions **probabilistes ou floues**.

---

# 3. Principe général retenu

La philosophie proposée est :

> Ne pas utiliser un modèle complexe là où une formule ou un modèle tabulaire simple suffit.

L’architecture cible est :

```text
                  Load tests
                      │
                      ▼
              Capacity Profile
                      │
          ┌───────────┴───────────┐
          │                       │
          ▼                       ▼
   logique déterministe      modèle prédictif
        Rust                 LightGBM / autre
          │                       │
          └───────────┬───────────┘
                      ▼
               Scaling Policy
                      │
                      ▼
         External Scaler gRPC
                      │
                      ▼
                    KEDA
                      │
                      ▼
                    HPA
```

Un modèle de type Jev/Laya peut venir plus tard en complément pour les décisions réellement ambiguës.

---

# 4. Choix proposé pour la V1

## 4.1 KEDA

KEDA est retenu comme candidat principal pour la couche d’exécution du scaling.

Rôles :

- transformer les métriques en désir de replicas ;
- gérer le scale-to-zero ;
- utiliser des métriques custom ;
- piloter les workers / runners ;
- permettre l’utilisation d’un External Scaler.

L’intérêt principal est de conserver Kubernetes comme moteur de scaling tout en permettant au projet de fournir sa propre intelligence de décision.

---

## 4.2 External Scaler gRPC

Le projet étant déjà orienté gRPC, un **External Scaler maison en Rust** est une bonne frontière d’architecture.

Exemple :

```text
Capacity Controller
       │
       │ gRPC
       ▼
KEDA External Scaler
       │
       ▼
      HPA
```

Le contrôleur maison peut exposer une métrique synthétique ou une décision issue :

- d’un benchmark ;
- d’un forecast ;
- d’un modèle ML ;
- de règles métier ;
- d’un calcul coût/SLO.

Il ne doit pas nécessairement décider lui-même du nombre final de pods si KEDA/HPA peut exécuter la politique.

---

# 5. LightGBM comme candidat de prédiction

## 5.1 Pourquoi un modèle tabulaire est adapté

Le problème de capacité est largement tabulaire.

Exemples de features :

```text
CPU
RAM
RPS
concurrency
p50 / p95 / p99
error rate
startup time
queue depth
working set
payload size
replica count
traffic trend
hour/day
resource class
provider
hardware
```

Les sorties possibles sont également structurées :

```text
safe capacity per replica
expected traffic
recommended resource class
burst risk
expected latency
expected cost
```

Ce type de problème est naturellement adapté à des modèles comme :

- LightGBM ;
- XGBoost ;
- modèles de régression ;
- modèles de séries temporelles simples.

---

## 5.2 Pourquoi LightGBM est un bon choix pragmatique

Avantages recherchés :

- simple ;
- mature ;
- léger ;
- CPU-only ;
- très faible coût d’inférence ;
- facile à entraîner régulièrement ;
- adapté à des datasets tabulaires ;
- facile à interpréter relativement à un LLM ;
- compatible avec un système fortement optimisé en ressources.

Le modèle ne doit pas remplacer les formules.

Exemple :

```text
LightGBM prédit :

safe_capacity_per_replica = 410 RPS
expected_traffic_5m       = 3200 RPS
burst_risk                = 0.78
```

Puis le code Rust calcule :

```text
required =
ceil(
  expected_traffic
  × headroom
  / safe_capacity_per_replica
)
```

Le calcul reste déterministe.

---

# 6. Forecasting

Pour le forecast de trafic, la V1 peut rester volontairement simple.

Ordre recommandé de complexité :

```text
EWMA
↓
Holt-Winters
↓
régression / modèle tabulaire
↓
LightGBM
↓
modèle plus complexe seulement si nécessaire
```

Le système doit démontrer qu’un modèle complexe apporte un gain avant de l’adopter.

La baseline doit rester forte et peu coûteuse.

---


# 6bis. Economic Capacity Governance

## 6bis.1 Objectif

La chaîne de décision doit intégrer dès la V1 une contrainte économique simple.

L’objectif n’est pas encore de construire un moteur FinOps prédictif complet.

Pour la V1, le système doit uniquement répondre à la question :

> **La capacité supplémentaire demandée reste-t-elle compatible avec le budget infrastructure restant ?**

La contrainte économique intervient **après le calcul du besoin technique** et avant l’application effective du scaling.

Architecture V1 :

```text
besoin technique
      │
      ▼
Capacity Controller
      │
      ▼
Economic Guard
      │
      ├── coût du nœud
      ├── budget mensuel
      └── budget déjà consommé
      │
      ▼
autorisation de scale
      │
      ▼
KEDA / HPA / Karpenter
```

---

## 6bis.2 Les trois inputs économiques de la V1

La V1 ne nécessite que trois informations.

### 1. Coût du nœud

Le système connaît le coût de chaque classe de nœud disponible.

Exemple :

```text
node_type: cpu-medium
cost_per_hour: 0.42 €

node_type: cpu-large
cost_per_hour: 0.81 €

node_type: gpu-l40s
cost_per_hour: 1.82 €
```

Le stockage peut être réalisé dans :

- une table SQL ;
- un fichier de configuration ;
- un registry de hardware interne.

La granularité interne peut être :

```text
€/heure
```

ou :

```text
€/minute
```

Le système peut convertir automatiquement l’une vers l’autre.

---

### 2. Budget mensuel infrastructure

Exemple :

```text
monthly_infra_budget = 3 000 €
```

Cette valeur constitue la limite budgétaire globale utilisée par le contrôleur.

Elle peut être définie manuellement en V1.

Aucune prévision de revenu n’est nécessaire.

---

### 3. Budget déjà consommé

Exemple :

```text
monthly_infra_budget = 3 000 €
spent_this_month      = 1 740 €
```

Le budget restant est donc :

```text
remaining_budget =
monthly_infra_budget
-
spent_this_month
```

Dans l’exemple :

```text
remaining_budget = 1 260 €
```

---

## 6bis.3 Décision V1

Le Capacity Controller calcule d’abord le besoin technique.

Exemple :

```text
current nodes: 4

technical decision:
+2 cpu-large nodes
```

Le Economic Guard calcule ensuite le coût supplémentaire.

Exemple :

```text
cpu-large:
0.81 €/h

2 nodes:
1.62 €/h
```

La V1 peut utiliser une durée minimale de projection configurable.

Exemple :

```text
minimum_capacity_window = 30 minutes
```

Le coût de la décision devient :

```text
1.62 €/h × 0.5 h
=
0.81 €
```

Le contrôle devient alors :

```text
if decision_cost <= remaining_budget:
    allow_scale
else:
    deny_scale
```

La logique reste volontairement simple.

---

## 6bis.4 Chaîne de décision V1

```text
                       METRICS
                          │
                          ▼
                 Technical Capacity
                          │
                          ▼
                  Scaling Candidate
                          │
                          ▼
                   Economic Guard
                          │
             ┌────────────┴────────────┐
             │                         │
             ▼                         ▼
      budget disponible          budget insuffisant
             │                         │
             ▼                         ▼
          SCALE                    TEMPORISE
             │                         │
             ▼                         ▼
        KEDA / HPA                 queue / backpressure
```

Pour la V1, la sortie économique peut être binaire :

```text
ALLOW_SCALE
DENY_SCALE
```

Une troisième sortie peut être ajoutée si utile :

```text
ALLOW_PARTIAL_SCALE
```

Exemple :

```text
besoin technique:
+10 nodes

budget disponible:
seulement +4 nodes

résultat:
ALLOW_PARTIAL_SCALE(4)
```

---

## 6bis.5 Comportement lorsque le budget bloque le scaling

Lorsque :

```text
technical_capacity
>
economically_allowed_capacity
```

la plateforme ne doit pas continuer à scaler sans limite.

Elle doit passer dans un mode de dégradation contrôlée.

Selon le type de workload :

### Services interactifs

```text
admission control
→ queue courte
→ backpressure
→ message UI
```

Exemple d’état :

```text
QUEUED_CAPACITY_LIMIT
```

### Jobs asynchrones

```text
queue
→ attente de capacité disponible
```

### GPU

```text
backlog
→ attente d’une masse critique
→ nouveau spawn lorsque le budget le permet
```

La V1 n’a pas besoin d’un moteur complexe pour cela.

---

## 6bis.6 Données économiques dans le Decision Ledger

Chaque décision de scaling doit enregistrer les trois inputs économiques.

Exemple :

```json
{
  "economics": {
    "node_type": "cpu-large",
    "node_cost_per_hour": 0.81,

    "monthly_budget": 3000.0,
    "spent_this_month": 1740.0,
    "remaining_budget": 1260.0,

    "decision_estimated_cost": 0.81
  }
}
```

Le ledger doit également stocker :

```text
economic_decision:
ALLOW_SCALE
```

ou :

```text
economic_decision:
DENY_SCALE
```

Cela permettra plus tard d’analyser :

- combien de décisions ont été contraintes par le budget ;
- combien de requêtes ont été mises en queue ;
- le coût des décisions autorisées ;
- les effets d’un budget plus ou moins élevé.

---

## 6bis.7 V1 : règle volontairement conservatrice

La V1 ne cherche pas à optimiser la totalité du mois.

Elle cherche uniquement à éviter :

```text
autoscaling incontrôlé
→ facture incontrôlée
```

La règle de base est donc :

```text
technical demand
        ↓
estimated marginal cost
        ↓
remaining monthly budget
        ↓
scale / partial scale / queue
```

Cette logique doit rester :

- explicable ;
- testable ;
- déterministe ;
- auditable.

---

# 6ter. Evolution future de la gouvernance économique

La V1 doit conserver une interface suffisamment générique pour permettre une politique plus riche ultérieurement.

La future version pourra remplacer les trois inputs simples par un objet économique plus complet.

Exemple :

```yaml
economicPolicy:
  revenue:
    realized: ...
    committed: ...
    forecastP10: ...
    forecastP50: ...

  infrastructure:
    monthlyBudget: ...
    spent: ...
    projectedSpend: ...

  reserve:
    burst: ...

  priorities:
    paid: guaranteed
    standard: best-effort
    free: deferred
```

---

## 6ter.1 Revenus

Une évolution future pourra intégrer :

```text
revenus réalisés
revenus prépayés / engagés
prévision de revenus
```

afin de calculer dynamiquement une enveloppe infrastructure.

Exemple conceptuel :

```text
allowed_infra_budget =
effective_revenue
×
infra_ratio
```

La V1 ne dépend pas de cette mécanique.

---

## 6ter.2 Budget élastique

La version avancée pourra distinguer :

```text
base budget
elastic budget
burst reserve
```

Exemple :

```text
3 000 € / mois

1 100 € base
1 400 € elastic
  500 € burst reserve
```

Cela permettra de dépasser temporairement la consommation horaire moyenne lors d’un rush tout en respectant l’objectif mensuel.

---

## 6ter.3 Coût marginal

La version avancée pourra raisonner sur le coût de la prochaine unité de capacité.

Exemple :

```text
runner supplémentaire
sur node existant
→ coût marginal ≈ faible

runner suivant
nécessite un nouveau node
→ coût marginal élevé
```

Le contrôleur pourra donc différencier :

```text
scale gratuit ou quasi gratuit
```

et :

```text
scale déclenchant une nouvelle dépense
```

---

## 6ter.4 Valeur / revenu marginal

Une évolution ultérieure pourra intégrer :

```text
expected additional revenue
-
additional infrastructure cost
```

pour décider s’il est rationnel de scaler.

Cette logique est particulièrement pertinente pour :

- jobs payants ;
- GPU ;
- tâches sponsorisées ;
- services avec niveaux de priorité.

---

## 6ter.5 Politique économique future

La sortie ne sera plus seulement :

```text
ALLOW
DENY
```

mais potentiellement :

```text
SCALE
PARTIAL_SCALE
QUEUE
DEGRADE
REJECT
USE_BURST_RESERVE
SELECT_CHEAPER_HARDWARE
SELECT_DIFFERENT_PROVIDER
```

---

## 6ter.6 Intégration ML / System-One

Les données économiques pourront plus tard devenir des features pour :

```text
LightGBM
Laya
Capacity-Jev
autre policy model
```

Exemple :

```text
State:
traffic
SLO
capacity
node prices
remaining budget
forecast
priority

Decision:
SCALE
QUEUE
DEGRADE
```

Le modèle ne doit toutefois jamais contourner les contraintes financières absolues.

Une limite budgétaire dure reste une contrainte déterministe.

---

## 6ter.7 Invariant d’architecture

Le système doit toujours conserver la séparation suivante :

```text
Technical Capacity Planner
          │
          ▼
Economic Capacity Policy
          │
          ▼
Execution Layer
```

Ainsi :

- la capacité technique répond à **« combien faudrait-il ? »** ;
- la politique économique répond à **« combien voulons-nous / pouvons-nous payer ? »** ;
- KEDA/HPA/Karpenter répondent à **« comment l’exécuter ? »**.

Cette séparation permet de commencer avec trois valeurs très simples en V1 tout en conservant la possibilité d’évoluer vers un véritable contrôleur économique adaptatif.


# 7. Place des modèles Jev / Laya / System-One

## 7.1 Conclusion actuelle

Aucun modèle open-weight mature et largement adopté n’a été identifié comme solution prête à l’emploi pour :

```text
profil de benchmark
→ politique HPA optimale
```

Les modèles de type Jev/Laya sont néanmoins intéressants pour une évolution ultérieure.

---

## 7.2 Pourquoi ils ne sont pas retenus comme cœur V1

Raisons :

- adoption publique encore limitée ;
- modèles très récents ;
- coût et complexité inutiles si des règles simples suffisent ;
- absence de preuve qu’ils battent une baseline tabulaire sur ce problème précis ;
- nécessité de disposer d’un dataset spécifique au projet.

---

## 7.3 Cas d’usage potentiel futur

Un modèle spécialisé pourrait décider :

```text
Choice:
  metric =
    CONCURRENCY
    RPS
    CPU
    QUEUE_DEPTH

Choice:
  warm_strategy =
    COLD
    WARM_NODE
    WARM_SERVICE

Choice:
  resource_class =
    XS
    S
    M
    L
    XL

Score:
  burst_risk

Score:
  confidence

Noul:
  cpu_bound

Noul:
  horizontal_scaling_preferred
```

Le modèle ne produirait pas directement du YAML Kubernetes.

Il produirait une **Policy structurée**, compilée ensuite par du code déterministe.

---

# 8. Objectif futur : Capacity-Jev / Capacity-Laya

Une piste R&D possible est de construire un modèle spécialisé très petit.

Le dataset serait produit principalement par la plateforme elle-même :

```text
plugins
×
benchmarks
×
resource configurations
×
traffic shapes
×
SLO
×
production outcomes
```

Le système pourrait accumuler des millions de couples :

```text
state
→ candidates
→ decision
→ outcome
```

Ce dataset pourrait permettre d’entraîner un modèle spécialisé beaucoup plus petit qu’un LLM généraliste.

---

# 9. Decision Ledger

## 9.1 Principe

Toutes les décisions de capacité importantes doivent être journalisées dans un **Decision Ledger append-only**.

Ce ledger n’est pas un simple système de logs.

Il constitue :

1. une piste d’audit ;
2. une source d’analyse FinOps ;
3. une base de comparaison entre politiques ;
4. un futur dataset ML ;
5. une base d’évaluation de modèles shadow.

Le format conceptuel est :

```text
STATE
  ↓
CANDIDATES
  ↓
DECISION
  ↓
ACTION
  ↓
OUTCOME
  ↓
REWARD / LABEL
```

---

# 10. Types de décisions à journaliser

Exemples :

```text
PLUGIN_SCALE
RUNNER_POOL_SCALE
NODE_SCALE
GPU_CAPACITY_PURCHASE
GPU_LEASE_EXTENSION
RESOURCE_CLASS_CHANGE
WARM_STATE_CHANGE
PROVIDER_SELECTION
GPU_HARDWARE_SELECTION
HEADROOM_CHANGE
MAX_REPLICA_CHANGE
TARGET_CONCURRENCY_CHANGE
VERTICAL_RESIZE
PLUGIN_MIGRATION
```

Chaque décision importante doit être représentée de manière explicite.

---

# 11. Structure canonique d’une décision

Exemple conceptuel :

```text
DecisionRecord
├── identity
│   ├── decision_id
│   ├── timestamp
│   └── decision_type
│
├── subject
│   ├── project_id
│   ├── plugin_id
│   └── plugin_version
│
├── context
│   ├── raw_features
│   ├── derived_features
│   ├── infrastructure_state
│   └── traffic_state
│
├── constraints
│
├── candidate_actions[]
│
├── decision
│   ├── chosen_action
│   ├── selection_probability
│   ├── policy_version
│   ├── model_versions[]
│   └── reason_codes[]
│
├── predicted_outcome
│
├── actual_outcomes[]
│
└── reward
```

Chaque enregistrement doit comporter :

```text
schema_version
```

dès la V1.

---

# 12. Exemple — décision GPU

```json
{
  "decision_id": "dec_01",
  "decision_type": "GPU_CAPACITY_PURCHASE",

  "context": {
    "queue_jobs": 1842,
    "estimated_gpu_seconds": 5284,
    "oldest_job_seconds": 821,
    "paid_jobs": 12,

    "current_gpu_count": 1,
    "gpu_utilization": 0.91,
    "free_vram_gb": 4.2
  },

  "constraints": {
    "max_cost_eur_hour": 9.0,
    "max_paid_wait_seconds": 30,
    "required_vram_gb": 24
  },

  "candidates": [
    {
      "action": "DO_NOTHING",
      "estimated_cost": 0
    },
    {
      "action": "SPAWN_L40S",
      "duration_minutes": 30,
      "estimated_cost": 0.94
    },
    {
      "action": "SPAWN_H100",
      "duration_minutes": 30,
      "estimated_cost": 1.87
    }
  ],

  "decision": {
    "action": "SPAWN_L40S",
    "policy": "capacity-policy-v7",
    "reason_codes": [
      "QUEUE_ABOVE_PROFITABILITY_THRESHOLD",
      "PAID_SLA_AT_RISK"
    ]
  }
}
```

---

# 13. Logging des options non choisies

Il est essentiel de journaliser les alternatives disponibles.

Mauvais dataset :

```text
state
→ SPAWN_L40S
```

Meilleur dataset :

```text
state

candidates:
  DO_NOTHING
  L4
  L40S
  H100

chosen:
  L40S

outcome:
  ...
```

Cela permet au futur modèle de raisonner sur un espace de choix.

---

# 14. Probabilité de sélection

Le schéma doit prévoir :

```text
selection_probability
```

même si la politique V1 est déterministe.

Exemple V1 :

```text
probability = 1.0
```

Exemple futur :

```text
probability = 0.83
```

Cela permet plus tard :

- contextual bandits ;
- exploration contrôlée ;
- off-policy evaluation ;
- comparaison de politiques.

---

# 15. Raw Features vs Derived Features

Le ledger doit conserver les données brutes.

Exemple :

```json
{
  "raw": {
    "queue_jobs": 1842,
    "queue_gpu_seconds": 5284,
    "gpu_utilization": 0.91
  },

  "derived": {
    "queue_pressure": 0.78,
    "profitability_ratio": 1.47,
    "sla_risk": 0.83
  }
}
```

Raison :

Les derived features peuvent être mal conçues ou changer avec le temps.

Les raw features permettent de recalculer les features historiques avec une nouvelle méthode.

---

# 16. Reason Codes

La décision doit être expliquée par des codes structurés.

Exemple :

```text
PAID_SLA_AT_RISK
QUEUE_ABOVE_THRESHOLD
L40S_BEST_COST_PER_JOB
CAPACITY_HEADROOM_LOW
HIGH_BURST_RISK
MODEL_ALREADY_WARM
```

Un champ textuel peut exister en complément, mais ne doit jamais remplacer les reason codes structurés.

---

# 17. Versioning

Chaque décision doit permettre de reconstruire exactement le système qui l’a produite.

À versionner :

```text
policy_version
feature_schema_version
capacity_profile_version
forecast_model_version
decision_model_version
scheduler_version
pricing_snapshot_id
runtime_version
```

Exemple :

```json
{
  "models": {
    "capacity": "lightgbm-capacity-v17",
    "traffic": "holt-winters-v4"
  }
}
```

---

# 18. Pricing Snapshot

Le contexte économique doit être historisé.

Exemple :

```json
{
  "pricing": {
    "provider": "provider-X",
    "region": "eu-west",

    "L4_eur_h": 0.42,
    "L40S_eur_h": 1.81,
    "H100_eur_h": 3.42,

    "snapshot_timestamp": "..."
  }
}
```

Une décision passée doit être évaluée avec les prix connus à l’époque, et non avec les prix actuels.

---

# 19. Outcome Logging

Une décision doit être évaluée après exécution.

Exemple :

```json
{
  "decision_id": "dec_01",

  "observed": {
    "startup_seconds": 14.2,
    "gpu_utilization_mean": 0.88,
    "jobs_completed": 1694,
    "paid_p95_wait_seconds": 18.3,
    "actual_cost_eur": 0.91,
    "sla_violations": 0
  }
}
```

Ces observations servent de labels.

---

# 20. Multi-Horizon Outcomes

Une décision peut être bonne à court terme mais mauvaise à long terme.

Exemple :

```text
T+10 s
queue diminue

T+1 min
SLO respecté

T+10 min
GPU sous-utilisé

T+1 h
coût final trop élevé
```

Le ledger doit donc permettre plusieurs outcomes.

Exemple :

```text
IMMEDIATE
SHORT_TERM
OPERATIONAL
ECONOMIC
```

---

# 21. Reward

Une reward composite peut être calculée.

Exemple :

```text
reward =
  SLA_score
+ cost_efficiency
+ utilization
+ queue_reduction
- idle_penalty
- error_penalty
```

Le reward doit rester décomposé.

Exemple :

```json
{
  "reward": 0.87,

  "components": {
    "sla": 1.0,
    "cost_efficiency": 0.93,
    "utilization": 0.88,
    "queue_reduction": 0.82
  }
}
```

Cela évite de perdre l’information derrière un score unique.

---

# 22. Shadow Decisions

Un futur modèle ne doit pas être promu immédiatement en production.

Le système doit permettre :

```text
Production policy:
SPAWN_L40S

LightGBM shadow:
SPAWN_L40S

Laya shadow:
SPAWN_H100
confidence = 0.62
```

Les décisions shadow sont journalisées mais jamais appliquées.

Cela permet ensuite de comparer :

```text
policy actuelle
vs
nouveau modèle
```

sans prise de risque.

---

# 23. Corrélation avec l’observabilité

Chaque décision doit disposer d’un identifiant global :

```text
decision_id
```

Cet identifiant doit être propagé dans :

- logs ;
- traces ;
- jobs ;
- provisioning ;
- billing ;
- metrics ;
- scheduler events.

Exemple :

```text
Decision D4287
  ├── scheduler
  ├── Karpenter
  ├── instance cloud
  ├── runner
  ├── jobs
  └── termination
```

L’objectif est de pouvoir reconstruire intégralement la causalité d’une décision.

---

# 24. Stockage V1 proposé

L’architecture de stockage doit rester simple.

```text
                  Decision Service
                       │
           ┌───────────┴───────────┐
           ▼                       ▼
      PostgreSQL               Object Storage
     opérationnel               historique
                                Parquet
```

### PostgreSQL

Pour :

- inspection récente ;
- audit ;
- debugging ;
- dashboards ;
- décision en cours.

### Object Storage / Parquet

Pour :

- historique long ;
- datasets ML ;
- réentraînement ;
- analyses massives ;
- archivage immutable.

Il n’est pas nécessaire d’introduire immédiatement :

- ClickHouse ;
- Iceberg ;
- Delta Lake ;
- gros pipeline data.

Ces outils pourront être ajoutés uniquement si les volumes le justifient.

---

# 25. Training Dataset futur

Avec cette instrumentation, le projet pourra accumuler :

```text
State
+
Candidate Actions
+
Chosen Action
+
Selection Probability
+
Predicted Outcome
+
Observed Outcome
+
Reward
```

C’est exactement le format nécessaire pour plusieurs approches futures :

- supervised learning ;
- ranking ;
- contextual bandits ;
- reinforcement learning offline ;
- Jev/Laya-like ;
- policy learning.

---

# 26. Stratégie d’évolution du moteur de décision

## Phase 1 — Rules

```text
règles
+
calculs déterministes
```

Objectif :

- comportement explicable ;
- créer des données propres.

---

## Phase 2 — Forecasting simple

```text
EWMA
Holt-Winters
```

Objectif :

- anticipation basique des bursts.

---

## Phase 3 — LightGBM

```text
capacity prediction
traffic prediction
resource classification
burst risk
```

Objectif :

- apprendre les relations non linéaires.

---

## Phase 4 — Shadow models

Tester :

```text
Laya
Capacity-Jev
RL
autres modèles
```

sans contrôle production.

---

## Phase 5 — Hybrid Policy

```text
Rules
+
optimizer
+
LightGBM
+
System-One model
```

Chaque composant est utilisé uniquement pour le type de décision où il apporte une amélioration mesurée.

---

# 27. Principes d’architecture

1. **Chaque décision doit être observable.**
2. **Chaque décision doit être reproductible.**
3. **Les alternatives non choisies doivent être conservées.**
4. **Les données brutes doivent être conservées.**
5. **Les derived features ne doivent jamais remplacer les raw features.**
6. **Les prix doivent être historisés.**
7. **Les résultats doivent être mesurés sur plusieurs horizons.**
8. **Les modèles doivent être versionnés.**
9. **Un nouveau modèle doit d’abord tourner en shadow.**
10. **La V1 doit privilégier les solutions simples.**
11. **Le ML n’est utilisé que lorsqu’il bat une baseline déterministe.**
12. **Le Decision Ledger constitue un actif stratégique du projet.**

---

# 28. Questions à faire traiter par l’architecte

## Architecture du Decision Ledger

- Event sourcing complet ou simple append-only ?
- PostgreSQL JSONB ou schéma relationnel fortement typé ?
- stratégie de partitionnement ;
- rétention ;
- export Parquet ;
- immutabilité ;
- idempotence ;
- ordering.

## Feature Store

- faut-il un feature store dès la V1 ?
- quelles raw features sont indispensables ?
- quelles features doivent être calculées online ?
- quelles features peuvent être calculées offline ?

## Model lifecycle

- où stocker les modèles ?
- comment gérer leur version ?
- comment déployer LightGBM depuis Rust ?
- faut-il un service modèle séparé ou embarqué ?
- comment comparer deux policies ?

## Reward

- quelle fonction de reward ?
- comment pondérer :
  - SLA ;
  - coût ;
  - idle ;
  - throughput ;
  - délai ?
- reward spécifique CPU versus GPU ?

## Shadow execution

- fréquence d’évaluation ;
- nombre maximal de shadow policies ;
- calcul de counterfactual ;
- critères de promotion.

## Data Governance

- anonymisation ;
- rétention ;
- données sensibles ;
- séparation projet / plugin ;
- possibilité d’utiliser les données pour entraînement interne.

---

# 29. Résumé de décision actuel

Le choix proposé à ce stade est :

```text
AUTOSCALING
KEDA + HPA

CUSTOM CONTROL
External Scaler gRPC en Rust

CALCULS
Rust déterministe

ECONOMIC GUARD V1
3 inputs :
- coût du nœud / heure ou minute
- budget infrastructure mensuel
- budget déjà consommé

Décision :
- SCALE
- PARTIAL_SCALE
- QUEUE / DENY_SCALE

FORECAST V1
EWMA / Holt-Winters

ML PRINCIPAL
LightGBM

SYSTEM-ONE
Laya/Jev-like en R&D / shadow

DATA
Decision Ledger append-only

OPERATIONAL STORE
PostgreSQL

LONG-TERM DATASET
Object Storage + Parquet
```

Cette architecture vise à rester :

- simple ;
- peu coûteuse ;
- open source ;
- non dépendante d’une stack ML lourde ;
- progressivement améliorable ;
- compatible avec la philosophie d’économie de ressources du projet.

Pour la V1, la gouvernance économique doit rester volontairement minimale : la décision de scaling n’utilise que le coût unitaire du nœud, le budget infrastructure mensuel et le budget déjà consommé. Si la capacité technique désirée dépasse la capacité économiquement autorisée, la plateforme temporise la demande via queue/backpressure plutôt que de scaler sans limite.

L’interface de cette couche doit néanmoins rester générique afin de pouvoir évoluer plus tard vers une politique économique élastique intégrant revenus, projections de dépenses, coût marginal, réserve de burst, priorités et éventuellement modèles ML/System-One.

Le Decision Ledger est considéré comme une brique stratégique : il permet de commencer avec des règles simples tout en accumulant progressivement les données réelles nécessaires pour construire un moteur de décision beaucoup plus performant et spécifiquement adapté à la plateforme.
