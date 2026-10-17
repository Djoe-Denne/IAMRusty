# Plan QA — credentials opérateur (M2, apparatus-operator)

- **Lot** : M2 du ledger `.cursor/handoffs/20261017-credentials-0412-mission.md` (section M2).
- **Propriétaire** : `qa-planner` (mission `team_mvc7mvuh_0000_znipio`, tâche `task_mvc95wio_0001_tgbd8w`). Implémentation : M4 (`hard-implementer`). Audit : M8 (`test-reviewer`), ses findings référenceront `G-<n>`.
- **HEADbase vérifié le 2026-10-17** : `329fec3424006f4e903ad7dfdef60b62ab2abe25` (`git rev-parse HEAD`). Porcelain parent : `?? .opencode/missions/` seulement (mission non commitée, hors lot).
- **Périmètre** : `workers/apparatus-operator/src/admit.rs` (5 sites + `run_cli` + `redact`/`cli_failed` + cleanup config) et le module de tests `admit::cli_transport_tests` (n'existe pas encore à la base).
- **Gate de sortie (citée, non lancée par ce plan)** :
  `cargo test --locked -p apparatus-operator --no-default-features --features admit --lib admit::cli_transport_tests -- --test-threads=1`
- **Niveaux** : pin ADR 0200–0202 (`docs/adr/README.md` : 0200 IT infra réelle Implemented, 0201 mocks sortant seulement Implemented, 0202 queues opt-in Partial). Ce lot reste **unitaire (`--lib`)** par décision du ledger : aucun registry, Kind, daemon Docker ni binaire téléchargé. Aucune stratégie de test écrite possédée par des humains n'a été trouvée au-delà des ADR : l'absence est nommée, pas comblée.

## 1. Faits vérifiés (file:line, base `329fec342400`)

- Pins : `COSIGN_IMAGE = gcr.io/projectsigstore/cosign:v2.4.3` (`admit.rs:24`), `ORAS_IMAGE = ghcr.io/oras-project/oras:v1.2.2` (`admit.rs:26`).
- Cinq sites passent le mot de passe registry en argv via `run_cli` (`admit.rs:581-623`, `Command::output()` : sans stdin dédié, sans deadline, sans plafond de sortie) :
  - ORAS push `--password` : `admit.rs:327-328`, appel `334-341` ;
  - ORAS manifest fetch `--password` : `admit.rs:352-353`, appel `357-364` ;
  - Cosign sign `--registry-password` : `admit.rs:471-472`, appel `477-484` ;
  - Cosign verify `--registry-password` : `admit.rs:519-520`, appel `525-532` ;
  - ORAS tag `.sig` `--password` : `admit.rs:566-567`, appel `570-577` (`signature_tag_exists`).
- `cli_failed` (`admit.rs:681-687`) recopie stdout/stderr ; `redact` (`admit.rs:690-695`) ne masque que le token Vault. Un mot de passe registry en sortie fuit.
- `VAULT_TOKEN`/`VAULT_ADDR`/`TRANSIT_SECRET_ENGINE_PATH`/`COSIGN_YES` passés en env enfant Cosign (`admit.rs:506-511`, sign `460-461`) — canal existant à conserver.
- Double transport dans `run_cli` : binaire hôte via `ORAS_BIN_ENV`/`COSIGN_BIN_ENV` (`host_tool`, `admit.rs:625-627`), sinon `docker run --rm --network` (+ `-e` par env, + `-v`/`-w` si workdir) (`admit.rs:607-617`).
- `Cargo.toml:9-12` : `default = []`, `admit = ["dep:reqwest", "dep:tokio"]`. Le module `admit::cli_transport_tests` n'existe pas : **aucun `#[cfg(test)]` dans `admit.rs`** (grep exact). Tests existants du crate, hors périmètre : `admission.rs:595`, `controller.rs:731/1248/1444`.
- `DOCKER_CONFIG` figure dans `PRIVILEGED_IDENTITY_ENV` (`build.rs:15-29`) : le worker de build refuse de tourner si elle est définie dans **son** environnement. Aucune contradiction avec un usage enfant-scopé côté admit.
- `tests/fixtures/kind/mod.rs:787,815` utilise `DOCKER_CONFIG=/cfg` (IT hors gate).
- Historique : `git log -S "--password-stdin" -- admit.rs` vide (ledger, revérifié par lecture) ; dernier commit touchant `admit.rs` : `aa985cd` 2026-10-09, 3 commits depuis le 10-03 — churn modéré.
- Gate M4 : `--test-threads=1` obligatoire. Inférence d'ingénierie (à confirmer à l'implémentation) : les tests positionnent `ORAS_BIN_ENV`/`COSIGN_BIN_ENV`, variables d'environnement **globales au processus** ; le parallélisme les ferait racer.

