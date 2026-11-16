# Campagnes de correction : Clippy prioritaire

Workflow courant pour les services AIForAll. Il est appelé par le skill
[`aiforall-sonar-policy`](../../.agents/skills/aiforall-sonar-policy/SKILL.md)
et remplace le dispatch fondé sur les lignes Sonar des campagnes historiques.
Il ne lance pas une campagne automatiquement et ne crée pas de gate bloquant.
Le sous-module RustyCog reste un lot distinct, sur autorisation et avec sa
politique propre ; ne pas lui appliquer implicitement ce dispatch services.

## 1. Produire une photographie locale fraîche

Le lead réserve l'unique slot Cargo et attend la fin des éditions avant de
lancer **`just clippy-ci`**, nativement sous Windows, depuis la racine intégrée.
Les paramètres sont ceux de [Clippy local](clippy-local.md) : deux passes
production/tests, workspace locked all-features, toolchain épinglée, 24 jobs.
Ne pas exécuter de services ou de tests d'intégration pour ce simple inventaire.

Sources de l'inventaire :
- `target/sonar/clippy-production.jsonl` ;
- `target/sonar/clippy-tests.jsonl` ;
- `target/sonar/clippy.json`, concaténation des deux précédents.

Les deux passes doivent être terminées avec succès ; le lanceur vérifie leurs
codes retour et événements `build-finished`. Une passe échouée ou incomplète
donne un **inventaire partiel**, pas zéro diagnostic ni une preuve d'absence.
Si une passe échoue, ne pas réutiliser un ancien fichier de l'autre passe.

Avant modification, conserver sous `.tmp/<campagne>/<run>/` les rapports utiles
et un petit manifeste : date, HEAD **et état des sources non committées**,
racine/worktree, toolchain, options et résultat de chaque passe. HEAD seul ne
décrit pas un arbre dirty. Après toute modification des sources analysées,
l'inventaire n'est plus une photographie du code actuel. Ne pas comparer les
chiffres de deux configurations différentes comme une progression de dette.

## 2. Lire les diagnostics, pas les anciennes lignes Sonar

Filtrer et agréger les JSON dans Context Mode. Les messages utiles ont
`reason = "compiler-message"` :
- règle : `message.code.code` (`clippy::...` pour Clippy) ;
- niveau et texte : `message.level`, `message.message` ;
- emplacements : `message.spans` où `is_primary = true`, avec `file_name`,
  `line_start`, `column_start`, `byte_start` et `byte_end` ;
- provenance : passe production/tests, `package_id` et `target`.

Conserver les avertissements rustc séparément et remonter les erreurs bloquantes.
Les autres lignes JSON (artefacts, fin de build) ne sont pas des diagnostics.
Un diagnostic sans emplacement primaire reste dans une file à examiner ; ne
pas lui inventer une ligne ou un propriétaire de fichier.

Dans **une photographie**, dédupliquer par niveau + règle + message + ensemble
des emplacements primaires (chemins normalisés par rapport à la racine du run).
Conserver toutes les appartenances production/tests/packages/targets derrière
un finding fusionné. Ne pas additionner naïvement les deux passes ; des tests
peuvent réanalyser le même code de bibliothèque.

Les positions ne sont **pas** des identifiants persistants. Pour un rapprochement
entre runs, utiliser fichier/règle/symbole/contexte après lecture des sources,
et signaler les cas ambigus. Ne pas fusionner des occurrences distinctes au seul
motif qu'elles ont la même règle et le même texte. Ne pas convertir une ligne
Sonar en ligne actuelle par ajout/soustraction d'un décalage.

## 3. Classer et attribuer des lots exclusifs

Conserver deux axes distincts :
- **Dispersion de la règle** : locale si une seule aire du dépôt, transverse si
  plusieurs aires (`services/<nom>`, `workers/<nom>`, `crates/<nom>`,
  `runtime/<nom>`). Ce comptage n'établit pas l'impact d'un correctif.
- **Impact du changement**, confirmé par le code et ses consommateurs :
  local mécanique ; structure privée ; comportement ; API/signatures/appelants.
  Une règle présente partout peut se corriger fichier par fichier ; un seul
  diagnostic sur une API publique peut nécessiter un lot transverse.

Le lead consulte le [registre des décisions](clippy-decisions/README.md) et les
tables du skill, puis constitue des lots de fichiers disjoints. L'unité de
conflit reste le fichier ; respecter aussi le mutex package × src/tests.
Une signature et ses consommateurs restent dans le même lot, même si plusieurs
règles sont concernées. Ne pas scinder un gros fichier commun entre auteurs.
Faire valider les décisions structurelles selon les contrats du dépôt.

Chaque claim consigne : propriétaire, photographie source, fichiers autorisés,
règles et symboles/contexte concernés, invariants, précédents applicables,
validation attendue et dépendances sur les autres lots. La ligne du run est une
aide de navigation, **pas l'identité du lot**. Aucun nombre fixe de sous-agents :
choisir selon les lots indépendants et charger `opencode-ensemble` si délégation.

In a divergent worktree, canonical locations are hints: inspect the actual local
code and delta before editing. Canonical indexes do not include unintegrated
worktree edits. A worker does not run Cargo during the editing phase, even after
finishing its own lot. An out-of-claim consumer or stale location requires
relocalization or a blocker report, not an ownership extension.

## 4. Continuously refill workers; validate at the final barrier

The default is a bounded continuous queue, not synchronized waves of ten agents.
Choose concurrency from independent claims and available authorized workers.
Only the lead assigns lots; using this guide does not launch agents or a campaign.

