# Contrat de jalons — Apparatus 0002→0008 fonctionnel

> **Ce n’est pas un prompt T2–T12.**  
> `docs/apparatus-p4-core-implementation-prompt.md` est **historique POST-LOT**, plus le contrat « maintenant ».  
> **Flips Réalité accordés A-DEC 2026-09-22** : 0002, 0003, 0005, 0008 → **Implemented** (Kind = V1). Pas d’ADR-0009. Pas de SuperSède 0003/0005. Dette : **D-TRANSIT-TCB**, **D-ADMB**, **D-PROD** — ne bloque plus Implemented.

Les mentions « Ne flip pas » du § (A) ci-dessous sont **historiques pré-A-DEC** (avant accord utilisateur du 2026-09-22).

Photo Réalité (canon + A-DEC, 2026-09-22) — à ne pas rediscuter :

| ADR | Réalité | Sens |
|---|---|---|
| 0001 | Partial | tenants / backfill (`D-LEGACY`) hors chaîne M1–M6 |
| 0002 | Implemented | pin descripteur 0002 + chaîne M1–M6 ; Factory/host = P5/P6 **après** |
| 0003 | Implemented | isolation Kind/Calico + 4 SA sur chemin invoke ; harness test-only OK |
| 0004 | Implemented | gateway = Lazaret ; `APP-05` **non tranché** |
| 0005 | Implemented | Adm-A seule source `VALID` ; install fail-closed ; 0 `trusted_*` ; `VERIFIED` = P6 après |
| 0006 | Implemented | G/E **en vigueur = succès** du titre P2 |
| 0007 | Implemented | T1–T14b ; holes hors-jalon (`D-APP05` / `D-0006G` / `D-0006E` / `D-LEGACY`) |
| 0008 | Implemented | T2–T12 + M1–M6 Kind V1 ; dette D-TRANSIT-TCB / D-ADMB / D-PROD |

**Déjà livré — un milestone qui les rejoue est FAUX :** P3 T1–T14b ; P4 T1 ; operator T2–T12 ; preuves 22 sept. (pin 64 hex, pull zot `IfNotPresent`, RBAC `can-i`, CR `NotValid`, automount) ; Calico T11 `t11_plugin_exec_cannot_reach_transit_or_system` vert ; Cosign fail-closed (tag `.sig` ≠ preuve). S-1 et S-3 = **faits** (briefing 0756).

Cible utilisateur : un Apparatus **piné** (0002) peut être **admis** (0005 / 0008 Adm-A), **isolé** (0003 / 0008), **installé seulement si `VALID`**, **invoke** via Lazaret (0004 / 0007) — **bout en bout**, pas seulement des tests de crate.

0001 tenants / backfill (`D-LEGACY`) : **hors chaîne**. P1 SQL bindings suffisent. Si un lot le bloque, le nommer — ne pas l’enterrer.

---

## (A) Lots d’implémentation — pour que ça marche

Six jalons **séquencés**. Réutiliser T2–T12 / T11 / Cosign ; ne pas les relivrer. BC-A = `apparatus-operator` + Jobs (pas `Factory/` racine, pas nest HTTP, pas 6ᵉ hexagone). `invoke` reste Lazaret (`D-0006G`).

### M1 — Pin produit de l’Apparatus de référence (Pkg-B)

**But.** L’Apparatus KV de référence (`apparatus-reference-kv`) devient une **enveloppe pinée** : digest descripteur **0002** (64 hex, jamais `latest`) + blob image CRI distinct. C’est le premier artifact de la chaîne, pas un rejeu T3–T6.

- **ADRs :** 0002, 0008 (Pkg-B, Reg-A+D-ACL déjà livrés)
- **IN :** enchaîner digest / build / sign **existants** (`apparatus-operator/src/digest.rs`, `build.rs`, bins `apparatus-build`) sur le manifeste + image de référence ; identité catalogue = digest descripteur, `spec.image` = `name@sha256` issu de l’enveloppe
- **OUT :** répertoire `Factory/` ; host UI / iframe / CLI ; image auteur P6 ; relivrer T3–T6 ; IAM 0407–0410 ; cloud 0600–0602
- **Preuves :** un artifact zot dont le descripteur 0002 est 64 hex ; le digest d’enveloppe ≠ identité ; pull `IfNotPresent` déjà vrai — ne pas le réécrire. Test nouveau (pas T3) : `apparatus-operator/tests/` *ou* IT Kind qui nomme **l’Apparatus de référence**, pas seulement `testdata/platform-plugin`
- **Livré (2026-09-22) :** `m1_reference_kv_pinned_envelope_on_zot` (`apparatus-operator/tests/apparatus_m1_reference_kv_pin.rs`) vert.
- **Ne flip pas :** 0002, 0008 (reste Partial)

