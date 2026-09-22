# Prompt — Implémentation Apparatus P4-core (T2–T12)

> **POST-LOT (2026-09-22).** Ce fichier **n’est plus** le contrat d’implémentation « maintenant ». T2–T12 **livrés** dans `apparatus-operator`. **Ne pas relivrer** P4-core. ADR-0008 **Accepted / Partial** (**pas** Implemented). Un prochain plan ≠ rejouer ce prompt. T1 absence : ne pas relivrer. Le corps ci-dessous est le contrat **historique** du lot usine (TDD T2–T12).

Tu travailles dans le dépôt `C:\Users\djden\source\repos\AIForAll`. **Ne commite rien.**

## Contrainte modèle

Politique utilisateur **courante** (2026-09-20). **Ne pas** recopier la contrainte P3-era « seul Grok pour tout ».

| Rôle | Slug Cursor (`model:` de chaque Task / sous-agent) |
|---|---|
| Orchestration, architecture, implémentation difficile, revues avec jugement | `cursor-grok-4.6-xhigh` |
| Explore, mécanique, test-reviewer | `composer-2.5-fast` |

Slugs **autorisés seulement** : `inherit`, `composer-2.5-fast`, `cursor-grok-4.6-xhigh`, `muse-spark-1.3-max`.

- Ne jamais choisir **silencieusement** un autre modèle.
- Un agent lancé hors table / hors slugs autorisés = non conforme ; arrêter et relancer.
- `inherit` n’autorise pas à dévier vers un slug absent de la liste.

## Mission

Implémente **P4-core** : premier workload **isolé** (Kubernetes **hors** Manifesto) **et admis** (Adm-A = seule source `VALID`), image de test **plateforme**, `invoke` **toujours** Lazaret.

TDD strict RED-GREEN-REFACTOR, **une tranche après l’autre** (T2 → T12). Ne relivre **pas** T1. Ne dépasse pas P4-core. Ne livre **pas** P5/P6.

**Canons primaires (ne pas SuperSéder, ne pas reratifier)** :

- **ADR-0008** Accepted / Réalité **Partial** (post-lot T2–T12 livrés ; T1 ≠ mécanisme) — moteur, Cosign+Transit, Adm-A, BC-A, Pkg-B, Run-A. **Pas** Implemented (TCB Transit `-dev`, Adm-B, PSS).
- **ADR-0005** : un protocole `manifesto-apparatus/1` ; admission / `VALID` / `VERIFIED` / installabilité distincts ; register Manifesto ≠ admit ; harness jamais `VALID`/`VERIFIED` ; pas de `trusted_skip_gateway`.
- **ADR-0003** : plugin hors processus privilégiés ; quatre identités OS (build, admission/signature, contrôleur, gateway) ; harness ≠ prod ; WASM **pas** premier runtime V1.
- **ADR-0002** : identité d’installation = digest descripteur (`ReleaseDigest` `sha256:`+hex), jamais `latest` ; Factory = P4 **comme jalon**, **pas** comme répertoire ; host = P5 OUT.
- **ADR-0004 / 0007** Implemented (A-DEC) : `invoke` = Lazaret `POST /invoke` ; 0 `gateway` sous `Manifesto/*/src` ; `/components` = 5.

N’invente **aucune** décision `Accepted` au-delà de 0008. Les **choix locaux** ci-dessous sont réversibles P4-core, **pas** une ADR.

## Scénario usine (acceptation du lot)

Îlot **presses-nord**, quart 03:00. Le plugin plateforme `alarme-palier-3` (identité = digest **0002**, pas le tag Git `main`) boucle.

L’opérateur non-dev doit pouvoir :

1. **Faire confiance au digest** — un `AdmissionRecord` `VALID` n’existe que s’il a été écrit par Adm-A (politique + rapport), pas par Manifesto register, pas par le harness, pas par le Job de build.
2. **Tuer la cellule** — Pod/Job dans `apparatus-plugins` ; Manifesto, Lazaret et les autres îlots restent debout.
3. **Ne pas relancer `latest`** — reprise seulement depuis l’enveloppe admise (descripteur 0002 + `image@sha256` CRI pinée).
4. **Constater l’isolation** — le plugin n’a ni token API, ni Transit, ni push registry ; NetworkPolicy **réelle** (pas un mock YAML).
5. **Invoquer encore via Lazaret** — `POST /invoke` P3 toujours vrai ; **pas** d’`invoke` sur Manifesto `ApparatusRuntime`.

