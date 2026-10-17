# Plan QA — lot G-17 : transport binaire réel ORAS/Cosign (axe 1, P1→P2)

- **Lot** : G-17 du registre `.cursor/qa-plans/20261017-operator-cli-credentials.md` (G-17 « fidélité réelle ORAS/Cosign », **GAP-defer** du lot M2, décision lead §8.3) — comblé ici. Ordre : ledger `.cursor/handoffs/20261017-g17-e2e-chain-mission-prep.md` (P1/P2), tâche `task_mvcmn5mc_0001_tn3fkp`.
- **Propriétaire** : `qa-planner` (ce plan). Exécution : P2 (`test-engineer`). Audit : P3 (`test-reviewer`), ses findings référenceront `G-17.<n>` de CE fichier.
- **HEADbase vérifié le 2026-10-17** : `6f34b6d7a104d52058cd0e54c1aeee35ac12e167` (`git rev-parse HEAD`). Pin rustycog `41a86dba0a24844f900783e19ef2405319d58b3f` (`heads/main`), immobile. Porcelain parent : `?? .opencode/missions/` seulement (mission, hors lot).
- **Planification seule (P1)** : aucun runtime démarré, aucun cargo lancé, aucun code ni test écrit. Ce plan désigne les niveaux et les oracles ; P2 exécute.
- **Niveaux** : pin ADR 0200–0202 (IT infra réelle, mocks sortants seulement, queues opt-in). Ici : **IT testcontainers réelle** (zot + OpenBao Transit, Docker infra tierce) contre les binaires ORAS/Cosign réels. Aucune stratégie de test écrite possédée par des humains au-delà des ADR : l'absence reste nommée, pas comblée.

## 0. Contradiction structurelle détectée — escalade au lead (avant tout run)

**Fait** : 4 des 7 IT du lot appellent `KindCluster::ensure()` — fixture du cluster **protégé** `kind-apparatus-p4-it` :

- `tests/apparatus_m3_install_fail_closed.rs:295,350,469,520` (4 tests)
- `tests/apparatus_m4_desired_state_bridge.rs:329,501` (2 tests)
- `tests/apparatus_m5_invoke_isolated.rs:450` (1 test)
- `tests/apparatus_p4_t10_platform_workload.rs:124,311,391,596` (4 tests)
- `tests/apparatus_p4_t11_runtime.rs:450,472,500,527` (4 tests)

Total : **15 tests sur 31** du lot. Le fixture (`tests/fixtures/kind/mod.rs:35-65`) **réutilise** `apparatus-p4-it` si API up + Calico présent, sinon le **supprime** (`kind delete cluster --name apparatus-p4-it`, `:99-112`) et le recrée. Or : (a) le cluster est protégé — jamais touché (infra-safety, AGENTS.md, ledger §Barrières) ; (b) « IT jamais via Kind » (AGENTS.md) ; (c) le runtime P2 ordonné est Docker seul (« fixtures testcontainers »).

**Conséquence** : la porte de sortie littérale « 7 IT vertes » **n'est pas atteignable en l'état**. Le ledger n'avait cadré cette contradiction que pour l'axe 2 (E2E, résolution P4) ; la vérification grep montre qu'elle frappe le lot G-17 lui-même. Ce plan ne la résout pas : il scinde explicitement (§4, §5) et **escalade** — le lead tranche le scope P2 (recommandation : transport-only, conforme aux barrières) et l'arbitrage « quand ré-cibler les 15 tests Kind » reste au GO utilisateur (P4/P5, déjà prévu au ledger). Aucun test Kind ne sera lancé, ni filtré silencieusement : chaque test non exécuté est tracé nominativement (§5 G-17.10).

## 1. Faits vérifiés (file:line, base `6f34b6d7`)

### 1.1 Transport réel et sites CLI (état post-correctif `c2fe7172ccc4`)

- Pins : `COSIGN_IMAGE = gcr.io/projectsigstore/cosign:v2.4.3` (`admit.rs:30`), `ORAS_IMAGE = ghcr.io/oras-project/oras:v1.2.2` (`admit.rs:32`). Env binaires : `APPARATUS_COSIGN_BIN`/`APPARATUS_ORAS_BIN` (`admit.rs:26-28`).
- Chaîne `push_and_sign_envelope` (`admit.rs:265-321`) : `validate_cri_pin` (269) → `inspect_transit_key` reqwest (270-275) → écriture `envelope.json` (277-289) → choix cible registry selon mode (291-296) → ORAS push + ORAS manifest fetch → digest OCI (298-303, `oras_push_and_oci_digest` 323-364) → `sign_and_verify` (306, 434-455) → nettoyage workdir.
- Cinq hops CLI réels :
  1. **ORAS push** `--password-stdin` : args `admit.rs:328-338`, `invoke_oras` (339).
  2. **ORAS manifest fetch** `--password-stdin --descriptor` : args `344-353`, `invoke_oras` (354).
  3. **Cosign sign** `--registry-username` (sans flag password) : args `436-447`, `invoke_cosign` (448).
  4. **Cosign verify** `--insecure-ignore-tlog` : args `464-474`, `invoke_cosign` (475).
  5. **ORAS tag .sig** (`signature_tag_exists`, fetch de `:sha256-{hex}.sig`) : args `502-510`, `invoke_oras` (511).
