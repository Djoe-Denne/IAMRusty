# Review briefing — correctness transport CLI credentials (M5)

```yaml
reviewer: correctness
written_at: 2026-10-17T13:25Z
branch: master
head_sha: c2fe7172ccc4
dirty: false # hors .opencode/missions/ (mission non commitée, hors lot)
scope_slug: operator-cli-credentials
verdict: PASS_WITH_NOTES # = GO_WITH_COMMENTS du canevas ; aucun BLOCK
related_paths:
  - workers/apparatus-operator/src/admit.rs
  - workers/apparatus-operator/src/admit/cli_transport.rs
  - workers/apparatus-operator/src/admit/cli_transport_tests.rs
```

## Verdict

Le correctif fait ce que le contrat M2 du ledger exige sur les sept points contrôlés : les cinq sites n'émettent plus le mot de passe en argv, le framing stdin ORAS est non tronquant avec EOF avant l'attente, l'auth Cosign passe par un `DOCKER_CONFIG` privé enfant-scopé sans `set_var` parent, le cleanup conteneur/config est fail-closed, `cli_failed` ne recopie plus stdout/stderr, et le fallback `docker run` reste sans secret en argv avec id exact 64 hex et sans `rm -f`/`-v`/prune/adoption. Trois notes non bloquantes (INFO), une ligne DEFER_TO rust-perf. Aucun correctif demandé par cette revue.

## Findings

### N-1 — INFO (LOW reachability, HIGH impact si atteint)

- **Location:** `workers/apparatus-operator/src/admit/cli_transport.rs:197,261-269` (`write_stdin`)
- **What:** `write_all` du frame stdin est synchrone, **avant** le spawn des lecteurs et sans deadline ; seule `wait_direct` est bornée. Si le frame excède le buffer du pipe (~4-64 KB), l'enfant peut bloquer en écriture stdout/stderr sans lire stdin et l'écriture parent reste bloquée indéfiniment — hors de toute deadline (le blocage précède `wait_direct`).
- **Why:** atteignable uniquement avec un mot de passe registry > buffer du pipe. Le mot de passe vient de `APPARATUS_REGISTRY_PASSWORD` (`admit.rs:95`) et zot l'hash en bcrypt (≤ 72 bytes utiles) ⇒ frame ≤ 74 bytes, jamais proche du buffer. Mode de défaillance résiduel = hang d'admission, pas une fuite.
- **Suggested fix shape:** si un jour le canal stdin transporte plus qu'un frame htpasswd : écrire stdin depuis un thread avec la même deadline, ou borner la taille du frame à la création (`oras_password_frame` refuse > N). Ne pas le faire dans ce lot.
- **Tests to run:** aucun requis ; un test de frame géant serait un surrogate non représentatif.

### N-2 — INFO (cosmétique, fail-closed préservé)

- **Location:** `workers/apparatus-operator/src/admit/cli_transport.rs:401-403` (`assemble`)
- **What:** une erreur de lecture mi-flux (`PipeDrain.eof == false`) est rapportée `"cannot start helper"`, message réservé ailleurs aux échecs de spawn (`spawn_child`, `spawn_reader`, `write_stdin`).
- **Why:** aucun impact comportemental — erreur générique, admission fail-closed, aucun octet de sortie exposé. Seule la sémantique du message est imprécise pour le diagnostic opérateur.
- **Suggested fix shape:** message distinct genre `"cli pipe read failed"` si M7/joueur d'exploitation le veut. Pas un bug.
- **Tests to run:** n/a.

### DEFER_TO: rust-perf — détachement des lecteurs à l'échéance

`Reader::wait_until` abandonne le `JoinHandle` à timeout (`cli_transport.rs:306-327`) : thread lecteur détaché jusqu'à EOF réel. Hors périmètre M5 (assigné M6, doc `cli_transport.rs:1-7,320-326`). Ne pas corriger ici.

Aucun finding BLOCKER/HIGH/MEDIUM/LOW sur le comportement.

## Ce qui a été vérifié (contrat M2 ledger, fichier:ligne)

