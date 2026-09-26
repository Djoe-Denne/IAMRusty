# Guide d’implémentation — cas classique local (gold path)

**Statut** : guide / contrat d’exécution. **Ce n’est pas une ADR.** Ne SuperSède **rien** (ni 0601, ni 0604, ni 0007–0008).

**Canon** : [ADR-0601](adr/0601-cluster-trust-namespaces-standalones.md) = cluster V1 **4+1** standalones.
**Démo** : [ADR-0604](adr/0604-j3-overlay-demo-monolith-kind-invoke.md) = overlay kind `kind-demo-monolith` (écart de séquence J3).
**Gold path local** : [ADR-0605](adr/0605-gold-path-kind-j3-dns-attach.md) = Kind J3 HTTP 200 ; DNS Service=Pod ; attach writer managed (Proposed / Implemented). Preuve : `just prove-gold`.
**Ce guide** = combler les trous pour qu’un **cas classique local** (monolithe + Compose + kind) fonctionne **sans** seed SQL ni admission manuscrite comme solution finale. Pas prod. Pas GKE.

Tranche / plans voisins : [platform-local-v1-implementation-contract.md](platform-local-v1-implementation-contract.md), [platform-local-monolith-kind-implementation-plan.md](platform-local-monolith-kind-implementation-plan.md), [platform-cloud-v1-implementation-contract.md](platform-cloud-v1-implementation-contract.md) (cloud hors scope ici).

**Rappel** : le 200 historique 0604 (Job `lazaret-enroll-reference-kv`) **n’est pas** la preuve gold 0605. Preuve actuelle = `just prove-gold` + enroll **workload** + hostname `plugin-{32hex}`. Ce n’est **pas** le parcours utilisateur unique. Cible de ce guide = signup → login → projet → ajout `io.aiforall.reference-kv` → appel aboutissant au pod, via les **chemins produit** déjà tranchés.

---

## Écarts gold case vs 200 technique (confrontation ADR)

| Limite | Verdict | Où |
|---|---|---|
| L1 — 200 ≠ signup → login → projet → add KV → invoke | Hors scope / écart de séquence (preuve J3 ≠ parcours utilisateur) | [0604](adr/0604-j3-overlay-demo-monolith-kind-invoke.md) §Décision point 3 ; [plan](platform-local-monolith-kind-implementation-plan.md) J1≠J3, Factory/host hors plan ; [0002](adr/0002-apparatus-contract-first.md) / [0406](adr/0406-crates-apparatus-p0-pas-le-host.md) / [0603](adr/0603-tranche-locale-deploy-kind-apparatus-lazaret.md) : P5/P6 non livrés |
| L2 — VALID à la main (`schedule-reference-kv` + CR) | Écart : interdit comme flux produit | [0008](adr/0008-apparatus-p4-k8s-isolation-outside-manifesto.md) point 2 : Adm-A **seule** source `VALID` ; Adm-B/C/D et consumer events→`VALID` interdits. [0005](adr/0005-apparatus-same-protocol-valid-verified.md) : harness / register ≠ admit |
| L3 — seed SQL grants / digest / consents | Écart : interdit comme solution finale | [0007](adr/0007-apparatus-p3-capability-boundary-after-accept.md) T4 GET domaine + T5 `PUT …/consents` Admin. « Style SQL … preuve de forme » = schéma `apparatus_bindings`, **pas** un seed grants |
| L4 — enroll par Job, pas le plugin au boot | Voulu démo vs cible produit | Produit : 0007 T3/T10 CSR-from-workload. Démo : 0604 Job/pod = preuve réseau. [0406](adr/0406-crates-apparatus-p0-pas-le-host.md) : `apparatus-reference-kv` ≠ runtime prod. Job comme **seule** finale = **pas** ratifié |
| L5 — curl hôte `:8080` ≠ Lazaret J3 in-cluster | Voulu : deux preuves | 0604 : localhost / extra-port = **faux amis**. Plan : J1 host vs J3 in-cluster. [0404](adr/0404-runtime-microservices-et-monolithe.md) dual ; [0601](adr/0601-cluster-trust-namespaces-standalones.md) : monolithe ≠ kind canon. **Gold path** = J3 ([0605](adr/0605-gold-path-kind-j3-dns-attach.md)) |
| L6 — hop / locator DNS | **Tranché** [0605](adr/0605-gold-path-kind-j3-dns-attach.md) | Operator : Service ClusterIP **même nom que le Pod** ; URL `http://plugin-{digest}.apparatus-plugins.svc:8080` ; Lazaret **sans** client kube ; op dans le corps HTTP ; StaticPluginLocator = dette |