- `invoke_oras` (`admit.rs:515-532`) : stdin = `oras_password_frame` (CRLF, `cli_transport.rs:61-66`), secrets = [password registry, vault token]. `invoke_cosign` (`534-561`) : `with_registry_config` (dir privé + `config.json` AuthConfig host+network, `cli_transport.rs:551-601`) + env enfant `VAULT_ADDR`/`VAULT_TOKEN`/`TRANSIT_SECRET_ENGINE_PATH`/`COSIGN_YES`/`DOCKER_CONFIG` (`542-548`), stdin None.
- Deux transports dans `run_cli` (`cli_transport.rs:68-86`) : binaire hôte via `host_tool(bin_env)` (`72`), sinon `run_docker` (`88-151`) : `docker run -i --network … --cidfile …` + `-v {dir}:/auth:ro -e DOCKER_CONFIG=/auth` (104-110) + `-e` par env ; release par **id** (cidfile 64 hex, stop → inspect `exited|dead` → rm, `414-471`), jamais par nom, jamais `-f`.
- Garde-fous : `reject_secret_argv` (`cli_transport.rs:153-169`) refuse les valeurs secrètes **et** les flags `--password`/`--registry-password` ; deadline 120 s (`CLI_DEADLINE`, `:23`), plafond sortie 1 MiB (`:24`), scrub env (`:28-34`), supervision enfant+joins (`194-238`).

### 1.2 Hooks `#[cfg(test)]` — inventaire complet (7 sites)

- `admit.rs:22` (`mod cli_transport_tests`), `admit.rs:564` (`host_tool` → `forced_host`).
- `cli_transport.rs:187` (`execute` → `intercept`), `:724` (`cli_deadline` override), `:732` (`output_limit` override), `:739` (`forced_host`), `:744` (`drop_container_release`).

**Inférence (confirmée par lecture Cargo.toml + organisation, standard Cargo)** : les binaires d'IT (`[[test]]`, `Cargo.toml:35-72`) compilent la lib comme dépendance **sans** `cfg(test)` — les 7 hooks sont inertes en IT ; seul `--lib` les compile. La preuve d'exécution réelle est néanmoins **à l'exécution** : si l'intercept était actif, aucun enfant ORAS/Cosign ne spawnérait et zot ne recevrait rien — les artefacts zot (digest + tag `.sig` + verify OK) prouvent le transport réel (voir G-17.7/G-17.8).

### 1.3 Fixtures testcontainers (service-local, pas rustycog-testing)

- `SignRegistryStack::start()` (`tests/fixtures/mod.rs:28-33`) : `assert_docker_running` fail-loud (`:47-63`, `docker info`).
- **zot** : `ghcr.io/project-zot/zot:v2.1.8` (`zot/mod.rs:16-18`), conteneur `apparatus_operator_test-zot` (`:20`), port 5000, réseau `apparatus_operator_test-net` (`fixtures/mod.rs:11`), htpasswd `signer`/`apparatus-signer-push` (`:22-24`), startup 180 s + ready 60×250 ms (`:26-27,106`), config/htpasswd copiés depuis `tests/fixtures/zot/config.json` et `htpasswd` (présents, glob vérifié). Singleton `OnceLock` process-local (`:29,92-127`).
- **OpenBao Transit** : `openbao/openbao:2.6.2` (`openbao-transit/mod.rs:15-17`), conteneur `apparatus_operator_test-openbao-transit` (`:19`), port 8200, dev root token `apparatus-operator-transit-dev` (`:21`), clé `apparatus-p4-cosign` ecdsa-p256 (`:23,103-134`), startup 180 s + ready 60×200 ms.
- **Nettoyage résidu par le code fixture** : `docker_rm(CONTAINER_NAME)` avant création (`fixtures/mod.rs:65-72`, appelé `zot/mod.rs:99`, `openbao-transit/mod.rs:144`) — `docker stop` + `docker rm -f` **ciblé sur ses propres noms** (résidus de runs précédents). Comportement de test existant, exécuté par le code de test, **pas une action d'agent** : P2 le documente, ne l'exécute jamais à la main (§6 règles runtime).
- **Fixtures Kind** (`fixtures/kind/mod.rs`, `with_kind.rs`) : réservées aux 15 tests Kind (§0). `push_platform_plugin_to_zot` (`kind/mod.rs:544-599`) pousse via skopeo/crane (fixture-side, hors transport `admit.rs`).
- `chain.rs` : inclus uniquement par `apparatus_m6_e2e_chain` (axe 2) — hors lot.

### 1.4 Cartographie des 31 tests des 7 fichiers

Vérifié par lecture intégrale + greps (`KindCluster::ensure` 16 occurrences au total = 15 dans le lot + 1 (`m6_e2e_chain:59`, axe 2) ; `stack().await` ; `push_and_sign_envelope` 12 appels, dont 11 dans le lot).

| Fichier | Tests | Transport réel (push+sign) | Docker seuls (zot/openbao) | Sans Docker | Kind (bloqué §0) |
|---|---|---|---|---|---|
| `apparatus_p4_t6_sign_registry.rs` | 9 | 4 : `t6_admit_signer_pushes_envelope_and_signs` (163), `t6_verify_ko_with_sig_tag_refuses` (182), `t6_catalog_identity_…` (206), `t6_envelope_pins_cri_image_digest` (230) | 2 : `t6_docker_zot_openbao_start_or_fail_loud` (137), `t6_build_anonymous_push_forbidden` (317, HTTP zot 401/403) | 3 : `t6_cri_pin_rejects_short_or_non_hex` (250, refus avant tout CLI), `t6_fixture_names_are_not_lazaret` (337), `t6_build_sources_have_no_transit_client` (369) | 0 |
| `apparatus_m1_reference_kv_pin.rs` | 1 | 1 : `m1_reference_kv_pinned_envelope_on_zot` (137, build docker reference-kv réel + chaîne complète) | — | — | 0 |
| `apparatus_m3_install_fail_closed.rs` | 6 | 1 (dans test Kind) | — | 2 : `m3_would_schedule_only_after_json_valid_admit` (195), `m3_bind_cri_persists_pin_only_when_valid` (241) | 4 : (294, 349, 468, 519) |
| `apparatus_m4_desired_state_bridge.rs` | 3 | 1 (dans test Kind) | — | 1 : `m4_http_skips_non_active_and_missing_digest` (262, stub local) | 2 : (328, 500) |
| `apparatus_m5_invoke_isolated.rs` | 1 | 1 (dans test Kind) | — | — | 1 : (449) |
| `apparatus_p4_t10_platform_workload.rs` | 4 | 2 (dans tests Kind, dont `t10_kubelet_pulls_pinned_cri_from_zot` 390 qui pousse via fixture skopeo/crane) | — | — | 4 : (123, 310, 390, 595) |
| `apparatus_p4_t11_runtime.rs` | 7 | 1 (helper `schedule_plugin` 196-278, appelé par tests Kind) | — | 3 : `t11_manifesto_src_has_no_k8s_…` (543), `t11_wget_oracle_is_http_only` (611), `t11_wget_missing_is_not_np_block` (624) | 4 : (449, 471, 499, 526) |
| **Total** | **31** | 11 appels (5 exécutables P2 dans t6+m1, 6 dans les tests Kind) | 2 | 9 | 15 |