Sans T2–T12 **ensemble**, ce scénario est faux : scaffold ≠ cellule ; admission sans runtime = registre mort ; runtime sans admission = kubelet d’octets non admis (interdit Pkg-B / 0005).

## Gel A-DEC (2026-09-20) — ne pas rouvrir

| Fait | État |
|---|---|
| ADR-0004 / 0007 | Accepted / **Implemented** |
| Preuve P3 | T1–T14b livrés ; **ne pas relivrer** |
| `APP-05` | **Ouvert** — hors lot |
| 0006 G / E | **En vigueur** — ne pas lever (pas d’`invoke` sur `ApparatusRuntime` ; `/components` = 5) |
| K8s-as-P3 | **Interdit** |
| Second protocole / `trusted_skip_gateway` | **Interdit** |
| Invoke | Lazaret `POST /invoke` |
| `gateway` sous `Manifesto/*/src` | **0** |
| Gate Manifesto | `apparatus_p2_t7_gate.rs` + `apparatus_p3_t2_gate.rs` + T1 P4 : **ne pas** retargeter pour autoriser `k8s` **dans Manifesto** |
| APP-01 | **ADRé (0008)** — ne pas « re-réconcilier » |
| SuperSède 0003 / 0005 | **Interdit** |
| Flip 0005 (ni 0002/0003) `Implemented` | **Interdit** en slice-1 |

Compagnon [`docs/adr/0008-app01-reconciliation.md`](adr/0008-app01-reconciliation.md) = **méthode**, pas canon.

## Accepted vs Réalité

`Accepted` = cible. `Réalité` = code. Ne les confonds pas. Ne réécris pas `0001`–`0008` Accepted.

Wiki / QMD / plan L152 `KubernetesAdapter` / factory `proposed` = **conception**. Interdit de les copier comme contrat RED.

Post-lot (2026-09-22) : 0008 Réalité **est** **Partial** (mécanisme IT Kind, pas TCB prod). 0002 / 0003 / 0005 **restent Partial**. T12 OpenBao `-dev` ≠ TCB de release.

### Contradictions ADR / code / wiki (photo **pré-lot** — le lot T2–T12 est **livré**)

Les points ci-dessous décrivent l’état **avant** le lot usine ; ce n’est **pas** une liste de travail ouverte. Wiki / plan peuvent encore contenir des photos ; **canon courant** : 0007 Implemented (A-DEC), 0008 Accepted / **Partial** (T2–T12 livrés).

1. Wiki QMD `decisions/index` : 0004+0007 encore Partial ; **0008 absent** ; plan L190 APP-01 encore « ouvert » — **photo antérieure**.
2. Hub wiki `0007-apparatus-p3-lazaret.md` `feature_status: partial` vs ADR Implemented — **photo antérieure**.
3. Plan L152 « implémenter KubernetesAdapter » vs 0008 : **aucun** type K8s sous `Manifesto/*/src` — toujours vrai.
4. Closeout `D-P4` sac « Factory + host UI » vs 0002 : host = **P5 OUT** — toujours vrai.
5. GrepAI index `last_updated` 2026-08-28, RPG disabled — **ne pas** s’y fier.
6. QMD collection : 0 hits `0008` — possible décalage d’index, pas une mission de lot.

**Ne pas** relivrer P4-core pour « corriger le wiki » à la place du mécanisme déjà livré.

Si une contradiction **bloque** (2e proto, bearer IAM, K8s dans Manifesto, nouveau type FGA, lever G/E, clôturer APP-05, budget/CPU numériques APP-06) : **escalade humaine**, n’invente pas.

## Sources de vérité à lire avant de coder

