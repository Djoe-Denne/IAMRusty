# Clippy local / CI

- Skill : ci-cd ; mode : amélioration ciblée ; date : 2026-11-13.
- Périmètre : arguments Clippy CI, alias Cargo et lancement Windows natif.
- Vérification : deux passes réelles réussies, huit tests Python réussis,
  rapports entièrement parsés et concaténation binaire exacte.

## P2 — écarts corrigés

- [x] **#1 Le premier lancement local ne reproduisait pas le périmètre CI.**
  - Localisation : `.github/workflows/ci.yml:594–628`, `.cargo/config.toml`.
  - Preuve : CI all-features, production/tests séparés, unwrap/expect/panic
    warn production et allow tests ; ancien run default-features/all-targets.
  - Correction : alias locaux reprenant exactement les deux listes d'arguments,
    avec tests statiques de parité et de détection de dérive. CI inchangée.
- [x] **#2 Le lanceur doit attendre une preuve de fin, pas un retour shell.**
  - Localisation : `ops/scripts/clippy-ci.ps1`, recette `just clippy-ci`.
  - Preuve : les premières recettes shell expérimentales ont laissé les deux
    arbres Cargo actifs malgré le retour du shell ; rapports incomplets rejetés,
    aucun résultat de ces essais présenté comme mesure valide.
  - Correction : Start-Process -Wait, code retour et build-finished réussi
    obligatoires ; production puis tests ; ancien rapport combiné invalidé.
    Échecs/incomplétude/publication et attente couverts par tests sans Cargo.

## Résultats et limites

- Rapports retenus : `target/sonar/clippy-{production,tests}.jsonl` et `clippy.json`,
  zéro JSON invalide, sans BOM, concaténation exacte, deux build-finished true.
- Production : 258 Clippy / 304 avertissements ; tests : 536 / 1 028.
  Union : 538 Clippy + 492 rustc = 1 030 avertissements, zéro erreur.
- Logs locaux : `.tmp/clippy-ci-local-verified/` ; premiers essais distincts dans
  `.tmp/clippy-ci-local-*`, non retenus. Aucun Docker/Kind/WSL créé ou modifié.
- Même toolchain 1.99.1 ; OS Windows != Ubuntu, branches cfg potentiellement
  distinctes. Les règles Sonar natives et exclusions coverage/cpd ne sont pas
  celles de Clippy. Aucun objectif d'égalité des compteurs Sonar/Clippy annoncé.
- Les modifications Rust/k6 et worktrees étrangers en cours sont préservés,
  exclus du commit et non qualifiés par cette mesure antérieure à leur ajout.

Total : deux écarts corrigés, zéro ouvert dans ce périmètre. Différences OS et
règles Sonar restent des limites documentées, pas des correctifs implicites.