## 2. Décisions déjà tranchées (humaines — couvertes, non rouvertes)

Les oracles de la section M2 du ledger sont des décisions : cinq sites sans valeur de mot de passe en argv (username et références restent) ; ORAS v1.2.2 `--password-stdin` avec suffixe CRLF sans tronquer un mot de passe déjà terminé par CR/LF, writer stdin fermé avant l'attente ; Cosign v2.4.3 sans password-stdin, auth par `DOCKER_CONFIG` → répertoire neuf, `config.json` `AuthConfig` username/password, clés host **et** network, sans `credsStore`, `credHelpers`, `docker login` ni `COSIGN_REGISTRY_PASSWORD` ; `VAULT_TOKEN`/`VAULT_ADDR` inchangés ; `run_cli` couvre binaire hôte et `docker run` sous un même contrat ; deadline unique enfant + joins (régression RP-1) ; sorties plafonnées, `cli_failed` non adversarial, spawn générique ; config privée owner-only, suppression fail-closed, cleanup en échec = échec ; fallback Docker sans secret en argv, id exact, sans `rm -f`/`-v`/prune/adoption par nom ; gate sans ressource externe, callbacks Docker mockés y compris à Drop.

## 3. Étape A — Matrice de risques du lot

Impact (gravité) × probabilité (churn, branches, catégories historiques) scorés séparément. Churn modéré de `admit.rs` (fait, §1) et correctif neuf jamais atterri (`git log -S` vide) élèvent la **probabilité**, pas l'impact. Catégories historiques du dépôt actives ici : RP-1 (deadline bornant seulement `try_wait`, bloqué en revue rust-perf 10/10), mock qui masque un invariant.

| Id | Comportement / contrat | Impact | Prob. | Score | Zone | Profondeur |
|---|---|---|---|---|---|---|
| R-01 | Mot de passe en argv, sites ORAS (push, fetch, tag) | 5 | 4 | 20 | critique | nominal + négatif par site |
| R-02 | Mot de passe en argv, sites Cosign (sign, verify) | 5 | 4 | 20 | critique | nominal + négatif par site |
| R-03 | Framing stdin ORAS : suffixe CRLF, non-troncation, EOF avant wait | 4 | 3 | 12 | haut | nominal + frontières |
| R-04 | Auth Cosign par `DOCKER_CONFIG` (schéma, clés host+network, négatifs config) | 5 | 4 | 20 | critique | nominal + négatifs |
| R-05 | Canal `VAULT_TOKEN`/`VAULT_ADDR` inchangé | 3 | 2 | 6 | moyen | nominal |
| R-06 | Parité des deux transports (`host_tool` vs `docker run`) | 4 | 4 | 16 | haut | nominal ×2 + négatif |
| R-07 | Deadline unique enfant + joins stdin/stdout/stderr (RP-1) | 5 | 4 | 20 | critique | régression bornée |
| R-08 | Sorties plafonnées ; `cli_failed` ne recopie pas stdout/stderr adversariaux | 4 | 3 | 12 | haut | négatif + frontière |
| R-09 | Erreurs de spawn génériques | 2 | 3 | 6 | moyen | négatif |
| R-10 | Répertoire config neuf, owner-only, suppression fail-closed, cleanup en échec = échec | 4 | 3 | 12 | haut | nominal + erreurs + unwind |
| R-11 | Fallback `docker run` argv-safe (id exact, pas `rm -f`/`-v`/prune/adoption) | 4 | 3 | 12 | haut | négatif structurel |
| R-12 | Isolation de la gate (aucune ressource externe, mocks à Drop) | 3 | 2 | 6 | moyen | structurel |

Aucune contradiction avec le score PM (M2 : complexité 40, risque 28, lot de préparation sans implémentation) : la matrice porte la surface du correctif, le score PM porte la tâche de planification. Rien à signaler au board.

### Modes de défaillance (items ≥ 10)

