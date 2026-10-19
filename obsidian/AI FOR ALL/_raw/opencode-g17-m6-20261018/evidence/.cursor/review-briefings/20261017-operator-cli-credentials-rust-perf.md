# Review briefing — rust-perf transport CLI credentials (M6)

```yaml
reviewer: rust-perf
written_at: 2026-10-17T13:34Z
branch: master
head_sha: c2fe7172ccc4
dirty: false # hors .opencode/missions/ (mission non commitée, hors lot)
scope_slug: operator-cli-credentials
verdict: PASS_WITH_NOTES
related_paths:
  - workers/apparatus-operator/src/admit.rs
  - workers/apparatus-operator/src/admit/cli_transport.rs
  - workers/apparatus-operator/src/admit/cli_transport_tests.rs
supersedes: RP-1 du briefing 20261010T1151Z-rust-perf-j3-cli-supervision-e107167.md (CLOSED)
```

## Verdict

PASS_WITH_NOTES. Revue statique, lecture seule, contexte fresh ; aucune conclusion de M4/M5 utilisée comme preuve (les notes M5 ont été rejugées indépendamment). Réponses aux quatre points assignés :

1. **Deadline unique enfant + joins : OUI** pour l'enfant direct et les deux joins stdout/stderr. `wait_direct` (`cli_transport.rs:271-282`) borne le polling par `Instant >= deadline_at` (overshoot ≤ `POLL_SLICE` 15 ms), puis `Reader::wait_until` (`:306-317`) borne chaque join par `rx.recv_timeout(wait)` avec `wait = deadline_at - now`. Chemin kill : les lecteurs reçoivent `deadline_at + READ_GRACE_AFTER_KILL` (`:229-234`, 1 s). stdin n'est **pas** un join : il n'existe aucun thread writer, l'écriture est inline pré-lecteurs (jugée au point 4). Bound total de `supervise` (`:194-238`) = `write_stdin` (non borné, point 4) + `spec.deadline` + ~15 ms + 1 s (chemin kill seulement). Chemins d'erreur tous bornés et tous réapent l'enfant (`reap_direct` `:284-292`) : échec `write_stdin` `:197-200`, échec `spawn_reader` `:204-218`, erreur `try_wait` `:220-228`.
2. **Cas RP-1 (héritier de pipe) : borné, sans rejeu d'admission réelle.** Enfant sorti + héritier tenant le pipe ⇒ pas d'EOF ⇒ `recv_timeout` timeout ⇒ `None` ⇒ `assemble` let-else → `"cli deadline exceeded"` (`:393-400`), lecteurs détachés par Drop (`:320-327`). Preuves enfants réels sans ORAS/Cosign/registry/Docker : `pipe_heir_deadline_returns_before_bound` Windows (`cmd /c start /b ping -n 40`, `cli_transport_tests.rs:1069-1089`) et Unix (`sh -c "sleep 30 &"`, `:1191-1202`), avec `assert_bounded` (`:393-403`) qui exige ≥ 1,5 s (la deadline a engagé) et < deadline + 3 s (retour borné ; l'ancien code aurait joint ~39 s). Chemin kill direct : `direct_child_deadline_returns_without_hunting_heirs` Windows/Unix (`:1053-1065`, `:1176-1187`). **RP-1 du 2026-10-10 : CLOSED.**
3. **JoinHandle abandonné (`Reader::drop`, timeout `wait_until`) : NON BLOCK.** Voir RP-A ci-dessous : design délibéré documenté (`cli_transport.rs:1-7,320-326`), rendu obligatoire par `unsafe_code = forbid` (pas de CancelIoEx ni poll natif sur pipe anonyme Windows en std) et par l'impossibilité d'annuler une lecture bloquante (tokio `spawn_blocking` joindrait jusqu'au drop du runtime). Le défaut RP-1 — retour appelant non borné — est corrigé ; la rétention résiduelle est bornée par occurrence et auto-terminal à l'EOF du pipe.
4. **`write_stdin` hors deadline : OUI hors deadline, NON BLOCK.** Voir RP-B : `write_all` synchrone (`:261-269`, appelé `:197`) précède lecteurs et `wait_direct`, sans borne ; mais le seul canal stdin est le frame htpasswd ORAS (≤ ~74 bytes ≪ buffer pipe OS ≥ 4 KB), le blocage exige un payload > buffer pipe, inatteignable avec ce credentiel. Confirme indépendamment la N-1 de M5.

Conformité contraintes du lot : pas de `process::exit` (aucun dans les trois fichiers), pas de chasse d'héritiers en prod (`reap_direct` tue l'enfant direct seul ; `PingSweep`/`taskkill` strictement test-only `:405-450`), pas de `set_var`/`remove_var` (uniquement `command.env_remove` enfant-scopé `:243-245`). Gate non relancée (23 passed, décision ledger).

## Findings

### RP-A — P3

- ID: RP-A (sans lien avec RP-1 du 2026-10-10, clos)
- Severity: P3 | Confidence: PROVEN (mécanisme) ; le scénario d'accumulation reste SPECULATIVE
- Scope: cross-function | Category: concurrency | Hotness: WARM (contrôle, chemin anomalie seulement)
- Location: `workers/apparatus-operator/src/admit/cli_transport.rs:306-317` (`wait_until` timeout), `:320-327` (`Drop for Reader`), `:26` (`READ_GRACE_AFTER_KILL`)
- Current behavior: à l'échéance, chaque lecteur non terminé est détaché : 1 thread OS + `Vec` ≤ `output_limit` (1 MB) + buf 8 KB retenus jusqu'à l'EOF/error du pipe (héritier vivant). À la fin, le thread envoie dans un canal fermé (`let _ = tx.send` `:345`) et s'arrête.
- Why it matters: l'occurrence unitaire est bornée (~quelques MB max, auto-libérée à la mort de l'héritier). Une accumulation non bornée exige des admissions répétées avec un héritier persistant — pathologique, et chaque cycle est fail-closed visible (`"cli deadline exceeded"`). C'est le résidu accepté sous `unsafe_code = forbid` ; l'alternative réellement annulable serait `tokio::process` (pipes overlapped Windows), un changement d'architecture hors contrat du lot.
- Proposed fix: aucun dans ce lot. Si un lot futur veut supprimer le résidu : bascule du transport sur `tokio::process` avec deadline unique, ou plafond de rétention côté appelant. Ne PAS proposer : chasse d'héritiers en prod, `unsafe`, détachement « déguisé » en spawn_blocking tokio (même rétention).
- Trade-offs: retour appelant borné (contrat RP-1) contre rétention temporaire par occurrence — Pareto correct sous la contrainte no-unsafe.
- How to validate: n/a (statique) ; un test de rétention longue exigerait un héritier durable, non représentatif.
- Expected result: statu quo. Pas de métrique réclamée.