### M2 — `VALID` persisté hors store Kind (Adm-A seule source)

**But.** Le rapport Adm-A (digest 0002 + version de politique + rapport) devient un **fait catalogue / runtime produit** lisible hors `InMemoryAdmissionStore` et hors une CR Kind éphémère. Avant ce jalon : 0 `VALID`/`VERIFIED` persistés côté produit (0005).

- **ADRs :** 0005, 0008 (Adm-A)
- **IN :** Adm-A (`apparatus-operator/src/admission.rs`, bin `apparatus-admit`, SA `apparatus-admit`) **seule** source ; persister digest + `policyVersion` + `reportDigest` (forme CR `AdmissionRecord` `apparatus.aiforall.dev` déjà spécifiée) dans un store **produit** que l’install peut lire ; register Manifesto ≠ admit
- **OUT :** harness / `apparatus dev` écrivant `VALID` ; `VERIFIED` éditorial ; Adm-B comme source ; `trusted_skip_gateway` ; second protocole ; Manifesto qui auto-admet
- **Preuves :** après admit réussi, une lecture produit (pas seulement `HashMap` T7) renvoie `VALID` pour ce digest 0002 ; refus Adm-A ⇒ **aucune** ligne `VALID` ; `claimed_verified` ignoré (déjà T7 — ne pas relivrer, étendre au store produit)
- **Livré (2026-09-22) :** `m2_admit_get_returns_valid_for_m1_digest` (`apparatus-operator/tests/apparatus_m2_valid_persist.rs`) vert.
- **Ne flip pas :** 0005, 0008

### M3 — Install fail-closed si non-`VALID`

**But.** Le runtime n’installe (ne schedule un plugin) **que** si M2 a un `VALID` pour ce digest 0002. Un digest enregistré, signé, ou « connu » sans Adm-A reste non installable.

- **ADRs :** 0005 (installabilité), 0008 (contrôleur, Pkg-B)
- **IN :** `apparatus-controller` (SA `apparatus-controller`) refuse `NotValid` / `MissingAdmissionRecord` (logique T8/T10 **déjà** là) sur le **chemin produit** ; kubelet n’exécute que `image@sha256` de l’enveloppe **admise** ; plugins ns `apparatus-plugins`
- **OUT :** relivrer T8/T10 ; PSS ; drain / destruction des bindings `ready` (0005 Non décidé) ; APP-03 / APP-06
- **Preuves :** digest non admis ⇒ pas de Pod plugin ; digest `VALID` ⇒ Pod piné ; observable CR / events sans `ErrImageNeverPull` (déjà T10 — étendre au digest de référence M1)
- **Livré (2026-09-22) :** `m3_kind_cr_only_valid_does_not_schedule` (`apparatus-operator/tests/apparatus_m3_install_fail_closed.rs`) vert.
- **Ne flip pas :** 0005, 0008

### M4 — Pont desired_state → operator (sans K8s dans Manifesto)

**But.** Un binding Manifesto `ready` / `desired_state` (ticker P2 **inchangé**) déclenche M3. Manifesto reste ignorant de Kubernetes.

