# Plan QA — mission M6 « preuve fiable et reproductible » (N0, proof hardening)

- **Lot** : mission `docs/missions/20261018-apparatus-m6-proof-hardening.md`, étape **N0** (`qa-planner`, scores manuels PM 52/55). Tâche `task_mvdslyvu_001d_qpx7wu`, équipe `mission-ce5af9225f72e1c6`, branche `ensemble-crisp-atlas-mission-ce5af9225f72e1c6#0cfldw-pool-1`.
- **Propriétaire du plan** : `qa-planner` (ce fichier). Exécution : N2/N3 (`test-engineer`, worktrees disjoints), N4 (lead), N5 (`test-engineer`), N6 (revues fresh). Audit : findings N6 référenceront **PH-<n>** de ce fichier.
- **HEADbase vérifié (2026-10-18)** : `02dcd23f579a2e8ca3d3526046eb019bf433186e` — racine canonique (master) et worktree pool-1 au même commit (arbre worktree propre) ; c'est le commit de clôture m6 attendu par la mission. Gitlink rustycog `160000 41a86dba0a24844f900783e19ef2405319d58b3f` (`git ls-files --stage rustycog`), immobile. Porcelain canonique : 6 fichiers Obsidian modifiés + non suivis `.opencode/missions/`, `docs/missions/`, `_raw/`, référence wiki G17 — **modifications utilisateur à préserver, jamais incluses dans une livraison writer**.
- **Planification seule (N0)** : aucun Docker/Kind/WSL/kubectl, aucun cargo, aucun test lancé ; aucune écriture hors `.cursor/qa-plans/20261018-m6-proof-hardening.md` (worktree pool-1). Les faits runtime (état Docker/cluster jour J) restent des gaps « à vérifier à N4/N5 ».
- **Canon** : ADR `Accepted` (`docs/adr/`), contrats `docs/contracts/`, code = réalité. Niveaux : pin ADR 0200–0202 (IT réelle, mocks **sortants** seulement, queues opt-in). **Aucune stratégie de test écrite possédée par des humains au-delà des ADR** : l'absence reste nommée, pas comblée (décision humaine requise si un jour elle est voulue).

## Sommaire
- §0 Périmètre, obligations sources, exclusions
- §1 Faits vérifiés (file:line, base `02dcd23`)
- §2 Étape A — matrice de risques (distincte du scoring modèle)
- §3 Modes de défaillance (items ≥ 10) + passe heuristique
- §4 Étape B — inventaire des preuves existantes (réutilisé, pas redécouvert)
- §5 Étape C — registre obligation→test (C-1…C-6, T-1…T-3, M2, M10, E-12)
- §6 Limites N1/N2/N3 proposées + besoins prod séparés
- §7 Commandes exactes (vérifiées depuis Cargo.toml)
- §8 Étape D — critères d'entrée/sortie, échec probant
- §9 Budget (noyau 75 % / buffer 25 %)
- §10 Incertitudes et décisions restant à prendre
- §11 Journal

## 0. Périmètre, obligations sources, exclusions

**Sources des obligations** : briefing correctness P6a (`20261018-m6-e2e-chain-correctness.md` : C-1…C-6), briefing tests P6b (`20261018-m6-e2e-chain-tests.md` : T-1…T-3, M1…M10, registre E-1…E-14), briefing infra P6 (`…-infra.md` : PASS + écarts G-1/G-2/G-3), mission §Registre (lignes 25–36). Toutes sont **explicites** (comments de revue) sauf mention contraire au registre.

**Objet de la mission** : fermer les commentaires m6 en une tranche — fixture non destructif testé, construction HTTP sans dérive de staging, canary probant, preuves par hop archivées — puis UNE campagne IT Windows et UNE exécution E2E Kind autorisée.

**Exclusions (citées, non rouvertes)** : `kind-aiforall-local-full` et lab OAuth 0412 (GO dédiées), gold path 0605, **m5 run Kind** (jamais exécuté ; seuls ses producteurs sont testés), les **quinze anciennes suites Kind** G-17.10 (jamais réputées exécutées/compilées par m6), les résidus credentials S-1/S-2/RP-A/RP-B/N-1/N-2 et 7 unités `cfg(unix)` (cités au handoff, jamais corrigés ici), m2/t12/t2/t7 hors axes, changement de Réalité ADR, promotion ADR, mesh/CI/SDK, `kind-apparatus-p4-it` et `rancher-desktop` (jamais ciblés), `.cargo/config.toml` et gitlink rustycog (intouchés).

**Barrière de lot** : aucun lot suivant (N1 inclus) avant relecture + commit du plan par le lead.

## 1. Faits vérifiés (file:line, base `02dcd23`)

