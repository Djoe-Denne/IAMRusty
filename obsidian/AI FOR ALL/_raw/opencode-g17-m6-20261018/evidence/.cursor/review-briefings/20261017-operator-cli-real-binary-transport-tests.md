# Review briefing — tests transport binaire réel G-17 + P2-fix (665110c)

```yaml
reviewer: tests
written_at: 2026-10-17T18:35Z
branch: master
head_sha: f9d0aa7 # HEAD au moment de la revue ; commit audité = 665110c (parent direct)
dirty: false # hors .opencode/missions/ (mission non commitée, hors lot)
scope_slug: operator-cli-real-binary-transport
verdict: PASS_WITH_NOTES # aucun BLOCK ; 2 notes INFO, aucune action dans ce lot (vocable template = PASS/DEFER : rien à DEFER)
related_paths:
  - workers/apparatus-operator/tests/apparatus_m1_reference_kv_pin.rs
  - workers/apparatus-operator/tests/apparatus_m3_install_fail_closed.rs
  - workers/apparatus-operator/tests/apparatus_m4_desired_state_bridge.rs
  - workers/apparatus-operator/tests/apparatus_m5_invoke_isolated.rs
  - workers/apparatus-operator/tests/apparatus_p4_t11_runtime.rs
```

## Verdict

Le diff `665110c` (harmonisation mécanique des helpers `repo_root()`, 5 fichiers `tests/`,
20+/12−, aucun `src/`, gitlink `41a86dba` immobile) conserve l'oracle qui attrape la classe
ROUGE : les assertions restent panic-loud sur des cibles réelles existantes, et le retour à
l'ancre fausse a été **démontré par exécution** (famille ROUGE pré-fix, panics à 0,00 s,
avant tout Docker) puis éteint sur master (comptages post-fix, zéro FAIL). Les états
COVERED du registre G-17 reposent sur des preuves exécutées tracées (t6 9/9 hops pos/neg via
zot ; m1 1/1 avec build CRI réel ; fallback docker 16+17 helpers, zéro résidu) ; le defer
G-17.10 reste nominal et honnête (aucun skip silencieux, m5/t10 jamais exécutés et jamais
prétendus). Deux notes INFO, aucune action requise.

## Q1→Q5 de la tâche (mutation mindset, preuves file:line à head `665110c`)

### Q1 — mutation « dérive d'ancrage » : l'oracle ferme (démonstration exécutée)

**Faits :**

- Helper post-fix identique sur les 5 fichiers : `CARGO_MANIFEST_DIR/../..` puis
  `join("crates")` (m1 `:25-33`, m3 `:36-44`, m4 `:37-45`, m5 `:62-69`, t11 `:83-87`) ;
  assertions panic-loud inchangées par le diff :
  m1 `:40-42` (« manifeste disque crates/apparatus-reference-kv/apparatus.toml »),
  m3 `:50-53` (`descriptor_from_disk`) et `:64-68` (`m1_descriptor_and_report`, panic :67),
  m4 `:48` + contexte build `:125`,
  m5 `:73-74` / `:81-82` (panics loud sur apparatus.toml absent) et usages
  `:196` / `:199` / `:201` (Dockerfile.http),
  t11 `:416-418` (`read_dir` expect « services/Manifesto/ lisible »), `:583-587`
  (contrôleur réel `workers/apparatus-operator/src/controller.rs` vs TICK_SRC),
  `:598-601` (TICK_SRC existe toujours).
- Mutation (a) « retour à une seule remontée » ou (b) « join du mauvais sous-répertoire »
  ou (c) « double préfixe workers/workers/ » : **exécutées en réel** par la famille ROUGE
  pré-fix — `.tmp/g17-m1-run.log:36-41` (panic `apparatus_m1_reference_kv_pin.rs:39:9`,
  1 failed @0,00 s), `.tmp/g17-m3-filtered.log:143,149,157` (2 failed @0,00 s, panic :65:9),
  `.tmp/g17-m4-filtered.log:45,53` (1 failed @0,00 s, panic :51:9), `.tmp/g17-t11-filtered.log:46,54`
  (1 failed @0,00 s, panic :418:44 ; les 2 oracles wget, sans helper, restaient verts —
  cohérent). Les trois modes de dérive produisent une cible impossible
  (`workers/crates/…`, `…/crates` au mauvais niveau, `workers/workers/…`) → panic au
  premier accès, jamais un pass.