---

## Décisions déjà tranchées (ne pas re-arbitrer)

| Sujet | Décision | Source |
|---|---|---|
| Identity install | Digest descripteur 0002, jamais `latest` | 0002 |
| VALID | **Adm-A** worker-only seule source ; harness / register / events → VALID **interdit** | 0005, 0008 |
| Isolation | Plugin hors processus privilégiés ; monolithe n’embarque pas le binaire | 0003 |
| Gateway | Lazaret `POST /invoke` (public monolithe `/lazaret/invoke`) ; pas d’`invoke` sur `ApparatusRuntime` Manifesto ; 0006 G/E en vigueur | 0004, 0007 |
| Consents | `PUT /api/projects/{project_id}/bindings/{component_id}/consents` (Admin) | 0007 T5 |
| Grants live | `GET /api/projects/{project_id}/bindings/{component_id}` + intersection Lazaret | 0007 T4 |
| Enroll / session | `POST /lazaret/enroll`, `POST /lazaret/session` ; CSR-from-workload défaut ; T11b mTLS HTTPS session | 0007 T3/T10/T11b ; `docs/services/lazaret.md` |
| Dual runtime | Standalones Compose **et** `oodhive-monolith` ; `sentinel-sync` **hors** nest | 0404 |
| Cluster canon | 4+1 ; monolithe ≠ Deployment kind/prod canon | 0601 |
| Overlay J3 | Démo non-canon ; preuve plugins→Service ; faux amis = extra-port / hostNetwork / localhost hôte | 0604 |
| Gold path Kind | **Option B (J3)** : monolithe in-cluster = seul Lazaret ; HTTP 200 Kind ; curl hôte ≠ preuve | 0605 |
| Hop / DNS | Service ClusterIP nom=Pod ; formule `plugin-{digest}` ; pas client kube Lazaret | 0605 |
| Attach digest / declared | Writer managed `POST …/components` → `insert_managed_binding` ; pas seed ; pas nouvelle route | 0605 |
| Factory / host UI | P5/P6, hors plan local | 0002, 0406, 0603, plan J* |
| Pre-prod scale | 0009 Proposed / Unimplemented ; HPA **Non décidé** — **ne pas implémenter** | 0009 |
| Pre-prod cert / CA | 0010 Proposed / Unimplemented ; CSR vs keypair **Non décidé** — **ne pas figer prod** | 0010 |

---

## Canon vs démo vs cas classique

| Surface | Rôle |
|---|---|
| 0601 + overlay `deploy/apps/overlays/kind` | Canon / stub M2 |
| 0604 + `kind-demo-monolith` | Démo J3 (plugin→Lazaret in-cluster) |
| 0605 | Gold path local Kind (s’appuie sur overlay 0604 sans unité prod) |
| Monolithe host + Compose | Laptop / cas classique (0404) — **pas** la preuve gold path |
| Ce guide | Exécution du **cas classique** ; réutilise chemins produit 0007/0008 ; n’érige pas le seed J3 en cible |

---

## Chaîne utilisateur attendue (URLs déjà présentes)

Préfixe monolithe (`docs/services/monolith.md`) : `/iam` `/hive` `/manifesto` `/telegraph` `/lazaret`. Standalone = même suffixe sans nest.

Chaîne documentée / scriptée (`monolith/prove-e2e-curl.ps1`) :

