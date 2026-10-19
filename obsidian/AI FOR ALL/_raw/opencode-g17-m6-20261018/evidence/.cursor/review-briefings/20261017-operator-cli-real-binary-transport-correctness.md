# Review briefing — correctness transport binaire réel G-17 + correctif P2-fix (665110c)

```yaml
reviewer: correctness
written_at: 2026-10-17T20:55Z
branch: master
head_sha: 665110c # commit audité ; HEAD au moment de la revue = f9d0aa7 (fermeture plan, parent direct 665110c)
dirty: false # hors .opencode/missions/ (mission non commitée, hors lot)
scope_slug: operator-cli-real-binary-transport
verdict: PASS_WITH_NOTES # aucun BLOCK ; 2 notes INFO, 1 archive HC périmètre P4
related_paths:
  - workers/apparatus-operator/tests/apparatus_m1_reference_kv_pin.rs
  - workers/apparatus-operator/tests/apparatus_m3_install_fail_closed.rs
  - workers/apparatus-operator/tests/apparatus_m4_desired_state_bridge.rs
  - workers/apparatus-operator/tests/apparatus_m5_invoke_isolated.rs
  - workers/apparatus-operator/tests/apparatus_p4_t11_runtime.rs
```

## Verdict

Le diff `665110c` est une harmonisation purement mécanique des helpers `repo_root()`
post-`f060d47` (5 fichiers `tests/`, aucun `src/`, gitlink `41a86dba` immobile vérifié) :
aucun oracle n'est dilué, aucune assertion affaiblie, aucune cible tautologique. Les trois
fichiers exécutés passent sur master (m1 1/1, m3 2/4 filtrés, m4 1/2 filtrés, t11 3/4
filtrés), la preuve transport de référence t6 9/9 est intacte (fichier non touché), le mode
fallback docker servi (G-17.7) et zéro résidu helper (G-17.9) sont confirmés par le registre
et les events Docker, et les 15 tests Kind restent non exécutés (G-17.10 nominatif). La
variante statique m5 (jamais exécutée, Kind defer) est correcte par construction : texte
identique aux trois frères compilés et passés, usages résolus vers des répertoires réels.
Deux notes INFO (aucune action requise dans ce lot) + archive HC statiques hors périmètre
pour le plan P4 (axe 2).

## Findings

### N-1 — INFO (correctness read-only de la variante statique m5, jamais exécutée)

- **Location:** `workers/apparatus-operator/tests/apparatus_m5_invoke_isolated.rs:62-70`,
  usages `:73,:81,:196,:199`
- **Fait:** le fix m5 (`:62-65` helper, `:69` `reference_kv_dir()`) est texte-identique à
  celui des trois frères m1/m3/m4 compilés et passés sur master. Ses quatre usages sont
  préexistantes ET ortographiées depuis la racine workspace (`crates/apparatus-contracts`
  `:196`, `reference_kv_dir()` `:69,:199`, `apparatus.toml` `:73,:81`) : pré-image vérifiée
  (`git show 665110c^`, usages inchangés). Toutes les cibles existent réellement :
  `crates\apparatus-reference-kv\apparatus.toml` (Test-Path True), `crates\apparatus-contracts`
  (True), `Dockerfile.http` présent dans le crate référence. Aucun absolu codé en dur, pas de
  tautologie CARGO_MANIFEST_DIR.
- **Borne:** la cible `[[test]]` m5 (required-features `admit,controller`) n'a été compilée
  par AUCUN run du lot (les quatre cibles master = m1/m3/m4/t11, chaque `[[test]]` est un
  binaire séparé). La correction de m5 est donc correcte par lecture seule ET par équivalence
  texte des frères, mais sans preuve de compilation — acceptable (auxiliary Kind defer,
  aucun run prévu P4). Si un jour m5 doit être relancé : compile d'abord (`--list`) dans le
  slot Windows, puis exécution via Kind au GO utilisateur seulement.
- **Cannot verify:** absolument rien d'autre — le diff m5 est purement le helper.

### N-2 — INFO (rétention d'évidence, sans impact porte)

- **Location:** `.tmp\g17-t6-run2.log` (unique preuve t6 persistée)
- **Fait:** le ledger §P2 mentionne « t6 9/9 (run1 + run2 capture brute) » mais un seul
  fichier t6 existe dans `.tmp` (run2, 9/9 @23.03s). Aucun `g17-t6-run1.log` (30+ extensions
  `.tmp` scannées, `.tmp/` complet à 292 fichiers).
