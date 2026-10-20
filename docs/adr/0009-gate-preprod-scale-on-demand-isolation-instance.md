# ADR-0009 : La limite P4-core (un pod H24 partagé par digest) est acceptable hors production et bloquante avant prod : scale on demand obligatoire ; partage d’instance inter-projets uniquement par décision explicite

- Statut : Proposed
- Réalité : Unimplemented
- Date : 2026-09-25
- Décideurs : (à remplir à l’acceptation)
- Jalon concerné : P4 (complète 0008 ; pré-production après P4-core)
- SuperSède : aucune
- SuperSédée par : —
- Related : [0003](0003-apparatus-untrusted-plugin.md), [0008](0008-apparatus-p4-k8s-isolation-outside-manifesto.md), [0012](0012-apparatus-http-knative-par-binding.md) ; ancienne proposition [0011](0011-apparatus-p4-pod-par-binding.md), Rejected

`Accepted` ratifierait la **gate** ci-dessous. `Réalité : Unimplemented` : la cible (scale on demand + isolation d’instance non silencieuse) **n’est pas** dans le dépôt. Le code P4-core actuel (`apparatus-operator`) reste la **dette acceptée hors prod** — il ne satisfait pas cette ADR. Cette ADR **ne SuperSède pas** [0003](0003-apparatus-untrusted-plugin.md) ni [0008](0008-apparatus-p4-k8s-isolation-outside-manifesto.md) (`Réalité : Implemented` inchangée). Mise à jour du 2026-10-19 : le mécanisme proposé est désormais [0012](0012-apparatus-http-knative-par-binding.md), 0011 étant Rejected. Fermer la Réalité de 0009 exige **0012 Accepted et Implemented ET les critères propres de 0009 prouvés**, avec revue explicite des preuves scale on demand/isolation. Accept de la gate, Accept du mécanisme ou simple cold-start vert ne suffisent pas. Aucune clôture pré-prod globale ni de 0010 n’en découle.

## Contexte

[0008](0008-apparatus-p4-k8s-isolation-outside-manifesto.md) livre le moteur d’isolation P4-core (K8s hors Manifesto). Les humains qui croient « P4 fini » lisent 0008 Implemented. Or le contrôleur actuel :

- `pod_name_for(digest)` → `plugin-{32 hex}` ; `cr_name_for` → `sha256-{hex}` ;
- `reconcile_object` : si `Api<Pod>.get` OK → Scheduled no-op ; sinon create Pod nu + labels du **premier** créateur ;
- labels `apparatus.aiforall.dev/project` + `.../binding` = premier créateur seulement ; **pas** d’`ownerReferences` ;
- pas de Deployment plugin, pas de HPA, pas de KEDA, pas de scale-to-zero ;
- `envelope_to_podspec` : `restartPolicy: Always`.

[0003](0003-apparatus-untrusted-plugin.md) laissait ouvert le mécanisme scale-to-zero ; [0012](0012-apparatus-http-knative-par-binding.md) y apporte désormais une cible forward Proposed, sans modifier 0003. [0008](0008-apparatus-p4-k8s-isolation-outside-manifesto.md) laisse le drain / destruction des bindings `ready` à une décision ultérieure ; le contrat 0012 identifie les précisions encore bloquantes, sans prétendre décider tout le cycle métier. Sans gate écrite, la limite « un pod H24 partagé par digest » pourrait être shippée en prod par omission.

## Décision

1. **Limite actuelle acceptable hors prod.** Un pod H24 partagé par digest d’enveloppe, labels du premier créateur, pas de scale automatique : **acceptable** pour laptop / kind / environnements hors production.
2. **Gate pré-prod bloquante.** Cette limite **doit** être adressée **avant** toute mise en production. Interdit de présenter 0008 Implemented comme « prêt prod » sur ce point.
3. **Exigence : scale on demand.** Avant prod, les plugins (services qu’ils représentent) doivent pouvoir être **scalés à la demande**. Plus seulement « un pod H24 partagé ».
4. **Exigence : pas de partage silencieux.** Une instance ne doit plus être **implicitement** partagée entre projets sans **décision explicite**. Le shared n’est **pas** le défaut silencieux.
5. **Mécanisme proposé = [ADR-0012](0012-apparatus-http-knative-par-binding.md).** Serving/KPA par projet × binding est désigné, pas détaillé ici. 0011 est une proposition Rejected conservée comme historique. Cette référence ne vaut ni Accept du mécanisme ni preuve de ses critères.

## Conséquences

- 0008 reste Implemented pour P4-core ; cette ADR marque la **dette pré-prod** visible.
- Accept de cette ADR = ratifier la **gate**, pas livrer le mécanisme ; `Réalité : Implemented` seulement après [0012](0012-apparatus-http-knative-par-binding.md) `Accepted` + `Implemented` **et** preuves propres de scale on demand/isolation exigées par 0009, revues explicitement.
- Toute roadmap « ship prod » doit citer 0009 comme gate ouverte tant que `Réalité` ≠ Implemented.

## Alternatives rejetées

| Option | Pourquoi pas (maintenant) |
|---|---|
| SuperSéder 0008 / rétrograder sa Réalité | P4-core est livré ; la gate est distincte |
| SuperSéder 0003 | Isolation untrusted reste ; le mécanisme HTTP proposé est [0012](0012-apparatus-http-knative-par-binding.md) |
| Trancher le mécanisme dans cette gate | Hors périmètre de **cette** ADR ; le véhicule du choix est [0012](0012-apparatus-http-knative-par-binding.md), Proposed |
| Accepter le partage-par-digest comme défaut prod | Contredit l’exigence d’isolation explicite |
| Reporter la gate à une note wiki / closeout seul | Les lecteurs de 0008 Implemented ne verraient pas le blocage |

## Non décidé ici

- Drain / destruction des bindings déjà `ready` (renvoie [0008](0008-apparatus-p4-k8s-isolation-outside-manifesto.md) Non décidé)
- Mécanisme scale-to-zero HTTP et replicas internes : cible Serving/KPA proposée par [0012](0012-apparatus-http-knative-par-binding.md), à ratifier ; pas de HPA/KEDA concurrent implicite.
- Valeurs de capacité, profil d’isolation, identité/transport et sémantique de révocation/drain : précisions bloquantes du [contrat 0012](../contracts/apparatus-knative-runtime.md), pas des capacités déjà livrées.
- Budgets CPU numériques (`APP-06`), `APP-02` / `APP-05`

## Références

- Canon voisin : [0003](0003-apparatus-untrusted-plugin.md), [0008](0008-apparatus-p4-k8s-isolation-outside-manifesto.md), [0012](0012-apparatus-http-knative-par-binding.md) ; [0011](0011-apparatus-p4-pod-par-binding.md) = proposition Rejected historique ; closeout [0008-closeout.md](0008-closeout.md) (**non** modifié par cette ADR)
- Code (dette hors prod, preuve de la limite) : `workers/apparatus-operator/src/controller.rs` — `pod_name_for`, `cr_name_for`, `reconcile_object`, `envelope_to_podspec`
- Preuve d’implémentation de **cette** cible : **aucune** (`Réalité : Unimplemented`)
