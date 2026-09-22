# Prompt — Implémentation Apparatus P4

> **POST-LOT (2026-09-22).** Ce fichier est **historique** du tour d’écriture ADR-0008 (2026-09-20 : Phase 0, T1, gel A-DEC). [`docs/apparatus-p4-core-implementation-prompt.md`](apparatus-p4-core-implementation-prompt.md) = contrat **historique** du lot usine T2–T12 **livrés** — **plus** une mission ouverte. T2–T12 **livrés** ; 0008 **Partial**, pas Implemented. **Ne pas** relivrer P4-core ni T1 depuis ici.

Tu travailles dans le dépôt `C:\Users\djden\source\repos\AIForAll`. Ne commite rien. **Pas** de mission implementer ouverte : archive du tour d’écriture 2026-09-20 uniquement.

## Contrainte modèle

Politique utilisateur **courante** (2026-09-20). **Ne pas** recopier la contrainte P3-era « seul Grok pour tout ».

| Rôle | Slug Cursor (`model:` de chaque Task / sous-agent) |
|---|---|
| Orchestration, architecture, implémentation difficile, revues avec jugement, ADR-0008 **Accepted** | `cursor-grok-4.6-xhigh` |
| Explore, mécanique, test-reviewer | `composer-2.5-fast` |

Slugs **autorisés seulement** : `inherit`, `composer-2.5-fast`, `cursor-grok-4.6-xhigh`, `muse-spark-1.3-max`.

- Ne jamais choisir **silencieusement** un autre modèle.
- Un agent lancé hors table / hors slugs autorisés = non conforme ; arrêter et relancer.
- `inherit` n’autorise pas à dévier vers un slug absent de la liste.

## Mission (historique 2026-09-20 ; ne pas relivrer)

**Ne pas** relivrer P4-core. T2–T12 **livrés**. 0008 **Partial**, pas Implemented. Le texte TDD ci-dessous était le contrat d’écriture 2026-09-20.

Le gel « Implémente **P4 — Factory et runtime** » n’est **plus** une mission ouverte. **APP-01 est tranché** ; ADR **0008** **Accepted**. **Au tour d’écriture 2026-09-20** : Réalité **Unimplemented**. **Aujourd’hui (post-lot)** : Réalité **Partial** — T2–T12 livrés ; **pas** Implemented. Ne dépasse pas P4. Ne livre **pas** P5/P6.

**Canons primaires (Accepted / Réalité Partial — preuves P4–P6, ne pas SuperSéder)** :

- **ADR-0005** : un seul protocole ; admission, `VALID`, `VERIFIED` et installabilité **distincts** ; register Manifesto **≠** admit ; harness / `apparatus dev` **jamais** `VALID`/`VERIFIED` ; pas de `trusted_skip_gateway`.
- **ADR-0003** : plugin hors processus privilégiés ; workers Factory, admission/signature, contrôleur et gateway = identités OS distinctes ; harness in-process ≠ production ; moteur = **Kubernetes P4 hors Manifesto** (ADR-0008).
- **ADR-0002** : Factory = **P4** ; host UI + CLI = **P5** (L19, L35). Les contrats P0 se réutilisent ; pas de second schéma.

**Ancre (2026-09-20)** : ADR-0004 et ADR-0007 **Accepted / Implemented** (A-DEC). T1–T14b = preuve P3. `invoke` = Lazaret `POST /invoke`. 0 identifiant `gateway` sous `Manifesto/*/src`. `/components` = 5. Gate `Manifesto/tests/apparatus_p2_t7_gate.rs` (+ P3 t2) : `k8s`/`kubernetes`, `wasm`/`wasi`/`wasmtime`, `iframe`, `messagechannel`, `apparatus_host`, `ui_host` **restent interdits dans Manifesto** même si 0008 choisit K8s **ailleurs**.

