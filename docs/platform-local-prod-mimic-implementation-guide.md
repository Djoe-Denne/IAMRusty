# Guide d’implémentation — rapprocher le Kind gold d’un mimic prod (registre, catalogue, OpenBao)

**Statut** : contrat d’exécution pour **une session d’implémentation**. **Ce n’est pas une ADR.** Ne SuperSède rien.

**Prompt session** : lire ce fichier en entier, puis implémenter les jalons **L1 → L2 → L3 → L4** dans cet ordre. Chaque jalon se termine par `just prove-gold` (invoke in-cluster **HTTP 200**, hostname `plugin-{32hex}`) **et** le cas négatif du jalon. Pas de commit sauf demande humaine. Ne pas toucher `apparatus-p4-it` ni `apparatus-operator/tests/fixtures/kind/`.

Canon déjà livré (ne pas re-arbitrer) : [0605](adr/0605-gold-path-kind-j3-dns-attach.md), [0008](adr/0008-apparatus-p4-k8s-isolation-outside-manifesto.md), Cosign verify fail-closed au schedule, Calico **v3.29.7** sur `aiforall-local`, CIDR local **10.244.0.0/16**. Preuve : `just prove-gold`.

Décisions humaines **2026-09-26** (cette session de cadrage) :

| Sujet | Choix |
|---|---|
| Ordre | **Zot d’abord**, puis OpenBao, puis catalogue Bearer |
| Auth OpenBao | **Kubernetes auth** liée aux ServiceAccounts (pas AppRole, pas token root/périodique dans les Jobs) |
| NP `:8200` | **Dans le jalon OpenBao** : plus `0.0.0.0/0` ; seulement le CIDR hôte Compose joignable depuis Calico |
| Leftover | **Oui** : isoler encore seed SQL / Job enroll / Service `apparatus-reference-kv` hors nominal |
| Hors cette session | ADR-0009, ADR-0010, réécrire la preuve invoke en `exec` dans le pod (`gold-curl`), refermer [gold-path-kind-plugin-crashloop.md](gold-path-kind-plugin-crashloop.md), OIDC zot, mesh/Envoy |

---

## But

Le Kind `aiforall-local` doit permettre d’**évaluer et tester** localement les mêmes surfaces qu’on aura en prod : registre OCI (ACL, pas d’anonyme), catalogue de composants authentifié (même JSON), OpenBao (Transit ≠ KV plugin, identités machine). Les YAML / Jobs / recettes `just` restent le contrat **transmissible** : en prod on change URL, CA, et le backend d’auth Kubernetes, pas le protocole.

---

## État actuel (à casser)

| Surface | Fichier | Problème |
|---|---|---|
| zot | [deploy/p4/zot.yaml](../deploy/p4/zot.yaml) | `anonymousPolicy: ["read"]`. Un seul user htpasswd `signer`. Probe `GET /v2/` sans auth. |
| Adm-A / operator | [deploy/p4/admit-job.yaml](../deploy/p4/admit-job.yaml), [deploy/p4/operator.yaml](../deploy/p4/operator.yaml), [prove-gold-path.ps1](../deploy/apps/overlays/kind-demo-monolith/prove-gold-path.ps1) | `APPARATUS_VAULT_TOKEN=lazaret-dev-root` en clair. Operator verify Cosign via ce token. |
| OpenBao | [scripts/openbao-gold-seed.sh](../scripts/openbao-gold-seed.sh), Compose | Une instance `-dev`. Seed root. Transit + KV sur le même bureau, **aucune** policy séparée. |
| NP OpenBao | [deploy/p4/allow-admit-zot-np.yaml](../deploy/p4/allow-admit-zot-np.yaml) | egress `0.0.0.0/0:8200`. |
| Catalogue | [monolith/component-catalog-server.js](../monolith/component-catalog-server.js) | `GET /api/components` sans auth, bind `0.0.0.0:9000`. |
| Client catalogue | [Manifesto/infra/src/adapters/component_service_client.rs](../Manifesto/infra/src/adapters/component_service_client.rs) | Bearer **déjà** si `api_key` est Some. Config [Manifesto/configuration](../Manifesto/configuration/src/lib.rs) `component_service.api_key`. Gold ne le pose pas. |
| Leftover | overlay `kind-demo-monolith` | `seed-reference-kv-grants.sql`, `enroll-job.yaml`, `reference-kv-plugin-service.yaml`, `just debt-schedule-reference-kv`. Hors kustomization nominale **en partie** ; encore exécutables. |

