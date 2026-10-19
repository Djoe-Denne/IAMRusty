# Review briefing

```yaml
reviewer: security
written_at: 2026-10-17T14:02Z
branch: master
head_sha: c2fe7172ccc4fa6f0d1ed18fc850b7da9b6b92df
short_sha: c2fe717
dirty: false # hors ??.opencode/missions/ (mission non commitée, hors lot)
scope_slug: operator-cli-credentials
verdict: PASS_WITH_NOTES # = GO_WITH_COMMENTS du canevas ; aucun BLOCK
related_paths:
  - workers/apparatus-operator/src/admit.rs
  - workers/apparatus-operator/src/admit/cli_transport.rs
  - workers/apparatus-operator/src/admit/cli_transport_tests.rs
```

## Verdict

PASS_WITH_NOTES. Aucun CRITICAL ni HIGH : pas de BLOCK, aucun correctif appliqué. Les cinq sites ne mettent pas la valeur du mot de passe registry dans l'argv. `cli_failed` et les erreurs de transport ne recopient pas de secret. Le cleanup config/conteneur est fail-closed. L'ACL Windows finale est une seule ACE de l'utilisateur courant. `VAULT_TOKEN` / `VAULT_ADDR` restent le canal Cosign déjà voulu ; ce diff le rétrécit sur l'argv `docker` (`-e NOM` sans valeur) et ne demande pas leur retrait.

L'invariant « mot de passe registry absent de l'environnement de l'enfant » n'est tenu que pour les canaux écrits par ce diff (`set_env`, `-e`, valeur de `DOCKER_CONFIG` = chemin). Il ne l'est pas pour l'héritage du processus parent : `APPARATUS_REGISTRY_PASSWORD` n'est pas dans le scrub. Impact limité au même utilisateur, qui possède déjà la variable. S-1 est LOW, non bloquant.

## Findings

### S-1 — LOW