### RP-B — P3

- ID: RP-B | Confirme indépendamment N-1 (M5) sans copier sa conclusion
- Severity: P3 | Confidence: PROVEN (mécanisme) ; inatteignabilité PROVEN pour le credentiel actuel
- Scope: cross-function | Category: concurrency (liveness) | Hotness: WARM
- Location: `workers/apparatus-operator/src/admit/cli_transport.rs:261-269` (`write_stdin`), appel `:197` ; sources stdin : `admit.rs:529` (`invoke_oras`), `admit.rs:557` (Cosign `stdin: None`)
- Current behavior: `write_all` bloquant sans deadline, avant le spawn des lecteurs et de `wait_direct`. Un payload > buffer pipe OS (≥ 4 KB typ.) avec un enfant qui écrit stdout/stderr sans lire stdin = deadlock hors deadline.
- Why it matters: seul ORAS transporte stdin, frame = password + CRLF (`oras_password_frame` `:61-66`) ; le password vient de `APPARATUS_REGISTRY_PASSWORD` (`admit.rs:95`), hashé bcrypt côté zot (≤ 72 bytes utiles) ⇒ frame ≤ ~74 bytes. Le seuil de blocage ne peut pas être atteint en configuration fonctionnelle. Mode de défaillance résiduel = hang d'admission (pas une fuite).
- Proposed fix (différé, pas dans ce lot) : si un jour le canal stdin dépasse le frame htpasswd — cap de taille à la création dans `oras_password_frame` (refuser > N bytes), ou écriture stdin threadée sous la même deadline.
- Trade-offs: aucun changement maintenant ; le cap gratuit peut être ajouté le jour où le canal grossit.
- How to validate: n/a.
- Expected result: statu quo.

## Observations hors périmètre (une ligne chacune, aucune action demandée)

- `whoami` (`cli_transport.rs:655-658`) et `icacls` (`:685-687`) utilisent `Command::output()` sans deadline : binaires système locaux de confiance, chemin COLD (par `ConfigGuard::create`). Parité de deadline possible via `supervise` dans un lot futur ; pas une régression de ce diff.
- `run_cli` synchrone est appelé depuis `push_and_sign_envelope` async (`admit.rs:298,306`) : bloque le thread executor jusqu'à ~121 s ; forme préexistante (l'ancien `Command::output` bloquait déjà), hors des quatre points jugés.

## Ce qui a été vérifié (fichier:ligne)

