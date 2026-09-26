# Plan d’implémentation — premier lancement monolithe + Apparatus (kind local)

- Date : 2026-09-25
- Statut : plan seulement (aucune implémentation dans ce tour)
- Canon long terme : [ADR-0404](adr/0404-runtime-microservices-et-monolithe.md) (Accepted), [ADR-0600](adr/0600-cloud-portable-opentofu-k8s-gitops.md) / [0601](adr/0601-cluster-trust-namespaces-standalones.md) (Proposed)
- Tranche locale déjà là : [ADR-0603](adr/0603-tranche-locale-deploy-kind-apparatus-lazaret.md) Partial · contrat [platform-local-v1-implementation-contract.md](platform-local-v1-implementation-contract.md)
- Briefing architectes : `.cursor/review-briefings/20260925T0923Z-orchestrator-monolithe-kind-briefing.md`

Ce fichier n’est **pas** une ADR. Il ne SuperSède pas 0404 ni 0601. Il enregistre un **ordre de livraison** demandé par l’utilisateur (monolithe d’abord, env local type cloud ensuite), comme 0603 l’a fait pour A+B sans GKE.

---

## Contexte — plaques de compilation (faits repo)

Ce n’est pas un feature Cargo `monolith` vs `micro`. Ce sont **deux graphes de binaires** :

| Plaque | Binaire | Contenu HTTP |
|---|---|---|
| Standalone | `iam-service`, `telegraph-service`, `hive-service`, `manifesto-service`, `lazaret-service` | Un process par slice (`just up`) |
| Monolithe | `oodhive-monolith` (`monolith/`) | Un listener : nests `/iam` `/telegraph` `/hive` `/manifesto` **`/lazaret`** |

Le monolithe compose les **setup** (`monolith/src/runtime.rs`) et `compose_routes` (`monolith/src/routes.rs`). Il n’appelle jamais `run()` standalone. **Lazaret est déjà dans le monolithe** ; `docs/services/monolith.md` et ADR-0404 (texte « quatre » préfixes) sont en retard — le code prime.

**Structurellement hors monolithe** : `apparatus-operator` (client kube, bins `apparatus-controller` / `apparatus-admit` / `apparatus-build`, « jamais dans Manifesto/Lazaret ») ; pods plugins ; `sentinel-sync` ; GitHubConnect / GitLabConnect.

`deploy/` 0603 est livré en **stubs** (nginx Lazaret, pause operator). kind `aiforall-local` ≠ IT `apparatus-p4-it`.

---

## Décision de frontière

**Un processus `oodhive-monolith`** (IAM + Telegraph + Hive + Manifesto + **Lazaret**) et **des processus Apparatus séparés** (`apparatus-admit`, `apparatus-controller`, `apparatus-build` selon le besoin VALID / schedule / build).

Lazaret **reste dans le monolithe** pour ce premier lancement : c’est déjà la plaque, et Lazaret n’a pas de client Kubernetes. Extraire Lazaret serait la plaque standalone, pas « tester en monolithe ».

L’operator **n’entre pas** dans le monolithe : CRD, informers, SA `controller` / `admit-sign` / `build`, `deploy/p4/` inchangé dans l’esprit (jamais fusion Manifesto).

Factory / host UI (P5/P6) : absents, hors plan.

---

## Ce que devient l’ancienne étape « vrais binaires sur kind »

**Reformulée, pas ignorée.**

- « Vrai Lazaret » = le nest `/lazaret` **dans** `oodhive-monolith` (preuve host d’abord), plus un Deployment `lazaret` nginx à remplacer en premier.
- « Vrai operator » = images des bins `apparatus-operator` sur `aiforall-local` via `deploy/p4/` + `kind load` (remplace pause).
- Le couple « image standalone Lazaret + operator tous deux réels sur kind » est **reporté** à la plaque 4+1 (cible 0601), après ce plan.

---

## Jalons valuables (à implémenter plus tard — commandes prévues, pas codées)

### J1 — Plaque monolithe + invoke réel (host)

**Valeur.** L’utilisateur voit Apparatus/Lazaret **dans le même binaire** que le reste, sans nginx.

**Contenu.** Infra compose (Postgres, OpenFGA, LocalStack, OpenBao) **sans** exiger les quatre `*-service`. `cargo run -p oodhive-monolith`. Corriger la doc stale « quatre BC » si on y touche (doc only).

**Preuve.** `GET /health` et un `POST /lazaret/invoke` (ou équivalent documenté) qui n’est **pas** le `ok` nginx — corps / erreur métier Lazaret réel.

**Reste stub.** Cluster kind inchangé (nginx/pause).

### J2 — Operator réel sur `aiforall-local`

**Valeur.** Le control plane Apparatus n’est plus `pause`.

**Contenu.** Image(s) `apparatus-controller` (et admit si VALID est requis pour J3). `kind load docker-image … --name aiforall-local`. Overlay Kustomize `images:` sur `deploy/p4/` uniquement. Tags **non-`latest`**, `imagePullPolicy: IfNotPresent` ou `Never`.

**Preuve.** Pod Ready, logs de reconcile, pas de crash-loop. Ne pas retarget `apparatus-p4-it`.

**Reste stub.** iam/hive/manifesto/telegraph en pause ; Lazaret kind encore nginx.

### J3 — Invoke isolé : plugin kind → Lazaret du monolithe **in-cluster** (overlay démo)

**Valeur.** Preuve **plateforme** : un pod `aiforall-plugins` atteint le **vrai** `/lazaret/invoke` (pas nginx, pas `localhost` hôte).

