# Review briefing — P6b test-reviewer (chaîne m6, diff 43c3804..e66b736)

    reviewer: test-reviewer (P6b)
    written_at: 2026-10-18T10:44Z
    branch: master
    head_sha: a3a6a17 (docs-only au-dessus de e7e6630 ; les 4 fichiers du diff sont inchangés depuis e66b736)
    dirty: false (parent porcelain : ?? .opencode/missions/ seulement)
    scope_slug: m6-e2e-chain
    verdict: GO_WITH_COMMENTS
    related_paths:
      - workers/apparatus-operator/tests/fixtures/kind/mod.rs
      - workers/apparatus-operator/tests/fixtures/kind/cluster.yaml
      - workers/apparatus-operator/tests/fixtures/chain.rs
      - crates/apparatus-reference-kv/Dockerfile.http
      - workers/apparatus-operator/tests/apparatus_m6_e2e_chain.rs (hors diff, oracles audités)

## Verdict

GO_WITH_COMMENTS. Chaque risque introduit par le diff possède un détecteur exécuté (bogus-neg, run2, run3, run4 vert) ou une limite nommée acceptée ; un run vert seul n'a jamais été utilisé comme preuve d'une obligation : les branches fail-loud jamais déclenchées (kindnet, calico-absent, SSA-conflict) et les captures brutes H1–H4 absentes sont rendues GAP-accept avec propriétaire et condition de réouverture. Aucun BLOCK : aucun finding T-n n'ouvre une voie de régression non détectée dans le périmètre GO.

## Faits vérifiés indépendamment (preuves relues, pas recopiées)

1. run4 (log UTF-16, 42 lignes, `C:/Users/djden/.local/share/opencode/worktree/186749/ensemble-crisp-atlas-mission-e63ddfca70084b31#8ud2p5-pool-14/.tmp/m6-e2e-chain-run4.log`) : commande plan echo verbatim (`cargo test --locked -p apparatus-operator --features admit,controller --test apparatus_m6_e2e_chain -- --test-threads=1`, KUBECONFIG exporté) ; `Finished test profile 16.74s` (code FINAL, post-révert/E-14) ; `test m6_e2e_0002_0008_chain ... ok` ; `test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 311.79s`. **La sortie du test est capturée par libtest (pass) : les eprintlns par hop sont absents du log.**
2. `--list` (`m6-compile-list.log`, 37 KB) : `Finished 6m30s` (lazaret-http/infra, axum-test, serial_test, testcontainers compilés) ; `m6_e2e_0002_0008_chain: test` ; `1 test, 0 benchmarks`. Timestamp 10:43:05 local (08:43Z), AVANT tout run.
3. Négatif bogus (`m6-neg-cluster.log`) : `test result: FAILED. 0 passed; 1 failed; ... finished in 0.86s` ; panic à `tests/apparatus_m6_e2e_chain.rs:59` avec le message d'Err explicite « cluster Kind bogus-inexistant absent — démarrer le runtime est une opération runtime, pas un effet de test » ; capture stdout (libtest imprime en échec) montre `M6 catalog digest (0002): sha256:98ba747fc572de29d76dfd08f92537782bf0353b652adcace10200020ee560cf` — identique au digest du CR post-run4 (`m6-post-run4-cr.txt`) : double confirmation indépendante de l'identité catalogue.
4. `m6-clusters-after-neg.txt` : exactement `aiforall-local` — aucun create/delete pendant le négatif.
5. run2 (`m6-e2e-chain-run2.log`, capture sur échec) : stdout montre `aiforall-local: wait Calico + CoreDNS Ready (v3.29.7, timeout 600s)` — preuve exécutée que la branche réutilise-ready d'ensure() traverse export→API→CNI→apply ; puis panic `chain.rs:234` build :m6 exit 1 (COPY chemin racine) = mécanisme E-13.
6. run3 (`m6-e2e-chain-run3.log`) : parse fail verbatim — `error inheriting rcgen from workspace root manifest workspace.dependencies` → mécanisme E-14 exécuté (le révert :16 était donc déjà validé au stage COPY).
7. Pré-run (inventaires 10:31–10:32Z worktree) : CRD NotFound, aucun ns apparatus-*, aucun deploy apparatus, Calico 1/1 Ready, kindnet absent, nœud Ready.
8. Post-run2 : les DEUX netpols présents (`plugins-isolate` ns plugins, `system-deny-plugins` ns system) ; CRD créée 08:48:34Z.
9. Snapshots master `.tmp/m6-post-run4-*.txt` : nœud Up 19h ; calico-node 1/1 ; CR VALID sha256-98ba747fc… ; pod plugin 1/1 Running, image @sha256:9e932a31… ; canary deploy/pod 1/1 Ready + Service 8080 ; 4 SA + roles/rb ; labels fixture sur les 2 ns ; labels J3 des ns préexistants inchangés. NB : `m6-post-run4-netpol.txt` est scopé ns plugins (1 seul netpol visible) — le fichier post-run2 complet est la preuve des deux.
10. Registres 17/18 relus : enregistrement tardif explicité (« Registration was late (worker omission), not contemporaneous »), G-1/G-2/G-3 reclassés par le lead après infra-verifier ; BuildKit UNKNOWN+ask ; aucune suppression.
11. run1 (`m6-e2e-chain.log`) : panic « Docker/Kind must be running: kubectl not found in PATH » — faux diagnostic exécuté (ROUGE run1), confirme l'hypothèse C-2 de P6a.
## Ancres diff rejugées (lecture directe)