Conduis : Phase 0 **faite** (APP-01 tranché ; 0008 **Accepted**). T1 absence **déjà livrée** (ne pas relivrer). T2–T12 **débloqués par cet Accept** — au tour d’écriture 2026-09-20 **ne pas** les implémenter ; **post-lot**, le mécanisme T2–T12 est livré (0008 **Partial**). N’invente **aucune** décision `Accepted` au-delà de 0008. N’invente **pas** un moteur, un registry ou une signature **autres** que ceux que 0008 nomme.

## Gel A-DEC (2026-09-20) — faits courants, ne pas rouvrir

Accord humain exact (closeout `A-DEC`) : « Implemented autorisé sans fermer APP-05, sans lever G/E, sans K8s, sans second protocole. Ces items restent hors-jalon. »

| Fait | État |
|---|---|
| ADR-0004 | Accepted / **Implemented** |
| ADR-0007 | Accepted / **Implemented** |
| Preuve P3 | T1–T14b livrés ; **ne pas relivrer** |
| `APP-05` | **Non décidé** dans 0004 (options **A/B non choisies**) — hors-jalon |
| 0006 G / E | **Encore en vigueur** — ne pas lever |
| K8s comme isolation **P3** | **Interdit** (A-DEC + 0007 L211 + closeout `D-K8S`) |
| Second protocole / `trusted_skip_gateway` | **Interdit** (0005, 0007 L211, `D-PROTO2`) |
| Invoke | Lazaret `POST /invoke` ; **pas** de méthode `invoke` sur Manifesto `ApparatusRuntime` |
| `gateway` sous `Manifesto/*/src` | **0** |
| Routes `/components` | **5** (gel 0006 E) |
| Gate Manifesto | `apparatus_p2_t7_gate.rs` **et** `apparatus_p3_t2_gate.rs` : ne **pas** retargeter le gate Manifesto pour **autoriser `k8s`/`kubernetes` dans Manifesto**, même si 0008 nomme K8s dans un autre BC |

Closeout §D reste ouvert (ne pas « clôturer » pendant P4-core) : `D-APP05`, `D-0006G`, `D-0006E`, `D-PROTO2`, `D-APP03`, `D-APP06`, drain/destruction bindings déjà `ready` (0005 L34, README L138 → ADR **future**). `D-K8S` (K8s-**as-P3**) reste interdit. `D-APP01` : **tranché** par ADR-0008 (K8s **P4 hors Manifesto**).

## Accepted vs Réalité

`Accepted` = cible ratifiée. `Réalité` = code présent. Ne les confonds pas. Ne réécris pas les décisions `Accepted` de `0001`–`0008`. **Interdit** : SuperSède de **0003** ou **0005** par 0008.

Le wiki (plan, factory, platform) est **conception** (`^[inferred]` / `status: proposed`), **pas** une ADR, **pas** un contrat RED. Interdit de traiter le wiki comme Accepted, et d’inventer un moteur, un registry, un schéma de signature, un `KubernetesAdapter`, ou un répertoire `Factory/` « parce que le wiki les nomme ».

`docs/adr/README.md` L18 : **`APP-01` est ADRé (0008)**. `APP-02`…`APP-07` restent des arbitrages ouverts. 0008 = **Accepted** / Réalité **Partial** post-lot (T2–T12 ; T1 absence ≠ mécanisme). Au tour d’écriture 2026-09-20 la Réalité était **Unimplemented**.

En fin de **slice-1 P4-core**, laisser **0002 / 0003 / 0005** en Réalité **Partial**. **Ne pas** flipper 0005 à `Implemented` dans slice-1 (preuves P5/P6 encore dues).

### Contradictions ADR / code / wiki (ne pas lisser)

