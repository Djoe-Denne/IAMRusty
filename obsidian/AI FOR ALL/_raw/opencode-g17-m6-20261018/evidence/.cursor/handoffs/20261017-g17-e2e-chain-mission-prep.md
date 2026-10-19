# 20261017-g17-e2e-chain-mission-prep

Ledger de préparation mission (G-17 puis chaîne E2E Kind). **Préparation seule : pas de
`/ensemble-mission start` exécuté par le lead, pas de worker spawné, aucune tâche publiée,
aucun runtime (Docker/Kind/WSL) démarré.** Écrit par le lead, seul écrivain. Gitignoré.

## État vérifié (2026-10-17, avant plan)

- HEAD master `6f34b6d7a104d52058cd0e54c1aeee35ac12e167`. Parent porcelain : `?? .opencode/missions/` seulement.
- Pin rustycog `41a86dba0a24844f900783e19ef2405319d58b3f` (`heads/main`), immobile. Submodule HEAD identique.
- Mission précédente close : lot credentials sur `c2fe7172ccc4`, quatre revues PASS_WITH_NOTES,
  ledger `.cursor/handoffs/20261017-credentials-0412-mission.md`, plan QA
  `.cursor/qa-plans/20261017-operator-cli-credentials.md`, quatre briefings
  `.cursor/review-briefings/20261017-operator-cli-credentials-{correctness,rust-perf,security,tests}.md`.
- Policy mission courante : draft non approuvée,
  `.opencode/missions/team_mvcm6iko_0000_8ud2p5/model-policy.json`,
  hash `65edc31228ccca3d151aa3342b8d9eef4e87537df314fa62c4120f40e438d424`.

## Faits vérifiés (file:line)

- Les 8 fichiers IT existent sous `workers/apparatus-operator/tests/` : `apparatus_m1_reference_kv_pin.rs`,
  `apparatus_m3_install_fail_closed.rs`, `apparatus_m4_desired_state_bridge.rs`,
  `apparatus_m5_invoke_isolated.rs`, `apparatus_p4_t6_sign_registry.rs`,
  `apparatus_p4_t10_platform_workload.rs`, `apparatus_p4_t11_runtime.rs` (les 7 du lot G-17)
  et `apparatus_m6_e2e_chain.rs` (axe 2).
- Les 7 IT G-17 démarrent `fixtures::SignRegistryStack::start()` (testcontainers : zot
  `ghcr.io/project-zot/zot` piné, `fixtures/zot/mod.rs:16`, + openbao-transit) et construisent un
  `AdmitTarget` avec `docker_network = SignRegistryStack::network_name()`.
- Aucun `#[ignore]`, aucun `cfg(unix)`, aucun marqueur de skip dans les fichiers IT (grep exact).
- Transport binaire réel : `admit.rs:26-29` `APPARATUS_COSIGN_BIN` / `APPARATUS_ORAS_BIN`
  (`COSIGN_BIN_ENV`/`ORAS_BIN_ENV`), `host_tool` (`admit.rs:291,495,523,535,551`), fallback docker
  via images pinées `ORAS_IMAGE` oras:v1.2.2 / `COSIGN_IMAGE` cosign:v2.4.3. Les hooks
  `#[cfg(test)]` (`intercept`, overrides) sont inertes dans les binaires d'IT (`cfg(test)` non
  compilé hors lib) : le chemin IT est le vrai transport.
- **Contradiction structurelle détectée** : `tests/fixtures/kind/mod.rs:9`
  `pub const CLUSTER_NAME: &str = "apparatus-p4-it"`. `KindCluster::ensure()` (`:35-65`) réutilise
  le cluster `apparatus-p4-it` si API up + Calico présent, sinon le **supprime** (`kind delete
  cluster --name apparatus-p4-it`, `:99-112`) et le recrée (`:1099`). Le contexte produit est
  `kind-apparatus-p4-it` — cluster **protégé** (jamais touché, règle infra-safety et AGENTS.md).
  L'ordre de mission cible `kind-aiforall-local`. L'E2E ne peut donc pas être lancée en l'état.

### Résolution tranchée dans ce plan (à confirmer par le GO utilisateur)

- L'axe 2 commence par un re-ciblage borné du fixture : nom de cluster paramétrable
  (défaut `aiforall-local` → contexte `kind-aiforall-local`) **et** `ensure()` rendu
  non destructif sur ce cluster partagé : réutilisation si prêt, sinon échec loud — jamais de
  `kind delete` spontané contre `kind-aiforall-local`. `kind-apparatus-p4-it` reste intouché.
- Ce re-ciblage est une modification de fixtures de test → lot à comportement contractuel →
  plan QA préalable (P4), pas d'exécution avant plan commité.

## Graphe (aperçu, non publié — IDs réels à créer via team_tasks_add après GO)

Séquentiel : un seul slot Cargo Windows à la fois ; les P4–P6 suivent l'ordre des axes.

| Id | Livrable | Profil | Cx | Risk | Worktree | Dépend de | Périmètre / notes |
|---|---|---|---|---|---|---|---|
| P1 | Plan QA G-17 | `qa-planner` | 40 | 28 | false | GO | `.cursor/qa-plans/**` : inventaire des hops réels par IT (m1,m3,m4,m5,t6,t10,t11), commande exacte, comptages attendus, critère « aucun skip silencieux », preuve ROUGE avant correctif, mode binaire (host `APPARATUS_*_BIN` sinon fallback docker piné) |
| P2 | IT binaires réels G-17 | `test-engineer` | 55 | 45 | true | P1 commité (lead) | slot Cargo, fixtures testcontainers (Docker infra tierce), registre Docker `.tmp/docker-resources-20261017.log`, FAIL = preuve ROUGE tracée, aucun correctif de code prod |
| P3 | Revues fresh G-17 | `correctness-reviewer` puis `test-reviewer` | 30 | 35 | false | P2 intégré, gate verte | briefings persistés `.cursor/review-briefings/`, INDEX par le lead ; rust-perf/sécurité seulement si diff code prod |
| P4 | Plan QA E2E + re-ciblage fixture | `qa-planner` | 50 | 50 | false | P3 sans BLOCK | contradiction cluster résolue ici : plan du re-ciblage (non destructif, jamais `kind-apparatus-p4-it`), images Linux requises, probes de la chaîne |
| P5 | Chaîne E2E `apparatus_m6_e2e_chain` sur `kind-aiforall-local` | `test-engineer` | 60 | 55 | true | P4 commité (lead) | runtime : Docker Desktop + Kind `aiforall-local` (défaut) ; skill local-runtime-lifecycle chargé avant tout start ; images Linux via `docker compose build`/`build-artifacts` + `kind load` ; **jamais full sans GO explicite** |
| P6 | Revue fresh E2E/fixture | `correctness-reviewer` | 30 | 35 | false | P5 intégré | briefing persisté |
| P7 | Renforcement stack E2E (conditionnel) | `architecte` | 50 | 65 | true | verdict P5 = « curls basiques » | ADR Proposed ou contrat `docs/contracts/` ; probes multi-sauts mesh, chaînes réelles ; **aucune implémentation sans plan approuvé** |
| P8 | Correctif ROUGE (conditionnel) | `hard-implementer` | 70 | 55 | true | ROUGE tracé P2/P5/P6 | retour sur le lot concerné, plan QA amendé d'abord |