1. **Cinq sites sans valeur de mot de passe en argv** : ORAS push `admit.rs:328-341` (`--password-stdin`:333), ORAS fetch `admit.rs:344-356` (:350), Cosign sign `admit.rs:436-450` (flag supprimé, `--registry-username`:443 conservé), Cosign verify `admit.rs:462-482` (:470), ORAS tag `.sig` `admit.rs:490-512` (:508). Garde générique `reject_secret_argv` (`cli_transport.rs:153-173`) : refuse tout arg contenant le secret (égalité ou valeur après `=`), plus tout `--password`/`--registry-password` même sans valeur. Grep crate complet : aucun sixième site `Command::new` en prod hors `whoami`/`icacls`/`spawn_child`. Négatif : `password_flags_never_spawn` (tests:516-556) prouve qu'aucun spec n'est construit.
2. **Framing ORAS CRLF non tronquant** : `oras_password_frame` (`cli_transport.rs:61-66`) = bytes + CRLF, jamais de trim. Test `oras_password_frames_keep_cr_lf_and_empty` (tests:452-468) : `ends-cr\r`, `ends-lf\n`, `ends-crlf\r\n`, vide (`\r\n` seul), 4096 bytes. Décision CRLF = oracle humain ledger §M2 ; fidélité ORAS réel = G-17 defer (hors lot).
3. **stdin fermé avant l'attente** : `supervise` (`cli_transport.rs:194-238`) : `write_stdin` :197 → `drop(child.stdin.take())` :201 → readers :204-218 → `wait_direct` :220. EOF atteint avant tout poll. Preuves réelles : `stdin_is_closed_before_wait` (findstr/cat, tests:1028-1049 win, 1157-1172 unix). Docker fallback ORAS : `docker run -i` ajouté (`cli_transport.rs:96`) pour router stdin vers le conteneur ; Cosign sans stdin → pipe fermé immédiatement, sans hang.
4. **Cosign sans `--registry-password`, `DOCKER_CONFIG` privé, pas de `set_var` parent** : `invoke_cosign` (`admit.rs:534-561`) via `with_registry_config` → `ConfigGuard` (`cli_transport.rs:500-566`) : répertoire temporaire unique (nanos+atomic :709-715), `config.json` `AuthConfig{username,password}` sous clé host **et** network (`:573-579`), sérialisé par struct ⇒ aucun `credsStore`/`credHelpers` possible, négatif asserté (`assert_auth_json` tests:299-320). Pas de `docker login`, pas de `COSIGN_REGISTRY_PASSWORD` (struct :592-601 + assert tests:611-614). ACL Windows : `icacls /inheritance:r` + `/grant:r *SID:(OI)(CI)F` dir puis fichier, fenêtre sans secret (config écrit après lock) ; tests réels `windows_acl_is_single_owner_ace` (tests:943-979), modes unix `cfg(unix)` tests:1007-1026. Aucun `set_var`/`remove_var` dans le crate (grep) ; preuve parent inchangée par snapshot (`assert_parent_unchanged`, tests:239-245) sur sign host et sign docker.
5. **Cleanup fail-closed** : conteneur — `run_docker` (`cli_transport.rs:136-151`) : succès sans cidfile lisible ⇒ erreur ; `ContainerRelease` (:414-442) Drop = unwind-safe ; `release_container` (:444-471) : id 64-hex validé (:486-488) → `stop` → `inspect` état `exited|dead` → `rm`, tout échec = `"container cleanup failed"` remonté via `combine` (:695-707), cleanup en échec ne masque pas l'erreur d'origine (combinée). Pas de `-f`, pas de `-v`, pas de prune, pas d'adoption par nom (id invalide ⇒ aucun spawn, test tests:823-843). Config — `ConfigGuard::finish`/Drop + `remove_private_dir` (:544-549) : absent ⇒ Ok (double suppression sûre), fichier non-dir ⇒ Err ; `locked_config_cleanup_fails_closed` (tests:917-931) : fichier verrouillé ⇒ suppression échoue ⇒ erreur, répertoire persiste.
6. **`cli_failed` sans stdout/stderr** : `admit.rs:623-625` — status.code() seul. Sorties jamais copiées dans une erreur : `assemble` (`cli_transport.rs:393-412`) retourne des messages génériques (`cli deadline exceeded` / `cannot start helper` / `cli output exceeded limit`), plafond 1 MB `read_capped` (:354-391). Preuves : `cosign_failure_omits_output_and_removes_config` (tests:636-660, marqueur adversarial + 0xFF non transcrit), `output_over_limit_is_generic` (win/unix), `cli_failed_and_redact_hide_secrets` (tests:845-852, `redact` intact pour le canal Transit HTTP `admit.rs:394`).
7. **Fallback docker sans secret, id 64 hex, pas de `rm -f`** : `run_docker` (`cli_transport.rs:88-151`) : argv sans secret re-vérifié après composition (:125) ; secrets d'env passés par `-e KEY` **nu** (valeur tirée de l'env du client docker, elle-même enfant-scopée via `set_env`) — le secret ne figure plus dans l'argv de `docker run` (correctif du défaut historique `-e KEY=VALUE`) ; `DOCKER_CONFIG` spécialisé en mount `:ro` + `-e DOCKER_CONFIG=/auth` (:104-114). `assert_docker_run` (tests:260-275) interdit `--rm`/`--name`/`-t`/`prune`/`login` et tout secret ; `runs_cleaned_once` (tests:277-297) : exactement un `rm` par `run`, cible = id 64 hex exact ; deadline couvre aussi les outils de cleanup (`docker_tool_spec` :473-484) ; `docker_deadline_still_removes_exact_id` (tests:803-820).

