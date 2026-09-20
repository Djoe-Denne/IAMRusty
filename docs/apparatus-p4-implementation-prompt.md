# Prompt — Implémentation Apparatus P4

Tu travailles dans le dépôt `C:\Users\djden\source\repos\AIForAll`. Ne commite rien. **N’implémente pas P4 dans le tour qui a seulement produit ce prompt.** Ce fichier est le contrat pour un **futur** implementer.

## Contrainte modèle

Politique utilisateur **courante** (2026-09-20). **Ne pas** recopier la contrainte P3-era « seul Grok pour tout ».

| Rôle | Slug Cursor (`model:` de chaque Task / sous-agent) |
|---|---|
| Orchestration, architecture, implémentation difficile, revues avec jugement, ADR-0008 **Proposed** | `cursor-grok-4.6-xhigh` |
| Explore, mécanique, test-reviewer | `composer-2.5-fast` |

Slugs **autorisés seulement** : `inherit`, `composer-2.5-fast`, `cursor-grok-4.6-xhigh`, `muse-spark-1.3-max`.

- Ne jamais choisir **silencieusement** un autre modèle.
- Un agent lancé hors table / hors slugs autorisés = non conforme ; arrêter et relancer.
- `inherit` n’autorise pas à dévier vers un slug absent de la liste.

## Mission

Implémente **P4 — Factory et runtime de production** en TDD strict RED-GREEN-REFACTOR, **après** (1) décision humaine **APP-01** puis (2) ADR **0008** **Accepted**. Slice-1 = **P4-core** seulement. Livre code, tests et mises à jour documentaires **factuelles**. Ne dépasse pas P4. Ne livre **pas** P5/P6.

**Canons primaires (Accepted / Réalité Partial — preuves P4–P6, ne pas SuperSéder)** :

- **ADR-0005** : un seul protocole ; admission, `VALID`, `VERIFIED` et installabilité **distincts** ; register Manifesto **≠** admit ; harness / `apparatus dev` **jamais** `VALID`/`VERIFIED` ; pas de `trusted_skip_gateway`.
- **ADR-0003** : plugin hors processus privilégiés ; workers Factory, admission/signature, contrôleur et gateway = identités OS distinctes ; harness in-process ≠ production ; moteur = **APP-01** (Non décidé jusqu’à 0008).
- **ADR-0002** : Factory = **P4** ; host UI + CLI = **P5** (L19, L35). Les contrats P0 se réutilisent ; pas de second schéma.

**Ancre (2026-09-20)** : ADR-0004 et ADR-0007 **Accepted / Implemented** (A-DEC). T1–T14b = preuve P3. `invoke` = Lazaret `POST /invoke`. 0 identifiant `gateway` sous `Manifesto/*/src`. `/components` = 5. Gate `Manifesto/tests/apparatus_p2_t7_gate.rs` (+ P3 t2) : `k8s`/`kubernetes`, `wasm`/`wasi`/`wasmtime`, `iframe`, `messagechannel`, `apparatus_host`, `ui_host` **restent interdits dans Manifesto** même si 0008 choisit K8s **ailleurs**.

Conduis : attente APP-01 → Phase 0 ADR-0008 Proposed → **STOP T2+** jusqu’à Accept humain → T1 (absence) peut précéder l’Accept → T2–T12 après Accept. N’invente **aucune** décision `Accepted`. N’invente **pas** le moteur, le registry, la signature, ni K8s-oui/non dans le code **ni** dans une ADR auto-Acceptée.

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

Closeout §D reste ouvert (ne pas « clôturer » pendant P4-core) : `D-APP05`, `D-0006G`, `D-0006E`, `D-PROTO2`, `D-APP03`, `D-APP06`, drain/destruction bindings déjà `ready` (0005 L34, README L138 → ADR **future**). `D-K8S` / `D-APP01` : **P3** les laisse ouverts ; **P4 les ouvre seulement via APP-01 + 0008**, pas en les préemptant ici.

## Accepted vs Réalité

`Accepted` = cible ratifiée. `Réalité` = code présent. Ne les confonds pas. Ne réécris pas les décisions `Accepted` de `0001`–`0007`. **Interdit** : SuperSède de **0003** ou **0005** par 0008.

Le wiki (plan, factory, platform) est **conception** (`^[inferred]` / `status: proposed`), **pas** une ADR, **pas** un contrat RED. Interdit de traiter le wiki comme Accepted, et d’inventer un moteur, un registry, un schéma de signature, un `KubernetesAdapter`, ou un répertoire `Factory/` « parce que le wiki les nomme ».

