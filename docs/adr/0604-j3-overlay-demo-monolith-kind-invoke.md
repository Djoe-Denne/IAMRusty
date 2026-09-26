# ADR-0604 : L’écart de séquence J3 est un overlay Kustomize démo non canon sur kind `aiforall-local` : Deployment `oodhive-monolith` pour prouver plugins → `/lazaret/invoke`, sans SuperSéder le canon cluster 4+1 de 0601

- Statut : Proposed
- Réalité : Implemented
- Date : 2026-09-25
- Décideurs : (à remplir à l’acceptation)
- Jalon concerné : Cloud-portable (hors Apparatus P0–P6)
- SuperSède : aucune
- SuperSédée par : —
- Related : [0600](0600-cloud-portable-opentofu-k8s-gitops.md), [0601](0601-cluster-trust-namespaces-standalones.md), [0603](0603-tranche-locale-deploy-kind-apparatus-lazaret.md), [0404](0404-runtime-microservices-et-monolithe.md), [0008](0008-apparatus-p4-k8s-isolation-outside-manifesto.md)

`Accepted` ratifie une cible. `Réalité` **Implemented** : overlay démo + image `aiforall-oodhive-monolith:j3` + `just deploy-j3` (accord de coding explicite ; Statut reste Proposed). Cette ADR **ne SuperSède pas** [0601](0601-cluster-trust-namespaces-standalones.md), [0404](0404-runtime-microservices-et-monolithe.md), [0600](0600-cloud-portable-opentofu-k8s-gitops.md), [0603](0603-tranche-locale-deploy-kind-apparatus-lazaret.md), ni [0008](0008-apparatus-p4-k8s-isolation-outside-manifesto.md). Elle enregistre un **écart de séquence** : prouver le chemin plugin → nest Lazaret **in-cluster** avec le monolithe, **sans** déclarer le monolithe unité cluster V1.

## Contexte

[0601](0601-cluster-trust-namespaces-standalones.md) fige : unité cluster V1 = **4+1** (`iam` `hive` `manifesto` `telegraph` + `lazaret`) ; le monolithe = laptop / démo, **pas** Deployment prod / staging / **kind canon**. [0404](0404-runtime-microservices-et-monolithe.md) (Accepted) garde le dual runtime laptop ; 0601 ferme « 1 vs 4 » **du côté 4+1**.

[0603](0603-tranche-locale-deploy-kind-apparatus-lazaret.md) livre A+B local : overlay `deploy/apps/overlays/kind` ; M2 = stub nginx Lazaret + Job probe → `200` `ok\n` sur GET `/lazaret/invoke`. Ce stub **n’est pas** le nest métier Lazaret du monolithe.

Le plan [platform-local-monolith-kind-implementation-plan.md](../platform-local-monolith-kind-implementation-plan.md) (pas une ADR) ordonne J1 (host) → J2 (operator image sur `deploy/p4`) → **J3** (invoke isolé in-cluster). Un Job kind ne peut pas viser le Lazaret **hôte** (DNS in-cluster ≠ `localhost`). Extra-port kind / hostNetwork / curl localhost hôte = **faux amis** : ils ne prouvent pas plugins → Service cluster.

NetworkPolicy base (`deploy/apps/base/networkpolicies.yaml`) : pods `aiforall-plugins` ne peuvent (sur le papier) parler qu’aux pods `app.kubernetes.io/name=lazaret` dans `aiforall-gateway` port **8080**. kindnet **n’enforce pas** ; la carte 0601 reste normative. L’overlay démo doit soit (a) garder un Service / labels joignables depuis plugins **sans** casser la carte 0601, soit (b) patcher la NP **dans l’overlay démo seulement**.

## Décision

1. **Canon inchangé.** [0601](0601-cluster-trust-namespaces-standalones.md) reste le canon cluster : 4+1 standalones. Le monolithe n’entre **pas** dans prod / staging / kind **canon**. Cette ADR ne réécrit pas M2 comme preuve monolithe.
2. **J3 = overlay démo séparé.** Sur kind **`aiforall-local` seulement**, un overlay Kustomize **démo, non canon**, **distinct** de `deploy/apps/overlays/kind` (M2 stub nginx `ok\n` reste la preuve stub 0603). Cet overlay déploie un Deployment nommé **`oodhive-monolith`** + Service, **à la place** du stub nginx Lazaret **pour ce chemin de preuve**. Le stub nginx de **cet** overlay démo ne doit plus pouvoir servir `ok\n` sur le chemin de preuve (sinon le stub n’est pas remplacé).
3. **Preuve J3.** Pod ou Job dans `aiforall-plugins` → **POST** vers le Service du nest monolithe `/lazaret/invoke`. Contrat métier acceptable : réponse **401** JSON unauthorized (authn absente / rejet métier). **Interdit** comme preuve : extraPortMapping kind, hostNetwork, curl vers localhost / IP hôte.
4. **Config in-cluster (bornes).** rustycog a besoin de `config/*.toml`. La démo kind **peut** réutiliser l’infra Compose (Postgres, OpenFGA, …) via DNS hôte (`host.docker.internal` ou équivalent documenté). **Ne pas** poser Postgres / OpenFGA dans kind pour J3. **Ne pas** vendre extraPortMapping comme preuve invoke.
5. **Hors monolithe / hors cette ADR comme impl.** Operator = `deploy/p4` (J2) ; pas Factory ; pas [0602](0602-observabilite-portable-otlp-lgtm.md) ; pas GKE ; pas Calico sur `aiforall-local` ; pas retarget fixture `apparatus-p4-it`.
6. **J4 (plus tard).** Retirer l’overlay démo et revenir 4+1. **Pas** d’implémentation J4 dans cette ADR ; seule l’intention de séquence est notée.