- Post-fix, les mêmes cibles existent en réel (Test-Path re-vérifié par cette revue :
  `crates\apparatus-reference-kv\apparatus.toml`, `…\Dockerfile`, `…\Dockerfile.http`,
  `services\Manifesto\infra\src\apparatus_runtime\tick.rs`,
  `workers\apparatus-operator\src\controller.rs`, `crates\apparatus-contracts`) →
  comptages master 0 failed (Q4). Le diff ne déplace aucune assertion vers une cible plus
  faible ; le helper demeure volontairement « bête » (pas de check FS), la détection est
  portée par les asserts loud en aval — un helper ré-ancré une troisième fois (ou une
  relocation crates↔workers type f060d47) serait re-attrapé par ces mêmes panics à 0,00 s.

Classe de preuve : **executed** (ROUGE pré-fix + comptages master post-fix) + static-presence
(Test-Path des cibles courantes).

### Q2 — registre G-17.x : COVERED uniquement sur preuve exécutée tracée

Plan : `.cursor/qa-plans/20261017-operator-cli-real-binary-transport.md` ; ledger :
`.cursor/handoffs/20261017-g17-e2e-chain-mission-prep.md`.

| Gap | État porté (Journal plan `:305-310`, ledger `:215-230`) | Preuve file:line | Classe |
|---|---|---|---|
| G-17.1 push ORAS réel | COVERED (exécution P2) | `t6_admit_signer_pushes_envelope_and_signs … ok` `.tmp/g17-t6-run2.log:29,39` (9/9) ; extraits zot tracés plan `:305` + ledger `:173` (digest `sha256:b707d331…96df1` 64 hex, catalogue, artifactType) | executed (log) + static-presence (extraits zot) |
| G-17.2 fetch manifest réel | COVERED (exécution P2) | `.tmp/g17-t6-run2.log:32` (`t6_catalog_identity_…`, digest lu ≠ descriptor) ; m1 1/1 master `.tmp/g17-p2fix-master-m1.log:32` avec build CRI réel (registre `.tmp/docker-resources-20261017.log:16`, image `apparatus-reference-kv:m1` id 6b629795e389) | executed |
| G-17.3 cosign sign réel | COVERED (exécution P2) | tag `.sig` HTTP 200 (ledger `:174-176`) en précondition du verify-ko ; `.tmp/g17-t6-run2.log:29,37` | executed (log) + static-presence (extraits) |
| G-17.4 verify positif réel | COVERED (exécution P2) | `.tmp/g17-t6-run2.log:29` — `signature_verified=true` exigé (`admit.rs:225-226` cité plan §4) | executed |
| G-17.5 verify négatif réel | COVERED (exécution P2) | `.tmp/g17-t6-run2.log:37` (`t6_verify_ko_with_sig_tag_refuses … ok`, clé Transit wrong + précondition tag présent) | executed |
| G-17.6 hop 5 ORAS tag `.sig` | COVERED (exécution P2) | précondition codée du verify-ko (`t6_verify_ko:188-192` cité plan §4-5) + ledger `:174` | executed via G-17.5 |
| G-17.7 mode transport prouvé | COVERED (décision lead §9.5 : fallback docker servi) | pulls réels `oras:v1.2.2`/`cosign:v2.4.3` `.tmp/docker-resources-20261017.log:1-2` (17:36:00Z, delta) ; helpers 16 oras + 17 cosign `:10` + `.tmp/g17-docker-events.txt` (comptage refait par cette revue : oras create 16 / die 16 / destroy 16 ; cosign create 17 / die 17 / destroy 18 — T-2) ; ré-attesté sur master « mode fallback docker servi : APPARATUS_*_BIN unset » registre `:17` | executed |
| G-17.8 hooks cfg(test) inertes en IT | COVERED (inférence structurelle + sanction à l'exécution) | plan §1.2 (7 sites, modèle Cargo `[[test]]`) ; si l'intercept tournait, zot ne recevrait rien — les artefacts zot réels prouvent le contraire (délégué à G-17.1→G-17.5, conforme §5) | inference + executed (délégué) |
| G-17.9 fallback complet, zéro résidu | COVERED (par le mode servi lui-même, décision lead §9.5) | `.tmp/docker-resources-20261017.log:10,17` (lifecycle cidfile/id create→start→die→destroy par hop, `residue_post_run: aucun`, filtre images pinées = 0) + lifecycle équilibré events (ci-dessus) | executed |
| G-17.10 admission + transport DANS ITs Kind | GAP-defer honnête (lead scope + GO utilisateur, cadré P4/P5) | plan §0 (15 tests nommés) ; grep courant 15 occurrences in-lot (+1 `m6_e2e_chain:59`, axe 2) — m3 `:297,352,471,522`, m4 `:331,503`, m5 `:452`, t10 `:124,311,391,596`, t11 `:450,472,500,527` ; commandes §6.2 : filtres exécutés = exactement les sous-ensembles sans Kind (Q4) — jamais `--include-ignored` ; m5/t10 jamais offerts (aucun log, journal `:308-309`) | static-presence (nomination) |
| G-17.11 push_platform_plugin_to_zot (fixture skopeo/crane) | GAP-defer avec G-17.10 | plan §5 row (hors transport `admit.rs`) ; journal `:310` | static-presence |

- La couverture **unitaire G-14 (mockée) n'est comptée nulle part comme preuve réelle** :
  plan §6.4 (`:213`) la garde séparée (« parité prouvée unitairement… ici seul le mode servi
  doit être prouvé ») et la row G-17.9 de §5 (`:146`) dit explicitement « G-14 unitaire
  mocké seulement ». ✓
- Note de structure (conforme au design du plan, porte §6.5 ligne 220 = « append au
  Journal ») : les rows §5 gardent leurs états planifiés ; la fermeture est portée par le
  Journal `:305-310` et le ledger `:215-230`. Mécanisme append prévu, pas une incohérence.

### Q3 — defer honnête : aucun filtrage silencieux, aucune prétention d'exécution

- Les 15 tests Kind sont cités nominativement (plan §0) et leur exclusion est **rendue
  visible** par les comptages `filtered out` des logs : m3 4 + m4 2 + t11 4 = 10 des 11
  Kind des fichiers exécutés, + m5 (1) et t10 (4) jamais offerts = 15. ✓
- m5/t10 : fichiers entiers jamais exécutés (aucun log m5/t10 dans `.tmp`, vérifié liste
  `g17*`), journalisés plan `:308-309`. La correction **statique** de m5 dans `665110c`
  (`:62-69`) ne prétend PAS créer une preuve d'exécution : aucun endroit du plan, du ledger
  ou du briefing correctness ne le réclame (« Kind defer, jamais exécuté », journal `:308`,
  N-1 correctness `:47-53`). Borne corroborée ici : m5 jamais compilé (aucune cible
  `[[test]]` m5 dans aucun run — chaque `[[test]]` est un binaire séparé), texte du fix
  identique aux trois frères compilés et verts, cibles réelles existantes (Test-Path,
  y compris `Dockerfile.http`). Précisément ce que la question exige : pas de preuve
  fabriquée.
- Protocol ROUGE respecté : preuve persistée avant tout correctif (commits `614a6e5` →
  fix `665110c`, 11 min après), correctif test-only, aucune dilution d'oracle (Q1).

### Q4 — attendus post-fix exacts : concordance logs master ↔ plan §6.5, zéro FAIL

| IT attendu (plan §9.6 / §6.2) | Log master (référence) | Résultat | Concordance |
|---|---|---|---|
| m1 1/1 | `.tmp/g17-p2fix-master-m1.log:29,32` | `running 1 test` → `ok. 1 passed; 0 failed … 9.62s` | ✓ |
| m3 2 passed / 4 filtrés | `.tmp/g17-p2fix-master-m3.log:35,37,42` | `running 2 tests` (les 2 noms §6.2, vérifiés nom à nom) → `ok. 2 passed; 0 failed; 4 filtered out` @0,01s | ✓ |
| m4 1 passed / 2 filtrés | `.tmp/g17-p2fix-master-m4.log:38,41` | `ok. 1 passed; 0 failed; 2 filtered out` @0,01s (nom vérifié) | ✓ |
| t11 3 passed / 4 filtrés | `.tmp/g17-p2fix-master-t11.log:38,43` | `ok. 3 passed; 0 failed; 4 filtered out` @0,07s (les 3 noms §6.2 vérifiés) | ✓ |
| t6 9/9 (preuve de référence P2, fichier non touché par le diff) | `.tmp/g17-t6-run2.log:28,39` | `ok. 9 passed; 0 failed … 23.03s` (9 noms listés) | ✓ |

Les noms exécutés dans les logs master sont exactement les filtres §6.2 (vérifiés ligne à
ligne) : les comptages « filtered out » correspondent précisément aux tests Kind du plan —
pas un masquage. Zéro FAIL sur les 5 logs ; porte de sortie §6.5 atteinte sur son propre
libellé.

### Q5 — statique hors périmètre tracée sans prétention de couverture

- Journal plan `:310` : m2 `apparatus_m2_valid_persist.rs:20-25`, t12
  `apparatus_p4_t12_gate_regression.rs:13-14` (+ usages :70/:79/:178/:209/:380-442),
  t2 `apparatus_p4_t2_scaffold.rs:16-19`, t7 `apparatus_p4_t7_admission.rs:73-76`, et
  `tests/fixtures/chain.rs:49-50` — « résidus structuraux hors axes de cette mission →
  tracés, décision utilisateur » ; `chain.rs` explicitement routé « consommé par
  `m6_e2e_chain` → prérequis du ré-ciblage axe 2, à traiter dans le plan P4 ». Le plan ne
  prétend **aucune** couverture sur ces sites ; le briefing correctness table-archive leur
  mode d'échec si un jour exécutés. m6 jamais lancé (aucun log) → aucune preuve absente
  feintée.
- Le diff n'empiète pas dessus : les 5 fichiers = m1/m3/m4/m5/t11 (t6, t10, m2, t12, t2,
  t7, `chain.rs` absents du commit — vérifié `git show --stat 665110c`).