`docs/adr/README.md` L18 : ne pas ADR le moteur d’isolation / registry / signatures **tant que** `APP-01` reste un arbitrage ouvert **et** que le jalon n’ouvre pas. **P4 ouvre ce jalon.** APP-01 **devient** ADR seulement alors (cible : **0008**). Tant que l’humain n’a pas tranché APP-01, 0008 n’existe pas.

En fin de **slice-1 P4-core**, laisser **0002 / 0003 / 0005** en Réalité **Partial**. **Ne pas** flipper 0005 à `Implemented` dans slice-1 (preuves P5/P6 encore dues).

### Contradictions ADR / code / wiki (ne pas lisser)

1. **Plan L152 `KubernetesAdapter` vs 0003 L45–L46 + A-DEC + `D-K8S`.** Le plan wiki `^[inferred]` dit d’« implémenter KubernetesAdapter ». 0003 **Non décidé** : adaptateur de production, y compris le choix **éventuel** de Kubernetes. A-DEC interdit K8s-**as-P3**. Factory **sans** K8s est **autorisée après APP-01** si 0008 nomme **un autre** moteur. `KubernetesAdapter` **seulement si** 0008 **nomme** Kubernetes. Wiki L152 = conception.
2. **Closeout `D-P4` vs 0002 L35 / plan P5.** `D-P4` liste « Factory, **host UI**, OCI, `VALID`/`VERIFIED`, iframe » comme un seul sac P4. 0002 L19/L35 et le plan : Factory = **P4**, host/CLI/iframe = **P5**. **P5 est OUT** de ce jalon. Ne pas « avancer le host » parce que `D-P4` le cite.
3. **Plan / wiki encore « 0007 Partial ».** Ex. plan L148 « ADR-0007 **reste Partial** », matrice L170 « preuves P3 Partial ». **Périmé** vs A-DEC : 0007 **Implemented**. Pointer, ne pas promouvoir la matrice en contrat, ne pas « corriger » le wiki en inventant du P4.

Si une contradiction **bloque** le contrat (second protocole, `trusted_skip_gateway`, bearer IAM, K8s-as-P3, lever G/E, nouveau type FGA, flip 0005 Implemented en slice-1) : **escalade humaine**, n’invente pas.

## Sources de vérité à lire avant de coder

1. `AGENTS.md` et les règles du workspace.
2. `docs/adr/README.md` : prochain entier libre plage **0001–0099** = **0008** (pas de fichier `0008-*` aujourd’hui). L18 (APP-01 n’est pas une ADR avant ouverture du jalon). L134–L136 (pipeline Git→OCI / builders / registry **avant P4, bloqué par APP-01** ; host UI **avant P5** ; moteur `APP-01` **avant tout runtime réel**).
3. ADR `0001`–`0007` dans `docs/adr/` (ne pas réécrire `Accepted`). Canons P4 : **0005**, **0003**, **0002** (L19, L35). P3 livré : **0004** + **0007** Implemented. 0007 L11 (preuves T1–T14b + holes hors-jalon), L211 (escalade : 2e proto / skip gateway / K8s-as-P3 / FGA / levée G/E), L228 (holes A-DEC).
4. `docs/adr/0007-closeout.md` §D (laisser ouvert). Le leftover compose service-local n’est **pas** une source P4 — voir Hors périmètre.
5. `docs/adr/template.md` pour la Phase 0. Prompts `docs/apparatus-p0-implementation-prompt.md` … **`p3`** : **esprit TDD + gates** seulement. **Beaucoup de faits P3 dans le prompt P3 sont périmés** (ex. 0004 Partial) ; l’état réel = A-DEC ci-dessus.
6. Wiki — **conception seulement** :
   - `obsidian/AI FOR ALL/projects/manifesto/references/apparatus-implementation-plan.md` **P4** L150–L196 `^[inferred]` (pipeline, preuve de sortie, `APP-01`…`APP-07`)
   - `obsidian/AI FOR ALL/projects/manifesto/references/apparatus-factory-and-distribution.md` (`status: proposed`) — **pas** un RED
   - `obsidian/AI FOR ALL/projects/manifesto/concepts/apparatus-platform.md`