1. **Plan L152 `KubernetesAdapter` vs 0003 L45–L46 + A-DEC + `D-K8S`.** Le plan wiki `^[inferred]` dit d’« implémenter KubernetesAdapter ». 0008 **nomme** Kubernetes **P4 hors Manifesto**. A-DEC interdit K8s-**as-P3**. **Ne pas** poser `KubernetesAdapter` **dans Manifesto**. Wiki L152 = conception, pas un contrat de crate Manifesto.
2. **Closeout `D-P4` vs 0002 L35 / plan P5.** `D-P4` liste « Factory, **host UI**, OCI, `VALID`/`VERIFIED`, iframe » comme un seul sac P4. 0002 L19/L35 et le plan : Factory = **P4**, host/CLI/iframe = **P5**. **P5 est OUT** de ce jalon. Ne pas « avancer le host » parce que `D-P4` le cite.
3. **Plan / wiki encore « 0007 Partial ».** Ex. plan L148 « ADR-0007 **reste Partial** », matrice L170 « preuves P3 Partial ». **Périmé** vs A-DEC : 0007 **Implemented**. Pointer, ne pas promouvoir la matrice en contrat, ne pas « corriger » le wiki en inventant du P4.

Si une contradiction **bloque** le contrat (second protocole, `trusted_skip_gateway`, bearer IAM, K8s-as-P3, lever G/E, nouveau type FGA, flip 0005 Implemented en slice-1) : **escalade humaine**, n’invente pas.

## Sources de vérité à lire avant de coder

1. `AGENTS.md` et les règles du workspace.
2. `docs/adr/README.md` : 0008 **existe** (Accepted / **Partial** post-lot ; Unimplemented au tour d’écriture 2026-09-20). L18 (`APP-01` ADRé ; `APP-02`…`APP-07` ouverts). L135–L137 (pipeline Git→OCI et moteur = **ADR-0008 Accepted 2026-09-20** ; host UI **avant P5** ; CPU/budget numériques encore `APP-06`).
3. ADR `0001`–`0008` dans `docs/adr/` (ne pas réécrire `Accepted`). Canons P4 : **0008**, **0005**, **0003**, **0002** (L19, L35). P3 livré : **0004** + **0007** Implemented. 0007 L11 (preuves T1–T14b + holes hors-jalon), L211 (escalade : 2e proto / skip gateway / K8s-as-P3 / FGA / levée G/E), L228 (holes A-DEC). Compagnon **méthode** : `docs/adr/0008-app01-reconciliation.md` (pas canon).
4. `docs/adr/0007-closeout.md` §D (laisser ouvert). Le leftover compose service-local n’est **pas** une source P4 — voir Hors périmètre.
5. Prompts `docs/apparatus-p0-implementation-prompt.md` … **`p3`** : **esprit TDD + gates** seulement. **Beaucoup de faits P3 dans le prompt P3 sont périmés** (ex. 0004 Partial) ; l’état réel = A-DEC ci-dessus. Phase 0 P4 = **faite** (0008 Accepted).
6. Wiki — **conception seulement** :
   - `obsidian/AI FOR ALL/projects/manifesto/references/apparatus-implementation-plan.md` **P4** L150–L196 `^[inferred]` (pipeline, preuve de sortie, `APP-01`…`APP-07`)
   - `obsidian/AI FOR ALL/projects/manifesto/references/apparatus-factory-and-distribution.md` (`status: proposed`) — **pas** un RED
   - `obsidian/AI FOR ALL/projects/manifesto/concepts/apparatus-platform.md`
7. Code réel P3 à **préserver** (régression, pas à réécrire) :
   - `Manifesto/tests/apparatus_p2_t7_gate.rs`, `Manifesto/tests/apparatus_p3_t2_gate.rs`
   - `Manifesto/tests/apparatus_p3_t1_absence.rs` ; Lazaret `Lazaret/tests/apparatus_p3_t7_invoke.rs` (+ t8…t13)
   - Contrats P0 : `apparatus-contracts` (digest canonique, protocole `manifesto-apparatus/1`, **pas** `trusted_skip_gateway`)
   - Harness `test-harness` : double de test, **jamais** `VALID`/`VERIFIED`

