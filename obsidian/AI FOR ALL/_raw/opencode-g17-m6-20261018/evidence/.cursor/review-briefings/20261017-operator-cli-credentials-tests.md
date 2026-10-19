# Review briefing — M8 audit des tests du transport CLI credentials

```yaml
reviewer: tests
written_at: 2026-10-17T16:10Z
branch: master
head_sha: c2fe7172ccc4fa6f0d1ed18fc850b7da9b6b92df
dirty: false
scope_slug: operator-cli-credentials-tests
verdict: PASS_WITH_NOTES
related_paths:
  - workers/apparatus-operator/src/admit/cli_transport_tests.rs
  - workers/apparatus-operator/src/admit/cli_transport.rs
  - workers/apparatus-operator/src/admit.rs
  - .cursor/qa-plans/20261017-operator-cli-credentials.md
```

## Verdict

PASS_WITH_NOTES. Les 17 gaps du registre (G-1…G-17) sont couverts par un test qui asserte l'oracle du plan, ou defer tracé (G-17 seul). Aucun G critique non couvert et non defer : pas de BLOCK, aucun finding T-<n> actionnable. Les notes ci-dessous n'exigent aucune correction.

## Comptage de la gate (cohérence vérifiée)

- 23 tests compilés par la gate Windows : 15 cross-platform + 8 `cfg(windows)`. Cohérent avec le « 23 passed » du ledger (gate non relancée, preuve rapportée par le lead).
- 7 tests `cfg(unix)` présents mais non compilés par cette gate (static-presence) : modes 0700/0600, stdin, deadline direct/heir, plafond, spawn, scrub env.
- Hooks de test : `intercept` (cli_transport.rs:187-190, mock in-process de `execute`), `forced_host` (admit.rs:563-569), `deadline_override`/`limit_override`. Toute la surface admit passe par ces mocks ; les enfants réels de la gate sont limités à cmd/findstr/ping/sh/cat/sleep.

## Registre G → preuve (cli_transport_tests.rs sauf mention)