Parité transports (R-06) : même argv, même stdin frame, même config pour host vs `docker run` (`oras_docker_run_uses_stdin_and_exact_container_id` tests:715-747, `cosign_docker_run_mounts_config_and_keeps_secrets_out_of_argv` tests:662-713) ; sélection `VAULT_ADDR` host/network inchangée (`admit.rs:535-540`). Canal Vault enfant Cosign inchangé (VAULT_ADDR/VAULT_TOKEN/TRANSIT_SECRET_ENGINE_PATH/COSIGN_YES, tests:582-609). Scrub env enfant (`SCRUB_ENV` :28-34, remove avant set :243-248) = durcissement nouveau, sans consommateur (ORAS n'a pas besoin de VAULT_TOKEN ; assertions tests:741-742).

## Reasoning (déjà tranché — ne pas ré-explorer)

- Le garde `reject_secret_argv` est un échec fail-closed volontaire ; un mot de passe substring d'un arg légitime (ex. `=`-value) rejette l'admission sans fuite. C'est le contrat, pas un bug de faux positif.
- `docker run` sans `--rm` : requis par le flux stop→inspect→rm (un conteneur `--rm` disparaît avant l'inspect). Cleanup manuel = décision ledger.
- `-e KEY` nu (sans `=valeur`) : sémantique docker officielle d'héritage depuis l'env du client ; la valeur vit dans `set_env` de l'enfant docker, jamais en argv. Vérifié par specs capturés (tests:685-697).
- Deux clés `auths` (host + network) : nécessaire car l'hostname du digest_ref change selon le transport. Décision ledger.
- Succès CLI + héritier de pipe au-delà de la deadline ⇒ converti en erreur (`cli deadline exceeded`) : fail-closed voulu (anti-RP-1), pas une régression de succès.
- Les hooks `#[cfg(test)]` (`intercept`, `forced_host`, overrides deadline/limit) sont des seams hors prod ; la prod n'exécute ni `intercept` ni overrides.
- Gate Windows : 23 tests = total des `#[cfg(windows)]` + neutres ; les `cfg(unix)` (tests:1007-1026, 1157-1252) ne sont pas compilés par cette gate — couverture unix assumée par le plan (G-12 scindé, décision lead §8).

## Do not redo

- Ne pas re-prouver que les cinq sites sont argv-safe : c'est fait et gardé par `reject_secret_argv` + `password_flags_never_spawn`.
- Ne pas chercher un sixième site d'invocation ORAS/Cosign : grep `Command::new` sur le crate fait (prod = `spawn_child`, `whoami`, `icacls` uniquement).
- Ne pas re-trancher le framing CRLF ni « io.ReadAll attend EOF » : décisions humaines ledger §M2 ; la fidélité binaire réelle est G-17, defer hors mission.
- Ne pas relancer la gate ni aucun cargo (slot Windows, décision un-seul-Cargo) : 23 passed déjà produits par le lead.
- Ne pas rouvrir la conversion succès→erreur en cas d'héritier de pipe (fail-closed voulu).
- Ne pas auditer la sémantique `redact` restante (canal Transit HTTP uniquement, inchangé, testé).

## Anti-goals

- Ne pas corriger le détachement des lecteurs (DEFER_TO rust-perf, M6).
- Ne pas réintroduire `--rm`, `-e KEY=VALUE`, `rm -f`, `-v`, prune ou l'adoption par nom dans le fallback docker.
- Ne pas toucher au canal env Cosign existant (VAULT_TOKEN/VAULT_ADDR/TRANSIT/COSIGN_YES).
- Ne pas « améliorer » `cli_failed` en recopiant des sorties (contrat non-adversarial).
- Pas d'IT réelle ORAS/Cosign dans ce lot (G-17 defer, décision lead §8).

## Open questions

- N-1 : rester INFO tant que le canal stdin ne transporte que le frame htpasswd (≤ 74 bytes). Si un futur lot passe un payload plus large, réévaluer en MEDIUM avant implémentation.
- INDEX.md non mis à jour volontairement : contrainte tâche « un seul fichier écrit » ; le lead peut ajouter la ligne d'index.

## Tests

- Déjà exécuté (lead, ne pas relancer) : `cargo test --locked -p apparatus-operator --no-default-features --features admit --lib admit::cli_transport_tests -- --test-threads=1` → 23 passed (Windows ; `cfg(unix)` non compilés).
- Aucun test supplémentaire requis par cette revue (aucun fix demandé).

## Staleness

Briefing lié à `c2fe7172ccc4` sur master, arbre propre hors `.opencode/missions/`. Si M6/M7 font bouger `admit.rs` ou `cli_transport.rs`, revérifier les locations ci-dessus avant de s'appuyer sur les numéros de ligne.