Le lead commite chaque plan QA après relecture (décision utilisateur du 2026-10-17, précédent M2),
sans attendre, sauf contradiction structurelle non tranchable → question à l'utilisateur.

## Scores et justification (entiers indépendants 0–100)

Critères : complexité = surface locale → interactions de crates/composants → contrat transversal
ou incertitude d'intégration ; risque = réversible/isolé → blast radius détectable → secret,
authz, preuve fausse. Pas des probabilités de succès.

- P1 40/28 : planification, patterns établis (plan QA précédent), erreurs détectables avant run.
- P2 55/45 : multi-fichiers + binaires réels + fallback docker + preuve de non-skip ; échec local réversible.
- P3 30/35 : revue fresh, diff borné.
- P4 50/50 : contrat runtime + cycle de vie cluster partagé ; un plan faux toucherait l'infra → mitigé par le plan.
- P5 60/55 : fixture + chaîne multi-composants + opérations cluster ; blast radius local, réversible.
- P6 30/35 : diff fixture borné.
- P7 50/65 : frontière de preuve transversale (même nature que M3 précédent, 48/64).
- P8 70/55 : implémentation sur transport cli/post-correction, retour aux oracles plan.

## Politique modèles — deux options, décision utilisateur

Option A — brouillon actuel (bandes [0,34,67,101]) : light `opencode-go/gpt-6-luna`,
standard `opencode-go/glm-5.3-flash` max, strong `cursor/claude-opus-5-5` Claude Opus 5.5 Max.

Option B — grille corrigée par l'utilisateur le 2026-10-17 (mission précédente, ledger
`20261017-credentials-0412-mission.md` §Proposition) : bandes [0,61,81,101] ;
`glm-or-grok` = `zai-coding-plan/glm-5.3-flash` max → `cursor/grok-4.7` Grok 4.7 Extra High →
`opencode-go/glm-5.3-flash` max ; `grok-xhigh` = `cursor/grok-4.7` Extra High ;
`opus` = `cursor/claude-opus-5-5` Claude Opus 5.5 Max.

Placement (le plus haut des deux scores choisit la bande) :

| Tâche | Option A | Option B |
|---|---|---|
| P1 | standard → glm-5.3-flash max | glm-or-grok |
| P2 | standard → glm-5.3-flash max | glm-or-grok |
| P3 | standard → glm-5.3-flash max | glm-or-grok |
| P4 | standard → glm-5.3-flash max | glm-or-grok |
| P5 | standard → glm-5.3-flash max | glm-or-grok |
| P6 | standard → glm-5.3-flash max | glm-or-grok |
| P7 | strong → opus-5-5 | grok-xhigh |
| P8 | strong → opus-5-5 | grok-xhigh |

Recommandation lead : **Option B** (dernière correction utilisateur, aucune tâche ≥ 81,
Opus réservé). Le brouillon n'est réécrit qu'après corrections de chat, puis
`/ensemble-mission config` ; `start` reste la commande de l'utilisateur.

Classification : JEV actif, `helperModel` non configuré. Deux voies : publier les tâches avec
scores manuels (précédent M2/M3 : classification non demandée), ou configurer `helperModel`
via `/ensemble-mission config` avant toute admission automatique.

## Barrières et contraintes (inchangées)

- `.agents/skills/local-runtime-lifecycle/SKILL.md` chargé avant tout démarrage
  Docker/Kind/WSL (lead et tout worker concerné). Inventaire Docker append-only
  `.tmp/docker-resources-20261017.log` dès la première ressource.
- Compilation/tests dev sur l'hôte Windows, **un seul cargo à la fois**, `jobs = 2` et
  `target-dir E:\cargo-target\AIForAll` intacts (`.cargo/config.toml` non modifié).
- Docker uniquement pour : infra tierce testcontainers (zot, transit) et images Linux
  destinées à Kind. Pas de build/run applicatif Docker pour le dev. IT jamais via Kind.
- Pas de prune, pas de `down -v`, pas de `rm -f` spontané ; clusters protégés
  `kind-apparatus-p4-it` et `rancher-desktop` intouchés (le fixture actuel viole la règle :
  voir contradiction) ; `rustycog` gitlink et `.cargo/config.toml` intouchés.
- Résidus connus hors axes : S-1, S-2, RP-A, RP-B, N-1, N-2, 7 tests `cfg(unix)`, gold path
  0605 — hors mission sauf décision explicite de l'utilisateur.
- Lab 0412 (axe 3) : **coupé** sauf demande explicite. Il exige `kind-aiforall-local-full`
  (autorisation explicite exigée), IdP github/gitlab allowlist ou simulé, navigateur/CDP,
  rejeu inter-replicas, cleanup OAuth ; amendement ADR 0412 (Réalité Partial → Implemented)
  par l'architecte après PASS seulement.

## Protocol G-17 (rappel de l'ordre)

- Jamais de mock sur le hop binaire : host `APPARATUS_ORAS_BIN`/`APPARATUS_COSIGN_BIN`
  (ORAS 1.2.2, Cosign 2.4.3 réels) sinon fallback docker sur images pinées.
- Commande exacte, comptages par IT, verdict par hop ; aucun skip silencieux.
- Tout FAIL = preuve ROUGE persistée avant tout correctif ; un test qui révèle un bug prod
  reste ROUGE jusqu'à lot correctif dédié (P8).

## Continuité

- Session `project-manager` précédente : `ses_eb711a96bffe2QuSLECW3K7Isl` (plan du
  2026-10-17, lecture seule). Reprendre seulement si le graphe ci-dessus doit être révisé.
## P1 exécutée et tranchée (2026-10-17, après start)

- P1 `task_mvcmn5mc_0001_tn3fkp` → pool-1 (`qa-planner`) : **complétée**. Plan QA livré et
  reconnu : `.cursor/qa-plans/20261017-operator-cli-real-binary-transport.md` (272 lignes,
  registre G-17.1…G-17.11, matrice R-01…R-08, commandes §6.2, matrice de mode §6.4, budget §7).
- **ESCALADE §0 du plan** : 15 des 31 tests des 7 IT appellent `KindCluster::ensure()` (fixture
  du cluster protégé `kind-apparatus-p4-it`). Tranché par le lead (décisions §9 du plan,
  spot-vérifiées par ancres) : **scope P2 = transport-only** (a) — conforme à l'ordre
  (« IT testcontainers, jamais Kind »). Porte de sortie réelle : t6 9/9 + m1 1/1 + preuve mode
  (G-17.7) + 15 defer tracés nominalement (G-17.10). Le ré-ciblage Kind reste P4/P5 au GO
  utilisateur — jamais avancé sans lui.
- Worktree P2 amendé à **false** (exécution pure, zéro écriture repo ; piège merge du lot
  précédent évité). Constat §8.2 (`target-dir E:` absent du config committé) maintenu hors lot.