Fixture IT zot (`apparatus-operator/tests/fixtures/zot/config.json`) **intouchable** dans cette session (même règle que `fixtures/kind`).

---

## Décisions d’implémentation (ne pas ré-ouvrir)

1. **Zot** : HTTP Basic + htpasswd. Deux identités : `signer` (create/update/read) et `reader` (read seulement). `anonymousPolicy: []`. Pas d’OIDC dans Adm-A. Secrets K8s (pas le ConfigMap htpasswd comme seul secret de runtime si on peut le monter en Secret). Transmissible = Harbor robot / GHCR PAT / ECR login-password : toujours Basic machine, on change URL + secret.
2. **Catalogue** : même JSON, même URL `GET /api/components`. Header `Authorization: Bearer <token>`. 401 sans Bearer / Bearer faux. **Pas** de nouvelle route Manifesto (0006 E). **Pas** d’objet OCI comme contrat d’attache.
3. **OpenBao** : une instance Compose `-dev` **conservée**. Deux mounts (Transit Cosign vs KV plugin). Deux policies. Auth **Kubernetes** : rôles liés aux SA Kind. **Zéro** `lazaret-dev-root` dans Job admit, Deployment operator, et si le monolithe J3 lit encore ce token pour le KV, le remplacer par le JWT SA / login Kubernetes. Root **uniquement** bootstrap (`openbao-gold-seed.sh`).
4. **NP `:8200`** : remplacer `0.0.0.0/0` par le CIDR de l’hôte Docker Desktop joignable depuis les pods Calico (`host.docker.internal` / IP pinée par `deploy-j3`, typiquement `192.168.65.254/32`). Ne pas réintroduire le CIDR pod `10.244.0.0/16` comme destination OpenBao.
5. **Leftover** : ne plus les appliquer dans `prove-gold` / kustomization overlay. Les garder derrière `just debt-*` ou les supprimer du cluster s’ils trainent. Ne pas les effacer du git s’ils servent encore un script debt documenté.

---

## Contraintes dures

- `just prove-gold` → enroll 200, session mTLS 200, invoke `kv.get` **200**, corps avec `hostname` du pod `plugin-{32hex}`.
- Cosign verify fail-closed **reste** (VALID forgé sans enveloppe signée → aucun Pod).
- Calico v3.29.7, kindnet interdit, CIDR `10.244.0.0/16`.
- Pas de client kube dans Lazaret. Service operator = nom du Pod.
- Pas de seed SQL / `admission.json` manuel comme étape nominale.
- Pas de GKE, pas de Factory, pas de commit.
- Probe zot : `GET /v2/` deviendra **401** sans anonyme — **changer la probe** (ex. httpGet avec header Basic, ou exec curl authentifié, ou endpoint health zot documenté). Un Deployment NotReady n’est pas un succès.

---

## L1 — Zot : plus d’anonyme, dual identités

**Pourquoi maintenant** : couper l’anonyme sans compte `reader` casse Cosign verify (operator + admit). D’où zot **avant** OpenBao.

### Travail

- [deploy/p4/zot.yaml](../deploy/p4/zot.yaml) : `anonymousPolicy: []` ; policy `reader` read-only ; `signer` inchangé en write. Deux lignes htpasswd (générer bcrypt, ne pas committer un mot de passe prod).
- Secret(s) K8s consommés par Job admit (`APPARATUS_REGISTRY_USER/PASSWORD` = signer) et operator (user `reader` pour verify). Retirer les passwords en clair des manifests si présents.
- Probe Ready : ne plus dépendre d’un `GET /v2/` anonyme 200.
- [prove-gold-path.ps1](../deploy/apps/overlays/kind-demo-monolith/prove-gold-path.ps1) : injecter les deux identités. Un GET anonyme `/v2/` ou `/v2/apparatus/envelope/tags/list` → **401**.