- **Why sans impact:** le plan §9.7 ne réclame aucun re-run t6 ; les preuves de hops
  (G-17.1→G-17.6) sont apportées par run2 + artefacts zot consignés au Journal du plan
  (digest `sha256:b707d331…`, 4 tags `.sig` HTTP 200, verify positif/négatif) + events
  Docker. Le delta mode fallback (pulls 17:36:00Z oras:e2/cosign pics registre
  `.tmp\docker-resources-20261017.log` premières lignes) est persisté.
- **Suggested fix shape:** aucun. Si le lead veut la redondance run1, il la trouva dans le
  Journal du plan et le registre (noté une fois ici, ne pas « réparer »).

### Archive hors périmètre — HC statiques de la même famille (prérequis plan P4, axe 2)

Aucune correction demandée dans ce lot (décision lead = résidus hors axes ; QC statique
méthodique). Constat ROUGE si relancés, précisé pour le plan P4 :

| Site (head `665110c`) | Cassé parce-que | Mode d'échec si exécuté |
|---|---|---|
| `tests/apparatus_m2_valid_persist.rs:20-25` | helper `join("..")` (= `workers/`), cible `workers/apparatus-reference-kv/apparatus.toml` inexistante | panic `manifeste disque crates/apparatus-reference-kv/apparatus.toml: …` à 0,00s (message déjà apologétique famille m1) |
| `tests/apparatus_p4_t12_gate_regression.rs:12-17` + usages `:52,:63,:70,:79,:90,:178,:209,:380,:407,:421,:442` | `workspace_root()` = `CARGO_MANIFEST_DIR/..` canonisé = `workers/` | `collect_rs` → `read_dir` panic `dossier prod lisible` (:20) ; `expect("ports.rs/Cargo.toml/HTTP lib")` sur `workers/crates/…`, `workers/services/Manifesto/…` inexistants. TOUTES les orthographies d'usage sont workspace-rooted → fix = même mécanique 1 ligne que t11 |
| `tests/apparatus_p4_t2_scaffold.rs:16-21` + usages `:119,:139-143,:151-154,:196-205,:222,:287-291` | helper `.parent()` = `workers/` | `expect("Cargo.toml racine")` sur `workers/Cargo.toml` (absent) ; `root.join("workers")…is_dir()` false → assert panic ; `workers/services/Manifesto`, `workers/workers/apparatus-operator/…` inexistants (Test-Path False vérifié sur workers/Cargo.toml, workers/services, workers/crates, workers/runtime, workers/Factory) |
| `tests/apparatus_p4_t7_admission.rs:73-78` + usage `:275-278` | helper `.parent()` = `workers/` | `t7_manifesto_src_has_no_admission_store` : `read_dir`/collect sur `workers/services/Manifesto` inexistante → panic. ATTENTION : `:97-104` `operator_admission_src()` est package-relatif (`CARGO_MANIFEST_DIR/src/admission.rs`) — correct indépendamment, à préserver tel quel ; un `:244`-analogique accidentel n'existe PAS ici (le seul usage repo_root = :275) |
| `tests/fixtures/chain.rs:49-51` (+ `:55-57`) | `repo_root()` public = `join("..")` = `workers/`, `reference_kv_dir()` = `workers/apparatus-reference-kv` | consommé UNIQUEMENT par `apparatus_m6_e2e_chain.rs:48` (axe 2) ; `descriptor_from_disk` panic `apparatus.toml: …` au premier appel |
| `tests/fixtures/zot/mod.rs:101`, `apparatus_p4_t6_sign_registry.rs:59` | CORRECTS (package-relatifs `CARGO_MANIFEST_DIR/tests/fixtures/zot`, `CARGO_MANIFEST_DIR/{relative}`) | à ne PAS toucher lors d'une harmonisation P4 |
| `apparatus_p4_t2_scaffold.rs:244` (`repo_root().join("apparatus-operator").join("Cargo.toml")`) | résoud SURPRISE correctement = `workers/apparatus-operator/Cargo.toml` existe (parent = workers/) | « juste par accident » après harmonisation racine le chemin resterait valide (workspace root + crates…) : attention à la clauses P4 — harmonisation sans audit usages peut créer un faux-positif ici (le fichier existe de toute façon, mais pour la mauvaise raison). Non bloquant car le test vérifie un toml existant, pas la racine |

(P4 traitera 4 fichiers + chain.rs (m6_e2e_chain) avec le même pattern m1/t11 ; porte P4 = plan QA dédié.)

## Ce qui a été vérifié (Q1→Q5 de la tâche, faits file:line)