1. `AGENTS.md` et règles workspace.
2. `docs/adr/README.md` L18 (`APP-01` ADRé ; `APP-02`…`APP-07` ouverts) ; L135–L137.
3. ADR `0001`–`0008` dans `docs/adr/`. Canon P4 = **0008**. Preuves d’absence : `Manifesto/tests/apparatus_p4_t1_absence.rs`, `apparatus_p2_t7_gate.rs`, `apparatus_p3_t2_gate.rs`.
4. Contrats P0 : `apparatus-contracts` (`ReleaseDigest`, `digest_manifest`, protocole `manifesto-apparatus/1`, DTO sans `trusted_skip_gateway`). Harness `apparatus-contracts/src/harness.rs` = TEST-ONLY, jamais `VALID`/`VERIFIED`.
5. Runtime Manifesto **à ne pas élargir** : `Manifesto/infra/src/apparatus_runtime/{mod,in_process,tick,cleanup}.rs` — `bind` / `configure` / `unbind` / `observe` / `teardown` **seulement**. Contrôleur P4 ≠ ticker P2.
6. Invoke à **préserver** : `Lazaret/application/src/invoke.rs`, `Lazaret/http/src/invoke.rs`, `Lazaret/tests/apparatus_p3_t7_invoke.rs` (+ t8…t13).
7. OpenBao KV plugin **déjà** : `Lazaret/tests/fixtures/openbao/` (`lazaret_test-openbao`, mount `secret`, image `openbao/openbao:2.6.2`). **Ne pas** réutiliser ce mount / ce token / ce nom de container pour Transit.
8. Skills : `.cursor/skills/creating-testcontainer-fixtures/SKILL.md` (fixtures **locales**). Skill **`aiforall-new-service` NON**. RustyCog HTTP / OpenFGA / JWT / nest monolithe **NON**. `ServiceTestDescriptor` : **ne pas** ajouter `has_openbao` / `has_k8s` (casserait tous les services).

`apparatus-events/` existe mais **n’est pas** member workspace — **ne pas** en faire un bus « register → VALID » (Adm-D furtif, 0008 compagnon).

## Choix locaux P4-core (réversibles, **pas** une ADR)

Canon 0008 laisse libres : nom du crate BC-A, produit registry IT, split admit vs signer, Kind dès quelle tranche. **Tranché ici** pour un implementer unique. Revenir dessus = note de suivi, **pas** SuperSède.

| Sujet | Choix local | Pourquoi usine | Interdit |
|---|---|---|---|
| Crate BC-A | Member workspace **`apparatus-operator`** à la racine (`apparatus-operator/`). Un crate : lib + bins. | Un îlot ops, pas un 6ᵉ hexagone | `Factory/` ; PascalCase service nest ; skill new-service ; entrée `monolith/Cargo.toml` |
| Bins | `apparatus-controller` ; `apparatus-admit` (signe **puis** atteste) ; `apparatus-build` (entrypoint Job) | 3h du matin : trois processus visibles, zéro listener HTTP d’admission | `axum` / `create_router` sur admit ou build |
| Admit vs signer | **Colocation d’identité** : un SA `apparatus-admit` (libellé 0003 « admission/signature » + Run-A **4** SA). Deux *fonctions* séquentielles dans le même binaire privilégié, **pas** deux HTTP. | 4 SA = quota canon ; 5ᵉ SA = escalade | Split en 5 SA sans ADR ; consumer d’events Manifesto |
| 4 SA Kind | `apparatus-build` (`automountServiceAccountToken: false`) ; `apparatus-admit` ; `apparatus-controller` ; `apparatus-gateway` | Kill / sign / schedule / invoke = identités séparées | Token API sur le Job build ; Transit sur build ou plugin |
| SA gateway | Identité K8s **future de Lazaret** (0003 « capability gateway »). **Pas** un 4ᵉ binaire d’admission, **pas** un 2ᵉ `POST /invoke`. Ce lot **n’héberge pas** Lazaret in-cluster. | Quart : tuer le plugin ≠ tuer la gateway | Binaire `apparatus-gateway` HTTP ; nest invoke dans l’operator |
| Namespaces | `apparatus-system` (privilégié) ; `apparatus-plugins` (workloads) | Tuer un plugin ≠ tuer Manifesto | Plugin dans le ns du contrôleur |
| Registry IT | **zot** (OCI artifacts **et** pull d’image). Auth : seul le signer pousse ; pull image possible pour kubelet. | `registry:2` image-only **casse** Pkg-B | Prouver T6/T10 avec `registry:2` seul |
| Kind | **À partir de T10**. T2–T9 = testcontainers Docker (zot + OpenBao Transit) **sans** cluster | β 0008 : pas Kind dès T2 ; usine cellule = T10 | Mock NetworkPolicy ; Kind « pour le scaffold » T2 |
| OpenBao Transit | Fixture **locale** `apparatus-operator/tests/fixtures/openbao-transit/` ; container ≠ `lazaret_test-openbao` ; engine **Transit** ≠ mount KV `secret` ; token root **distinct** | Forge de signatures si Transit = KV T12 | Partager le fixture Lazaret T12 |
| `VALID` persisté | CRD **`AdmissionRecord`** (cluster `apparatus-system`), écrite **uniquement** par SA admit. Champs min. : digest descripteur 0002, version de politique, digest du rapport. Contrôleur **refuse** de scheduler sans CR. | Ops : `kubectl get admissionrecords` à 3h ; pas de GET IAM | Colonne SQL Manifesto ; auto-admit sur register ; harness |
| Adaptateur runtime | Dans **`apparatus-operator`** (ex. `WorkloadReconciler` / `envelope_to_podspec`). **Jamais** de type `KubernetesAdapter` sous `Manifesto/*/src` | Plan L152 = conception | Retargeter les gates Manifesto |
| Dépendances K8s / Cosign | `kube` / `k8s-openapi` **seulement** dans `apparatus-operator`. Cosign = **CLI** dans l’image admit (`cosign sign` + Transit/Vault API). | Preuve usine = outil nommé 0008 | Dep kube dans Manifesto/Lazaret |
| Health T2 | **Pas** de HTTP health 0008. Équivalent : bins `--version` / lib compile / CRD apply à T10 | Évite un nest « debug » | Listener HTTP « temporaire » |
| Invoke → Pod | Lot = Job/Pod isolé **plus** régression Lazaret P3. Mesh Lazaret-in-cluster **hors** lot. IT T12 **peut** prouver qu’un client protocole depuis SA gateway atteint le Service plugin **dans Kind** (même contrat `manifesto-apparatus/1`), sans second proto. | Cellule tuable + invoke historique intact | Bearer IAM ; `trusted_skip_gateway` ; invoke Manifesto |

