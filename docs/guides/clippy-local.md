# Clippy local aligné sur la CI

Depuis la racine, sous Windows natif : `just clippy-ci`.
Sans Just, lancer successivement `cargo clippy-ci-production` puis
`cargo clippy-ci-tests`. Ces commandes analysent les tests sans les exécuter.
Un seul processus Cargo à la fois ; les 24 jobs et le target-dir sur E: restent
ceux de `.cargo/config.toml`.

Pour classer les diagnostics, les répartir entre agents et relancer après les
corrections, suivre [Campagnes Clippy-first](clippy-campaigns.md). Les positions
des nouveaux rapports remplacent les anciennes lignes Sonar ; aucun calcul
d'offset entre scans n'est requis.

## Périmètre reproduit

Source : `.github/workflows/ci.yml`, étape `Run Clippy report (production vs tests)`.
Les deux alias dans `.cargo/config.toml` utilisent `--workspace --locked
--all-features` et le format JSON Cargo.

- Production : `--lib --bins --examples`, avec `unwrap_used`, `expect_used` et
  `panic` en avertissements.
- Tests : `--tests --benches`, avec ces trois règles autorisées. Cargo peut aussi
  réanalyser les bibliothèques nécessaires aux tests : les rapports se recouvrent.
- Commun : `all`, `pedantic`, `nursery`, `cargo`, `todo`, `unimplemented` en
  avertissements ; `cargo_common_metadata` et `multiple_crate_versions` autorisés.
- Aucun ajout de `-D warnings`, aucune correction automatique.
- Même toolchain épinglée par `rust-toolchain.toml` et l'action CI `setup-rust`.

`just clippy-ci` conserve les deux JSONL et leur concaténation dans
`target/sonar/`, comme la CI et `sonar-project.properties`. Un succès Cargo
n'implique pas zéro avertissement. Pour compter, dédoublonner les diagnostics
par niveau, règle, message et emplacement primaire ; ne pas compter les lignes
JSON ni additionner naïvement les deux passes.
Le lanceur `ops/scripts/clippy-ci.ps1` attend explicitement chaque arbre de
processus, exige l'événement final Cargo `build-finished` réussi et invalide
l'ancien rapport combiné avant de commencer. Les rapports sont sans BOM et la
concaténation préserve leurs octets ; les stderr restent dans `target/sonar/`.

Vérification statique de parité (Python 3.11+) :
`python -m unittest discover -s ops/tests -p test_clippy_profile.py`.
Le test échoue si les arguments CI et locaux divergent ; il ne lance pas Cargo.
Les tests du lanceur remplacent uniquement la frontière processus dans une copie
temporaire, pour vérifier les échecs de chaque passe, les rapports incomplets,
l'attente explicite et l'absence de publication d'un ancien rapport.

## Mesure locale du 13 novembre 2026

Deux passes natives réussies, rapports JSON entièrement parsés, zéro erreur :
- Production : 258 avertissements Clippy, 304 avertissements avec rustc.
- Tests/benches : 536 avertissements Clippy, 1 028 avertissements avec rustc.
- Union dédoublonnée : **538 Clippy + 492 rustc = 1 030 avertissements**.

Le précédent passage `--all-targets` avec features par défaut mesurait 425
Clippy + 475 rustc. L'écart n'est pas une régression du code : les options
d'analyse ont changé. Cette photographie porte sur les sources présentes au
moment du run ; elle ne qualifie pas les modifications d'autres missions
ajoutées ultérieurement.

## Limites explicites

La CI tourne sur Ubuntu et le local sur Windows : les branches `cfg`, dépendances
spécifiques à l'OS et diagnostics peuvent différer malgré des arguments identiques.
Ne pas remplacer silencieusement `--all-features` par les features par défaut si
une dépendance native bloque : consigner l'échec et l'écart de couverture.
Un simple `cargo clippy` ne sélectionne pas automatiquement ces alias et ne
reproduit donc pas les deux passes CI.

Sonar analyse les fichiers Rust (`sonar.sources=.`, `sonar.inclusions=**/*.rs`),
hors les chemins de `sonar.exclusions`. Ses exclusions `coverage` et `cpd`
concernent uniquement la couverture et la duplication, pas le périmètre Clippy.
Sonar importe le rapport Clippy, mais applique aussi ses propres règles : une
parité des commandes ne garantit pas l'égalité des totaux Clippy et Sonar.
Le sous-module RustyCog n'est pas un workspace à analyser séparément dans ces
commandes ; il est consommé comme dépendance, comme dans la CI du monorepo.

Les alias Cargo sont des raccourcis de commandes, pas des profils de compilation
`[profile.*]` : <https://doc.rust-lang.org/cargo/reference/config.html#alias>.