| G | État | Preuve (file:line) |
|---|---|---|
| G-1 ORAS ×3 sans password argv | COVERED | `oras_sites_omit_password_and_frame_stdin` L470-513 : 3 specs exactement (L496), `assert_no_secret` (L503), `--password-stdin`+`--username`/`signer` (L500-502), refs conservées (L510-511) ; défense en profondeur `password_flags_never_spawn` L515-556 + `reject_secret_argv` (cli_transport.rs:153-169). Mutation « réintroduire `--password` » : rejetée avant spawn OU arg détecté → test rouge. |
| G-2 Cosign ×2 sans `--registry-password` | COVERED | host L558-634 (sign/verify L625-626, L579) ; docker L662-713 (L683) ; `password_flags_never_spawn` L541-555. |
| G-3 framing stdin CRLF/bord | COVERED | `oras_password_frames_keep_cr_lf_and_empty` L452-468 (5 cas dont vide = CRLF seul, décision §8.4, + long) ; e2e host L504, docker L737-740 ; non-troncation L456-458. |
| G-4 EOF avant attente | COVERED | `stdin_is_closed_before_wait` Windows L1028-1049 + unix L1157-1172 : si stdin n'était pas fermé avant l'attente, l'enfant réel n'EOF jamais → échec à la deadline. Volet « sans EOF → échec borné » : `pipe_heir_deadline_returns_before_bound` L1067/L1189. |
| G-5 schéma DOCKER_CONFIG | COVERED | `assert_auth_json` L299-320 : JSON valide, exactement 2 auths, username/password exacts sous clé host ET network (mot de passe avec CRLF interne testé L563) ; répertoires distincts par appel L627-629 ; contenu lu sur disque par le mock (L94-113). |
| G-6 env parent inchangée, DOCKER_CONFIG enfant seul | COVERED | snapshot PARENT_ENV 8 clés L22-31, L565+L573 (host) et L666+L675 (docker) ; `set_env` DOCKER_CONFIG L610 (host) ; `-e DOCKER_CONFIG=/auth` L687 et PAS dans set_env L690 (docker) ; mapping set/remove_env → enfant réel prouvé par `child_env_scrubs_vault_token_unless_explicit` L1122-1155 (parent inchangé L1139/L1154). Aucun `set_var`/`remove_var` en prod (cli_transport.rs:7). Conforme à la décision §8.1 : ce n'est PAS un scrub complet du parent, et les tests n'en exigent pas un. |
| G-7 négatifs config | COVERED | pas credsStore/credHelpers/COSIGN_REGISTRY_PASSWORD dans le JSON L300-302 ; pas de var COSIGN_REGISTRY_PASSWORD L614 ; pas de `login` L268. |
| G-8 deadline unique + joins (RP-1) | COVERED | `direct_child_deadline_returns_without_hunting_heirs` L1051/L1174 ; `pipe_heir_deadline_returns_before_bound` L1067/L1189 = la régression RP-1 exacte (héritier de pipe → retour borné) ; `assert_bounded` L393-403 (durée < borne+3s) ; `docker_deadline_still_removes_exact_id` L803-820. Mutation « borner seulement try_wait » : le test heir hang → timeout rouge. |
| G-9 sorties plafonnées, cli_failed non adversarial | COVERED | `output_over_limit_is_generic` L1091/L1204 ; `cosign_failure_omits_output_and_removes_config` L636-660 (marqueurs + octet non-UTF8 ; message = op + status uniquement) ; `cli_failed_and_redact_hide_secrets` L845-852 (contrat status-only + régression `redact`). Mutation « recopier stdout dans l'erreur » : L653 rouge. |
| G-10 spawn générique | COVERED | `missing_helper_error_hides_the_path` L1111/L1220 : chemin à secret → « cannot start helper », non divulgué. |
| G-11 fail-closed cleanup | COVERED | `config_removed_on_success_error_and_unwind` L872-904 : succès L881, double suppression sans panic L882, erreur corps → erreur d'origine préservée + dir supprimée L885-893, unwind/panic → Drop supprime L895-903 ; `remove_failure_is_returned_once` L906-915 (échec = erreur, pas retry silencieux) ; `locked_config_cleanup_fails_closed` L917-931 (Windows, échec réel de suppression). |
| G-12 perms owner-only | COVERED (scission §8.2) | Windows DANS la gate : `windows_acl_is_single_owner_ace` L943-958 + `assert_owner_only` L961-979 (exactement 1 ACE, non héritée, compte/SID courant, dir ET fichier). cfg(unix) ÉCRIT : `unix_config_modes_are_owner_only` L1007-1026 (0700/0600), non compilé par cette gate — conforme à la décision, non bloquant ici. |
| G-13 canal Vault inchangé | COVERED | host L582-609 (VAULT_ADDR host, VAULT_TOKEN, TRANSIT, COSIGN_YES) ; docker L685-704 (-e VAULT_TOKEN, VAULT_ADDR réseau) ; valeurs non dumpées : L655, L271-274 ; ORAS retire VAULT_TOKEN L505/L742. |
| G-14 parité transports | COVERED | cosign host L558 vs docker L662 (même schéma config asserté des deux côtés, mêmes négatifs argv) ; ORAS host L470 vs docker L715 (même framing stdin). |
| G-15 fallback docker argv-safe | COVERED | `assert_docker_run` L260-275 (pas --rm/--name/-t/prune/login, pas de secret) ; `runs_cleaned_once` L277-297 (rm == ["rm", id] exact : pas de `-f` ni `-v`) ; `docker_bad_id_skips_rm` L777-801 (pas d'adoption par nom) ; `docker_stop_failure_skips_rm` L749-775 ; Drop mocké : L822-843. |
| G-16 isolation gate | COVERED | mock in-process + host factice ; aucun docker réel, aucun socket registry, aucun téléchargement ; enfants réels = outils système only ; PingSweep L405-450 tue les pings résiduels (Drop). Gate exécutée sans Docker (ledger). |
| G-17 fidélité réelle ORAS/Cosign | DEFER | Plan §8.3 et ledger : hors mission, aucune IT réelle dans ce lot. Tracé, rien à exécuter ici. |

## Findings

None. Aucun finding T-<n> : aucun G critique non couvert ni non defer.

## Notes (sans action exigée)

- **S-1 (M7 LOW) non testé — noté, pas transformé en obligation.** `SCRUB_ENV` (cli_transport.rs:28-34) ne contient ni `APPARATUS_REGISTRY_PASSWORD` ni `APPARATUS_VAULT_TOKEN` ; aucun test n'asserte leur absence de l'env enfant (les assertions remove_env L615-623 listent exactement les 5 clés de SCRUB_ENV). Hors de l'oracle G-6 tel que tranché en §8.1 (env parent inchangée + DOCKER_CONFIG enfant seul), consigne de mission respectée.
- La frontière « mot de passe contenant le token Vault en sous-chaîne » (jeu de données §6 du plan) n'est pas exercée verbatim. Le risque visé (recopie dans l'erreur) est couvert plus fort : `cli_failed` est status-only (L845-852) et le test adversarial L636-660 met les deux marqueurs dans la même sortie. Aucune mutation plausible ne survit ; pas de finding.
- Les 7 tests cfg(unix) sont static-presence sur cet hôte (même logique de scission que G-12). Une gate Unix future les activerait sans modification.