- Point de nommage unique : `cluster_name()` (mod.rs:19-24) dérivé aux 8 sites (ensure :58, build/load :307, load_and_pin :425, mirror :446, cri-absent :497, eprintlns :204/:196, temp kubeconfig :1069, export :1072). Grep `apparatus-p4-it` sous `tests/` = **1 seule ligne, doc-comment :17** (jamais une cible) — écart assumé vs l'invariant D-1 « 0 » : la ligne commente une protection, pas un site de nommage.
- Suppressions prouvées par diff : `delete_cluster`, `create_cluster`, `install_calico_and_wait` → `assert_cni_ready`, `set_calico_kind_env`, `FELIX_IGNORELOOSERPF`, `CALICO_MANIFEST_URL` — grep post-diff = 0 hors doc.
- SSA : `apply --server-side --field-manager=apparatus-it-fixtures` (mod.rs:86-92) sans `--force-conflicts` — conflit = erreur = fail-loud.
- E-14 verbatim vérifié : staging `chain.rs:202-203` ≡ racine `Cargo.toml:88` (reqwest 0.12.0 json/rustls-tls) et `:142` (rcgen 0.13.2 x509-parser), strings identiques.
- m6 `Cargo.toml:70-72` : `required-features = ["admit", "controller"]` — le piège skip-silencieux est couvert par le comptage `--list`+run.
- Oracles m6 préservés : `apparatus_m6_e2e_chain.rs` absent du diff (git diff --stat = 4 fichiers).

## Findings

### T-1 — MEDIUM (limite documentaire du lot ; GAP-accept, pas de rerun)

- ID: T-1 / Reviewer: test-reviewer / Severity: MEDIUM / Confidence: HIGH / Scope: CROSS_FILE
- Location: `.tmp/m6-e2e-chain-run4.log` + obligation plan §6.3 (colonne « preuve à recopier dans le Journal »).
- Problem: les captures autonomes H1 (`signature_verified`, digest OCI), H2 (extrait JSON temp), H3 (statuts HTTP, valeur `hops`), H4 (sortie wget brute) sont **absentes** de la livraison : run4 vert = sortie capturée non imprimée par libtest ; les seuls extraits crus proviennent des échecs (négatif/run2/run3) et de la corroboration cluster post-run4.
- Expected behavior: extraits autonomes par hop recopiés au Journal (verbatim plan §6.3).
- Observed behavior: résumé summary-level au Journal (plan :380) + snapshots cluster indépendants ; aucun extrait brut inventé (comportement correct du lead).
- Why it matters: H1/H3 n'ont été exécutés QUE par run4 (run2/run3 mouraient avant ces hops) ; si un assert de ces hops était affaibli par une édition future, il n'existerait aucun artefact brut historique à opposer — le 1/1 resterait le seul transport de l'oracle.
- Evidence: mutation « affaiblir un assert H3 (ex. hops>=1 → hops>=0) » survivrait aux preuves actuelles (aucun artefact brut de run4) ; détectable seulement à un futur run avec captures.
- Evidence class: executed (assertions exécutées) + static-presence (absence de captures dans les fichiers fournis).
- Suggested correction: AUCUN rerun demandé. Condition de réouverture : tout run futur de m6 (ré-run idempotence R-09, lot touchant m6/fixtures) doit s'exécuter avec `--nocapture` redirigé vers un log archivé avant toute mise à jour de verdict.
- Owner: lead. Reopen condition: premier run m6 suivant.