1. `POST /iam/api/auth/signup`
2. `POST /iam/api/auth/complete-registration`
3. (si besoin) `GET /iam/api/auth/verify?email=&token=`
4. `POST /iam/api/auth/login` → Bearer IAM (`iss=iamrusty`) — **≠** session Lazaret
5. `POST /manifesto/api/projects` (`owner_type` personal, `visibility` private)
6. `POST /manifesto/api/projects/{project_id}/components` body `{ "component_type": "io.aiforall.reference-kv" }`
   — poll 403 jusqu’à projection FGA (`sentinel-sync` **processus séparé**, 0404)
7. Consents produit : `PUT /manifesto/api/projects/{project_id}/bindings/{component_id}/consents` (0007 T5) — **pas** INSERT SQL
8. Workload : `POST /lazaret/enroll` puis session mTLS `POST /lazaret/session` (HTTPS, T11b) — **pas** Job démo comme finale
9. `POST /lazaret/invoke` (operation `kv.get`, etc.)

**N’inventer aucune URL.** Pas de nouvelles routes `/components` (0006 E). HTTP nouveau = Lazaret.

---

## Qui écrit quoi

| Donnée | Qui (cible produit) | Interdit comme finale |
|---|---|---|
| Ligne `project_components` + binding managed | `POST …/components` → `insert_managed_binding` (source=managed, desired_generation=1) | Seed SQL |
| `declared_capabilities` / digest | **Même writer** à l’attache : digest = pin catalogue / descripteur ; declared = manifeste du type ([0605](adr/0605-gold-path-kind-j3-dns-attach.md)) | Seed SQL ; plugin ; Adm-A ; nouvelle route `/components` |
| Consents + `grant_revision` | **T5** `PUT …/consents` (Admin) | INSERT `apparatus_capability_consents` |
| `AdmissionRecord` VALID | **Adm-A** (`apparatus-admit`) | `admission.json` manuel, Job schedule comme seule source, events→VALID, SQL Manifesto |
| Enrollment | Workload → `POST /lazaret/enroll` (CSR-from-workload T3/T10) | Job `lazaret-enroll-reference-kv` comme solution produit |
| Schedule pod plugin | Operator P4 (`deploy/p4`) sur digest **admis** | Contournement VALID ; retarget `apparatus-p4-it` |

---

## Quel Lazaret (gold path)

**Invariant** : le client du cas classique parle au **même** processus Lazaret qui **enroll** et **évalue** le grant.

**Tranché [0605](adr/0605-gold-path-kind-j3-dns-attach.md)** : gold path = **option B (J3)** — monolithe dans `aiforall-local` = **seul** Lazaret ; preuve = HTTP 200 Kind. Curl hôte `127.0.0.1:8080` **n’est pas** la preuve. 0601 (canon 4+1) et 0604 (overlay = écart) **restent**.

---

## Hop plugin (L6)

**Tranché [0605](adr/0605-gold-path-kind-j3-dns-attach.md)** : operator crée Service ClusterIP **du même nom que le Pod** ; hostname `plugin-{32 premiers hex du digest descripteur}` (`pod_name_for`) ; base URL `http://plugin-{32hex}.apparatus-plugins.svc:8080` ; opération dans le **corps** HTTP ; binding UUID ≠ hostname ; Lazaret **sans** API kube. Digest-as-DNS = gold path Kind **seulement** (prod → 0009). `StaticPluginLocator` + URL manuelle = **dette**, pas la cible.

---

## Questions encore ouvertes

- **Modalités CSR / CA** : renvoie [0010](adr/0010-gate-preprod-workload-certificate-ca.md) (et résidu 0007). Le guide construit G4 sur le défaut T3 hors prod.
- **ADR-0009** : mécanisme HPA / KEDA / pod-par-binding **Non décidé** — hors impl (marqueur pre-prod seulement).

---

## Jalons ordonnés + acceptation testable

### G0 — Prérequis plaque

`just up-infra` + Kind `aiforall-local` + `just deploy-j2` + `just deploy-j3` + catalogue hôte `:9000` + `sentinel-sync` hôte. Le monolithe **host** n’est **pas** le prereq gold. Transit = même OpenBao `-dev` (D-TRANSIT-TCB, dette locale). Cold wipe Kind ⇒ `deploy-j2` + `deploy-j3` avant `just prove-gold`. Delta prod D-TRANSIT-TCB : Transit dédié séparé du KV plugin ; seul `admit-sign` signe ; controller verify-only — local `-dev` reste OK.

