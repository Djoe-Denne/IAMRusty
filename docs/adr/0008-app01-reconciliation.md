# Compagnon Phase 0 — réconciliation APP-01 (pas une ADR)

- **Ce n’est pas** ADR-0008. **Pas** Proposed. **Pas** Accepted. **Pas** de SuperSède. **Méthode**, pas canon.
- **RATIFIÉ 2026-09-20** (« Je valide tout. Je ratifie tout. ») — même force que 0007 « accepté ».
- Canon : [`0008-apparatus-p4-k8s-isolation-outside-manifesto.md`](0008-apparatus-p4-k8s-isolation-outside-manifesto.md) — **Accepted** / Réalité **Implemented** (A-DEC 2026-09-22).
- Date : 2026-09-20
- Méthode : deux `architecte` indépendants (α sécu / β ops) par question, puis un réconciliateur. Priorités : (1) sécurité (2) maintenabilité (3) extensibilité ; sécu gagne.
- Gel humain déjà : moteur **K8s P4 hors Manifesto** ; K8s-as-P3 interdit ; A-DEC 2026-09-20 ; `invoke` = Lazaret ; WASM pas premier runtime ; harness jamais `VALID`/`VERIFIED` ; register Manifesto ≠ admit.
- T1 absence déjà livrée : ne pas relivrer. T2+ **débloqué** par l’Accept 0008 ; **ce tour ne pas implémenter T2+** (historique du tour d’écriture 2026-09-20). État courant (2026-09-22, A-DEC) : T2–T12 + M1–M6 **livrés** ; 0008 **Implemented**. Dette : **D-TRANSIT-TCB**, **D-ADMB**, **D-PROD**.

## Paquet ratifié (canon = ADR-0008)

`Reg-A+D-ACL (portable)` · `Adm-A` (worker sans HTTP) · `BC-A` · `Pkg-B-enveloppe + blob image CRI` · `Run-A`

| Q | Choix | Dissent α/β |
|---|---|---|
| 2 Registry | Cosign + OpenBao **Transit** (≠ KV Lazaret) + OCI Distribution **portable** + ACL push **signer-only** | α voulait ACL-D ; β refusait un registre in-cluster. Réconcilié : ACL **sur le registre**, pas K8s. |
| 3 Admission | Worker OS privilégié, **sans** BC HTTP ; seule source de `VALID` | α laissait HTTP ouvert ; β fermait. Réconciliateur : **sécu** → pas HTTP (surface IAM). |
| 4 BC | Pas de BC HTTP ; operator + Jobs ; skill `aiforall-new-service` **non** | α : BC-D (HTTP admit). β : BC-A. Réconcilié : indépendance 0005 = processus/SA + rapport signé, **pas** un GET. |
| 5 Package | Enveloppe OCI/ORAS **non** CRI + image pinée pour kubelet | α : Pkg-B. β : Pkg-A. **Sécu gagne** : le paquet n’est pas l’image. |
| 6 Processus | 4 SA K8s ; Job build **sans** token API ; plugins autre ns | Accords Run-A. β : CI T4/T5 = candidats seulement ; Kind pas dès T2. |

## Par question

### 2 — Registry + qui signe

**Pourquoi sécu.** Le builder exécute du code hostile. Transit : signer sans extraire la clé. ACL push : le builder ne pollue pas le repo admis. OpenBao existe (T12) ; un KMS cloud (Reg-B) ajoute un TCB. Reg-C unpack et Reg-D in-cluster comme TCB sont écartés.

**Risque.** Root OpenBao partagé = forge → isoler Transit du KV plugin. T12 `-dev` ≠ TCB de release.

### 3 — Autorité d’admission

**Pourquoi sécu.** 0005 fige l’indépendance ; il restait *qui / où / quoi atteste*. `VALID` = digest 0002 + politique + rapport, émis par une identité **≠** Manifesto, Lazaret, plugin, harness. Adm-B = enforceur runtime optionnel, **jamais** source. Adm-C/D rejetés.

**Risque.** Consumer d’events « register → VALID » = Adm-D furtif. Split admission vs signer encore ouvert (colocation 0003 possible, pas tranchée).

### 4 — Nouveau BC

**Pourquoi sécu.** Un nest Factory dans le monolithe colle kubeconfig au HMAC IAM. Le skill `aiforall-new-service` pousse JWT / events / FGA. Lazaret est un BC HTTP **parce que** `invoke` est un protocole ; Factory n’en est pas un. L’anti-fusion operator+admit se tient avec un **SA/processus Adm-A distinct**, sans listener HTTP.

**Risque.** Même binaire operator+admit. HTTP « debug » qui devient nest. Repli BC-D seulement si APP-02/P5 impose un POST hors-cluster (hors P4-core).

### 5 — Format de package

**Pourquoi sécu.** Identité d’install = digest descripteur 0002 seulement. Si le paquet **est** l’image (Pkg-A), kubelet peut exécuter des octets non admis. L’enveloppe n’est pas runnable CRI ; elle pin descripteur + image ; le Pod n’a que `image@sha256` issu de l’enveloppe **admise**.

**Risque.** IT `registry:2` image-only vs artifacts (zot/ORAS à prouver). Catalogue qui stocke le digest OCI à la place de 0002. `spec.image` = digest d’enveloppe.

### 6 — Lieu des quatre processus

**Pourquoi sécu.** 0003 = 4 identités OS. Moteur = K8s → 4 SA. Job build sans token API (propriété Run-C **dans** Run-A). Run-B : jeton CI = publish. Run-D : un hôte. Contrôleur P4 ≠ ticker P2 Manifesto.

**Risque.** SA build trop large. CI qui gagne `packages: write`. Confondre compose local et prod.

## Couplages (canon = ADR-0008)

- Q2 portable + Q5 enveloppe : le registre d’IT doit servir **artifacts et** pull d’image. Ne pas présumer `registry:2` suffisant.
- Q3 worker + Q4 BC-A : **alignés** (pas de 6ᵉ hexagone).
- Q4 operator + Q6 Run-A : contrôleur = SA cluster, **hors** `Manifesto/*/src`.
- Q2 signer-only + Q6 : seul le SA signer pousse / appelle Transit.
- Q5 image blob + CRI : compatible moteur K8s ; identité ≠ digest d’image.
- Adm-B webhook : complément **après** 0008, jamais source `VALID`.
- Zéro token `k8s`/`kubernetes` sous `Manifesto/*/src` même si l’operator vit ailleurs.

## Gel rappelé (inchangé)

G/E restent. APP-05 ouvert. Pas de second protocole. Pas de SuperSède 0003/0005. `invoke` = Lazaret. WASM pas premier runtime.

## Suite

Canon : ADR-0008 **Accepted** / **Implemented** (A-DEC 2026-09-22 ; T2–T12 + M1–M6 ; Kind = V1). Au tour d’écriture 2026-09-20, T2+ n’était **pas** à implémenter. Dette hors-jalon : **D-TRANSIT-TCB**, **D-ADMB**, **D-PROD**. Cette note est **méthode**, pas canon.