1. Couverture deadline : `supervise` `:194-238` ; `wait_direct` `:271-282` (clock-check avant sleep, overshoot ≤ 15 ms) ; `wait_until` `:306-317` (`saturating_duration_since`, `recv_timeout(0)` sûr si échéance passée) ; chemin kill + grâce 1 s `:229-234` ; conversion fail-closed `assemble` `:393-400` (documentée voulu côté M5, cohérente avec le code).
2. Réap de l'enfant sur toutes les sorties d'erreur : `:198`, `:207`, `:214`, `:223`, `:230` — aucun `Child` abandonné non réapé ; joins Ok bornés (le `handle.join()` `:310-312` suit un send déjà effectué ⇒ retour immédiat).
3. RP-1 : scénario héritier couvert statiquement (ci-dessus) + tests enfants réels bornés win/unix `:1069-1089`, `:1191-1202`, `:1053-1065`, `:1176-1187`, sans rejeu d'admission (ping/sleep/cmd/sh/findstr/cat seulement).
4. Ownership/alloc/CPU : frame copié en `CliSpec.stdin` (`to_vec` `:78`, `:131`) = ~74 bytes, COLD ; `read_capped` `:354-391` = un seul `Vec` en croissance amortie + drain-to-EOF nécessaire (sinon succès serait converti à tort en deadline) ; `assemble` déplace les `Vec` des threads vers `Output` sans copie (bon modèle) ; mpsc = 1 message/lecteur ; `POLL_SLICE` 15 ms ⇒ ≤ ~8000 `try_wait` sur 120 s, négligeable en contrôle ; monomorphisation `spawn_reader`/`read_capped` = 2 instanciations triviales ; aucun `Arc`, aucun lock, aucun clone non trivial dans le transport.
5. Contraintes lot : pas de `process::exit` ; pas de chasse d'héritiers en prod (kill enfant direct seul `:284-292` ; sweeps test-only) ; pas de `set_var`/`remove_var` ; hooks `#[cfg(test)]` (`intercept` `:187-190`, overrides `:723-737`) absents du binaire prod.

## Reasoning (déjà tranché — ne pas ré-explorer)

- Le détachement du `JoinHandle` est le résidu assumé de « retour borné » sous `unsafe_code = forbid` ; le doc module `:1-7` l'explicite. Rejugé et accepté (RP-A, P3).
- La conversion « enfant OK + pipes non drainés à l'échéance → erreur » est le contrat anti-RP-1 voulu (fail-closed), pas une régression de succès.
- La fenêtre `write_stdin` est réelle mais inatteignable avec le frame htpasswd ; elle ne casse pas la deadline unique du transport pour ce canal (RP-B, P3).
- Pas de finding alloc/CPU/ownership au-delà de RP-A/RP-B : le chemin est WARM contrôle-plane, toutes les copies sont des petits payloads froids, le flux Output est zero-copy depuis les threads lecteurs.

## Do not redo

- Ne pas rouvrir RP-1 (2026-10-10) : le retour est borné et prouvé par tests enfants réels ; son verdict BLOCK est obsolète pour ce diff.
- Ne pas proposer de chasse d'héritiers en prod, de `unsafe`, de `process::exit`, ni de port `spawn_blocking` tokio du même schéma (rétention identique).
- Ne pas rejuger les cinq sites argv / le framing CRLF / la config `DOCKER_CONFIG` : hors périmètre M6, couverts par M5 et `reject_secret_argv`.
- Ne pas relancer la gate Cargo (23 passed, slot Windows, décision ledger) ni aucun cargo.
- Ne pas corriger RP-A/RP-B dans ce lot : tout BLOCK ici renverrait à M4 ; aucun des deux n'en est un.

## Anti-goals

- Pas de modification de code (revue read-only, un seul fichier écrit : ce briefing).
- Pas de benchmark ni d'affirmation de perf mesurée : tout est statique ; aucune métrique réclamée.
- Pas d'IT réelle ORAS/Cosign (G-17 defer).

## Tests

- Statique seulement : lecture complète des trois fichiers à `c2fe7172ccc4` ; HEAD vérifié `c2fe7172ccc4fa6f0d1ed18fc850b7da9b6b92df`, arbre propre hors `.opencode/missions/`.
- Gate non relancée (décision ledger) : `cargo test --locked -p apparatus-operator --no-default-features --features admit --lib admit::cli_transport_tests -- --test-threads=1` → 23 passed (lead).
- Aucun test supplémentaire exigé par cette revue.

## Staleness

Briefing lié à `c2fe7172ccc4` sur master. Si `cli_transport.rs`/`admit.rs` bougent, revalider les locations : `supervise:194`, `write_stdin:261-269`, `wait_direct:271-282`, `Reader:300-327`, `assemble:393-412`, `reap_direct:284-292`, tests héritiers `cli_transport_tests.rs:1053-1089` (win) et `:1176-1202` (unix).

## Synthèse

```text
Correctness: PASS (liveness bornée sur toutes les sorties)
Borrowing: PASS
Allocation efficiency: PASS
CPU efficiency: PASS
Memory footprint: PASS (rétention résiduelle bornée par occurrence, auto-terminal)
Concurrency: PASS (RP-A/RP-B notés, aucun BLOCK)
Binary size: PASS (rien à signaler)

P0: 0  P1: 0  P2: 0  P3: 2 (RP-A, RP-B)  MEASURE: 0
Top-3 ratio gain/risque/complexité : (1) cap de taille du frame stdin si le canal grossit un jour ; (2) parité deadline pour whoami/icacls via supervise (lot futur) ; (3) rien d'autre — aucune action ce lot.
```