| Id | Déclencheur | Blast radius | Détection actuelle | Mitigation actuelle | Gap |
|---|---|---|---|---|---|
| R-01 | Invocation ORAS avec password en argv (3 sites) | Secret visible dans la table de processus hôte, logs d'erreur, historiques ; registre P4 compromis | Aucune (aucun test du module) | Aucune | Comportement actuel = le bug ; tests absents |
| R-02 | Invocation Cosign avec `--registry-password` (2 sites) | Idem R-01 + chemin de signature/verify | Aucune | Aucune | Idem |
| R-03 | Mot de passe finissant `\r`, `\n`, `\r\n`, ou vide ; writer non fermé avant wait | Auth registry en échec ou EOF jamais atteint (`io.ReadAll` attend) → blocage | Aucune | Aucune | Framing neuf : cas de bord jamais exercés |
| R-04 | `config.json` mal formé, clé host ou network absente, `credsStore` résiduel | Cosign ignore l'auth → échec loud, ou fallback cred helper → fuite | Aucune | Aucune | Schéma docker `AuthConfig` difficile, double clé host/network |
| R-06 | Divergence hôte/`docker run` (argv, stdin, config) | Fonctionne en dev (binaire hôte), casse en CI/livraison (conteneur) ou l'inverse | Aucune | Aucune | Deux chemins de code dans `run_cli` |
| R-07 | Héritier de pipe garde stdout ouvert ; child zombie ; join sans borne | Admission bloquée indéfiniment, worker coincé | Aucune (RP-1 : `try_wait` seul déjà refusé en revue) | Aucune | Régression RP-1 jamais écrite ; risque de réintroduire le défaut |
| R-08 | CLI adverse (sortie énorme, secret encodé dans stderr) | Mémoire non bornée ; secret recopié dans `AdmitError` → logs | `redact` masque le token Vault seulement | `redact` | Plafond absent ; registry password non couvert par la rédaction |
| R-10 | Crash, erreur CLI, panic/unwind pendant l'appel | Fichier mot de passe en clair persistant sur disque | Aucune | Aucune | Chemins d'erreur et Drop jamais exercés |
| R-11 | `docker run` avec secret en argv, ou `rm -f`/`-v`/adoption par nom | Le fallback défait le correctif ; violation infra-safety | Aucune | Aucune | argv `docker run` jamais asserté |

### Passe heuristique (états / frontières / ordre / idempotence)

- **Frontières** : mot de passe vide ; finissant `\r` ; `\n` ; `\r\n` ; contenant le token Vault en sous-chaîne (interaction `redact`) ; long (plafond de sortie) ; bytes non-UTF8 (lecture lossy, pas de panique).
- **États** : stdin ouvert vs fermé ; enfant vivant vs zombie ; répertoire config présent/absent avant suppression ; Drop pendant unwind.
- **Ordre** : writer fermé **avant** l'attente (EOF de `io.ReadAll`) ; suppression après l'appel même en erreur ; une seule deadline au niveau parent.
- **Idempotence** : double suppression sans panic ; cleanup rejoué sur chemin d'erreur ne masque pas l'erreur d'origine ; tests séquentiels (`--test-threads=1`) du fait des env globales.

## 4. Étape B — Inventaire des tests existants