## Périmètre IN / OUT

**IN (ce lot)** : crate `apparatus-operator` ; Git → digest 0002 → enveloppe OCI (≠ image CRI) → Cosign+Transit → zot (signer-only) → Adm-A `VALID` (CR) → Job/Pod isolé Kind (image plateforme) ; refus 0005 ; worker malveillant ; 4 SA ; NetworkPolicy réelle ; gate P5 **hors Manifesto** + régression Lazaret ; T1 reste vert ; 0008 Réalité Partial (faits).

**OUT** : T1 à relivrer ; `Factory/` ; nest monolithe ; skill new-service ; 6ᵉ hexagone HTTP ; K8s-as-P3 / `KubernetesAdapter` Manifesto ; lever G/E ; APP-05 ; host UI / iframe / CLI P5 ; catalogue tiers P6 / APP-02 ; WASM premier runtime ; `shared`/`organization` ; drain/destroy bindings `ready` ; scale-to-zero / OSB / Knative ; Adm-B webhook ; BC-D ; budget/CPU numériques APP-06 ; A-C1 compose service-local ; flip 0005 Implemented ; SuperSède 0003/0005 ; 2e protocole ; bearer IAM ; nouveau type FGA ; `trusted_skip_gateway` ; wiki/QMD « ménage » comme livrable ; commit git.

## Invariants à ne pas casser

- Identité = `project_components.id` ; binding 1:1 ; `source` ∈ `legacy|managed` ; pas de 2e UUID public.
- `/components` = 5 ; 0 `gateway` / 0 `invoke` sous `Manifesto/*/src` ; 0 `fn invoke` sur `ApparatusRuntime`.
- Tokens `k8s`/`kubernetes`/`wasm`/`wasi`/`wasmtime`/`iframe`/`messagechannel`/`apparatus_host`/`ui_host` **interdits dans Manifesto prod** (gates + T1). **Autorisés** dans `apparatus-operator/`.
- Harness ≠ production ; pas de `VALID`/`VERIFIED` émis par `harness.rs`.
- Register Manifesto ≠ admit. Kubelet n’exécute que `image@sha256` **issu de l’enveloppe admise**, jamais digest d’enveloppe comme `spec.image`, jamais `latest`.
- Job build : pas de token API, pas de clés Transit, pas de push registry.
- Contrôleur P4 ≠ `Manifesto/infra/src/apparatus_runtime/tick.rs` (pas de copie du lease 30s comme moteur d’isolation).
- Isolation : **pas** de mock NetworkPolicy comme preuve (0003).
- `unsafe_code = forbid` ; Clippy workspace ; pas d’`unwrap` prod non documenté.

## Tranches T2–T12 (TDD, séquentielles)