**Contenu.** Overlay kind **démo, non canon 0601** : un Deployment `oodhive-monolith` (une image, un Service) dans un ns plateforme, **à la place** du Deployment lazaret stub pour ce chemin. Dockerfile monolithe à créer alors. Plugin / Job probe `POST http://<svc>:8080/lazaret/invoke`. Operator J2 schedule ou probe existant.

**Pourquoi un Deployment monolithe ici.** Un Job kind ne peut pas prouver le Lazaret **host** (DNS in-cluster ≠ `localhost`). Extra-port / hostNetwork = faux ami. Donc le monolithe entre dans kind **seulement** comme overlay de démo pour fermer le chemin plugin→gateway. 0601 (4+1 standalones = canon cluster) **n’est pas SuperSédé**.

**Preuve.** HTTP 200 (ou contrat métier) depuis le ns plugins vers le nest `/lazaret/invoke` du pod monolithe ; pas le ConfigMap nginx.

**Reste stub.** 4 slices visibles uniquement via les prefixes du monolithe ; pas de 4+1 images standalone ; kindnet toujours sans enforce NP.

### J4 — (plus tard, hors ce premier lancement) bascule plaque standalone 0601

Remplacer le Deployment monolithe kind par 4+1. Pas bloquant pour « tester en monolithe ».

---

## Patterns web retenus / rejetés

**Retenus**

- Modular monolith = graphe compile-time, un déploiement applicatif — [Fowler, Monolith First](https://martinfowler.com/bliki/MonolithFirst.html)
- Operator = control plane hors de l’app HTTP — [Kubernetes Operator](https://kubernetes.io/docs/concepts/extend-kubernetes/operator/), [CNCF Operator White Paper](https://tag-app-delivery.cncf.io/whitepapers/operator/), [Kubebuilder / `make run` puis kind](https://book.kubebuilder.io/quick-start.html)
- Kustomize base + overlay + transformer `images` — [Kustomize](https://kubernetes.io/docs/tasks/manage-kubernetes-objects/kustomization/)
- `kind load docker-image` + pull policy Never/IfNotPresent — [kind quick start](https://kind.sigs.k8s.io/docs/user/quick-start/#loading-an-image-into-your-cluster)

**Rejetés**

- Un chart Helm first-party par slice (0600 : Helm = tiers)
- Kompose / docker-compose-as-kube — [limites Kompose](https://kompose.io/conversion/)
- Operator en sidecar du monolithe
- Registry locale obligatoire dès J1 (load suffit)
- Relivrer cargo M5/M6 sur `apparatus-p4-it` comme **seule** preuve plateforme (0603)

---

## Hors scope (premier lancement)

0602 / `deploy/obs` / `aiforall-obs` · GKE / OpenTofu apply · Flux live · Factory · Calico sur `aiforall-local` · casser `apparatus-p4-it` · fusion `deploy/p4/` × Manifesto · dossier `infra/` · sentinel-sync et IdP Connect dans le monolithe · images Rust « prod » digest Cosign · SuperSéde 0601.

Non bloquant : aucun de ces trous n’empêche J1 (Lazaret réel en process) ni J2 (operator réel).

---

## Croisement des architectes

Trois agents `architecte` (même briefing, web autorisé, lenses différentes).

| Question | A (plaques / 0404) | B (industrie K8s) | C (jalons depuis deploy/) | Décision orchestre |
|---|---|---|---|---|
| Lazaret in/out monolithe | In | In | In | **In** — code déjà nesté |
| Operator séparable | Oui, toujours | Oui, toujours | Oui, toujours | **Oui** — kube hors HTTP |
| Monolithe Deployment kind | Non (0601) | Oui dès J2 (load image) | Non pour le premier pas | **J1–J2 sans** ; **J3 overlay démo non canon** pour fermer plugin→invoke |
| Ancienne étape vrais binaires | Lazaret absorbé, operator reporté séparé | Idem + 0601 reporté | Scindée : Lazaret=J1 host, operator=J2 kind | **Reformulée** ainsi |
| Load vs registry | load | load | load | **load** |
| Helm / Kompose | Rejetés | Rejetés | Overlay `images:` | **Rejetés** |

**Dissensus principal.** Faut-il mettre `oodhive-monolith` dans kind pour que ce soit « cloud-like » ? A et C : non (0601 + pod≠host). B : oui pour une solution complète locale. **Tranche :** A/C ont raison sur le **canon** et sur le fait que J1 host est la première preuve monolithe. B a raison que le chemin **plugin → Lazaret** n’existe pas tant que Lazaret n’est pas in-cluster. J3 est donc un overlay démo **explicite**, pas un SuperSéde de 0601. Avant d’**implémenter** J3, figer cet écart (note de séquence ou ADR 0604 Proposed) — pas dans ce tour.

Risque réseau retenu (C) : ne pas vendre un extraPortMapping kind→localhost comme preuve in-cluster.

---

## Ce que ce plan n’autorise pas tout de suite

Pas d’édition `deploy/`, justfile, crates Rust, ni création de cluster dans le tour qui a produit ce document. Accept humain 0600/0601/0603 inchangé.

Suite cas classique (signup → projet → KV → pod, sans seed SQL) : [platform-local-gold-case-implementation-guide.md](platform-local-gold-case-implementation-guide.md) + [ADR-0605](adr/0605-gold-path-kind-j3-dns-attach.md) (gold path Kind J3 / DNS / attach) — le guide n’est pas une ADR ; ne SuperSède pas ce plan ni 0604.