### Acceptation

| Cas | Attendu |
|---|---|
| `just prove-gold` | invoke 200 |
| GET zot sans auth | 401 |
| push avec `reader` | refus |
| push + sign avec `signer`, verify avec `reader` | OK (déjà le chemin Adm-A) |

**Delta prod à une ligne dans le README overlay / operator.yaml** : remplacer htpasswd par le mécanisme du registre cible ; mêmes env `APPARATUS_REGISTRY_*`.

---

## L2 — OpenBao Kubernetes auth + NP `:8200`

### Travail

OpenBao Compose doit **vérifier les JWT** des pods Kind :

- `auth enable kubernetes` (seed, root bootstrap seulement).
- `kubernetes_host` = API Kind **vue depuis le conteneur OpenBao** (pas `127.0.0.1` du laptop). En pratique `host.docker.internal` + port publié du control-plane Kind (souvent 6443). Documenter comment le lire (`kind get kubeconfig` / mapped port).
- CA du cluster Kind montée ou injectée (fail-closed si CA absente).
- Rôles (noms indicatifs, binders **exacts** SA/ns) :
  - `admit-sign` → SA `admit-sign` ns `aiforall-apparatus` → policy **Transit sign** sur `apparatus-p4-cosign` uniquement.
  - `controller` → SA `controller` ns `aiforall-apparatus` → policy **Transit read/verify**, **pas** sign.
  - KV plugin Lazaret : policy **uniquement** le mount KV plugin. Relier le SA gateway / le pod `oodhive-monolith` si c’est lui qui parle à Bao. Pas de Transit sur ce rôle.
- Job admit et Deployment operator : **plus** `APPARATUS_VAULT_TOKEN=lazaret-dev-root`. Login Kubernetes (token SA projeté) puis Cosign `hashivault://` / Vault API comme aujourd’hui.
- [scripts/openbao-gold-seed.sh](../scripts/openbao-gold-seed.sh) : crée mounts + policies + rôles K8s. Root reste pour ce script seulement.
- NP [allow-admit-zot-np.yaml](../deploy/p4/allow-admit-zot-np.yaml) : egress `:8200` vers **`192.168.65.254/32`** (ou l’IP `hostAliases` réelle), **pas** `0.0.0.0/0`. Si l’IP est recalée par `deploy-j3.ps1`, la NP doit suivre (patch JSON / même source d’IP). Un pod `apparatus-plugins` vers `:8200` reste **timeout** (preuve isolation, déjà le cas zot deny).

**Piège** : auth Kubernetes depuis un OpenBao **hors** cluster exige que Bao joigne l’apiserver (TokenReview). Si ça bloque sur Docker Desktop, corriger le `kubernetes_host` / extraPortMappings Kind — **ne pas** retomber sur le token root dans les Jobs. Ne pas inventer AppRole (décision humaine : K8s auth).

### Acceptation

| Cas | Attendu |
|---|---|
| `just prove-gold` | invoke 200 |
| Job admit sans projection de token SA | Cosign/Transit fail-closed, pas de VALID |
| Token root encore présent dans admit-job / operator env | **échec de jalon** |
| NP : pod plugins → `host.docker.internal:8200` | timeout / deny |
| Pod admit-sign → OpenBao:8200 | OK |

**Delta prod** (déjà [0008](adr/0008-apparatus-p4-k8s-isolation-outside-manifesto.md) D-TRANSIT-TCB) : instance Transit dédiée ≠ `-dev` ; unseal/HA/audit ; même **auth Kubernetes + policies**. Local `-dev` reste OK.

---

## L3 — Catalogue Bearer

### Travail

- [monolith/component-catalog-server.js](../monolith/component-catalog-server.js) : exiger `Authorization: Bearer <token>` (env `CATALOG_TOKEN` / équivalent). 401 sinon. JSON **inchangé**.
- [monolith/run-component-catalog.ps1](../monolith/run-component-catalog.ps1) + `just component-catalog` : passer le token.
- Monolithe J3 : poser `MANIFESTO_SERVICE__COMPONENT_SERVICE__API_KEY` (déjà câblé → Bearer). Même token. [prove-gold-path.ps1](../deploy/apps/overlays/kind-demo-monolith/prove-gold-path.ps1) : le poll `GET :9000/api/components` doit envoyer le Bearer ; sans header → 401 (ne plus accepter 200 anonyme).
- IT Manifesto qui stubent le catalogue via wiremock : **ne pas** casser (le stub de test n’est pas le serveur Node). Si un IT tape le stub Node, lui donner le token.