Nomme les tests `apparatus_p4_t*.rs`. **Nouveau code de tests T2–T12** : `apparatus-operator/tests/`. Ne **pas** relivrer `Manifesto/tests/apparatus_p4_t1_absence.rs` (sauf si un GREEN T2+ le casse — alors ajuster **allowlists factuelles**, pas relâcher les tokens Manifesto).

RED avant GREEN à chaque tranche. Pas de Kind avant T10.

### Tranche 2 — Scaffold BC-A

- **RED** : member `apparatus-operator` absent ; bins absents ; `Factory/` toujours absent ; **pas** dans `monolith/Cargo.toml`.
- **GREEN** : `Cargo.toml` workspace + crate ; `src/lib.rs` ; bins `controller` / `admit` / `build` **sans** serveur HTTP ; `--version` ou équivalent.
- **Fichiers** : `apparatus-operator/Cargo.toml`, `src/lib.rs`, `src/bin/controller.rs`, `src/bin/admit.rs`, `src/bin/build.rs`.
- **Tests** : `apparatus-operator/tests/apparatus_p4_t2_scaffold.rs` (member présent, pas `Factory/`, pas de listener HTTP, pas de dep kube **dans Manifesto**).
- **Kind** : non.
- **Sortie** : crate compile (`cargo test -p apparatus-operator --test apparatus_p4_t2_scaffold`).

### Tranche 3 — Git → digest canonique

- **RED** : `latest` / branche / tag flottant ≠ identité. Utiliser `apparatus_contracts::{ReleaseDigest, digest_manifest, …}` — **pas** un second schéma.
- **GREEN** : ref Git mobile → `APPARATUS_FLOATING_REF` / `APPARATUS_INVALID_DIGEST` (codes P0 existants). Le futur contrôleur ne consomme pas de tags.
- **Fichiers** : `apparatus-operator/src/digest.rs` (ou module lib) — wrapping, pas fork.
- **Tests** : `apparatus_p4_t3_digest.rs`.
- **Kind** : non.

### Tranche 4 — Worker de build (OS distinct)

- **RED** : entrypoint `apparatus-build` **sans** env Transit, **sans** creds push, **sans** kube token. Code hostile (`build.rs` / proc-macro) : sandbox Job / refus, pas « builder connu = safe ». Pas de `build = "…"` libre dans le manifeste (0003).
- **GREEN** : preuve **unitaire + contrat** que le binaire build n’embarque pas le client Transit/registry write. IT cluster au T10 (Job annoté `automountServiceAccountToken: false`).
- **Tests** : `apparatus_p4_t4_build_worker.rs`.
- **Kind** : non (contrat) ; Job réel = T10.

### Tranche 5 — Conformance (indépendante du publisher)

- **RED** : suite plateforme sur **artifacts candidats** ; reconstruire après test **invalide** la preuve. `VALID` ≠ installable partout (0005). Harness P0 **n’est pas** la conformance.
- **GREEN** : rapport de suite versionné (id de politique) consommé plus tard par Adm-A.
- **Tests** : `apparatus_p4_t5_conformance.rs`.
- **Kind** : non.

### Tranche 6 — Signataire + registry (identité ≠ build)

- **RED** : fixture zot + OpenBao Transit (testcontainers, skill locale). `apparatus-admit` pousse l’enveloppe **et** signe (Cosign+Transit). Build **ne peut pas** pusher (auth zot). Le signer n’exécute pas le code auteur.
- **GREEN** : enveloppe ORAS **non** CRI + blob image pinée ; identité catalogue = digest **0002**, pas digest OCI d’enveloppe.
- **Fixtures** : `apparatus-operator/tests/fixtures/zot/` ; `tests/fixtures/openbao-transit/`.
- **Tests** : `apparatus_p4_t6_sign_registry.rs` (**IT Docker**).
- **Kind** : non.

### Tranche 7 — Admission indépendante (register ≠ admit)

- **RED** : enregistrer une release côté Manifesto **n’écrit pas** `AdmissionRecord`. Seul `apparatus-admit` écrit `VALID`. `VERIFIED` ≠ admission. Harness / publisher / plugin ne s’auto-admettent pas.
- **GREEN** : persistance CR **en mémoire / fake kube** acceptable en T7 ; apply cluster = T10. Pas d’HTTP admit.
- **Tests** : `apparatus_p4_t7_admission.rs`.
- **Kind** : non (fake client OK) ; CRD live = T10.

### Tranche 8 — Refus 0005