Lectures directes (grep/lecture, zéro runtime ; worktree pool-1 ≡ canon au même commit). Les artefacts `.tmp/` sont ignorés par git : lus dans la **racine canonique** `C:\Users\djden\source\repos\AIForAll\.tmp\` (absents du worktree — attendu).

### 1.1 Fixture Kind (`workers/apparatus-operator/tests/fixtures/kind/mod.rs`)

- `:10` `DEFAULT_CLUSTER_NAME = "aiforall-local"` ; `:12` `CLUSTER_NAME_ENV = "APPARATUS_KIND_CLUSTER"` ; `:19-24` `cluster_name()` — filtre `!name.trim().is_empty()` mais **retourne la valeur brute** (padding non tronqué → mismatch `kind get clusters` → Err fail-loud non destructif) : ancre **C-5**. **Aucun filtre des noms protégés** (`kind-apparatus-p4-it`, `rancher-desktop`) ni des noms invalides : ancre **C-6** (protection aujourd'hui procédurale, doc-comment :16-17).
- `:51-75` `ensure()` non destructif : `assert_docker_running` → `resolve_kind_bin` → `cluster_exists` (**absent → Err explicite**, :59-65, jamais create) → `export_kubeconfig` → `wait_api_server` → `assert_cni_ready` (kindnet → Err ; calico-node absent → Err « opération runtime deploy-j2.ps1 ») → `apply_operator_manifests`. **Ordre actuel : la validation du nom, si ajoutée, doit précéder `assert_docker_running` pour tenir « rejet avant tout appel externe » (C-6).**
- `:83-105` `apply_operator_manifests` : `kubectl apply --server-side --field-manager=apparatus-it-fixtures` (:88-89) **sans `--force-conflicts`** + wait CRD Established 90 s — ancre **M10**.
- `:1039-1051` `resolve_kubectl`/`bin_ok` : `Command::new(name).arg("version")` **sans `--client`** pour kubectl → contacte le serveur du kubeconfig **par défaut** (périmé → exit 1 → faux « kubectl not found in PATH », prouvé ROUGE run1) : ancres **C-2/T-3**. Le probe `kind` (`kind version`) est déjà client-only — ne pas le durcir.
- Aucune fonction destructive restante (audit P6b : grep 0 pour `delete_cluster`/`create_cluster`/install-Calico/`FELIX_*`, 1 doc-comment :17 seul vestige du littéral `apparatus-p4-it`).

### 1.2 Stagings — trois producteurs, deux sans reqwest/rcgen (ancre C-1)

| Producteur | Bloc `[workspace.dependencies]` | reqwest/rcgen ? |
|---|---|---|
| `tests/fixtures/chain.rs` | :192-203 | **OUI** :202-203, verbatim ≡ racine `Cargo.toml:88` (reqwest 0.12.0 json/rustls-tls) et `:142` (rcgen 0.13.2 x509-parser) — vérifié lecture |
| `tests/apparatus_m5_invoke_isolated.rs` | :168-177 | **NON** (serde :169, axum :176, pas de reqwest/rcgen — grep 0) |
| `ops/deploy/apps/overlays/kind-demo-monolith/schedule-reference-kv.ps1` | :124-133 | **NON** (serde :125, axum :132 — grep 0) |

La crate `crates/apparatus-reference-kv/Cargo.toml:11,24-25` hérite `reqwest`/`rcgen = { workspace = true, optional = true }` (feature http) ; `Dockerfile.http:8` construit `--features http`. Conséquence : tout run futur m5/gold path serait ROUGE au stage cargo (mécanisme prouvé par ROUGE run3 sur m6 avant E-14).

### 1.3 Test m6 (`tests/apparatus_m6_e2e_chain.rs`) — oracles inchangés, fenêtres connues

- Pré-suppression bornée pod/CR homonymes (noms dérivés du digest) : :183-190 — idempotence re-run.
- **Canary (ancre T-2)** : sonde wget :392-425 **AVANT** le wait canary Ready **best-effort** (:427-436, `let _ =` résultat ignoré) → fenêtre faux-vert « canary sans endpoints ». L'oracle accepte `wget_blocked_signal || !success` → « connection refused » d'un canary mort ressemble à un blocage netpol.
- Cargo.toml :70-72 : `[[test]] name = "apparatus_m6_e2e_chain"`, `required-features = ["admit", "controller"]` (vérifié lecture) — le piège du skip silencieux (features manquantes → cible sautée avec warning) est traité par comptage `--list`, jamais par l'absence d'erreur.

### 1.4 Docs ops et config fixture (ancres C-3/C-4)

- `ops/deploy/README.md:52` « …(ne pas retarget) » et `:55` « Fixture … Intouchable ; Calico + nom `apparatus-p4-it` » — contredits l'état committé (retarget borné autorisé, Calico jamais installé par le fixture).
- `workers/apparatus-operator/tests/fixtures/kind/cluster.yaml` : `:3` `name: aiforall-local`, `:8` `podSubnet: "192.168.0.0/16"` — divergent du canon `ops/deploy/kind/cluster.yaml:11` (`10.244.0.0/16`) **pour le même nom** ; fichier mort (unique consommateur `create_cluster` supprimé ; grep zéro référence code).

### 1.5 Preuves runtime existantes (racine canonique, ignorées par git)

- `.tmp/m6-evidence-20261018-pool14/` : 23 fichiers, 116 894 octets, SHA-256 source/copie identiques ; `m6-e2e-chain-run4.log` = `6033b54c…` ; run4 `1 passed; 0 failed … 311.79s` sur code `e66b736`. Négatif bogus : FAILED 0,86 s, Err explicite, `kind get clusters` inchangé.
- `.tmp/docker-resources-20261018.log` : 22 lignes — enregistrements **rétrospectifs** explicités, G-1/G-2/G-3 reclassés, BuildKit `UNKNOWN+ask`, réseau du 2026-09-20 non possédé.
- `.tmp/m6-lead-recovery-20261018.md` : limites de livraison (captures autonomes absentes ; **target E: non démontré** — binaire issu du target local C: du worktree).

## 2. Étape A — Matrice de risques du lot

**Distincte du scoring modèle** : les scores PM (complexité/risque 0–100, bandes de routage policy) mesurent la difficulté d'exécution des lots ; cette matrice mesure la **gravité d'un échec d'obligation** (impact) et sa **vraisemblance** (probabilité : churn, branches, catégories historiques du dépôt). Le churn élève la probabilité, jamais l'impact. Zones : 15–25 critique, 10–14 haut, 5–9 moyen, 1–4 bas. Catégories historiques actives : mock/staging qui masque un invariant (dérive staging prouvée 2×), faux diagnostic environnemental (run1), faux vert canary (T-2), preuve non traçable (registre tardif, target C:).

| Id | Comportement / contrat (obligation) | Impact | Prob. | Score | Zone | Profondeur attendue |
|---|---|---|---|---|---|---|
| PH-R1 | Dérive de staging ré-introduite chez un des trois producteurs (C-1) | 3 | 4 | **12** | haut | test de cohérence statique nouveau (N2) ; buffer : gate de compilation staging m5 |
| PH-R2 | Nom protégé/invalid atteint `ensure()` via override (C-5/C-6) | 5 | 2 | **10** | haut | contrat N1 + garde N3 rejetée avant tout appel externe, tests sans Docker |
| PH-R3 | Voie destructive re-réintroduite dans le fixture (M2) | 5 | 2 | **10** | haut | scan source statique + négatif bogus (déjà exécuté) + audit diff N4/N6 |
| PH-R4 | Preuve canary fausse : canary mort → pass « netpol bloque » (T-2) | 4 | 2 | **8** | moyen | réordonnancement fail-loud + contrôle positif avant négatif (N3) |
| PH-R5 | Faux diagnostic probe serveur → ROUGE environnemental à chaque shell frais (C-2/T-3) | 2 | 4 | **8** | moyen | probe client-only (N3, 1 ligne) + validation N5 |
| PH-R6 | Captures H1–H4 absentes au run final (T-1) | 3 | 4 | **12** | haut | instrumentation N3 + `--nocapture` archivé N5 |
| PH-R7 | Écrasement silencieux de champs d'autres managers (M10) | 4 | 2 | **8** | moyen | scan statique SSA + inventaires avant/après N5 |
| PH-R8 | Preuve non traçable : target C:, registre tardif, généalogie floue (E-12/G-1..G-3) | 2 | 4 | **8** | moyen | protocole N4/N5 (env-only + registre à la création) |
| PH-R9 | Secret (token/kubeconfig/argv) dans les captures archivées | 5 | 2 | **10** | haut | liste de marqueurs autorisés au contrat N1 + revue sécurité N6 |
| PH-R10 | Régression des IT existantes par les diffs N2/N3 | 3 | 2 | **6** | moyen | campagne IT finale N4 (t6 9/9 + m1 1/1), statiques filtrées en buffer |
| PH-R11 | Run E2E N5 sur cluster partagé avec code de test modifié (résidus, co-régulation) | 4 | 3 | **12** | haut | préconditions CNI/cluster fail-loud, ordre IT→E2E, inventaires avant/après |

Aucun item critique (15–25) : les pires combos (impact 5) ont une probabilité contenue (2) parce que le code vient d'être audité et que le blast radius est borné par l'`ensure()` non destructif. **Concordance avec le PM** : le risque mission 55 (et N3 65, N5 65) se concentre sur « faux positifs de campagne et limites du runtime partagé » — la matrice dit la même chose (PH-R2/R3/R9/R11) ; aucune contradiction structurelle à escalader, board non modifié.

## 3. Modes de défaillance (items ≥ 10) + passe heuristique

| Id | Déclencheur | Blast radius | Détection actuelle | Mitigation actuelle | Gap |
|---|---|---|---|---|---|
| PH-R1 | Édition future d'un staging m5/ps1 ou de la crate sous feature http sans répercuter reqwest/rcgen | ROUGE au stage cargo d'un futur run m5/gold path — campagne perdue, diagnostic tardif | Aucune automatisée (P6a = inspection manuelle) | Aucune | **PH-1** : test de cohérence statique (N2) |
| PH-R2 | `APPARATUS_KIND_CLUSTER=kind-apparatus-p4-it` (ou nom invalide) posé dans un shell, `ensure()` lancé | Mutation additive d'un cluster **protégé** (apply SSA + kind load + lectures) — violation d'une règle de sécurité dure même sans destruction | Aucune en code (procédurale seulement) | GO + doc | **PH-2/PH-3** : rejet code + tests sans Docker |
| PH-R3 | Régression du fixture vers delete/create/install-Calico (copier-coller d'un ancien pattern) | Destruction possible du cluster partagé si branche non-conforme réactivée | Audit diff humain seulement | Barrière D-7 documentaire | **PH-4** : scan source statique (pattern t11) |
| PH-R6 | Run final sans instrumentation ou sans `--nocapture` | Aucun artefact brut H1–H4 opposable ; mutation d'assert indétectable rétroactivement | Aucune (constat T-1 du lot précédent) | Aucune | **PH-6** : marqueurs N3 + protocole N5 |
| PH-R9 | Marqueur de preuve imprudent (dump config, argv, kubeconfig, token) | Secret persisté dans un log archivé du dépôt canonique | Revue a posteriori seulement | Aucune | **PH-5/PH-6** : liste de marqueurs au contrat N1 + contrôle N6 sécurité |
| PH-R11 | Précondition cluster non vérifiée (CNI, résidus) ou delta inattendu au run N5 | Faux vert / mutation non tracée du cluster partagé | Inventaires manuels du lot précédent (ponctuels, pas automatisés) | Ordre IT→E2E | **PH-7/PH-11** : préconditions fail-loud + inventaires avant/après obligatoires |

**Passe heuristique (états/frontières/ordre/idempotence)** :
- **États** : cluster prêt/absent/non-conforme (kindnet, Calico absent) ; canary Ready/ImagePullBackOff/sans endpoints ; kubeconfig par défaut périmé vs exporté ; staging cohérent/dérivé ; Docker tiers Up/Exited.
- **Frontières** : override env vide/avec padding/protégé/invalide (C-5/C-6) ; kubeconfig par défaut périmé + probe client-only (C-2) ; canary joignable vs bloqué vs mort (T-2 : trois signaux distincts) ; SSA conflit vs success (M10) ; subnet divergent (C-3).
- **Ordre** : N0→N1→(N2∥N3)→N4→N5→N6 ; dans le code : validation du nom AVANT `assert_docker_running` ; compilation/unités → IT → E2E ; contrôle positif canary AVANT oracle négatif ; registre Docker AVANT créations.
- **Idempotence** : re-run m6 = pré-suppression bornée pod/CR (:183-190) + store temp unique + apply idempotent ; registre append-only (jamais de réécriture G-1/G-2/G-3) ; tests de cohérence re-runnable sans état.

## 4. Étape B — Inventaire des preuves existantes (réutilisé)

- **Acquis à NE PAS refaire** (Do not redo P6a/P6b) : run4 vert 1/1 (311,79 s) ; négatif bogus-cluster (0,86 s, sans mutation) ; `--list` m6 1 test ; mécanisme inheritance reqwest/rcgen (run3 ROUGE + run4 vert) ; 16 sites `ensure()` comptés ; t6 9/9 + m1 1/1 (transport de référence G-17). Un re-run n'est justifié que par le diff N2/N3 lui-même (N4/N5).
- **Niveaux réellement présents** : IT testcontainers réelle (t6/m1), IT Kind réelle (m6, 1 test), scans source statiques (patterns t6/t11), 23 tests unitaires mockés du lot credentials (hors mission). Aucun méta-test du fixture à ce jour (M2/M10) — c'est le vide que PH-4 comble au plus petit niveau.
- **Mocks qui neutralisent un invariant** : stub Manifesto HTTP de m6 = mock **sortant** autorisé (ADR 0201) ; Lazaret serveur in-test = design préexistant (porte P7, hors mission). Le hop binaire ORAS/Cosign et le Pod final ne sont **jamais** mockés pour la preuve réelle.
- **Orphelins / périmés / doublons** : aucun nouveau dans le périmètre ; historiques tracés (N-1/N-2 G-17, T-1/T-2 ancêtres) restent des accepts nommés.

## 5. Étape C — Registre obligation→test

### Amendement de relecture lead — obligatoire (2026-10-18)

Les prescriptions suivantes **remplacent** les niveaux et acceptations insuffisants des lignes PH-2/3/4/5/8/9/10/16 ci-dessous. Ces lignes décrivent la proposition N0 initiale, pas des acceptations du lead. Le contrat N1 doit concrétiser le port de commandes test-side nécessaire ; aucune modification de production n'est autorisée par cet amendement.

- **PH-2** : appeler le véritable flux d'entrée `ensure()` avec un enregistreur de commandes sortantes injecté. Chaque cible protégée, non autorisée ou invalide produit une erreur explicite et **zéro commande enregistrée**, avant même Docker/version ; un simple test de fonction pure ne prouve pas cette propriété d'ordre. Nom autorisé nominal : progression jusqu'aux préconditions avec les seules commandes attendues. Les autres lectures externes éventuelles sont incluses dans le port ou explicitement contrôlées par l'oracle.
- **PH-3** : nominal padding normalisé vers `aiforall-local`, sans élargir la liste autorisée. Négatifs distincts : chaîne vide après trim, séparateur de chemin, alias protégé et nom valide mais non autorisé ; tous doivent échouer avant appels. Contrat N1 fige noms/alias.
- **PH-4 / PH-10 (M2)** : enregistreur/sorties CLI scriptées pour les branches cluster absent, CNI incompatible/absent, erreur de commande. Vérifier erreur fail-loud et absence de create/delete/install/remédiation dans **toute la trace**. Les scans source sont seulement un backstop complémentaire. Aucun cluster partagé n'est dégradé pour ces tests ; pas de GAP-accept automatique des branches simulables.
- **PH-5 / PH-9 (M10)** : vérifier les argv effectivement transmis au port sur le chemin SSA nominal (`--server-side`, field-manager attendu, jamais force-conflicts). Simuler conflit d'objet appartenant à un autre manager : propagation d'erreur, aucune seconde tentative forcée, aucune suppression/recréation ; état préexistant du simulateur inchangé. Cela prouve la réaction du client, **pas** le comportement serveur Kubernetes. La non-destruction d'objets réels préexistants reste vérifiée par inventaire lecture seule avant/après N5 ; un conflit serveur réel non provoqué volontairement reste une limite nommée, pas une preuve fabriquée ni un accept global.
- **PH-8** : test comportemental de résolution via port enregistré : `kubectl version --client` et probe kind client-only, sans dépendance à une réponse serveur/kubeconfig fonctionnel ; erreur du binaire distinguée de serveur indisponible. Un test d'argv à la frontière sortante est discriminant et autorisé ; la mention « mocké = théâtre » ci-dessous est caduque. Le run réel demeure complémentaire ; ne pas modifier le kubeconfig utilisateur pour tester la panne.
- **PH-16** : un nom bogus désormais refusé par la garde prouve un rejet, **pas** l'absence d'un cluster autorisé. La branche `aiforall-local` absent doit être testée via port scripté avec erreur et trace sans mutation. Pas de re-run réel avec cible hors liste autorisée.
- **PH-7** : outre la preuve réelle N5, prévoir négatifs test-side readiness échouée, endpoints vides et contrôle positif échoué ; chacun bloque l'oracle négatif et interdit l'émission d'un marqueur d'isolation réussie.
- Aucun de ces tests de commandes sortantes ne remplace ORAS/Cosign, les services réels ou le Pod final de la chaîne m6 réelle. Oracles et commandes des nouvelles cibles seront consignés après leurs entrées Cargo effectives. Le nombre de tests réels de la cible m6 reste un.
- **Portes** : N0 validé par le lead sous cet amendement ; N1 reste sans runtime et doit être relu avant N2/N3. Les GAP-accept initiaux PH-9/10/16 ne sont pas approuvés tels quels. Tout résidu matériel est présenté séparément avec propriétaire, impact, raison et condition de réouverture à la sortie.

États : `COVERED` (fichier:mnémonique + pourquoi ça prouve), `GAP` (actionnable : priorité/niveau/scénario/oracle/effort), `GAP-accept` (pourquoi acceptable), `GAP-defer` (qui décide, quand). Chaque obligation matérielle porte ≥ 1 nominal + 1 négatif ; un intent par invariant ; une inférence reste étiquetée. Les findings N6 référenceront PH-<n>.

| PH | Obligation (source, type) | Intent de test / preuve attendue | Preuve existante | État | Priorité | Niveau | Effort |
|---|---|---|---|---|---|---|---|
| **PH-1** | **C-1** (MEDIUM, explicite) : les 3 producteurs de staging fournissent toutes les deps héritées sous feature http | Nouveau `[[test]]` dédié (N2, fichier nouveau, sans run Kind) : pour chaque producteur (chain.rs, m5, ps1 — chemins listés), toute dep `{ workspace = true }` de `crates/apparatus-reference-kv/Cargo.toml` sous feature http est présente verbatim dans le bloc staging. **Oracle** : entrées exactes attendues (reqwest 0.12.0 json/rustls-tls, rcgen 0.13.2 x509-parser, cf. racine :88/:142) ; échec = rouge nommant producteur + dep manquante. **Négatif du discriminant** : retirer une entrée → rouge (prouvé par construction du test, journal N2) | run3 ROUGE (mécanisme) ; grep confirme m5/ps1 sans reqwest/rcgen | **GAP** | haut | test statique sans Docker | S |
| **PH-2** | **C-6** (INFO, explicite) : noms protégés et invalides rejetés **avant tout appel externe** ; noms/alias/comportement figés au contrat | Contrat N1 (`docs/contracts/`) fige : liste des noms protégés (`kind-apparatus-p4-it`, `rancher-desktop`), critères de nom invalide, comportement (Err explicite, jamais create/delete). N3 : la validation devient la **première** étape d'`ensure()` (avant `assert_docker_running`) et vit dans `cluster_name()`/fonction pure. **Test nominal** : nom valide → Ok. **Test négatif** (nouveau `[[test]]` garde, sans Docker) : `kind-apparatus-p4-it` → Err ; `rancher-desktop` → Err ; nom avec séparateurs de chemin → Err ; le test tourne sans Docker (pureté = aucun spawn dans la fonction validée) | néant (protection procédurale seule) | **GAP** (bloqué sur décision N1 pour la liste exacte) | haut | test unitaire test-side | M |
| **PH-3** | **C-5** (INFO, explicite) : padding normalisé sans accepter nom invalide | Fusionné au garde PH-2 : `cluster_name()` retourne `trim()` **et** rejette l'invalide ; test négatif : `"  aiforall-local  "` → `aiforall-local` (ou Err si N1 choisit le rejet strict — décision à figer) | néant | **GAP** | moyen | test unitaire test-side | S |
| **PH-4** | **M2** (acceptée → discriminant exigé) : aucun create/delete/install dans les branches d'erreur, rien de caché | Scan source statique (nouveau `[[test]]` garde, pattern `t11_manifesto_src_has_no_k8s`/`t6_build_sources_have_no_transit_client`) : `tests/fixtures/kind/mod.rs` ne contient aucune séquence destructive (`kind delete cluster`, `kind create cluster`, apply/install Calico, `FELIX_IGNORELOOSERPF`) ; les erreurs d'`ensure()` sont des `Err` sans remédiation. **Anti-théâtre** : le motif interdit est exactement celui des fonctions supprimées (mutation « re-add delete_cluster » → rouge) | audit diff P6b (grep 0) + négatif bogus exécuté — mais aucun test automatisé qui RENFORCE l'invariant | **GAP** | haut | test statique sans Docker | S |
| **PH-5** | **M10** (acceptée → discriminant exigé) : SSA sans `--force-conflicts`, préservation des objets d'autres managers | Même fichier garde que PH-4 : `apply_operator_manifests` contient `--server-side` + `--field-manager=apparatus-it-fixtures` et **aucun** `--force-conflicts` (conflit = erreur = fail-loud) | SSA exécuté run4 (labels J3 intacts) mais non renforcé par test ; branche conflit jamais exercée | **GAP** (statique) + **GAP-accept** (runtime, voir PH-9) | moyen | test statique sans Docker | S |
| **PH-6** | **T-1** (MEDIUM, explicite) : captures autonomes H1–H4 sans secrets | N3 instrumente m6/chain.rs : marqueurs eprintln par hop (digests 64 hex, `VALID`, `Scheduled`, HTTP 403/200, `hops`, hostname, résultat canary) **sans** argv/auth/config privée/clé/token. N5 : run `--nocapture` redirigé vers log archivé (dépôt canonique avant livraison). **Oracle** : log contenant chaque valeur brute du tableau §6.3 du plan P4 + contrôle négatif de confidentialité N6 (liste de marqueurs autorisés = contrat N1) | run4 vert = assertions exécutées, captures absentes (T-1 GAP-accept historique) | **GAP** | haut | instrumentation + IT Kind réelle | M |
| **PH-7** | **T-2** (LOW, explicite) : canary Ready + endpoints + contrôle positif **avant** l'oracle négatif | N3 réordonne : (a) `kubectl wait --for=condition=Ready pod -l app=apparatus-system-canary --timeout=30s` en **assert** (panic si échec) ; (b) endpoints du Service non vides, fail-loud ; (c) **contrôle positif** de disponibilité (cible joignable autorisée par la netpol — définition exacte au contrat N1 causalité canary) ; (d) négatif distinguant cible absente/indisponible/refus réseau — « n'importe quelle erreur wget » ne vaut plus preuve de NetworkPolicy. **Oracle** : ordre des asserts au diff + captures N5 montrant le contrôle positif OK avant le négatif | fenêtre faux-vert documentée (P6b) ; canary observé Ready en post-run (corroboration, pas preuve) | **GAP** | haut | IT Kind réelle | M |
| **PH-8** | **C-2 / T-3** (MEDIUM/LOW, explicites) : probe kubectl client-only | N3 : `kubectl_ok()` dédié avec `kubectl version --client` dans `resolve_kubectl` ; `bin_ok(kind)` **inchangé** (kind version déjà client-only). **Validation N5** : shell frais sans KUBECONFIG pré-exporté, kubeconfig par défaut périmé → `ensure()` franchit le resolve (capture dans le log du run). Un test unitaire mocké du probe serait du théâtre : le discriminant est le comportement runtime | ROUGE run1 (faux « not found ») prouve le mécanisme adverse | **GAP** | moyen | fix fixture 1 ligne + validation E2E | S |
| **PH-9** | **M10** volet runtime : préservation prouvée sur objet préexistant d'un autre manager | Non exercisable sans planter un objet préexistant = mutation du cluster partagé (interdit pour tester une branche de fixture) | run4 : ns créées PAR le fixture (non discriminant) ; labels J3 intacts | **GAP-accept** — owner lead ; impact : écrasement silencieux non détecté en runtime ; raison : impossibilité propre sans casser le partage ; **réouverture** : avant tout run fixture sur un cluster où J2 préexiste dans `apparatus-system`/`apparatus-plugins` (l'inventaire avant devient alors discriminant) | — | — | — |
| **PH-10** | **M2** volet runtime : branches kindnet / Calico-absent jamais déclenchées | Idem PH-9 : provoquer la branche exigerait un cluster non conforme = panne provoquée du partage | backstops structurels nommés (P6b : rollout calico ; canary joignable → ROUGE si netpol inappliqué) | **GAP-accept** — owner lead ; **réouverture** : avant toute évolution du flux `ensure()` ou changement de CNI du cluster partagé | — | — | — |
| **PH-11** | **E-12** (explicite) + G-1/G-2/G-3 : target E: prouvé, registre à la création, IDs réels et généalogie | **N4** : `$env:CARGO_TARGET_DIR = "E:\cargo-target\AIForAll"` env-only dans le shell des cargo + preuve du chemin effectif consignée (artefacts frais sous E:, aucun `target\` local dans le worktree). **N4/N5** : registre Docker du jour append-only **à la création** (id retourné, `safe_to_delete`, généalogie : remplacement d'une paire = ligne dédiée, jamais `prior_state=absent` trompeur) ; création opaque = `UNKNOWN+ask` ; caches/réseaux tiers jamais adoptés ; **aucune réécriture de l'historique** G-1/G-2/G-3 | recovery : target C: utilisé au run4 (écart tracé) ; registre du 18 régularisé tardivement (omission explicite) | **GAP** (protocole) | moyen | protocole N4/N5 | S |
| **PH-12** | **C-3** (LOW, explicite) : config fixture morte alignée ou supprimée proprement | N2 : aligner `tests/fixtures/kind/cluster.yaml` sur le canon (`podSubnet: "10.244.0.0/16"`, canon `ops/deploy/kind/cluster.yaml:11`) + commentaire « référence seule ; la création du cluster est une opération runtime (deploy-j2.ps1), jamais un effet de test » — OU suppression après vérification des références (grep zéro usage code déjà fait P6a) et décision tracée au diff. **Oracle** : cohérence subnet entre les deux YAML si le fichier reste ; zéro référence code | divergence 192.168/16 vs 10.244/16 vérifiée | **GAP** | bas | config test-only + doc | S |
| **PH-13** | **C-4** (LOW, explicite) : trois sites docs ops alignés sur l'état committé | N2 : `ops/deploy/README.md:52,55` + `ops/deploy/kind/cluster.yaml:5` disent : défaut `aiforall-local`, override `APPARATUS_KIND_CLUSTER`, ensure réutilise sinon fail-loud, jamais delete/create/install-Calico. **Oracle** : lecture des 3 sites (revue N6) — pas de test | sites pré-mission vérifiés | **GAP** | bas | doc | S |
| **PH-14** | Stagings m5/gold path **jamais exécutés** — honnêteté de la preuve | Aucune prétention de run m5 Kind/gold path 0605 dans les livrables ; la compilation possible du staging m5 (gate de cohérence) reste du **buffer** N2, jamais une exécution | N-1 hérité (m5 corrigé jamais compilé) | **GAP-defer** — owner lead/utilisateur (GO dédiée m5/0605) | — | — | — |
| **PH-15** | Quinze suites Kind G-17.10 + résidus credentials (S-1/S-2/RP-A/RP-B/N-1/N-2, 7 cfg(unix)) + m2/t12/t2/t7 | Cités au handoff, jamais implémentés ici | tracés aux plans/briefings précédents | **GAP-defer** — owner lead/utilisateur ; **réouverture** : GO explicite dédiée | — | — | — |
| **PH-16** | Négatif « cluster absent → Err sans mutation » reste vrai après les éditions N3 de `ensure()` | Re-proof **seulement si** la sémantique d'erreur change (ordre/msgs) : un re-run bogus en préfixe N5 (commande identique au lot précédent, log séparé). Sinon : Do not redo | négatif bogus exécuté 0,86 s, clusters inchangés | **GAP-accept** — owner lead ; **réouverture** : tout changement de l'ordre d'erreur d'`ensure()` | — | — | — |

**Séparation faits / inférences / recommandations / décisions** : §1 = faits vérifiés. PH-2/PH-3 dépendent d'un **contrat N1** (décision humaine à approuver). PH-9/PH-10/PH-16 = accepts bornés (décisions lead). PH-6 : l'inférence « `--nocapture` suffira » est **interdite** — l'instrumentation N3 doit rendre les valeurs réellement imprimées (leçon T-1 : le flag seul n'imprime pas ce que le test ne produit pas). Recommandation de niveau par gap = désignation de ce plan ; l'implémentation appartient à N2/N3.

## 6. Limites N1/N2/N3 proposées + besoins prod séparés

- **N1 (`architecte`, sans runtime)** — contrat sous `docs/contracts/` : (a) garde de périmètre : liste figée des noms/alias protégés, critères de nom invalide, comportement d'une cible non autorisée (Err explicite avant tout appel externe) ; (b) causalité canary : définition du contrôle positif et des trois signaux négatifs distincts ; (c) schéma de preuves : liste fermée des marqueurs autorisés par hop (sans secrets). **Ne décide pas** des niveaux de test (ce plan), n'écrit aucun code, pas d'ADR rétroactive. Approbation lead (voire utilisateur si choix structurel nouveau) **avant** N3.
- **N2 (`test-engineer`, worktree isolé)** — propriétaire exclusif : `tests/apparatus_m5_invoke_isolated.rs` (bloc staging :168-177), `ops/deploy/apps/overlays/kind-demo-monolith/schedule-reference-kv.ps1` (:124-133), `tests/fixtures/kind/cluster.yaml` (PH-12), les 3 sites docs ops (PH-13), le **nouveau fichier de test de cohérence** (PH-1). Interdits : run m5/gold path, `Dockerfile.http`, `tests/fixtures/kind/mod.rs`, `apparatus_m6_e2e_chain.rs`, tout `src/`.
- **N3 (`test-engineer`, autre worktree)** — propriétaire exclusif : `tests/fixtures/kind/mod.rs` (PH-2/3/8), `tests/fixtures/chain.rs`, `tests/apparatus_m6_e2e_chain.rs` (PH-6/7), le(s) **nouveau(x) fichier(s) garde** (PH-4/5 + tests unitaires cluster_name). Vrais ORAS/Cosign et vrai Pod au hop final, jamais mockés pour la preuve réelle. Invariant de structure : **aucun test ajouté dans `apparatus_m6_e2e_chain.rs`** — les tests nouveaux sont des cibles `[[test]]` séparées, pour que `--list` m6 reste « exactement 1 test ».
- **N2∥N3** : fichiers disjoints, **jamais de cargo parallèle** (un slot Windows). Conflit prévisible : aucun (cluster.yaml N2 ≠ mod.rs N3, même répertoire fixtures).
- **Besoins prod séparés** : tout FAIL révélant un bug prod (ex. dérive du workspace racine, `Dockerfile.http` qui redérive) reste **ROUGE** jusqu'à un lot implementer dédié approuvé — jamais glissé au test-engineer ; signaler au coordinateur au fil de l'eau.

## 7. Commandes exactes (vérifiées depuis `workers/apparatus-operator/Cargo.toml`, base `02dcd23`)

Préfixe commun (chaque shell, env-only — `.cargo/config.toml` **jamais modifié**) :

```powershell
$env:CARGO_TARGET_DIR = "E:\cargo-target\AIForAll"   # preuve du chemin effectif consignée (PH-11)
```

Cibles existantes (features vérifiées : m6 `:70-72` = `["admit","controller"]` ; t6 `:35-37` et m1 `:54-56` = `["admit"]`) :

```text
# N4 — preflight compile m6 + comptage (PAS d'exécution) :
cargo test --locked -p apparatus-operator --features admit,controller --test apparatus_m6_e2e_chain -- --list
# attendu : exactement 1 test listé : m6_e2e_0002_0008_chain (invariant préservé par la règle §6 N3)

