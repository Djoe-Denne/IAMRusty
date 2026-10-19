# Résultats — Fiabiliser le cycle de vie des fixtures testcontainers (2026-10-19)

Mission : `docs/missions/20261019-it-fixtures-lifecycle.md` (ordre) —
**terminée et prouvée**. Policy approuvée rev 1
(`.opencode/missions/team_mvf5gxng_002m_dwio4v/model-policy.json`, hash
`404d48f8…`). Ledger complet de coordination :
`.cursor/handoffs/20261019-it-fixtures-lifecycle.md` (33 décisions). Plan QA :
`.cursor/qa-plans/20261019-it-fixtures-lifecycle.md` (committé `7a895eb`).

## Résultat attendu (ordre §4) — tenu et prouvé

| Objectif | Preuve |
|---|---|
| 1. Zéro container possédé running en fin de campagne | Campagne G-13 (`it-8979fe482d…`) : **74 créés / 74 retirés**, `owned-running=0`, `new-unowned=0` ; nœud Kind `aiforall-local-control-plane` intact (before=after) |
| 2. Pic borné et reproductible ; flakes rares/absents | **`peak_running_total=4`** (contre 35 simultanés / 97 accumulés avant mission) ; **0 flake** sur 74 créations (contre 2 `unknown_opaque_start_failure` + 1 Lazaret avant) |
| 3. Retrait des `Exited(0)` possédés en fin de campagne | 74/74 rm sans force/volumes, état inspect-vérifié `exited|0`, jamais par nom |
| 4. Ledgers et registre append-only exploitables | 296 lignes de registre cohérentes ; mutex nommé `Global\AIForAll-DockerResources-Append` ; chaîne de propriété = ledger frais + témoin Engine, jamais nom/image |
| 5. Suites vertes aux mêmes oracles | 56 binaires de test : **0 failed, 0 ignored** ; cible pure `fixture_runtime` **17/17** ; baseline G-9 : bloc `safety_tests` octet-à-octet identique hors CR |

## Livraisons (HEAD canonique `70e89eb`, 11 commits locaux)

- **Q0** `7a895eb` — plan QA (matrice 14 risques, gaps G-1..G-13, critères E/S).
- **F1** `716c4eb` + `0b5982c` + `1126181` — lanceur `ops/tests/it-native.ps1` :
  témoin Engine (`docker events`, curseur sur l'heure du daemon), propriété par
  chaîne ledger frais + événement create (jamais nom/image/diff d'inventaire),
  stop gracieux `--time -1` (jamais SIGKILL conteneur, borne CLI 30 s), retrait
  fail-closed des `Exited(0)` possédés, inventaires full-ID `--no-trunc`,
  registre append-only ; tolérance stderr natif sous hôte redirigé (pitfall
  PS 5.1 `ErrorRecord`+Stop) ; mutex nommé sur les appends registre (interleaving
  réel prouvé et corrigé : ligne tronquée à 490 chars → hold fail-closed parasite).
- **F2** `ee8ee48` (câblage minimal des singletons nommés Lazaret redis/openbao,
  Telegraph smtp) + bump `ad089f7` → SDK `2f78d54` ; round 2 → SDK `6c63b65` +
  bump `70e89eb`. SDK (branche `fix/it-fixtures-lifecycle` de
  `Djoe-Denne/rustycog`, **publiée**) : guard exit-process
  (`fixture_runtime/process_exit.rs`, thread/runtime Tokio dédié créé AVANT
  création, stop vérifié exact-ID sans rm, 15 s/ID / 20 s global),
  `start_service_testcontainer` (préarme le guard avant polling, délègue à
  `Attempt::start`), réexport `testing::testcontainers` (unification 0.23.3/0.24
  sans changement Cargo.toml/lock), retrait des handlers no-op
  (database/sqs/kafka/openfga + wiremock T-4 y compris le `ctrlc exit(0)` qui
  bypassait le guard).
- **F3** `8c9989b` — cap libtest 2 par binaire (`RUST_TEST_THREADS`), override
  `AIFORALL_IT_TEST_THREADS` (entier strictement positif, invalidé = STOP avant
  tout effet), restauration `finally` tous chemins, README
  `ops/deploy/README.md`, test hermétique `ops/tests/test-it-native-threads.ps1`.
- **F4** `d93e125` — règles (`.cursor/rules/docker-resource-registry.mdc`,
  `AGENTS.md`) et wiki
  (`obsidian/AI FOR ALL/projects/aiforall/concepts/testcontainers-resource-churn.md`)
  réconciliés avec les résultats réels.
- **T-1** `d2a7769` — tests durables hermétiques du lanceur
  (`ops/tests/support/it-native-fake.ps1` + `test-it-native-stop.ps1` +
  `test-it-native-stop-failclosed.ps1`) : G-1..G-5 + branches fail-closed
  (T-3b/c/d), rejonction rejetée (T-5), override de bout en bout (T-6).

## Preuve frontière Windows (contrat documenté)

`fixture_process_exit_it` (`it-exit7-aa991102`, 2/2) : `std::process::exit(7)`
bypass le CRT → `libc::atexit` ne tire pas, conteneur réel resté running
constaté par Engine, puis stop exact-ID par le parent dans son propre finally
(`process_exit_it_parent_stopped`). Contrat en couches : guard in-process pour
les sorties normales (le cas du churn historique) ; **fallback obligatoire F1**
pour exit nonzero/abort/crash. La supervision out-of-process a été rejetée
(hors périmètre, ordre §7).

## Reviews indépendantes

- **correctness : PASS** (0 P0/P1/P2) — briefing
  `.cursor/review-briefings/20261019T1420Z-correctness-it-fixtures-lifecycle-2f78d54.md`.
- **tests : GO_WITH_COMMENTS** — briefing
  `.cursor/review-briefings/20261019T1257Z-tests-it-fixtures-lifecycle-1126181.md`.
  Tous les findings traités : T-1 (durables), T-2 (composition guard×join
  `unresolved` testée), T-3/T-5/T-6/T-7 (couverts), T-4 (no-op + ctrlc
  supprimés), T-8 (réconciliation faite).

## Garantie anti-churn (état post-mission)

Un `it-native.ps1 -Suite <p>…` : cap 2 par binaire, zéro container possédé
running à la fin (succès OU échec), `Exited(0)` possédés retirés, rapport
d'inventaire — pic constaté : 4. Un `cargo test -p <pkg> --tests` direct : le
guard stoppe les conteneurs possédés à la sortie normale du binaire (zéro
running), mais le retrait des `Exited(0)` reste au preflight/lanceur — c'est
volontaire (le rm est un droit du lanceur, pas du guard). Les sorties abruptes
(kill/exit(7) direct) restent couvertes par le finally du lanceur uniquement.

## Défauts préexistants hors mission (à traiter séparément)

1. Tests JWT de `rustycog-framework` non compilants (`jwks_seed_tests.rs:368`
   import manquant ; `jwks.rs:693` `Instant::saturating_sub` inexistant) —
   propriétaire rustycog.
2. Hook pre-commit `cargo fmt --all` : os error 206 environnemental Windows.
3. 6 warnings clippy préexistants du framework (hors F2).

## Hors périmètre (inchangé)

Prod/cloud/HA, suppression de l'herméticité, migration d'infrastructure au-delà
de `rustycog-testing`, décision 2026-10-07 (IT natives), extinctions de
runtimes partagés, réparation du harness OpenCode.