1. **Freeze a backlog from the initial valid snapshot.** Give findings run-local
   IDs, rule/symbol/context and explicit file ownership. Agree the campaign scope
   and final validation commands before dispatch. This backlog is historical
   evidence of what to inspect, not a live warning counter.
2. **Refill without waiting for peers.** When a worker finishes, first secure its
   report and identifiable delta, release its old claim, then assign another
   eligible lot from that backlog. A queued finding must still be confirmed in
   the current code; never recalculate old line offsets. Record already-absent or
   changed findings as such, not as worker fixes. Do not manufacture new findings
   from stale positions or start a new scan while edits continue.
3. **Respect dependencies rather than forcing occupancy.** Keep signatures and
   consumers together. A lot depending on unintegrated work stays queued until
   that dependency is available and its scope is rechecked. Use isolated writer
   worktrees according to project policy; reports do not replace integration.
   A free worker may stay idle when no independent lot is eligible.
4. **Do not compile after each return.** During the editing phase, workers use
   source/reference/diff inspection and record Cargo-dependent checks as not run.
   No `cargo build`, `check`, `test`, `clippy` or formatter/hook mutating active
   source files is launched as a background shortcut. Do not bypass normal hooks;
   defer an operation whose hook requires an unsafe validation/mutation.
5. **Drain and close the editing phase.** Once the frozen backlog is completed,
   blocked or explicitly deferred, stop refilling. Obtain explicit no-more-writes
   acknowledgements from all writers, including pending/background edits. Secure
   their reports and deltas, integrate the retained changes and record the exact
   integrated source state. A returned chat message alone is not this barrier.
6. **Validate the frozen integrated tree.** The lead reserves the single Windows
   Cargo slot (24 internal jobs) while writers remain paused. Run the agreed
   build and risk-appropriate tests, then complete `just clippy-ci` to refresh both
   passes and compare findings at constant scope. Clippy analyzes/compiles code;
   it is not a substitute for linked-build proof or behavioral tests when those
   are required. Do not add redundant checks merely because each lot returned.
7. **Repair from evidence.** Use per-lot reports and the actual integrated diff
   to attribute compilation errors or incompatible changes, including interacting
   lots. Open bounded repair lots, then repeat the no-writer barrier before
   revalidation. Rebuild the next backlog from the fresh complete reports, not
   from guessed offsets. Failed/partial analysis is not zero warnings.

This intentionally detects some integration mistakes only at the end. An earlier
coordinated barrier is possible if a dependency or safety issue makes further
editing unsafe: stop refilling and collect acknowledgements first. It is not the
default response to each worker completion. No global `--fix`, new `allow`,
feature reduction or exclusions to manufacture progress.

### Per-lot files are the recovery record

Before dispatch, the lead creates `.tmp/<campaign>/index.md` and assigns each
worker a unique absolute report path corresponding to
`.tmp/<campaign>/lots/<lot-id>.md`. The lead alone edits the index (claims,
dependencies, report/delta locations, integration and final validation states);
each worker writes only its own report. If tool permissions require a worktree-
local report, the lead must collect it in the canonical folder before reassignment
or worktree cleanup. Never depend on a future chat summary to reconstruct it.

Start the report at claim time and checkpoint meaningful progress. Finalize it
before reporting completion or accepting another lot; one reused worker produces
one file per lot, not one overwritten worker diary. Secure a per-lot commit or
patch/reference identifying its actual changes; cumulative worker diffs must not
erase attribution between successive lots. Preserve reports through final
validation. A crashed worker or missing final report is incomplete work to inspect,
not implied completion; keep its claim until disposition is verified.

Copy this textual template; keep it concise and never store secrets:

```markdown
# Lot <id>
- Status: in_progress | edited_unvalidated | no_change | blocked
- Owner/session; repository/worktree/branch; base HEAD and dirty-source identity:
- Snapshot/run ID; finding IDs; rules; current symbols/context:
- Authorized files; actual changed/added/deleted files:
- Changes and rationale; applicable decision IDs/skill precedents:
- Signatures/consumers affected (or none); invariants preserved:
- Delta: per-lot commit SHA or secured patch path/base/hash:
- Checks actually run: commands, results and evidence paths:
- Checks NOT run: commands and reason (including the shared editing phase):
- Remaining risks, dependencies, blockers and next diagnostic steps:
- Final handoff: no pending source writes; report/delta ready for lead collection:
```

`edited_unvalidated` means code was delivered, not that it compiles or fixes a
finding. The lead records acceptance against final evidence separately. After
context compaction or a lead change, read the index, referenced reports, source
deltas and current ownership before scheduling or validating. Reconcile them
with the actual tree; text is evidence, not proof that the tree matches it.
Temporary files support execution/recovery; durable decisions and final outcomes
still belong in the tracked decision register/mission handoff as appropriate.

## 5. Garder Sonar comme contrôle complémentaire

Les règles Sonar propres (notamment un bug `rust:S...` sans diagnostic Clippy
équivalent) restent une file séparée, à traiter selon leur risque. Clippy-first
ne veut pas dire ignorer une vulnérabilité ou un bug critique jusqu'au zéro
Clippy. Relocaliser ces cas dans le code actuel par règle, symbole et contexte,
pas par un calcul d'offset. Une égalité des compteurs n'est pas attendue.

Le scan Sonar suivant confirme la fermeture de ses issues et les écarts liés
à Linux/Windows ou aux règles différentes. Ne pas clôturer une issue Sonar/GitHub
sur la seule absence d'un warning local, ni changer manuellement son statut
pour masquer un problème. L'ancienne `.cursor/sonar-queue/` et les inventaires
des handoffs d'août/octobre sont des archives, pas le backlog courant.