Skills : `.cursor/skills/rustycog/SKILL.md` (et `.agents/skills/rustycog/SKILL.md`) si API RustyCog. Fixtures DB : `.cursor/skills/creating-testcontainer-fixtures/SKILL.md`. **Nouveau service** : skill `aiforall-new-service` **non** (0008 = BC-A operator+Jobs, pas de 6ᵉ hexagone HTTP).

## Périmètre P4 obligatoire (TDD, une tranche après l’autre) — slice-1 P4-core **après Accept 0008**

Invariants **toujours vrais** (sauf ADR Accepted qui les lève — 0008 **ne** lève **pas** G/E, **ne** SuperSède **pas** 0003/0005) : pas de second UUID public ; identité = `project_components.id` ; 1:1 ; `source` ∈ `legacy|managed` ; pas de nouveau type FGA ; `/components` = 5 ; 0 `gateway` sous `Manifesto/*/src` ; `invoke` serveur = Lazaret ; pas de `trusted_skip_gateway` ; un seul protocole ; harness ≠ production ; pas de `VALID`/`VERIFIED` émis par le harness ; register Manifesto ≠ admission ; workers **OS-distincts** (0003) : build / conformance / signer / contrôleur ; sécurité d’abord.

**IN slice-1 (après Accept 0008)** : Git → digest **canonique** (0002) → package → conformance → signature → registry → **admission indépendante**. Refus 0005 (digest altéré, manifeste non conforme, signature inattendue, politique runtime manquante). Image de test **plateforme** avant soumissions tierces. Worker **malveillant** sans clés ni accès plateforme. Adaptateur de runtime **seulement si 0008 le nomme**. En fin de slice-1 : 0002/0003/0005 **restent Partial**.

Nomme les tests `apparatus_p4_t*.rs`. **Ne pas** inventer un répertoire `Factory/` (0008 : pas ce nom). Le crate / chemin T2+ = **0008** (hors Manifesto).

### Décidés par ADR-0008 (ne pas reratifier ; budget/CPU numériques encore ouverts)

0008 **Accepted** fige : moteur **K8s P4 hors Manifesto** ; Cosign + OpenBao Transit ; registre OCI portable + push signer-only ; Adm-A worker-only ; BC-A operator+Jobs (**pas** `aiforall-new-service`) ; Pkg-B enveloppe ≠ image CRI ; Run-A 4 SA. WASM **pas** premier runtime V1 (0003). **Encore ouverts** : budget/CPU **numériques** (`APP-06`), split admit vs signer, `APP-02`/`APP-05`/`APP-03`.

Ne pas poser `KubernetesAdapter` dans Manifesto. Ne pas créer `Factory/`.

### Phase 0 — APP-01 tranché ; ADR 0008 **Accepted** (T2+ débloqué, **pas** à implémenter ce tour)

**État :**

1. **APP-01 tranché.** Moteur / registry / signing / K8s-oui = **0008**. README L18 : APP-01 **ADRé**.
2. Canon : `docs/adr/0008-apparatus-p4-k8s-isolation-outside-manifesto.md` — **Accepted** / **Partial** post-lot (**Unimplemented** au tour d’écriture 2026-09-20). **Pas** de SuperSède 0003/0005.
3. Digest Serena `architecture/apparatus-p4-adr-0008` : **orchestrateur**, même tour (pas l’implementer docs).
4. T2–T12 **débloqués par cet Accept**. Au tour d’écriture 2026-09-20 : **ne pas** implémenter T2+ ; **post-lot**, operator / Cosign / Jobs / Kind sont livrés (0008 **Partial**, pas Implemented).

T1 absence **déjà livrée** : ne pas relivrer.

Checklist Phase 0 (**tranchée** sauf mentions) :