1. **Q1 aucun oracle dilué** : diff = `git show 665110c`, 5 fichiers tests, 20+/12−. Les
   assertions restent des cibles réelles panica-loud : références Test-Path True sur
   `crates\apparatus-reference-kv\apparatus.toml`, `…\Dockerfile`,
   `services\Manifesto\infra\src\apparatus_runtime\tick.rs` (= `TICK_SRC` t11:70),
   `workers\apparatus-operator\src\controller.rs`, `crates\apparatus-contracts`. Messages
   conservés et cohérents (`"manifeste disque crates/apparatus-reference-kv/apparatus.toml"`
   etc.). Aucun absolu (grep `C:\|C:/` vide sur les 5 fichiers). Pas de tautologie
   CARGO_MANIFEST_DIR (aucune assertion ne vérifie le manifeste/crate de l'opérateur lui-même
   via son propre CARGO_MANIFEST_DIR ; t11 :97-104 package-relatif est en t7, hors diff).
   `:583` t11 : pré-image déjà ortographiée `repo_root().join("workers/apparatus-operator/…")`
   → pré-fix = `workers/workers/…` (double préfixe), post-fix = chemin réel, éliminé PAR
   CONSTRUCTION sans toucher la ligne (vérifié `git show 665110c^`, seul le helper a changé).
   Contexte docker build m4 `:122-135` : `reference_kv_dir()` → `crates/apparatus-reference-kv`,
   contient `Dockerfile` réel ; image produite et enregistrée
   (`.tmp\docker-resources-20261017.log`, entrée pool-4 `created kind image id 6b629795e389
   name apparatus-reference-kv:m1`). Idem m1 `:78-97`. Aucune assertion déplacée vers une
   cible plus faible.