### T-2 — LOW (préexistant, m6 hors diff ; fenêtre faux-vert canary)

- ID: T-2 / Reviewer: test-reviewer / Severity: LOW / Confidence: MEDIUM / Scope: LOCAL
- Location: `tests/apparatus_m6_e2e_chain.rs:392-436` (sonde wget AVANT le wait canary Ready best-effort) + `chain.rs:546-554` (`wget_blocked_signal`).
- Problem: le dernier oracle H4, `wget_blocked_signal(text) || !wget.status.success()`, accepte « connection refused »/timeout — or une absence d'endpoints (canary pas Ready) produit exactement ces signaux : le pass peut venir de « pas d'endpoints » et non de « netpol bloque ». Le wait canary Ready :427-436 est best-effort APRÈS la sonde.
- Expected behavior: établir le canary Ready (ou endpoints non vides) en fail-loud AVANT le wget.
- Observed behavior: aucun assert de Ready pré-sonde ; résultat du wait ignoré.
- Why it matters: mutation « netpol retiré + canary cassé » → wget échoue → test vert → fausse attestation T11 sur le cluster partagé. Pendant run4 le canary était Ready (snapshot indépendant) : fenêtre théorique.
- Evidence: mutation « has_kindnet→Ok(false) / netpol retiré » est en revanche détectée par le design exécuté (canary joignable → `wget_reached_target` → ROUGE) ; la fenêtre T-2 est le complément inverse (canary mort) — non exercée.
- Evidence class: static-presence + inference.
- Suggested correction: lot futur test-engineer — réordonner : `kubectl wait --for=condition=Ready pod -l app=apparatus-system-canary --timeout=30s` en assert (panic si échec) AVANT le wget ; garder le wait post best-effort facultatif.
- Quality dimension: couverture (oracle affaissable) ; fiabilité inchangée.
- Owner: test-engineer. Reopen condition: prochaine modification du fichier m6.

### T-3 — LOW (préexistant, dormant ; diagnostic faux du probe kubectl — reconvergence C-2 P6a)