## Reasoning (déjà tranché — ne pas ré-explorer)

- Les cinq sites sont couverts par leurs fonctions d'entrée (`oras_push_and_oci_digest`, `signature_tag_exists`, `sign_and_verify` → specs[0..2] / specs[0..1] comptés exactement), pas par des mock de bas niveau : une mutation sur un site précis rend un test rouge.
- Le lien CliSpec → env réelle de l'enfant est prouune fois pour toutes par les tests enfants réels (L1122-1155) ; les tests admit l'utilisent au niveau spec, ce qui est suffisant par composition.
- La preuve du « 23 passed » est rapportée par le lead (executed) ; cette revue n'a relancé ni Cargo ni la gate (consigne).
- Boucle fermée : M4 a écrit code et tests. Les oracles assertés proviennent du plan/ledger (§2 décisions, registre G), pas de l'implémentation ; l'oracle « cannot start helper » (message interne) est un détail de libellé non spécifié par le plan, avec un comportement asserté cohérent — pas une assertion affaiblie.

## Do not redo

- Ne pas recompter les tests : 23 gate (15 cross + 8 windows) + 7 unix non compilés.
- Ne pas relancer la gate ni `cargo` d'aucune sorte pour ce lot (décision mission).
- Ne pas chercher une IT ORAS/Cosign réelle : G-17 est defer hors mission (plan §8.3).
- Ne pas rouvrir G-6 comme « scrub complet du parent » : décision §8.1, les tests figent exactement la sémantic enfant-seul.
- Ne pas transformer S-1 en obligation de test : consigne de mission explicite.

## Anti-goals

- Ne pas écrire de test supplémentaire dans ce lot (M8 lecture seule).
- Ne pas toucher à workers/, docs/, rustycog, .cargo/config.toml, au plan QA ni à INDEX.md.

## Open questions

None. (G-17 reste la seule incertitude de fidélité réelle, déjà tracée comme defer hors mission.)

## Tests

Déjà exécutés (par le lead, preuve ledger) : `cargo test --locked -p apparatus-operator --no-default-features --features admit --lib admit::cli_transport_tests -- --test-threads=1` → 23 passed. À exécuter sur un runner Unix le cas échéant : la même commande compilerait alors les 7 tests cfg(unix) (modes 0700/0600 inclus).

## Staleness

Briefing lié à `c2fe7172ccc4` / arbre propre. Si le working tree bouge, revérifier les numéros de ligne avant toute action.