- **ADRs :** 0001 (binding = `ProjectComponent`, **pas** tenants), 0006 (G/E), 0008 (BC-A, contrôleur ≠ ticker)
- **IN :** l’operator **observe** l’état déjà persisté (P2) depuis **hors** `Manifesto/*/src` — lecture des **5** routes `/components` (`D-0006E`) ou lecture DB hors Manifesto ; 4 SA déjà nommées (`apparatus-build`, `apparatus-admit`, `apparatus-controller`, `apparatus-gateway`)
- **OUT :** token `k8s` / `kubernetes` sous `Manifesto/*/src` (gates T1/P2 T7/P3 T2 **non retargetés**) ; 6ᵉ route / HTTP 202 ; nest dans le monolithe ; skill `aiforall-new-service` ; nouvel event P2 ; `invoke` sur `ApparatusRuntime` ; backfill 0001
- **Preuves :** un `ProjectComponent` ready dont le digest est `VALID` ⇒ le contrôleur schedule ; même binding avec digest non `VALID` ⇒ pas de schedule ; `rg` gates Manifesto restent verts
- **Bloqueur 0001 ?** Non. P1 SQL suffit. Tenants / `D-LEGACY` restent hors chaîne.
- **Livré (2026-09-22) :** `m4_ready_valid_digest_schedules` (`apparatus-operator/tests/apparatus_m4_desired_state_bridge.rs`) vert.
- **Ne flip pas :** 0001, 0006, 0008

### M5 — Invoke E2E d’un plugin isolé via Lazaret (ferme T-3)

**But.** Un plugin **admis + isolé** (ns `apparatus-plugins`, Calico déjà vert) n’est joignable en I/O que par Lazaret `POST /invoke`. T12 P4 = scan fichiers — **ce n’est pas** cette preuve.

- **ADRs :** 0003, 0004, 0007 (G/E), 0008 (Run-A)
- **IN :** transport = identité de workload P3 déjà livrée (instance, binding, release, génération, `grant_revision`, audience gateway, session mTLS T11b) ; le plugin **sort** vers Lazaret ; Lazaret **n’est pas** client kube ; plugin hors processus Manifesto / monolithe / operator
- **OUT :** `invoke` sur `InProcessApparatusRuntime` / `ApparatusRuntime` (`D-0006G`) ; `trusted_skip_gateway` ; 2ᵉ protocole ; K8s dans Lazaret ; relivrer T7 P3 / T11 P4 / T12 P4
- **Preuves :** nouveau test (pas `apparatus_p4_t12_gate_regression`, pas `apparatus_p3_t7_invoke` seul) : pin M1 + `VALID` M2 + install M3 + `POST /invoke` Lazaret **atteint** le Pod isolé ; T11 reste vrai (plugin n’atteint pas Transit / `apparatus-system`) ; projet public sans consentement n’ouvre pas invoke (T7 0007 — ne pas relivrer)
- **Livré (2026-09-22) :** `m5_invoke_reaches_isolated_plugin_pod` (`apparatus-operator/tests/apparatus_m5_invoke_isolated.rs`) vert.
- **Ne flip pas :** 0003, 0004 (déjà Implemented), 0007 (déjà Implemented), 0008

### M6 — Chaîne unique de preuve (un seul IT bout en bout)

**But.** Un scénario unique, pas six crates : référence pinée → Adm-A → `VALID` persisté → install → invoke Lazaret → isolation tient.

- **ADRs :** 0002–0008 (preuve transversale)
- **IN :** Kind `apparatus-p4-it` + Calico v3.29.7 déjà exigé ; Lazaret réel ; Manifesto bindings P1 ; Apparatus de référence
- **OUT :** rejeu T2–T12 ; host P5 ; PSS ; catalogue tiers P6 ; flip Réalité dans les ADR
- **Preuves :** un test nommé (ex. `tests/apparatus_e2e_0002_0008_invoke.rs` **ou** équivalent IT) vert une fois ; logs / CR / `POST /invoke` observables alignés sur le même digest 0002
- **Livré (2026-09-22) :** `m6_e2e_0002_0008_chain` (`apparatus-operator/tests/apparatus_m6_e2e_chain.rs`) vert.
- **Ne flip pas :** aucune ADR (les flips sont la section B, après accord)

---

## (B) Conditions de flip Réalité → Implemented

**Flips accordés A-DEC 2026-09-22.** Ce ne sont **pas** des lots de code. Le tableau ci-dessous reste l’**historique** des critères satisfaits par M1–M6 / Kind = V1. Les anciennes conditions TCB (Transit hors `-dev`, Adm-B, cluster prod) = **dette hors-jalon** — ne bloquent plus Implemented (comme `APP-05` pour 0007).