7. Code réel P3 à **préserver** (régression, pas à réécrire) :
   - `Manifesto/tests/apparatus_p2_t7_gate.rs`, `Manifesto/tests/apparatus_p3_t2_gate.rs`
   - `Manifesto/tests/apparatus_p3_t1_absence.rs` ; Lazaret `Lazaret/tests/apparatus_p3_t7_invoke.rs` (+ t8…t13)
   - Contrats P0 : `apparatus-contracts` (digest canonique, protocole `manifesto-apparatus/1`, **pas** `trusted_skip_gateway`)
   - Harness `test-harness` : double de test, **jamais** `VALID`/`VERIFIED`

Skills : `.cursor/skills/rustycog/SKILL.md` (et `.agents/skills/rustycog/SKILL.md`) si API RustyCog. Fixtures DB : `.cursor/skills/creating-testcontainer-fixtures/SKILL.md`. **Nouveau service** : `.cursor/skills/aiforall-new-service/SKILL.md` **seulement si** l’ADR **0008 Accepted** nomme un nouveau BC. **Ne pas** copier le défaut P3 « pas de microservice ». Le nouveau BC est **inconnu jusqu’à 0008**.

## Périmètre P4 obligatoire (TDD, une tranche après l’autre) — slice-1 P4-core **après Accept 0008**

Invariants **toujours vrais** (sauf ADR Accepted qui les lève — 0008 **ne** lève **pas** G/E, **ne** SuperSède **pas** 0003/0005) : pas de second UUID public ; identité = `project_components.id` ; 1:1 ; `source` ∈ `legacy|managed` ; pas de nouveau type FGA ; `/components` = 5 ; 0 `gateway` sous `Manifesto/*/src` ; `invoke` serveur = Lazaret ; pas de `trusted_skip_gateway` ; un seul protocole ; harness ≠ production ; pas de `VALID`/`VERIFIED` émis par le harness ; register Manifesto ≠ admission ; workers **OS-distincts** (0003) : build / conformance / signer / contrôleur ; sécurité d’abord.

**IN slice-1 (après Accept 0008)** : Git → digest **canonique** (0002) → package → conformance → signature → registry → **admission indépendante**. Refus 0005 (digest altéré, manifeste non conforme, signature inattendue, politique runtime manquante). Image de test **plateforme** avant soumissions tierces. Worker **malveillant** sans clés ni accès plateforme. Adaptateur de runtime **seulement si 0008 le nomme**. En fin de slice-1 : 0002/0003/0005 **restent Partial**.

Nomme les tests `apparatus_p4_t*.rs`. **Ne pas** inventer un répertoire `Factory/` **avant** 0008. Le crate / chemin T2+ = **0008**.

### Défauts prudents (ce prompt, **pas** une ADR Accepted)

À ratifier ou écarter **dans l’ADR 0008 après APP-01 humain**, **pas** à coder avant Accept — **et pas à préremplir ici** :

- Moteur d’isolation, budget, CPU, registry, autorité de signature (**APP-01**).
- Kubernetes **oui/non**. Factory sans K8s = possible **si** 0008 nomme un autre moteur. `KubernetesAdapter` **iff** 0008 nomme K8s.
- Nouveau bounded context **oui/non** (skill `aiforall-new-service` **iff** 0008 le nomme).
- Forme du package / registry (le wiki et 0005 **citent** souvent OCI — conception jusqu’à Accept).
- WASM comme runtime : 0003 a **rejeté** WASM/WASI **en V1 premier runtime** (direction conservée, pas le premier). Ne pas le réintroduire comme moteur slice-1.

### Phase 0 — APP-01 humain puis ADR 0008 (bloquant avant T2+)

**Séquence obligatoire :**

1. **WAIT-FOR-HUMAN `APP-01`.** L’implementer **n’invente pas** moteur / registry / signing / K8s-oui-non. README L18 : APP-01 n’est pas une ADR tant que le jalon n’ouvre pas ; **P4 ouvre** le jalon, **l’humain tranche** le contenu.
2. **Ensuite seulement** : rédiger `docs/adr/0008-….md` depuis `docs/adr/template.md` en **Proposed**. Titre = **la décision**, une phrase. Plage 0001–0099, prochain libre = **0008**. **Pas** de SuperSède 0003/0005. L’implementer **n’accepte pas** lui-même.
3. Digest Serena `architecture/apparatus-p4-adr-0008` **dans le même tour** que le fichier 0008 (règle `adr-serena-digest`). Pas ce tour-ci (ce prompt seul).
4. **STOP T2–T12** (et tout code Factory / workers / registry / signature / adaptateur runtime / admission persistante) jusqu’à Accept **explicite** (utilisateur / PR). Puis ligne `docs/adr/README.md` + index wiki. Wiki reste conception.