1. **APP-01** : moteur K8s P4 hors Manifesto ; registry Cosign+Transit ; Adm-A — **fait**. Budget/CPU **numériques** encore `APP-06`.
2. 0008 **nomme** Kubernetes **hors Manifesto**. Pas de `KubernetesAdapter` dans Manifesto.
3. BC-A : operator+Jobs ; skill `aiforall-new-service` **non**. Pas de nom `Factory`.
4. 4 SA K8s ; Job build sans token API ; plugins autre ns. Contrôleur P4 ≠ ticker P2.
5. Identité d’installation = digest **0002**.
6. Enveloppe OCI/ORAS **non** CRI + blob image CRI. Registre portable (artifacts **et** pull).
7. Conformance : suite plateforme + version de politique (0005 `VALID`) — détail de suite encore à implémenter, pas reratifier le moteur.
8. Admission = Adm-A worker, seule source `VALID`.
9. Clés : OpenBao Transit ; worker de build **n’en a pas** ; seul le SA signer pousse.
10. Adaptateur runtime = K8s **hors** Manifesto (T11). T11 absence **dans Manifesto** reste vraie.
11. `invoke` reste Lazaret ; 0008 ne déplace pas l’I/O vers Manifesto.
12. **Zéro** SuperSède 0003/0005 ; **zéro** levée G/E ; **zéro** type FGA nouveau.
13. Slice-1 laisse 0002/0003/0005 **Partial** — 0008 le dit.
14. Drain / destroy des bindings déjà `ready` : **hors** 0008 (ADR future, 0005 L34).

### Tranche 1 — Preuves d’absence Factory / runtime / admission (**déjà livrée** — ne pas relivrer)

Autorisé **uniquement** par ADR **déjà Accepted** : 0005 (pas d’auto-admission, pas de `VALID`/`VERIFIED` harness, pas de `trusted_skip_gateway`) + 0003 (harness ≠ production, pas de plugin in-process) + 0002 (Factory absente) + gates P2/P3.

- RED : `Manifesto/tests/apparatus_p4_t1_absence.rs` : pas de pipeline Git→package→admission en prod ; harness sans `VALID`/`VERIFIED` ; DTO sans `trusted_skip_gateway` ; 0 `gateway` Manifesto src ; `/components` = 5 ; `ApparatusRuntime` sans `invoke` ; tokens `k8s`/`kubernetes`/`wasm`/`iframe`/`messagechannel`/`apparatus_host`/`ui_host` toujours interdits **dans Manifesto** ; **ne pas** créer `Factory/` ; **ne pas** retargeter les gates. **Ne pas** ajouter de SQL.
- GREEN : caractérisation seulement.
- Sortie : baseline verte. Interdit : workers, registry, signature, adaptateur, scaffold BC.

### Tranche 2 — Scaffold BC (APRÈS Accept 0008 — **débloqué**, **pas** ce tour d’écriture ADR)

- RED : bounded context / crate **nommé par 0008** seulement. Skill `aiforall-new-service` **non** (BC-A). Pas de répertoire `Factory/`.
- GREEN : health/ready ou équivalent **si** 0008 le demande ; pas d’admission, pas de clés plateforme dans un worker de build.
- Sortie : `apparatus_p4_t2_*.rs` (chemin = 0008).

### Tranche 3 — Git → digest canonique

- RED : ref Git mobile ≠ identité ; identité d’installation = digest du descripteur **canonique** (0002). `latest` / branche / tag flottant = erreur de validation.
- GREEN : pas de consommation de tags par un futur contrôleur (0005).
- Sortie : `apparatus_p4_t3_*.rs`.

### Tranche 4 — Worker de build (processus OS distinct)

- RED : build dans un worker **sans** clés de signature, **sans** droits registry, **sans** secrets plateforme (0003). Code auteur hostile (`build.rs`, proc-macros) : sandbox / refus, pas « builder connu = safe ».
- GREEN : pas de commande de build libre dans le manifeste (0003 alternative rejetée).
- Sortie : `apparatus_p4_t4_*.rs`.

### Tranche 5 — Conformance (indépendante du publisher)