Partition non chevauchante des 31 tests : **7 Docker requis exécutables P2** (5 transport CLI + 2 HTTP/fixtures seuls) + **9 sans Docker** (3 dans le t6 primaire, 6 dans le secondaire filtré) + **15 Kind defer** (§0) = 31. Le comptage binaire est par fichier, transport et Docker requis se chevauchent pour t6 (4 tests) et m1 (1 test).

- Aucun `#[ignore]`, aucun `#[cfg(unix)]`, aucun `env::set_var` dans `tests/` (greps exacts vides) — les 7 tests `cfg(unix)` connus sont unitaires (`--lib`, lot précédent), hors lot.
- t6 n'a **aucun** test Kind : fichier intégralement exécutable en P2. m1 idem.

### 1.5 Commandes et features (fixées depuis `Cargo.toml`, aucune supposition)

- `[[test]]` et `required-features` : t6 et m1 = `admit` (`Cargo.toml:35-37,54-56`) ; m3/m4/m5/t10/t11 = `admit,controller` (`:43-49,58-72`).
- **Piège noté** : avec un `required-features` non satisfait, cargo **saute silencieusement** la cible (warning seulement) — exactement le « skip silencieux » interdit. Les commandes ci-dessous portent donc les features exactes, et la preuve est le **comptage libtest rendu**, pas l'absence d'erreur.
- Cargo : un seul `cargo` à la fois, `--locked`, jobs=2 déjà en config (`.cargo/config.toml` = `[build] jobs = 2`, 2 lignes, rien d'autre — pas de RUSTFLAGS ni de `cfg(test)` injecté). Constat environnement : le `target-dir = E:\cargo-target\AIForAll` cité par AGENTS.md **n'est pas** dans ce fichier (vérifié lecture + `git show 1312a9d`) ; `E:\cargo-target\AIForAll` existe peuplé (artefacts antérieurs) mais le mécanisme est externe au dépôt. Écart doc/environnement mineur, signalé ; barrière inchangée : **ne pas modifier** `.cargo/config.toml`.

## 2. Décisions déjà tranchées (humaines — citées, non rouvertes)

- Oracles §Protocol G-17 du ledger (rappel) : jamais de mock/intercept sur le hop binaire ; host-bin (`APPARATUS_*_BIN`, ORAS 1.2.2 / Cosign 2.4.3 réels) **sinon** fallback docker sur images pinées — les deux sont de vrais binaires ; binaires hôtes absents = fallback acceptable ; preuve positive par hop, échec loud ; aucun skip silencieux ; tout FAIL = preuve ROUGE persistée avant tout correctif ; P2 n'écrit pas de code prod ; un test qui révèle un bug prod reste ROUGE (correctif = P8).
- Contrats du correctif credentials (`c2fe7172ccc4`) NON rouverts ici (décisions §2 du plan précédent) : argv sans secret, framing CRLF non tronquant, EOF avant wait, `DOCKER_CONFIG` enfant seulement, clés host+network, deadline enfant+joins (RP-1 close), sorties plafonnées, config owner-only + suppression fail-closed, fallback docker argv-safe id-exact. Ce lot ne réexamine pas ces contrats : il prouve leur **fidélité réelle** face aux binaires.
- Briefings M5–M8 du lot précédent : « fidélité réelle = G-17, defer » (correctness:82, security:96, tests:48, rust-perf:89). G-17 est la seule incertitude de fidélité — c'est ce que ce lot comble.
- Résidus connus hors axes (ledger) : S-1, S-2, RP-A, RP-B, N-1, N-2, 7 tests cfg(unix), gold path 0605, lab 0412, axe E2E P4–P7 — exclus.

## 3. Étape A — Matrice de risques du lot

Impact (gravité) × probabilité (churn, branches, catégories historiques) scorés séparément. Catégories historiques actives : « mock qui masque l'invariant » (toute la preuve du correctif est mockée à ce jour — 23 tests unitaires, zéro exécution réelle des binaires), preuve fausse (signature/verify). Churn faible sur `admit.rs`/`cli_transport.rs` depuis `c2fe717` (1 commit) — la probabilité vient de la **première exécution réelle**, pas du churn.

| Id | Comportement / contrat | Impact | Prob. | Score | Zone | Profondeur |
|---|---|---|---|---|---|---|
| R-01 | ORAS push + manifest fetch **réels** vs zot (framing CRLF réel, `--password-stdin`, auth htpasswd) | 5 | 3 | 15 | critique | IT t6×4 + m1, preuve zot-side |
| R-02 | Cosign sign + verify **réels** vs Transit+zot (auth `DOCKER_CONFIG` réellement lue par Cosign 2.4.3, clés host+network, env Vault) | 5 | 3 | 15 | critique | IT t6×4 + m1, négatif inclus |
| R-03 | Fallback `docker run` **réel** (cidfile, `-v :ro`, `-e`, DNS réseau user-defined, release id → stop/inspect/rm) | 4 | 3 | 12 | haut | IT (si mode fallback servi) |
| R-04 | Cohérence cible selon le mode : registry `127.0.0.1:{port}` vs `{conteneur}:5000`, vault host vs network (`admit.rs:291-296,535-540`) | 4 | 2 | 8 | moyen | IT + preuve mode |
| R-05 | Hooks `cfg(test)` inertes en IT (intercept/overrides absents du binaire IT) | 5 | 1 | 5 | moyen | structurel (§1.2) + preuve à l'exécution |
| R-06 | Négatif verify : tag `.sig` présent ≠ admission (crypto KO refusée loud) | 4 | 2 | 8 | moyen | `t6_verify_ko_with_sig_tag_refuses` |
| R-07 | Cleanup réel des conteneurs helpers (pas de résidu, id exact 64 hex) | 3 | 3 | 9 | moyen | IT + `docker ps -a` post-run |
| R-08 | Idempotence re-run (zot singleton + `docker_rm` résidu ; tag OCI mutable réécrit ; binaire séparé → conteneur recréé) | 2 | 3 | 6 | moyen | re-run t6 (buffer) |

Aucune contradiction avec le score PM (P1 : complexité 40, risque 28 — tâche de planification). En revanche la matrice révèle le risque **gouvernance** du lot (§0) : 15/31 tests inatteignables — signalé, board non modifié.

### Modes de défaillance (items ≥ 10)

| Id | Déclencheur | Blast radius | Détection actuelle | Mitigation actuelle | Gap |
|---|---|---|---|---|---|
| R-01 | ORAS 1.2.2 réel refuse le framing CRLF, l'auth htpasswd, ou le payload artifact-type ; push partiel | Enveloppe jamais poussée ou poussée corrompue : toute la chaîne admission (T10→M6) aveugle alors que les 23 tests unitaires sont verts | Aucune (aucune exécution réelle à ce jour) | Aucune | G-17.1/G-17.2 : première preuve réelle |
| R-02 | Cosign 2.4.3 réel ignore le schéma `config.json` (AuthConfig host/network), ou la clé Transit `hashivault://` ne matche pas l'engine réel | Signature créée mais invérifiable (admission bloquée), ou pire : verify passe avec une auth dégradée → preuve fausse | Briefing sécurité M7 : « si Cosign ignore le schéma, l'admission échoue » — non testé en réel | Aucune | G-17.3/G-17.4/G-17.5 : sign, verify positif, verify négatif |
| R-03 | `docker run` fallback : DNS réseau absent, montage `/auth:ro` non lisible en conteneur, cidfile non écrit, release en échec | Chaîne fonctionnelle en dev (binaires hôtes) et cassée en fallback (ou l'inverse) : parité jamais prouvée en réel | G-14 unitaire (mocks) uniquement | Aucune | G-17.9 : fallback réel de bout en bout |

### Passe heuristique (états / frontières / ordre / idempotence)

- **Frontières** : premier run (images oras/cosign/zot/openbao absentes → pull) ; Windows (`docker_host_path` canonise + strip `\\?\`, `admit.rs:571-579`) ; 127.0.0.1 vs alias DNS conteneur ; htpasswd push-only (négatif `t6_build_anonymous_push_forbidden`).
- **États** : binaires hôtes présents/absents (mode) ; zot/transit déjà up (singleton réutilisé) vs neuf ; conteneur résidu d'un run précédent (`docker_rm` ciblé).
- **Ordre** : push → fetch → sign → verify → tag `.sig` (séquence `admit.rs:298-306,434-455`) ; le tag `.sig` n'existe qu'après sign ; verify négatif exige la précondition tag présent (`t6_verify_ko:188-192`).
- **Idempotence** : re-run du même tag → tag OCI réécrit, nouveau run OK ; deux binaires d'IT successifs → conteneur zot recréé (singleton process-local, `docker_rm` gère le résidu).

## 4. Étape B — Inventaire des tests existants

- **Le lot est un lot d'EXÉCUTION** : les tests existent déjà (31, §1.4) et portent les oracles ; P2 n'écrit aucun test sauf découverte d'un gap à l'exécution (buffer, décision lead).
- Couverture du transport par les tests exécutables P2 :
  - `t6_admit_signer_pushes_envelope_and_signs` : les 4 hops positifs (push, fetch, sign, verify) — `signature_verified=true` exige le verify réel (`admit.rs:225-226`).
  - `t6_verify_ko_with_sig_tag_refuses` : négatif verify (clé Transit wrong `t6-wrong-verify-key`) **avec** précondition tag `.sig` présent (hop 5).
  - `t6_catalog_identity_…` / `t6_envelope_pins_…` : contrats de sortie (digest ≠ descriptor, `sha256:` 64 hex, pin CRI conservé).
  - `m1_reference_kv_pinned_envelope_on_zot` : chaîne complète avec **vraie image CRI construite** (docker build reference-kv + digest `docker inspect`), pas le CRI synthétique.
- Négatifs structurels déjà en place : `t6_build_anonymous_push_forbidden` (zot 401/403 anonyme), `t6_cri_pin_rejects_…` (refus avant tout CLI/Transit), `t6_docker_zot_openbao_start_or_fail_loud` (fixtures fail-loud).
- Tests orphelins / preuves périmées : aucun identifié dans le périmètre. Doublons : les 4 tests t6 réexécutent la même chaîne — valeur = contrats de sortie distincts (pas des doublons à supprimer).
- Les 15 tests Kind (§0) : preuves d'admission/schedule **existantes mais inatteignables** — defer tracé (G-17.10), pas des tests à écrire.

## 5. Étape C — Registre de gaps

Toutes les obligations sont **explicites** (oracles 1–7 de l'ordre de mission + Protocol G-17 du ledger), sauf G-17.8 (inférence structurelle confirmée par lecture, preuve d'exécution jointe à G-17.7). L'ancêtre est G-17 du plan précédent (defer) ; les enfants héritent de la numérotation.

| Gap | Obligation (source) | Intent de test / preuve positive | Preuve existante | État | Priorité | Niveau | Effort |
|---|---|---|---|---|---|---|---|
| G-17.1 | R-01 : ORAS push réel (explicite) | Enveloppe poussée : `push_and_sign_envelope` Ok + digest OCI `sha256:` 64 hex rendu ; fetch suivant le confirme chez zot | aucun (G-17 defer lot précédent) | GAP | critique | IT testcontainers | S (run) |
| G-17.2 | R-01 : ORAS manifest fetch réel (explicite) | Digest lu ≠ `descriptor_digest` (`t6_catalog_identity:216-225`, `m1:176-185`) ; fetch en échec = `cli_failed` loud | aucun | GAP | critique | IT | S |
| G-17.3 | R-02 : cosign sign réel (explicite) | Tag `.sig` présent chez zot (fetch OK via hop 5) ; sign en échec = loud | aucun | GAP | critique | IT | S |
| G-17.4 | R-02 : cosign verify réel positif (explicite) | `signature_verified == true` seulement si verify OK (`t6_admit_signer:168-171`) | aucun | GAP | critique | IT | S |
| G-17.5 | R-06 : cosign verify réel négatif (explicite) | Clé Transit wrong + tag `.sig` présent → `Err` contenant « cosign verify » (`t6_verify_ko:195-201`) | aucun | GAP | critique | IT | S |
| G-17.6 | Hop 5 : ORAS tag `.sig` (précondition de test, pas oracle crypto — docstring `admit.rs:485`) | Fetch `.sig` OK après sign ; sert de précondition à G-17.5 | aucun | GAP | haut | IT | S |
| G-17.7 | Mode transport prouvé (explicite, oracle 4) | Matrice §6.4 : (a) env `APPARATUS_*_BIN` enregistré avant run (vide = fallback **par construction**, `admit.rs:563-569`) ; (b) preuve runtime fallback = `docker events`/`docker images` montrant oras:v1.2.2 + cosign:v2.4.3 pendant la fenêtre ; (c) mode host = binaires présents + pas de helper containers. Si binaires hôtes présents : arm fallback en buffer (2e run t6 sans env) | aucun | GAP | haut | structurel + run | S |
| G-17.8 | Hooks `cfg(test)` inertes en IT (inférence confirmée, oracle 1) | Structure : §1.2 (7 sites, modèle Cargo `[[test]]`). Preuve d'exécution = G-17.1–G-17.5 (si l'intercept était compilé, zot ne recevrait rien). Aucun test dédié nécessaire | lecture (§1.2) | COVERED (structurel, confirmé à l'exécution par G-17.7) | moyen | structurel | — |
| G-17.9 | R-03 : fallback docker réel complet (explicite) | Chaîne complète via fallback : cidfile/id, release stop→inspect→rm sans résidu (`docker ps -a` post-run filtré sur les deux images pinées) ; si le mode servi est host-bin, item en buffer via l'arm fallback de G-17.7 | aucun (G-14 unitaire mocké seulement) | GAP | haut | IT | S |
| G-17.10 | **Admission** (reconcile/schedule) + transport DANS les ITs Kind : 15 tests nommés §0 (explicite, oracle 1 pour m3/m4/m5/t10/t11) | Bloqué structurellement (§0 : cluster protégé + IT jamais via Kind + runtime P2 Docker seul) | tests existants, non exécutables | **GAP-defer** — qui décide : lead (scope P2) + GO utilisateur (ré-ciblage fixture, déjà cadré P4/P5 au ledger) | — | IT Kind (post ré-ciblage) | — |
| G-17.11 | `push_platform_plugin_to_zot` (fixture skopeo/crane, `kind/mod.rs:544-599`) — hors transport `admit.rs` | Hors périmètre du lot (fixture-side, lié au pull kubelet) | — | **GAP-defer** avec G-17.10 (Kind) | — | — | — |

Règles respectées : chaque hop matériel a un nominal (G-17.1–G-17.4) et le hop verify a son négatif (G-17.5) ; un intent par invariant ; pas de doublon de comportement (les 4 runs t6 prouvent des contrats de sortie distincts) ; registre borné au lot.

**Faits / inférences / recommandations / décisions** : §1 = faits ; G-17.8 = inférence confirmée par lecture (modèle Cargo) avec preuve d'exécution déléguée à G-17.7 ; la matrice de mode et les commandes = désignation de ce plan ; le scope P2 face à la contradiction Kind = décision lead/GO (§0, §8) ; les oracles §2 = décisions humaines déjà prises.

## 6. Étape D — Critères d'entrée / sortie

### 6.1 Porte d'entrée (vérifiée par le lead avant P2)

- **Ce plan commité par le lead** après relecture. Exception ledger : contradiction structurelle non tranchable → question à l'utilisateur. Le constat §0 est tranchable côté scope (le lead choisit la recommandation transport-only) ; l'arbitrage « quand ré-cibler les 15 tests Kind » reste au GO.
- Base intégrée : `6f34b6d7` propre (porcelain = `?? .opencode/missions/` seulement) ; pin rustycog `41a86dba` immobile ; `.cargo/config.toml` intact.
- Slot Cargo Windows libre (un seul `cargo` à la fois, séquentiel — jamais parallèle avec autre travail).
- Docker Desktop : démarré **après** chargement de `.agents/skills/local-runtime-lifecycle/SKILL.md` ; registre append-only `.tmp/docker-resources-20261017.log` ouvert à la première ressource (id à la création, `safe_to_delete`).
- Données de test : déjà dans les fixtures (`SIGNER_USER`/`SIGNER_PASSWORD`, `ROOT_TOKEN`, `TRANSIT_KEY`, manifests inline) — rien à créer.
- Images requises (pull au premier run, fail-loud si pull échoue) : `ghcr.io/project-zot/zot:v2.1.8`, `openbao/openbao:2.6.2`, `ghcr.io/oras-project/oras:v1.2.2`, `gcr.io/projectsigstore/cosign:v2.4.3` + contexte `crates/apparatus-reference-kv` (build docker m1).
- Worktree P2 = **false** (décision lead §9.2, remplace le `true` du ledger) ; ce plan lu par chemin absolu.

### 6.2 Commandes exactes (oracle 3 — hôte Windows, ordre séquentiel, UN SEUL cargo)

```text
# 1) t6 — fichier complet, feature admit seule
cargo test --locked -p apparatus-operator --features admit --test apparatus_p4_t6_sign_registry -- --test-threads=1
# attendu : "test result: ok. 9 passed" (9/9, 0 failed)

# 2) m1 — fichier complet, feature admit seule
cargo test --locked -p apparatus-operator --features admit --test apparatus_m1_reference_kv_pin -- --test-threads=1
# attendu : "test result: ok. 1 passed"
```

Secondaire (buffer — compile `controller` lourde ; coupe en premier si capacité) :

```text
# 3) m3 filtré — 2 tests sans Docker ; les 4 tests Kind ne sont PAS exécutés (tracé G-17.10)
cargo test --locked -p apparatus-operator --features admit,controller --test apparatus_m3_install_fail_closed -- m3_would_schedule_only_after_json_valid_admit m3_bind_cri_persists_pin_only_when_valid --test-threads=1
# attendu : 2 passed / 0 failed / 4 filtrés (tracés)

# 4) m4 filtré — 1 test sans Docker ; 2 tests Kind non exécutés
cargo test --locked -p apparatus-operator --features admit,controller --test apparatus_m4_desired_state_bridge -- m4_http_skips_non_active_and_missing_digest --test-threads=1
# attendu : 1 passed / 2 filtrés

# 5) t11 filtré — 3 tests sans Docker ; 4 tests Kind non exécutés
cargo test --locked -p apparatus-operator --features admit,controller --test apparatus_p4_t11_runtime -- t11_manifesto_src_has_no_k8s_and_controller_is_not_tick t11_wget_oracle_is_http_only t11_wget_missing_is_not_np_block --test-threads=1
# attendu : 3 passed / 4 filtrés
```

- m5 et t10 : **non exécutés** (fichiers entiers Kind : 1/1 et 4/4 tests). Tracé G-17.10.
- Pas `--include-ignored`, pas `--all-features`, pas `cargo check` séparé, pas `--no-default-features` (inutile : `default = []`).
- `--test-threads=1` : coût négligeable (≤9 tests), supprime tout risque d'entrelacement sur le singleton Docker ; `#[serial]` est déjà en place dans les fichiers.

### 6.3 Timeouts et bornes (faits, §1)

- Enfant CLI : 120 s par appel (`CLI_DEADLINE`, `cli_transport.rs:23`), plafond sortie 1 MiB.
- Fixtures : startup 180 s + readiness bornés (zot 60×250 ms, openbao 60×200 ms) — fail-loud si dépassé.
- Transit reqwest : 10 s (`admit.rs:378`) ; clients fixtures : 5 s.
- Guidance wall-clock (pas une gate dure) : premier run froid (pulls + build m1) ≤ ~30 min par binaire ; runs tièdes ≤ ~10 min. Au-delà : interrompre, traiter en suspect (triage/ROUGE), jamais un pass silencieux.

### 6.4 Matrice de décision du mode transport (oracle 4)

| Condition à l'entrée de P2 | Mode servi | Cible registry | Cible Vault | Preuve du mode |
|---|---|---|---|---|
| `APPARATUS_ORAS_BIN`/`APPARATUS_COSIGN_BIN` définies + binaires existants | **host-bin** (vrais binaires hôte) | `127.0.0.1:{port zot}` (`admit.rs:291-296`) | `vault_addr_host` (`535-540`) | Valeurs env consignées avant run + absence de helper containers dans `docker events` |
| Env absentes/vides | **fallback docker** (vraies images pinées) | `apparatus_operator_test-zot:5000` | `vault_addr_network` | Env vide consignée + `docker events` (ou delta `docker images`) montrant `oras:v1.2.2`/`cosign:v2.4.3` pendant la fenêtre + artefacts zot réels |
| Les deux disponibles | host-bin par défaut ; **arm fallback en buffer** (2e run t6, env retirées du shell de la commande seulement) | les deux | les deux | Les deux preuves, run par run |

- Les deux modes satisfont « transport réel » (décision ordre : binaires hôtes absents = fallback acceptable). La parité des deux chemins a déjà été prouvée unitairement (G-14, lot précédent) ; ici seul le mode **servi** doit être prouvé, l'autre arm est du buffer.
- Indépendamment du mode : les artefacts zot (digest poussé + tag `.sig` + verify OK) prouvent qu'aucun intercept n'a court-circuité le hop (G-17.8).

### 6.5 Porte de sortie (auditée par P3 `test-reviewer`)

- t6 **9/9** et m1 **1/1** verts (premier run ou triage documenté) — preuve réelle par hop rendue : G-17.1→G-17.6 cochés avec les extraits (digest poussé, tag `.sig`, verify OK/KO).
- Preuve du mode transport rendue (G-17.7 : env + evidence Docker) et comptages libtest recopiés par IT.
- Les 15 tests Kind tracés **nominalement** dans le journal de ce fichier (append, §8.2) comme defer G-17.10 — jamais un pass ou skip silencieux.
- Secondaire (si retenu) : 6 tests filtrés m3/m4/t11 verts avec comptages « X passed / Y filtrés ».
- Aucun FAIL sans preuve ROUGE persistée (sortie complète, repro, file:line) **avant** tout correctif ; P2 n'écrit pas de code prod ; un test révélant un bug prod reste ROUGE jusqu'à P8.
- Aucune écriture de code → **pas de porte fmt/clippy** dans ce lot (réintroduite seulement si un lot correctif s'ouvre).
- Audit P3 rendu : PASS ou findings traités, références `G-17.<n>`.
- Cleanup Docker vérifié avant rendu (conteneurs zot/transit arrêtés, helpers inexistants, registre à jour).

## 7. Budget

- **Setup** (ligne propre) : Docker Desktop (skill lifecycle) + pulls des 4 images + build reference-kv (m1) + registre `.tmp/docker-resources-20261017.log`. Estimation 30–40 % du wall-clock du lot (pulls dominent au froid).
- **Planifié ~75 %** : t6 (9 tests) + m1 (1) + preuve mode (G-17.7) + comptages + trace defer nominative. Le noyau critique (G-17.1–G-17.5) tient dans t6+m1.
- **Buffer nommé 25 %** : arm fallback si host-bin servi (G-17.9) ; secondaire m3/m4/t11 filtrés (6 tests, compile controller) ; triage du premier run ; capture `docker events` ; RED protocol.
- Coupe au-delà de la capacité, par risque × effort : le secondaire part avant tout item critique ; l'arm fallback part avant le secondaire ; jamais le buffer, jamais un item critique.
- Recommandation au lead : le noyau critique (t6+m1) est suffisant pour combler G-17 ; le secondaire n'améliore pas la preuve transport, il sert l'honnêteté des comptages par fichier.

## 8. Incertitudes et décisions restant à prendre

1. **Contradiction Kind (§0)** — escalade. Options : (a) scope P2 = transport-only (recommandé, conforme aux barrières existantes) ; (b) avancer le ré-ciblage du fixture Kind avant P2 (modifie le graphe, nouveaux lots — décision lead/GO) ; (c) autre. **Décide** : lead (scope) + GO utilisateur (ré-ciblage, déjà cadré P4/P5 au ledger).
2. **Écart doc/environnement** : `target-dir E:` cité par AGENTS.md absent du `.cargo/config.toml` committé (mécanisme externe au dépôt). Fait noté §1.5 ; barrière inchangée (ne pas modifier le fichier). **Décide** : personne (constat documentaire, à recaler par le lead un jour).
3. **Binaires hôtes présents ?** P2 constate à l'entrée et suit §6.4. Les deux modes valides ; l'arm secondaire est du buffer.
4. **Portée de « 7 IT vertes »** : si le lead retient (a), la porte de sortie du lot devient « t6+m1 vertes + 15 defer tracés » (§6.5) — le libellé de l'ordre ne tient qu'après le ré-ciblage (P5). À reconcilier dans le board par le lead.

## 9. Décisions du lead (2026-10-17, après relecture complète)

Ancres spot-vérifiées par le lead avant commit : `COSIGN_IMAGE` admit.rs:30, `ORAS_IMAGE` :32,
`host_tool`/`forced_host` :563/:565, 16 occurrences `KindCluster::ensure` (15 dans le lot + m6:59
hors lot, axe 2), `Cargo.toml` required-features (t6/m1 = `admit` ; m3/m4/m5/t10/t11 =
`admit,controller`), `.cargo/config.toml` = `[build] jobs = 2` seulement.

1. **§0 tranche — scope P2 = transport-only (option a).** Conforme à l'ordre de mission
   (« IT testcontainers, jamais Kind ») et aux barrières (cluster protégé `kind-apparatus-p4-it`
   intouché, instance Docker seule). Le ré-ciblage du fixture Kind (15 tests, G-17.10 + G-17.11)
   reste cadré P4/P5 au **GO utilisateur** — jamais avancé sans lui. Conséquence (§8.4) : la
   porte de sortie réelle de ce lot est « t6 9/9 + m1 1/1 + preuve mode (G-17.7) + 15 defer
   tracés nominalement » ; le libellé « 7 IT vertes » de l'ordre ne tient qu'après le
   ré-ciblage P5. Aucun skip silencieux : G-17.10 reste un defer tracé, pas un pass.
2. **§6.1 amendé — worktree P2 = false.** Le lot est une exécution pure : aucune écriture de
   repo attendue, la preuve passe par le handoff du worker (comptages, extraits, artefacts zot,
   preuve Docker) que le lead persiste dans ce plan (Journal) et le ledger. Évite un merge de
   branche sans diff (piège `worktree` du lot précédent). Toute écriture involontaire = stop
   et rapport immédiat au lead.
3. **§8.2 : constat documentaire non corrigé dans ce lot.** `target-dir E:\cargo-target\AIForAll`
   cité par AGENTS.md n'est pas dans le `.cargo/config.toml` committé ; barrière inchangée
   (ne pas modifier le fichier) ; mécanisme externe au dépôt, présent et peuplé. À recaler un
   jour par le lead, hors axes.
4. **Binaires hôtes (§8.3) : décision P2 à l'entrée**, selon la matrice §6.4 ; les deux modes
   valident « transport réel ». Buffers dans l'ordre de coupe §7.
5. **Mode servi = fallback docker (P2, G-17.7 COVERED).** Env `APPARATUS_*_BIN` absentes à
   l'entrée (consignées), binaires absents du PATH → fallback par construction. Preuves : delta
   images `oras:v1.2.2` (33,8 MB) + `cosign:v2.4.3` (177 MB), 16 + 17 conteneurs helpers
   create→start→die→destroy (un par hop, connect `apparatus_operator_test-net`), `docker ps -a`
   post-run filtré sur les images pinées = **zéro résidu** → **G-17.9 prouvé par le mode servi
   lui-même** (release cidfile id, stop/inspect/rm). Arm host-bin impossible en l'état (aucun
   binaire hôte) ; re-arm reste du buffer le jour où des binaires hôtes sont installés.
6. **ROUGE P2 — une seule famille, test-fixture (tracée, aucun correctif par le worker).**
   Helpers `repo_root()` = `CARGO_MANIFEST_DIR + ".."` résolvent `workers/` au lieu de la
   racine workspace depuis `f060d47` (« refactor(layout) », 2026-10-03, ancêtre de
   `6f34b6d7`). Paniques à 0,00 s, **avant tout Docker** — pas un bug prod transport (t6 9/9
   couvre G-17.1→6). Sites vérifiés lead :
   - `tests/apparatus_m1_reference_kv_pin.rs:25-31` — vrai manifeste
     `crates/apparatus-reference-kv/apparatus.toml` (existe, 402 B) ;
   - `tests/apparatus_m3_install_fail_closed.rs:36-46` — même helper (panic via
     `m1_descriptor_and_report()` :64-65, 2 tests filtrés) ;
   - `tests/apparatus_m4_desired_state_bridge.rs:37-46` + `:123` (contexte docker build →
     pointer `crates/apparatus-reference-kv`) ;
   - `tests/apparatus_p4_t11_runtime.rs:83` — vrai `services/Manifesto` (tick.rs réel
     `services/Manifesto/infra/src/apparatus_runtime/tick.rs`) + double préfixe
     `workers/workers/…` à signaler à `:583` (chemin réel
     `workers/apparatus-operator/src/controller.rs`).
   Correctif : lot **P2-fix** (test-engineer, worktree isolé, commite lui-même) — harmoniser
   les helpers vers la vraie racine workspace, aucune dilution d'invariant (les assertions
   restent des cibles réelles), aucun `src/`, aucune doc, plan inchangé. Attendu post-fix :
   m1 **1/1** ; secondaire filtré m3 **2/2**, m4 **1/1**, t11 **3/3**. t10 : statique dans le
   lot ; fixer son helper **seulement si** même pattern `repo_root()`, sans l'exécuter (Kind).
7. **Porte P3** : après merge de P2-fix + reprise des comptages par le lead sur master
   (m1 + secondaire), la porte §6.5 est complète et P3 (correctness + test-reviewer) part.
   t6 9/9 reste la preuve transport de référence (aucun re-run t6 exigé).




## Journal des mises à jour

- 2026-10-17 : création du plan (P1, base `6f34b6d7`, pin `41a86dba`). Contradiction Kind escaladée (§0, 15/31 tests). Matrice R-01…R-08, registre G-17.1…G-17.11, commandes exactes (§6.2), matrice de mode (§6.4), portes et budget. Aucun runtime, aucun cargo, aucun code écrit.
- 2026-10-17 : audit lead complet (ancres spot-vérifiées, §9). Scope P2 tranché = transport-only (a) ; worktree P2 = false ; constat doc/environnement §8.2 maintenu hors lot. Plan commité par le lead avant toute admission P2.
- 2026-10-17 (après P2 exécutée) : t6 **9/9** ; preuves zot réelles par hop (catalogue, artifactType, digest `sha256:b707d331…`, 4 tags `.sig` HTTP 200, verify positif + négatif) + preuve mode = fallback docker (G-17.7) ; G-17.9 zéro résidu helper. **ROUGE** une famille (helpers périmés f060d47) sur m1 + secondaire filtré — tracé et correctif cadrés §9.6 ; 15 tests Kind non exécutés (G-17.10 nominatif). Registre `.tmp/docker-resources-20261017.log` (10 entrées), evidence `.tmp/g17-*.log` + `g17-docker-events.txt` ; tiers `aiforall-local-control-plane` Up, non possédé, intouché.
- 2026-10-17 (lead, post-P2) : décisions §9.5–§9.7 — mode fallback COVERED, ROUGE tracé + lot correctif P2-fix (test-only, 4 fichiers, t10 en statique conditionnelle) avant la porte P3.
- 2026-10-17 (incident P2-fix) : première admission pool-3 échouée **provider.auth** (clé opencode-go expirée ; worktree vérifié vierge, zéro commit ; recovery = décision lead, clé renouvelée par l'utilisateur). Réadmission automatique par le dispatcher sur pool-4 (session fresh, même contenu autoporteur). pool-2 non réutilisé (tâche self-suffisante ; interférer = churn).
- 2026-10-17 (P2-fix exécutée par pool-4, commit `1e91353` sur sa branche) : fix test-only **5 fichiers** : m1 `:25-31` (+ `crates/`), m3 `:36-46`, m4 `:37-46` + contexte build `:123`, t11 `:83` + `:583` éliminé par construction, et **m5 statique corrigé** (`:62-68`, + `:194/:199` exacts) — Kind defer, jamais exécuté. t10 : aucun helper de la famille (non touché) ; t6 : package-relatif, hors famille (non touché). Attendus post-fix atteints dans le worktree. Deux écarts bornés documentés : `.tmp/` créé dans le worktree (capture) ; hook `.githooks/pre-commit` en **os-206 préexistant** (ligne trop longue, hors lot) → commit `--no-verify` avec preuve équivalente : `rustfmt --check --edition 2021` sur les 5 fichiers = exit 0 (intent préservé, hook non modifié).
- 2026-10-17 (lead, merge + reprise comptages sur MASTER) : team_merge pool-4, squash committé **`665110c`** — diff vérifié : exactement 5 fichiers `tests/`, aucun `src/`, gitlink `41a86dba` immobile. Comptages master (logs `.tmp/g17-p2fix-master-{m1,m3,m4,t11}.log`) : **m1 1 passed/0 failed** (9,62 s), **m3 2 passed/4 filtered** (0,01 s), **m4 1 passed/2 filtered** (0,01 s), **t11 3 passed/4 filtered** (0,07 s), tous exit 0. t6 9/9 (preuve P2, fichier non touché). ROUGE famille éteinte : **porte de sortie §6.5 complète** → P3 publiée (correctness puis test-reviewer, briefings persistés).
- Statique même famille HORS périmètre (constaté par pool-4, non touché, arbitrage lead) : `apparatus_m2_valid_persist.rs:20-25`, `apparatus_p4_t12_gate_regression.rs:13-14` (+ usages :70/:79/:178/:209/:380-442), `apparatus_p4_t2_scaffold.rs:16-19`, `apparatus_p4_t7_admission.rs:73-76`, `tests/fixtures/chain.rs:49-50` (consommé par `m6_e2e_chain` → **prérequis du ré-ciblage axe 2, à traiter dans le plan P4**) ; fixtures package-relatives zot/kind correctes (`zot/mod.rs:101`, `kind/mod.rs:73/323/547/1097` — ne pas toucher). m2/t12/t2/t7 : résidus structuraux hors axes de cette mission → tracés, décision utilisateur.

