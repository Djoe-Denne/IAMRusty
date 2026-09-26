# ADR-0009 : La limite P4-core (un pod H24 partagé par digest) est acceptable hors production et bloquante avant prod : scale on demand obligatoire ; partage d’instance inter-projets uniquement par décision explicite

- Statut : Proposed
- Réalité : Unimplemented
- Date : 2026-09-25
- Décideurs : (à remplir à l’acceptation)
- Jalon concerné : P4 (complète 0008 ; pré-production après P4-core)
- SuperSède : aucune
- SuperSédée par : —
- Related : [0003](0003-apparatus-untrusted-plugin.md), [0008](0008-apparatus-p4-k8s-isolation-outside-manifesto.md), [0011](0011-apparatus-p4-pod-par-binding.md)

`Accepted` ratifierait la **gate** ci-dessous. `Réalité : Unimplemented` : la cible (scale on demand + isolation d’instance non silencieuse) **n’est pas** dans le dépôt. Le code P4-core actuel (`apparatus-operator`) reste la **dette acceptée hors prod** — il ne satisfait pas cette ADR. Cette ADR **ne SuperSède pas** [0003](0003-apparatus-untrusted-plugin.md) ni [0008](0008-apparatus-p4-k8s-isolation-outside-manifesto.md) (`Réalité : Implemented` inchangée). `Réalité : Implemented` de 0009 **ssi** [ADR-0011](0011-apparatus-p4-pod-par-binding.md) est `Accepted` **et** `Réalité : Implemented`. `Accepted` sur 0009 (ratifier la gate) n’éteint pas l’ADR tant que 0011 n’est pas fermée (`Accepted` + `Implemented`).

## Contexte

[0008](0008-apparatus-p4-k8s-isolation-outside-manifesto.md) livre le moteur d’isolation P4-core (K8s hors Manifesto). Les humains qui croient « P4 fini » lisent 0008 Implemented. Or le contrôleur actuel :

- `pod_name_for(digest)` → `plugin-{32 hex}` ; `cr_name_for` → `sha256-{hex}` ;
- `reconcile_object` : si `Api<Pod>.get` OK → Scheduled no-op ; sinon create Pod nu + labels du **premier** créateur ;
- labels `apparatus.aiforall.dev/project` + `.../binding` = premier créateur seulement ; **pas** d’`ownerReferences` ;
- pas de Deployment plugin, pas de HPA, pas de KEDA, pas de scale-to-zero ;
- `envelope_to_podspec` : `restartPolicy: Always`.

[0003](0003-apparatus-untrusted-plugin.md) laisse **Non décidé** : scale-to-zero, OSB, Knative, etc. [0008](0008-apparatus-p4-k8s-isolation-outside-manifesto.md) laisse le drain / destruction des bindings `ready` à une ADR future. Sans gate écrite, la limite « un pod H24 partagé par digest » pourrait être shippée en prod par omission.

## Décision

1. **Limite actuelle acceptable hors prod.** Un pod H24 partagé par digest d’enveloppe, labels du premier créateur, pas de scale automatique : **acceptable** pour laptop / kind / environnements hors production.
2. **Gate pré-prod bloquante.** Cette limite **doit** être adressée **avant** toute mise en production. Interdit de présenter 0008 Implemented comme « prêt prod » sur ce point.
3. **Exigence : scale on demand.** Avant prod, les plugins (services qu’ils représentent) doivent pouvoir être **scalés à la demande**. Plus seulement « un pod H24 partagé ».
4. **Exigence : pas de partage silencieux.** Une instance ne doit plus être **implicitement** partagée entre projets sans **décision explicite**. Le shared n’est **pas** le défaut silencieux.
5. **Mécanisme = [ADR-0011](0011-apparatus-p4-pod-par-binding.md).** Le choix V1 (pod-par-binding) est **désigné**, pas détaillé ici.

## Conséquences

- 0008 reste Implemented pour P4-core ; cette ADR marque la **dette pré-prod** visible.
- Accept de cette ADR = ratifier la **gate**, pas livrer le mécanisme ; `Réalité : Implemented` seulement via [0011](0011-apparatus-p4-pod-par-binding.md) fermée (`Accepted` + `Implemented`).
- Toute roadmap « ship prod » doit citer 0009 comme gate ouverte tant que `Réalité` ≠ Implemented.

## Alternatives rejetées

| Option | Pourquoi pas (maintenant) |
|---|---|
| SuperSéder 0008 / rétrograder sa Réalité | P4-core est livré ; la gate est distincte |
| SuperSéder 0003 | Isolation untrusted reste ; le mécanisme V1 est [0011](0011-apparatus-p4-pod-par-binding.md) |
| Trancher HPA / KEDA / pod-par-binding / scale-to-zero maintenant | Hors périmètre de **cette** ADR ; le véhicule du choix V1 est [0011](0011-apparatus-p4-pod-par-binding.md) |
| Accepter le partage-par-digest comme défaut prod | Contredit l’exigence d’isolation explicite |
| Reporter la gate à une note wiki / closeout seul | Les lecteurs de 0008 Implemented ne verraient pas le blocage |

## Non décidé ici

- Drain / destruction des bindings déjà `ready` (renvoie [0008](0008-apparatus-p4-k8s-isolation-outside-manifesto.md) Non décidé)
- Scale-to-zero produit / OSB / Knative : reste Non décidé [0003](0003-apparatus-untrusted-plugin.md) (hors V1, pas le mécanisme)
- Replicas internes (KEDA ou HPA dans un workload déjà scopé binding) : ADR future ; voir Non décidé de [0011](0011-apparatus-p4-pod-par-binding.md) — pas un trou de 0009
- Budgets CPU numériques (`APP-06`), `APP-02` / `APP-05`

## Références

- Canon voisin : [0003](0003-apparatus-untrusted-plugin.md), [0008](0008-apparatus-p4-k8s-isolation-outside-manifesto.md), [0011](0011-apparatus-p4-pod-par-binding.md) ; closeout historique [0008-closeout.md](0008-closeout.md) (**non** modifié par cette ADR)
- Code (dette hors prod, preuve de la limite) : `apparatus-operator/src/controller.rs` — `pod_name_for`, `cr_name_for`, `reconcile_object`, `envelope_to_podspec`
- Preuve d’implémentation de **cette** cible : **aucune** (`Réalité : Unimplemented`)