- RED : suite plateforme sur **artifacts candidats** ; une reconstruire après test **invalide** la preuve (wiki factory = conception du *risque*, 0008 fige le mécanisme). `VALID` ≠ installable partout (0005).
- GREEN : harness P0 **n’est pas** la conformance.
- Sortie : `apparatus_p4_t5_*.rs`.

### Tranche 6 — Signataire + registry (processus OS distinct du build)

- RED : signature et publication par une identité **distincte** du worker de build. Registry / format = **0008**. Le signer n’exécute pas le code auteur.
- GREEN : pas de clés sur l’image de test auteur.
- Sortie : `apparatus_p4_t6_*.rs`.

### Tranche 7 — Admission indépendante (register ≠ admit)

- RED : enregistrer une release dans Manifesto **n’auto-admet pas** (0005). Seule l’autorité d’admission (0008 / APP-01) atteste un digest. `VERIFIED` ≠ admission.
- GREEN : pas de `VALID` auto-déclaré par Manifesto, le publisher ou le harness.
- Sortie : `apparatus_p4_t7_*.rs`.

### Tranche 8 — Refus 0005

- RED : digest altéré ; manifeste non conforme ; signature inattendue ; politique runtime manquante → **refus** (plan L154 = **tests**, pas spec d’API).
- GREEN : un seul protocole ; pas de chemin `trusted`.
- Sortie : `apparatus_p4_t8_*.rs`.

### Tranche 9 — Worker malveillant sans clés plateforme

- RED : un worker (build ou plugin) **sans** clés / **sans** accès plateforme ne s’auto-admet pas, ne signe pas, n’écrit pas le registry, n’obtient pas les secrets IAM.
- GREEN : officiel et communautaire = même confinement (0003/0005).
- Sortie : `apparatus_p4_t9_*.rs`.

### Tranche 10 — Git → digest → workload (image de test plateforme)

- RED : soumission Git → release par digest → workload sur **image de test plateforme** (pas le catalogue tiers P6). Isolation **par projet et par binding** (0003). Profil d’isolation manquant → refus, pas « un peu moins isolé ».
- GREEN : pas de `shared` / `organization` V1.
- Sortie : `apparatus_p4_t10_*.rs`.

### Tranche 11 — Adaptateur de runtime **iff 0008**

- RED : si 0008 **nomme** un adaptateur (ex. Kubernetes **seulement si nommé**) : l’implémenter sur l’infra que 0008 provisionne. Si 0008 **ne le nomme pas** : preuve d’**absence** (pas de `KubernetesAdapter` inventé depuis le plan L152).
- GREEN : WASM **pas** premier runtime V1 (0003). K8s-as-P3 toujours interdit. Ne pas retargeter le gate **Manifesto** pour y autoriser `k8s`.
- Sortie : `apparatus_p4_t11_*.rs`.

### Tranche 12 — Gate P5 + régression Lazaret `invoke`

- RED : gate **P5** (tokens host/iframe/CLI/`messagechannel`/`ui_host`/`apparatus_host` interdits **hors** allowlist **si** 0008 en crée une **hors Manifesto**). Régression : Lazaret `POST /invoke` (P3 T7) toujours vrai ; 0 `gateway` Manifesto ; `/components` = 5 ; pas d’`invoke` sur `ApparatusRuntime`.
- GREEN : ne pas ouvrir P5. Ne pas flipper 0005 `Implemented`.
- Sortie : `apparatus_p4_t12_*.rs` + gates P2/P3 Manifesto **verts** sans élargir Manifesto à `k8s`.

## Stratégie de tests (couches pertinentes P4 seulement)