### Acceptation

| Cas | Attendu |
|---|---|
| GET catalogue sans Bearer | 401 |
| GET avec Bearer | 200, même schéma |
| `POST …/components` gold | 201 digest + `declared_capabilities` (0605) |
| `just prove-gold` | invoke 200 |

---

## L4 — Leftover hors nominal

### Travail

- Confirmer que [kustomization.yaml](../deploy/apps/overlays/kind-demo-monolith/kustomization.yaml) **n’inclut pas** `reference-kv-plugin-service.yaml` / `enroll-job.yaml` / seed SQL.
- `just prove-gold` : `kubectl delete` best-effort du Service leftover `apparatus-reference-kv` s’il existe encore.
- `just debt-schedule-reference-kv` reste le **seul** just visible pour l’ancien chemin ; bannières déjà présentes : les garder.
- Ne pas supprimer les fichiers git s’ils documentent la dette ; ne plus les appeler depuis L1–L3.

### Acceptation

- `just --list` : pas de `schedule-reference-kv` comme recette gold.
- Overlay apply nominal : pas de Service `apparatus-reference-kv`.
- `just prove-gold` 200 sans seed SQL.

---

## Preuve globale (fin de session)

1. `just prove-gold` → invoke **200**, hostname pod digest 32 hex.
2. Négatifs : zot anonyme 401 ; catalogue sans Bearer 401 ; VALID sans signature → aucun Pod ; plugins → zot:5000 timeout ; plugins → :8200 timeout.
3. `kubectl --context kind-aiforall-local` : aucun env `lazaret-dev-root` sur admit-job / operator.
4. `apparatus-p4-it` : `kind get clusters` encore présent et non retarget.

---

## Interdits

- Nouvelle ADR (assez sur le sujet). Mettre à jour **Réalité** seulement si un canon existant le demande ; ce guide n’est pas un Accept 0008.
- AppRole, OIDC zot, anonyme « le temps que ça passe ».
- `0.0.0.0/0:8200`.
- Token root dans un workload.
- Nouvelle route `/components`. Catalogue OCI comme source d’attache.
- Implémenter 0009 / 0010.
- Fusionner overlay démo dans M2 canon. SuperSéder 0601.
- Recréer `infra/` à la racine.
- `cargo test --workspace` comme preuve plateforme.

---

## Fichiers d’entrée (session)

- [deploy/p4/zot.yaml](../deploy/p4/zot.yaml), [admit-job.yaml](../deploy/p4/admit-job.yaml), [operator.yaml](../deploy/p4/operator.yaml), [allow-admit-zot-np.yaml](../deploy/p4/allow-admit-zot-np.yaml)
- [deploy/apps/overlays/kind-demo-monolith/prove-gold-path.ps1](../deploy/apps/overlays/kind-demo-monolith/prove-gold-path.ps1), [oodhive-monolith.yaml](../deploy/apps/overlays/kind-demo-monolith/oodhive-monolith.yaml)
- [scripts/openbao-gold-seed.sh](../scripts/openbao-gold-seed.sh), Compose OpenBao
- [monolith/component-catalog-server.js](../monolith/component-catalog-server.js), [Manifesto/infra/src/adapters/component_service_client.rs](../Manifesto/infra/src/adapters/component_service_client.rs)
- [deploy/kind/cluster.yaml](../deploy/kind/cluster.yaml) (CIDR : ne pas remettre `192.168.0.0/16`)

---

## Escalade

Si auth Kubernetes OpenBao ↔ Kind API est **impossible** sur Docker Desktop après un essai réel (TokenReview timeout, pas de port publié) : **stop** et poser la question. Alternative déjà écartée par l’humain : AppRole. Ne pas silencieusement remettre le root token.