# N4 — campagne IT finale (ordre : unités/gardes d'abord, IT ensuite, UN SEUL cargo à la fois) :
cargo test --locked -p apparatus-operator --features admit --test apparatus_p4_t6_sign_registry -- --test-threads=1
# attendu : "test result: ok. 9 passed; 0 failed"
cargo test --locked -p apparatus-operator --features admit --test apparatus_m1_reference_kv_pin -- --test-threads=1
# attendu : "test result: ok. 1 passed; 0 failed"

# N5 — run E2E unique (GO runtime + préconditions vertes) :
cargo test --locked -p apparatus-operator --features admit,controller --test apparatus_m6_e2e_chain -- --test-threads=1 --nocapture
# attendu : "test result: ok. 1 passed; 0 failed" + log archivé contenant les marqueurs PH-6
```

- **Piège du skip silencieux** : features manquantes → cible sautée (warning seul). La preuve est le **comptage rendu** (`--list` puis `X passed; Y filtered`), jamais l'absence d'erreur. Aucun `--all-features`, aucun `--include-ignored`, aucun filtre de nom sur m6.
- **Cibles nouvelles** (PH-1/2/4/5) : commandes fixées par N2/N3 depuis leurs entrées `[[test]]` réelles après écriture, format `cargo test --locked -p apparatus-operator --test <nom> -- --test-threads=1` (+ features selon `required-features`) et consignées au journal de ce plan avant N4.
- Le `--list` m6 ne compile **pas** les quinze autres cibles (cibles séparées) — aucune prétention contraire (correction déjà actée au Journal P4 :380).
- Statiques filtrées m3/m4/t11 (2+1+3, buffer) : commandes au plan G-17 §6.2, inchangées.

## 8. Étape D — Critères d'entrée / sortie, échec probant

### 8.1 Porte d'entrée (vérifiée par le lead avant chaque admission)

- **Ce plan relu et commité par le lead** (fichier untracked ne se propage pas aux worktrees) — **barrière : aucun lot suivant avant relecture/intégration**.
- Base `02dcd23` + porcelain utilisateur préservé (jamais committé par un writer) ; gitlink `41a86dba` et `.cargo/config.toml` intacts.
- N2/N3 : worktrees isolés créés depuis HEAD intégré ; slot Cargo unique ; `CARGO_TARGET_DIR` E: env-only.
- N3 démarre seulement après **contrat N1 approuvé** (PH-2 bloque dessus).
- N5 : GO runtime explicite **pour cette mission** + skill `local-runtime-lifecycle` chargé AVANT tout start + registre du jour ouvert AVANT créations + **uniquement `kind-aiforall-local`** (jamais full/protégés) + préconditions lecture : Docker Up, cluster présent, calico-node Ready, kindnet absent, inventaire avant consigné. Cluster non conforme = fail-loud STOP (jamais de remédiation par le test).
- Fixtures/données : existantes (creds fixtures `signer`/root token dev, manifests `k8s/` en repo) — rien à créer.

### 8.2 Porte de sortie (auditée par N6, findings → PH-<n>)

1. Registre §5 fermé ou accept/defer **explicite** (owner, impact, raison, condition de réouverture) — aucune disparition silencieuse de commentaire C/T/M/E.
2. Discriminants négatifs prouvés : noms protégés/invalides rejetés **sans appel externe** (PH-2/3) ; préconditions cluster/CNI fail-loud (PH-9/10/16 selon état) ; SSA sans force-conflicts (PH-5) ; contrôle positif canary **avant** oracle négatif (PH-7).
3. Trois producteurs de staging cohérents + test de cohérence vert (PH-1) ; aucune prétention de run m5/gold path.
4. **Une** chaîne m6 réelle verte (premier run ou triage documenté), captures H1–H4 archivées **sans secrets** (PH-6 + contrôle N6 sécurité), comptages recopiés, aucun skip silencieux.
5. Target E: prouvé (PH-11) ou ROUGE explicite ; registre contemporain ; cleanup vérifié ; ressources inconnues non adoptées.
6. fmt/clippy sans régression (fmt ciblé ; hook os-206 préexistant → preuve équivalente documentée, hook non modifié) ; briefings N6 persistés et findings traités.
7. **Échec probant** : tout FAIL = preuve ROUGE persistée (sortie complète, repro, file:line) **avant** tout correctif ; un test révélant un bug prod reste ROUGE → lot dédié (§6) ; deux tentatives techniques sans progrès vérifiable = escalade AGENTS (jamais un 4e worker identique).

## 9. Budget (noyau 75 % / buffer 25 %)

- **Setup (ligne propre)** : worktrees N2/N3, env-only target E:, registre du jour, préflight inventaire N5 (lecture seule). Estimation faible (~5–10 %) : aucune fixture nouvelle, aucune image nouvelle (réutilise le cache existant).
- **Noyau planifié ~75 %** : PH-1 (cohérence + fix stagings), PH-2/3 (garde + contrat), PH-4/5 (scans M2/M10), PH-7 (canary fail-loud + contrôle positif), PH-6 (instrumentation), PH-8 (probe client-only), PH-12/13 (config+docs), campagne N4 (`--list` + t6 9/9 + m1 1/1 + gardes), run N5 unique avec captures + inventaires avant/après.
- **Buffer nommé ~25 %** : triage du premier run N5 ; gate de compilation du staging m5 (option N2, coupe en premier) ; statiques filtrées m3/m4/t11 (6 tests) ; re-run bogus (PH-16, seulement si déclenché) ; re-run idempotence m6 (seulement hypothèse bornée + GO lead, jamais une boucle) ; capture `docker events` enrichie.
- **Coupe au-delà de la capacité, par risque × effort** : (1) gate de compilation staging m5 ; (2) statiques filtrées ; (3) events enrichis — **jamais** le buffer en général, **jamais** un item haut (PH-2/4/6/7) pour garder des items moyens ; coupe d'abord toute extension mesh/CI et factorisation ; jamais canary, garde de périmètre, confidentialité des captures ou traçabilité. Recommandation au lead : le noyau ci-dessus suffit à la porte de sortie.

## 10. Incertitudes et décisions restant à prendre

1. **Contrat N1** (PH-2/3/5/7) : liste exacte des noms/alias protégés, critères d'invalidité, trim vs rejet strict du padding, définition du contrôle positif canary, liste fermée des marqueurs de preuves. Décide : lead (approbation utilisateur si structurel). **PH-2 reste bloqué dessus.**
2. **C-3 : aligner ou supprimer** `tests/fixtures/kind/cluster.yaml` — les deux sont acceptables ; vérification des références déjà faite (zéro usage code). Décide : N2 au diff, tracée.
3. **État runtime jour N5** (Docker, cluster, CNI, résidus pod/CR du run4 encore en place) : faits à constater, jamais posés ici. Les résidus connus (pod plugin, CR VALID, canary, manifests operator) sont **tracés comme conservés** — leur suppression ciblée reste une autorisation lead, jamais un effet de test.
4. **Re-run bogus PH-16** : seulement si `ensure()` change de sémantique d'erreur.
5. **Négatif cluster absent au N5** si le cluster n'existe plus (froid extrême) : le fail-loud est alors la preuve PH-10 gratuite — le créer reste une opération runtime séparée (lifecycle + GO), jamais un effet de test.
6. **Conformité effective du target E:** jamais démontrée à ce jour (recovery :45) — PH-11 la rend obligatoire à N4 ; si l'hôte l'empêche, ROUGE explicite, pas de contournement silencieux.

## 11. Journal

- 2026-10-18 : création du plan (N0, pool-1, base `02dcd23`, gitlink `41a86dba` immobile). Sources réutilisées : ledgers `20261018-m6-proof-hardening-preparation.md` et clôture `20261017-g17-e2e-chain-mission-prep.md` ; plans QA `20261017-operator-cli-real-binary-transport.md` et `20261017-m6-e2e-chain-kind-ready.md` ; briefings m6 du 18 (correctness/tests/infra) ; quatre briefings credentials du 17 (résidus S-1/S-2/RP-A/RP-B/N-1/N-2 cités, non rouverts) ; `MANIFEST.md` + registres Docker 17/18 + recovery lus dans la racine canonique. Ancres re-vérifiées par lecture (canon ≡ worktree au même commit) : Cargo.toml m6 :70-72, mod.rs :10-24/:51-75/:83-105/:1039-1051, m6 :183-190/:392-436, chain.rs :192-203, racine Cargo.toml :88/:142, m5 :168-177, ps1 :124-133, README :52/:55, canon cluster.yaml :11, fixture cluster.yaml :3/:8. Matrice PH-R1…PH-R11 (haut : PH-R1/R6/R11 = 12, PH-R2/R3/R9 = 10), registre PH-1…PH-16 (GAP 8, GAP-accept 3, GAP-defer 2, dont PH-2 bloqué sur N1). Aucun Docker/Kind/WSL/Cargo/test lancé ; écriture via outil fichier dédié uniquement.