- **RED** : (1) digest altéré (2) manifeste non conforme (3) signature inattendue (4) politique runtime manquante → **refus**, pas de CR `VALID`, pas de schedule.
- **GREEN** : un seul protocole ; pas de chemin `trusted`.
- **Tests** : `apparatus_p4_t8_refus.rs`.
- **Kind** : non.

### Tranche 9 — Worker malveillant

- **RED** : processus build **ou** image plugin **sans** clés / **sans** accès plateforme : pas d’auto-admit, pas de signature, pas d’écriture registry, pas de secrets IAM.
- **GREEN** : officiel et communautaire = même confinement.
- **Tests** : `apparatus_p4_t9_malicious_worker.rs` (contrat + IT zot/Transit T6).
- **Kind** : non (renforcé T10 si le Job hostile tourne dans plugins ns).

### Tranche 10 — Git → digest → workload (image plateforme) + Kind

- **RED** : Kind cluster `apparatus-p4-it` ; 4 SA ; 2 ns ; zot joignable depuis le cluster (registry IP Kind) ; image de test **plateforme in-repo** (`apparatus-operator/testdata/platform-plugin/`) **avant** P6 ; soumission → enveloppe → `VALID` → Job/Pod dans `apparatus-plugins` ; profil d’isolation manquant → **refus** (pas « un peu moins isolé ») ; pas de `shared`/`organization`.
- **GREEN** : `kubectl`/client : Pod Running ; `spec.image` = `image@sha256` CRI ; **pas** tag flottant.
- **Fichiers** : `apparatus-operator/k8s/*.yaml` (SA, ns, RBAC, CRD, NetworkPolicy) ; fixture `tests/fixtures/kind/`.
- **Tests** : `apparatus_p4_t10_platform_workload.rs` (**IT Kind+Docker**). Fail-loud si Docker/Kind absent (comme les fixtures testcontainers existantes) — **pas** de skip silencieux qui verdit le lot.

### Tranche 11 — Runtime K8s hors Manifesto

- **RED** : contrôleur crée/tue le workload **seulement** si `AdmissionRecord` existe pour le digest 0002. Isolation **par projet et par binding** (labels). Preuve NetworkPolicy **réelle** : depuis le Pod plugin, échec d’accès Transit / API secrets `apparatus-system` (probe ou exec, pas assert YAML). Tuer le Pod plugin **ne** casse **pas** Manifesto. Zéro token `k8s` ajouté sous `Manifesto/*/src`. WASM pas premier runtime.
- **GREEN** : pas de type `KubernetesAdapter` dans Manifesto (T1 reste rouge si on l’ajoute).
- **Tests** : `apparatus_p4_t11_runtime.rs` (**IT Kind**).
- **Contrôleur** ≠ ticker P2.

### Tranche 12 — Gate P5 + régression Lazaret `invoke`

- **RED** : tokens host/iframe/CLI/`messagechannel`/`ui_host`/`apparatus_host` interdits **dans Manifesto** (gates existants verts). Allowlist K8s **uniquement** sous `apparatus-operator`. Régression : `Lazaret/tests/apparatus_p3_t7_invoke.rs` (et t8–t13 pertinents) ; 0 `gateway` Manifesto src ; `/components` = 5 ; pas d’`invoke` sur `ApparatusRuntime` ; T1 P4 vert.
- **GREEN** : ne pas ouvrir P5. Ne pas flipper 0005 `Implemented`.
- **Tests** : `apparatus_p4_t12_gate_regression.rs` **plus** cargo test ciblé P3 Lazaret + gates Manifesto (commandes ci-dessous). Option IT : client `manifesto-apparatus/1` depuis SA gateway → Service plugin (même proto, pas un 2e).

## Stratégie de tests

- **Unitaires / contract** : digest ; register ≠ admit ; refus 0005 ; DTO sans `trusted_*` ; harness sans `VALID`/`VERIFIED` ; bins sans HTTP.
- **IT Docker (T6–T9)** : zot + OpenBao Transit testcontainers **service-local**.
- **IT Kind (T10–T12)** : cluster réel ; NetworkPolicy réelle ; 4 SA.
- **Régression P3** : Lazaret invoke/KV/consents ; gates Manifesto ; T1 P4.
- **Adverse** : worker malveillant ; digest altéré ; image plateforme avant P6.
- **Pas** de harness parallèle « de production ».
- **E2E host / iframe / CLI / catalogue tiers** : hors P4.

