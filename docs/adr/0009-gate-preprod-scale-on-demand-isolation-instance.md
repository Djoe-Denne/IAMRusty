# ADR-0009 : La limite P4-core (un pod H24 partagé par digest) est acceptable hors production et bloquante avant prod : scale on demand obligatoire ; partage d’instance inter-projets uniquement par décision explicite

- Statut : Proposed
- Réalité : Unimplemented
- Date : 2026-09-25
- Décideurs : (à remplir à l’acceptation)
- Jalon concerné : P4 (complète 0008 ; pré-production après P4-core)
- SuperSède : aucune
- SuperSédée par : —
- Related : [0003](0003-apparatus-untrusted-plugin.md), [0008](0008-apparatus-p4-k8s-isolation-outside-manifesto.md), [0012](0012-apparatus-http-knative-par-binding.md) ; ancienne proposition [0011](0011-apparatus-p4-pod-par-binding.md), Rejected

`Accepted` would ratify this gate. `Réalité : Unimplemented` means the gate
has not been met: legacy P4-core does not satisfy the on-demand scaling and
instance isolation requirements. [ADR-0012](0012-apparatus-http-knative-par-binding.md)
was accepted on 2026-10-20, and portions of its implementation are in the
repository, but its complete target is not qualified. Closing 0009 still
requires **0012 Accepted and Implemented, plus an explicit review of
evidence for 0009's own criteria**. Neither accepting the mechanism nor a
single successful cold start is sufficient. This ADR does not supersede
[0003](0003-apparatus-untrusted-plugin.md) or
[0008](0008-apparatus-p4-k8s-isolation-outside-manifesto.md) or close 0010.

## Contexte

[0008](0008-apparatus-p4-k8s-isolation-outside-manifesto.md) delivered P4-core
isolation outside Manifesto. The legacy controller used by that path has the
following limits; it is not the Knative design:

- `pod_name_for(digest)` → `plugin-{32 hex}` ; `cr_name_for` → `sha256-{hex}` ;
- `reconcile_object` : si `Api<Pod>.get` OK → Scheduled no-op ; sinon create Pod nu + labels du **premier** créateur ;
- labels `apparatus.aiforall.dev/project` + `.../binding` = premier créateur seulement ; **pas** d’`ownerReferences` ;
- pas de Deployment plugin, pas de HPA, pas de KEDA, pas de scale-to-zero ;
- `envelope_to_podspec` : `restartPolicy: Always`.

[0003](0003-apparatus-untrusted-plugin.md) left the scale-to-zero mechanism open;
[0012](0012-apparatus-http-knative-par-binding.md) now provides the accepted HTTP
target. [0008](0008-apparatus-p4-k8s-isolation-outside-manifesto.md) deferred
draining and destroying `ready` bindings; the 0012 contract narrows the
runtime transition without deciding the full business lifecycle.

## Décision

1. **Limite actuelle acceptable hors prod.** Un pod H24 partagé par digest d’enveloppe, labels du premier créateur, pas de scale automatique : **acceptable** pour laptop / kind / environnements hors production.
2. **Gate pré-prod bloquante.** Cette limite **doit** être adressée **avant** toute mise en production. Interdit de présenter 0008 Implemented comme « prêt prod » sur ce point.
3. **Exigence : scale on demand.** Avant prod, les plugins (services qu’ils représentent) doivent pouvoir être **scalés à la demande**. Plus seulement « un pod H24 partagé ».
4. **Exigence : pas de partage silencieux.** Une instance ne doit plus être **implicitement** partagée entre projets sans **décision explicite**. Le shared n’est **pas** le défaut silencieux.
5. **Designated mechanism: [ADR-0012](0012-apparatus-http-knative-par-binding.md).**
   Serving/KPA per project and binding was accepted in 0012, not by this reference.
   0011 remains a rejected historical proposal. Ratification of 0012 does not prove this gate.

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
- The HTTP scale-to-zero mechanism and replica policy are decided in
  [0012](0012-apparatus-http-knative-par-binding.md), not here. No concurrent
  HPA/KEDA controller is implied.
- Valeurs de capacité, profil d’isolation, identité/transport et sémantique de révocation/drain : précisions bloquantes du [contrat 0012](../contracts/apparatus-knative-runtime.md), pas des capacités déjà livrées.
- Budgets CPU numériques (`APP-06`), `APP-02` / `APP-05`

## Références

- Canon voisin : [0003](0003-apparatus-untrusted-plugin.md), [0008](0008-apparatus-p4-k8s-isolation-outside-manifesto.md), [0012](0012-apparatus-http-knative-par-binding.md) ; [0011](0011-apparatus-p4-pod-par-binding.md) = proposition Rejected historique ; closeout [0008-closeout.md](0008-closeout.md) (**non** modifié par cette ADR)
- Code (dette hors prod, preuve de la limite) : `workers/apparatus-operator/src/controller.rs` — `pod_name_for`, `cr_name_for`, `reconcile_object`, `envelope_to_podspec`
- Evidence for this gate: **not established** as of 2026-11-21. The
  [0012 implementation evidence](0012-apparatus-http-knative-par-binding.md)
  does not establish 0009's real scaling and isolation criteria.