| ADR | Critères satisfaits (M1–M6 / Kind V1) | Reste hors flip (= dettes) |
|---|---|---|
| **0002** | M1+M6 : identité d’installation **produit** = digest descripteur 0002 ; contrats + KV de référence = pin réel | Factory/host = P5/P6 après ; D-TRANSIT-TCB / D-ADMB / D-PROD |
| **0003** | M5+M6 : plugin hors processus privilégiés sur **chemin invoke** ; 4 SA Run-A | Harness test-only peut rester ; dettes ci-dessus |
| **0005** | M2+M3+M6 : `VALID` persisté ; install seulement si `VALID` ; 0 `trusted_*` | `VERIFIED` éditorial ; APP-02 ; drain bindings `ready` ; D-ADMB |
| **0008** | T2–T12 + M1–M6 ; Adm-A seule source `VALID` | **D-TRANSIT-TCB** (Transit `-dev`) ; **D-ADMB** (webhook jamais source `VALID`) ; **D-PROD** (cluster prod N/A) |
| 0004 / 0006 / 0007 | Déjà Implemented (A-DEC antérieur) | `APP-05`, G/E, 2ᵉ protocole restent hors-jalon |

Interdit : SuperSède 0003/0005 ; lever G/E pour « aider » un flip ; traiter T1 absence comme TCB prod.

---

## (C) Décisions humaines (oui / non — pas du code)

Ces items **ne sont pas des jalons d’implémentation**. Un lot qui les « ferme » tout seul est hors contrat.

| ID | Question | Bloque le code E2E (A) ? | Bloque un flip (B) ? |
|---|---|---|---|
| **APP-05** A/B (vit dans 0004) | A = V1 privé durable ; B = lame publique plus tard (nouvelle ADR) | **Non.** V1 privé déjà testé (`t7_public_project_without_consent_does_not_open_call`). `D-APP05` reste ouvert | Non pour 0004 (déjà Implemented) |
| **Lever G/E** (0006 / 0007) | Ajouter `invoke` sur `ApparatusRuntime` ; gateway sous Manifesto ; 6ᵉ route / 202 | **Oui si on l’essaie.** Le code **doit** continuer sans les lever (`D-0006G`, `D-0006E`) | Lever ≠ preuve 0002/0003/0005 |
| **Transit TCB hors `-dev`** | OpenBao Transit release, isolé du KV Lazaret | **Non** pour l’IT Kind | **Non** (A-DEC) — reste **D-TRANSIT-TCB** |
| **Adm-B oui / non** | Webhook enforceur runtime **après** 0008 | **Non** pour E2E (Adm-A suffit) | **Non** (A-DEC) — reste **D-ADMB**. Jamais source `VALID` |

0008 = **Implemented** (A-DEC) avec dettes ci-dessus ; M1–M6 verts ne requièrent plus un second flip.

---

## Hors chemin critique (optionnel, après E2E)

**Un** lot optionnel, **pas** sur M1–M6 : PSS / `APP-03` / `APP-06` (CPU/budget) / P6 catalogue tiers / host UI P5 / iframe / CLI / isolation east-west S-2 / Forbidden RBAC réel (T-8). Ne pas les ouvrir pour « finir 0002→0008 ».

---

## Invariants (tous les lots)

- K8s-as-P3 interdit ; zéro token `k8s`/`kubernetes` sous `Manifesto/*/src`
- Pas de 2ᵉ protocole / `trusted_skip_gateway`
- Pas de `Factory/` racine ; pas de nest HTTP ; pas de 6ᵉ hexagone
- `invoke` = Lazaret seulement ; G/E en vigueur
- Adm-A seule source `VALID` ; Adm-B jamais source
- Identité d’install = digest descripteur 0002 ≠ digest enveloppe ≠ `spec.image`
- Transit ≠ KV plugin Lazaret
- 0001 tenants/backfill hors chaîne
- IAM 0407–0410 et cloud 0600–0602 hors sujet

## Escalades

- Accord humain **requis** avant tout flip Réalité (B) et avant Accept d’une nouvelle ADR.
- Accord humain **requis** pour lever G/E, clôturer `APP-05`, TCB Transit, Adm-B.
- Si M4 ne peut pas observer le desired_state sans nouvelle route Manifesto : **escalader** — ne pas inventer une 6ᵉ route ni un event P2.
- Si M5 exige que Lazaret devienne client kube : **escalader** — contraire à 0008 (operator/Jobs hors Lazaret) et à 0004 (I/O = gateway, identité workload).