2. **Q2 m5 statique** : voir N-1 (cohérent et correct par construction + équivalence texte).
3. **Q3 preuve ↔ plan** :
   - Master post-fix (`git log -1 665110c` = 4 fichiers+logs) : `.tmp\g17-p2fix-master-m1.log`
     → `test result: ok. 1 passed; 0 failed; … finished in 9.62s` ; m3 → `2 passed / 4
     filtered out / 0.01s` ; m4 → `1 passed / 2 filtered / 0.01s` ; t11 → `3 passed / 4
     filtered / 0.07s`. Exit 0. Exactement les attendus §9.6 du plan (m1 1/1 ; secondaire
     2/2, 1/1, 3/3 — « filtrés » tant attendus : comptages bons).
   - t6 `.tmp\g17-t6-run2.log` : `9 passed; 0 failed … 23.03s`, fichier NON touché par le
     diff (correct : preuve transport de référence reste t6).
   - ROUGE pré-fix (`.tmp\g17-{m1-run,m3-filtered,m4-filtered,t11-filtered}.log`) : m1 1
     failed @0.00s, m3 2 failed @0.00s, m4 1 failed @0.00s, t11 1 failed @0.00s (2 wget
     oracles passaient — n'utilisent pas repo_root). Cohérent avec la famille « panic avant
     Docker » et ÉTEINT par le fix (comptages master 0 failed).
   - Matrice de mode G-17.7 (fallback docker servi) : registre première image pull delta
     oras:v1.2.2 + cosign:v2.4.3 à 17:36Z (env unset), helpers `create→connect(
     apparatus_operator_test-net)→start→die→destroy` par hop, `.tmp\g17-docker-events.txt`
     (34 Ko) + summary registre « 16 oras + 17 cosign », « APPARATUS_*_BIN unset ».
   - G-17.9 zéro résidu : registre `residue_post_run: aucun (docker ps -a vérifié)` pour les
     deux runs (P2 pool-2 + master pool-4) ; stops gracieux enregistrés (Exited (0),
     préservés pas rm).
   - G-17.10 : m5/t10 aucun log (jamais offerts), 4+2+4 filtrés = 10 des 11 Kind tests des
     fichiers exécutés (+ m5 1 + t10 4 hors run) = 15 tracés nominalement. Cohérent avec la
     plan (§6.2 commandes, aucun `--include-ignored`).
   - Registre : 20 lignes append-only (10 P2/pool-2 + 10 master/pool-4), timeline
     17:30Z→18:05Z→19:22Z cohérente avec le merge 20:12:36Z et logs master 20:13-20:14.
   - Le tiers `aiforall-local-control-plane` (kindest/node) Up non possédé, intouché,
     handback enregistré — barrière infra respectée.
4. **Q4 périmètre tenu** : aucune ré-ouverture de N-1/N-2/RP-A/RP-B/S-1/S-2
   (briefing c2fe7172). Seuls le diff et la preuve d'exécution sont jugés ; aucun test
   exécuté par cette revue, aucune cargo relancée, aucun fmt/clippy check.
5. **Q5 archive HC** : voir table ci-dessus (m2, t12, t2, t7, chain.rs + non-toucher zot/t6).

## Reasoning (déjà tranché — ne pas ré-explorer)

- La correction « join("..") × 2 » est du texte identique aux trois frères compilés+passés ;
  la toute première exécution post-fix (master, logs 20:13-20:14) a validé la mécanique
  complète pour m1/m3/m4/t11 (y compris lecture de tick.rs réel et controller réel).
- Le helper est volontairement « bête » (pas de vérification FS) : la détection d'un mauvais
  root passe par les assertions loud en aval (« Dockerfile absent », « services/Manifesto/
  lisible », « crates/apparatus-reference-kv/apparatus.toml: … »). Direction correcte :
  pas de tautologie, échec au code, pas au helper.
- Le commit a été squashé en `665110c` (pool-4 `1e91353`, worktree) ; hook pre-commit os-206
  préexistant (ligne >120) contourné par `--no-verify` avec preuve `rustfmt --check` sur les
  5 fichiers (Journal plan, ligne 308). Hors périmètre correctness (fmt, pas comportement).
- Les messages de panic « explicatifs » (nouveaux commentaires du diff) expliquent
  f060d47 ; c'est méthodologie de preuve, pas une dilution : le chemin QR pointe toujours
  vers le fichier réel.

## Do not redo

- Ne pas re-vérifier les cibles Test-Path ci-dessus (fait le 2026-10-17, head 665110c,
  arbre propre hors `.opencode/missions/`).
- Ne pas relancer aucun des 4 tests master ni aucune cargo (slot Windows ; preuves
  persistées, plan §9.7 : aucun re-run t6 exigé).
- Ne pas « corriger » hook pre-commit os-206 (hors lot, pre-existant, preuve équivalente en
  place).
- Ne pas rouvrir N-1/N-2/RP-A/RP-B/S-1/S-2 (defects transport correctif c2fe7172, hors axes).
- Ne pas rejuger t6 m1 dans sa partie transport (couvert par le stub de 23 tests unitaires
  du lot précédent + cette IT réelle) : le seul objet de revue de ce lot = helpers + preuve.
- Ne pas re-trancher G-17.10 (Kind defer GO utilisateur, cadré P4/P5).

## Anti-goals

- Ne PAS toucher aux 5 fichiers statiques hors périmètre (m2, t12, t2, t7, chain.rs) :
  archive P4, décision lead au Journal (ligne 310).
- Ne PAS harmoniser les fixtures correctes zot:101 / t6:59 (package-relatifs, ajustées).
- Ne PAS modifier `.cargo/config.toml`, `.tmp/`, le registre, les logs, le plan, le ledger
  pendant cette revue (lecture seule + briefing unique).
- Ne pas « renforcer » les assertions des tests m* (aucune demande ; toute resseration
  d'oracle = nouveau lot avec plan).
- Pas de relance Kind, jamais.

## Tests

- Preuves déjà persistées (aucun test relancé par cette revue) :
  `.tmp\g17-p2fix-master-{m1,m3,m4,t11}.log`, `.tmp\g17-t6-run2.log`,
  `.tmp\g17-{m1-run,m3-filtered,m4-filtered,t11-filtered}.log` (ROUGE),
  `.tmp\docker-resources-20261017.log` (20 entrées), `.tmp\g17-docker-events.txt`.
- Aucun test supplémentaire requis : pas de fix demandé par cette revue.
- Si P4 souhaite une preuve compile de m5 (optionnel) :
  `cargo test --locked -p apparatus-operator --features admit,controller --test
  apparatus_m5_invoke_isolated -- --list` (ne pas exécuter les tests).

## Staleness

Briefing lié à `665110c` sur master (HEAD revue = `f9d0aa7`, uniquement fermeture plan).
Tout nouveau commit touchant `workers/apparatus-operator/tests/` (fix P4 des 5 HC statiques,
nouveau fixture) invalide les numéros de ligne de la table d'archive ci-dessus. Si f060d47
reverse ou que un crate bouge (workers↔racine), le helper retrouvé en `join("..")×2` redeviet
faux : recentrér sur le pattern « helper bête + assertions loud » (any orphelinage au FAIT que
les cibles existent, pas au helper).
