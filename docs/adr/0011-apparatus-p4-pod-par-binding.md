# ADR-0011 : Le runtime plugin Apparatus V1 est un Pod par binding (clé projet × binding) ; l’operator P4 le crée, met au repos ou le détruit ; deux projets même digest d’enveloppe ⇒ deux Pods, sauf partage explicite

- Statut : Proposed
- Réalité : Unimplemented
- Date : 2026-09-26
- Décideurs : (à remplir à l’acceptation)
- Jalon concerné : P4 (complète 0009 ; mécanisme de la gate)
- SuperSède : aucune
- SuperSédée par : —
- Related : [0009](0009-gate-preprod-scale-on-demand-isolation-instance.md), [0008](0008-apparatus-p4-k8s-isolation-outside-manifesto.md), [0003](0003-apparatus-untrusted-plugin.md), [0004](0004-apparatus-capability-gateway.md), [0007](0007-apparatus-p3-capability-boundary-after-accept.md)

`Accepted` ratifierait le **mécanisme** ci-dessous. `Réalité : Unimplemented` : ce mécanisme **n’est pas** dans le dépôt. Satisfaire cet ADR (`Accepted` **et** `Réalité : Implemented`) est **la condition de fermeture de la Réalité** de [0009](0009-gate-preprod-scale-on-demand-isolation-instance.md). Cette ADR **ne SuperSède pas** [0008](0008-apparatus-p4-k8s-isolation-outside-manifesto.md), [0009](0009-gate-preprod-scale-on-demand-isolation-instance.md), [0003](0003-apparatus-untrusted-plugin.md), [0004](0004-apparatus-capability-gateway.md) ni [0007](0007-apparatus-p3-capability-boundary-after-accept.md).

## Contexte

[0009](0009-gate-preprod-scale-on-demand-isolation-instance.md) est la **gate** (scale on demand + pas de partage silencieux). Elle ne détaille pas le mécanisme ; elle le désigne ici. Le contrôleur actuel `pod_name_for(digest)` / get-or-create par digest est la **dette hors prod** (`apparatus-operator/src/controller.rs`, même constat que 0009). [0003](0003-apparatus-untrusted-plugin.md) V1 isole par projet et par binding. P4-core (moteur K8s hors Manifesto) reste [0008](0008-apparatus-p4-k8s-isolation-outside-manifesto.md) ; cet ADR ne le redécrit pas.

## Décision

1. **Clé d’instance V1.** Projet × binding. Pas le digest d’enveloppe seul.
2. **Scale on demand V1.** L’operator P4 crée, met au repos ou détruit le Pod de ce binding.
3. **Pas de partage silencieux.** Même digest d’enveloppe, deux projets ⇒ deux Pods, sauf **décision explicite** de partage. `shared` / organisation = hors V1 ([0003](0003-apparatus-untrusted-plugin.md)).
4. **Moteur et I/O inchangés.** Operator + Pod nu [0008](0008-apparatus-p4-k8s-isolation-outside-manifesto.md). I/O = Lazaret ([0004](0004-apparatus-capability-gateway.md), [0007](0007-apparatus-p3-capability-boundary-after-accept.md)). Pas de second plan de trafic. L’axe V1 est ce Pod par binding ; un wrapper Deployment pour replicas internes n’est pas un trou de l’axe (même famille que l’ADR future KEDA-dans-binding).
5. **Trade-off.** Plus d’idle (un Pod par binding) jusqu’à une densification **explicite** ultérieure.
6. **Condition 0009.** `Réalité : Implemented` de [0009](0009-gate-preprod-scale-on-demand-isolation-instance.md) **ssi** cet ADR est `Accepted` **et** `Réalité : Implemented`.

## Conséquences

- Preuve future attendue : deux bindings / même digest → deux Pods ; plus de `pod_name_for(digest)` partagé ; labels / owner = binding courant.
- [0008](0008-apparatus-p4-k8s-isolation-outside-manifesto.md) reste Implemented (moteur). Discovery et notes de conception **ne sont pas** un contrat.
- Tant que `Réalité` ≠ Implemented, le get-or-create par digest reste la dette hors prod de 0009.

## Alternatives rejetées

| Option | Pourquoi pas (V1) |
|---|---|
| Knative | Hors V1 ; scale-to-zero produit reste Non décidé 0003 |
| KEDA comme scaler principal | Recréerait une clé au-dessus du binding ; replicas internes = ADR future distincte |
| HPA seul | Ne porte pas la clé projet × binding ni le scale on demand V1 |
| Warm pools / bin packing | Densification implicite ; partage ou packing silencieux |
| WASM | Autre runtime ; hors mécanisme Pod V1 |
| GPU mutualisé | Partage de ressource hors V1 |
| Scheduler propriétaire | 0008 = operator + Pod nu ; pas un second orchestrateur |
| CRD Capacity / IA | Surface nouvelle hors gate 0009 |
| Karpenter comme prérequis | Provisionnement nœuds, pas l’instance plugin |
| LightGBM | Modèle d’allocation hors V1 |
| Decision Ledger | Compte de décisions hors V1 |
| Economic Guard | Garde économique hors V1 |
| Jev / Laya | Hors périmètre runtime plugin V1 |
| Scaler dont la clé est le digest | Recrée le partage silencieux ; rejeté |
| SuperSéder 0008 ou 0009 | 0008 reste le moteur ; 0009 reste la gate |

## Non décidé ici

- KEDA (ou HPA) comme nombre de replicas **à l’intérieur** d’un workload déjà scopé par binding → ADR **future distincte**, pas un trou de 0011
- **Scale intelligent ou prédictif, hors Apparatus seul.** Prévision et politique apprise (la famille laissée hors V1 : HPA au-delà du seuil déterministe, KEDA prédictif, LightGBM) ne servent pas qu’aux plugins Apparatus. L’ADR future qui les tranche couvre aussi le monolithe, `sentinel-sync`, et tout service hosté par la plateforme. 0011 ne la rédige pas, et le Pod par binding ne s’étend pas à ces services.
- Drain / destruction des bindings `ready` → [0008](0008-apparatus-p4-k8s-isolation-outside-manifesto.md)
- Scale-to-zero produit / OSB / Knative → reste Non décidé [0003](0003-apparatus-untrusted-plugin.md), hors V1
- UX gateway `QUEUED` \| `SCALING` = produit, pas le mécanisme
- Densification explicite post-V1 (partage **décidé**, jamais silencieux)

## Références

- Canon voisin : [0009](0009-gate-preprod-scale-on-demand-isolation-instance.md), [0008](0008-apparatus-p4-k8s-isolation-outside-manifesto.md), [0003](0003-apparatus-untrusted-plugin.md), [0004](0004-apparatus-capability-gateway.md), [0007](0007-apparatus-p3-capability-boundary-after-accept.md)
- Code (dette hors prod, même constat que 0009) : `apparatus-operator/src/controller.rs` — `pod_name_for`
- Preuve d’implémentation de **cette** cible : **aucune** (`Réalité : Unimplemented`)