- `admit.rs` : **aucun test** (grep `cfg(test)` exact : 0 occurrence). Toute obligation ci-dessous part de `GAP`.
- Tests existants hors périmètre : `admission.rs:595`, `controller.rs:731/1248/1444` — pas de preuve réutilisable sur argv/stdin/config/deadline.
- `tests/fixtures/kind/mod.rs:787,815` (`DOCKER_CONFIG=/cfg`) : IT hors gate, ne prouve rien sur ce lot — pas comptée comme couverture.
- Aucun test orphelin ni preuve périmée identifié **dans le périmètre** (le périmètre n'a pas de tests du tout).
- Niveau réellement possible ici : unitaire in-process avec mocks (callbacks Docker) et enfant factice de plateforme pour la sémantique pipe/EOF ; conforme au pin ADR 0201 (mocks sortants seulement) puisque tout collaborateur externe est sortant et mocké, et à la décision du ledger (zéro ressource externe).

## 5. Étape C — Registre de gaps

Toutes les obligations sont **explicites** (section M2 du ledger, décisions humaines) sauf **G-6 (inférée)** : une inférée n'est pas une spec, elle attend la confirmation de M4/lead.

| Gap | Obligation (source) | Intent de test | Preuve existante | État | Priorité | Niveau | Effort |
|---|---|---|---|---|---|---|---|
| G-1 | R-01 : 3 sites ORAS sans valeur de mot de passe en argv (explicite) | argv push/fetch/tag : `--password` absent, `--password-stdin` présent, `--username` et refs conservés | aucun (module absent) | GAP | critique | unit | M |
| G-2 | R-02 : 2 sites Cosign sans `--registry-password` (explicite) | argv sign/verify : flag absent, `--registry-username` conservé | aucun | GAP | critique | unit | M |
| G-3 | R-03 : framing stdin ORAS (explicite) | payload exact = password + CRLF ; cas bord `\r`, `\n`, `\r\n`, vide → non tronqués | aucun | GAP | haut | unit | M |
| G-4 | R-03 : writer fermé avant l'attente (explicite) | le mock enfant observe EOF avant de rendre ; un enfant sans EOF finit en échec borné (avec G-8), jamais en hang | aucun | GAP | haut | unit | M |
| G-5 | R-04 : schéma `DOCKER_CONFIG` (explicite) | `config.json` : `AuthConfig` username/password sous la clé host **et** la clé network ; répertoire neuf par appel | aucun | GAP | critique | unit | M |
| G-6 | R-04 : `DOCKER_CONFIG` passée par `Command::env` enfant, env parent non polluée (**inférée** de `build.rs:15-29` + sémantique env Rust) | env parent inchangée après appel ; `PRIVILEGED_IDENTITY_ENV` non déclenchée | aucun | GAP (inférée — confirmation M4/lead) | critique | unit | S |
| G-7 | R-04 : négatifs config (explicite) | aucun `credsStore`, aucun `credHelpers`, aucun `docker login`, aucune var `COSIGN_REGISTRY_PASSWORD` | aucun | GAP | critique | unit | S |
| G-8 | R-07 : deadline unique enfant + joins, régression RP-1 (explicite) | enfant qui ne termine pas + héritier de pipe maintenant stdout ouvert → échec borné (pas de hang) ; durée < borne assertée ; joins couverts | aucun | GAP | critique | unit | M |
| G-9 | R-08 : sorties plafonnées, `cli_failed` non adversarial (explicite) | stdout/stderr géants → message plafonné ; secret encodé dans la sortie → non recopié ; token Vault toujours masqué (régr. de `redact`) | aucun | GAP | haut | unit | M |
| G-10 | R-09 : erreurs de spawn génériques (explicite) | binaire introuvable → message générique sans argv ni secret | aucun | GAP | moyen | unit | S |
| G-11 | R-10 : suppression fail-closed, cleanup en échec = échec (explicite) | répertoire absent après succès, après erreur CLI et après unwind (garde Drop) ; échec de suppression → erreur remontée, pas retry silencieux ; double suppression sans panic | aucun | GAP | haut | unit | M |
| G-12 | R-10 : perms owner-only 0700/0600 (explicite) | assertion `cfg(unix)` sur mode du répertoire et du fichier | aucun | **GAP-defer** — cette gate Windows ne peut pas prouver un mode Unix ; qui décide : `architecte` (runner Unix CI), hors jalon M4. Recommandation : écrire l'assertion `cfg(unix)` quand même, coût S | haut | unit cfg(unix) | S |
| G-13 | R-05 : canal Vault inchangé (explicite) | `VAULT_TOKEN`/`VAULT_ADDR`/`TRANSIT_SECRET_ENGINE_PATH` présents dans l'env enfant Cosign (les deux transports), valeurs non dumpées dans les erreurs | aucun | GAP | moyen | unit | S |
| G-14 | R-06 : parité transports (explicite) | même contrat argv/stdin/config asserté pour binaire hôte et chemin `docker run` | aucun | GAP | haut | unit | M |
| G-15 | R-11 : fallback docker argv-safe (explicite) | argv `docker run` sans secret ; id exact (pas de nom) ; pas de `rm -f`, `-v`, prune ; callbacks mockés y compris à Drop | aucun | GAP | haut | unit | M |
| G-16 | R-12 : isolation de la gate (explicite) | aucun processus docker/registry/réseau/binaire téléchargé ; Drop des mocks sans effet externe (enfant factice = shell de plateforme ou mock in-process) | aucun | GAP | moyen | structurel | S |
| G-17 | Fidélité réelle ORAS v1.2.2 / Cosign v2.4.3 (inférence de couverture, pas une spec) | IT réelle consommant le framing stdin et le schéma `config.json` | aucun | **GAP-defer** — ledger exclut tout runtime de ce lot ; qui décide : lead/utilisateur, après M8, hors mission. Signalé comme incertitude : rien ne prouve aujourd'hui la fidélité réelle des deux pins | — | IT hors lot | — |

Règles respectées : au moins un nominal et un négatif par obligation matérielle (G-1/G-2/G-5/G-7/G-9/G-11/G-15 portent les négatifs) ; un intent par invariant ; pas de doublon de comportement. Le registre est borné au lot : pas de couverture du reste du crate.

**Faits / inférences / recommandations / décisions** : §1 = faits ; G-6 et G-17 = inférences (à confirmer) ; le niveau et l'oracle de chaque G = désignation de ce plan ; les oracles §2 = décisions humaines déjà prises ; l'écriture `cfg(unix)` de G-12 = recommandation.

## 6. Étape D — Critères d'entrée / sortie

**Entrée** (porte, vérifiée par le lead avant M4) :
- Base intégrée : `329fec342400` + diff M4 sur `admit.rs` ; arbre propre hors fichiers du lot et du dossier mission.
- Slot Cargo Windows libre (un seul `cargo` à la fois) ; `jobs = 2` et `target-dir E:\cargo-target\AIForAll` intacts. Pas de `cargo check` séparé : la seule commande Cargo de M4 est la gate.
- Fixtures : aucune externe — mocks in-tree (callbacks Docker + enfant factice de plateforme). Runtime local (Docker/Kind/WSL) **non réveillé**.
- Données de test décidées (jeu fixé par ce plan) : `s3cret` ; `ends-cr\r` ; `ends-lf\n` ; `ends-crlf\r\n` ; vide ; mot de passe contenant le token Vault en sous-chaîne ; mot de passe long pour le plafond.

**Sortie** (porte, auditée par M8) :
- Gate verte au premier run ou après triage documenté : `cargo test --locked -p apparatus-operator --no-default-features --features admit --lib admit::cli_transport_tests -- --test-threads=1`. Pas `--include-ignored`, pas `controller`, pas `--all-features`.
- Chaque G d'état GAP couvert par au moins un test du module conforme à son oracle, ou basculé accept/defer tracé dans ce fichier (append, pas de fork).
- Audit `test-reviewer` (M8) rendu : PASS ou findings traités ; fmt/clippy sans régression.
- Aucun test de la gate ne spawn docker, n'ouvre de socket registry et ne télécharge de binaire ; Drop des mocks vérifié sans effet externe.

**Hors sortie** (rappels ledger) : pas de promotion ADR 0412/0605, pas de commit par ce lot (le lead commite ce plan seul, avant le worktree M4), gitlink `rustycog` immobile.

## 7. Budget

- **Setup** (ligne propre) : harnais de mocks — callback Docker mocké y compris Drop, enfant factice de plateforme pour pipe/EOF, écriture `config.json` témoin. Estimation : 30–40 % de l'effort total du lot M4 ; c'est le poste principal.
- **Planifié ~75 %** : G-1 à G-11, G-13 à G-16 (nucleus critique + haut).
- **Buffer nommé 25 %** : cas adversariaux supplémentaires découverts au triage, variante Windows de G-12, premier run non vert (triage), ajustements d'oracle demandés par les revues M5–M7.
- Coupe au-delà de la capacité : par risque × effort — G-10 (moyen) et la variante buffer de G-12 partent avant tout item critique ; aucun buffer sacrifié, aucun item critique coupé pour garder des moyens.

## 8. Décisions du lead (2026-10-17)

Tranchées par le lead après relecture. M4 les applique. Pas d'escalade utilisateur.

1. **G-6 confirmée.** `DOCKER_CONFIG` passe seulement par l'environnement de l'enfant (`Command::env`, ou `-e` de `docker run`). `std::env::set_var` sur le processus parent est interdit.
2. **G-12.** La gate Windows prouve l'ACL owner-only du répertoire et du fichier. L'assertion `cfg(unix)` 0700/0600 est écrite dans le même module ; elle ne conditionne pas la gate Windows. Pas d'attente d'un runner Unix pour démarrer M4.
3. **G-17 reste defer**, hors mission. Aucune IT réelle ORAS ou Cosign dans ce lot.
4. **Mot de passe vide.** Le payload stdin est le suffixe CRLF seul. Pas de refus explicite.
5. **Un seul Cargo.** M4 lance la gate, puis s'arrête. Pas de `cargo check` préalable.

## Journal des mises à jour

- 2026-10-17 : création du plan (M2, base `329fec342400`). Matrice R-01…R-12, registre G-1…G-17, portes entrée/sortie, budget. Aucun test écrit, aucune gate lancée, aucun commit.
- 2026-10-17 : décisions du lead en §8. G-6 confirmée, G-12 scindée (ACL Windows dans la gate, mode Unix en `cfg(unix)`), G-17 defer, mot de passe vide accepté, un seul Cargo.