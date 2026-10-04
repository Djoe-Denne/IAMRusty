# ADR-0604 : L’écart de séquence J3 est un overlay Kustomize démo non canon sur kind `aiforall-local` : Deployment `oodhive-monolith` pour prouver plugins → `/lazaret/invoke`, sans SuperSéder le canon cluster 4+1 de 0601

- Statut : Proposed
- Réalité : Partial
- Date : 2026-09-25
- Décideurs : (à remplir à l’acceptation)
- Jalon concerné : Cloud-portable (hors Apparatus P0–P6)
- SuperSède : aucune
- SuperSédée par : —
- Related : [0600](0600-cloud-portable-opentofu-k8s-gitops.md), [0601](0601-cluster-trust-namespaces-standalones.md), [0603](0603-tranche-locale-deploy-kind-apparatus-lazaret.md), [0404](0404-runtime-microservices-et-monolithe.md), [0008](0008-apparatus-p4-k8s-isolation-outside-manifesto.md)

`Accepted` ratifierait une cible ; le Statut reste Proposed. `Réalité : Partial` au 2026-10-04 : overlay démo, Dockerfile/image J3 et helpers présents ; l’ancien état Implemented et le résultat J3 sont conservés comme preuve historique de cette tranche, pas validation du root actuel modifié. Cette ADR ne SuperSède ni [0601](0601-cluster-trust-namespaces-standalones.md), ni [0404](0404-runtime-microservices-et-monolithe.md), ni [0600](0600-cloud-portable-opentofu-k8s-gitops.md), ni [0603](0603-tranche-locale-deploy-kind-apparatus-lazaret.md), ni [0008](0008-apparatus-p4-k8s-isolation-outside-manifesto.md). Elle décrit l’écart J3 monolithe démo ; le nouveau lab standalone est distinct ([0606](0606-local-full-kind-isole.md)), sans Accept implicite.

## Contexte

[0601](0601-cluster-trust-namespaces-standalones.md) fige : unité cluster V1 = **4+1** (`iam` `hive` `manifesto` `telegraph` + `lazaret`) ; le monolithe = laptop / démo, **pas** Deployment prod / staging / **kind canon**. [0404](0404-runtime-microservices-et-monolithe.md) (Accepted) garde le dual runtime laptop ; 0601 ferme « 1 vs 4 » **du côté 4+1**.

[0603](0603-tranche-locale-deploy-kind-apparatus-lazaret.md) livre A+B local : overlay `ops/deploy/apps/overlays/kind` ; M2 = stub nginx Lazaret + Job probe → `200` `ok\n` sur GET `/lazaret/invoke`. Ce stub **n’est pas** le nest métier Lazaret du monolithe.

Le plan [platform-local-monolith-kind-implementation-plan.md](../platform-local-monolith-kind-implementation-plan.md) (pas une ADR) ordonne J1 (host) → J2 (operator image sur `ops/deploy/p4`) → **J3** (invoke isolé in-cluster). Un Job kind ne peut pas viser le Lazaret **hôte** (DNS in-cluster ≠ `localhost`). Extra-port kind / hostNetwork / curl localhost hôte = **faux amis** : ils ne prouvent pas plugins → Service cluster.

NetworkPolicy base (`ops/deploy/apps/base/networkpolicies.yaml`) : pods `aiforall-plugins` ne peuvent (sur le papier) parler qu’aux pods `app.kubernetes.io/name=lazaret` dans `aiforall-gateway` port **8080**. kindnet **n’enforce pas** ; la carte 0601 reste normative. L’overlay démo doit soit (a) garder un Service / labels joignables depuis plugins **sans** casser la carte 0601, soit (b) patcher la NP **dans l’overlay démo seulement**.

## Décision