- **Unitaires / contract** : digest canonique ; register ≠ admit ; refus 0005 ; DTO sans `trusted_skip_*` ; harness sans `VALID`/`VERIFIED`.
- **Intégration** : testcontainers (skill) pour registry / DB **si** 0008 les nomme ; **pas** de mock NetworkPolicy comme preuve d’isolation (0003, plan).
- **Adverse** : worker malveillant ; digest altéré ; image plateforme **avant** P6.
- **Régression P3** : Lazaret invoke, consents, KV, gates Manifesto.
- **Pas** de harness parallèle « de production ».
- **E2E host / iframe / CLI / catalogue tiers** : hors P4 (P5/P6).
- **K8s cluster réel** : seulement si 0008 nomme K8s.

## Hors périmètre strict

Ne pas ajouter / ne pas « glisser » :

- **P5** : host UI, iframe, CSP, MessageChannel, serveur de bundles, CLI `check/dev/publish` / proc-macro (0002 L35, README L136).
- **P6** : catalogue tiers, `APP-02`, qualification community.
- **A-C1** : leftover compose **service-local** IAMRusty / Telegraph (closeout A-C1, T14b C-1). **Pas ce jalon. Pas un T15.** Ne pas « réparer le compose » pendant P4.
- **APP-05** (A/B 0004 **non choisies**).
- **Lever 0006 G/E.** Second protocole. `trusted_skip_gateway`.
- **K8s-as-P3.** WASM comme **premier** runtime V1. `shared` / `organization`.
- Drain / destroy des bindings déjà `ready` (ADR **future**). Scale-to-zero / OSB / Knative (0003 L48).
- Nouveau type FGA. `invoke` sur Manifesto `ApparatusRuntime`.
- Flip 0005 (ni 0002/0003) à **Implemented** en slice-1.

Préempter P5–P6 = interdit. Si un besoin P5+ apparaît, note-le en suivi.

## Contraintes de qualité

- Respecter les lints workspace (`unsafe_code = forbid`, Clippy pedantic/nursery/cargo).
- Éviter `unwrap`/`expect` dans le code de production ; documenter types/erreurs publics (`# Errors`, `# Panics` si requis).
- Dépendances minimales, versions du workspace ; migration additive, réversible, sans perte — **seulement si 0008 la nomme**.
- Préserver les modifications utilisateur ; aucune commande Git destructive.
- Sécurité d’abord : clés, registry, workers, admission.

## Vérification

Exécuter au minimum :

1. `cargo fmt --all -- --check`
2. `cargo check` ciblé sur les crates nommés par 0008
3. `cargo test` ciblé (P0–P3 non régressés + `apparatus_p4_t1`…`t12`, distingués unit/contract/IT)
4. `cargo clippy` niveaux workspace si le temps reste raisonnable
5. Diagnostics IDE sur les fichiers modifiés

Corriger toute régression introduite. Ne pas élargir aux défauts préexistants sans rapport (compose service-local inclus). Le gate Manifesto doit rester **plus strict** que « K8s autorisé partout parce que 0008 l’a choisi ailleurs ».

## Documentation de sortie

Mettre à jour **uniquement les faits** après preuves T2+ : champ `Réalité` de **0008** (`Partial` en slice-1) ; **ne pas** passer 0002/0003/0005 à `Implemented` ; preuve P4 dans le plan wiki **en tant que faits de tests**, pas en copiant L152 comme spec. Index ADR + pointeur wiki déjà posés à l’Accept. Canvas lecture seule.

## Compte rendu final

Fournir : Phase 0 (**0008 Accepted** / Réalité **Partial** post-lot ; Unimplemented au tour d’écriture 2026-09-20) ; T2–T12 **livrés** post-lot (gel « ne pas T2+ » = historique 2026-09-20) ; moteur / registry / signature **tels que 0008 les nomme** ; BC-A (pas de 6ᵉ hexagone) ; fichiers / tests T1–T12 ; commandes + résultats ; gates Manifesto (tokens **réels**, `k8s` toujours interdit **dans Manifesto**) ; régression Lazaret `invoke` ; 0002/0003/0005 **toujours Partial** ; items Hors périmètre **non touchés** ; éléments volontairement différés P5/P6 / drain / APP-05 / budget numérique ; risques réellement bloquants.