## Conséquences

- Coding agent J3 : créer un overlay démo séparé + image monolithe chargeable kind ; **ne pas** fusionner dans l’overlay M2 canon ; **ne pas** SuperSéder 0601 dans les README.
- Preuve M2 (0603) et preuve J3 (cette ADR) coexistent : M2 = stub ; J3 = nest monolithe. Ne pas invalider `just deploy-m2` en réécrivant le stub comme « vrai Lazaret ».
- NP : si labels / Service ne collent plus à `app.kubernetes.io/name=lazaret` + `aiforall-gateway:8080`, le patch NP reste **scoped** overlay démo.
- Après J4 : overlay démo retiré ; kind canon = 4+1 à nouveau.
- **Écart gold path :** « Calico hors J3 » (alternatives) reste vrai pour le **livrable J3** — overlay démo inchangé. L’enforcement CNI Calico v3.29.7 sur `aiforall-local` est le **gold path local** ([0605](0605-gold-path-kind-j3-dns-attach.md)), pas un SuperSéde de cette ADR.

## Alternatives rejetées

| Option | Pourquoi pas (maintenant) |
|---|---|
| SuperSéder 0601 : monolithe = unité kind/prod | Contredit 0404 + 0601 ; blast IAM/Hive/Manifesto/Telegraph |
| Extra-port / hostNetwork / localhost hôte comme « preuve invoke » | Faux ami : ne prouve pas plugins → Service in-cluster |
| Fusionner monolithe dans `deploy/apps/overlays/kind` (M2) | Écrase la preuve stub 0603 ; confond canon et démo |
| Poser Postgres/OpenFGA dans kind pour J3 | Hors tranche ; Compose hôte suffit via DNS hôte |
| Calico / CNI enforce sur `aiforall-local` | Hors J3 ; kindnet reste le runtime local |
| Retarget `apparatus-p4-it` ou Factory | Fixture IT P4 et P5/P6 hors scope |
| Implémenter J4 dans cette ADR | Séquence future ; pas le livrable J3 |

## Non décidé ici

- Accept formel 0600 / 0601 / 0603 / 0604 (humain / PR).
- Contenu exact des YAML overlay démo, Dockerfile monolithe, tags d’image (impl J3).
- Choix (a) labels/Service vs (b) patch NP dans l’overlay démo — les deux restent autorisés tant que la carte 0601 n’est pas présentée comme cassée.
- J4 : bascule 4+1 standalone images (ADR ou contrat ultérieur).
- Couche C GKE, 0602 / `aiforall-obs`, Flux live, Cosign CI.

## Références

- Canon cluster : `docs/adr/0601-cluster-trust-namespaces-standalones.md` ; dual runtime : `docs/adr/0404-runtime-microservices-et-monolithe.md`
- Tranche locale A+B : `docs/adr/0603-tranche-locale-deploy-kind-apparatus-lazaret.md` ; preuve M2 stub : `deploy/apps/overlays/kind/`
- Plan (ordre, pas canon) : `docs/platform-local-monolith-kind-implementation-plan.md` (J3 / J4)
- NP : `deploy/apps/base/networkpolicies.yaml` (`app.kubernetes.io/name: lazaret`, port 8080)
- Operator P4 : `deploy/p4/` ; [0008](0008-apparatus-p4-k8s-isolation-outside-manifesto.md)
- Wiki pointeur (optionnel) : `obsidian/AI FOR ALL/projects/aiforall/decisions/0604-j3-overlay-demo-monolith.md`
- Preuve d’implémentation : `just deploy-j3` ; overlay `deploy/apps/overlays/kind-demo-monolith/` ; Job `invoke-probe-j3` POST `lazaret.aiforall-gateway.svc.cluster.local:8080/lazaret/invoke` → HTTP 401 `{"error":"unauthorized"}`