- **Location:** `workers/apparatus-operator/src/admit/cli_transport.rs:28-34` (`SCRUB_ENV`), `:240-248` (`spawn_child`), `:655-674` (`whoami`), `:678-694` (`icacls`). Source : `workers/apparatus-operator/src/admit.rs:95` et `:117`.
- **Confidence:** HIGH (mécanisme). **Category:** secret-leak. **Scope:** cross-file.
- **Security property violated:** confidentialité du mot de passe registry dans l'environnement des enfants, telle que posée pour ce lot.
- **Attack surface:** processus enfants ORAS, Cosign, client `docker`, `whoami`, `icacls`.
- **Trust boundary:** opérateur → enfants du même compte. Pas de franchissement vers un autre utilisateur, ni vers l'`Env` du conteneur.
- **Attacker capability:** le parent du chemin schedule a déjà `APPARATUS_REGISTRY_PASSWORD` et `APPARATUS_VAULT_TOKEN`. L'attaquant qui lit l'environnement d'un enfant est ce même compte, ou un binaire enfant déjà de confiance.
- **What:** `Command` hérite l'environnement du parent. `spawn_child` ne retire que `VAULT_TOKEN`, `DOCKER_CONFIG`, `REGISTRY_PASSWORD`, `ORAS_PASSWORD`, `COSIGN_REGISTRY_PASSWORD`. Les noms réels du schedule, `APPARATUS_REGISTRY_PASSWORD` (`admit.rs:95`) et `APPARATUS_VAULT_TOKEN` (`admit.rs:117`), restent. `whoami` et `icacls` n'appellent ni `env_remove` ni le scrub : ils héritent tout le bloc parent. Aucun `std::env::set_var`.
- **Why:** le mot de passe registry est donc présent dans l'environnement des enfants supervisés et des deux outils d'ACL, en plus du canal voulu (stdin ORAS, fichier `config.json` Cosign). Le conteneur ne le reçoit pas : `-e` ne liste que les clés de `extra_env`, et `DOCKER_CONFIG` y vaut le chemin privé ou la littérale `/auth`. Ce n'est pas le retour du mot de passe dans l'argv.
- **Exploit scenario:** un processus du même compte lit l'environnement de l'enfant ORAS, Cosign, `docker` ou `whoami` et y trouve la variable schedule. Il pouvait déjà la lire sur le parent. Un autre utilisateur du même hôte ne gagne pas la lecture qu'il avait via l'argv (cmdline plus exposée que l'environnement). Pas de HIGH.
- **Impact:** copie supplémentaire, même compte, vers des processus qui n'en ont pas tous besoin (`docker stop`/`inspect`/`rm`, `whoami`, `icacls`). ORAS et Cosign reçoivent déjà le secret par le canal prévu.
- **Affected components:** `apparatus-operator` feature `admit`.
- **Suggested fix shape:** ajouter `APPARATUS_REGISTRY_PASSWORD` et `APPARATUS_VAULT_TOKEN` à `SCRUB_ENV`. Pour `whoami` et `icacls`, retirer ces mêmes noms (et le scrub existant) avant `output`. Éviter `env_clear` nu : ces binaires ont besoin de `SystemRoot` / `PATH`. Ne pas retirer le `VAULT_TOKEN` / `VAULT_ADDR` posés explicitement pour Cosign.
- **Tests to run:** étendre les assertions de `remove_env` déjà présentes pour exiger ces deux noms sur chaque `CliSpec` issu de `run_cli`. Ne pas afficher la valeur. Gate existante, un seul Cargo, non relancée ici.
- **Regression test:** un spec ORAS et un spec Cosign (hôte et `docker run`) dont `remove_env` contient `APPARATUS_REGISTRY_PASSWORD` et `APPARATUS_VAULT_TOKEN`, tandis que `set_env` Cosign conserve `VAULT_TOKEN` et un `DOCKER_CONFIG` qui est un chemin.
- **CWE:** CWE-526.

### S-2 — HARDENING

- **Location:** `workers/apparatus-operator/src/admit/cli_transport.rs:655-674` (`current_user_sid`), `:630-633` et `:649-650` (grant icacls).
- **Confidence:** HIGH sur le code, LOW sur l'exploitabilité. **Category:** authz. **Scope:** local.
- **What:** `whoami` et `icacls` sont résolus par le `PATH`. Le SID accepté est toute chaîne qui commence par `S-1-`. Ce SID devient l'unique ACE Full du répertoire qui recevra ensuite le mot de passe.
- **Why:** un `whoami` remplacé plus tôt dans le `PATH` peut renvoyer un SID large (forme `S-1-…` seulement) et faire accorder cette ACE avant l'écriture du fichier. Un maliciel du même compte peut déjà lire la variable parente et réécrire l'ACL lui-même. Un service dont le `PATH` commence par System32 ne l'expose pas. Pas une vulnérabilité démontrée.
- **Suggested fix shape:** appeler `whoami.exe` et `icacls.exe` par leur chemin System32, et n'accepter le SID que s'il est celui du jeton du processus. Hors de ce lot si S-1 est le seul correctif retenu.
- **Tests to run:** aucun Cargo pour ce durcissement dans ce lot.
- **CWE:** CWE-426.

## Reasoning

Revue statique, deux passes, contexte fresh. M5 et M6 relus comme hypothèses puis rejugés sur le source à `c2fe717`. Gate non relancée (23 passed, attestation du lead). Aucun Cargo, Kind, Docker, `unsafe` nouveau, dépendance nouvelle.

### PASS A — contrôles présents

- Cinq sites, valeur absente de l'argv. ORAS push `admit.rs:333`, fetch `:350`, tag `.sig` `:508` : `--password-stdin` plus username et références. Cosign sign `:442` et verify `:469` : `--registry-username` conservé, `--registry-password` absent (grep `password` sur `admit.rs` : aucun flag de valeur). Frame stdin : `oras_password_frame` (`cli_transport.rs:61-66`), appel `admit.rs:520`. Garde `reject_secret_argv` (`cli_transport.rs:153-168`) sur les args de l'outil et, pour le fallback, sur l'argv `docker` complet (`:125`).
- `DOCKER_CONFIG` Cosign = chemin du répertoire privé (`admit.rs:547`), pas le mot de passe. Côté `docker run`, cas spécial `:104-109` : montage `:ro` et argv `DOCKER_CONFIG=/auth`. Pas de `set_var` / `remove_var` parent (commentaire `:7`, source lu).
- `VAULT_ADDR` / `VAULT_TOKEN` / `TRANSIT_SECRET_ENGINE_PATH` / `COSIGN_YES` uniquement dans `env_vars` Cosign (`admit.rs:543-548`). Le client docker reçoit `-e NOM` nu (`cli_transport.rs:114-116`) ; la valeur n'est plus dans l'argv. ORAS : `extra_env` vide (`admit.rs:524`). Ce diff retire l'ancienne forme `-e NOM=valeur`. Il n'élargit pas le canal Cosign.
- `cli_failed` (`admit.rs:623-625`) : nom d'opération et `status.code()` seulement. `assemble` renvoie des messages génériques. `parse_oci_digest` n'embarque pas la sortie brute dans son erreur. Pas de `println` / `tracing` / `log` dans les trois fichiers. `redact` (`admit.rs:627-632`) reste le canal HTTP Transit, inchangé, masque le token seulement : hors du transport CLI, ne pas le rouvrir.
- Argv `docker` de cleanup : `stop` / `inspect` / `rm` + id 64 hex (`cli_transport.rs:444-471`, `:486-488`). Pas de `-f`, `-v`, prune, `--rm`, adoption par nom. Id invalide : erreur sans spawn.
- Cleanup fail-closed. `combine` (`:695-707`) : échec de cleanup = échec, y compris si l'action avait réussi. Succès `docker run` sans id lisible : `container cleanup failed`. `ConfigGuard::finish` propage `remove_dir_all`. Drop couvre l'unwind et avale l'erreur de drop (le chemin explicite, lui, la remonte).
- Fichier secret écrit après lockdown. `write_private_config` (`:568`) : `lock_down_dir`, `create_private_file`, `lock_down_file`, puis sérialisation. Unix : `0o700` / `0o600` (`:604`). Windows : `/inheritance:r` puis `/grant:r *SID:(OI)(CI)F` sur le répertoire (`:630-633`) et `*SID:F` sur le fichier (`:649-650`). JSON = `auths` username/password pour les clés host et network (`:576`) ; le struct n'émet ni `credsStore` ni `credHelpers`. La gate déjà verte inclut `windows_acl_is_single_owner_ace` (`cli_transport_tests.rs:952` et helper `:962`) : une ligne d'ACE, sans `(I)`, compte ou SID courant. Non rejoué ici.
- Sorties plafonnées (`CLI_OUTPUT_LIMIT`, `read_capped`). Échéance : `wait_direct` puis `recv_timeout` sur les deux lecteurs, kill de l'enfant direct, erreur `cli deadline exceeded` si un pipe ne se ferme pas. Pas de secret dans ce message.

### PASS B — contournements écartés

- Rejeu de l'argv : les cinq constructions et le grep `password` ne laissent aucune valeur. Un mot de passe égal à un argument entier est refusé par `arg == secret`. Le filtre ne voit pas une sous-chaîne hors forme `clé=valeur` ; les sites actuels ne dépendent pas de ce filtre, ils ne passent pas la valeur. Ne pas traiter le filtre comme la preuve.
- Écho CLI du secret : en échec, la sortie n'entre pas dans `AdmitError`. En succès, seul un digest `sha256:` + 64 hex est renvoyé. Le tag `.sig` ne renvoie qu'un booléen.
- Conteneur : `APPARATUS_REGISTRY_PASSWORD` hérité par le client docker n'est pas repassé par `-e`. `docker inspect` du conteneur Cosign voit `VAULT_TOKEN` (canal voulu) et `DOCKER_CONFIG=/auth`, pas le mot de passe registry.
- Fenêtre ACL : le répertoire est vide entre `create_dir` et `icacls` ; les octets du mot de passe sont écrits après les deux lockdowns. `create_dir` sur un chemin déjà pris échoue (fail-closed). Remplacement du cidfile par un autre utilisateur : `/tmp` sticky ou temp utilisateur, le fichier créé par docker n'est pas remplaçable par un tiers ; un même compte peut déjà `docker rm`. Pas un franchissement.
- Chaîne PATH + SID large + secret hérité : réelle dans le code (S-2), exploitable seulement si un principal moindre écrit un `whoami` antérieur sur le `PATH` du compte opérateur. Non démontré pour un service dont le `PATH` est System32. Ne monte pas au HIGH.
- `write_stdin` hors deadline et `JoinHandle` abandonné : relus. Le frame ORAS est le mot de passe plus CRLF, très en dessous du buffer de pipe pour un secret registry. L'appelant revient à l'échéance même si un héritier tient le pipe. Résidu = un thread et au plus 1 Mio jusqu'à l'EOF, sur binaires épinglés. Pas une fuite, pas un DoS distant simple. RP-1 du 2026-10-10 n'est pas rouvert comme finding sécurité.
- Schéma `config.json` sans champ `auth` base64 : si Cosign l'ignore, l'admission échoue. Pas de `credsStore` vers lequel se replier. Fidélité réelle = G-17, defer hors mission, pas un trou de confidentialité.

### Mini threat model

- **ASSET:** mot de passe registry et token Transit. **ATTACKER:** autre compte local, sortie CLI hostile, binaire du PATH. **ENTRY POINT:** cinq lancements plus `whoami`/`icacls`. **TRUST BOUNDARY:** opérateur → ORAS/Cosign/docker de confiance, fichier owner-only. **CAPABILITY:** argv, environnement hérité, fichier temp, stdout. **ABUSE CASE:** relire le secret dans l'argv, une erreur, ou un fichier à ACE large. **EXISTING MITIGATION:** stdin / fichier privé, argv filé, erreurs opaques, ACL finale une ACE, cleanup fail-closed, `-e` sans valeur. **MISSING MITIGATION:** scrub des noms `APPARATUS_*` (S-1) et `whoami` non ancré (S-2). Pas de mitigation manquante de niveau HIGH.

### Hypothèses M5 / M6 rejugées

- M5 : les sept points d'argv, stdin, `DOCKER_CONFIG` chemin, parent non muté, cleanup, `cli_failed`, docker sans secret dans l'argv tiennent, revérifiés sur le source. La phrase qui traite le scrub comme suffisant pour l'environnement enfant est rejetée : la liste ne contient pas le nom schedule, et `assert_parent_unchanged` (`cli_transport_tests.rs:23-31` et le snapshot parent) ne lit pas l'environnement de l'enfant. La gate verte ne prouve pas S-1.
- M6 : l'échéance borne l'enfant direct et les deux joins. RP-1 n'est plus un BLOCK sécurité sur ce diff. RP-A et RP-B restent hors finding sécurité (liveness, pas de fuite).
- N-1 / N-2 de M5 : confirmés sans en faire des findings. N-2 est un libellé fail-closed, sans octet de sortie.

## Do not redo

- Ne pas recompter les cinq sites ni relancer la gate (23 passed, slot Cargo unique).
- Ne pas rouvrir RP-1, ni classer le thread détaché ou `write_stdin` en HIGH sécurité sur ce frame.
- Ne pas exiger le retrait de `VAULT_TOKEN` / `VAULT_ADDR` du canal Cosign.
- Ne pas traiter `redact` Transit comme une régression de ce diff.
- Ne pas confondre snapshot du parent (`PARENT_ENV`) et environnement réel de l'enfant.
- Ne pas rouvrir G-17 (binaire ORAS/Cosign réel) dans cette mission.
- INDEX non écrit ici : contrainte de la tâche, un seul fichier. Le lead peut ajouter la ligne.

## Anti-goals

Pas de APPLY SECURITY FIXES. Ne pas modifier `workers/`, `docs/`, `rustycog`, `.cargo/config.toml`, le plan QA, ni `INDEX.md`. Ne pas remettre le mot de passe dans l'argv, ni `-e NOM=valeur`, ni `--rm` / `rm -f` / `-v` / prune. Ne pas vider l'environnement Cosign voulu. Ne pas promouvoir l'ADR 0412.

## Open questions

None pour un BLOCK. S-1 peut partir dans un correctif ultérieur ; il ne retient pas ce lot.

## Tests

Exécutés par cette revue : aucun. Preuve citée, non rejouée : `cargo test --locked -p apparatus-operator --no-default-features --features admit --lib admit::cli_transport_tests -- --test-threads=1` → 23 passed (lead, Windows ; `cfg(unix)` hors de cette gate).

À exécuter seulement si un correctif S-1 est demandé : la même gate, plus les assertions `remove_env` décrites ci-dessus.

## Staleness

Lié à `c2fe7172ccc4fa6f0d1ed18fc850b7da9b6b92df` sur `master`, arbre propre hors `.opencode/missions/`. Si `admit.rs` ou `cli_transport.rs` bougent, revérifier `SCRUB_ENV:28-34`, `spawn_child:240-248`, `whoami:655`, `icacls:678`, `cli_failed:623-625`, `invoke_cosign:534-549`, et les trois `--password-stdin` (`:333`, `:350`, `:508`).