**Acceptation** : Kind `aiforall-local` + overlay J3 Ready ; `GET http://127.0.0.1:9000/api/components` 200 ; sentinel-sync hôte (pas nest) ; OpenFGA joignable.

### G1 — Identité + projet + composant (sans SQL)

Chaîne IAM + `POST /manifesto/api/projects` + `POST …/components` `io.aiforall.reference-kv` → 201 (après grants FGA). Writer remplit `digest` + `declared_capabilities` (0605).

**Acceptation** : `component_id` UUID ; ligne `apparatus_bindings` managed **sans** script seed grants ; digest + declared présents.

### G2 — Consents produit (T5)

`PUT …/bindings/{id}/consents` Admin.

**Acceptation** : GET binding montre consents ; `grant_revision` bump ; **zéro** `seed-reference-kv-grants.sql` dans le chemin d’acceptation.

### G3 — VALID produit (Adm-A)

Admission worker écrit VALID pour le digest 0002.

**Acceptation** : CR/store VALID présent **sans** `schedule-reference-kv` seed manuel comme étape nominale ; controller refuse sans admission (0008).

### G4 — Enroll workload (T3/T10)

CSR depuis le workload → `POST /lazaret/enroll` sur le **même** Lazaret que G5 (nest J3).

**Acceptation** : ligne `apparatus_enrollments` ; second enroll autre empreinte → 409 ; Job démo **non** requis pour le critère.

### G5 — Session mTLS + invoke → pod

`POST /lazaret/session` (HTTPS, T11b) puis `POST /lazaret/invoke` (`kv.get`) jusqu’au pod isolé via formule DNS 0605.

**Acceptation** : HTTP **200** métier grant-allow Kind ; hop = Service nom=Pod ; **pas** de contournement mTLS ; **pas** curl hôte / extra-port comme preuve.

### G6 — Preuve plateforme plugins→Service

Gold path = J3 : prouver depuis `aiforall-plugins` vers Service nest ([0604](adr/0604-j3-overlay-demo-monolith-kind-invoke.md) / [0605](adr/0605-gold-path-kind-j3-dns-attach.md)).

**Acceptation** : conforme 0604/0605 ; distinct d’un curl localhost hôte.

---

## Interdits

- Seed SQL (`seed-reference-kv-grants.sql` et équivalents) comme **solution finale**
- Admission manuscrite / Job schedule comme **seule** source VALID
- Consumer events register → VALID
- Second chemin curl qui **ne** parle **pas** au Lazaret qui enroll
- Contourner mTLS session (0007 T11b)
- Implémenter ADR-0009 / HPA / KEDA
- Toucher / retarget `apparatus-p4-it`
- Nester `sentinel-sync` dans le monolithe
- Créer dossier `Factory/` ; vendre host UI / Factory comme livrés
- SuperSéder 0601 ; fusionner overlay démo dans M2 canon
- Extra-port / hostNetwork / localhost hôte comme preuve invoke in-cluster
- Nouvelles routes Manifesto « pour Lazaret » ; lever 0006 E/G sans Accept
- `cargo test --workspace` comme preuve plateforme
- Client kube dans Lazaret ; figer digest-as-DNS comme modèle prod

---

## Hors scope explicite

Factory / host UI (P5/P6) · 0602 / obs · GKE / live OpenTofu · J4 bascule 4+1 · ADR-0009 / 0010 impl · Cosign CI prod · commit (humain / orchestrateur). Calico v3.29.7 sur `aiforall-local` = gold path local (`deploy-j2`), pas un SuperSéde de 0604 « Calico hors J3 ».

---

## Escalade humaine

Avant Accept formel de [0605](adr/0605-gold-path-kind-j3-dns-attach.md) (ou autre ADR forward) : **décision humaine / PR**. J1↔J3 et hop locator **ne sont plus** ouverts (fermés par 0605). Ce guide ne change pas le Statut des ADR. 0009 et 0010 restent Proposed / Unimplemented (gates pre-prod).