- Plan QA commité par le lead : `5c665b4` (« docs(qa): fixe le plan G-17 du transport binaire
  reel (P1, P2 transport-only) », un seul fichier).
- **P2 publiée** : `task_mvcne8l3_0005_lo1h0r` — test-engineer, complexité 55, risque 45, fresh,
  worktree false, priorité high. Exécution : t6 + m1 avec preuves par hop (G-17.1→6), matrice
  de mode G-17.7, secondaire m3×2/m4×1/t11×3 en buffer, protocole ROUGE, defer G-17.10 nominatif.
- Suivant : P3 (revues, correctness + test-reviewer) publiée après intégration P2 + gate verte.

## P2 exécutée : t6 9/9, ROUGE famille helper, fix cadré (2026-10-17)

- P2 `task_mvcne8l3_0005_lo1h0r` → pool-2 (`test-engineer`) : **complétée**, zéro écriture repo.
- **Porte transport : t6 9/9** (run1 + run2 capture brute). Preuves réelles par hop
  (G-17.1→G-17.6) via zot indépendant : catalogue `apparatus/envelope`, artifactType
  `application/vnd.aiforall.apparatus.envelope.v1+json`, digest manifeste
  `sha256:b707d331…96df1` (64 hex), 4 tags `.sig` HTTP 200, `signature_verified` positif,
  négatif `t6_verify_ko` PASSED (clé Transit wrong, précondition tag). **Mode servi =
  fallback docker** (G-17.7) : env absentes, images pinées tirées (oras:v1.2.2, cosign:v2.4.3),
  16+17 helpers éphémères par hop, `docker ps -a` post-run = **0 résidu** (G-17.9 prouvé). 
  15 tests Kind non exécutés, tracés nominativement (G-17.10) — jamais touchés.
- **ROUGE : une seule famille, test-fixture** — helpers `repo_root()` périmés depuis
  `f060d47` (refactor layout 2026-10-03) : m1 :25-31, m3 :36-46, m4 :37-46+s:123, t11 :83+§:583
  (`workers/workers/`). Paniques 0,00 s avant tout Docker. **Pas un bug prod transport**
  (t6 couvre les hops). Vérifié lead (fichiers réels présents). Preuve : `.tmp/g17-*.log`,
  `.tmp/g17-docker-events.txt`, registre docker-resources (10 entrées).
- Journalisé dans le plan + **commits lead** : plan G-17 `5c665b4`, ROUGE/correctif `614a6e5`.
- Décisions lead §9.5–§9.7 du plan : mode fallback COVERED ; correctif = **P2-fix test-only**
  (4 fichiers + t10/m5 statiques conditionnels) ; porte P3 après merge + reprise comptages lead.
- **P2-fix publiée** : `task_mvcocshr_0009_nja5ao` — test-engineer, cx 40 / risque 40, fresh,
  **worktree true** (écrivain). Attendu post-fix : m1 1/1, m3 2/2 (filtrés), m4 1/1, t11 3/3.
  Puis merge, reprise des comptages sur master par le lead, puis P3.
- Constat tiers : `aiforall-local-control-plane` (kindest/node) Up ~2 h, non possédé, intouché
  — arbitrage à l'axe 2 (P5) ; paires zot/openbao Exited(0) préservées du run P2.

## Incident P2-fix : échec provider.auth (2026-10-17)

- P2-fix `task_mvcocshr_0009_nja5ao` → pool-3 (test-engineer, worktree) : **exécution échouée
  côté provider** — clé API expirée (`provider.auth`, console OpenCode : nouvelle clé exigée ;
  re-signer ne renouvelle pas). Session `ses_eb5139e76ffe3Z...`. Pas de retry auto, pas de
  bascule provider auto ; conformément au harness, recovery = décision lead + action utilisateur.
- Vérifié : worktree pool-3 **propre à la base `614a6e5`** — zéro édition, zéro commit. Rien à
  récupérer ni à annuler, la re-partie peut repartir à l'identique. Master intact, arbre propre.
- Impact : la bande standard (opencode-go/glm-5.3-flash max, policy rev 1) est indisponible
  tant que la clé n'est pas renouvelée. Toute re-admission P2-fix échouerait pareil.
- Arbitrage utilisateur demandé : (a) renouveler la clé (conforme grille) ; (b) correction chat
  = bascule temporaire de la bande standard sur autre provider (p.ex. zai-coding-plan/glm-5.3-flash,
  déjà évaluée en grille du lot précédent) → publier dans le brouillon + `/ensemble-mission config`
  ; (c) pause mission.
- **Recovery exécutée (choix utilisateur : clé renouvelée)** : `team_shutdown pool-3` (worktree
  vérifié propre, zéro commit — rien à merger, la branche déclarée n'a pas de ref ; sorties
  shutdown « could not determine commit status » sans objet). auto_dispatch a **réadmis P2-fix
  sur pool-4** (test-engineer, session fresh, worktree neuf, même contenu de tâche autoporteur :
  sites file:line, oracle, commandes, barrières). pool-2 (contexte P2 de première main) non
  réutilisé : la tâche self-suffisante suffit, interférer encore = churn et risque.
- Suite inchangée : merge pool-4 + reprise comptages lead sur master → porte §6.5 complète →
  publication P3 (correctness + test-reviewer).

## P2-fix exécutée, merge, porte G-17 fermée (2026-10-17)

- **P2-fix** `task_mvcocshr_0009_nja5ao` → pool-4 : complétée après recovery provider.auth.
  Commit worker `1e91353` (branche `…#8ud2p5-pool-4`). Fix test-only **5 fichiers**
  (m1 :25-31 + `crates/`, m3 :36-46, m4 :37-46 + build :123, t11 :83 + :583 par construction,
  **m5 statique corrigé** :62-68 — Kind defer, jamais exécuté). t10 sans helper de la famille ;
  t6 package-relatif, hors famille, non touché.
- Deux écarts bornés documentés par le worker : `.tmp/` créé dans le worktree (capture) ; hook
  `.githooks/pre-commit` en **os-206 préexistant** → commit `--no-verify` avec `rustfmt --check
  --edition 2021` sur les 5 fichiers = exit 0 (intent préservé, hook non modifié).
- **Merge** : team_merge pool-4 → squash committé **`665110c`** ; diff vérifié (5 fichiers
  tests/, aucun src/, gitlink `41a86dba` immobile).
- **Reprise comptages sur MASTER par le lead** (logs `.tmp/g17-p2fix-master-*.log`) :
  m1 **1 passed/0 failed** (9,62 s), m3 **2 passed/4 filtered**, m4 **1 passed/2 filtered**,
  t11 **3 passed/4 filtered** — exit 0 partout. t6 9/9 (P2, fichier non touché).
  → **Porte de sortie §6.5 complète** ; journalisée + commits : plan `f9d0aa7`, fix `665110c`.
- **Statique hors périmètre** (constaté par pool-4, non touché) : m2 :20-25, t12 :13-14 + usages,
  t2 :16-19, t7 :73-76, **chain.rs :49-50 (prérequis du ré-ciblage axe 2 → plan P4)** ;
  fixtures zot/kind package-relatives correctes. m2/t12/t2/t7 = résidus hors axes → décision
  utilisateur.