## Findings

### T-1 — INFO

- **Location:** `.tmp` (absence de capture brute zot) vs plan
  `.cursor/qa-plans/20261017-operator-cli-real-binary-transport.md:305` + ledger
  `.cursor/handoffs/20261017-g17-e2e-chain-mission-prep.md:173-176`
- **What:** les extraits zot-side (catalogue `apparatus/envelope`, artifactType, digest
  `sha256:b707d331…96df1`, 4 tags `.sig` HTTP 200) ne sont pas des captures brutes
  autonomes dans `.tmp` ; ils sont tracés en extraits cités dans le plan/ledger. Les 9/9
  verts de t6 portent déjà les oracles in-test (signature_verified exigé, digest lu ≠
  descriptor, verify-ko Err) — la conclusion « hops réels » ne dépend pas de cette capture.
- **Why:** si un re-run ou un arbitrage postérieur exige la raw capture (p.ex. vérifier un
  tag précis chez zot), elle devra être générée à ce stade ; la porte §6.5 (« avec les
  extraits ») est honnie en l'état.
- **Suggested correction:** none (aucune action dans ce lot ; ne pas ré-exécuter t6).
- **GAP ref:** supporte G-17.1→G-17.6 (qualité de trace, pas d'intent manquant).
- **Evidence class:** static-presence (les extraits) + executed (logs libtest).

### T-2 — INFO

- **Location:** `.tmp/g17-docker-events.txt` (comptage refait par cette revue) vs
  `.tmp/docker-resources-20261017.log:10`
- **What:** oras : create 16 / die 16 / destroy 16 (équilibré) ; cosign : create 17 /
  die 17 / **destroy 18** — un destroy au-delà de la fenêtre de capture des creates. Le
  résumé registre (« 16 + 17 ») s'aligne sur les creates.
- **Why:** cohérent avec le `docker_rm` codé par la fixture sur ses propres noms de
  résidus (plan §1.3 ; registre `:6`-`:7`, `:14` : destruction de résidus P2 avant
  recréation) ; les paires create/die sont équilibrées (le sens d'un leak serait l'inverse)
  et le `docker ps -a` post-run filtré = 0 (`registre :10,17`). Aucun impact G-17.9 ; noté
  pour qu'un audit futur ne ré-attrape pas ce delta comme anomalie.
- **Suggested correction:** none (lecture seule ; ne pas re-capturer).
- **GAP ref:** G-17.9 (confirme, ne l'affaiblit pas).
- **Evidence class:** inference (attribution du destroy supplémentaire) sur base executed
  (les 16/16 et 17/17).

Autre finding : none.

## Reasoning (adjudiqué — ne pas ré-explorer)

- Niveau de test conformément au pin ADR 0200-0202 : IT réelles testcontainers (zot v2.1.8 +
  OpenBao Transit) contre binaires ORAS 1.2.2 / Cosign 2.4.3 réels en fallback docker sur images
  pinées — jamais de mock sur le hop transport (Protocol G-17). La couverture unitaire
  mockée G-14 reste disqualifiée comme preuve du mode réel (Q2).
- Le helper « bête » sans check FS est un choix méthodologique correct : le diagnostic
  passe par les asserts loud pointant vers les fichiers réels (messages nommés), la
  détection à 0,00 s est **démontrée** par la famille ROUGE — c'est de la méthodologie de
  preuve, pas une dilution (aligné briefing correctness §Reasoning).
- Ancres du plan §0/§1.4 légèrement décalées par les insertions du propre diff (net +2
  lignes dans m1/m3/m4/m5, net 0 dans t11 → lignes Kind t11 inchangées) ; les comptes, eux,
  sont exacts (Q3, Q4). Le mapping courant de cette revue prime pour tout G-17.10 futur.
- Porte de sortie §6.5 complète : t6 9/9 + m1 1/1 + preuve mode (G-17.7) + 15 defer
  nominalement tracés + secondaire 6 filtrés verts, zéro FAIL. Conformité postures
  infra : registre `.tmp/docker-resources-20261017.log` (20 entrées append-only), tiers
  `aiforall-local-control-plane` non possédé (handback `:13`,`:20`), exits Exited(0)
  préservés.

## Do not redo

- Ne pas relancer de cargo (t6/m1/m3/m4/t11) ni de la gate : preuves persistées, slot
  Windows, plan §9.7 (aucun re-run t6 exigé). Ne pas exécuter m5/t10 (Kind defer, GO
  utilisateur) ni `--include-ignored`.
- Ne pas rejuger G-17.10 (Kind defer cadré P4/P5, GO utilisateur) ni la décision fallback
  §9.5 (mode servi prouvé par lui-même).
- Ne pas re-vérifier les Test-Path ci-dessus (fait ici, head `665110c`).
- Ne pas re-scanner `.tmp` à la recherche d'un run1-t6 : N-2 du briefing correctness
  (`:57-67`) a tranché la borne (1 fichier t6 persisté ; extraits au Journal).
- Ne pas compter G-14 (unitaire, mockée) comme preuve du mode réel — jamais (Q2).
- Ne pas re-trancher la méthode helper-bête + asserts loud (adjudiquée : méthodologie de preuve).

## Anti-goals

- Ne pas resserrer ou élargir les axes de la mission ; ne pas toucher aux statiques hors
  périmètre (m2, t12, t2, t7, `chain.rs`) — archive P4, décision lead au Journal `:310`.
- Ne pas modifier les assertions des tests m*, les fixtures (zot/kind/t6 package-relatifs
  corrects), ou le mode des helpers — tout resserrage d'oracle = nouveau lot avec plan QA.
- Ne pas toucher `.cargo/config.toml`, le registre Docker, les logs, le plan ou le ledger
  pendant cette revue (lecture seule ; revue = un seul fichier écrit, celui-ci).
- Pas de relance Kind, jamais (cluster protégé + IT jamais via Kind).

## Open questions

None pour la couverture. (Item futur déjà cadré : arm host-bin le jour où des binaires
hôte ORAS/Cosign sont installés — §6.4/§9.5 plan, buffer nommé.)

## Tests

- Cette revue : lecture seule, exécutions interdites (aucun cargo, aucun test, aucun
  Docker/Kind touché). Constat : aucune commande exécutée par la revue beyond git/greps/lectures.
- Preuves de référence persistées (executed) : `.tmp/g17-t6-run2.log`,
  `.tmp/g17-p2fix-master-{m1,m3,m4,t11}.log`, `.tmp/g17-{m1-run,m3-filtered,m4-filtered,t11-filtered}.log`
  (ROUGE), `.tmp/docker-resources-20261017.log`, `.tmp/g17-docker-events.txt`.
- Rien à exécuter en plus pour fermer ce lot. Optionnel P4+ (N-1 correctness déjà formulé,
  repris tel quel) : compile-only de m5 (`cargo test --locked -p apparatus-operator
  --features admit,controller --test apparatus_m5_invoke_isolated -- --list`) au slot
  Windows **avant** tout run Kind, au GO utilisateur.

## Staleness

Briefing lié à `665110c` / HEAD `f9d0aa7`, arbre propre (hors `.opencode/missions/`). Un
nouveau commit touchant `workers/apparatus-operator/tests/` (fix P4 des statiques,
re-ciblage Kind, relocation crates↔workers) invalide les file:line ci-dessus ; la classe
mutation « dérive d'ancrage » reste valable tant que le pattern « helper bête + asserts
loud sur cibles réelles » est conservé — c'est le pattern à recentrer en P4, pas les
numéros de ligne.