## Hors périmètre strict (rappel)

P5 host UI / iframe / CSP / MessageChannel / CLI. P6 catalogue. A-C1 compose leftover. APP-05. Lever G/E. K8s-as-P3. WASM V1 premier runtime. Drain/destroy `ready`. Scale-to-zero / OSB / Knative. Nouveau FGA. Invoke Manifesto. Flip 0005 Implemented. Relivrer T1 comme mécanisme. Réconcilier APP-01. Corriger QMD/wiki comme livrable principal.

Préempter P5–P6 = interdit. Besoin P5+ → note de suivi, pas de code.

## Contraintes de qualité

- Lints workspace (`unsafe_code = forbid`, Clippy pedantic/nursery/cargo).
- Éviter `unwrap`/`expect` prod ; `# Errors` / `# Panics` si requis.
- Dépendances minimales, versions workspace. **Pas** de migration SQL Manifesto (T1 l’interdit). CRD = schéma K8s, pas SeaORM.
- Préserver les modifications utilisateur ; aucune commande Git destructive.
- Sécurité d’abord : clés, registry, workers, admission.

## Vérification (minimum)

1. `cargo fmt --all -- --check`
2. `cargo check -p apparatus-operator` (et crates touchés)
3. Tests ciblés, **pas** « tout cargo » :
   - `cargo test -p apparatus-operator --test apparatus_p4_t2_scaffold` … `t12_*` (distinguer unit/IT)
   - `cargo test -p manifesto-service --test apparatus_p4_t1_absence --test apparatus_p2_t7_gate --test apparatus_p3_t2_gate`
   - `cargo test -p lazaret-service --test apparatus_p3_t7_invoke`
4. `cargo clippy -p apparatus-operator --all-targets -- -D warnings` si le temps reste raisonnable
5. Diagnostics IDE sur fichiers modifiés

Corriger les régressions **introduites**. Ne pas élargir aux défauts préexistants sans rapport (compose service-local inclus). Le gate Manifesto reste **plus strict** que « K8s autorisé partout ».

## Documentation de sortie (faits seulement)

Après preuves T2–T12 **vertes** :

- ADR-0008 champ **Réalité** → `Partial` + phrase de preuve (tests nommés, Kind IT). **Ne pas** `Implemented` (TCB prod / Transit release / Adm-B encore ouverts). **Ne pas** flipper 0002/0003/0005.
- Digest Serena `architecture/apparatus-p4-adr-0008` : **même tour** que la maj Réalité 0008 (règle adr-serena-digest) — orchestrateur / auteur de l’ADR, pas un dump wiki.
- Pointeur wiki **faits de tests** seulement. **Pas** copier L152 comme spec. **Pas** d’index QMD comme lot.
- Canvas lecture seule.

**Pas de nouvelle ADR. Pas de SuperSède.**

## Compte rendu final (implementer)

Fournir :

1. Lot **P4-core T2–T12** : scénario presses-nord 03:00 **prouvé** (digest VALID, Pod tuable, isolation réelle, pas `latest`, invoke Lazaret vert).
2. Choix locaux **tels quels** (crate `apparatus-operator`, zot, Kind@T10, colocation admit/signer, CR `AdmissionRecord`).
3. Fichiers / tests T2–T12 ; T1 **non relivré**.
4. Commandes + résultats (unit vs IT Docker vs IT Kind).
5. Gates Manifesto : `k8s` **toujours interdit dans Manifesto** ; tokens réels dans `apparatus-operator`.
6. Régression Lazaret `invoke` ; 0002/0003/0005 **toujours Partial** ; 0008 Réalité **Partial** si les preuves y sont.
7. Items Hors périmètre **non touchés**.
8. Différés : P5/P6, drain, APP-05, APP-06 numériques, Adm-B, BC-D, Transit release ≠ `-dev`, mesh Lazaret-in-cluster.
9. Risques bloquants réels (Docker/Kind absent = lot **non** accepté).

## Escalade humaine (ne pas inventer)

Second protocole · `trusted_skip_gateway` · bearer IAM · K8s sous `Manifesto/*/src` · nouveau type FGA · SuperSède 0003/0005 · flip 0005 Implemented · clôturer APP-05 · lever G/E · budget/CPU numériques APP-06 · 5ᵉ SA / split admit-signer **contre** Run-A · WASM premier runtime · déployer P3 « en prod » sans isolation.