**T1 seulement** est autorisé avant Accept.

Checklist Phase 0 (questions **ouvertes** — ne pas y répondre dans le code ni dans une ADR auto-Acceptée) :

1. Contenu humain **APP-01** : moteur, budget/CPU, registry, autorité de signature — **verbatim humain**, pas le wiki L152.
2. 0008 nomme-t-il Kubernetes, un autre moteur, ou reporte-t-il l’adaptateur runtime hors slice-1 ?
3. Nouveau BC ? Quel nom ? Skill `aiforall-new-service` **oui/non**.
4. Combien de processus OS, où vivent build / conformance / signer / contrôleur ?
5. Digest canonique : réutiliser le digest P0 (0002) comme identité d’installation ?
6. Format **package** + produit **registry** (ne pas présumer OCI).
7. Conformance : quelle suite plateforme, quelle version de politique (0005 `VALID`) ?
8. Déploiement de l’autorité d’admission (0005 : indépendante du publisher, du plugin, du harness, de `VERIFIED`).
9. Clés de signature : qui les détient ; le worker de build **n’en a pas**.
10. Adaptateur runtime : **iff** nommé. Sinon T11 = preuve d’**absence**.
11. `invoke` reste Lazaret ; 0008 ne déplace pas l’I/O vers Manifesto.
12. Confirmer **zéro** SuperSède 0003/0005 ; **zéro** levée G/E ; **zéro** type FGA nouveau.
13. Slice-1 laisse 0002/0003/0005 **Partial** — 0008 le dit.
14. Drain / destroy des bindings déjà `ready` : **hors** 0008 (ADR future, 0005 L34).

### Tranche 1 — Preuves d’absence Factory / runtime / admission (sans attendre 0008)

Autorisé **uniquement** par ADR **déjà Accepted** : 0005 (pas d’auto-admission, pas de `VALID`/`VERIFIED` harness, pas de `trusted_skip_gateway`) + 0003 (harness ≠ production, pas de plugin in-process) + 0002 (Factory absente) + gates P2/P3.

- RED : `Manifesto/tests/apparatus_p4_t1_absence.rs` : pas de pipeline Git→package→admission en prod ; harness sans `VALID`/`VERIFIED` ; DTO sans `trusted_skip_gateway` ; 0 `gateway` Manifesto src ; `/components` = 5 ; `ApparatusRuntime` sans `invoke` ; tokens `k8s`/`kubernetes`/`wasm`/`iframe`/`messagechannel`/`apparatus_host`/`ui_host` toujours interdits **dans Manifesto** ; **ne pas** créer `Factory/` ; **ne pas** retargeter les gates. **Ne pas** ajouter de SQL.
- GREEN : caractérisation seulement.
- Sortie : baseline verte. Interdit : workers, registry, signature, adaptateur, scaffold BC.

### Tranche 2 — Scaffold BC (APRÈS Accept 0008, **STOP si non Accepted**)

- RED : bounded context / crate **nommé par 0008** seulement. Skill `aiforall-new-service` **iff** 0008 exige un nouveau service. Pas de répertoire `Factory/` inventé si 0008 n’existe pas ou ne le nomme pas.
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

- **P5** : host UI, iframe, CSP, MessageChannel, serveur de bundles, CLI `check/dev/publish` / proc-macro (0002 L35, README L135).
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

Mettre à jour **uniquement les faits** après preuves : champ `Réalité` de **0008** (`Partial` en slice-1) ; **ne pas** passer 0002/0003/0005 à `Implemented` ; preuve P4 dans le plan wiki **en tant que faits de tests**, pas en copiant L152 comme spec. Index ADR + pointeur wiki. Canvas lecture seule. **Ne pas** écrire 0008 dans le tour qui n’a produit que ce prompt.

## Compte rendu final

Fournir : Phase 0 (**WAIT APP-01** ou ADR **0008** Proposed / **Accepted** / **blocage**) ; moteur / registry / signature **tels que 0008 les nomme** (ou « non tranchés — STOP ») ; nouveau BC oui/non ; fichiers / tests T1–T12 ; commandes + résultats ; gates Manifesto (tokens **réels**, `k8s` toujours interdit **dans Manifesto**) ; régression Lazaret `invoke` ; 0002/0003/0005 **toujours Partial** ; items Hors périmètre **non touchés** ; éléments volontairement différés P5/P6 / drain / APP-05 ; risques réellement bloquants.
