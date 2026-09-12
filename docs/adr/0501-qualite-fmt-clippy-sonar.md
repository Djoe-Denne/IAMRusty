# ADR-0501 : fmt, Clippy et Sonar forment la politique de qualité du dépôt

- Statut : Accepted
- Réalité : Partial
- Date : 2026-09-12
- Décideurs : Architecture AIForAll — photographie rétroactive du dépôt
- Jalon concerné : architecture actuelle (hors P-Apparatus)
- SuperSède : aucune
- SuperSédée par : —

`Accepted` ratifie une cible. Le champ `Réalité` indique séparément ce que le code du dépôt réalise.

## Contexte

Le monorepo Rust doit rester formaté, analysé et mesuré sans que Clippy « deny » casse chaque PR, ni qu’un second projet Sonar dilue le gate. La cible de couverture sur le *new code* est plus haute que le chiffre live.

## Décision

1. **`cargo fmt --all -- --check`** est obligatoire : job CI `fmt` (les jobs suivants `needs: [fmt]`) **et** hook `.githooks/pre-commit`.
2. **Clippy** tourne dans le job **`sonar`**, pas comme job deny autonome. Groupes `-W` (`all`, `pedantic`, `nursery`, `cargo`, plus `todo` / `unimplemented` ; unwrap/expect/panic en `-W` sur prod, `-A` sur tests). **Pas de `-D`** Clippy (le seul `-D` du workflow est `-Dsonar.rust.lcov.reportPaths`).
3. **Projet Sonar monorepo** = `Djoe-Denne_IAMRusty` (`sonar-project.properties` ; organisation `djoe-denne06`, nom affiché `AIForAll`). Clippy natif Sonar off ; rapport `target/sonar/clippy.json`.
4. **Gate `new_coverage` 80 %** : cible ratifiée, **non tenue**. Mesure expert live **2026-09-12 : 73,3 %**. Mémoire Serena `sonar-clippy-2026-09-09` : **64,1 %** au 9 septembre. **Hotspots sécurité : 0**.

`Partial` parce que fmt + pipeline Clippy/Sonar sont en place, mais le quality gate couverture échoue.

## Conséquences

- Ne pas ajouter un job `clippy -D warnings` sans ADR.
- Ne pas présenter le dépôt comme « gate vert » tant que `new_coverage` < 80 %.
- Politique de correction Clippy : skill `aiforall-sonar-policy` (faux positifs ponctuels, pas un skip de gate).

## Alternatives rejetées

| Option | Pourquoi pas |
|---|---|
| Clippy `-D` bloquant en CI | Le workflow actuel collecte un rapport `-W` pour Sonar |
| Deuxième clé Sonar pour le monorepo | La clé canonique est `Djoe-Denne_IAMRusty` |
| Déclarer le gate 80 % tenu | 73,3 % live (12 sept.) ; 64,1 % au 9 sept. |

## Non décidé ici

- Date et plan pour passer `new_coverage` ≥ 80 %.
- Projet Sonar du submodule rustycog (`Djoe-Denne_rustycog`) — hors monorepo.
- Alignement des lints workspace `Cargo.toml` sur les groupes CI.

## Références

- CI : `.github/workflows/ci.yml` (jobs `fmt`, `coverage-*`, `merge-coverage`, `sonar`)
- Hook : `.githooks/pre-commit`
- Sonar : `sonar-project.properties` (`sonar.projectKey=Djoe-Denne_IAMRusty`)
- Skill : `.agents/skills/aiforall-sonar-policy/SKILL.md`
- Mémoire : Serena `sonar-clippy-2026-09-09` (64,1 %, hotspots 0)
- Preuve Partial : fmt+Clippy+scan implémentés ; gate 80 % non tenu (73,3 % le 2026-09-12)