- ID: T-3 / Reviewer: test-reviewer / Severity: LOW / Confidence: HIGH / Scope: LOCAL
- Location: `mod.rs:1046-1051` (`bin_ok` : `kubectl version` sans `--client`) + `:1039-1044` (`resolve_kubectl`).
- Problem: le probe de résolution teste la joignabilité serveur du kubeconfig PAR DÉFAUT ; périmé (ports remappés) → exit 1 → « kubectl not found in PATH » — diagnostic faux, exécuté en ROUGE run1 (183 s + auto-repair #1).
- Expected behavior: `kubectl version --client` (client-only) ; `bin_ok(kind)` est correct (kind version = client-only).
- Why it matters: tout re-run en shell frais sans KUBECONFIG pré-exporté reproduit le ROUGE environnemental avec diagnostic mensonger — coût de diagnostic, pas un risque d'oracle.
- Evidence: run1 log (« kubectl not found in PATH ») ; `bin_ok` inchangé hors diff.
- Evidence class: executed (run1 log) + static-presence.
- Suggested correction: lot fixture 1 ligne — `kubectl_ok()` dédié avec `version --client` ; ne pas durcir le probe kind.
- How to validate: `kubectl version --client` exit 0 avec kubeconfig par défaut périmé ; re-run m6 (GO) en shell frais → ensure() franchit le resolve.
- Owner: mechanical-worker, petit lot futur.

### C-x rejugés (conclusions P6a rejugées, pas réécrites)

- C-1 (MEDIUM, préexistant) : CONCURRE — vue tests : gap de couverture des couches staging m5/ps1, hors GO du lot ; GAP-defer (owner lead ; réouverture à la GO m5 réactivée ou 0605). L'argument « rétablit le contrat pour tous les consommateurs » n'est vrai que pour la couche COPY — la couche staging reste non prouvée pour m5/ps1 (aucun test, aucun build jour J).
- C-2 : reconverge avec T-3 (classée MEDIUM par P6a ; gardée LOW ici car l'impact est diagnostique, pas d'oracle).
- C-3 (LOW, cluster.yaml mort nommé aiforall-local, subnet divergent) : CONCURRE — aucun test utile ; risque documentaire de ré-usage ; GAP-accept (owner lead ; réouverture si une écriture recrée un usage code du fichier).
- C-4 (LOW, écart doc ops/README « intouchable ») : CONCURRE — hors périmètre tests (aucune preuve testable) ; suite doc.
- C-5 (INFO, padding cluster_name) : CONCURRE — pas de test ; le défaut échoue fail-loud non destructif ; durcissement au prochain passage mod.rs.
- C-6 (INFO, override non filtré contre les clusters protégés) : CONCURRE — protection procédurale ; pire cas borné par l'ensure non destructif (aucun chemin destructif restant) ; durcissement optionnel (rejet explicite des noms protégés dans cluster_name()) = candidat au prochain passage.
## Registre borné E-1…E-14 — preuve / classe / statut / limites

| E | Obligation (source plan) | Preuve disponible | Classe | Statut | Limites (accept/defer + owner) |
|---|---|---|---|---|---|
| E-1 | défaut aiforall-local + override env + point unique (D-1) | diff cluster_name() 8 sites ; grep tests = 1 doc-comment ; run4 vert en défaut ; bogus-neg 0,86 s honorant l'override ; clusters-after-neg inchangé | static-presence + executed | COVERED | branche override→cluster existant non exécutée (même code, chaîne différente) : accept. Owner lead, reopen sur prochaine édition mod.rs. |
| E-2 | ensure() non destructif 9 étapes ; jamais delete/create/install (D-2) | diff supprime les 4 remédiations (grep 0) ; run2+run4 branche réutilise-ready exécutée bout en bout ; bogus-neg = absent→Err sans create/delete | static-presence + executed | COVERED | branches kindnet / calico-absent / SSA-conflict jamais exécutées (pré-run propre) : accept — backstops structuraux (rollout calico ; canary joignable → ROUGE si netpol inappliqué). Pas de méta-test du fixture : la garantie anti-destructive est diff-audit + négatif. Owner lead. |
| E-3 | fix chain.rs mécanique + oracles préservés (D-3) | diff = 2 éditions racine + deps staging ; m6 hors diff | static-presence + executed | COVERED | — |
| E-4 | compile avant run + comptage exact | --list : 1 test rendu, 6m30 ; run4 Finished 16,74 s sur code final | executed | COVERED | --list sur code pré-révert/E-14 ; la compilation du code final est prouvée par run4 lui-même. Pas de preuve de compilation des 15 autres cibles (formulations D-3/E-4 « lie les 16 sites » corrigées au Journal plan :380 ; m5 --list HORS lot §9bis.5) — aucune preuve inventée. |
| E-5 | commandes exactes (oracle 3) | run4 log = commande plan verbatim + env KUBECONFIG echo | executed | COVERED | — |
| E-6 | H1 pin zot réel (oracle 4) | asserts :93-121 exécutés (1 passed ⇒ signature_verified, tri-identités, artifact_type) ; Docker events zot/oras/cosign create-die 09:10:03–11Z ; zot/bao exited 0 | executed + indirect | COVERED | captures brutes absentes (T-1, GAP-accept) |
| E-7 | H2 JSON VALID disque (oracle 4) | asserts :147-175 exécutés (raw contains VALID+catalog, reopen==Valid) | executed | COVERED | fichier temp détruit par Drop ; capture absente (T-1) |
| E-8 | H2b would_schedule + schedule réel (oracle 4) | asserts :194-263 exécutés + corroboration indépendante post-run4 (pod/CR/image/labels) | executed + observation | COVERED | hop le mieux corroboré |
| E-9 | H3 invoke Lazaret via pf réel (oracle 4) | asserts :361-390 exécutés (403+hops==0 ; 200+hops≥1+hostname==pod) ; pf kubectl spawn + wait_tcp 30 s | executed | COVERED | captures HTTP/hops absentes (T-1). Stub Manifesto = mock HTTP sortant autorisé (ADR 0201) ; Lazaret serveur in-test = design préexistant m6 (porte §6.5 P7, hors lot) |
| E-10 | H4 canary T11 bloqué, jamais reached (oracle 4) | asserts :414-425 exécutés ; post-run4 : les 2 netpols + canary Ready + endpoints | executed + observation | COVERED (limites T-2) | fenêtre faux-vert « mauvaise raison » nommée ; owner test-engineer, reopen sur prochaine édition m6 |
| E-11 | aucun skip silencieux + fail-loud + ROUGE protocol | comptages rendus (--list + 1 passed) ; runs1-3 ROUGE tracés §9bis file:line avant tout correctif | executed + static-presence | COVERED | — |
| E-12 | runtime/barrières jour J (oracle 5) | inventaires avant/après (worktree + master) ; registre régularisé tardivement explicité ; aucune suppression ; lifecycle = process | executed (inventaire) | COVERED avec écarts tracés | omission registre à la création (régularisé tardif) ; BuildKit UNKNOWN+ask ; conformité target-dir E: NON démontrée (recovery :45) — accept tracé. Owner lead. Reopen: tout run futur enregistre à la création et consigne le target effectif. |
| E-13 | révert Dockerfile.http:16 (§9bis.8/.9) | diff 1 ligne ; run2 ROUGE prouve le mécanisme inverse ; run4 build :m6 vert (COPY + apt wget) | static-presence + executed | COVERED | couche staging m5/ps1 non couverte (C-1 rejugé, GAP-defer hors GO) |
| E-14 | deps verbatim staging (§9bis.10) | diff :202-203 ≡ racine :88/:142 exact ; run3 ROUGE prouve le mécanisme inverse ; run4 stage-1 vert | static-presence + executed | COVERED | — |

## Mutation mindset — récapitulatif

| Mutation | Détecteur | Exécuté ? |
|---|---|---|
| M1 ignore CLUSTER_NAME_ENV | bogus-neg aurait muté aiforall-local au lieu d'échouer 0,86 s | OUI (bogus-neg) |
| M2 re-add delete/create (branche non-reusable) | AUCUN test (pas de négatif API-down) — barrière = diff-audit + doc D-2 | NON (accept, owner lead) |
| M3 has_calico_node→Ok(true) | rollout calico-node échoue | INFERENCE (backstop structural) |
| M4 has_kindnet→Ok(false) / netpol retiré | canary joignable → wget_reached_target → ROUGE | design exécuté (canary Ready pendant run4) |
| M5 digest biaisé (catalog≠descriptor) | assert_eq :93/:124/:151 | OUI (run4) |
| M6 COPY chemin racine ré-introduit | docker build exit 1 | OUI (run2) |
| M7 deps staging absentes | parse fail exit 101 | OUI (run3) |
| M8 compteur hops débranché | asserts hops==0 / hops>=1 | OUI (run4) |
| M9 hostname≠pod_name | assert :386 | OUI (run4) |
| M10 SSA→client-side apply | aucun test runtime discriminant (ns absentes avant ; delta labels non discriminant) | NON (accept — non exercisable jour J ; owner lead, reopen si J2 préexistant sur le cluster avant un run fixture) |

## Séparation introduction / antérieur

- **Introduits par le lot** : sémantique cluster_name() (exécutée : défaut + négatif) ; SSA (statique + additif exécuté, conflit jamais exercé) ; révert COPY (exécuté run2/run4) ; deps staging (exécuté run3/run4) ; limite documentaire captures (T-1, accept).
- **Antérieurs (hors diff)** : T-2 (m6 non touché), T-3 (bin_ok hors diff), triple dérivation staging (C-1), écarts registre (process, reclassés), absence de méta-test fixture (M2/M10).

## Do not redo

- Ne pas rejuger run4/négatif/--list : logs relus, résultats verbatim ci-dessus (recovery :7-9 ; neg log).
- Ne pas re-grepper les 16 sites ensure() ni les littéraux (comptes ci-dessus : 16 appels, 1 doc-comment).
- Ne pas re-prover le mécanisme inheritance reqwest/rcgen (run3 ROUGE + run4 vert).
- Ne pas ré-exécuter le négatif bogus ni demander un re-run m6 pour les captures (T-1 est GAP-accept).
- Ne pas demander la compilation des 15 autres cibles (hors lot, §9bis.5 ; Journal plan :380 corrige déjà D-3/E-4).

## Anti-goals

- Ne jamais déclarer tous les E-x couverts sur le seul 1/1 : les limites ci-dessus restent nommées.
- Ne jamais ré-introduire delete/create/install-Calico dans le fixture (D-7).
- Ne pas miroiter les deps côté m6 pour masquer les stagings m5/ps1 (refus P5 valide).
- Pas de changement code/plan/INDEX/ledger/commit par ce reviewer ; pas de Cargo/runtime/kubectl.

## Open questions

Aucun bloquant. T-1 (captures au prochain run), T-2 (réordonner oracle canary), T-3 (kubectl --client) et C-1 (stagings m5/ps1) = petits lots futurs à arbitrer par le lead.

## Staleness

Lie à e66b736 (code) / e7e6630→a3a6a17 (docs, arbre propre). Si HEAD bouge, reverifier les fichier:ligne (mod.rs volatil).