1. **Canon inchangé.** [0601](0601-cluster-trust-namespaces-standalones.md) reste le canon cluster : 4+1 standalones. Le monolithe n’entre **pas** dans prod / staging / kind **canon**. Cette ADR ne réécrit pas M2 comme preuve monolithe.
2. **J3 = overlay démo séparé.** Sur kind **`aiforall-local` seulement**, un overlay Kustomize **démo, non canon**, **distinct** de `ops/deploy/apps/overlays/kind` (M2 stub nginx `ok\n` reste la preuve stub 0603). Cet overlay déploie un Deployment nommé **`oodhive-monolith`** + Service, **à la place** du stub nginx Lazaret **pour ce chemin de preuve**. Le stub nginx de **cet** overlay démo ne doit plus pouvoir servir `ok\n` sur le chemin de preuve (sinon le stub n’est pas remplacé).
3. **Preuve J3.** Pod ou Job dans `aiforall-plugins` → **POST** vers le Service du nest monolithe `/lazaret/invoke`. Contrat métier acceptable : réponse **401** JSON unauthorized (authn absente / rejet métier). **Interdit** comme preuve : extraPortMapping kind, hostNetwork, curl vers localhost / IP hôte.
4. **Config in-cluster (bornes de la tranche J3 livrée).** rustycog a besoin de `config/*.toml`. La démo kind livrée réutilise l’infra Compose (Postgres, OpenFGA, …) via DNS hôte (`host.docker.internal`). **Ne pas** vendre extraPortMapping comme preuve invoke. Amendement 2026-10-02 ([0308](0308-mesh-authn-jwt.md) point 8) : hors tests d’intégration, le local isoprod sert les third parties dans Kind. Le point 4 reste la photo de la tranche J3 telle qu’elle a été codée (`host.docker.internal:5432`). Cet écart n’est pas résorbé ici ; il ne prime pas sur 0308 pour le canal isoprod.
5. **Hors monolithe / hors cette ADR comme impl.** Operator = `ops/deploy/p4` (J2) ; pas Factory ; pas [0602](0602-observabilite-portable-otlp-lgtm.md) ; pas GKE ; pas Calico sur `aiforall-local` ; pas retarget fixture `apparatus-p4-it`.
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
| Fusionner monolithe dans `ops/deploy/apps/overlays/kind` (M2) | Écrase la preuve stub 0603 ; confond canon et démo |
| Poser Postgres/OpenFGA dans kind pour la tranche J3 livrée | Photo de tranche : Compose hôte via DNS hôte. Cible isoprod ultérieure : [0308](0308-mesh-authn-jwt.md) point 8 (2026-10-02), third parties in-kind hors IT |
| Calico / CNI enforce sur `aiforall-local` | Hors J3 ; kindnet reste le runtime local |
| Retarget `apparatus-p4-it` ou Factory | Fixture IT P4 et P5/P6 hors scope |
| Implémenter J4 dans cette ADR | Séquence future ; pas le livrable J3 |

## Non décidé ici

- Accept formel 0600 / 0601 / 0603 / 0604 (humain / PR).
- Contenu exact des YAML overlay démo, Dockerfile monolithe, tags d’image (impl J3).
- Choix (a) labels/Service vs (b) patch NP dans l’overlay démo — les deux restent autorisés tant que la carte 0601 n’est pas présentée comme cassée.
- J4 : bascule 4+1 standalone images (ADR ou contrat ultérieur).
- Couche C GKE, 0602 / `aiforall-obs`, Flux live, Cosign CI.

## Réalité courante et preuve historique — 2026-10-04

- Source J3 : `ops/deploy/apps/overlays/kind-demo-monolith/`, `ops/deploy/deploy-j3.ps1` et runtime monolithe. Le résultat HTTP401 du Job référencé ci-dessous est historique ; ce travail documentaire ne réexécute ni build ni Job. Source root HEAD `5348a63` non committée : aucune compilation complète/IT/E2E de l’ensemble modifié ne justifie de conserver Implemented comme état courant.
- Le point 4 et `host.docker.internal` restent une limite historique explicite : pas une option conforme de fermeture local isoprod. Le local-full source utilise `ops/deploy/apps/overlays/local-full/`, standalones et dépendances in-Kind ; il ne transforme pas une preuve monolithe J3 en preuve de quatre services.
- Kind-demo-monolith et gold path restent séparés du nouveau lab/context `kind-aiforall-local-full` autorisé après IT. Le cluster legacy étranger n’est ni modifié ni recréé pour révalider J3. Si la preuve historique n’est pas reproductible dans le périmètre autorisé, rendre INCONCLUSIVE et demander un arbitrage, pas un transfert automatique de preuve.
- D6 render112 et topologie1+2 sont SOURCE/offline ; NetworkPolicy avec CNI enforce, vrai trafic Envoy, reprise et restore distinct exigent leurs preuves au hash courant. Une promotion future doit qualifier précisément J3/gold vs standalone/full. 0404/0500 et leurs limites historiques restent inchangés.

## Références

- Canon cluster : `docs/adr/0601-cluster-trust-namespaces-standalones.md` ; dual runtime : `docs/adr/0404-runtime-microservices-et-monolithe.md`
- Tranche locale A+B : `docs/adr/0603-tranche-locale-deploy-kind-apparatus-lazaret.md` ; preuve M2 stub : `ops/deploy/apps/overlays/kind/`
- Plan (ordre, pas canon) : `docs/platform-local-monolith-kind-implementation-plan.md` (J3 / J4)
- NP : `ops/deploy/apps/base/networkpolicies.yaml` (`app.kubernetes.io/name: lazaret`, port 8080)
- Operator P4 : `ops/deploy/p4/` ; [0008](0008-apparatus-p4-k8s-isolation-outside-manifesto.md)
- Wiki pointeur (optionnel) : `obsidian/AI FOR ALL/projects/aiforall/decisions/0604-j3-overlay-demo-monolith.md`
- Preuve historique de la tranche J3 (pas de nouvelle exécution) : `just deploy-j3` ; overlay `ops/deploy/apps/overlays/kind-demo-monolith/` ; Job `invoke-probe-j3` POST `lazaret.aiforall-gateway.svc.cluster.local:8080/lazaret/invoke` → HTTP 401 `{"error":"unauthorized"}`