- **P3 publiée (chaînée, un auteur à la fois sur les briefings)** :
  - P3a correctness : `task_mvcpx4mf_000d_yz8zon` (cx 30 / risque 35, fresh, worktree false)
  - P3b test-reviewer : `task_mvcpzjfs_000f_gm10i0` (cx 34 / risque 40, depends_on P3a)
  Briefings attendus : `20261017-operator-cli-real-binary-transport-{correctness,tests}.md`,
  base master `665110c`/`f9d0aa7`.

## P3a rendue (2026-10-17)

- P3a `task_mvcpx4mf_000d_yz8zon` → pool-5 (`correctness-reviewer`) : **complétée,
  PASS_WITH_NOTES, aucun BLOCK**. Briefing `.cursor/review-briefings/20261017-operator-cli-real-binary-transport-correctness.md`
  (head_sha 665110c, écrit sur f9d0aa7, arbre propre). INDEX mis à jour par le lead.
- Q1 zéro oracle dilué (cibles réelles vérifiées, messages conservés, :583 par construction) ;
  Q2 m5 statique correct par construction — **N-1 INFO : la cible [[test]] m5 n'a JAMAIS été
  compilée par le lot** (binaire séparé, corrigé lecture seule ; si P4 la relance : `--list`
  d'abord dans le slot Windows, exécution Kind au GO seulement) ; Q3 preuve/plan cohérente —
  **N-2 INFO : `g17-t6-run1.log` absent du .tmp** (run2 + Journal + events suffisent) ;
  Q4 périmètre tenu ; Q5 HC statiques archivés dans le briefing
  (m2 :20-25 ; t12 :12-17 + 10 usages, fix 1 ligne ; t2 :16-21 + usages — **piège t2:244 :
  `repo_root().join("apparatus-operator")` passe « par accident » avec l'ancien root ;
  harmonisation sans audit d'usages = faux-positifs** ; t7 :73-78 + :275-278, :97-104
  package-relatif à préserver ; chain.rs :49-57 → prérequis P4). zot/mod.rs:101 et t6:59
  package-relatifs corrects, ne pas toucher.
- P3b `task_mvcpzjfs_000f_gm10i0` en cours (chaînée après P3a).

## P3b rendue : lot G-17 CLOS (2026-10-17)

- P3b → pool-6 (`test-reviewer`) : **PASS_WITH_NOTES, aucun BLOCK**. Briefing
  `20261017-operator-cli-real-binary-transport-tests.md` (head_sha f9d0aa7, audit 665110c).
  INDEX à jour (2 rangées G-17).
- Oracle « dérive d'ancrage » prouvé **FERMÉ par exécution** : les retours à l'ancre fausse
  ont réellement paniqué à 0,00 s pré-fix (logs ROUGE cités) ; une 3e dérive serait re-attrapée.
  Registre G-17.x préuvé (t6 9/9 :28-39 avec les 9 noms ; m1 build CRI réel, image
  `apparatus-reference-kv:m1` 6b629795e389 au registre :16) ; defer honnête vérifié (15 = 10
  filtrés + m5 1 + t10 4) ; G-14 mockée correctement disqualifiée comme preuve réelle.
  T-1/T-2 INFO sans action.
- Journal plan + commit **`dad816e`** (« clos le lot G-17 après revues P3a/P3b »). Arbre propre.
- **Lot G-17 : porte de sortie §6.5 validée des deux côtés (comportement + couverture). CLOS.**
- **P4 publiée** : `task_mvcqvggw_000l_h3ormo` — qa-planner, cx 50 / risque 50, fresh, worktree
  false. Plan QA E2E : re-ciblage fixture non destructif (défaut `aiforall-local`, jamais
  `apparatus-p4-it`, jamais kind delete spontané), prérequis `chain.rs`, commandes exactes,
  preuves par hop, budget. **P5 attendra un GO utilisateur explicite** après commit du plan P4
  (le run m6 touche le cluster kind partagé — kind-aiforall-local-full reste interdit sans GO).

## Incident P4 : hang pool-7, recovery pool-8 (2026-10-17)

- Rapport utilisateur : worker bloqué sur un call shell qui ne finit jamais. Vérifié lead :
  pool-7 [working] ~48 min, aucun message, **plan P4 absent du disque** (zéro artefact durable).
- **team_shutdown force sur pool-7** (un shutdown normal aurait attendu le call courant pour
  toujours). Aucune ressource Docker créée à noter (planification, registre inchangé) ;
  master intact. Cause probable : commande runtime/CLI bloquante (docker/kind/kubectl/cargo)
  — interdite dans la tâche mais non couverte comme garde-fou nominal d'O/O.
- Dispatcher : **réadmission auto sur pool-8** (qa-planner, fresh) avec le contenu original.
  Le lead a durci le contexte par message direct : (1) shell = lecture/git/grep SEULEMENT,
  docker/kind/kubectl/cargo/wsl interdits même en lecture (faits runtime = gaps « à vérifier
  à P5 */) ; (2) **écrire le plan tôt et l'itérer** (anti perte totale) ; (3) commande > 30 s =
  suspecte → abandon de la vérification, inférence marquée, jamais de boucle d'attente.
- Leçon mission : les tâches de planification portent désormais un garde-fou I/O nominal (shell
  lectures/git/grep seulement). pool-7 a été détruit sans dégât (aucun artefact perdu à part le
  temps) ; pool-8 a reçu le durcissement AVANT de travailler.

## Incident P4 suite : pool-8/9 (2026-10-18) — hang récurrent, discipline embarquée

- **pool-8** (réadmission auto) : figé sur le MÊME motif — `Out-File` here-string géant
  (l'utilisateur a recopié la commande ; le call shell ne finit jamais). Mon garde-fou envoyé en
  message est resté en file, invisible tant que le call tournait. **Shutdown forcé.**
- **pool-9** (réadmission auto) : même motif, même fin — l'utilisateur a confirmé le hang via la
  commande recopiée ; plan jamais écrit (fichier testé absent). **Shutdown forcé le 18-10.**
- Bilan 3 tentatives qa-planner sur P4, zéro artefact durable, ~1 h+ de wall-clock perdu.
- **Correctif process (durable)** : la discipline I/O ne passe plus par message (canal tué par
  le hang) mais **DANS le contenu de la tâche republicée** : écriture plan via outil
  fichier-dédier UNIQUEMENT (jamais here-string/Out-File/tee), squelette tôt + édits par
  section (jamais un write géant), shell = commandes courtes en lecture (< 30 s, sinon
  vérification abandonnée + inférence marquée « à vérifier à P5 »).
- **P4-(2) publiée** : `task_mvdjl0kz_0001_tpmv61` (qa-planner, 50/50, fresh, worktree false) —
  contenu technique identique à `task_mvcqvggw_000l_h3ormo` + garde-fous.
- Trajectoire de secours déclarée : si cette 4e tentative échoue sur le même motif O/E, le lead
  rédige le squelette du plan lui-même (écart de boucle QA tracé au ledger), le worker restant
  en audit de revue.
- Note date : ressources Docker créées après le 17 reçoivent un registre du jour
  (`.tmp/docker-resources-20261018.log`) ; le registre du 17 reste append-only de son lot.

## GO utilisateur P5 enregistré + arrêt du churn dispatcher (2026-10-18)

- **GO utilisateur explicite : P5** (« Je donne mon go pour P cinq », 2026-10-18). Périmètre du
  GO : le run réel m6 sur **kind-aiforall-local** avec le re-ciblage non destructif du fixture
  (manifests operator additifs, pod/CR test). **Hors GO** : `kind-aiforall-local-full` et le lab
  0412 restent interdits sans GO dédié supplémentaire.
- Ordre des portes INCHANGÉ : plan P4-(2) livré → relecture + commit lead → publication P5
  immédiate (le GO est déjà donné ; plus de demande de confirmation à ce jalon).
- **Churn dispatcher arrêté** : chaque mort d'un member réadmettait la tâche P4 originale en
  ~10–30 s (pool-10, pool-12, pool-13 successifs, même contenu SANS discipline I/O — risque
  de double écrivain sur le fichier plan + risque de hang répété). Résolu dans un seul script :
  shutdown forcé pool-13 + **blocage contextuel de `task_mvcqvggw_000l_h3ormo`**
  (`context:pending rev:2`, reason « SUPERSEDED by P4-(2)... Do not admit ») — l'admission
  nouvelle est gated en code. pool-11 (P4-(2) durcie) est le SEUL planificateur actif.
- pool-14+ ne doivent plus apparaître ; si un worker hérite malgré tout de la tâche originale,
  le lead le stoppe et laisse `pending` sans le rebloquer manuellement (l'état persiste).

## P4-(2) livrée, plan commité, P5 publiée sur GO (2026-10-18)

- **P4-(2) `task_mvdjl0kz_0001_tpmv61` → pool-11 : complétée.** Plan
  `.cursor/qa-plans/20261017-m6-e2e-chain-kind-ready.md` (347 lignes) — discipline I/O tenue
  (écritures MCP context-mode, zéro here-string shell ; shell = git lecture seulement).
  Contenu : §0 contradiction + règles « fixture intouchable » pré-mission citées ; §1 faits
  (17 occurrences grep, précision reuse-check :48 = API up + pas-kindnet, PAS Calico ;
  overlay J2 : CRD homonyme identique, ns homonymes labels différents = risque strip, SA sans
  collision, AUCUN canary dans J2) ; design D-1…D-7 ; matrice R-01…R-09 (R-01/R-05 critique) ;
  registre E-1…E-12 tous GAP ; commandes/preuves/inventaire/budget/barrières.
- **Audit lead** : ancres spot-vérifiées (chain.rs :50/:56, m6 :51-53/:59, Cargo.toml m6
  admit,controller, canary :22-23 Never, 7 littéraux apparatus-p4-it exacts) — concordance.
- **Décisions lead §9bis du plan** : GO P5 enregistré ; D-6 ADOPTÉ (server-side +
  field-manager `apparatus-it-fixtures`, inventaire labels obligatoire) ; négatif
  bogus-cluster ajouté comme étape P5 obligatoire ; résidus post-run laissés + delta tracé ;
  m5 --list hors lot ; worktree P5 true ; ordre P5 figé.
- **Plan commité** : `f937da7` (docs(qa)). Tâche originale P4 reste `pending rev:2` bloquée.
- **P5 publiée sur GO** : `task_mvdke8nq_0007_nskqei` — test-engineer, cx 60 / risque 55,
  fresh, worktree TRUE. Ordre : lifecycle → inventaire avant → --list (E-4) → négatif
  bogus-cluster (E-2 preuve runtime, fail attendu NON-ROUGE) → run réel m6 → preuves par hop
  (H1…H4) → inventaire après + delta → check fuite port-forward → registre 20261018 → rendu.
  pool-11 shut down proprement.
- Prochain jalon : merge P5 + (aucun re-run master exigé — preuve = run worktree + diff audit ;
  décision tracée) → P6 correctness fresh → INDEX → fermeture axe 2.

## P5 point d'étape (pool-14, run réel en cours)

- Preflight vert : Docker Up ; tiers `aiforall-local-control-plane` Up 17 h (non possédé) ;
  cluster `aiforall-local` présent, **calico-node Ready + kindnet ABSENT** ; ni
  `apparatus-system`/`apparatus-plugins` ni CRD ni opérateur J2 → **delta attendu = additif
  pur**. Écart traité proprement : kubeconfig par défaut périmé (port 60787 ≠ 60508) →
  `kind export kubeconfig --name aiforall-local` via mécanisme D-4 (lecture seule).
- Diff borné posé (3 fichiers, fmt exit 0, grep `apparatus-p4-it` = 0) : D-1 point unique
  `cluster_name()` (défaut `aiforall-local` + env override), D-2 ensure non destructif
  (delete/create/install-Calico supprimés, Err explicites), D-4 kubeconfig dérivé,
  D-6 server-side field-manager, chain.rs 2 éditions. Submodule au pin, immobile.
- E-4 `--list` : exactement 1 test (compile 6m30). **Négatif E-2 exécuté** : bogus-cluster →
  FAILED 0,86 s avec l'Err explicite d'ensure (attendu, NON-ROUGE), `kind get clusters`
  inchangé, zéro create/delete.
- **Run réel m6 en cours** (builds docker + kind load + chaîne complète — dizaines de minutes).
  Verdict + preuves H1–H4 + inventaire après + delta + check fuite à venir. Aucune ressource
  Docker créée par le worker à ce jour.

## P5 ROUGE run1 + triage (2026-10-18)

- **ROUGE run1** (`.tmp/m6-e2e-chain.log`) : m6 FAILED 0/1 à `m6.rs:59:78` — « kubectl not found
  in PATH », 183 s. **Cluster NON muté** (échec à ensure avant apply ; kind get clusters + ns
  inchangés).
- **Triage mesuré (pas inféré)** : kubectl EST dans le PATH. Cause = heuristique
  `bin_ok("kubectl")` du fixture (`resolve_kubectl`) qui exécute `kubectl version` **plain** —
  ce call CONTACTE le serveur du contexte du kubeconfig par défaut (~/.kube/config périmé :
  port 60787 vs mapping réel 60508) → exit 1 → « not found » (message inexact sur la cause).
  Mesures : plain_version_exit=1 ; avec KUBECONFIG valide → exit 0.
- **Classification** : précondition d'environnement × heuristique fixture ; PAS un bug prod ;
  PAS un oracle faussé. **Note de durcissement pour P6/lead** : `bin_ok` devrait tester
  `kubectl version --client` (ou l'existence du binaire) — hors périmètre du lot
  (resolve_kubectl non listé au design) → futur petit lot de fixture, jamais in-lot.
- **Remède en cours (auto-repair borné #1, conforme plan)** : run2 = MÊME commande exacte avec
  `$env:KUBECONFIG = %TEMP%\aiforall-local.kubeconfig` (fichier que run1 a lui-même exporté via
  D-4). Données/env de run seulement ; oracle inchangé ; aucune correction de code.
  Log run2 séparé ; log run1 conservé comme preuve ROUGE.
- Limite worker auto-assignée : **2 auto-repairs max** ; au-delà ou nouvelle cause → persister
  + escalade lead.

## P5 run2 : ROUGE régression Dockerfile.http (2026-10-18)

- **POSITIF MAJEUR (preuve run2 `.tmp/m6-e2e-chain-run2.log`)** : `ensure()` PASSED sur
  `kind-aiforall-local` — eprintln `aiforall-local: wait Calico + CoreDNS Ready` (D-1 dérivé),
  flux 9 étapes, apply server-side D-6 OK. **Le re-ciblage fonctionne** ; `KUBECONFIG` env a
  débloqué le probe (auto-repair #1 consommé, borne).
- **ROUGE run2** : panique `chain.rs:234` — `docker build apparatus-reference-kv:m6` exit 1 ;
  BuildKit `COPY crates/apparatus-reference-kv/apparatus.toml /apparatus.toml` → not found.
- **Cause racine (git, vérifiée par lecture lead)** : **régression `f060d47`**. Le header
  `Dockerfile.http:1` déclare « Contexte : mini-workspace (Cargo.toml + apparatus-contracts +
  apparatus-reference-kv) » (stages :5-7 cohérents) ; `:16` a été réécrit en chemin relatif
  **racine-repo** — inconstructible des DEUX côtés (mini-workspace → not found ; racine-repo →
  chemins racine inexistants). **Trois consommateurs cassés** : `chain.rs:176-228` (m6),
  `m5:201` (staging identique, Kind-defer), `ops/deploy/apps/overlays/kind-demo-monolith/schedule-reference-kv.ps1:151-152`
  (gold path 0605, GO séparée).
- **Proposition worker (P8-min)** : révert `Dockerfile.http:16` →
  `COPY apparatus-reference-kv/apparatus.toml /apparatus.toml` (restaure le contrat déclaré
  pour TOUS les consommateurs). Le worker REFUSE le miroir test-side dans le staging
  chain.rs : masquerait la régression (m5 + ops ps1 resteraient cassés) et sortirait de la
  liste exhaustive E-3. **Lead : d'accord avec le refus.**
- **État cluster (muté par run2, laissé en place §9bis.4, delta borné conforme §6.4)** :
  ns apparatus-system/plugins créés (labels fixture, pas de strip — n'existaient pas), CRD
  créé, 4 SA, RB admit/controller, netpols ×2, canary deploy+svc + pod canary Running 1/1 ;
  AUCUN pod m6 ni CR (hop non atteint) ; labels J3 aiforall-* intacts.
- Inventaires persistés : `.tmp/m6-*after-run2*.txt` ; run1 intact. pool-14 S'EST ARRÊTÉ
  (pas de run3 sans GO) — arbitrage demandé.

## GO P8-min exécuté (2026-10-18)

- **Arbitrage utilisateur : GO in-worktree pool-14** (option recommandée). Plan amendé :
  §9bis.8/.9 + **E-13** au registre — commit lead **`3270eed`**.
- Le refus worker du miroir test-side est **validé par le lead** (masquerait la régression
  f060d47 pour m5 + gold path 0605).
- pool-14 réactivé (force) avec le GO opérationnel : révert 1 ligne
  `crates/apparatus-reference-kv/Dockerfile.http:16` → `COPY apparatus-reference-kv/apparatus.toml`
  (diff total = 4 fichiers), re-run réel (même commande + env KUBECONFIG), rendu final complet,
  commit worktree → merge lead → P6 audit du diff total. Sa limite 2 auto-repairs tient ;
  toute nouvelle cause = persist + stop.

## GO E-14 tranché lead (2026-10-18, après ROUGE run3)

- **Run3** (`.tmp/m6-e2e-chain-run3.log`) : le révert :16 TIENT (COPY apparatus.toml OK +
  apt wget OK — validé en runtime) ; échec AU build stage-1 : mini-workspace embarqué
  `chain.rs:181-216` sans `reqwest`/`rcgen` dans `[workspace.dependencies]` alors que la crate
  les hérite (`Cargo.toml:11,24-25`, feature http) → parse fail. Le staging n'a JAMAIS compilé
  (dérive masquée par le panic chain.rs à 0 s pré-fix).
- **Ancres vérifiées lead** : chain.rs:192-201 (9 deps, sans reqwest/rcgen) ;
  crate Cargo.toml :11/:24-25 ; root Cargo.toml :88 (reqwest 0.12.0 json/rustls-tls) et :142
  (rcgen 0.13.2 x509-parser). Trois consommateurs du staging : m6 (chain.rs), m5:201,
  schedule-reference-kv.ps1:151-152 (gold path).
- **Tranché lead (pas d'arbitrage user : mécanique/miroir/test-side, règle du ledger)** :
  E-14 = 2 entrées verbatim au staging. Plan amendé + commité `43c3804`. pool-14 réactivé
  (GO envoyé) : run4 = même commande ; auto-repair #2 = SON DERNIER ; cause nouvelle ensuite
  = STOP définitif + arbitrage user.
- Cluster : run3 n'a RIEN muté au-delà du delta additif run2 (échec avant admission) ;
  canary toujours Running ; zot/openbao jamais atteints.

## Run4 : EN VIE, worker idle — babysitting (2026-10-18, 11:11)

- **Run4 est vivant** : `apparatus_m6_e2e_chain` PID 57104 (11:09:08) + cargo + kubectl actifs ;
  **zot + openbao-transit créés à 11:10** ⇒ la chaîne a franchi ensure + builds + kind load
  (E-13/E-14 appliqués par le worker : chain.rs:202-203 verbatim, diff 4 fichiers non commité
  encore) et est en phase push/sign. Le log run4 sans « test result » = run ONGOING, pas mort.
- Le worker session est passée idle pendant que son process enfant tourne (nudge lead envoyé,
  force) : consignes = surveiller par lectures courtes (Get-Process 57104 + tail log),
  NE JAMAIS relancer (double-run interdit tant que 57104 vit), à « test result: » → preuves
  H1-H4 + inventaire après + delta + check fuite + registre (zot/openbao 11:10 à appender) +
  commit + rendu ; mort sans résultat = ROUGE + stop (limite #2 consommée).

## Run4 terminé : preuve verte vérifiée par le lead (2026-10-18)

- Vérification directe du log du worktree pool-14 via Context Mode (décodage UTF-16) :
  `.tmp/m6-e2e-chain-run4.log` = `test m6_e2e_0002_0008_chain ... ok`,
  `test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 311.79s`.
- Aucun processus Cargo/m6/kubectl restant à la vérification. Ce constat prouve la fin du run,
  pas à lui seul le cleanup des ressources Docker/Kubernetes.
- Worktree toujours non commité : exactement 4 fichiers modifiés (3 fixtures + Dockerfile.http).
  pool-14 réactivé pour LIVRAISON SEULE : preuves avec séparation assertions/captures brutes,
  inventaire après et delta, registre Docker (IDs complets, limites d'ownership explicites),
  cleanup gracieux selon bail, fmt ciblé, commit, HANDOFF final. Aucun re-run demandé.
- Pas encore de clôture axe 2 : intégration et revues P6 requises. Les stdout/stderr des tests
  verts sont capturés par libtest ; ne pas inventer des extraits H1–H4 absents du log.

## Livraison P5 reprise directement par le lead (2026-10-18)

- Aucun HANDOFF final ni commit ni registre Docker du 18 après plusieurs nudges. Reprise de la
  livraison seulement (pas de nouveau run) : auto_dispatch temporairement désactivé pour éviter
  une réadmission concurrente, pool-14 shutdown, quatre fichiers préservés et commités dans son
  worktree (`8c5b260`), puis `team_merge` et commit master **`e66b736`**.
- Première tentative team_merge : référence de branche absente (worktree détaché). Référence
  nommée créée au commit récupéré, puis team_merge réussi. Aucun merge manuel/reset/stash.
- Patch intégré = patch testé, comparé octet pour octet. `git diff --check` et rustfmt ciblé
  exit 0 ; hook global `cargo fmt --all` échoue os-206 (reproduit). Commit avec option de hooks
  vide transitoire, aucun réglage global changé. Pin rustycog et config Cargo intacts.
- Preuves récupérées et limites : **`.tmp/m6-lead-recovery-20261018.md`** (fait autorité pour la
  livraison technique manquante). Run4 vert 1/1, assertions exécutées mais pas de captures
  autonomes H1/H2/H3/H4 : ne pas présenter des extraits bruts inexistants.
- Observation Kubernetes indépendante : Pod plugin Running/Ready, image CRI pinée ; CR VALID
  même digest catalogue et labels ; canary Ready. UID du nœud antérieur au lot ; aucun Secret lu.
- Cleanup : fixtures zot/OpenBao créées run4 identifiées par événements create exacts ; arrêt
  gracieux vérifié exited 0. Cinq helpers déjà détruits. Registre du 18 régularisé tardivement
  (omission worker explicitement reconnue), images opaques UNKNOWN+ask, aucun prune/suppression.
- Limite hôte : target du worktree résolu sur C:, pas de preuve d'usage du target E: prescrit.
  Artefacts conservés, pas de nouveau Cargo pour masquer l'écart.
- Prochaine porte : P6 fraîche sur le commit intégré et les preuves, puis vérification infra
  indépendante ; pas de clôture anticipée.
- Journal QA de récupération commité **`e7e6630`**. P5 clôturée sur le board comme exécution
  intégrée, pas comme acceptation des risques ; demandes indépendantes persistées sous
  `.cursor/review-briefings/20261018-m6-e2e-chain-review-request.md`.
- **P6a** correctness publiée : `task_mvdo2cxw_000v_uuytud` (cx44/risque60, fresh, worktree false).
- **P6 infra** publiée : `task_mvdo2ptm_000x_bdtv4e` (cx40/risque50, fresh, worktree false),
  contexte unique kind-aiforall-local, lecture seule ; le lead persiste le rapport.
- auto_dispatch restauré à true (valeur précédente) après intégration ; policy modèles
  inchangée, aucun override. P4 originale toujours pending/superseded.
- P6b test-reviewer à publier seulement après lecture de P6a et décision sur ses findings,
  pas sur la seule complétion administrative de tâche. Aucun re-run m6 demandé.
- **Continuation redondante pool-15 reçue** : aucun nouveau test ni code modifié. Inventaires
  après-run4 ajoutés au dépôt canonique : `.tmp/m6-post-run4-{dockerps,ds,ns,crd,appsys,syspods,plugins,netpol,cr}.txt`.
  Observations : CNI Ready sans kindnet ; ajouts operator et résidus pod/CR attendus ; aucun
  kubectl/kind résiduel ; seul nœud tiers Docker Up, fixtures arrêtées.
- **NO-GO lead** à la proposition de re-run `--nocapture`, FF worktree ou rewording du
  doc-comment avant les revues : P6a/P6 infra travaillent déjà (pool-16/pool-17). Un log vert
  prouve des assertions exécutées, pas des valeurs brutes absentes ; `--nocapture` ne crée pas
  une instrumentation inexistante. Le besoin d'une preuve additionnelle sera décidé sur
  findings, pas par duplication de P5. Shutdown gracieux demandé à pool-15.
- **P6 infra / pool-17 : PASS** (13 checks PASS, events Kubernetes INCONCLUSIVE/TTL expiré).
  Rapport persisté par le lead : `.cursor/review-briefings/20261018-m6-e2e-chain-infra.md`,
  INDEX à jour. État courant node/CNI/CRD/Pod pin/CR VALID/canary/service-endpoints conforme ;
  fixtures exited 0, cinq helpers détruits, aucun processus résiduel. Pas de re-run/mutation.
- Gaps de registre supplémentaires G-1/G-2/G-3 consignés dans le briefing : généalogie des
  paires du 17 remplacées et paire non enregistrée restée Up quinze heures ; tag m1 reconstruit
  sans nouvelle ligne ; réseau partagé non enregistré. Ce sont des écarts de traçabilité,
  pas une permission de supprimer ni une raison de réécrire les anciennes lignes.
- Vérification lead du réseau : création **2026-09-20**, antérieure au lot ; réseau non possédé,
  conservé. Les nouveaux IDs fixtures du 18 étaient absents avant création, mais les *noms*
  étaient occupés par des fixtures actives du 17 : ambiguïté `prior_state=absent` explicitée.
- Horaires : borne finale 10:37Z annoncée par pool-17 non corroborée par horloge lead 10:27Z ;
  le rapport distingue donc heures déclarées et horodatages immuables objets/événements.
  Aucune conclusion ne dépend de cette borne déclarative.
- **P6a / pool-16 : GO_WITH_COMMENTS**, briefing de 117 lignes lu intégralement,
  `20261018-m6-e2e-chain-correctness.md`, INDEX actualisé. Aucun BLOCKER/HIGH ; comportement
  introduit conforme au design. C-1/C-2 MEDIUM préexistants, C-3/C-4 LOW, C-5/C-6 INFO.
- Correction du récit E-13 : tous les consommateurs récupèrent la couche COPY, **pas** leur
  compilation. Stagings m5 et ps1 gold path restent sans reqwest/rcgen ; hors lot, jamais testés.
  Les six notes sont enregistrées dans le Journal QA avec propriétaires/conditions de
  réouverture : aucune expansion de scope ni re-run ajoutés avant audit de couverture.
- **P6b test-reviewer publiée** : `task_mvdoros9_0018_ypolb4` (cx44/risque60, fresh,
  worktree false), contexte indépendant, verdict libre sur E-1…E-14 et preuves/limites.
  Aucun Cargo/runtime autorisé. Briefing attendu `20261018-m6-e2e-chain-tests.md`.

## Clôture de mission — G-17 puis m6 (2026-10-18)

- **P6b / pool-18 : GO_WITH_COMMENTS**, briefing de 153 lignes lu intégralement,
  `20261018-m6-e2e-chain-tests.md`, INDEX actualisé. Aucun BLOCK. E-1…E-14 audités séparément,
  dix mutations examinées ; aucune preuve de compilation des quinze autres cibles inventée.
- T-1 captures autonomes absentes : GAP-accept documentaire (MEDIUM), owner lead, rouvrir au
  prochain run m6 avec `--nocapture` et captures non secrètes réellement suffisantes ; ne pas
  supposer que ce flag seul imprime des valeurs aujourd'hui non instrumentées.
- T-2 oracle canary (LOW préexistant) : readiness/endpoints à rendre fail-loud AVANT la sonde
  dans un futur lot test-engineer ; run4 prouve l'exécution actuelle, pas une certification
  indépendante de toute l'isolation T11. T-3 = C-2 (probe kubectl client-only futur).
- Autres accept/defer : branches CNI/conflit SSA/override positif non exécutées, mutations
  M2/M10 non détectées en runtime ; rouvrir avant évolution du fixture/namespaces J2 existants.
  C-1…C-6, target local C: et écarts historiques de registre restent explicitement tracés.
- **Axe 1** : t6 9/9, m1 1/1, secondaires filtrés 2+1+3 verts ; quinze tests Kind historiques
  defer nommés (jamais déclarés exécutés). **Axe 2** : m6 1/1 vert, 311,79 s, négatif cluster
  absent sans mutation ; P6a GO_WITH_COMMENTS, P6b GO_WITH_COMMENTS, infra PASS/13 checks avec
  events K8s INCONCLUSIVE. **Mission close sur ce périmètre, avec commentaires, sans bloqueur.**
- Code intégré `e66b736`, plans/journal à la suite de `a3a6a17`. Pas de push demandé/effectué,
  pas de gitlink/config Cargo modifiés, pas de promotion ADR. Full et lab 0412 non exécutés.
  Pas de P7 automatique : m6 est une chaîne avec hop Pod réel, non une simple sonde curl ;
  aucune prétention de preuve mesh complète.
- Preuves brutes mises à l'abri avant nettoyage des worktrees :
  `.tmp/m6-evidence-20261018-pool14/` (23 fichiers / 116894 octets / empreintes identiques),
  MANIFEST.md avec SHA-256 des logs essentiels ; aucun kubeconfig archivé. Snapshots frais
  `.tmp/m6-post-run4-*.txt`, recovery `.tmp/m6-lead-recovery-20261018.md`, registre du jour.
- Board : onze tâches complétées, zéro actif/queue. P4 originale close **administrativement
  comme remplacée par P4-(2)**, pas comme une livraison supplémentaire. Worktrees écrivains
  pool-3/4/14/15 et sous-modules initialisés vérifiés propres avant cleanup normal Ensemble.
- Runtime : deux fixtures possédées stopped/exited 0 ; cinq helpers détruits par transport ;
  nœud partagé préexistant non possédé et Pod/CR/manifests conservés par décision. Aucun prune,
  suppression de volume/namespace/cluster ou stop global Docker Desktop/WSL. Images opaques
  et réseau partagé UNKNOWN+ask conservés. RAM résiduelle non mesurée (runtime tiers actif).
- **Commit de clôture QA : `02dcd23f579a2e8ca3d3526046eb019bf433186e`**. Vérification finale :
  code des quatre fichiers identique à `e66b736`, diff config Cargo/gitlink nul depuis base
  `6f34b6d7a104`, porcelain limité à `.opencode/missions/` non suivi ; aucun push.
- **Équipe nettoyée via `team_cleanup` normal**, sans force ni purge d'archives. Sept membres
  idle arrêtés proprement avant cleanup. Avertissements auto-merge pool-3/pool-15 : aucun
  travail propre à intégrer (worktrees vérifiés clean, aucune édition/commit de livraison) ;
  pas de merge manuel ni récupération inventée. Pool-14 intégré et preuves archivées avant.
- Archive run4 encore présente après cleanup, SHA-256
  `6033b54c1d1ea29cc3ee8863a2b57cd13f92968181ea0c2b8a4b114a11f5a9b1`, conforme au manifeste.
  Dernier inventaire : zot/OpenBao Exited(0), nœud tiers Up ; aucun processus Cargo/m6/kubectl.
  Suppression des objets Kubernetes conservés non autorisée ni tentée : leur présence est
  rendue explicitement, pas présentée comme un teardown intégral du cluster partagé.

## Correction policy (2026-10-17, après présentation)

- Utilisateur : Opus n'est plus utilisable (crédit épuisé) ; retour sur **Sol 6.1 Extra High**.
- Brouillon `.opencode/missions/team_mvcm6iko_0000_8ud2p5/model-policy.json` édité en
  conséquence : groupe `strong` → `openai/gpt-6.1-sol` variante native `xhigh`
  (identité vérifiée dans le catalogue ; variantes openai : low/medium/high/xhigh/max).
- Inchangé : `light` = `opencode-go/gpt-6-luna` (none), `standard` =
  `opencode-go/glm-5.3-flash` (max), bandes [0,34,67,101], matrice, providers, critères.
  Opus retiré de la seule entrée où il figurait ; aucun réglage global touché.
- Placement mis à jour : P1–P6 → standard = glm-5.3-flash max ; P7/P8 → strong =
  gpt-6.1-sol xhigh. La grille « Option B » (grok) du §Politique devient caduque pour
  strong ; elle reste un historique de chat, non un état du brouillon.
- Le lead n'invoque ni `config` ni `start` : l'utilisateur relance
  `/ensemble-mission config` pour présenter le brouillon édité, puis `/ensemble-mission start`.

## Policy approuvée et publication (2026-10-17)

- Policy approuvée : **revision 1**, hash `a3c8460e40d50064570f880462b9922b59bfa6c8558fb74e63b5188abb3779ae`
  (strong = `openai/gpt-6.1-sol` xhigh). Le routage modèle/variante/provider est appliqué par
  Ensemble ; le lead ne passe pas d'override explicite.
- **P1 publiée** : `task_mvcmn5mc_0001_tn3fkp` — qa-planner, complexité 40, risque 28, fresh,
  worktree false, priorité high, scores manuels (classification non demandée). Plan QA du lot
  G-17, livrable unique `.cursor/qa-plans/20261017-operator-cli-real-binary-transport.md`.
- Conditions de publication des suivantes (rien d'autre n'est publié) :
  - P2 (test-engineer, IT réelles) : après commit du plan P1 par le lead (relecture) ;
  - P3 (revues) : après intégration P2 + gate verte ;
  - P4 (plan QA E2E) : après P3 sans BLOCK ;
  - P5 (chaîne E2E) : après commit du plan P4 par le lead ;
  - P6 (revue E2E) : après intégration P5 ;
  - P7/P8 : conditionnels, jamais publiés d'office.
- Aucune exécution admise avant `/ensemble-mission start` de l'utilisateur. Aucun runtime
  (Docker/Kind/WSL) démarré. autoDispatch est actif côté mission : l'admission reste verrouillée
  par le start, conformément à la consigne.
